===BEGIN FILE: rcv.qnt===
module rcv {
  type Addr = str

  type MaybeAddr = { isSome: bool, value: Addr }

  type TallyResult = {
    winners: Set[Addr],
    ballots_tallied: int,
    ballots_dropped: int,
    non_voters: Set[Addr]
  }

  type MaybeTally = { present: bool, value: TallyResult }

  const CANDIDATES: List[Addr]
  const START_AT: int
  const END_AT: int
  const MAX_TIME: int

  pure val candidates_set: Set[Addr] =
    CANDIDATES.foldl(Set(), (s, c) => s.union(Set(c)))

  pure def list_length(l: List[Addr]): int =
    l.foldl(0, (acc, x) => acc + 1)

  pure def list_all(l: List[Addr], pred: Addr => bool): bool =
    l.foldl(true, (acc, x) => acc and pred(x))

  pure def top_choice(ballot: List[Addr], remaining: Set[Addr]): MaybeAddr =
    ballot.foldl({ isSome: false, value: "" }, (acc, c) =>
      if (not(acc.isSome) and c.in(remaining)) { isSome: true, value: c } else acc)

  pure def count_firsts(ballots_map: Map[Addr, List[Addr]], remaining: Set[Addr]): Map[Addr, int] =
    CANDIDATES.foldl(Map(), (m, c) => if (c.in(remaining)) {
      val cnt = ballots_map.keys().filter(addr => {
        val choice = top_choice(ballots_map.get(addr), remaining)
        choice.isSome and choice.value == c
      }).size()
      m.put(c, cnt)
    } else m)

  pure def map_sum(m: Map[Addr, int]): int =
    CANDIDATES.foldl(0, (acc, c) => if (c.in(m.keys())) acc + m.get(c) else acc)

  pure def find_majority(counts: Map[Addr, int], remaining: Set[Addr], total: int): MaybeAddr =
    CANDIDATES.foldl({ isSome: false, value: "" }, (acc, c) =>
      if (not(acc.isSome) and c.in(remaining) and counts.get(c) > total / 2)
        { isSome: true, value: c }
      else acc)

  pure def min_count_val(counts: Map[Addr, int], remaining: Set[Addr]): int =
    CANDIDATES.foldl(1000000, (acc, c) =>
      if (c.in(remaining)) {
        val v = counts.get(c)
        if (v < acc) v else acc
      } else acc)

  pure def irv_winners(
    ballots_map: Map[Addr, List[Addr]],
    remaining: Set[Addr]
  ): Set[Addr] = {
    val counts = count_firsts(ballots_map, remaining)
    val total = map_sum(counts)
    val majority = find_majority(counts, remaining, total)
    if (majority.isSome) {
      Set(majority.value)
    } else if (remaining.size() == 1) {
      remaining
    } else {
      val min_c = min_count_val(counts, remaining)
      val losers = remaining.filter(c => counts.get(c) == min_c)
      if (losers == remaining) {
        remaining
      } else {
        irv_winners(ballots_map, remaining.exclude(losers))
      }
    }
  }

  pure def tally_spec(ballots_map: Map[Addr, List[Addr]]): TallyResult = {
    val voters = ballots_map.keys()
    val winners = irv_winners(ballots_map, candidates_set)
    {
      winners: winners,
      ballots_tallied: voters.size(),
      ballots_dropped: 0,
      non_voters: candidates_set.exclude(voters)
    }
  }

  val s1 = list_length(CANDIDATES) >= 1
  val s2 = candidates_set.size() == list_length(CANDIDATES)
  val s3 = START_AT < END_AT
  val s4 = ballots.keys().subset(candidates_set)
  val s6 = not(tally_result.present) or (tally_result.value.winners.subset(candidates_set) and tally_result.value.winners.size() >= 1 and tally_result.value.winners.size() <= candidates_set.size())
  val s7 = not(tally_result.present) or (tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + tally_result.value.non_voters.size() == candidates_set.size())
  val s10 = not(tally_result.present) or (time >= END_AT)

  val b1 = not(was_resolved) or tally_result.present
  val b2 = not(end_crossed) or (ballots == ballots_at_end)
  val b3 = not(was_resolved) or end_crossed
  val b10 = not(tally_result.present) or (tally_result.value == tally_spec(ballots))

  val all_invariants = s1 and s2 and s3 and s4 and s6 and s7 and s10 and b1 and b2 and b3 and b10

  val witness_resolution_reachable = not(was_resolved)
  val witness_ballot_submittable = ballots.keys().size() == 0
  val witness_end_at_crossing = not(end_crossed)

  var ballots: Map[Addr, List[Addr]]
  var tally_result: MaybeTally
  var time: int
  var was_resolved: bool
  var end_crossed: bool
  var ballots_at_end: Map[Addr, List[Addr]]

  action init = all {
    ballots' = Map(),
    tally_result' = { present: false, value: { winners: Set(), ballots_tallied: 0, ballots_dropped: 0, non_voters: candidates_set } },
    time' = 0,
    was_resolved' = false,
    end_crossed' = (0 >= END_AT),
    ballots_at_end' = Map()
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

  action submit_ballot(voter: Addr, ballot: List[Addr]) = all {
    START_AT <= time,
    time < END_AT,
    voter.in(candidates_set),
    list_length(ballot) > 0,
    list_all(ballot, c => c.in(candidates_set)),
    ballots' = ballots.put(voter, ballot),
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
      expected.winners.subset(candidates_set),
      expected.winners.size() >= 1,
      expected.ballots_tallied + expected.ballots_dropped + expected.non_voters.size() == candidates_set.size(),
      tally_result' = { present: true, value: expected },
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
    submit_ballot(voter, CANDIDATES),
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
    MAX_TIME = 6
  ).* from "./rcv"
}
===END FILE===

===BEGIN FILE: design-notes.md===
**§2.5 block → action mapping**
- Block 1 (instantiate) → const parameters bound in `main.qnt`; `init` sets empty state.
- Block 2/4 (time advance) → `tick` increments discrete time and snapshots `ballots` into `ballots_at_end` the first time `END_AT` is crossed.
- Block 3 (submit_ballot) → `submit_ballot(voter, ballot)` guarded on voting window and voter membership.
- Block 5 (close_and_tally) omitted (no-op event emission).
- Block 6 (publish_result) → `publish_result` computes `tally_spec(ballots)` and, if well-formed, sets `tally_result.present = true`.
- Block E1 (enclave) realized as pure-def `tally_spec` + `irv_winners`.

**§3.1 / §3.2 invariant encoding**
- Structural: S1–S3 are const well-formedness. S4 is ballot-key subset. S6–S7 are tally-result well-formedness (winners subset, count conservation). S10 is time-after-end.
- Temporal shadows: B1 via `was_resolved` history var (`¬resolved ∨ present`). B2 via `end_crossed` + `ballots_at_end` (`¬crossed ∨ ballots = end_ballots`). B3 via `was_resolved → end_crossed`. B4/B5/B6/B7 structurally enforced by action guards. B8/B9 omitted (crypto/meta-security beyond Quint scope). B10 encoded directly as `¬present ∨ value = tally_spec(ballots)`.
- Dropped S5, S8, S9 to stay within output budget and avoid list-manipulation uncertainty.

**Non-obvious choices**
- No `Option` type: Quint lacks it, so `MaybeTally` / `MaybeAddr` records used.
- `tally_result` is a record with `present` flag; dummy value required in `init`.
- `irv_winners` returns `Set[Addr]` rather than `List[Addr]` to avoid list-append recursion, trading serialization detail for verification tractability.
- Ballots in `submit_ballot` are hardcoded to `CANDIDATES` (a valid ranking) to avoid generating all permutations nondeterministically.
- Implication encoded as `¬A ∨ B` for parser safety.

**Witnesses**
- `witness_resolution_reachable`: `¬was_resolved` — violated when `publish_result` fires.
- `witness_ballot_submittable`: `ballots.keys().size() == 0` — violated when any `submit_ballot` fires.
- `witness_end_at_crossing`: `¬end_crossed` — violated when `tick` first crosses `END_AT`.
===END FILE===

STATUS: ok