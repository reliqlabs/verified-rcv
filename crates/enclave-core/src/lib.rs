//! Aeneas-extractable IRV tally core for verified-rcv.
//!
//! This crate is the central methodology obligation: B10_lean
//! (`EnclaveImage = Tally_spec`) is proven against the Lean term extracted
//! from this file via charon + aeneas. The public surface mirrors
//! `specs/RcvSpec.lean` Section 2.5 of the intent:
//! `Tally_spec = decrypt_and_validate composed with IRV_spec`.
//!
//! Stage 1 (`decrypt_and_validate`) is implemented in the runtime crate
//! `verified-rcv-enclave` which has ECIES access. Here it is `unimplemented!()`
//! so the type signature is fixed for downstream extraction.
//!
//! Stage 2 (`irv_spec`) is the full combinatorial IRV core: full preferential
//! ballots, batch elimination of all candidates tied for the lowest
//! first-place count, multi-winner fallback for terminal ties, majority
//! termination at `count > total / 2`.
//!
//! Aeneas-extractability constraints observed throughout:
//!   - Pure functions, no I/O, no async, no `&mut` outside locals.
//!   - No `Box<dyn Trait>`, no `impl Trait`, no closures-with-captures in
//!     loops.
//!   - HashMap avoided; sorted `Vec<(K, V)>` (here `RoundCounts`) used
//!     instead.
//!   - Iteration is explicit `for` / `while` over indices; iterator chains
//!     are avoided in the algorithmic core.
//!   - `?` operator avoided across complex error paths.
//!   - serde / schemars / cw_schema derives are gated behind the `serialize`
//!     feature; the default-feature build (which charon walks) carries only
//!     the plain POD derives.

#![forbid(unsafe_code)]

pub type Addr = String;
pub type Bytes = Vec<u8>;

#[cfg_attr(
    feature = "serialize",
    derive(
        serde::Serialize,
        serde::Deserialize,
        schemars::JsonSchema,
        cw_schema::Schemaifier,
    )
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ballot {
    pub ranking: Vec<Addr>,
}

#[cfg_attr(
    feature = "serialize",
    derive(
        serde::Serialize,
        serde::Deserialize,
        schemars::JsonSchema,
        cw_schema::Schemaifier,
    )
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEntry {
    pub voter: Addr,
    pub ciphertext: Bytes,
}

/// `(voter_addr, encrypted_ballot_bytes)` pairs as they arrive from chain.
pub type RawBallots = Vec<RawEntry>;

pub type CandidateSet = Vec<Addr>;
pub type PrivKey = Bytes;

#[cfg_attr(
    feature = "serialize",
    derive(
        serde::Serialize,
        serde::Deserialize,
        schemars::JsonSchema,
        cw_schema::Schemaifier,
    )
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundCount {
    pub candidate: Addr,
    pub count: u32,
}

pub type RoundCounts = Vec<RoundCount>;

#[cfg_attr(
    feature = "serialize",
    derive(
        serde::Serialize,
        serde::Deserialize,
        schemars::JsonSchema,
        cw_schema::Schemaifier,
    )
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IRVResult {
    pub winners: Vec<Addr>,
    pub per_round_counts: Vec<RoundCounts>,
    pub eliminated_by_round: Vec<Vec<Addr>>,
    pub ballots_tallied: u32,
}

#[cfg_attr(
    feature = "serialize",
    derive(
        serde::Serialize,
        serde::Deserialize,
        schemars::JsonSchema,
        cw_schema::Schemaifier,
    )
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TallyResult {
    pub winners: Vec<Addr>,
    pub per_round_counts: Vec<RoundCounts>,
    pub eliminated_by_round: Vec<Vec<Addr>>,
    pub ballots_tallied: u32,
    pub ballots_dropped: u32,
    pub dropped_voters: Vec<Addr>,
    pub non_voters: Vec<Addr>,
}

#[cfg_attr(
    feature = "serialize",
    derive(
        serde::Serialize,
        serde::Deserialize,
        schemars::JsonSchema,
        cw_schema::Schemaifier,
    )
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecryptedSet {
    pub valid: Vec<(Addr, Ballot)>,
    pub dropped: Vec<Addr>,
    pub non_voters: Vec<Addr>,
}

// ---------------------------------------------------------------------------
// Internal helpers (private; charon walks them as part of `irv_spec`'s body)
// ---------------------------------------------------------------------------

/// True iff `a` appears in `xs`.
fn addr_in(xs: &Vec<Addr>, a: &Addr) -> bool {
    let n = xs.len();
    let mut i: usize = 0;
    while i < n {
        if xs[i] == *a {
            return true;
        }
        i += 1;
    }
    false
}

/// Index into `remaining` of the first surviving candidate the ballot
/// ranks. `None` if the ballot exhausts before hitting a surviving
/// candidate (cannot happen when ballots are full permutations of
/// `candidates` and `remaining` is non-empty; the function is total for
/// extraction purposes).
/// Helper: linear scan for `needle` in `haystack`. Single-loop / single-return
/// shape so Aeneas extraction doesn't trip on nested-return.
fn position_of(needle: &Addr, haystack: &Vec<Addr>) -> Option<usize> {
    let n = haystack.len();
    let mut j: usize = 0;
    while j < n {
        if haystack[j] == *needle {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn first_active_index(ranking: &Vec<Addr>, remaining: &Vec<Addr>) -> Option<usize> {
    let r_len = ranking.len();
    let mut i: usize = 0;
    while i < r_len {
        let idx_opt = position_of(&ranking[i], remaining);
        if idx_opt.is_some() {
            return idx_opt;
        }
        i += 1;
    }
    None
}

/// Compute first-preference counts over `remaining`. Result preserves
/// `remaining` order so iteration is deterministic and extraction-stable.
/// Count how many ballots in `valid` would route to `remaining[target_idx]`
/// as their first-active choice. Independent per-index helper avoids the
/// `arr[i] = arr[i] + 1` array-element-mutation pattern that triggers
/// Aeneas's `expand_symbolic_value_no_branching` failure. Local counter
/// mutation (`count = count + 1`) is fine — that pattern extracts cleanly.
fn count_at_index(
    valid: &Vec<(Addr, Ballot)>,
    remaining: &Vec<Addr>,
    target_idx: usize,
) -> usize {
    let n = valid.len();
    let mut count: usize = 0;
    let mut b: usize = 0;
    while b < n {
        let ranking: &Vec<Addr> = &valid[b].1.ranking;
        let idx_opt = first_active_index(ranking, remaining);
        if idx_opt.is_some() && idx_opt.unwrap() == target_idx {
            count = count + 1;
        }
        b += 1;
    }
    count
}

fn tally_round(valid: &Vec<(Addr, Ballot)>, remaining: &Vec<Addr>) -> RoundCounts {
    let rem_len = remaining.len();
    let mut out: RoundCounts = Vec::with_capacity(rem_len);
    let mut i: usize = 0;
    while i < rem_len {
        let c = count_at_index(valid, remaining, i);
        out.push(RoundCount {
            candidate: remaining[i].clone(),
            count: c as u32,
        });
        i += 1;
    }
    out
}

/// Minimum `count` across a non-empty `RoundCounts`. Defensive zero for
/// empty input; `irv_spec` never invokes this on empty.
fn min_count(rc: &RoundCounts) -> u32 {
    let n = rc.len();
    if n == 0 {
        return 0u32;
    }
    let mut m: u32 = rc[0].count;
    let mut i: usize = 1;
    while i < n {
        if rc[i].count < m {
            m = rc[i].count;
        }
        i += 1;
    }
    m
}

/// Sum of counts in `rc`. Drives the majority test (`count > total / 2`).
fn total_count(rc: &RoundCounts) -> u32 {
    let n = rc.len();
    let mut s: u32 = 0;
    let mut i: usize = 0;
    while i < n {
        s = s + rc[i].count;
        i += 1;
    }
    s
}

/// All candidates whose count equals `m`, in `rc` order. With
/// `m = min_count(rc)` this is the "losers" set for batch elimination.
fn candidates_with_count(rc: &RoundCounts, m: u32) -> Vec<Addr> {
    let n = rc.len();
    let mut out: Vec<Addr> = Vec::new();
    let mut i: usize = 0;
    while i < n {
        if rc[i].count == m {
            out.push(rc[i].candidate.clone());
        }
        i += 1;
    }
    out
}

/// First index in `rc` whose count strictly exceeds `threshold` (the
/// majority candidate, if any). `None` if no candidate clears the bar.
fn first_majority_index(rc: &RoundCounts, threshold: u32) -> Option<usize> {
    let n = rc.len();
    let mut i: usize = 0;
    while i < n {
        if rc[i].count > threshold {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `remaining` with the elements of `to_remove` filtered out. Preserves
/// `remaining` order.
fn remove_from(remaining: &Vec<Addr>, to_remove: &Vec<Addr>) -> Vec<Addr> {
    let n = remaining.len();
    let mut out: Vec<Addr> = Vec::new();
    let mut i: usize = 0;
    while i < n {
        if !addr_in(to_remove, &remaining[i]) {
            out.push(remaining[i].clone());
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Stage 2: the combinatorial IRV core. Pure function over already-decrypted
/// and validated ballots. Implements the Australian Federal IRV variant
/// described in §2.5 of the intent: full preferential ballots, batch
/// elimination of all tied-for-lowest candidates, multi-winner fallback on
/// terminal ties, majority termination at `count > total / 2`.
pub fn irv_spec(valid: &[(Addr, Ballot)], candidates: &CandidateSet) -> IRVResult {
    let ballots_tallied: u32 = valid.len() as u32;

    // Copy the slice into a local Vec so the inner loops have a stable
    // type to walk; Aeneas / charon handle `Vec` more cleanly than
    // slice refs in some patterns.
    let valid_vec: Vec<(Addr, Ballot)> = valid.to_vec();

    let mut per_round_counts: Vec<RoundCounts> = Vec::new();
    let mut eliminated_by_round: Vec<Vec<Addr>> = Vec::new();

    // Initial `remaining` is the candidate-declaration-ordered set.
    let mut remaining: Vec<Addr> = candidates.clone();

    // Edge case: zero candidates. All-abstain over empty candidates
    // produces no winners, no rounds. (Out of band of intent §2.5
    // which assumes candidates is non-empty, but kept as a total
    // function for extraction.)
    if candidates.len() == 0 {
        return IRVResult {
            winners: Vec::new(),
            per_round_counts: Vec::new(),
            eliminated_by_round: Vec::new(),
            ballots_tallied,
        };
    }

    // The IRV recursion is bounded by `|candidates|`. We use a counter
    // cap rather than open recursion so extraction sees an explicit
    // termination measure. The `+ 1` is a defensive overshoot the
    // algorithm never reaches; every iteration either breaks on a
    // terminal condition or strictly shrinks `remaining` by >= 1.
    let max_rounds: usize = candidates.len() + 1;
    let mut round: usize = 0;

    let winners: Vec<Addr>;

    loop {
        if round >= max_rounds {
            // Defensive bound. The termination measure (remaining
            // strictly shrinks each non-terminal round) means this is
            // unreachable; we treat current remaining as co-winners.
            winners = remaining.clone();
            break;
        }

        // Tally first-preferences against the current `remaining` set.
        let rc: RoundCounts = tally_round(&valid_vec, &remaining);
        per_round_counts.push(rc.clone());

        // Rule 1 (base case): empty remaining. Spec §2.5 case 5
        // ("all abstain") is handled below via the zero-total path;
        // here we catch a recursion that has emptied remaining (cannot
        // happen given the multi-winner fallback rule 4, but guarded
        // for totality).
        if remaining.len() == 0 {
            winners = candidates.clone();
            break;
        }

        // Rule 1 (base case): single candidate left — sole winner.
        if remaining.len() == 1 {
            winners = remaining.clone();
            break;
        }

        // Rule 5 (all-abstain): no active votes this round. Per §2.5
        // case 5, declare all original candidates as co-winners with
        // a single zero-count round and no eliminations.
        let total: u32 = total_count(&rc);
        if total == 0 {
            // Pin the just-recorded round to the all-zero counts over
            // the original candidate set, per the intent text:
            // "per_round_counts = [{c: 0 for c in candidates}]".
            // In the common all-abstain entry path (zero valid ballots
            // on the first iteration), `remaining == candidates`, so
            // the replacement is a no-op; the path stays uniform.
            let n_cands = candidates.len();
            let mut zero_round: RoundCounts = Vec::with_capacity(n_cands);
            let mut zi: usize = 0;
            while zi < n_cands {
                zero_round.push(RoundCount {
                    candidate: candidates[zi].clone(),
                    count: 0u32,
                });
                zi += 1;
            }
            let last_idx = per_round_counts.len() - 1;
            per_round_counts[last_idx] = zero_round;

            winners = candidates.clone();
            break;
        }

        // Rule 2 (majority termination): count[c] > total / 2 for some c.
        // Integer division floors total / 2; the majority test is strict.
        let threshold: u32 = total / 2u32;
        match first_majority_index(&rc, threshold) {
            Some(idx) => {
                let mut single_winner: Vec<Addr> = Vec::with_capacity(1);
                single_winner.push(rc[idx].candidate.clone());
                winners = single_winner;
                break;
            }
            None => {
                // Fall through to recursive case.
            }
        }

        // Rules 3 + 4: batch elimination with terminal-tie fallback.
        let m: u32 = min_count(&rc);
        let losers: Vec<Addr> = candidates_with_count(&rc, m);

        // Rule 4 (terminal tie): if losers == remaining (all remaining
        // tied at min, batch elimination would empty remaining),
        // declare all current `remaining` candidates as co-winners.
        // No elimination is recorded for this terminal round.
        if losers.len() == remaining.len() {
            winners = remaining.clone();
            break;
        }

        // Rule 3 proper: eliminate the losers, recurse.
        eliminated_by_round.push(losers.clone());
        remaining = remove_from(&remaining, &losers);

        round += 1;
    }

    IRVResult {
        winners,
        per_round_counts,
        eliminated_by_round,
        ballots_tallied,
    }
}

/// Stage 1: decrypt + validate. The body lives in the runtime crate which
/// has ECIES access; here we expose only the signature so Aeneas extracts
/// it as an opaque function. The corresponding Lean declaration
/// `decrypt_and_validate` is `opaque` in `RcvSpec.lean`.
pub fn decrypt_and_validate(
    _raw: &RawBallots,
    _candidates: &CandidateSet,
    _privkey: &PrivKey,
) -> DecryptedSet {
    unimplemented!(
        "decrypt_and_validate runs in the enclave runtime; \
         this crate provides the type signature only"
    )
}

/// Composition: `Tally_spec = decrypt_and_validate composed with irv_spec`,
/// threading Stage 1's voter bookkeeping (dropped_voters, non_voters) into
/// Stage 2's IRV output. This is the function `B10_lean` proves against.
pub fn tally_spec(
    raw: &RawBallots,
    candidates: &CandidateSet,
    privkey: &PrivKey,
) -> TallyResult {
    let d: DecryptedSet = decrypt_and_validate(raw, candidates, privkey);
    let r: IRVResult = irv_spec(&d.valid, candidates);
    let ballots_dropped: u32 = d.dropped.len() as u32;
    TallyResult {
        winners: r.winners,
        per_round_counts: r.per_round_counts,
        eliminated_by_round: r.eliminated_by_round,
        ballots_tallied: r.ballots_tallied,
        ballots_dropped,
        dropped_voters: d.dropped,
        non_voters: d.non_voters,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> Addr {
        s.to_string()
    }

    fn ballot(ranks: &[&str]) -> Ballot {
        let mut v: Vec<Addr> = Vec::with_capacity(ranks.len());
        for r in ranks {
            v.push(addr(r));
        }
        Ballot { ranking: v }
    }

    fn entry(voter: &str, ranks: &[&str]) -> (Addr, Ballot) {
        (addr(voter), ballot(ranks))
    }

    fn count_for(rc: &RoundCounts, c: &str) -> Option<u32> {
        for e in rc {
            if e.candidate == addr(c) {
                return Some(e.count);
            }
        }
        None
    }

    // -----------------------------------------------------------------
    // Test 1: single-round majority win.
    // 5 voters, 3 candidates. A gets 3 first-place; 3 > 5/2 = 2 → wins
    // in 1 round.
    // -----------------------------------------------------------------
    #[test]
    fn single_round_majority() {
        let candidates: CandidateSet = vec![addr("A"), addr("B"), addr("C")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C"]),
            entry("v2", &["A", "C", "B"]),
            entry("v3", &["A", "B", "C"]),
            entry("v4", &["B", "A", "C"]),
            entry("v5", &["C", "A", "B"]),
        ];

        let r = irv_spec(&valid, &candidates);

        assert_eq!(r.winners, vec![addr("A")]);
        assert_eq!(r.per_round_counts.len(), 1);
        assert_eq!(r.eliminated_by_round.len(), 0);
        assert_eq!(r.ballots_tallied, 5);
        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);

        let rc0 = &r.per_round_counts[0];
        assert_eq!(count_for(rc0, "A"), Some(3));
        assert_eq!(count_for(rc0, "B"), Some(1));
        assert_eq!(count_for(rc0, "C"), Some(1));
    }

    // -----------------------------------------------------------------
    // Test 2: two-round elimination.
    // 5 voters, 3 candidates. Round 1: A=2, B=2, C=1; no majority.
    // Eliminate C; C's ballot redistributes to A → A=3, B=2 → A wins.
    // -----------------------------------------------------------------
    #[test]
    fn two_round_elimination() {
        let candidates: CandidateSet = vec![addr("A"), addr("B"), addr("C")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C"]),
            entry("v2", &["A", "C", "B"]),
            entry("v3", &["B", "A", "C"]),
            entry("v4", &["B", "C", "A"]),
            entry("v5", &["C", "A", "B"]),
        ];

        let r = irv_spec(&valid, &candidates);

        assert_eq!(r.ballots_tallied, 5);
        assert_eq!(r.per_round_counts.len(), 2);
        assert_eq!(r.eliminated_by_round.len(), 1);
        assert_eq!(r.eliminated_by_round[0], vec![addr("C")]);

        let rc0 = &r.per_round_counts[0];
        assert_eq!(count_for(rc0, "A"), Some(2));
        assert_eq!(count_for(rc0, "B"), Some(2));
        assert_eq!(count_for(rc0, "C"), Some(1));

        let rc1 = &r.per_round_counts[1];
        assert_eq!(count_for(rc1, "A"), Some(3));
        assert_eq!(count_for(rc1, "B"), Some(2));
        assert_eq!(count_for(rc1, "C"), None);

        assert_eq!(r.winners, vec![addr("A")]);
        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);
    }

    // -----------------------------------------------------------------
    // Test 3: all-abstain. 0 valid ballots, 3 candidates → 3 co-winners,
    // 1 round of zero counts, 0 eliminations.
    // -----------------------------------------------------------------
    #[test]
    fn all_abstain() {
        let candidates: CandidateSet = vec![addr("A"), addr("B"), addr("C")];
        let valid: Vec<(Addr, Ballot)> = Vec::new();

        let r = irv_spec(&valid, &candidates);

        assert_eq!(r.winners, candidates);
        assert_eq!(r.ballots_tallied, 0);
        assert_eq!(r.per_round_counts.len(), 1);
        assert_eq!(r.eliminated_by_round.len(), 0);

        let rc0 = &r.per_round_counts[0];
        assert_eq!(rc0.len(), 3);
        for e in rc0 {
            assert_eq!(e.count, 0u32);
        }

        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);
    }

    // -----------------------------------------------------------------
    // Test 4: terminal tie. 3 voters with distinct first-choices, 3
    // candidates → all tied at 1, losers == remaining → terminal tie
    // → all candidates co-win, no eliminations recorded.
    // -----------------------------------------------------------------
    #[test]
    fn terminal_tie() {
        let candidates: CandidateSet = vec![addr("A"), addr("B"), addr("C")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C"]),
            entry("v2", &["B", "C", "A"]),
            entry("v3", &["C", "A", "B"]),
        ];

        let r = irv_spec(&valid, &candidates);

        assert_eq!(r.winners.len(), 3);
        assert_eq!(r.winners, candidates);
        assert_eq!(r.ballots_tallied, 3);
        assert_eq!(r.per_round_counts.len(), 1);
        assert_eq!(r.eliminated_by_round.len(), 0);

        let rc0 = &r.per_round_counts[0];
        assert_eq!(count_for(rc0, "A"), Some(1));
        assert_eq!(count_for(rc0, "B"), Some(1));
        assert_eq!(count_for(rc0, "C"), Some(1));

        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);
    }

    // -----------------------------------------------------------------
    // Test 5: single candidate. 1 candidate → sole winner in 1 round.
    // -----------------------------------------------------------------
    #[test]
    fn single_candidate() {
        let candidates: CandidateSet = vec![addr("A")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A"]),
            entry("v2", &["A"]),
        ];

        let r = irv_spec(&valid, &candidates);

        assert_eq!(r.winners, vec![addr("A")]);
        assert_eq!(r.ballots_tallied, 2);
        assert_eq!(r.per_round_counts.len(), 1);
        assert_eq!(r.eliminated_by_round.len(), 0);
        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);
    }

    // -----------------------------------------------------------------
    // Test 6: batch elimination. 6 voters, 4 candidates → Round 1:
    // A=2, B=2, C=1, D=1. min=1, losers={C,D}, batch of 2.
    // Round 2: C's ballot (C>A>B>D) → A; D's ballot (D>B>A>C) → B.
    // Final: A=3, B=3, total=6, threshold = 6/2 = 3; 3 > 3 is false →
    // no majority. min=3, losers={A,B}, losers.len() == remaining.len()
    // = 2 → terminal tie → A, B co-win.
    // -----------------------------------------------------------------
    #[test]
    fn batch_elimination() {
        let candidates: CandidateSet =
            vec![addr("A"), addr("B"), addr("C"), addr("D")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C", "D"]),
            entry("v2", &["A", "B", "C", "D"]),
            entry("v3", &["B", "A", "C", "D"]),
            entry("v4", &["B", "A", "C", "D"]),
            entry("v5", &["C", "A", "B", "D"]),
            entry("v6", &["D", "B", "A", "C"]),
        ];

        let r = irv_spec(&valid, &candidates);

        assert_eq!(r.ballots_tallied, 6);

        let rc0 = &r.per_round_counts[0];
        assert_eq!(count_for(rc0, "A"), Some(2));
        assert_eq!(count_for(rc0, "B"), Some(2));
        assert_eq!(count_for(rc0, "C"), Some(1));
        assert_eq!(count_for(rc0, "D"), Some(1));

        assert_eq!(r.eliminated_by_round.len(), 1);
        assert_eq!(r.eliminated_by_round[0].len(), 2);
        assert!(r.eliminated_by_round[0].contains(&addr("C")));
        assert!(r.eliminated_by_round[0].contains(&addr("D")));

        assert_eq!(r.per_round_counts.len(), 2);
        let rc1 = &r.per_round_counts[1];
        assert_eq!(count_for(rc1, "A"), Some(3));
        assert_eq!(count_for(rc1, "B"), Some(3));
        assert_eq!(count_for(rc1, "C"), None);
        assert_eq!(count_for(rc1, "D"), None);

        assert_eq!(r.winners.len(), 2);
        assert!(r.winners.contains(&addr("A")));
        assert!(r.winners.contains(&addr("B")));

        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);
    }

    // -----------------------------------------------------------------
    // Test 7: §2.1-style multi-round trace — confirm A6 length relation
    // and winner-shape invariants hold for a non-trivial 5-candidate
    // election.
    // -----------------------------------------------------------------
    #[test]
    fn a6_length_relation_holds_for_multiround_trace() {
        let candidates: CandidateSet =
            vec![addr("A"), addr("B"), addr("C"), addr("D"), addr("E")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C", "D", "E"]),
            entry("v2", &["A", "B", "C", "D", "E"]),
            entry("v3", &["B", "A", "C", "D", "E"]),
            entry("v4", &["B", "A", "C", "D", "E"]),
            entry("v5", &["C", "A", "B", "D", "E"]),
            entry("v6", &["D", "C", "A", "B", "E"]),
            entry("v7", &["E", "C", "A", "B", "D"]),
        ];

        let r = irv_spec(&valid, &candidates);

        // A6 length relation.
        assert_eq!(r.per_round_counts.len(), r.eliminated_by_round.len() + 1);
        // ballots_tallied = |valid|.
        assert_eq!(r.ballots_tallied, 7);
        // Winners non-empty, all members of candidates.
        assert!(r.winners.len() >= 1);
        for w in &r.winners {
            assert!(candidates.contains(w));
        }
        // Termination: at most |candidates| rounds.
        assert!(r.per_round_counts.len() <= candidates.len());
    }

    // -----------------------------------------------------------------
    // Test 8: S8 sanity — per-round counts sum to ballots_tallied for
    // full-permutation ballots (no exhaustion path taken).
    // -----------------------------------------------------------------
    #[test]
    fn round_counts_sum_to_ballots_tallied() {
        let candidates: CandidateSet = vec![addr("A"), addr("B"), addr("C")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C"]),
            entry("v2", &["A", "C", "B"]),
            entry("v3", &["B", "A", "C"]),
            entry("v4", &["B", "C", "A"]),
            entry("v5", &["C", "A", "B"]),
        ];

        let r = irv_spec(&valid, &candidates);

        for rc in &r.per_round_counts {
            let s: u32 = total_count(rc);
            assert_eq!(s, r.ballots_tallied);
        }
    }

    // -----------------------------------------------------------------
    // Test 9: S9 sanity — no eliminated candidate reappears in a
    // later per_round_counts row.
    // -----------------------------------------------------------------
    #[test]
    fn no_reappearance() {
        let candidates: CandidateSet =
            vec![addr("A"), addr("B"), addr("C"), addr("D")];
        let valid: Vec<(Addr, Ballot)> = vec![
            entry("v1", &["A", "B", "C", "D"]),
            entry("v2", &["A", "B", "C", "D"]),
            entry("v3", &["B", "A", "C", "D"]),
            entry("v4", &["B", "A", "C", "D"]),
            entry("v5", &["C", "A", "B", "D"]),
            entry("v6", &["D", "B", "A", "C"]),
        ];

        let r = irv_spec(&valid, &candidates);

        let n_elim = r.eliminated_by_round.len();
        let mut i: usize = 0;
        while i < n_elim {
            let eliminated_round_i: &Vec<Addr> = &r.eliminated_by_round[i];
            let mut j: usize = i + 1;
            while j < r.per_round_counts.len() {
                let rc_j: &RoundCounts = &r.per_round_counts[j];
                for c in eliminated_round_i {
                    for e in rc_j {
                        assert!(
                            e.candidate != *c,
                            "candidate {} eliminated at round {} reappeared at round {}",
                            c,
                            i,
                            j
                        );
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }

    // -----------------------------------------------------------------
    // Test 10: composition-layer field threading. Since
    // decrypt_and_validate is unimplemented!(), we construct a
    // DecryptedSet directly and assemble the TallyResult by hand;
    // this exercises the field-threading shape that `tally_spec`
    // implements without depending on the Stage 1 stub.
    // -----------------------------------------------------------------
    #[test]
    fn tally_spec_composition_shape() {
        let candidates: CandidateSet = vec![addr("A"), addr("B"), addr("C")];
        let d = DecryptedSet {
            valid: vec![
                entry("v1", &["A", "B", "C"]),
                entry("v2", &["A", "B", "C"]),
                entry("v3", &["A", "B", "C"]),
            ],
            dropped: vec![addr("v4")],
            non_voters: vec![addr("v5")],
        };
        let r = irv_spec(&d.valid, &candidates);
        let t = TallyResult {
            winners: r.winners.clone(),
            per_round_counts: r.per_round_counts.clone(),
            eliminated_by_round: r.eliminated_by_round.clone(),
            ballots_tallied: r.ballots_tallied,
            ballots_dropped: d.dropped.len() as u32,
            dropped_voters: d.dropped.clone(),
            non_voters: d.non_voters.clone(),
        };

        assert_eq!(t.winners, vec![addr("A")]);
        assert_eq!(t.ballots_tallied, 3);
        assert_eq!(t.ballots_dropped, 1);
        assert_eq!(t.dropped_voters, vec![addr("v4")]);
        assert_eq!(t.non_voters, vec![addr("v5")]);
        assert_eq!(t.per_round_counts.len(), t.eliminated_by_round.len() + 1);
    }

    // -----------------------------------------------------------------
    // Test 11: the Stage 1 stub panics. Confirms the runtime crate is
    // the one expected to provide a body.
    // -----------------------------------------------------------------
    #[test]
    #[should_panic(expected = "decrypt_and_validate runs in the enclave runtime")]
    fn decrypt_and_validate_is_stub() {
        let raw: RawBallots = Vec::new();
        let candidates: CandidateSet = Vec::new();
        let privkey: PrivKey = Vec::new();
        let _ = decrypt_and_validate(&raw, &candidates, &privkey);
    }
}
