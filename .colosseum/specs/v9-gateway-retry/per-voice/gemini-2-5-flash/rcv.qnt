module rcv {
  type Addr = str
  type Timestamp = int
  type PubKey = str
  type PrivKey = str

  // Optional type for TallyResult fields
  type OptionT[T] = { present: bool, value: T }

  // TallyResult schema
  type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, int]], // Using int for Nat
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: List[Addr],
    non_voters: List[Addr],
  }

  // Contract State
  var candidates: List[Addr]
  var start_at: Timestamp
  var end_at: Timestamp
  var enclave_pubkey: PubKey
  var ballots: Map[Addr, List[int]] // Map<Addr, Vec<u8>> represented as List[int] for simplicity
  var tally_result: OptionT[TallyResult] // Using OptionT for Option<TallyResult>

  // Environment (for derived states)
  var env_block_time: Timestamp

  // Derived States
  pure def isCreated: bool = env_block_time < start_at
  pure def isVoting: bool = start_at <= env_block_time and env_block_time < end_at
  pure def isTallying: bool = env_block_time >= end_at and not(tally_result.present)
  pure def isResolved: bool = tally_result.present

  // Helper for OptionT
  pure def some[T](v: T): OptionT[T] = { present: true, value: v }
  pure def noneTallyResult(): OptionT[TallyResult] = { present: false, value: { winners: List(), per_round_counts: List(), eliminated_by_round: List(), ballots_tallied: 0, ballots_dropped: 0, dropped_voters: List(), non_voters: List() } }

  // Actions
  action instantiate(
    new_candidates: List[Addr],
    new_start_at: Timestamp,
    new_end_at: Timestamp,
    new_enclave_pubkey: PubKey
  ): bool = all {
    // Requires
    List.size(new_candidates) >= 1,
    List.distinct(new_candidates),
    new_start_at > env_block_time,
    new_end_at > new_start_at,

    // Produces
    candidates' = new_candidates,
    start_at' = new_start_at,
    end_at' = new_end_at,
    enclave_pubkey' = new_enclave_pubkey,
    ballots' = Map(),
    tally_result' = noneTallyResult(),
    env_block_time' = env_block_time, // Time doesn't advance in this action
  }

  action submit_ballot(
    msg_sender: Addr,
    encrypted_preferences: List[int]
  ): bool = all {
    // Requires
    isVoting,
    List.exists(candidates, c => c == msg_sender),
    List.size(encrypted_preferences) > 0,

    // Produces
    ballots' = ballots.set(msg_sender, encrypted_preferences),
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    tally_result' = tally_result,
    env_block_time' = env_block_time,
  }

  action close_and_tally(): bool = all {
    // Requires
    isTallying,

    // Produces (no storage change, only event emission)
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    tally_result' = tally_result,
    env_block_time' = env_block_time,
  }

  action publish_result(
    tally: TallyResult,
    attestation_valid: bool // Simplified attestation check
  ): bool = all {
    // Requires
    isTallying,
    attestation_valid,
    // Simplified well-formedness checks for now
    List.size(tally.winners) >= 1,
    List.size(tally.winners) <= List.size(candidates),
    List.forall(tally.winners, w => List.exists(candidates, c => c == w)),
    tally.ballots_tallied + tally.ballots_dropped + List.size(tally.non_voters) == List.size(candidates),

    // Produces
    tally_result' = some(tally),
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    env_block_time' = env_block_time,
  }

  action advance_time(new_time: Timestamp): bool = all {
    new_time > env_block_time,
    env_block_time' = new_time,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    tally_result' = tally_result,
  }

  // Placeholder for Tally_spec and IRV_spec
  pure def Tally_spec(
    raw_ballots: Map[Addr, List[int]],
    cands: List[Addr],
    privkey: PrivKey
  ): TallyResult = {
    // This is a placeholder. Actual implementation would be complex.
    // For now, return a dummy TallyResult that satisfies basic well-formedness.
    winners: List(cands.head()),
    per_round_counts: List(Map(cands.head() -> 1)),
    eliminated_by_round: List(),
    ballots_tallied: 1,
    ballots_dropped: 0,
    dropped_voters: List(),
    non_voters: List.filter(cands, c => not(raw_ballots.keys().contains(c))),
  }

  // Invariants (initial placeholders)
  val S1_non_empty_candidates = List.size(candidates) >= 1
  val S2_distinct_candidates = List.distinct(candidates)
  val S3_well_ordered_voting_window = start_at < end_at
  val S4_ballot_keys_are_candidates = List.forall(ballots.keys(), k => List.exists(candidates, c => c == k))
  val S5_terminality_of_resolution = true // Placeholder for handler-set property
  val S6_winner_well_formedness = if (tally_result.present) then (List.forall(tally_result.value.winners, w => List.exists(candidates, c => c == w)) and List.size(tally_result.value.winners) >= 1 and List.size(tally_result.value.winners) <= List.size(candidates)) else true
  val S7_tally_count_conservation = if (tally_result.present) then (tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + List.size(tally_result.value.non_voters) == List.size(candidates)) else true
  val S8_per_round_count_consistency = if (tally_result.present) then (List.forall(tally_result.value.per_round_counts, m => Map.sum(m) == tally_result.value.ballots_tallied)) else true
  val S9_elimination_monotonicity = true // Complex to encode without full IRV_spec
  val S10_resolution_implies_past_end_at = if (tally_result.present) then (env_block_time >= end_at) else true

  val B1_tally_result_monotone_once_set = always (tally_result.present => always (tally_result.present and tally_result' = tally_result))
  val B2_no_late_ballots = always (env_block_time >= end_at => ballots' = ballots)
  val B3_no_premature_tally = always (tally_result.present and not(tally_result.present) => env_block_time >= end_at)
  val B4_no_premature_voting = always (ballots' /= ballots => start_at <= env_block_time and env_block_time < end_at)
  val B5_publish_result_fires_at_most_once = true // Derived corollary, hard to encode directly
  val B6_ballot_writer_is_the_ballot_voter = true // Complex, requires unique message attribution
  val B7_terminal_state_immutability = always (isResolved => always isResolved)
  val B8_attestation_binds_tally = true // Cross-layer, simplified as attestation_valid
  val B9_B8_negligibility_budget_decomposition = true // Meta-security, cannot encode
  val B10_tally_correctness = true // Cross-layer, simplified as attestation_valid implies well-formedness

  // Witnesses (negations of reachability targets)
  val witness_resolution_reachable = not(isResolved)
  val witness_ballot_submittable = not(isVoting and List.size(candidates) > 0)
  val witness_end_at_crossing = not(env_block_time >= end_at)

}
