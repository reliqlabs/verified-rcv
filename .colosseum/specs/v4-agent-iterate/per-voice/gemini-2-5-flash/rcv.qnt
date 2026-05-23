module rcv {
  type Addr = str
  type Timestamp = int
  type PubKey = str
  type PrivKey = str // Opaque type for off-chain use
  type Bytes32 = str // Opaque type for SHA-256 output

  // TallyResult schema
  type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, int]], // Nat encoded as int
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: int, // Nat encoded as int
    ballots_dropped: int, // Nat encoded as int
    dropped_voters: List[Addr],
    non_voters: List[Addr],
  }

  // State variables (contract storage)
  var candidates: List[Addr]
  var start_at: Timestamp
  var end_at: Timestamp
  var enclave_pubkey: PubKey
  var ballots: Map[Addr, List[int]] // Map<Addr, Vec<u8>> encoded as Map<Addr, List[int]>
  var tally_result: { present: bool, value: TallyResult } // Option<TallyResult>

  // Derived states
  var current_time: Timestamp
  def is_created: bool = current_time < start_at
  def is_voting: bool = start_at <= current_time and current_time < end_at
  def is_tallying: bool = current_time >= end_at and not(tally_result.present)
  def is_resolved: bool = tally_result.present

  // Initial state
  const CANDIDATES: List[Addr]
  const START_AT: Timestamp
  const END_AT: Timestamp
  const ENCLAVE_PUBKEY: PubKey

  action init = all {
    candidates' = CANDIDATES,
    start_at' = START_AT,
    end_at' = END_AT,
    enclave_pubkey' = ENCLAVE_PUBKEY,
    ballots' = Map(),
    tally_result' = { present: false, value: emptyTallyResult() },
    current_time' = 0,
  }

  // Helper for TallyResult placeholder
  pure def emptyTallyResult(): TallyResult = {
    winners: List(),
    per_round_counts: List(),
    eliminated_by_round: List(),
    ballots_tallied: 0,
    ballots_dropped: 0,
    dropped_voters: List(),
    non_voters: List(),
  }

  // Block 1: instantiate (handled by init action in main.qnt)

  // Block 2: time advances to start_at (implicit)
  action advance_to_voting(new_time: Timestamp) = all {
    current_time' = new_time,
    new_time >= start_at,
    new_time < end_at,
    is_created, // Must be in Created state
    // No other state changes
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    tally_result' = tally_result,
  }

  // Block 3: submit_ballot
  action submit_ballot(sender: Addr, encrypted_preferences: List[int]) = all {
    is_voting,
    sender.in(candidates),
    encrypted_preferences.size() > 0,
    ballots' = ballots.set(sender, encrypted_preferences),
    // No other state changes
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    tally_result' = tally_result,
    current_time' = current_time,
  }

  // Block 4: time advances to end_at (implicit)
  action advance_to_tallying(new_time: Timestamp) = all {
    current_time' = new_time,
    new_time >= end_at,
    is_voting, // Must be in Voting state
    // No other state changes
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    tally_result' = tally_result,
  }

  // Block 5: close_and_tally (no state change, emits event)
  action close_and_tally = all {
    is_tallying,
    // No state changes
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    tally_result' = tally_result,
    current_time' = current_time,
  }

  // Block 6: publish_result
  action publish_result(tally: TallyResult) = all {
    is_tallying,
    // Well-formedness checks (simplified for now, will expand after typecheck)
    tally.winners.size() >= 1,
    tally.winners.size() <= candidates.size(),
    tally_result' = { present: true, value: tally },
    // No other state changes
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    current_time' = current_time,
  }

  // Invariants (simplified for initial typecheck)
  val S1_non_empty_candidates = candidates.size() >= 1
  val S2_distinct_candidates = candidates.distinct().size() == candidates.size()
  val S3_well_ordered_voting_window = start_at < end_at
  val S4_ballot_keys_are_candidates = ballots.keys().forall(k => k.in(candidates))
  val S5_terminality_of_resolution = always (tally_result.present => next.tally_result.present and next.tally_result.value == tally_result.value) // This is B1, S5 is a handler-set property
  val S6_winner_well_formedness = not(tally_result.present) or (tally_result.value.winners.forall(w => w.in(candidates)) and tally_result.value.winners.size() >= 1 and tally_result.value.winners.size() <= candidates.size())
  val S7_tally_count_conservation = not(tally_result.present) or (tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + tally_result.value.non_voters.size() == candidates.size())
  val S10_resolution_implies_past_end_at = not(tally_result.present) or current_time >= end_at

  val B1_tally_result_monotone_once_set = always (tally_result.present => next.tally_result.present and next.tally_result.value == tally_result.value)
  val B2_no_late_ballots = always (current_time >= end_at => ballots' == ballots)
  val B3_no_premature_tally = always (not(tally_result.present) and next.tally_result.present => current_time >= end_at)
  val B4_no_premature_voting = always (ballots /= ballots' => start_at <= current_time and current_time < end_at)
  val B7_terminal_state_immutability = always (is_resolved => always is_resolved)

  val all_invariants = all {
    S1_non_empty_candidates,
    S2_distinct_candidates,
    S3_well_ordered_voting_window,
    S4_ballot_keys_are_candidates,
    S5_terminality_of_resolution,
    S6_winner_well_formedness,
    S7_tally_count_conservation,
    S10_resolution_implies_past_end_at,
    B1_tally_result_monotone_once_set,
    B2_no_late_ballots,
    B3_no_premature_tally,
    B4_no_premature_voting,
    B7_terminal_state_immutability,
  }

  // Witness specs (negations of reachability properties)
  val witness_resolution_reachable = not(is_resolved)
  val witness_ballot_submittable = not(is_voting)
  val witness_end_at_crossing = not(is_tallying)
}
