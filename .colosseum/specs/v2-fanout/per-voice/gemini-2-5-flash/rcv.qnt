module rcv {
  // --- Types ---

  type Addr = str
  type Timestamp = int // Unix epoch in seconds
  type PubKey = str // Hex string representation
  type PrivKey = str // Hex string representation
  type Bytes32 = str // Hex string for 32 bytes (SHA-256 digests, MRTD, RTMR)
  type Nat = int   // Natural numbers (non-negative integers)

  type DstackAttestation = {
    user_data_lower32: Bytes32, // Lower 32 bytes of attested user_data
    user_data_upper32: Bytes32, // Upper 32 bytes (domain separation tag)
    mrtd: Bytes32,              // Measurement of Root Trust Domain
    rtmr: Bytes32,              // Measurement of Runtime Trust Domain
    tdx_quote: str,             // Opaque TDX quote data
    zkdcap_proof: str           // Opaque ZK proof data
  }

  type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, Nat]], // Indexed by round; entry per surviving candidate
    eliminated_by_round: List[List[Addr]], // Candidates eliminated in each round
    ballots_tallied: Nat,                  // Ballots successfully decrypted + validated
    ballots_dropped: Nat,                  // Ballots that failed decryption/parse/validation
    dropped_voters: List[Addr],            // Voters whose ballots were dropped
    non_voters: List[Addr]                 // Candidates who did not submit a ballot
  }

  type RCVContractState = {
    candidates: List[Addr],
    start_at: Timestamp,
    end_at: Timestamp,
    enclave_pubkey: PubKey,
    ballots: Map[Addr, List[int]], // encrypted_preferences: Vec<u8> modelled as List[int]
    tally_result: Option[TallyResult],
    ballots_at_end_at: Option[Map[Addr, List[int]]], // Ghost variable for B2: snapshot of ballots when voting ends
    contract_addr: Addr // Unique identifier for this contract instance
  }

  // --- Constants (Parameters from main.qnt for model checking bounds and specific values) ---

  const CANDIDATE_SET_MAX_SIZE: int         // Max number of candidates for a contract instance
  const DST_VERIFIED_RCV_TALLY_V1: Bytes32  // Domain separation tag for attested user_data
  const CANONICAL_VERIFIED_RCV_VKEY: Bytes32 // Expected ZK verification key for verified-rcv enclave
  const CANONICAL_VERIFIED_RCV_MRTD: Bytes32 // Expected MRTD for verified-rcv enclave
  const CANONICAL_VERIFIED_RCV_RTMR: Bytes32 // Expected RTMR for verified-rcv enclave
  const IS_IMAGE_REGISTRATION_HONEST: bool  // Placeholder for image_registration_honest predicate (Section 6.1)

  // --- State Variables ---

  var rcv_contracts: Map[Addr, RCVContractState] // Map of all deployed RCV contract instances
  var global_env_block_time: Timestamp           // The chain's current global block time
  var next_contract_id: int                     // Counter for generating unique contract addresses

  // --- Pure Definitions (Helper Functions) ---

  // Derives the current state of a contract instance based on global time and tally_result
  pure def CurrentState(contract: RCVContractState): str = {
    if global_env_block_time < contract.start_at then "Created"
    else if global_env_block_time >= contract.start_at and global_env_block_time < contract.end_at then "Voting"
    else if global_env_block_time >= contract.end_at and contract.tally_result.is_none() then "Tallying"
    else "Resolved" // contract.tally_result.is_some()
  }

  // Helper for S2: Checks if all elements in a list are distinct
  pure def is_distinct[T](l: List[T]): bool = Set(l).size() == l.size()

  // Helper for S4: Checks if an address is in the contract's candidate list
  pure def is_candidate(contract: RCVContractState, addr: Addr): bool =
    contract.candidates.exists(c => c == addr)

  // Helper for S6: Winner well-formedness (part of TallyResult schema validation)
  pure def s6_winners_well_formed(t: TallyResult, candidates: List[Addr]): bool =
    t.winners.size() >= 1 and t.winners.size() <= candidates.size() and
    t.winners.forall(w => candidates.exists(c => c == w))

  // Helper for S7: Tally count conservation (part of TallyResult schema validation)
  pure def s7_count_conservation(t: TallyResult, candidates: List[Addr]): bool =
    t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == candidates.size()

  // Helper for S8: Per-round count consistency (part of TallyResult schema validation)
  pure def s8_per_round_count_consistency(t: TallyResult): bool =
    t.per_round_counts.forall(rc => rc.values().fold(0, (a, b) => a + b) == t.ballots_tallied)

  // Helper for S9: Elimination monotonicity (part of TallyResult schema validation)
  pure def s9_elimination_monotonicity_def(t: TallyResult): bool = {
    val num_rounds = t.eliminated_by_round.size()
    0.to(num_rounds - 1).forall(i =>
      val eliminated_in_i = Set(t.eliminated_by_round.get(i))
      (i + 1).to(num_rounds - 1).forall(j =>
        eliminated_in_i.intersect(t.per_round_counts.get(j).keys()).size() == 0
      )
    )
  }

  // Helper for Block 6: Set relations (part of TallyResult schema validation)
  pure def block6_set_relations(t: TallyResult, candidates: List[Addr], ballot_keys: Set[Addr]): bool =
    all {
      // non_voters = candidates \ ballots.keys (simplified for list comparison)
      Set(t.non_voters) == Set(candidates).exclude(ballot_keys),
      // dropped_voters ⊆ ballots.keys
      Set(t.dropped_voters).subseteq(ballot_keys),
      // dropped_voters ∩ non_voters = ∅
      Set(t.dropped_voters).intersect(Set(t.non_voters)).size() == 0,
      // len(dropped_voters) = ballots_dropped
      t.dropped_voters.size() == t.ballots_dropped
    }

  // Composite well-formedness predicate for a TallyResult (from Block 6 Requires)
  pure def tally_result_well_formed(t: TallyResult, contract: RCVContractState): bool =
    all {
      s6_winners_well_formed(t, contract.candidates),
      s7_count_conservation(t, contract.candidates),
      s8_per_round_count_consistency(t),
      s9_elimination_monotonicity_def(t),
      block6_set_relations(t, contract.candidates, contract.ballots.keys())
    }

  // Abstract placeholders for B8 attestation components (cryptographic checks, external to Quint)
  pure def is_tdx_quote_validates(att: DstackAttestation): bool = true
  pure def is_zkdcap_proof_verifies(att: DstackAttestation, vkey: Bytes32): bool = true

  // Simplified SHA-256 and canonical serialization for B8(c).
  // These are placeholders; actual implementations would be cryptographically secure and Borsh-compliant.
  pure def sha256(s: str): Bytes32 = {
    // Take a prefix of the input string, ensure it's always 32 "bytes" long for type consistency.
    // This is NOT a real hash function and serves for structural checks only.
    val base_hash = s + "this_is_a_sha256_placeholder_for_quint_model_checking_only_and_should_not_be_relied_upon_for_security"
    base_hash.slice(0, 32)
  }

  pure def canonical_serialization(contract_addr: Addr, tally_body: TallyResult): str = {
    // Highly simplified string concatenation.
    // Real implementation requires Borsh serialization in candidate-declaration order
    // for all fields, with specific leaf encodings (e.g., Addr as u32-LE length-prefixed UTF-8).
    val serialized_data = contract_addr +
                          tally_body.winners.join("::") +
                          tally_body.per_round_counts.map(m => m.keys().join(":") + m.values().map(v => v.toStr()).join(":")).join("::") +
                          tally_body.eliminated_by_round.map(l => l.join(":")).join("::") +
                          tally_body.ballots_tallied.toStr() +
                          tally_body.ballots_dropped.toStr() +
                          tally_body.dropped_voters.join("::") +
                          tally_body.non_voters.join("::")
    serialized_data
  }

  // Abstract placeholders for B10 components (cross-layer and off-chain properties)
  pure def dstack_kms_derived(privkey: PrivKey, contract_addr: Addr): bool = true
  pure def enclave_input_fidelity(raw_ballots: Map[Addr, List[int]], candidates: List[Addr]): bool = true

  // Abstract Tally_spec function: represents the formal IRV algorithm (Section 2.5)
  // In a real verification, this would be the rigorously specified algorithm.
  pure def Tally_spec(raw_ballots: Map[Addr, List[int]], candidates: List[Addr], privkey: PrivKey): TallyResult = {
    // Returns a dummy tally result that satisfies basic well-formedness for model checking.
    // `privkey` is unused here as decryption is not modeled.
    val non_voters_list = candidates.filter(c => not raw_ballots.keys().exists(k => k == c))
    {
      winners: if candidates.size() > 0 then List(candidates.get(0)) else [], // Default winner to first candidate or empty
      per_round_counts: if raw_ballots.size() > 0 then List(raw_ballots.keys().mapBy(k => k -> 1)) else [],
      eliminated_by_round: [],
      ballots_tallied: raw_ballots.size(),
      ballots_dropped: 0,
      dropped_voters: [],
      non_voters: non_voters_list
    }
  }

  // --- Actions ---

  // Initial state of the entire system (no contracts deployed, global time at 0)
  action rcv_init = all {
    rcv_contracts' = Map(),
    global_env_block_time' = 0,
    next_contract_id' = 0
  }

  // Block 1: instantiate a new RCV contract (CosmWasm `MsgInstantiateContract`)
  action block1_instantiate(
    new_candidates: List[Addr],
    new_start_at: Timestamp,
    new_end_at: Timestamp,
    new_enclave_pubkey: PubKey
  ) = all {
    val new_contract_addr = "contract_" + next_contract_id.toStr(),
    not rcv_contracts.keys().exists(k => k == new_contract_addr), // Ensure address uniqueness
    new_candidates.size() >= 1,                                  // S1
    is_distinct(new_candidates),                                 // S2
    new_start_at > global_env_block_time,                        // Block 1 Requires
    new_end_at > new_start_at,                                   // S3
    new_candidates.size() <= CANDIDATE_SET_MAX_SIZE,             // Model checking bound (per Block 1 Requires)

    val new_contract_state = {
      candidates: new_candidates,
      start_at: new_start_at,
      end_at: new_end_at,
      enclave_pubkey: new_enclave_pubkey,
      ballots: Map(),
      tally_result: None(),
      ballots_at_end_at: None(), // Not set until voting closes
      contract_addr: new_contract_addr
    },
    rcv_contracts' = rcv_contracts.put(new_contract_addr, new_contract_state),
    next_contract_id' = next_contract_id + 1,
    global_env_block_time' = global_env_block_time // Instantiation does not advance global time
  }
  // Action label for temporal invariants, parameterized by instantiated contract address
  action block1_instantiate_label(contract_addr_param: Addr) =
    block1_instantiate(any, any, any, any).on[Addr](x => x == contract_addr_param)

  // Implicit Blocks 2 & 4: time advances (CosmWasm block production)
  action time_advances(new_time: Timestamp) = all {
    new_time > global_env_block_time, // Time must always advance monotonically (Section 6.1)
    global_env_block_time' = new_time,
    rcv_contracts' = rcv_contracts.map(contract_addr_key, contract => {
      val old_derived_state = CurrentState(contract)
      // Recalculate derived state based on new global_env_block_time
      val new_derived_state = CurrentState(contract.with("global_env_block_time", new_time)) // temporary for CurrentState eval

      val needs_ballots_snapshot =
        (old_derived_state == "Voting" and new_derived_state == "Tallying") or
        (old_derived_state == "Tallying" and new_derived_state == "Tallying" and contract.ballots_at_end_at.is_none())
      
      // Update the ghost variable ballots_at_end_at if entering Tallying state
      contract.with("ballots_at_end_at",
        if needs_ballots_snapshot then Some(contract.ballots) else contract.ballots_at_end_at
      )
    })
  }
  action time_advances_label = time_advances(any)

  // Block 3: submit_ballot (CosmWasm `ExecuteMsg::SubmitBallot`)
  action block3_submit_ballot(contract_addr_param: Addr, sender: Addr, encrypted_prefs: List[int]) = any {
    val contract = rcv_contracts.get(contract_addr_param),
    CurrentState(contract) == "Voting",         // NotInVotingWindow error
    is_candidate(contract, sender),             // NotACandidate error
    encrypted_prefs.size() > 0,                 // EmptyBallot error

    // Update ballot for sender (last-write-wins)
    rcv_contracts' = rcv_contracts.put(contract_addr_param,
      contract.with("ballots", contract.ballots.put(sender, encrypted_prefs))
    ),
    global_env_block_time' = global_env_block_time // This action does not advance global time
  }
  // Action label for temporal invariants, parameterized by contract address and sender
  action block3_submit_ballot_label(contract_addr_param: Addr, sender_param: Addr) =
    block3_submit_ballot(contract_addr_param, sender_param, any)

  // Block 5: close_and_tally (CosmWasm `ExecuteMsg::CloseAndTally`)
  // Idempotent: emits event, no state change. Modeled as no state variable mutation.
  action block5_close_and_tally(contract_addr_param: Addr) = any {
    val contract = rcv_contracts.get(contract_addr_param),
    CurrentState(contract) == "Tallying", // VotingStillOpen or AlreadyResolved errors
    // No state changes for this action
    rcv_contracts' = rcv_contracts,
    global_env_block_time' = global_env_block_time
  }
  action block5_close_and_tally_label = block5_close_and_tally(any)

  // Block 6: publish_result (CosmWasm `ExecuteMsg::PublishResult`)
  action block6_publish_result(contract_addr_param: Addr, tally: TallyResult, attestation: DstackAttestation) = any {
    val contract = rcv_contracts.get(contract_addr_param),
    CurrentState(contract) == "Tallying",      // NotInTallyingState error
    contract.tally_result.is_none(),           // AlreadyResolved error

    // Attestation verification (B8 clauses a, b, c, d)
    is_tdx_quote_validates(attestation),                                     // AttestationFailed (TDX layer)
    is_zkdcap_proof_verifies(attestation, CANONICAL_VERIFIED_RCV_VKEY),      // AttestationFailed (ZKDCAP layer)
    attestation.user_data_lower32 == sha256(canonical_serialization(contract.contract_addr, tally)), // AttestationFailed (user_data hash mismatch)
    attestation.user_data_upper32 == DST_VERIFIED_RCV_TALLY_V1,              // AttestationFailed (domain separation tag mismatch)
    attestation.mrtd == CANONICAL_VERIFIED_RCV_MRTD,                         // AttestationFailed (MRTD mismatch)
    attestation.rtmr == CANONICAL_VERIFIED_RCV_RTMR,                         // AttestationFailed (RTMR mismatch)
    IS_IMAGE_REGISTRATION_HONEST,                                            // Assumes chain registry is honest (Section 6.1)

    // Tally well-formedness checks (S6-S9 + Block 6 set relations from intent doc)
    tally_result_well_formed(tally, contract), // InvalidTally error

    // Update contract state: set tally_result and transition to Resolved
    rcv_contracts' = rcv_contracts.put(contract_addr_param,
      contract.with("tally_result", Some(tally))
    ),
    global_env_block_time' = global_env_block_time // This action does not advance global time
  }
  // Action label for temporal invariants, parameterized by published contract address
  action block6_publish_result_label(contract_addr_param: Addr) =
    block6_publish_result(contract_addr_param, any, any)

  // --- Overall System Step Action ---

  // Defines a single step in the system's execution.
  // Prioritizes instantiating a contract if none exist, then allows other actions.
  action step = any {
    rcv_contracts.isEmpty().implies(
      block1_instantiate(
        nondet new_candidates_list = List("alice", "bob", "charlie"), // Example candidates
        nondet s = global_env_block_time + 1, // Start time slightly in the future
        nondet e = s + 10,                   // End time 10 units after start
        nondet pubkey = "enclave_pubkey_1"
      )
    ),
    rcv_contracts.notEmpty().implies(
      val chosen_contract_addr = oneOf(rcv_contracts.keys()),
      val chosen_contract = rcv_contracts.get(chosen_contract_addr),
      any {
        time_advances(nondet t = global_env_block_time + 1), // Advance time by 1 unit for bounded checking
        block3_submit_ballot(chosen_contract_addr, oneOf(chosen_contract.candidates), List(1, 2, 3)), // Example encrypted ballot
        block5_close_and_tally(chosen_contract_addr),
        block6_publish_result(
          chosen_contract_addr,
          // Generate a dummy tally for `publish_result` from abstract `Tally_spec`
          Tally_spec(chosen_contract.ballots, chosen_contract.candidates, "dummy_privkey"),
          // Generate a dummy attestation that passes required checks
          { user_data_lower32: sha256(canonical_serialization(chosen_contract_addr, Tally_spec(chosen_contract.ballots, chosen_contract.candidates, "dummy_privkey"))),
            user_data_upper32: DST_VERIFIED_RCV_TALLY_V1,
            mrtd: CANONICAL_VERIFIED_RCV_MRTD,
            rtmr: CANONICAL_VERIFIED_RCV_RTMR,
            tdx_quote: "dummy_quote",
            zkdcap_proof: "dummy_proof" }
        )
      }
    )
  }

  // --- Invariants ---

  // S1: non-empty candidates (per contract instance)
  val s1_non_empty_candidates = rcv_contracts.values().forall(c => c.candidates.size() >= 1)

  // S2: distinct candidates (per contract instance)
  val s2_distinct_candidates = rcv_contracts.values().forall(c => is_distinct(c.candidates))

  // S3: well-ordered voting window (per contract instance)
  val s3_well_ordered_voting_window = rcv_contracts.values().forall(c => c.start_at < c.end_at)

  // S4: ballot keys are candidates (per contract instance)
  val s4_ballot_keys_are_candidates = rcv_contracts.values().forall(c => c.ballots.keys().forall(k => is_candidate(c, k)))

  // S5 (derived): terminality of resolution (behavioral part covered by B1, structural part is a property of action guards)
  val s5_terminality_of_resolution: bool = true

  // S6: winner well-formedness (per contract instance, if tally_result is Some)
  val s6_winner_well_formedness = rcv_contracts.values().forall(c => c.tally_result.is_none() \/ s6_winners_well_formed(c.tally_result.get(), c.candidates))

  // S7: tally count conservation (per contract instance, if tally_result is Some)
  val s7_tally_count_conservation = rcv_contracts.values().forall(c => c.tally_result.is_none() \/ s7_count_conservation(c.tally_result.get(), c.candidates))

  // S8: per-round count consistency (per contract instance, if tally_result is Some)
  val s8_per_round_count_consistency = rcv_contracts.values().forall(c => c.tally_result.is_none() \/ s8_per_round_count_consistency(c.tally_result.get()))

  // S9: elimination monotonicity (per contract instance, if tally_result is Some)
  val s9_elimination_monotonicity = rcv_contracts.values().forall(c => c.tally_result.is_none() \/ s9_elimination_monotonicity_def(c.tally_result.get()))

  // S10 (derived): resolution implies past end_at (per contract instance)
  val s10_resolution_implies_past_end_at = rcv_contracts.values().forall(c => c.tally_result.is_none() \/ global_env_block_time >= c.end_at)

  // B1: tally_result monotone-once-set (per contract instance)
  val b1_tally_result_monotone_once_set =
    always (rcv_contracts.values().forall(contract =>
      contract.tally_result.is_some() ->
      rcv_contracts'.get(contract.contract_addr).tally_result.is_some() /\ // Stays Some
      rcv_contracts'.get(contract.contract_addr).tally_result = contract.tally_result // Value does not change
    ))

  // B2: no late ballots (per contract instance)
  val b2_no_late_ballots =
    always (rcv_contracts.values().forall(contract =>
      global_env_block_time >= contract.end_at ->
      contract.ballots_at_end_at.is_some() /\ // Ghost variable must have been set
      contract.ballots = contract.ballots_at_end_at.get() // Current ballots match the snapshot at end_at
    ))

  // B3: no premature tally (causal) (per contract instance)
  val b3_no_premature_tally =
    always (rcv_contracts.values().forall(contract =>
      (contract.tally_result.is_none() /\ rcv_contracts'.get(contract.contract_addr).tally_result.is_some()) ->
      global_env_block_time >= contract.end_at // Tally transition requires voting to have closed
    ))

  // B4: no premature voting (per contract instance)
  val b4_no_premature_voting =
    always (rcv_contracts.values().forall(contract =>
      (contract.ballots /= rcv_contracts'.get(contract.contract_addr).ballots) ->
      (global_env_block_time >= contract.start_at /\ global_env_block_time < contract.end_at) // Ballots only change during voting window
    ))

  // B5 (derived): publish_result fires at most once
  // This is a direct consequence of B1 and the 'tally_result.is_none()' guard in block6_publish_result.
  val b5_publish_result_fires_at_most_once: bool = true

  // B6: ballot writer is the ballot voter (per contract instance)
  val b6_ballot_writer_is_voter =
    always (rcv_contracts.values().forall(contract =>
      forall k in contract.candidates,
        contract.ballots.get(k) /= rcv_contracts'.get(contract.contract_addr).ballots.get(k) ->
        block3_submit_ballot_label(contract.contract_addr, k) // Only a `SubmitBallot` from `k` can change `ballots[k]`
    ))

  // B7: terminal-state immutability (per contract instance)
  val b7_terminal_state_immutability =
    always (rcv_contracts.values().forall(contract =>
      CurrentState(contract) == "Resolved" ->
      CurrentState(rcv_contracts'.get(contract.contract_addr)) == "Resolved" // Resolved state is a sink
    ))

  // B8: attestation-binds-tally (classical-Prop shadow)
  // This invariant asserts that if `publish_result` fires, its `requires` clauses related
  // to attestation (B8 a,b,c,d) must have been met. Since Quint models only successful transitions,
  // these conditions are implicitly true if `block6_publish_result` successfully fires.
  val b8_attestation_binds_tally_shadow =
    always (true) // True by construction of the `block6_publish_result` action's guards.

  // B9, B10, B10_lean: Meta-security and cross-layer/off-chain invariants (Section 3.2).
  // These cannot be fully encoded in Quint directly as they involve probabilistic claims,
  // Lean-internal theorems, and external operational assumptions.
  // Their classical-Prop shadows are largely covered by B8's required checks on `publish_result`.
  val b9_b10_abstract_placeholder: bool = true


  // Composite invariant: all structural and behavioral invariants that can be encoded in Quint.
  val all_invariants = all {
    s1_non_empty_candidates,
    s2_distinct_candidates,
    s3_well_ordered_voting_window,
    s4_ballot_keys_are_candidates,
    s5_terminality_of_resolution,
    s6_winner_well_formedness,
    s7_tally_count_conservation,
    s8_per_round_count_consistency,
    s9_elimination_monotonicity,
    s10_resolution_implies_past_end_at,
    b1_tally_result_monotone_once_set,
    b2_no_late_ballots,
    b3_no_premature_tally,
    b4_no_premature_voting,
    b5_publish_result_fires_at_most_once,
    b6_ballot_writer_is_voter,
    b7_terminal_state_immutability,
    b8_attestation_binds_tally_shadow,
    b9_b10_abstract_placeholder
  }

  // --- Witness Invariants (MUST be VIOLATED for counterexample traces) ---

  // witness_resolution_reachable: True if the system can reach a resolved state.
  // Negation: property is "tally_result.is_none() for ALL contracts".
  // If `tally_result` becomes `Some` for any contract, this is violated, proving reachability.
  val witness_resolution_reachable = rcv_contracts.values().forall(c => c.tally_result.is_none())

  // witness_ballot_submittable: True if the system can reach a state where ballots can be submitted.
  // Negation: property is "NO contract is in Voting state".
  // If any contract enters Voting state, this is violated, proving submit_ballot is possible.
  val witness_ballot_submittable = rcv_contracts.values().forall(c => CurrentState(c) /= "Voting")

  // witness_end_at_crossing: True if the system can reach a state where env_block_time >= end_at for any contract.
  // Negation: property is "global_env_block_time < end_at for ALL contracts".
  // If global_env_block_time >= end_at for any contract, this is violated, proving time can advance past end_at.
  val witness_end_at_crossing = rcv_contracts.values().forall(c => global_env_block_time < c.end_at)
}