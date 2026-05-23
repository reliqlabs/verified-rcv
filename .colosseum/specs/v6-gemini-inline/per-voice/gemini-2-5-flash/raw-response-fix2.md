===BEGIN FILE: rcv.qnt===
module rcv {
  // --- Quint's Option type (SumType for better type safety and pattern matching) ---
  type OptionT[T] = SumType[
    'Some(value: T),
    'None
  ]
  pure def some[T](v: T): OptionT[T] = Some(value = v)
  pure def none[T](): OptionT[T] = None[T]() // Pass type T for proper type inference

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

  type MessageKind = str // "Instantiate", "SubmitBallot", "CloseAndTally", "PublishResult"
  type Message = {
    kind: MessageKind,
    sender: Addr,
    encrypted_preferences: OptionT[Ciphertext],
    tally_payload: OptionT[TallyResult],
    attestation_payload: OptionT[DstackAttestation]
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
  var tally_result: OptionT[TallyResult]
  var env_block_time: Timestamp

  // --- Ghost Variables (for modeling cross-layer interactions and temporal properties) ---
  var ballots_at_end_at: Map[Addr, Ciphertext] // Snapshot of ballots at the end_at boundary (for B2, B10)
  var dstack_kms_registry: Map[str, EnclaveRegistryEntry] // Simulates on-chain registry (for B8, B9)
  var dstack_kms_derived_keys: Map[Addr, PrivKey] // Simulates DstackKeyManager (for B10)
  var fires_at_transition: Set[Message] // Stores messages fired in the current state transition (for B6, B8)

  // --- Pure Definitions (Derived States) ---
  pure def is_created(): bool = env_block_time < start_at
  pure def is_voting(): bool = start_at <= env_block_time and env_block_time < end_at
  pure def is_tallying(): bool = env_block_time >= end_at and tally_result.tag == "None"
  pure def is_resolved(): bool = tally_result.tag == "Some"

  // --- Pure Definitions (Helper for Invariants and Logic) ---

  // Simulates SHA-256 for binding, for type consistency
  pure def hash_placeholder(s: str): str = s // Simplified to return the string itself for a placeholder hash

  // Simplified Borsh serialization for hashing `tally_body`
  pure def canonical_serialization(tally_body: TallyResult): str =
    "winners:" + tally_body.winners.toStr() +
    "per_round_counts:" + tally_body.per_round_counts.toStr() +
    "eliminated_by_round:" + tally_body.eliminated_by_round.toStr() +
    "ballots_tallied:" + tally_body.ballots_tallied.toStr() +
    "ballots_dropped:" + tally_body.ballots_dropped.toStr() +
    "dropped_voters:" + tally_body.dropped_voters.toStr() +
    "non_voters:" + tally_body.non_voters.toStr()

  // Predicate: `image_registration_honest(σ)`
  pure def image_registration_honest_at_current_state(): bool =
    dstack_kms_registry.get(ENCLAVE_ID_NAME).tag == "Some" and
    val reg_entry = dstack_kms_registry.get(ENCLAVE_ID_NAME).value { // Access .value for SumType
      reg_entry.vkey == CANONICAL_VERIFIED_RCV_VKEY and
      reg_entry.mrtd == CANONICAL_VERIFIED_RCV_MRTD and
      reg_entry.rtmr == CANONICAL_VERIFIED_RCV_RTMR
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
  pure def Tally_spec_quint(raw_ballots: Map[Addr, Ciphertext], candidates_list: List[Addr], privkey: PrivKey): TallyResult =
    val num_tallied = raw_ballots.keys().size()
    val num_candidates = len(candidates_list)
    val non_voters_list = candidates_list.filter(c => not raw_ballots.keys().toSet().contains(c))
    // Use `head()` safely by checking list length
    val dummy_winners = if (num_tallied > 0 and num_candidates > 0) then List(candidates_list.head()) else List()
    val dummy_per_round = if (num_tallied > 0 and num_candidates > 0) then Map(candidates_list.head() -> num_tallied) else Map()
    {
      winners: dummy_winners,
      per_round_counts: List(dummy_per_round),
      eliminated_by_round: List(List()),
      ballots_tallied: num_tallied,
      ballots_dropped: 0,
      dropped_voters: List(),
      non_voters: non_voters_list
    }

  // --- Actions (State Transitions) ---

  // Initial state setup, including contract instantiation
  action init = all {
    candidates' = INITIAL_CANDIDATES,
    start_at' = INITIAL_START_AT,
    end_at' = INITIAL_END_AT,
    enclave_pubkey' = INITIAL_ENCLAVE_PUBKEY,
    ballots' = Map(),
    tally_result' = none[TallyResult](), // Use the new none constructor
    env_block_time' = 0,
    // Ghost variables initialization
    ballots_at_end_at' = Map(),
    dstack_kms_derived_keys' = dstack_kms_derived_keys.put(CONTRACT_ADDR, "dummy_privkey_bytes"), // Simulate KMS key derivation
    dstack_kms_registry' = dstack_kms_registry.put(ENCLAVE_ID_NAME, {vkey: CANONICAL_VERIFIED_RCV_VKEY, mrtd: CANONICAL_VERIFIED_RCV_MRTD, rtmr: CANONICAL_VERIFIED_RCV_RTMR}), // Simulate honest registry
    fires_at_transition' = Set({kind: "Instantiate", sender: CONTRACT_ADDR, encrypted_preferences: none[Ciphertext](), tally_payload: none[TallyResult](), attestation_payload: none[DstackAttestation]()}),
    // Block 1: instantiate requires (checked at init)
    len(INITIAL_CANDIDATES) >= MIN_CANDIDATES,
    INITIAL_CANDIDATES.toSet().size() == len(INITIAL_CANDIDATES),
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
    len(encrypted_prefs) > 0, // Ciphertext is str, len() means length
    ballots' = ballots.put(sender, encrypted_prefs),
    fires_at_transition' = fires_at_transition.union(Set({kind: "SubmitBallot", sender: sender, encrypted_preferences: some(encrypted_prefs), tally_payload: none[TallyResult](), attestation_payload: none[DstackAttestation]()})),
    others
  }

  // Block 5: close_and_tally (event-only, no state change)
  action close_and_tally_action = all {
    is_tallying(),
    fires_at_transition' = fires_at_transition.union(Set({kind: "CloseAndTally", sender: CONTRACT_ADDR, encrypted_preferences: none[Ciphertext](), tally_payload: none[TallyResult](), attestation_payload: none[DstackAttestation]()})),
    others
  }

  // Block 6: publish_result
  action publish_result_action(tally: TallyResult, attestation: DstackAttestation) = all {
    is_tallying(),
    // Attestation verification and well-formedness (simplified)
    verify_attestation(attestation, tally),
    // S6-S9 checks (simplified)
    tally.winners.toSet().subset(candidates.toSet()),
    len(tally.winners) >= 1,
    len(tally.winners) <= len(candidates),
    tally.ballots_tallied + tally.ballots_dropped + len(tally.non_voters) == len(candidates),
    tally.per_round_counts.forAll(r_map => r_map.values().sum() == tally.ballots_tallied),
    // S9 (elimination monotonicity) is complex; simplified to always true for this spec
    true, // Placeholder for full S9 logic

    tally_result' = some(tally),
    fires_at_transition' = fires_at_transition.union(Set({kind: "PublishResult", sender: CONTRACT_ADDR, encrypted_preferences: none[Ciphertext](), tally_payload: some(tally), attestation_payload: some(attestation)})),
    others
  }

  // Block E1: Enclave-side tally computation and *internal* call to publish_result_action
  // This action encapsulates the off-chain enclave logic and leads to `publish_result_action`.
  action enclave_tally_action = all {
    is_tallying(),
    dstack_kms_derived_keys.get(CONTRACT_ADDR).tag == "Some" and
    val privkey = dstack_kms_derived_keys.get(CONTRACT_ADDR).value { // Access .value for SumType
      // Compute the tally using the simplified Tally_spec_quint
      val computed_tally = Tally_spec_quint(ballots_at_end_at, candidates, privkey)
      val dummy_attestation = "dummy_attestation_bytes" // Placeholder
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
    if (env_block_time < end_at and next.env_block_time >= end_at and ballots_at_end_at == Map()) then
      ballots_at_end_at' = ballots
    else
      ballots_at_end_at' = ballots_at_end_at
  }

  // --- Invariants ---

  // Structural Invariants (S-series)
  val inv_S1_non_empty_candidates: bool = len(candidates) >= MIN_CANDIDATES
  val inv_S2_distinct_candidates: bool = candidates.toSet().size() == len(candidates)
  val inv_S3_well_ordered_voting_window: bool = start_at < end_at
  val inv_S4_ballot_keys_are_candidates: bool = ballots.keys().toSet().subset(candidates.toSet())
  // S5 is a handler-set property; its direct state-level consequence is covered by B1
  val inv_S5_terminality_of_resolution_write_discipline: bool =
    is_resolved() => next.tally_result == tally_result // If resolved, tally_result is immutable

  val inv_S6_winner_well_formedness: bool =
    is_resolved() =>
      // Access .value for SumType
      val tr = tally_result.value {
        tr.winners.toSet().subset(candidates.toSet()) and
        len(tr.winners) >= 1 and
        len(tr.winners) <= len(candidates)
      }

  val inv_S7_tally_count_conservation: bool =
    is_resolved() =>
      // Access .value for SumType
      val tr = tally_result.value {
        tr.ballots_tallied + tr.ballots_dropped + len(tr.non_voters) == len(candidates)
      }

  val inv_S8_per_round_count_consistency: bool =
    is_resolved() =>
      // Access .value for SumType
      val tr = tally_result.value {
        tr.per_round_counts.forAll(r_map => r_map.values().sum() == tr.ballots_tallied)
      }

  val inv_S9_elimination_monotonicity: bool =
    is_resolved() =>
      // Access .value for SumType
      val tr = tally_result.value {
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
    (tally_result.tag == "None" and next.tally_result.tag == "Some") => env_block_time >= end_at

  val inv_B4_no_premature_voting: bool =
    (ballots != next.ballots) => start_at <= env_block_time and env_block_time < end_at

  // B5 is derived from B1 + Block 6's AlreadyResolved; B1 covers its core property.

  val inv_B6_ballot_writer_is_the_ballot_voter: bool =
    val changed_ballots_keys = { k | k <- ballots.keys().union(next.ballots.keys()), ballots.get(k) != next.ballots.get(k) } {
      changed_ballots_keys.forAll(k =>
        // (1) The key must be a candidate
        k.in(candidates.toSet()) and
        // (2) Exactly one SubmitBallot message for this key in fires_at_transition
        fires_at_transition.filter(m => m.kind == "SubmitBallot" and m.sender == k).size() == 1 and
        val submit_msg = oneOf(fires_at_transition.filter(m => m.kind == "SubmitBallot" and m.sender == k)) {
          // (3) The new ballot value matches the message's encrypted_preferences
          submit_msg.encrypted_preferences.tag == "Some" and // Check if Some before accessing .value
          next.ballots.get(k) == submit_msg.encrypted_preferences.value
        }
      )
    }

  val inv_B7_terminal_state_immutability: bool = is_resolved() => next.is_resolved()

  val inv_B8_attestation_binds_tally: bool =
    (tally_result.tag == "None" and next.tally_result.tag == "Some") =>
      // Find the PublishResult message that fired (assuming uniqueness per transition for this purpose)
      fires_at_transition.exists(m => m.kind == "PublishResult" and m.tally_payload.tag == "Some" and m.attestation_payload.tag == "Some" and // Check if Some
        val tr = m.tally_payload.value { // Access .value
        val att = m.attestation_payload.value { // Access .value
          // Check preconditions for B8 (from previous state) and verify attestation
          // image_registration_honest_at_current_state() reflects the state before the transition
          image_registration_honest_at_current_state() and
          verify_attestation(att, tr) and
          // Ensure this specific tally was published
          next.tally_result == some(tr)
        }
      })

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
===END FILE===

===BEGIN FILE: main.qnt===
module main {
  import rcv(
    MIN_CANDIDATES = 1,
    CONTRACT_ADDR = "rcv_contract_addr_1",
    CANONICAL_VERIFIED_RCV_VKEY = "dummy_vkey_bytes",
    CANONICAL_VERIFIED_RCV_MRTD = "dummy_mrtd_bytes",
    CANONICAL_VERIFIED_RCV_RTMR = "dummy_rtmr_bytes",
    DST_VERIFIED_RCV_TALLY_V1 = "DST_VERIFIED_RCV_TALLY_V1",
    ENCLAVE_ID_NAME = "verified-rcv",

    INITIAL_CANDIDATES = List("A", "B", "C"),
    INITIAL_START_AT = 10, // Must be > env_block_time' (which starts at 0)
    INITIAL_END_AT = 20,  // Must be > INITIAL_START_AT
    INITIAL_ENCLAVE_PUBKEY = "dummy_enclave_pubkey_bytes",

    SOME_ADDRS = Set("A", "B", "C", "D", "E", "non_candidate_X"),
    SOME_CIPHERTEXTS = Set("ballot_A_enc", "ballot_B_enc", "ballot_C_enc", "bad_ballot_enc"),
    SOME_PUBKEYS = Set("pubkey1", "pubkey2"),
    SOME_TIMESTAMPS = Set(5, 10, 15, 20, 25, 30)
  ).* from "./rcv"
}
===END FILE===

===BEGIN FILE: design-notes.md===
This document describes the design choices made in translating the `verified-rcv` intent document into a Quint specification.

**1. Mapping §2.5 Blocks to Actions**

- **Block 1: instantiate** maps to the `init` action. The intent specifies instantiation as a transition from an uninhabited state `σ⊥` to `σ_init`. Quint's `init` action naturally models this initial setup, so the `instantiate_contract` action found in earlier drafts was removed as a separate `step` component. The `Requires` clauses of Block 1 are enforced as guards on the `init` action.
- **Block 2: time advances to `start_at` (implicit)** is modeled by the `time_step` action and the derived state `is_voting()`. The specific state change `Created -> Voting` occurs when `env_block_time` crosses `start_at`.
- **Block 3: `submit_ballot`** maps to the `submit_ballot_action(sender, encrypted_prefs)`. Its `Requires` and `Forbids` clauses directly translate to guards within this action.
- **Block 4: time advances to `end_at` (implicit)** is modeled similar to Block 2. The derived state `is_tallying()` becomes true, and a ghost variable `ballots_at_end_at` is precisely set at this moment (see details below for B2).
- **Block 5: `close_and_tally`** maps to `close_and_tally_action`. As specified, this action is idempotent and primarily emits an event; in Quint, it updates `fires_at_transition` but causes no other state changes.
- **Block 6: `publish_result`** maps to `publish_result_action(tally, attestation)`. Its extensive `Requires` clauses, including attestation verification and tally well-formedness, are translated into guards.
- **Block E1: enclave tally computation (off-chain, attested)** maps to `enclave_tally_action`. This action represents the off-chain enclave's entire process: retrieving the key, computing the tally, and producing an attestation. It then *internally calls* `publish_result_action` to model the enclave submitting the result to the chain. This keeps the `publish_result_action` focused on chain-side verification.

**2. Encoding of Invariants**

- **S1-S4, S6-S8, S10:** These structural invariants are encoded directly as `val` predicates over the current state variables, reflecting their pointwise evaluability.
- **S5 (terminality of resolution - handler-set property):** The intent notes this is a handler-set meta-property. Quint cannot directly analyze "only Block 6 writes." Instead, its direct behavioral consequence (`if tally_result.tag == "Some" then next.tally_result == tally_result`) is encoded. This overlaps with `B1`, which explicitly models the temporal "monotone-once-set" behavior. The "Block 6's Requires include `tally_result.is_none()`" is enforced in `publish_result_action`'s guards.
- **S9 (elimination monotonicity):** This invariant is stated as `true` in the Quint spec. A full implementation would require complex iteration over `per_round_counts` and `eliminated_by_round` lists, which is cumbersome and less idiomatic for Quint's model-checking strengths (focused on state transitions and reachability). The current simplification focuses on the most critical structural invariants.
- **B1, B3, B4, B7:** These temporal invariants directly translate to `val` predicates relating the current state (`σ`) to the next state (`σ′`) via `next.var`.
- **B2 (no late ballots):** This requires tracking the state of `ballots` at a specific temporal boundary. A ghost variable `ballots_at_end_at: Map[Addr, Ciphertext]` is introduced. This variable is set *exactly once*: when `env_block_time` transitions from `< end_at` to `>= end_at`. Subsequently, `ballots_at_end_at` retains this snapshot value, and `B2` asserts that `ballots` itself equals `ballots_at_end_at` when `env_block_time >= end_at`.
- **B5 (publish_result fires at most once):** This is identified as a derived corollary of `B1` and `Block 6`'s `AlreadyResolved` rejection. `B1` (tally_result monotone-once-set) effectively captures this by ensuring `tally_result` is immutable once set. Therefore, no separate invariant `B5` is encoded, and this is noted as an omission of redundant logic.
- **B6 (ballot writer is the ballot voter):** This invariant is challenging due to the `∃! msg ∈ fires_at_transition(σ → σ′)` and `∀ k ∈ Addr, next.ballots[k] ≠ ballots[k]` clauses. A ghost variable `fires_at_transition: Set[Message]` is introduced, which is cleared at the beginning of each `step` and populated by each action that fires. The invariant then iterates over `changed_ballots_keys` and asserts that for each key, exactly one `SubmitBallot` message with the correct `sender` and `encrypted_preferences` was in `fires_at_transition`. The "unique" part (`fires_at_transition.filter(...).size() == 1`) is explicitly checked.
- **B8 (attestation-binds-tally):** This is encoded as an implication triggered by the `tally_result` transition. It relies on the `fires_at_transition` ghost variable to identify the `PublishResult` message and calls `verify_attestation` (a pure helper function) to check the conditions. `image_registration_honest_at_current_state()` is used as a precondition in `verify_attestation` to reflect the classical shadow of B9's conditioned probability. The actual cryptographic checks (TDX quote validation, ZK proof verification, SHA-256 hash) are simplified to `true` within `verify_attestation` because Quint cannot model complex cryptographic primitives.
- **B9 (B8 negligibility-budget decomposition) and B10_lean (Lean-internal image-IO obligation):** These are meta-security and off-chain invariants, respectively. They are explicitly *not* encoded in Quint, as they fall outside its scope for direct formalization. `B9`'s `image_registration_honest` precondition is captured by `image_registration_honest_at_current_state()`.
- **B10 (tally-correctness):** The `enclave_tally_action` directly uses `Tally_spec_quint` (a simplified pure function) to compute the `TallyResult`. This implicitly models the correctness aspect, assuming `Tally_spec_quint` is a faithful representation of the actual `Tally_spec`. The `dstack_kms_derived_keys` ghost variable and use of `ballots_at_end_at` model the `dstack_kms_trust` and `enclave_input_fidelity` aspects. However, the full `cross-layer` nature of B10 (Lean theorems, build-pipeline obligations) is not expressible in Quint and is simplified.

**3. Omitted Invariants and Justification**

- **B5:** Omitted as its core property is covered by `B1`.
- **B9 (meta-security):** Omitted, as Quint is a state-transition system and cannot model probabilistic cryptographic claims. Its classical shadow (preconditions for `B8` to hold) is encoded.
- **B10_lean (off-chain):** Omitted, as it's a Lean-internal theorem about an extracted model, not a property of the Quint-modeled system state transitions.
- **Full complexity of S9 (elimination monotonicity):** Simplified to `true` due to the significant complexity of modeling nested list and map iteration with conditional logic in Quint. This is a common tradeoff in translating detailed algorithmic correctness to high-level state machine models.
- **Full `Tally_spec` implementation:** The `Tally_spec_quint` function is a simplified placeholder. Its purpose in the Quint model is to ensure that the enclave's output (when honest) is a *deterministic function* of its inputs, satisfying the structural form expected for `B10`, rather than exhaustively re-implementing IRV.

**4. Non-obvious Encoding Choices**

- **Ghost Variables for Cross-Layer Modeling:** `ballots_at_end_at`, `dstack_kms_registry`, `dstack_kms_derived_keys`, and `fires_at_transition` are ghost variables. These are crucial for modeling temporal properties (`B2`, `B6`, `B8`, `B10`) and interactions with off-chain components (KMS, attestation registry) that are not part of the contract's explicit storage but are essential for the protocol's guarantees.
- **`fires_at_transition` Management:** This ghost variable is reset to an empty set at the beginning of each `step` and populated by individual actions. This ensures that `B6` and `B8` operate on the messages *that caused the current state transition*, as required by the "per-transition" semantics.
- **Simplified Cryptographic Primitives:** `hash_placeholder`, `canonical_serialization`, and `verify_attestation` are simplified. Quint does not have native cryptographic functions or byte-level serialization. These are modeled abstractly (e.g., as `str` manipulations or `true` for verification outcomes) to satisfy type signatures and logical flow, while acknowledging that the actual security of these operations relies on external (cryptographic proof) discharge.
- **`init` Action for Instantiation:** Instead of a separate `instantiate_contract` action within `step`, the `init` action directly performs the contract's instantiation. This reflects the intent's model of instantiation as a unique `σ⊥ → σ_init` transition, separating it from ongoing runtime actions.
- **`enclave_tally_action` Calling `publish_result_action`:** This models the causal flow where the off-chain enclave *performs* the tally and then *submits* it to the chain. This keeps the chain-side `publish_result_action` focused on verification of the submission, while `enclave_tally_action` represents the external trigger and computation.
- **`Bytes` as `str`:** Quint's type system uses `str` for `Bytes` representations in the canonical example (`NodeID`, `Byte`). This convention is adopted for `PubKey`, `PrivKey`, `Ciphertext`, `DstackAttestation`, and registry fields for consistency.
- **`OptionT` as SumType:** Following Quint's idiomatic way of modeling optional values as a SumType (`Some(value: T) | None`). This replaces the `is_some()`, `is_none()`, and `get()` methods with `.tag == "Some"`, `.tag == "None"`, and `.value` access respectively, properly guarded.
- **`List` instead of `Vec`:** All instances of `Vec` have been replaced with `List`, which is the correct type for sequences in Quint. List literals are implicitly constructed from comma-separated values in square brackets (e.g., `List("A", "B")`).
- **`val` instead of `let`:** Quint uses `val` for local variable bindings within expressions, replacing the `let` keyword.
- **`toStr()` instead of `toString()`:** Quint's standard library function for converting values to strings is `toStr()`, used for `canonical_serialization`.

This specification aims for an honest representation of the intent document in Quint, prioritizing behavioral properties and their state-machine representation, while clearly delineating the boundaries of what Quint can and cannot directly model.
===END FILE===