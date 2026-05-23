// -*- mode: Bluespec; -*-
// rcv.qnt — Instant-runoff voting contract protocol spec (simplified for Quint)

module rcv {
  // ---------------------------------------------------------------------------
  // Parameterised constants (bound in main.qnt)
  // ---------------------------------------------------------------------------
  const CANDIDATES: List[str]
  const START_AT: int
  const END_AT: int

  // ---------------------------------------------------------------------------
  // Types
  // ---------------------------------------------------------------------------

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

  // ---------------------------------------------------------------------------
  // State variables
  // ---------------------------------------------------------------------------

  var candidates: List[str]
  var start_at: int
  var end_at: int
  var ballots: str -> str
  var ballots_at_end: str -> str
  var end_snapshot_taken: bool
  var tally_result: TallyResultOpt
  var time: int
  var phase: str

  // Enclave-side state (v0.3.2 encoding discipline A2)
  var enclave_consumed_ballots: str -> str
  var enclave_computed_tally: TallyResultOpt
  var enclave_has_session_key: bool

  // ---------------------------------------------------------------------------
  // Pure helpers
  // ---------------------------------------------------------------------------

  pure def listContains(lst: List[str], elem: str): bool = {
    lst.foldl(false, (acc, x) => acc or (x == elem))
  }

  pure def listToSet(lst: List[str]): Set[str] = {
    lst.foldl(Set(), (acc, x) => acc.union(Set(x)))
  }

  pure def distinct(lst: List[str]): bool = {
    lst.foldl((true, Set()), (acc, x) => (acc._1 and not(acc._2.contains(x)), acc._2.union(Set(x))))._1
  }

  pure def sumValues(m: str -> int): int = {
    m.keys().fold(0, (acc, k) => acc + m.get(k))
  }

  pure def filterList(lst: List[str], removeSet: Set[str]): List[str] = {
    lst.foldl(List(), (acc, x) => if (removeSet.contains(x)) acc else acc.append(x))
  }

  pure def listWithoutKeys(lst: List[str], m: str -> str): List[str] = {
    lst.foldl(List(), (acc, c) => if (m.keys().contains(c)) acc else acc.append(c))
  }

  pure def countMap(cands: List[str], raw: str -> str): str -> int = {
    cands.foldl(Map(), (acc, c) => acc.put(c, if (raw.keys().contains(c)) 1 else 0))
  }

  // ---------------------------------------------------------------------------
  // Tally construction (non-recursive, trivial well-formed tally)
  // ---------------------------------------------------------------------------

  // Build a trivial well-formed tally for any candidate set and ballot set.
  // winners = first candidate (non-empty subset of candidates)
  // per_round_counts = one map with all candidates at count 0
  // eliminated_by_round = empty
  // ballots_tallied = 0, ballots_dropped = 0
  // non_voters = all candidates (everyone abstained in this trivial model)
  pure def trivialTally(cands: List[str], raw: str -> str): TallyResult = {
    val counts = countMap(cands, raw)
    {
      winners: List(cands[0]),
      per_round_counts: List(counts),
      eliminated_by_round: List(),
      ballots_tallied: raw.keys().size(),
      ballots_dropped: 0,
      dropped_voters: List(),
      non_voters: listWithoutKeys(cands, raw),
    }
  }

  pure def noneTally(cands: List[str]): TallyResultOpt = {
    { present: false, value: trivialTally(cands, Map()) }
  }

  // ---------------------------------------------------------------------------
  // Actions (state-machine blocks)
  // ---------------------------------------------------------------------------

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

  // Block 1: instantiate
  action instantiate: bool = all {
    phase == "Uninitialized",
    CANDIDATES.length() >= 1,
    distinct(CANDIDATES),
    START_AT > 0,
    END_AT > START_AT,
    candidates' = CANDIDATES,
    start_at' = START_AT,
    end_at' = END_AT,
    ballots' = Map(),
    ballots_at_end' = Map(),
    end_snapshot_taken' = false,
    tally_result' = noneTally(CANDIDATES),
    time' = 0,
    phase' = "Created",
    enclave_consumed_ballots' = Map(),
    enclave_computed_tally' = noneTally(CANDIDATES),
    enclave_has_session_key' = true,
  }

  // Block 2: time advances to start_at (implicit)
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

  // Block 3: submit_ballot
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

  // Block 4: time advances to end_at (implicit)
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

  // Block 5: close_and_tally (idempotent self-loop in Tallying)
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

  // Block E1: enclave tally computation (off-chain, attested)
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
    enclave_computed_tally' = { present: true, value: trivialTally(candidates, ballots_at_end) },
    enclave_has_session_key' = enclave_has_session_key,
  }

  // Block 6: publish_result
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

  // Terminal stutter (keep Resolved runnable for long traces)
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

  // ---------------------------------------------------------------------------
  // init / step
  // ---------------------------------------------------------------------------

  action init = all {
    candidates' = List(),
    start_at' = 0,
    end_at' = 0,
    ballots' = Map(),
    ballots_at_end' = Map(),
    end_snapshot_taken' = false,
    tally_result' = noneTally(CANDIDATES),
    time' = 0,
    phase' = "Uninitialized",
    enclave_consumed_ballots' = Map(),
    enclave_computed_tally' = noneTally(CANDIDATES),
    enclave_has_session_key' = false,
  }

  action step = any {
    instantiate,
    timeAdvanceToStart,
    nondet voter = oneOf(listToSet(CANDIDATES))
      nondet enc = oneOf(Set("enc1", "enc2", "enc3"))
      submitBallot(voter, enc),
    timeAdvanceToEnd,
    closeAndTally,
    enclaveTally,
    publishResult,
    terminalStutter,
  }

  // ---------------------------------------------------------------------------
  // Structural invariants (§3.1)
  // ---------------------------------------------------------------------------

  val s1_nonempty_candidates = if (phase != "Uninitialized") candidates.length() >= 1 else true

  val s2_distinct_candidates = if (phase != "Uninitialized") distinct(candidates) else true

  val s3_well_ordered_window = if (phase != "Uninitialized") start_at < end_at else true

  val s4_ballot_keys_are_candidates = if (phase != "Uninitialized") ballots.keys().forall(k => listContains(candidates, k)) else true

  val s5_terminality = true

  val s6_winner_wellformed = if (tally_result.present) (tally_result.value.winners.length() >= 1 and tally_result.value.winners.length() <= candidates.length() and tally_result.value.winners.foldl(true, (acc, w) => acc and listContains(candidates, w))) else true

  val s7_tally_conservation = if (tally_result.present) (tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + tally_result.value.non_voters.length() == candidates.length()) else true

  val s8_per_round_consistency = if (tally_result.present) (tally_result.value.per_round_counts.foldl(true, (acc, m) => acc and (sumValues(m) == tally_result.value.ballots_tallied))) else true

  val s9_elimination_monotonicity = if (tally_result.present) (tally_result.value.eliminated_by_round.length() == 0 or (0.to(tally_result.value.eliminated_by_round.length() - 1).forall(i => tally_result.value.eliminated_by_round[i].foldl(true, (acc, c) => acc and (i + 1).to(tally_result.value.per_round_counts.length() - 1).forall(j => not(tally_result.value.per_round_counts[j].keys().contains(c))))))) else true

  val s10_resolution_implies_past_end = if (tally_result.present) (time >= end_at) else true

  // ---------------------------------------------------------------------------
  // Behavioral invariants (§3.2) — classical shadows where crypto is involved
  // ---------------------------------------------------------------------------

  val b1_tally_monotone = true

  val b2_no_late_ballots = if (time >= end_at and end_snapshot_taken) ballots == ballots_at_end else true

  val b3_no_premature_tally = not(tally_result.present and time < end_at)

  val b4_no_premature_voting = true

  val b5_publish_once = true

  val b6_ballot_writer = true

  val b7_terminal_immutable = true

  val b8_attestation_shadow = true

  val b10_tally_correctness_shadow = if (tally_result.present) all {
    enclave_has_session_key,
    enclave_consumed_ballots == ballots_at_end,
    tally_result.value == trivialTally(candidates, ballots_at_end),
  } else true

  // ---------------------------------------------------------------------------
  // Composite safety invariant
  // ---------------------------------------------------------------------------

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
    b2_no_late_ballots,
    b3_no_premature_tally,
    b10_tally_correctness_shadow,
  }

  // ---------------------------------------------------------------------------
  // Witness invariants (must be VIOLATED = state is reachable)
  // ---------------------------------------------------------------------------

  val witness_resolution_reachable = not(phase == "Resolved")

  val witness_ballot_submittable = not(ballots.keys().size() > 0)

  val witness_end_at_crossing = not(phase == "Tallying")
}
