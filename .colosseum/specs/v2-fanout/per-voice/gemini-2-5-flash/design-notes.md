This Quint specification for the `verified-rcv` system is designed to adhere to the intent document while providing an idiomatic Quint encoding suitable for model checking.

### Mapping §2.5 Blocks to Actions

The protocol's structured behavior blocks from Section 2.5 are mapped to Quint actions as follows:

-   **Block 1: instantiate** -> `block1_instantiate` action. This action creates a new contract instance and populates its initial state variables within the global `rcv_contracts` map. The global `rcv_init` action initializes the entire system (empty contracts map).
-   **Blocks 2 & 4: time advances** (implicit) -> `time_advances` action. This action updates the `global_env_block_time` and triggers re-evaluation of derived states for all contracts, also handling the ghost variable `ballots_at_end_at`.
-   **Block 3: submit_ballot** -> `block3_submit_ballot` action. This updates the `ballots` map for a specific contract.
-   **Block 5: close_and_tally** -> `block5_close_and_tally` action. This is explicitly modeled as an idempotent action that causes no state changes, reflecting the intent's description of it emitting an event but not mutating storage.
-   **Block 6: publish_result** -> `block6_publish_result` action. This action sets the `tally_result` for a contract and implements all required attestation and tally well-formedness checks in its `requires` clause.
-   **Block E1: enclave tally computation** (off-chain) -> Represented abstractly by the `Tally_spec` pure definition and used within `block6_publish_result` to generate a `TallyResult` for the model.

### Encoding of Invariants

All structural (S-series) and behavioral (B-series) invariants that can be directly encoded in Quint are included in the `all_invariants` definition.

#### Structural Invariants (§3.1)

-   **S1-S4, S6-S9**: These are encoded as `val` definitions that iterate over all active `rcv_contracts` and check the respective properties on each contract's state. `s6_winners_well_formed`, `s7_count_conservation`, `s8_per_round_count_consistency`, `s9_elimination_monotonicity_def`, and `block6_set_relations` are helper `pure def`s that capture the detailed logic from the `TallyResult` schema and Block 6 requirements.
-   **S5 (derived): terminality of resolution**: The intent clarifies S5 as a "handler-set property" (a meta-property about the code structure) and its behavioral consequence as B1. In Quint, meta-properties of code are not directly expressible at runtime. Thus, S5 is encoded as `true`, and its runtime behavior (once `tally_result` is `Some`, it's immutable) is fully captured by `B1`.
-   **S10 (derived): resolution implies past end_at**: Encoded as a `val` that checks if `tally_result` is `Some`, then `global_env_block_time` must be past `end_at` for that contract.

#### Behavioral Invariants (§3.2)

-   **B1: tally_result monotone-once-set**: Encoded using Quint's `always` operator, ensuring that if `tally_result` is `Some` in the current state, it remains `Some` with the same value in the next state.
-   **B2: no late ballots**: This requires a ghost variable, `ballots_at_end_at`, which captures the `ballots` snapshot when a contract first transitions from `Voting` to `Tallying`. The `time_advances` action is responsible for setting this ghost variable. The invariant then asserts that once `global_env_block_time` is past `end_at`, `ballots` must equal `ballots_at_end_at`.
-   **B3: no premature tally**: Encoded using `always` to check that if a contract's `tally_result` transitions from `None` to `Some` (i.e., `publish_result` fired successfully for that contract), then `global_env_block_time` must be greater than or equal to `end_at`.
-   **B4: no premature voting**: Encoded using `always` to ensure that if any contract's `ballots` map changes, then `global_env_block_time` must be within that contract's `[start_at, end_at)` window.
-   **B5 (derived): publish_result fires at most once**: Similar to S5, this is a derived corollary of B1 and the action guard in `block6_publish_result` (which checks `contract.tally_result.is_none()`). It is encoded as `true`, as its behavioral enforcement is implicit.
-   **B6: ballot writer is the ballot voter**: This is encoded using `always` and an action label `block3_submit_ballot_label(contract_addr, sender)`. It asserts that if a contract's `ballots[k]` changes, it must be attributed to a `block3_submit_ballot` action with `sender` equal to `k` for that specific `contract_addr`. The "unique" aspect is handled by Quint's single-step semantics, where a specific action for a specific `(contract_addr, sender)` pair causes the state change.
-   **B7: terminal-state immutability**: Encoded using `always` to ensure that once a contract's `CurrentState()` is "Resolved", it remains "Resolved".
-   **B8: attestation-binds-tally**: This is a `temporal` and `cross-layer` invariant. Quint can only encode its *classical-Prop shadow*. The core of B8 (clauses a, b, c, d) is implemented as `requires` clauses within the `block6_publish_result` action. Since Quint only models successful transitions, if `block6_publish_result` fires, these conditions must have been met. Therefore, `b8_attestation_binds_tally_shadow` is set to `always (true)`, with the understanding that the actual validation logic resides in the action's guards.

### Omitted Invariants

-   **B9: B8 negligibility-budget decomposition** (`meta-security`): This invariant involves probabilistic claims over adversarial advantage functions and is a property of the cryptographic security proofs, not directly verifiable by a state-machine model checker like Quint. It is omitted, with `b9_b10_abstract_placeholder = true` included in `all_invariants`.
-   **B10: tally-correctness** (`cross-layer`): This invariant asserts that the published tally equals `Tally_spec` applied to the frozen ballot set. It relies on off-chain `privkey` values, `dstack_kms_derived`, `enclave_input_fidelity`, and `Tally_spec` itself. While `Tally_spec` is included as an abstract `pure def`, the full B10 claim is a complex cross-layer composition (Lean theorem, build-system binding, operational assumptions). It cannot be directly verified by Quint; hence, `b9_b10_abstract_placeholder = true` is used.
-   **B10_lean: Lean-internal image-IO obligation** (`off-chain`): This is a Lean-internal theorem proving the `EnclaveImage` equals `Tally_spec`. Being entirely off-chain, it is not part of the Quint specification.

### Non-Obvious Encoding Choices

1.  **Global vs. Contract-Specific State**: `rcv_contracts` (a map of `Addr` to `RCVContractState`) and `next_contract_id` are used to manage multiple contract instances within the single Quint module, following the canonical example. `global_env_block_time` tracks the chain's global clock.
2.  **`ballots_at_end_at` Ghost Variable**: The `b2_no_late_ballots` invariant requires a snapshot of `ballots` at the `end_at` boundary. A ghost variable (`ballots_at_end_at`) within `RCVContractState` is introduced and updated by the `time_advances` action when a contract transitions from `Voting` to `Tallying`.
3.  **Abstract Placeholders for Cryptography and `Tally_spec`**:
    -   Functions like `is_tdx_quote_validates`, `is_zkdcap_proof_verifies`, `sha256`, and `canonical_serialization` are defined as `pure def`s that always return `true` or produce simplified string outputs. This allows the Quint model to check the *flow* and *structure* of these operations (e.g., that the hash output is used correctly) without implementing the actual cryptographic primitives.
    -   `Tally_spec` is an abstract `pure def` that returns a structurally valid (but dummy) `TallyResult`. This allows `block6_publish_result` and the `step` action to produce valid `TallyResult` objects for model checking.
4.  **`IS_IMAGE_REGISTRATION_HONEST` Constant**: The `image_registration_honest(σ)` predicate from Section 6.1 (referenced by B9) is a dynamic check on the chain's registry. For Quint, it is simplified to a `const IS_IMAGE_REGISTRATION_HONEST: bool` that is set in `main.qnt`, allowing the model to assume honesty (or dishonesty) in a static manner.
5.  **Action Labels for Temporal Invariants**: Temporal invariants (`always (...)`) often refer to specific actions or their parameters (`block3_submit_ballot_label(contract_addr, k)`). These labels capture the "at the firing transition" semantics of certain invariants (e.g., B3, B4, B6) by associating the invariant with the specific action that causes the state change.
6.  **`step` Action for Model Checking**: The `step` action is structured to first instantiate a single contract if none exists, and then operate on that single contract instance. This limits the state space for model checking, making initial runs tractable. Arbitrary instantiation parameters are replaced with `nondet` and example values.
7.  **`Bytes32` Type**: `Bytes32` is mapped to Quint's `str` type, as Quint does not have a native fixed-size byte array type. Operations like `sha256` are modeled as `str` manipulations. This is a common simplification in Quint models where byte-level semantics are not the primary focus of verification.

These choices aim for a faithful and executable representation of the intent document within the capabilities and idioms of Quint, while clearly delineating the boundaries of what can be directly verified by the model checker versus what relies on external proofs or assumptions.