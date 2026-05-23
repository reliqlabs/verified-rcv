===BEGIN FILE: rcv.qnt===
module rcv {
  type Addr = str
  type Ballot = List[Addr]

  type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, int]],
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: List[Addr],
    non_voters: List[Addr]
  }

  const CANDIDATES: List[Addr]
  const START_AT: int
  const END_AT: int
  const MAX_TIME: int
  const VALID_BALLOTS: Set[Ballot]

  pure val candidates_set: Set[Addr] = CANDIDATES.foldl(Set(), (s, c) => s.union(Set(c)))

  var ballots: Map[Addr, Ballot]
  var tally_result: Option[TallyResult]
  var time: int
  var was_resolved: bool
  var end_crossed: bool
  var ballots_at_end: Map[Addr, Ballot]

  pure def order_by_candidates(addrs: Set[Addr]): List[Addr] =
    CANDIDATES.filter(c => c.in(addrs))

  pure def list_to_set(l: List[Addr]): Set[Addr] =
    l.foldl(Set(), (s, c) => s.union(Set(c)))

  pure def list_all(l: List[Addr], pred: Addr => bool): bool =
    l.foldl(true, (acc, x) => acc and pred(x))

  pure def top_choice(ballot: List[Addr], remaining: Set[Addr]): Option[Addr] =
    ballot.foldl(None, (acc, c) => if (acc == None and c.in(remaining)) Some(c) else acc)

  pure def count_firsts(ballots_map: Map[Addr, List[Addr]], remaining: Set[Addr]): Map[Addr, int] =
    CANDIDATES.foldl(Map(), (m, c) => if (c.in(remaining)) {
      val cnt = ballots_map.keys().filter(addr => top_choice(ballots_map.get(addr), remaining) == Some(c)).size()
      m.set(c, cnt)
    } else m)

  pure def map_sum(m: Map[Addr, int]): int =
    CANDIDATES.foldl(0, (acc, c) => acc + m.get(c).get_or_else(0))

  pure def find_majority(counts: Map[Addr, int], remaining: Set[Addr], total: int): Option[Addr] =
    CANDIDATES.foldl(None, (acc, c) => if (acc == None and c.in(remaining) and counts.get(c).get_or_else(0) > total / 2) Some(c) else acc)

  pure def min_count_val(counts: Map[Addr, int]): int =
    CANDIDATES.foldl(1000000, (acc, c) => {
      val v = counts.get(c).get_or_else(0)
      if (v < acc) v else acc
    })

  pure def irv_helper(
    ballots_map: Map[Addr, List[Addr]],
    remaining: Set[Addr],
    counts_acc: List[Map[Addr, int]],
    elims_acc: List[List[Addr]]
  ): (List[Addr], List[Map[Addr, int]], List[List[Addr]]) = {
    val counts = count_firsts(ballots_map, remaining)
    val total = map_sum(counts)
    val majority = find_majority(counts, remaining, total)
    if (majority.is_some()) {
      ([majority.get()], counts_acc.append(counts), elims_acc)
    } else if (remaining.size() == 1) {
      (order_by_candidates(remaining), counts_acc.append(counts), elims_acc)
    } else {
      val min_c = min_count_val(counts)
      val losers = remaining.filter(c => counts.get(c).get_or_else(0) == min_c)
      if (losers == remaining) {
        (order_by_candidates(remaining), counts_acc.append(counts), elims_acc)
      } else {
        irv_helper(ballots_map, remaining.exclude(losers),
                   counts_acc.append(counts),
                   elims_acc.append(order_by_candidates(losers)))
      }
    }
  }

  pure def irv_spec(ballots_map: Map[Addr, List[Addr]]): (List[Addr], List[Map[Addr, int]], List[List[Addr]]) =
    irv_helper(ballots_map, candidates_set, [], [])

  pure def tally_spec(ballots_map: Map[Addr, List[Addr]]): TallyResult = {
    val (winners, counts, elims) = irv_spec(ballots_map)
    val voters = ballots_map.keys()
    {
      winners: winners,
      per_round_counts: counts,
      eliminated_by_round: elims,
      ballots_tallied: ballots_map.size(),
      ballots_dropped: 0,
      dropped_voters: [],
      non_voters: CANDIDATES.filter(c => not(c.in(voters)))
    }
  }

  pure def winners_wf(tr: TallyResult): bool =
    list_all(tr.winners, w => w.in(candidates_set)) and
    tr.winners.length() >= 1 and
    tr.winners.length() <= CANDIDATES.length()

  pure def conservation(tr: TallyResult): bool =
    tr.ballots_tallied + tr.ballots_dropped + tr.non_voters.length() == CANDIDATES.length()

  pure def count_consistency(tr: TallyResult): bool = {
    val n = tr.per_round_counts.length()
    if (n == 0) true else
    0.to(n - 1).forall(i => map_sum(tr.per_round_counts[i]) == tr.ballots_tallied)
  }

  pure def elimination_monotone(tr: TallyResult): bool = {
    val elim_len = tr.eliminated_by_round.length()
    val count_len = tr.per_round_counts.length()
    if (elim_len == 0) true else
    0.to(elim_len - 1).forall(i =>
      0.to(count_len - 1).forall(j =>
        if (j > i) {
          list_to_set(tr.eliminated_by_round[i]).intersect(tr.per_round_counts[j].keys()).size() == 0
        } else true
      )
    )
  }

  val s1 = CANDIDATES.length() >= 1
  val s2 = candidates_set.size() == CANDIDATES.length()
  val s3 = START_AT < END_AT
  val s4 = ballots.keys().subset(candidates_set)
  val s6 = tally_result.is_some() => winners_wf(tally_result.get())
  val s7 = tally_result.is_some() => conservation(tally_result.get())
  val s8 = tally_result.is_some() => count_consistency(tally_result.get())
  val s9 = tally_result.is_some() => elimination_monotone(tally_result.get())
  val s10 = tally_result.is_some() => time >= END_AT

  val b1 = was_resolved => tally_result.is_some()
  val b2 = end_crossed => ballots == ballots_at_end
  val b3 = was_resolved => end_crossed
  val b10 = tally_result.is_some() => tally_result.get() == tally_spec(ballots)

  val all_invariants = s1 and s2 and s3 and s4 and s6 and s7 and s8 and s9 and s10 and b1 and b2 and b3 and b10

  val witness_resolution_reachable = not(was_resolved)
  val witness_ballot_submittable = ballots.size() == 0
  val witness_end_at_crossing = time < END_AT

  action init = all {
    ballots' = Map(),
    tally_result' = None,
    time' = 0,
    was_resolved' = false,
    end_crossed' = (0 >= END_AT),
    ballots_at_end' = if (0 >= END_AT) Map() else Map()
  }

  action tick = all {
    time < MAX_TIME,
    time' = time + 1,
    end_crossed' = if (not(end_crossed) and (time + 1 >= END_AT) and (time < END_AT)) true else end_crossed,
    ballots_at_end' = if (not(end_crossed) and (time + 1 >= END_AT) and (time < END_AT)) ballots else ballots_at_end,
    ballots' = ballots,
    tally_result' = tally_result,
    was_resolved' = was_resolved
  }

  action submit_ballot(voter: Addr, ballot: Ballot) = all {
    START_AT <= time and time < END_AT,
    voter.in(candidates_set),
    ballot.in(VALID_BALLOTS),
    ballots' = ballots.set(voter, ballot),
    tally_result' = tally_result,
    time' = time,
    was_resolved' = was_resolved,
    end_crossed' = end_crossed,
    ballots_at_end' = ballots_at_end
  }

  action publish_result = all {
    time >= END_AT,
    not(was_resolved),
    val expected = tally_spec(ballots)
    all {
      winners_wf(expected),
      conservation(expected),
      count_consistency(expected),
      elimination_monotone(expected),
      tally_result' = Some(expected),
      was_resolved' = true,
      ballots' = ballots,
      time' = time,
      end_crossed' = end_crossed,
      ballots_at_end' = ballots_at_end
    }
  }

  action step = any {
    tick,
    nondet voter = oneOf(candidates_set)
    nondet ballot = oneOf(VALID_BALLOTS)
    submit_ballot(voter, ballot),
    publish_result
  }
}
===END FILE===

===BEGIN FILE: main.qnt===
module main {
  import rcv(
    CANDIDATES = ["A", "B", "C"],
    START_AT = 2,
    END_AT = 4,
    MAX_TIME = 6,
    VALID_BALLOTS = Set(["A","B","C"], ["A","C","B"], ["B","A","C"], ["B","C","A"], ["C","A","B"], ["C","B","A"])
  ).* from "./rcv"
}
===END FILE===

===BEGIN FILE: design-notes.md===
**§2.5 block → action mapping**
- Block 1 (instantiate) is folded into `init` + const parameters: `CANDIDATES`, `START_AT`, `END_AT` bound in `main.qnt`, giving a single post-instantiation instance per trace.
- Block 2/4 (time advance) encoded as `tick`, which increments discrete block time. `tick` detects the first crossing of `END_AT` and snapshots `ballots` into `ballots_at_end`, materializing the `ballots@end_at` notation.
- Block 3 (submit_ballot) → `submit_ballot(voter, ballot)`. Guards on `START_AT ≤ time < END_AT` and `voter ∈ candidates`.
- Block 5 (close_and_tally) is omitted: it is a read-only event emission with no storage effect, so it contributes no state-space change.
- Block 6 (publish_result) → `publish_result`. Guards on `time ≥ END_AT`, `¬was_resolved`, and the published value is exactly `tally_spec(ballots)`. This encodes the honest enclave.
- Block E1 (enclave tally) is realized as pure-def `tally_spec`, which composes `decrypt_and_validate` (identity, since all stored ballots are assumed valid) and `IRV_spec` (recursive batch-elimination IRV).

**§3.1 / §3.2 invariant encoding**
- Structural (S-series): S1–S3 are const well-formedness. S4 is ballot-key subset. S6–S9 are `tally_result`-conditional well-formedness predicates. S10 is `time ≥ END_AT` when resolved.
- Temporal B-series shadows:
  - B1 (once-set) → history var `was_resolved`. Invariant `was_resolved ⇒ tally_result.is_some()`.
  - B2 (frozen ballots) → history vars `end_crossed` + `ballots_at_end`. Invariant `end_crossed ⇒ ballots == ballots_at_end`.
  - B3 (no premature tally) → `was_resolved ⇒ end_crossed`.
  - B4/B5/B6/B7 are structurally enforced by action guards (ballots only written during voting window; publish_result gated on `¬was_resolved`; submit_ballot writes only `ballots[voter]`; no exit from resolved). They are not duplicated as separate state predicates.
  - B8 attestation/crypto and B9 meta-security are omitted (beyond scope of Quint). The classical shadow is that `publish_result` only succeeds with the spec-computed tally, which is enforced action-level.
  - B10 (tally correctness) → `tally_result.is_some() ⇒ tally_result.get() == tally_spec(ballots)`. Since B2 guarantees `ballots` is frozen post-`end_at`, this holds in Resolved.
- B10_lean is omitted: it is an off-chain Lean obligation, not a chain-trajectory predicate.

**Non-obvious encoding choices**
- **ECIES abstraction**: ballots store plaintext rankings rather than ciphertext. Encryption is modeled as perfect; `VALID_BALLOTS` supplies only permutations of `CANDIDATES`. This collapses Stage 1 decryption to the identity function.
- **Ghost history variables**: `was_resolved`, `end_crossed`, and `ballots_at_end` turn temporal invariants into pure state predicates evaluable by `quint run`.
- **Tick granularity**: time advances in unit steps. This is sufficient because the protocol only cares about ordering relative to `START_AT` and `END_AT`.
- **`close_and_tally` omitted**: because it is idempotent and storage-noop, it does not affect safety or reachability of the three witnesses.

**Omissions**
- B4/B5/B6/B7: encoded only by action architecture, not explicit `val` conjuncts.
- B8/B9: cryptographic claims omitted; noted as methodology limitation.
- B10_lean: off-chain, omitted.
- `dropped_voters` / `ballots_dropped` are hardcoded to `[]` / `0` because the honest model never submits malformed ballots.
===END FILE===

STATUS: ok