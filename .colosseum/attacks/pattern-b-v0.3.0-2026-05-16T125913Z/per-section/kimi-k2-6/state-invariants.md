# kimi-k2-6 — slice state-invariants

## Cross-section reads
- Section 2.5 Block 6 Requires (lines 261–266) — to ground coverage-gap claim that four set-relation well-formedness clauses are missing from the S-series table
- Section 2.5 `TallyResult` schema (lines 123–131) — to verify that `winners`, `dropped_voters`, `non_voters` are `Vec<Addr>` (ordered sequences), not sets, making set-notation in S6 and Block 6 ambiguous
- Section 2.5 transaction-trace model paragraph (line 149) — to confirm "handler index" is not a state variable
- Section 3.2 B3 (line 341) — to evaluate S10's claimed derivation
- Section 6.1 trust boundary (line 475) — to verify block-time monotonicity is an operational assumption, not an invariant

## Attacks on this slice

### 1. S5 is a code property, not a pointwise state invariant [serious]
- **Category**: temporal-state mismatch
- **Affected**: S5 (line 326)
- **What's wrong**: The Section 3.1 preamble requires properties "evaluable on contract state at any reachable moment" with "No quantification over operations or time." S5 claims to satisfy this by being "Evaluable on a single state by inspecting the handler index." But the contract state (Section 2.5) contains only `candidates`, `start_at`, `end_at`, `enclave_pubkey`, `ballots`, and `tally_result` — there is no "handler index" in the state. The property "no handler writes to `tally_result` after it has been set to `Some(_)`" is a statement about the contract's code (which handlers exist and what they do), not about any single state snapshot. It quantifies over operations (handlers), violating the preamble. The restatement does not successfully purge successor quantification; it merely hides it behind an undefined "handler index" pseudo-state.
- **Cite**: > "Evaluable on a single state by inspecting the handler index, not by quantifying over successor states."
- **Fix recommendation**: Remove S5 from Section 3.1. The write-discipline is a static code property that belongs in the handler specification (Block 6). If a state-shape shadow is truly needed, replace S5 with the trivial type-level observation "`tally_result` is either `None` or `Some(_)`", and let B1 carry the temporal load.

### 2. Block 6 set-relation well-formedness clauses missing from S1..S10 [serious]
- **Category**: coverage gap
- **Affected**: S1..S10 table (lines 320–331) vs. Block 6 Requires (lines 261–266)
- **What's wrong**: Block 6's well-formedness predicate includes four pointwise-evaluable set-relation checks: `non_voters = candidates \ ballots.keys`, `dropped_voters ⊆ ballots.keys`, `dropped_voters ∩ non_voters = ∅`, and `len(dropped_voters) = ballots_dropped`. These are pure state-shape properties — they require only reading `ballots`, `candidates`, and the published `tally_result` at a single moment. Yet none appear in the structural invariant table. A reader or downstream spec author treating Section 3.1 as the exhaustive catalog of state-shape invariants will omit them.
- **Cite**: > "set relations (Round 3a first-pass adversary Attack 28 + 35): `non_voters = candidates \ ballots.keys`, `dropped_voters ⊆ ballots.keys`, `dropped_voters ∩ non_voters = ∅`. `len(dropped_voters) = ballots_dropped`."
- **Fix recommendation**: Add explicit structural invariants (e.g., S11–S14) capturing these four set relations in Section 3.1.

### 3. S6 allows duplicate winners and winners eliminated in earlier rounds [serious]
- **Category**: under-specification
- **Affected**: S6 (line 328)
- **What's wrong**: S6 requires `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`. Because `winners` is a `Vec<Addr>` (not a set) and the invariant uses set-subset notation, a malformed result with `winners = [A, A]` satisfies S6 (`{A} ⊆ candidates`, length 2 ≤ `len(candidates)`). More severely, S6 does not require winners to appear in the final round of `per_round_counts`. A tally with `winners = [Z]`, `eliminated_by_round = [[Z]]`, and `per_round_counts = [{A: 5}]` satisfies S6 and S9 (Z does not reappear in later rounds) while being nonsensical — the declared winner was eliminated in round 0. The worked examples (Sections 2.1, 2.2) always show winners as the surviving candidates of the last round, but no structural invariant enforces this.
- **Cite**: > `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`
- **Fix recommendation**: Strengthen S6 to: (a) `winners` contains no duplicate addresses, (b) `winners = per_round_counts[last].keys()` (as sets), and mirror this in Block 6's well-formedness predicate.

### 4. S9 permits duplicate eliminations and unrecorded disappearances [serious]
- **Category**: under-specification
- **Affected**: S9 (line 330)
- **What's wrong**: S9 only states that eliminated candidates do not reappear in later rounds' counts. It imposes no constraint on `eliminated_by_round` itself. A buggy enclave could produce `eliminated_by_round = [[B], [B, C]]` (B eliminated twice) and satisfy S9 because S9 never says each candidate is eliminated at most once. Conversely, a candidate could vanish from `per_round_counts[i]` to `per_round_counts[i+1]` without ever appearing in any `eliminated_by_round` entry, and S9 would still hold. The worked examples consistently show `len(per_round_counts) = len(eliminated_by_round) + 1` and pairwise-disjoint elimination rounds, but no invariant codifies either property.
- **Cite**: > "if a candidate appears in `eliminated_by_round[i]`, they appear in no `per_round_counts[j]` for `j > i`"
- **Fix recommendation**: Add structural invariants: (a) the vectors inside `eliminated_by_round` are pairwise disjoint; (b) `len(per_round_counts) = len(eliminated_by_round) + 1` when `tally_result.is_some()`; (c) any candidate present in `per_round_counts[i]` but absent from `per_round_counts[i+1]` must appear in `eliminated_by_round[i]`.

### 5. S10 derivation omits block-time monotonicity premise [cosmetic]
- **Category**: refinement mismatch
- **Affected**: S10 (line 331)
- **What's wrong**: S10 is labeled "Derived from B3." B3 guarantees `env.block.time ≥ end_at` at the exact transition where `tally_result` becomes `Some`. To conclude that `env.block.time ≥ end_at` holds at *all* later reachable states where `tally_result.is_some()`, one needs the additional premise that `env.block.time` is non-decreasing across blocks. This premise lives in Section 6.1 as a trust-boundary assumption, not as an invariant. The one-line "Derived from B3" note is therefore an incomplete proof sketch.
- **Cite**: > "Derived from B3 (temporal causal version)"
- **Fix recommendation**: Amend the derivation note to "Derived from B3 plus block-time monotonicity (Section 6.1)."

## Slice-local summary
- Critical: 0
- Serious: 4
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE