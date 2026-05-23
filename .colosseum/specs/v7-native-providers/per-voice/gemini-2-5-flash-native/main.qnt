module main {
  import rcv(
    CONTRACT_ADDR = 1000,
    EXPECTED_MRTD = 2000,
    EXPECTED_RTMR = 3000,
    DST_VERIFIED_RCV_TALLY_V1 = 4000
  ) .* from "./rcv"

  // --- Initial State ---
  action init = instantiate({
    initial_candidates: List(1, 2, 3), // Example candidates
    initial_start_at: 10,
    initial_end_at: 20,
    initial_enclave_pubkey: 123,
    current_block_time: 0,
  })

  // --- Next State Transition ---
  action step = any {
    advance_time(env_block_time + 1),
    advance_time(env_block_time + 5),

    // Submit ballot (only if in Voting state and sender is a candidate)
    val candidate_voter = oneOf(candidates),
    val some_encrypted_preferences = List(1, 2, 3, 4, 5), // Example preferences
    submit_ballot({
      sender: candidate_voter,
      encrypted_preferences: some_encrypted_preferences,
      current_block_time: env_block_time,
    }),

    // Capture ballots at end_at
    capture_ballots_at_end_at(end_at),

    // Close and tally
    close_and_tally(env_block_time),

    // Publish result
    val dummy_tally_result = TallyResult({
      winners: List(1), // Example winner
      per_round_counts: List(Map(1 -> 3)),
      eliminated_by_round: List(List(2,3)),
      ballots_tallied: 3,
      ballots_dropped: 0,
      dropped_voters: List(),
      non_voters: List(),
    }),
    val dummy_attestation = DstackAttestation({
      tdx_quote_valid: true,
      zkdcap_proof_valid: true,
      attested_user_data_lower_32_bytes: 5000,
      attested_user_data_upper_32_bytes: DST_VERIFIED_RCV_TALLY_V1,
      attested_enclave_identity_mrtd: EXPECTED_MRTD,
      attested_enclave_identity_rtmr: EXPECTED_RTMR,
    }),
    publish_result({
      tally: dummy_tally_result,
      attestation: dummy_attestation,
      current_block_time: env_block_time,
    }),
  }

  // --- Invariants to check ---
  // These are imported directly from rcv and are available in the top-level scope
  // by virtue of the `.*` import.
  // val all_invariants = all_invariants // This would be circular

  // --- Witness Invariants ---
  // val witness_resolution_reachable = witness_resolution_reachable // Circular
  // val witness_ballot_submittable = witness_ballot_submittable // Circular
  // val witness_end_at_crossing = witness_end_at_crossing // Circular
}
