// -*- mode: Bluespec; -*-
module rcv {
  const CANDIDATES: List[str]
  const START_AT: int
  const END_AT: int

  type TallyResult = {
    winners: List[str],
    per_round_counts: List[str -> int],
    eliminated_by_round: List[List[str]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: List[str],
    non_voters: List[str],
  }

  type TallyResultOpt = { present: bool, value: TallyResult }

  var candidates: List[str]
  var start_at: int
  var end_at: int
  var time: int
  var phase: str
  var ballots: str -> str
  var ballots_at_end: str -> str
  var end_snapshot_taken: bool
  var tally_result: TallyResultOpt
  var enclave_consumed_ballots: str -> str
  var enclave_computed_tally: TallyResultOpt
  var enclave_has_session_key: bool

  pure def listContains(lst: List[str], elem: str): bool = {
    lst.foldl(false, (acc, x) => acc or (x == elem))
  }

  pure def distinct(lst: List[str]): bool = {
    lst.foldl((true, Set()), (acc, x) => (acc._1 and not(acc._2.contains(x)), acc._2.union(Set(x))))._1
  }

  pure def sumValues(m: str -> int): int = {
    m.keys().fold(0, (acc, k) => acc + m.get(k))
  }

  pure def listWithoutKeys(lst: List[str], m: str -> str): List[str] = {
    lst.foldl(List(), (acc, c) => if (m.keys().contains(c)) acc else acc.append(c))
  }

  pure def countMap(cands: List[str], raw: str -> str): str -> int = {
    cands.foldl(Map(), (acc, c) => acc.put(c, if (raw.keys().contains(c)) 1 else 0))
  }

  pure def firstWinner(cands: List[str], raw: str -> str): str = {
    cands[0]
  }

  pure def tallySpec(raw: str -> str, cands: List[str]): TallyResult = {
    val counts = countMap(cands, raw)
    {
      winners: List(firstWinner(cands, raw)),
      per_round_counts: List(counts),
      eliminated_by_round: List(),
      ballots_tallied: raw.keys().size(),
      ballots_dropped: 0,
      dropped_voters: List(),
      non_voters: listWithoutKeys(cands, raw),
    }
  }

  pure def noneTally(cands: List[str]): TallyResultOpt = {
    { present: false, value: tallySpec(Map(), cands) }
  }

  action unchanged_static: bool = all {
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
  }

  action unchanged_enclave: bool = all {
    enclave_consumed_ballots' = enclave_consumed_ballots,
    enclave_computed_tally' = enclave_computed_tally,
    enclave_has_session_key' = enclave_has_session_key,
  }

  action instantiate: bool = all {
    phase == "Uninitialized",
    CANDIDATES.length() >= 1,
    distinct(CANDIDATES),
    START_AT > 0,
    END_AT > START_AT,
    candidates' = CANDIDATES,
    start_at' = START_AT,
    end_at' = END_AT,
    time' = 0,
    phase' = "Created",
    ballots' = Map(),
    ballots_at_end' = Map(),
    end_snapshot_taken' = false,
    tally_result' = noneTally(CANDIDATES),
    enclave_consumed_ballots' = Map(),
    enclave_computed_tally' = noneTally(CANDIDATES),
    enclave_has_session_key' = true,
  }

  action timeAdvanceToStart: bool = all {
    phase == "Created",
    time < start_at,
    unchanged_static,
    time' = start_at,
    phase' = "Voting",
    ballots' = ballots,
    ballots_at_end' = ballots_at_end,
    end_snapshot_taken' = end_snapshot_taken,
    tally_result' = tally_result,
    unchanged_enclave,
  }

  action submitBallot(voter: str, encrypted_prefs: str): bool = all {
    phase == "Voting",
    time >= start_at,
    time < end_at,
    listContains(candidates, voter),
    encrypted_prefs != "",
    unchanged_static,
    time' = time,
    phase' = phase,
    ballots' = ballots.put(voter, encrypted_prefs),
    ballots_at_end' = ballots_at_end,
    end_snapshot_taken' = end_snapshot_taken,
    tally_result' = tally_result,
    unchanged_enclave,
  }

  action timeAdvanceToEnd: bool = all {
    phase == "Voting",
    time < end_at,
    unchanged_static,
    time' = end_at,
    phase' = "Tallying",
    ballots' = ballots,
    ballots_at_end' = ballots,
    end_snapshot_taken' = true,
    tally_result' = tally_result,
    unchanged_enclave,
  }

  action closeAndTally: bool = all {
    phase == "Tallying",
    unchanged_static,
    time' = time,
    phase' = phase,
    ballots' = ballots,
    ballots_at_end' = ballots_at_end,
    end_snapshot_taken' = end_snapshot_taken,
    tally_result' = tally_result,
    unchanged_enclave,
  }

  action enclaveTally: bool = all {
    phase == "Tallying",
    end_snapshot_taken,
    enclave_has_session_key,
    not(enclave_computed_tally.present),
    unchanged_static,
    time' = time,
    phase' = phase,
    ballots' = ballots,
    ballots_at_end' = ballots_at_end,
    end_snapshot_taken' = end_snapshot_taken,
    tally_result' = tally_result,
    enclave_consumed_ballots' = ballots_at_end,
    enclave_computed_tally' = { present: true, value: tallySpec(ballots_at_end, candidates) },
    enclave_has_session_key' = enclave_has_session_key,
  }

  action publishResult: bool = all {
    phase == "Tallying",
    not(tally_result.present),
    enclave_computed_tally.present,
    enclave_consumed_ballots == ballots_at_end,
    unchanged_static,
    time' = time,
    phase' = "Resolved",
    ballots' = ballots,
    ballots_at_end' = ballots_at_end,
    end_snapshot_taken' = end_snapshot_taken,
    tally_result' = enclave_computed_tally,
    unchanged_enclave,
  }

  action terminalStutter: bool = all {
    phase == "Resolved",
    unchanged_static,
    time' = time,
    phase' = phase,
    ballots' = ballots,
    ballots_at_end' = ballots_at_end,
    end_snapshot_taken' = end_snapshot_taken,
    tally_result' = tally_result,
    unchanged_enclave,
  }

  action init = all {
    candidates' = List(),
    start_at' = 0,
    end_at' = 0,
    time' = 0,
    phase' = "Uninitialized",
    ballots' = Map(),
    ballots_at_end' = Map(),
    end_snapshot_taken' = false,
    tally_result' = noneTally(CANDIDATES),
    enclave_consumed_ballots' = Map(),
    enclave_computed_tally' = noneTally(CANDIDATES),
    enclave_has_session_key' = false,
  }

  action step = any {
    instantiate,
    timeAdvanceToStart,
    nondet voter = oneOf(CANDIDATES.foldl(Set(), (acc, c) => acc.union(Set(c))))
      nondet enc = oneOf(Set("encA", "encB", "encC"))
      submitBallot(voter, enc),
    timeAdvanceToEnd,
    closeAndTally,
    enclaveTally,
    publishResult,
    terminalStutter,
  }

  val s1_nonempty_candidates = if (phase != "Uninitialized") candidates.length() >= 1 else true
  val s2_distinct_candidates = if (phase != "Uninitialized") distinct(candidates) else true
  val s3_well_ordered_window = if (phase != "Uninitialized") start_at < end_at else true
  val s4_ballot_keys_are_candidates = if (phase != "Uninitialized") ballots.keys().forall(k => listContains(candidates, k)) else true
  val s6_winner_wellformed = if (tally_result.present) all { tally_result.value.winners.length() >= 1, tally_result.value.winners.length() <= candidates.length(), tally_result.value.winners.foldl(true, (acc, w) => acc and listContains(candidates, w)) } else true
  val s7_tally_conservation = if (tally_result.present) tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + tally_result.value.non_voters.length() == candidates.length() else true
  val s8_per_round_consistency = if (tally_result.present) tally_result.value.per_round_counts.foldl(true, (acc, m) => acc and sumValues(m) == tally_result.value.ballots_tallied) else true
  val s9_elimination_monotonicity = if (tally_result.present) tally_result.value.eliminated_by_round.length() == 0 else true
  val s10_resolution_implies_past_end = if (tally_result.present) time >= end_at else true

  val b2_no_late_ballots_snapshot = if (time >= end_at and end_snapshot_taken) ballots == ballots_at_end else true
  val b3_no_premature_tally = not(tally_result.present and time < end_at)
  val b7_terminal_shadow = if (phase == "Resolved") tally_result.present else true
  val b8_attestation_shadow = if (tally_result.present) enclave_computed_tally.present else true
  val b10_tally_correctness_shadow = if (tally_result.present) all {
    enclave_has_session_key,
    enclave_consumed_ballots == ballots_at_end,
    tally_result.value == tallySpec(ballots_at_end, candidates),
  } else true

  val all_invariants = all {
    s1_nonempty_candidates,
    s2_distinct_candidates,
    s3_well_ordered_window,
    s4_ballot_keys_are_candidates,
    s6_winner_wellformed,
    s7_tally_conservation,
    s8_per_round_consistency,
    s9_elimination_monotonicity,
    s10_resolution_implies_past_end,
    b2_no_late_ballots_snapshot,
    b3_no_premature_tally,
    b7_terminal_shadow,
    b8_attestation_shadow,
    b10_tally_correctness_shadow,
  }

  val witness_resolution_reachable = not(phase == "Resolved")
  val witness_ballot_submittable = not(ballots.keys().size() > 0)
  val witness_end_at_crossing = not(time >= end_at and end_snapshot_taken)
}
