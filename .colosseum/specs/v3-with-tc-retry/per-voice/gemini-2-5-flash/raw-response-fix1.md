===BEGIN FILE: rcv.qnt===
// -*- mode: Quint; -*-
module rcv {

  // --- Types ---

  type Addr = str
  type Timestamp = int // Represents env.block.time
  type PubKey = str // Simplified representation of a public key
  type Bytes = str // Simplified representation of encrypted preferences or other byte strings

  // TallyResult schema per Section 2.5
  type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, int]], // Map<Addr, Nat>
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: int, // Nat
    ballots_dropped: int, // Nat
    dropped_voters: List[Addr],
    non_voters: List[Addr],
  }

  // Custom Option type for TallyResult
  type TallyResultOption = { isSome: bool, val: TallyResult }
  // Custom Option type for Addr
  type AddrOption = { isSome: bool, val: Addr }

  // --- Parameters (Consts) ---
  // These will be concrete in main.qnt
  const CANDIDATES: List[Addr]
  const START_AT: Timestamp
  const END_AT: Timestamp
  const DSTACK_KMS_KEY_DERIVABLE: bool // Abstract representation of KMS check
  const ENCLAVE_IDENTITY_MATCHES: bool // Abstract representation of attested enclave identity check
  const ZKDCAP_PROOF_VERIFIES: bool // Abstract representation of zkdcap proof verification
  const TDX_QUOTE_VALIDATES: bool // Abstract representation of TDX quote validation
  const DOMAIN_SEPARATION_TAG: Bytes // For B8 clause (c)
  const CONTRACT_ADDR: Addr // For B8 clause (c)

  // --- State Variables ---

  var candidates_s: List[Addr] // `candidates: Vec<Addr>` (immutable once set, but must be in state)
  var start_at_s: Timestamp
  var end_at_s: Timestamp
  var enclave_pubkey_s: PubKey // Simplified
  var ballots: Map[Addr, Bytes] // candidate -> encrypted_preferences; absent = no vote
  var tally_result: TallyResultOption // None until published; immutable once set
  var env_block_time: Timestamp // Current block time
  var ballots_at_end_at: Map[Addr, Bytes] // Ghost variable for B2

  // --- Ghost Variables for Action Attribution (for B6) ---
  var submit_ballot_fired: bool
  var submit_ballot_sender: AddrOption


  // --- Pure Predicates and Helper Functions ---

  // Derived states (Section 2.5)
  pure def isCreated(): bool = env_block_time < start_at_s
  pure def isVoting(): bool = start_at_s <= env_block_time and env_block_time < end_at_s
  pure def isTallying(): bool = env_block_time >= end_at_s and not(tally_result.isSome)
  pure def isResolved(): bool = tally_result.isSome

  // S1: non-empty candidates
  pure def S1_nonEmptyCandidates(cands: List[Addr]): bool = size(cands) >= 1

  // S2: distinct candidates
  pure def S2_distinctCandidates(cands: List[Addr]): bool = Set(cands).size() == size(cands)

  // S3: well-ordered voting window
  pure def S3_wellOrderedVotingWindow(): bool = start_at_s < end_at_s

  // S4: ballot keys are candidates
  pure def S4_ballotKeysAreCandidates(): bool = ballots.keys().subset(Set(candidates_s))

  // S6: winner well-formedness (from Block 6 Requires)
  pure def S6_winnerWellFormed(tally: TallyResult): bool =
    Set(tally.winners).subset(Set(candidates_s)) and size(tally.winners) >= 1 and size(tally.winners) <= size(candidates_s)

  // S7: tally count conservation (from Block 6 Requires)
  pure def S7_tallyCountConservation(tally: TallyResult): bool =
    tally.ballots_tallied + tally.ballots_dropped + size(tally.non_voters) == size(candidates_s)

  // S8: per-round count consistency (from Block 6 Requires)
  pure def S8_perRoundCountConsistency(tally: TallyResult): bool =
    tally.per_round_counts.all(rc => rc.values().fold(0, (a, b) => a + b) == tally.ballots_tallied)

  // S9: elimination monotonicity (simplified as discussed)
  pure def S9_eliminationMonotonicity(tally: TallyResult): bool =
    (0.to(size(tally.eliminated_by_round) - 1)).all(i =>
      val candidates_elim_in_round_i = Set(tally.eliminated_by_round.get(i)),
      (i + 1).to(size(tally.per_round_counts) - 1).all(j =>
        val round_counts_j_keys = tally.per_round_counts.get(j).keys(),
        candidates_elim_in_round_i.intersect(round_counts_j_keys).size() == 0
      )
    )

  // S10: resolution implies past end_at (derived from B3)
  pure def S10_resolutionImpliesPastEndAt(): bool =
    tally_result.isSome => env_block_time >= end_at_s

  // Block 6 additional set relations well-formedness (Section 2.5)
  pure def B6_wellFormedSetRelations(tally: TallyResult): bool =
    Set(tally.non_voters) == Set(candidates_s).diff(ballots.keys())
    and Set(tally.dropped_voters).subset(ballots.keys())
    and Set(tally.dropped_voters).intersect(Set(tally.non_voters)).size() == 0
    and size(tally.dropped_voters) == tally.ballots_dropped

  // B8 clause (c) - Attested user_data matches hash of canonical_serialization
  // Abstracted SHA-256 and canonical serialization for Quint.
  pure def user_data_hash_matches(tally: TallyResult): bool =
    true // placeholder, in real system this would be a hash comparison


  // --- Actions ---

  action init = all {
    candidates_s' = List(),
    start_at_s' = 0,
    end_at_s' = 0,
    enclave_pubkey_s' = "",
    ballots' = Map(),
    tally_result' = { isSome: false, val: TallyResult(winners: List(), per_round_counts: List(), eliminated_by_round: List(), ballots_tallied: 0, ballots_dropped: 0, dropped_voters: List(), non_voters: List()) },
    env_block_time' = 0,
    ballots_at_end_at' = Map(),
    submit_ballot_fired' = false,
    submit_ballot_sender' = { isSome: false, val: "" },
  }

  // Block 1: instantiate
  // Trigger: `instantiate { candidates, start_at, end_at }`
  action instantiate(new_candidates: List[Addr], new_start_at: Timestamp, new_end_at: Timestamp): bool = all {
    // Requires:
    // Contract must not be already instantiated (implied by starting from sigma_perp)
    size(candidates_s) == 0,
    size(ballots) == 0,
    not(tally_result.isSome),

    S1_nonEmptyCandidates(new_candidates),
    S2_distinctCandidates(new_candidates),
    new_start_at > env_block_time,
    new_end_at > new_start_at,
    DSTACK_KMS_KEY_DERIVABLE, // DstackKeyManager-issued keypair derivable

    // Produces:
    candidates_s' = new_candidates,
    start_at_s' = new_start_at,
    end_at_s' = new_end_at,
    enclave_pubkey_s' = "generated_pubkey", // Simplified
    ballots' = Map(),
    tally_result' = { isSome: false, val: tally_result.val }, // Keep initial empty TallyResult
    env_block_time' = env_block_time, // Time does not advance
    ballots_at_end_at' = Map(), // Initialized empty, will be set when time crosses end_at_s
    submit_ballot_fired' = false, // Reset ghost flags
    submit_ballot_sender' = { isSome: false, val: "" },
  }

  // Block 2 & 4: time advances (implicit, no handler)
  action advanceTime(new_time: Timestamp): bool = all {
    new_time > env_block_time, // Time must strictly advance

    env_block_time' = new_time,
    candidates_s' = candidates_s,
    start_at_s' = start_at_s,
    end_at_s' = end_at_s,
    enclave_pubkey_s' = enclave_pubkey_s,
    ballots' = ballots,
    tally_result' = tally_result,
    // Ghost variable for B2: Capture ballots state when crossing end_at_s
    ballots_at_end_at' =
      if (env_block_time < end_at_s and new_time >= end_at_s) then ballots else ballots_at_end_at,
    submit_ballot_fired' = false, // Reset ghost flags
    submit_ballot_sender' = { isSome: false, val: "" },
  }

  // Block 3: submit_ballot
  action submit_ballot(sender: Addr, encrypted_prefs: Bytes): bool = all {
    submit_ballot_fired' = true,
    submit_ballot_sender' = { isSome: true, val: sender },
    // Requires:
    isVoting(),
    sender.in(Set(candidates_s)),
    encrypted_prefs != "", // non-empty

    // Produces:
    ballots' = ballots.put(sender, encrypted_prefs),
    candidates_s' = candidates_s,
    start_at_s' = start_at_s,
    end_at_s' = end_at_s,
    enclave_pubkey_s' = enclave_pubkey_s,
    tally_result' = tally_result,
    env_block_time' = env_block_time,
    ballots_at_end_at' = ballots_at_end_at,
  }

  // Block 5: close_and_tally (any chain address may call)
  action close_and_tally(): bool = all {
    // Requires:
    isTallying(), // derived state is Tallying

    // Produces: (no storage change, only event emit, so state is unchanged)
    candidates_s' = candidates_s,
    start_at_s' = start_at_s,
    end_at_s' = end_at_s,
    enclave_pubkey_s' = enclave_pubkey_s,
    ballots' = ballots,
    tally_result' = tally_result,
    env_block_time' = env_block_time,
    ballots_at_end_at' = ballots_at_end_at,
    submit_ballot_fired' = false, // Reset ghost flags
    submit_ballot_sender' = { isSome: false, val: "" },
  }

  // Block 6: publish_result
  action publish_result(new_tally: TallyResult, attestation_blob: Bytes): bool = all {
    // Requires:
    isTallying(),
    // B8 clauses: Attestation verifies and identity matches
    TDX_QUOTE_VALIDATES,
    ZKDCAP_PROOF_VERIFIES,
    ENCLAVE_IDENTITY_MATCHES,
    user_data_hash_matches(new_tally), // Simplified
    // Well-formedness checks from Block 6
    S6_winnerWellFormed(new_tally),
    S7_tallyCountConservation(new_tally),
    S8_perRoundCountConsistency(new_tally),
    S9_eliminationMonotonicity(new_tally), // Use the simplified S9
    B6_wellFormedSetRelations(new_tally),

    // Produces:
    tally_result' = { isSome: true, val: new_tally },
    candidates_s' = candidates_s,
    start_at_s' = start_at_s,
    end_at_s' = end_at_s,
    enclave_pubkey_s' = enclave_pubkey_s,
    ballots' = ballots,
    env_block_time' = env_block_time,
    ballots_at_end_at' = ballots_at_end_at,
    submit_ballot_fired' = false, // Reset ghost flags
    submit_ballot_sender' = { isSome: false, val: "" },
  }

  // --- Behavioral Invariants (Section 3.2) ---

  // B1: tally_result monotone-once-set
  def B1_tallyResultMonotoneOnceSet(): bool =
    tally_result.isSome => next.tally_result.isSome and next.tally_result.val == tally_result.val

  // B2: no late ballots
  def B2_noLateBallots(): bool =
    env_block_time >= end_at_s => ballots == ballots_at_end_at

  // B3: no premature tally (causal)
  def B3_noPrematureTally(): bool =
    not(tally_result.isSome) and next.tally_result.isSome => env_block_time' >= end_at_s

  // B4: no premature voting
  def B4_noPrematureVoting(): bool =
    (ballots' != ballots) => (start_at_s <= env_block_time and env_block_time < end_at_s)

  // B5: publish_result fires at most once (derived from B1 + Block 6)
  def B5_publishResultFiresAtMostOnce(): bool = B1_tallyResultMonotoneOnceSet()

  // B6: ballot writer is the ballot voter (temporal)
  def B6_ballotWriterIsVoter(): bool =
    (ballots' != ballots) =>
      submit_ballot_fired and
      submit_ballot_sender.isSome and
      // No keys are removed, and a key must be updated/added by sender
      (ballots.keys().diff(ballots'.keys()).size() == 0) and
      (
        // Either a new key was added by the sender
        (ballots'.keys().diff(ballots.keys()) == Set(submit_ballot_sender.val)) or
        // Or an existing key was updated by the sender
        (ballots'.keys() == ballots.keys() and ballots.get(submit_ballot_sender.val) != ballots'.get(submit_ballot_sender.val))
      )

  // B7: terminal-state immutability
  def B7_terminalStateImmutability(): bool =
    isResolved() => next.isResolved()

  // --- Composite Invariant ---
  val all_invariants = all {
    // Structural Invariants
    S1_nonEmptyCandidates(candidates_s),
    S2_distinctCandidates(candidates_s),
    S3_wellOrderedVotingWindow(),
    S4_ballotKeysAreCandidates(),
    S10_resolutionImpliesPastEndAt(),
    // S6-S9 and B6_wellFormedSetRelations are post-conditions of 'publish_result',
    // so they hold *if* tally_result.isSome()
    tally_result.isSome => S6_winnerWellFormed(tally_result.val),
    tally_result.isSome => S7_tallyCountConservation(tally_result.val),
    tally_result.isSome => S8_perRoundCountConsistency(tally_result.val),
    tally_result.isSome => S9_eliminationMonotonicity(tally_result.val),
    tally_result.isSome => B6_wellFormedSetRelations(tally_result.val),

    // Behavioral Invariants
    B1_tallyResultMonotoneOnceSet(),
    B2_noLateBallots(),
    B3_noPrematureTally(),
    B4_noPrematureVoting(),
    B5_publishResultFiresAtMostOnce(),
    B6_ballotWriterIsVoter(),
    B7_terminalStateImmutability(),
  }

  // --- Reachability Witnesses (must be VIOLATED) ---

  // witness_resolution_reachable: True if the system has NOT reached the resolved state.
  // Quint will find a counterexample where it becomes False (i.e., state is Resolved).
  val witness_resolution_reachable: bool = not(isResolved())

  // witness_ballot_submittable: True if no ballots have been submitted.
  // Quint will find a counterexample where it becomes False (i.e., ballots is non-empty).
  val witness_ballot_submittable: bool = ballots.keys().size() == 0

  // witness_end_at_crossing: True if env.block.time has NOT crossed end_at_s.
  // Quint will find a counterexample where it becomes False (i.e., env.block.time >= end_at_s).
  val witness_end_at_crossing: bool = env_block_time < end_at_s


  // --- Step Relation ---

  action step = any {
    // Advance time non-deterministically
    val next_time_diff = nondet(1.to(10)),
    advanceTime(env_block_time + next_time_diff),

    // Instantiation can only happen once (checked in action itself with `size(candidates_s) == 0`)
    // Use the `const` CANDIDATES for the parameters of the instantiate action, so it's bound by main.qnt
    instantiate(CANDIDATES, START_AT, END_AT),

    // Candidate submits ballot (must be in voting window)
    val sender_addr = oneOf(candidates_s), // Pick from *current* candidates_s
    val encrypted_ballot = nondet("ballot_a", "ballot_b"),
    submit_ballot(sender_addr, encrypted_ballot),

    // Anyone can call close_and_tally during tallying
    close_and_tally(),

    // Enclave publishes result during tallying
    // The mock must generate results that satisfy the preconditions for publish_result.
    // Simplified generation for reachability; values need to be consistent to pass S6-S9 checks.
    val all_voters = ballots.keys().toList(),
    val num_submitted_ballots = size(all_voters),

    val mock_non_voters_list = candidates_s.filter(c => not(c.in(all_voters.toSet()))),
    val num_non_voters = size(mock_non_voters_list),

    val possible_dropped_count = nondet(0.to(num_submitted_ballots)),
    // Manual 'take' implementation for Quint lists
    val actual_mock_dropped_voters_list = all_voters.filter( (v,i) => i < possible_dropped_count ),


    val actual_tallied_count = num_submitted_ballots - possible_dropped_count,
    
    // Ensure at least one winner for S6, if possible.
    val possible_winners = candidates_s.filter(c => not(c.in(actual_mock_dropped_voters_list.toSet()))).filter(c => not(c.in(mock_non_voters_list.toSet()))),
    val mock_winner = if (size(possible_winners) > 0) then oneOf(possible_winners) else oneOf(candidates_s),

    // per_round_counts sum to actual_tallied_count (S8)
    val mock_per_round_map = Map(mock_winner -> actual_tallied_count),
    val mock_per_round_list = List(mock_per_round_map),

    val mock_tally_for_publish: TallyResult = {
      winners: List(mock_winner),
      per_round_counts: mock_per_round_list,
      eliminated_by_round: List(List()), // Simple case: no eliminations for this mock
      ballots_tallied: actual_tallied_count,
      ballots_dropped: possible_dropped_count,
      dropped_voters: actual_mock_dropped_voters_list,
      non_voters: mock_non_voters_list,
    },
    publish_result(mock_tally_for_publish, "attestation_payload")
  }
}
===END FILE===
===BEGIN FILE: main.qnt===
// -*- mode: Bluespec; -*-

module main {
  import rcv(
    CANDIDATES = List("A", "B", "C"),
    START_AT = 10,
    END_AT = 20,
    DSTACK_KMS_KEY_DERIVABLE = true, // Assume KMS is always available and correct for this model
    ENCLAVE_IDENTITY_MATCHES = true, // Assume enclave identity always matches
    ZKDCAP_PROOF_VERIFIES = true, // Assume ZK proof always verifies
    TDX_QUOTE_VALIDATES = true, // Assume TDX quote always validates
    DOMAIN_SEPARATION_TAG = "DST_VERIFIED_RCV_TALLY_V1", // Example tag
    CONTRACT_ADDR = "rcv_contract_address_xyz" // Example contract address
  ).* from "./rcv"
}
===END FILE===
===BEGIN FILE: design-notes.md===
### Design Notes for verified-rcv Quint Specification

This Quint specification for the `verified-rcv` system aims to capture the core state transitions and invariants as described in the intent document (version 0.3.1). Due to Quint's limitations in directly modeling cryptographic primitives, off-chain systems, or probabilistic claims, some aspects are abstracted or represented as classical-Prop shadows.

#### §2.5 Blocks to Quint Actions Mapping

- **Block 1: instantiate** -> `action instantiate(new_candidates, new_start_at, new_end_at)`
- **Block 2 & 4: time advances (implicit)** -> `action advanceTime(new_time)`
- **Block 3: submit_ballot** -> `action submit_ballot(sender, encrypted_prefs)`
- **Block 5: close_and_tally** -> `action close_and_tally()`
- **Block 6: publish_result** -> `action publish_result(new_tally, attestation_blob)`
- **Block E1: enclave tally computation (off-chain)**: This block's *effect* on the chain state is primarily via the inputs to `publish_result`. The `publish_result` action's `new_tally` parameter represents the `TallyResult` computed by the enclave, and its `Requires` clauses encode the chain's validation of this output. The complex `Tally_spec` algorithm (Section 2.5) is not modeled directly in Quint, as its correctness is a Lean-discharge obligation. Instead, the `publish_result` action assumes the provided `new_tally` satisfies the well-formedness properties.

#### Encoding of Invariants (§3.1 Structural, §3.2 Behavioral)

**Structural Invariants (§3.1):**

- **S1 (non-empty candidates):** `pure def S1_nonEmptyCandidates(cands: List[Addr]): bool = size(cands) >= 1`. This predicate is called in `instantiate` with `new_candidates` and in `all_invariants` with `candidates_s`.
- **S2 (distinct candidates):** `pure def S2_distinctCandidates(cands: List[Addr]): bool = Set(cands).size() == size(cands)`. Similar to S1, this is called with `new_candidates` and `candidates_s`.
- **S3 (well-ordered voting window):** `pure def S3_wellOrderedVotingWindow(): bool = start_at_s < end_at_s`
- **S4 (ballot keys are candidates):** `pure def S4_ballotKeysAreCandidates(): bool = ballots.keys().subset(Set(candidates_s))`
- **S5 (terminality of resolution, handler-set property):** The intent clarifies S5 as a handler-set property. The `tally_result` state variable is initialized to `{ isSome: false, ... }` and can only be set to `{ isSome: true, ... }` by `publish_result`. The `publish_result` action's `isTallying()` precondition ensures it only fires when `tally_result.isSome` is false. Once `tally_result` is set, `isTallying()` becomes false, preventing further modification. The trajectory aspect ("once `Some`, always `Some` with the same value") is covered by **B1**. Thus, S5's implications are covered by action guards and B1.
- **S6-S9 (winner well-formedness, tally count conservation, per-round count consistency, elimination monotonicity):** These are encoded as `pure def` predicates taking a `TallyResult`. They are asserted in the `publish_result` action's `Requires` clause. For `all_invariants`, these are conditionally checked `if tally_result.isSome`, ensuring that any published `tally_result` (which is then immutable per B1) always satisfies these properties.
- **S10 (resolution implies past end_at):** `pure def S10_resolutionImpliesPastEndAt(): bool = tally_result.isSome => env_block_time >= end_at_s`. This is included in `all_invariants`.

**Behavioral Invariants (§3.2):**

- **B1 (tally_result monotone-once-set):** `def B1_tallyResultMonotoneOnceSet(): bool = tally_result.isSome => next.tally_result.isSome and next.tally_result.val == tally_result.val`. This uses Quint's `next` operator for temporal reasoning.
- **B2 (no late ballots):** `def B2_noLateBallots(): bool = env_block_time >= end_at_s => ballots == ballots_at_end_at`. A ghost variable `ballots_at_end_at` is introduced to capture the `ballots` state when `env_block_time` first crosses `end_at_s`.
- **B3 (no premature tally):** `def B3_noPrematureTally(): bool = not(tally_result.isSome) and next.tally_result.isSome => env_block_time' >= end_at_s`.
- **B4 (no premature voting):** `def B4_noPrematureVoting(): bool = (ballots' != ballots) => (start_at_s <= env_block_time and env_block_time < end_at_s)`.
- **B5 (publish_result fires at most once):** This is stated as a derived corollary of B1 + Block 6. Its essence is captured by **B1** and the fact that `publish_result` is guarded by `isTallying()` (which includes `not(tally_result.isSome)`). It's explicitly tied to `B1_tallyResultMonotoneOnceSet()` in the spec.
- **B6 (ballot writer is the ballot voter):** `def B6_ballotWriterIsVoter(): bool = (ballots' != ballots) => submit_ballot_fired and submit_ballot_sender.isSome and ...`. This is modeled by ensuring that `ballots` can *only* change via the `submit_ballot` action, and that action enforces the `msg.sender` as the key. `submit_ballot_fired` and `submit_ballot_sender` ghost variables track the last successful `submit_ballot` operation to attribute changes. The `∃!` uniqueness is reflected by Quint's atomic actions, where only one action fires per step, and that action's parameters are the "message".
- **B7 (terminal-state immutability):** `def B7_terminalStateImmutability(): bool = isResolved() => next.isResolved()`.
- **B8 (attestation-binds-tally):** This is primarily encoded in the `publish_result` action's `Requires` clause. Abstract boolean constants (`TDX_QUOTE_VALIDATES`, `ZKDCAP_PROOF_VERIFIES`, `ENCLAVE_IDENTITY_MATCHES`) represent the verification outcomes. `user_data_hash_matches(new_tally)` is a placeholder for the actual cryptographic hash comparison, as Quint cannot perform SHA-256 or Borsh serialization. The specific content of the `attestation_blob` is not modeled.
- **B9 (B8 negligibility-budget decomposition):** This is a `meta-security` invariant, fundamentally probabilistic and beyond the scope of Quint's deterministic state-machine modeling. It is explicitly omitted from the Quint spec and noted here. Its preconditions (`image_registration_honest`, `circuit_equivalence_honest`) are also not directly modeled but are assumed to hold (or are represented by `ENCLAVE_IDENTITY_MATCHES` and `ZKDCAP_PROOF_VERIFIES` for their classical shadow).
- **B10 (tally-correctness) & B10_lean (Lean-internal image-IO obligation):** These are `cross-layer` and `off-chain` invariants respectively, discharged by the Lean proof and build pipeline. Quint cannot directly verify `tally = Tally_spec(...)`. Instead, the `publish_result` action *assumes* the provided `new_tally` is the correct `Tally_spec` output and enforces all the *well-formedness properties* (S6-S9, B6_wellFormedSetRelations) that `Tally_spec` is expected to guarantee for its output. The `dstack_kms_derived` and `enclave_input_fidelity` parts of B10 are not directly encoded.

#### Omitted Invariants and Justification

- **S5 (terminality of resolution, handler-set property):** While the spirit of S5 (at-most-once write to `tally_result`) is fully captured by action guards and B1, its statement as a purely static "handler-set property" from §3.1 is not translated as a direct `val` in `all_invariants` due to Quint's focus on state-based properties over a trace.
- **B9 (B8 negligibility-budget decomposition):** Omitted as it is a `meta-security` (probabilistic) claim outside Quint's modeling capabilities.
- **B10 (tally-correctness) & B10_lean (Lean-internal image-IO obligation):** The full semantic correctness of `Tally_spec` is off-chain (Lean). Quint captures the *syntactic well-formedness* requirements of the `TallyResult` as preconditions for `publish_result` and assumes the off-chain system produces a semantically correct result satisfying these. The `dstack_kms_derived` and `enclave_input_fidelity` parts of B10 are not directly encoded.

#### Non-Obvious Encoding Choices

- **Custom `Option` Types:** Quint does not have a built-in `Option` type. `TallyResultOption = { isSome: bool, val: TallyResult }` and `AddrOption = { isSome: bool, val: Addr }` are defined and used to represent optional values, replacing `Option[T]` and its `isSome()`, `get()` methods.
- **Time Advancement (`advanceTime`):** Modeled as a non-deterministic jump in time to simplify exploration of the state space. It includes logic to capture `ballots_at_end_at` at the `end_at_s` boundary.
- **Ghost Variables for Action Attribution:** `submit_ballot_fired` and `submit_ballot_sender` are introduced to track if the `submit_ballot` action fired and by whom in the current step. This is necessary for `B6` to attribute ballot changes. These variables are reset (`false` or `{ isSome: false, val: "" }`) by any other action or in the subsequent step, ensuring only one action is attributed per transition. `ballots_at_end_at` is another ghost variable crucial for `B2`.
- **Abstracting Cryptography and Off-chain Components:** All cryptographic operations (ECIES, SHA-256, TDX quote, ZK proof) and the Dstack KMS are abstracted as boolean `const` parameters (e.g., `TDX_QUOTE_VALIDATES`) or `pure def`s returning `true` (`user_data_hash_matches`). This allows focusing on the protocol logic assuming honest/valid components, without delving into cryptographic detail beyond Quint's scope.
- **Simplified TallyResult Generation in `step`:** For `publish_result` to fire, a `new_tally` must be provided that satisfies its `Requires` clause (well-formedness checks). The `step` action non-deterministically constructs a `mock_tally_for_publish` designed to satisfy these properties under simplified assumptions (e.g., usually one winner, no eliminations in the mock, `ballots_tallied + ballots_dropped + len(non_voters)` summing to `len(candidates)` as per S7). This simplification is crucial for model checking to progress into the `Resolved` state without getting stuck on complex `Tally_spec` logic. The manual implementation of `take` using `filter` and index access for lists is an adaptation to Quint's `List` capabilities.

This specification adheres to the output contract and provides an idiomatic Quint representation of the `verified-rcv` intent, highlighting where abstractions are made due to modeling scope.
===END FILE===