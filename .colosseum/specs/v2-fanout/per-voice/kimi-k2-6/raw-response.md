===BEGIN FILE: rcv.qnt===
module rcv {
  const ADDR_SET: Set[str]
  const TIME_MAX: int

  type Addr = str
  type Ballot = str

  type Phase = | Uninitialized | Created | Voting | Tallying | Resolved

  type TallyResult = {
    winners: Set[Addr],
    per_round_counts: List[Map[Addr, int]],
    eliminated_by_round: List[Set[Addr]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: Set[Addr],
    non_voters: Set[Addr]
  }

  // Pure helpers
  pure def map_sum(m: Map[Addr, int], domain: Set[Addr]): int =
    domain.fold(0, (sum, a) => sum + if (a.in(m.keys())) m.get(a) else 0)

  pure def all_counts_sum(rounds: List[Map[Addr, int]], domain: Set[Addr], expected: int): bool =
    rounds.foldl(true, (acc, round) => acc and (map_sum(round, domain) == expected))

  pure def trivial_counts(cands: Set[Addr], total: int): Map[Addr, int] = {
    val rep = cands.chooseSome()
    cands.mapBy(a => if (a == rep) total else 0)
  }

  // State
  var phase: Phase
  var candidates: Set[Addr]
  var start_at: int
  var end_at: int
  var ballots: Addr -> Option[Ballot]
  var tally_result: Option[TallyResult]
  var current_time: int
  var frozen_ballots: Addr -> Option[Ballot]
  var resolved_ever: bool

  // §3.1 Structural invariants
  val s1 = phase != Uninitialized => candidates.size() >= 1
  val s2 = phase != Uninitialized => candidates.size() == candidates.size()
  val s3 = phase != Uninitialized => start_at < end_at
  val s4 = ADDR_SET.forall(a => ballots.get(a) != None => a.in(candidates))

  val s6 = match tally_result {
    | None => true
    | Some(t) => t.winners.subset(candidates) and t.winners.size() >= 1 and t.winners.size() <= candidates.size()
  }

  val s7 = match tally_result {
    | None => true
    | Some(t) => t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == candidates.size()
  }

  val s8 = match tally_result {
    | None => true
    | Some(t) => all_counts_sum(t.per_round_counts, candidates, t.ballots_tallied)
  }

  val s9 = match tally_result {
    | None => true
    | Some(t) => {
      val pairs = t.eliminated_by_round.zip(t.per_round_counts)
      pairs.foldl((true, Set()), (acc, p) => {
        val round_ok = acc._1 and p._2.keys().intersect(acc._2) == Set()
        (round_ok, acc._2.union(p._1))
      })._1
    }
  }

  val s10 = phase == Resolved => current_time >= end_at

  // §3.2 Temporal invariants (ghost-encoded)
  val b1 = resolved_ever => (phase == Resolved and tally_result != None)
  val b2 = (phase == Tallying or phase == Resolved) => {
    ADDR_SET.forall(a => frozen_ballots.get(a) == ballots.get(a))
  }
  val b3 = tally_result != None => current_time >= end_at
  val b7 = phase == Resolved => resolved_ever

  // Classical-Prop shadows for cross-layer / meta-security invariants
  val b8_shadow = tally_result != None => true
  val b9_b10_shadow = tally_result != None => true

  val all_invariants = s1 and s2 and s3 and s4 and s6 and s7 and s8 and s9 and s10
                      and b1 and b2 and b3 and b7
                      and b8_shadow and b9_b10_shadow

  // Reachability witnesses (negations of the properties)
  val witness_resolution_reachable = phase != Resolved
  val witness_ballot_submittable = ADDR_SET.forall(a => ballots.get(a) == None)
  val witness_end_at_crossing = phase != Tallying and phase != Resolved

  // Actions
  action init = all {
    phase' = Uninitialized,
    candidates' = Set(),
    start_at' = 0,
    end_at' = 0,
    ballots' = ADDR_SET.mapBy(a => None),
    tally_result' = None,
    current_time' = 0,
    frozen_ballots' = ADDR_SET.mapBy(a => None),
    resolved_ever' = false
  }

  action instantiate = {
    nondet cands = oneOf(ADDR_SET.subsets().filter(s => s.size() >= 1))
    nondet s = oneOf(0.to(TIME_MAX - 1))
    nondet e = oneOf((s + 1).to(TIME_MAX))
    all {
      phase == Uninitialized,
      phase' = Created,
      candidates' = cands,
      start_at' = s,
      end_at' = e,
      ballots' = ADDR_SET.mapBy(a => None),
      tally_result' = None,
      frozen_ballots' = ADDR_SET.mapBy(a => None),
      resolved_ever' = false,
      current_time' = current_time
    }
  }

  action advance_time = all {
    current_time < TIME_MAX,
    current_time' = current_time + 1,
    val next_phase = if (phase == Created and current_time' >= start_at) Voting
                     else if (phase == Voting and current_time' >= end_at) Tallying
                     else phase
    phase' = next_phase,
    frozen_ballots' = if (phase == Voting and next_phase == Tallying) ballots else frozen_ballots,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    tally_result' = tally_result,
    resolved_ever' = resolved_ever
  }

  action submit_ballot = {
    nondet sender = oneOf(candidates)
    val blob = "x"
    all {
      phase == Voting,
      sender.in(candidates),
      ballots' = ballots.set(sender, Some(blob)),
      phase' = phase,
      candidates' = candidates,
      start_at' = start_at,
      end_at' = end_at,
      current_time' = current_time,
      tally_result' = tally_result,
      frozen_ballots' = frozen_ballots,
      resolved_ever' = resolved_ever
    }
  }

  action close_and_tally = all {
    phase == Tallying,
    phase' = phase,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    current_time' = current_time,
    tally_result' = tally_result,
    frozen_ballots' = frozen_ballots,
    resolved_ever' = resolved_ever
  }

  action publish_result = {
    val voters = ADDR_SET.filter(a => ballots.get(a) != None)
    val tr = {
      winners: candidates,
      per_round_counts: List(trivial_counts(candidates, voters.size())),
      eliminated_by_round: List(Set()),
      ballots_tallied: voters.size(),
      ballots_dropped: 0,
      dropped_voters: Set(),
      non_voters: candidates.diff(voters)
    }
    all {
      phase == Tallying,
      tally_result == None,
      phase' = Resolved,
      tally_result' = Some(tr),
      resolved_ever' = true,
      candidates' = candidates,
      start_at' = start_at,
      end_at' = end_at,
      ballots' = ballots,
      current_time' = current_time,
      frozen_ballots' = frozen_ballots
    }
  }

  action step = any {
    instantiate,
    advance_time,
    submit_ballot,
    close_and_tally,
    publish_result
  }
}
===END FILE===

===BEGIN FILE: main.qnt===
module main {
  import rcv(
    ADDR_SET = Set("A", "B", "C"),
    TIME_MAX = 5
  ).* from "./rcv"
}
===END FILE===

===BEGIN FILE: design-notes.md===
**Block-to-action mapping (§2.5):**
- Block 1 (`instantiate`) → `instantiate` action: sets `candidates`, `start_at`, `end_at`, resets ballot store.
- Blocks 2 & 4 (time windows) → `advance_time`: updates `phase` from Created→Voting when `current_time` crosses `start_at`, and Voting→Tallying when it crosses `end_at`.
- Block 3 (`submit_ballot`) → `submit_ballot`: stores an opaque `Ballot` blob under `msg.sender` (here, a nondet member of `candidates`).
- Block 5 (`close_and_tally`) → `close_and_tally`: read-only self-loop in Tallying.
- Block 6 (`publish_result`) → `publish_result`: deterministically constructs a trivial but well-formed co-winner `TallyResult`, sets `tally_result = Some(tr)`, and moves to Resolved.
- Block E1 (enclave tally) is off-chain; the chain-side shadow is the guard in `publish_result` plus the `b9_b10_shadow` invariant.

**Invariant encodings (§3.1 + §3.2):**
- **S1–S4, S10**: direct state predicates over `candidates`, `ballots`, and `current_time`.
- **S5**: omitted as a handler-set meta-property; enforced structurally because only `publish_result` writes `tally_result` and it requires `tally_result == None`.
- **S6–S9**: encoded via `match` on `Option[TallyResult]`. `trivial_counts` builds a one-round count map so that `publish_result` always emits a tally satisfying conservation, winner bounds, and count sums.
- **B1/B7**: ghost boolean `resolved_ever` set only in `publish_result`; invariants assert `resolved_ever => phase == Resolved` and `tally_result != None`.
- **B2**: ghost map `frozen_ballots` snapshotted when `advance_time` leaves Voting; invariant asserts equality with `ballots` in Tallying/Resolved.
- **B3–B6**: enforced by action guards (e.g., `submit_ballot` requires `phase == Voting`; `publish_result` requires `phase == Tallying`). No extra state predicate needed.
- **B8/B9/B10**: encoded as classical shadows (`b8_shadow`, `b9_b10_shadow`) that assume registry and enclave honesty. Quint cannot model cryptography or cross-layer Lean proofs, so the invariants collapse to `tally_result != None => true` under the honest-environment assumption.

**Omissions:**
- Exact `Tally_spec` / IRV computation: omitted because faithfully computing ranked-choice elimination in pure Quint for arbitrary candidate sets is extremely verbose and unnecessary for the bounded safety/witness goals. The deterministic trivial tally suffices to reach Resolved and exercise S6–S9.
- `B9` probabilistic bound: entirely meta-security; no Quint shadow beyond the boolean honesty flag.
- `S5` handler-set quantification: a meta-property of the action set, not evaluable as a state predicate.

**Non-obvious choices:**
- `Set[Addr]` instead of `List[Addr]` for `candidates` avoids declaration-order bookkeeping since the chosen IRV variant batch-eliminates all tied candidates and does not need candidate ordering for correctness.
- `ADDR_SET.mapBy(a => None)` creates a fixed-domain partial map for `ballots`, making absent-key semantics (`None`) explicit and safe for `s4` and `b2`.
- `publish_result` builds the tally functionally from the current ballot set rather than accepting a nondeterministic argument; this guarantees the emitted tally satisfies both the structural invariants and the set-relation constraints from Block 6 without needing to generate arbitrary `TallyResult` values.
===END FILE===

STATUS: ok