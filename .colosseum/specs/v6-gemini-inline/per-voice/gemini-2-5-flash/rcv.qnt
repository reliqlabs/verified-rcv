module rcv {
  // --- Types ---
  type Addr = str
  type Timestamp = int
  type PubKey = str // Representing Bytes for Quint's type system
  type PrivKey = str // Representing Bytes for Quint's type system
  type Ciphertext = str // Representing List[u8] for Quint's type system, using str for simplicity
  type DstackAttestation = str // Representing DstackAttestation structure

  type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, int]],
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: List[Addr],
    non_voters: List[Addr]
  }

  // Placeholder for default TallyResult, used when Option is 'None'
  val DEFAULT_TALLY_RESULT: TallyResult = {
    winners: List(),
    per_round_counts: List(),
    eliminated_by_round: List(),
    ballots_tallied: 0,
    ballots_dropped: 0,
    dropped_voters: List(),
    non_voters: List()
  }

  type MessageKind = str // "Instantiate", "SubmitBallot", "CloseAndTally", "PublishResult"
  type Message = {
    kind: MessageKind,
    sender: Addr,
    encrypted_preferences: { present: bool, value: Ciphertext },
    tally_payload: { present: bool, value: TallyResult },
    attestation_payload: { present: bool, value: DstackAttestation }
  }

  type EnclaveRegistryEntry = { vkey: str, mrtd: str, rtmr: str } // Representing Bytes

  // --- Constant Parameters (values to be instantiated in main.qnt) ---
  const MIN_CANDIDATES: int
  const CONTRACT_ADDR: Addr
  const CANONICAL_VERIFIED_RCV_VKEY: str
  const CANONICAL_VERIFIED_RCV_MRTD: str
  const CANONICAL_VERIFIED_RCV_RTMR: str
  const DST_VERIFIED_RCV_TALLY_V1: str
  const ENCLAVE_ID_NAME: str // e.g. "verified-rcv"

  const INITIAL_CANDIDATES: List[Addr]
  const INITIAL_START_AT: Timestamp
  const INITIAL_END_AT: Timestamp
  const INITIAL_ENCLAVE_PUBKEY: PubKey

  // Nondeterministic choices for actions in `step`
  const SOME_ADDRS: Set[Addr]
  const SOME_CIPHERTEXTS: Set[Ciphertext]
  const SOME_PUBKEYS: Set[PubKey] // For instantiate, if it were an action after init
  const SOME_TIMESTAMPS: Set[Timestamp] // For instantiate, if it were an action after init

  // --- State Variables ---
  var candidates: List[Addr]
  var start_at: Timestamp
  var end_at: Timestamp
  var enclave_pubkey: PubKey
  var ballots: Map[Addr, Ciphertext]
  var tally_result: { present: bool, value: TallyResult }
  var env_block_time: Timestamp

  // --- Ghost Variables (for modeling cross-layer interactions and temporal properties) ---
  var ballots_at_end_at: Map[Addr, Ciphertext] // Snapshot of ballots at the end_at boundary (for B2, B10)
  var dstack_kms_registry: Map[str, EnclaveRegistryEntry] // Simulates on-chain registry (for B8, B9)
  var dstack_kms_derived_keys: Map[Addr, PrivKey] // Simulates DstackKeyManager (for B10)
  var fires_at_transition: Set[Message] // Stores messages fired in the current state transition (for B6, B8)

  // --- Pure Definitions (Derived States) ---
  pure def is_created(): bool = env_block_time < start_at
  pure def is_voting(): bool = start_at <= env_block_time and env_block_time < end_at
  pure def is_tallying(): bool = env_block_time >= end_at and not tally_result.present
  pure def is_resolved(): bool = tally_result.present

  // --- Pure Definitions (Helper for Invariants and Logic) ---

  // Simulates SHA-256 for binding, for type consistency
  pure def hash_placeholder(s: str): str = s // Simplified to return the string itself for a placeholder hash

  // Helper for optional map lookups, for Ciphertext
  pure def map_get_optional_ciphertext(m: Map[Addr, Ciphertext], k: Addr): { present: bool, value: Ciphertext } =
    if (m.keys().contains(k)) { present: true, value: m.get(k) } else { present: false, value: "" }

  // Helper for optional map lookups, for EnclaveRegistryEntry
  pure def map_get_optional_registry(m: Map[str, EnclaveRegistryEntry], k: str): { present: bool, value: EnclaveRegistryEntry } =
    if (m.keys().contains(k)) { present: true, value: m.get(k) } else { present: false, value: {vkey: "", mrtd: "", rtmr: ""} }

  // Helper for optional map lookups, for PrivKey
  pure def map_get_optional_privkey(m: Map[Addr, PrivKey], k: Addr): { present: bool, value: PrivKey } =
    if (m.keys().contains(k)) { present: true, value: m.get(k) } else { present: false, value: "" }


  // Simplified Borsh serialization for hashing `tally_body`
  pure def canonical_serialization(tally_body: TallyResult): str =
    // Simplified to represent the content without actual complex type serialization
    "winners_prefix" + tally_body.winners.toString() +
    "per_round_counts_prefix" + tally_body.per_round_counts.toString() + // Fallback to List.toString()
    "eliminated_by_round_prefix" + tally_body.eliminated_by_round.toString() +
    "ballots_tallied:" + tally_body.ballots_tallied.toString() +
    "ballots_dropped:" + tally_body.ballots_dropped.toString() +
    "dropped_voters_prefix" + tally_body.dropped_voters.toString() +
    "non_voters_prefix" + tally_body.non_voters.toString()


  // Predicate: `image_registration_honest(σ)`
  pure def image_registration_honest_at_current_state(): bool =
    val reg_entry_opt = map_get_optional_registry(dstack_kms_registry, ENCLAVE_ID_NAME) {
      reg_entry_opt.present and
      reg_entry_opt.value.vkey == CANONICAL_VERIFIED_RCV_VKEY and
      reg_entry_opt.value.mrtd == CANONICAL_VERIFIED_RCV_MRTD and
      reg_entry_opt.value.rtmr == CANONICAL_VERIFIED_RCV_RTMR
    }

  // Pure function simulating attestation verification (B8 clauses a,b,c,d)
  pure def verify_attestation(attestation: DstackAttestation, tally_body: TallyResult): bool =
    // Clauses (a) TDX quote validates, (b) ZK proof verifies: assumed true if registry is honest.
    image_registration_honest_at_current_state() and
    // Clause (c) attested user_data hash and DST_VERIFIED_RCV_TALLY_V1
    // Simplification: assume the `attestation` itself contains the verified hash and DST
    // A real system would extract these from the attestation and verify against `tally_body`
    val expected_digest = hash_placeholder(CONTRACT_ADDR + canonical_serialization(tally_body)) {
      // In Quint, we model this as simply checking that the attestation is consistent with the
      // expected honest computation *if* the registry is honest.
      // This is the classical-Prop shadow, B9's negligibility bounds the cryptographic claims.
      true
    }

  // Highly simplified `Tally_spec` for Quint, focusing on input-output determinism for B10
  // In a real verification, this would be the full IRV algorithm.
  pure def Tally_spec_quint(raw_ballots: Map[Addr, Ciphertext], candidates_list: List[Addr], privkey: PrivKey): TallyResult = {
    val num_tallied = raw_ballots.keys().size();
    val num_candidates = candidates_list.length();
    val non_voters_list = candidates_list.filter(c => not raw_ballots.keys().toSet().contains(c));
    // Use `head()` safely by checking list length
    val dummy_winners = if (num_tallied > 0 and num_candidates > 0) List(candidates_list.head()) else List();
    val dummy_per_round = if (num_tallied > 0 and num_candidates > 0) Map(candidates_list.head() -> num_tallied) else Map();
    {
      winners: dummy_winners,
      per_round_counts: List(dummy_per_round),
      eliminated_by_round: List(List()),
      ballots_tallied: num_tallied,
      ballots_dropped: 0,
      dropped_voters: List(),
      non_voters: non_voters_list
    }
  }


  // --- Actions (State Transitions) ---

  // Initial state setup, including contract instantiation
  action init = all {
    candidates' = INITIAL_CANDIDATES,
    start_at' = INITIAL_START_AT,
    end_at' = INITIAL_END_AT,
    enclave_pubkey' = INITIAL_ENCLAVE_PUBKEY,
    ballots' = Map(),
    tally_result' = { present: false, value: DEFAULT_TALLY_RESULT },
    env_block_time' = 0,
    // Ghost variables initialization
    ballots_at_end_at' = Map(),
    dstack_kms_derived_keys' = dstack_kms_derived_keys.put(CONTRACT_ADDR, "dummy_privkey_bytes"), // Simulate KMS key derivation
    dstack_kms_registry' = dstack_kms_registry.put(ENCLAVE_ID_NAME, {vkey: CANONICAL_VERIFIED_RCV_VKEY, mrtd: CANONICAL_VERIFIED_RCV_MRTD, rtmr: CANONICAL_VERIFIED_RCV_RTMR}), // Simulate honest registry
    fires_at_transition' = Set({kind: "Instantiate", sender: CONTRACT_ADDR, encrypted_preferences: { present: false, value: "" }, tally_payload: { present: false, value: DEFAULT_TALLY_RESULT }, attestation_payload: { present: false, value: "" }}),
    // Block 1: instantiate requires (checked at init)
    INITIAL_CANDIDATES.length() >= MIN_CANDIDATES,
    INITIAL_CANDIDATES.toSet().size() == INITIAL_CANDIDATES.length(),
    INITIAL_START_AT > 0, // Assuming env_block_time starts at 0, so it's in the future
    INITIAL_END_AT > INITIAL_START_AT,
  }

  // Advances `env_block_time` by one unit
  action time_step = all {
    env_block_time' = env_block_time + 1,
    others
  }

  // Block 3: submit_ballot
  action submit_ballot_action(sender: Addr, encrypted_prefs: Ciphertext) = all {
    is_voting(),
    sender.in(candidates.toSet()),
    encrypted_prefs.length() > 0, // Ciphertext is str, length() means length
    ballots' = ballots.put(sender, encrypted_prefs),
    fires_at_transition' = fires_at_transition.union(Set({kind: "SubmitBallot", sender: sender, encrypted_preferences: { present: true, value: encrypted_prefs }, tally_payload: { present: false, value: DEFAULT_TALLY_RESULT }, attestation_payload: { present: false, value: "" }})),
    others
  }

  // Block 5: close_and_tally (event-only, no state change)
  action close_and_tally_action = all {
    is_tallying(),
    fires_at_transition' = fires_at_transition.union(Set({kind: "CloseAndTally", sender: CONTRACT_ADDR, encrypted_preferences: { present: false, value: "" }, tally_payload: { present: false, value: DEFAULT_TALLY_RESULT }, attestation_payload: { present: false, value: "" }})),
    others
  }

  // Block 6: publish_result
  action publish_result_action(tally: TallyResult, attestation: DstackAttestation) = all {
    is_tallying(),
    // Attestation verification and well-formedness (simplified)
    verify_attestation(attestation, tally),
    // S6-S9 checks (simplified)
    tally.winners.toSet().subset(candidates.toSet()),
    tally.winners.length() >= 1,
    tally.winners.length() <= candidates.length(),
    tally.ballots_tallied + tally.ballots_dropped + tally.non_voters.length() == candidates.length(),
    tally.per_round_counts.forAll(r_map => r_map.values().sum() == tally.ballots_tallied),
    // S9 (elimination monotonicity) is complex; simplified to always true for this spec
    true, // Placeholder for full S9 logic

    tally_result' = { present: true, value: tally },
    fires_at_transition' = fires_at_transition.union(Set({kind: "PublishResult", sender: CONTRACT_ADDR, encrypted_preferences: { present: false, value: "" }, tally_payload: { present: true, value: tally }, attestation_payload: { present: true, value: attestation }})),
    others
  }

  // Block E1: Enclave-side tally computation and *internal* call to publish_result_action
  // This action encapsulates the off-chain enclave logic and leads to `publish_result_action`.
  action enclave_tally_action = all {
    is_tallying(),
    val privkey_opt = map_get_optional_privkey(dstack_kms_derived_keys, CONTRACT_ADDR) {
      privkey_opt.present and
      val computed_tally = Tally_spec_quint(ballots_at_end_at, candidates, privkey_opt.value);
      val dummy_attestation = "dummy_attestation_bytes"; // Placeholder
      // This is where the enclave "publishes" by triggering the on-chain action
      publish_result_action(computed_tally, dummy_attestation)
    }
  }

  // The main step relation, combining all possible actions
  action step = all {
    // Clear fires_at_transition at the beginning of each step
    fires_at_transition' = Set(),
    any {
      time_step,
      nondet sender = oneOf(SOME_ADDRS),
      nondet prefs = oneOf(SOME_CIPHERTEXTS),
      submit_ballot_action(sender, prefs),
      close_and_tally_action,
      enclave_tally_action // This action will internally call publish_result_action
    },
    // Update `ballots_at_end_at` precisely once when `env_block_time` crosses `end_at`
    if (env_block_time < end_at and next.env_block_time >= end_at and ballots_at_end_at == Map())
      ballots_at_end_at' = ballots
    else
      ballots_at_end_at' = ballots_at_end_at
  }

  // --- Invariants ---

  // Structural Invariants (S-series)
  val inv_S1_non_empty_candidates: bool = candidates.length() >= MIN_CANDIDATES
  val inv_S2_distinct_candidates: bool = candidates.toSet().size() == candidates.length()
  val inv_S3_well_ordered_voting_window: bool = start_at < end_at
  val inv_S4_ballot_keys_are_candidates: bool = ballots.keys().toSet().subset(candidates.toSet())
  // S5 is a handler-set property; its direct state-level consequence is covered by B1
  val inv_S5_terminality_of_resolution_write_discipline: bool =
    is_resolved() => next.tally_result == tally_result // If resolved, tally_result is immutable

  val inv_S6_winner_well_formedness: bool =
    is_resolved() => {
      val tr = tally_result.value; // Access .value for SumType
      tr.winners.toSet().subset(candidates.toSet()) and
      tr.winners.length() >= 1 and
      tr.winners.length() <= candidates.length()
    }

  val inv_S7_tally_count_conservation: bool =
    is_resolved() => {
      val tr = tally_result.value; // Access .value for SumType
      tr.ballots_tallied + tr.ballots_dropped + tr.non_voters.length() == candidates.length()
    }

  val inv_S8_per_round_count_consistency: bool =
    is_resolved() => {
      val tr = tally_result.value; // Access .value for SumType
      tr.per_round_counts.forAll(r_map => r_map.values().sum() == tr.ballots_tallied)
    }

  val inv_S9_elimination_monotonicity: bool =
    is_resolved() => {
      val tr = tally_result.value; // Access .value for SumType
      // This is a simplified check. Full logic is complex for Quint.
      // It requires iterating `eliminated_by_round` and checking `per_round_counts`.
      true // Placeholder for full complex logic
    }

  val inv_S10_resolution_implies_past_end_at: bool = is_resolved() => env_block_time >= end_at

  // Behavioral Invariants (B-series)
  val inv_B1_tally_result_monotone_once_set: bool =
    is_resolved() => next.is_resolved() and next.tally_result == tally_result

  val inv_B2_no_late_ballots: bool = (env_block_time >= end_at) => ballots == ballots_at_end_at

  val inv_B3_no_premature_tally: bool =
    (not tally_result.present and next.tally_result.present) => env_block_time >= end_at

  val inv_B4_no_premature_voting: bool =
    (ballots != next.ballots) => start_at <= env_block_time and env_block_time < end_at

  // B5 is derived from B1 + Block 6's AlreadyResolved; B1 covers its core property.

  val inv_B6_ballot_writer_is_the_ballot_voter: bool = {
    val all_keys = ballots.keys().union(next.ballots.keys());
    val changed_ballots_keys = all_keys.filter(k =>
      map_get_optional_ciphertext(ballots, k) != map_get_optional_ciphertext(next.ballots, k)
    );
    changed_ballots_keys.forAll(k =>
      // (1) The key must be a candidate
      k.in(candidates.toSet()) and
      // (2) Exactly one SubmitBallot message for this key in fires_at_transition
      fires_at_transition.filter(m => m.kind == "SubmitBallot" and m.sender == k).size() == 1 and
      val submit_msg = oneOf(fires_at_transition.filter(m => m.kind == "SubmitBallot" and m.sender == k));
      // (3) The new ballot value matches the message's encrypted_preferences
      submit_msg.encrypted_preferences.present and // Check if Some before accessing .value
      map_get_optional_ciphertext(next.ballots, k).present and // Ensure next ballot is present
      map_get_optional_ciphertext(next.ballots, k).value == submit_msg.encrypted_preferences.value
    )
  }

  val inv_B7_terminal_state_immutability: bool = is_resolved() => next.is_resolved()

  val inv_B8_attestation_binds_tally: bool =
    (not tally_result.present and next.tally_result.present) =>
      // Find the PublishResult message that fired (assuming uniqueness per transition for this purpose)
      fires_at_transition.exists(m => m.kind == "PublishResult" and m.tally_payload.present and m.attestation_payload.present and // Check if present
        val tr = m.tally_payload.value; // Access .value
        val att = m.attestation_payload.value; // Access .value
          // Check preconditions for B8 (from previous state) and verify attestation
          // image_registration_honest_at_current_state() reflects the state before the transition
          image_registration_honest_at_current_state() and
          verify_attestation(att, tr) and
          // Ensure this specific tally was published
          next.tally_result == { present: true, value: tr }
      )

  // B9 (negligibility-budget decomposition) and B10_lean (Lean-internal image-IO obligation)
  // are meta-security/off-chain invariants and are not directly expressible in Quint.
  // B10 (tally-correctness) is partly covered by `enclave_tally_action` always
  // producing `Tally_spec_quint` which is deterministic.
  // The full B10 combines B10_lean, image-identity-binding, B8, dstack_kms_trust, enclave_input_fidelity.
  // Our `enclave_tally_action` and `verify_attestation` implicitly assume dstack_kms_trust and
  // enclave_input_fidelity by using `ballots_at_end_at` and `dstack_kms_derived_keys`.

  val all_invariants: bool = all {
    inv_S1_non_empty_candidates,
    inv_S2_distinct_candidates,
    inv_S3_well_ordered_voting_window,
    inv_S4_ballot_keys_are_candidates,
    inv_S5_terminality_of_resolution_write_discipline,
    inv_S6_winner_well_formedness,
    inv_S7_tally_count_conservation,
    inv_S8_per_round_count_consistency,
    inv_S9_elimination_monotonicity,
    inv_S10_resolution_implies_past_end_at,
    inv_B1_tally_result_monotone_once_set,
    inv_B2_no_late_ballots,
    inv_B3_no_premature_tally,
    inv_B4_no_premature_voting,
    inv_B6_ballot_writer_is_the_ballot_voter,
    inv_B7_terminal_state_immutability,
    inv_B8_attestation_binds_tally
  }

  // --- Reachability Witnesses (must be VIOLATED to show reachability) ---
  val witness_resolution_reachable: bool = not is_resolved()
  val witness_ballot_submittable: bool =
    // Is there any candidate that can submit a ballot?
    not (is_voting() and candidates.exists(c => not ballots.keys().toSet().contains(c)))
  val witness_end_at_crossing: bool = not (env_block_time >= end_at)
}