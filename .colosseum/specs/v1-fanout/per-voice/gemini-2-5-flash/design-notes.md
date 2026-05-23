### Design Notes for `verified-rcv` Quint Specification

This Quint specification translates the `verified-rcv` intent document into an executable model, focusing on the state machine, transitions, and the classical shadows of behavioral invariants.

#### 1. Mapping §2.5 Blocks to Actions

The core state transitions and their associated `Requires`/`Forbids`/`Produces` clauses from Section 2.5 of the intent document are mapped to Quint `action` definitions as follows:

*   **Block 1: `instantiate`** -> `action instantiate(...)`: This action is parameterized by initial contract configuration and sets up the initial state, transitioning implicitly from an uninitialized state to `Created` (though Quint models `init` as the true initial state).
*   **Block 2: `time advances to start_at`** (implicit) -> `action time_advances(new_time: Timestamp)`: This single action is used for all implicit time advancements. The derived state predicates (`is_Created`, `is_Voting`, `is_Tallying`) use `env_block_time` to determine the current state.
*   **Block 3: `submit_ballot`** -> `action submit_ballot(...)`: Allows a candidate to submit an encrypted ballot, updating the `ballots` map. Includes checks for `is_Voting`, `is_candidate`, and non-empty preferences.
*   **Block 4: `time advances to end_at`** (implicit) -> `action time_advances(new_time: Timestamp)`: Handled by the generic time advancement action. The transition from `Voting` to `Tallying` is derived from `env_block_time` crossing `end_at`. This action also manages the `ballots_at_end_at` ghost variable, capturing the "snapshot" requirement.
*   **Block 5: `close_and_tally`** -> `action close_and_tally`: This action primarily emits an event off-chain; in Quint, it's modeled as a no-op on the stored state, performing only its `Requires` checks and updating the `fires_at_transition` ghost variable.
*   **Block 6: `publish_result`** -> `action publish_result(...)`: This is the terminal action, setting `tally_result` and transitioning to the `Resolved` state. It includes extensive checks for attestation validity (shadowed) and tally well-formedness via `tally_is_well_formed`.
*   **Block E1: `enclave tally computation`** (off-chain) -> Not directly an action. Its *output* and *correctness obligation* are reflected in the `publish_result` action's requirements (well-formedness, B8, B10) and the `Tally_spec_stub` function.

#### 2. Encoding of Invariants (§3.1 + §3.2)

**Structural Invariants (§3.1):**
*   `S1_non_empty_candidates`, `S2_distinct_candidates`, `S3_well_ordered_voting_window`: Encoded as direct boolean predicates on state variables.
*   `S4_ballot_keys_are_candidates`: Encoded by checking if all keys in `ballots` are present in `candidates`.
*   `S5_tally_result_write_discipline`: The intent clarifies this as a "handler-set property" (meta-property of the contract code), not a direct state invariant. Quint's model focuses on state transitions, not static code analysis. Therefore, this is *omitted as a direct invariant* and its behavioral implication ("once written, it's final") is covered by `B1_tally_result_monotone_once_set`. The `publish_result` action's `is_Tallying` guard also prevents overwrites.
*   `S6_winner_well_formedness`, `S7_tally_count_conservation`, `S8_per_round_count_consistency`, `S9_elimination_monotonicity`: Encoded as boolean predicates that apply `if is_Resolved(state)` and access `tally_result.get()`. These are implicitly part of `tally_is_well_formed` within `publish_result`.
*   `S10_resolution_implies_past_end_at`: Direct boolean predicate on `is_Resolved` and `env_block_time`.

**Behavioral Invariants (§3.2):**
*   `B1_tally_result_monotone_once_set`: Encoded using Quint's `always` temporal operator, asserting that `tally_result` does not change once set.
*   `B2_no_late_ballots`: Encoded using `always` and the `ballots_at_end_at` ghost variable, which snapshots `ballots` when `env_block_time` first crosses `end_at`.
*   `B3_no_premature_tally`: Encoded using `always` to assert that `tally_result` can only transition from `None` to `Some` if `env_block_time >= end_at`.
*   `B4_no_premature_voting`: Encoded using `always` to assert that `ballots` only changes when `env_block_time` is within `[start_at, end_at)`.
*   `B5_publish_result_fires_at_most_once`: Explicitly noted as a derived corollary of `B1` and the `publish_result` action's guards, thus *omitted as a separate Quint invariant*.
*   `B6_ballot_writer_is_the_ballot_voter`: This complex temporal invariant is encoded using `always` and the `fires_at_transition` ghost variable. It iterates over keys where `ballots[k]` changed and asserts that *exactly one* `SubmitBallot` message in `fires_at_transition` for sender `k` caused that change. The `Option<Vec<u8>>` inequality is captured by `prev_ballot_k_opt != next_ballot_k_opt` for `Option[Bytes]`.
*   `B7_terminal_state_immutability`: Encoded with `always` to assert that `Resolved` is a sink state.
*   `B8_attestation_binds_tally_shadow`: This is a `cross-layer` invariant. Quint cannot verify cryptographic proofs or off-chain data integrity directly. We encode its *classical-Prop shadow* by assuming the `DstackAttestation`'s fields reflect validity (e.g., `tdx_quote_valid: true`). The hashing of `tally_body` for `attested_user_data_lower_32_bytes` is omitted, as Quint lacks hashing primitives. `image_registration_honest(state)` is part of the `publish_result` action's preconditions.
*   `B9_B8_negligibility_budget_decomposition`: This is a `meta-security` invariant (probabilistic, with quantifiers over adversaries and security parameters). It is *omitted* from the Quint specification, as Quint models deterministic state transitions, not probabilistic security reductions.
*   `B10_tally_correctness_shadow`: This is a `cross-layer` invariant. Similar to B8, Quint cannot directly verify the full `Tally_spec` execution or its `dstack_kms_derived`/`enclave_input_fidelity` components. We encode its *classical-Prop shadow* by asserting that `next.tally_result.get() == Tally_spec_stub(ballots_at_end_at, candidates, "fake_privkey")`. `Tally_spec_stub` is an uninterpreted function that, for simulation purposes, returns a minimal well-formed `TallyResult`. The `privkey` is a placeholder for the existential `PrivKey` mentioned in the intent.
*   `B10_lean`: This is an `off-chain` invariant (a Lean-internal theorem). It is *omitted* from the Quint specification.

**Mandatory Invariants:**
*   `all_invariants`: A composite `val` combining all encoded S-series and classical B-series invariants.
*   `witness_resolution_reachable`: `not(is_Resolved(state))`.
*   `witness_ballot_submittable`: `not(is_Voting(state) and is_candidate(candidates.first()))`.
*   `witness_end_at_crossing`: `not(env_block_time >= end_at)`.
    These are formulated as negations, such that `quint run --invariant=<witness_name>` will produce a counterexample trace that reaches the desired state when the invariant is "violated."

#### 3. Non-Obvious Encoding Choices and Justifications

1.  **`Nat` as `int`**: Quint does not have a `Nat` type; `int` is used for all natural numbers, with the implicit assumption that values remain non-negative.
2.  **`Bytes` as `str`**: Quint lacks a `Bytes` type. `str` is used as a stand-in for arbitrary byte sequences (e.g., encrypted ballots, public/private keys, SHA-256 digests, domain separation tags). This is a common abstraction in Quint.
3.  **Order-Preserving Collections**: The intent heavily emphasizes "candidate-declaration order" for `Vec<Addr>` and for serializing `Map<Addr, Nat>` as `Vec<(Addr, Nat)>`.
    *   `candidates` and `eliminated_by_round`, `winners`, `non_voters`, `dropped_voters` are encoded as `Seq[Addr]`, preserving order.
    *   `per_round_counts` is `Seq[Map[Addr, int]]`. While `Map` in Quint is unordered, its operations (e.g., `keys()`, `values()`, `get()`) are well-defined. The intent's requirement for deterministic *serialization* of `Map` fields in "candidate-declaration order" (like `Vec<(Addr, Nat)>`) is a serialization detail not directly modeled by Quint's abstract `Map` type. The invariants checking consistency of counts and elimination sequences (`S8`, `S9`) operate on the logical content of these maps, not their byte-level ordering.
4.  **`env.block.time` as Ghost Variable**: `env.block.time` is modeled as a `var env_block_time: Timestamp` that explicitly updates via `action time_advances`. This allows precise control over time progression and derived state transitions. A `MAX_TIMESTAMP` constant is introduced to bound time for model checking.
5.  **`ballots@end_at` as Ghost Variable**: The `var ballots_at_end_at: Map[Addr, Bytes]` serves as a snapshot of `ballots` at the exact moment `env_block_time` transitions from `< end_at` to `>= end_at`, ensuring the "frozen ballot store" property for `B2` and `B10`.
6.  **`fires_at_transition` as Ghost Variable**: The `var fires_at_transition: Set[ContractMessage]` is reset to empty at the start of each step and then populated by any message(s) that fire within that step. This provides the `σ → σ′` context for `B6`'s message attribution. For simplicity in `step`, only one message is usually associated with an action, which covers the `∃!` requirement by making it implicitly the unique message.
7.  **`Tally_spec_stub`**: This pure definition acts as a stand-in for the idealized `Tally_spec` function. It's crucial for `B10_tally_correctness_shadow`. For `quint run` to succeed, this stub must produce outputs that satisfy `tally_is_well_formed`. A simplified, minimal "valid" tally is returned by the stub in the Quint code.
8.  **Absent-Key Semantics**: For `ballots: Map<Addr, Bytes>`, Quint's `Map.get(k)` returns `None()` if the key is absent. Comparisons like `next.ballots.get(k) != ballots.get(k)` (used in `B6`) implicitly handle `Option` values, correctly capturing transitions between `None` and `Some(v)`, or `Some(v)` and `Some(v')`.
9.  **`instantiate` Action in `step`**: `instantiate` is designed as a one-shot action. In `step`, it's guarded by `candidates.size() == 0` to ensure it only fires when the contract is in an uninitialized state, effectively simulating the contract creation.

This specification aims to provide an honest, idiomatic Quint encoding of the intent, explicitly noting the limitations of the modeling language for cross-layer and meta-security claims, as per the Colosseum methodology.