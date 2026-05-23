# gemma-4-26b-a4b — slice behaviors-types

## Cross-section reads
- Section 3.1/3.2: Used to verify the temporal/state distinction for B6, B8, and B10.
- Section 4.9: Used to ground the liveness implications of `InvalidTally`.

## Attacks on this slice
### 1. Underspecified Map Iteration for `dropped_voters` [serious]
- **Category**: under-specification
- **Affected**: Section 2.5, Stage 1 (line 159)
- **What's wrong**: The spec states `dropped_voters` are ordered by "iteration order over `raw_ballots` (deterministic per Map iteration discipline)". However, the spec does not explicitly define the `Map<Addr, Vec<u8>>` iteration discipline for the contract state. While Section 134 attempts to anchor ordering to "candidate-declaration order" for `TallyResult` fields, it does not explicitly mandate that the underlying `Map` implementation (or its serialization/iterator) follows this order for *all* iteration-dependent outputs. If the `Map` implementation (e.g., a standard CosmWasm/Rust `BTreeMap`) uses address-sorting instead of declaration-order, the `dropped_voters` list will diverge from the `non_voters` list (which *is* explicitly ordered by declaration order in line 158), creating an inconsistent audit trail for the election results.
- **Cite**: > `dropped_voters` ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline).
- **Fix recommendation**: Explicitly mandate that `raw_ballots` (and any `Map<Addr, ...>` used in the tally) are iterated according to the `candidates` declaration order.

### 2. Ambiguous `IRV_spec` Termination for Co-winners [serious]
- **Category**: ambiguity
- **Affected**: Section 2.5, Stage 2 (lines 165, 172)
- **What's wrong**: The spec provides two distinct "tie" termination rules that are not clearly reconciled for the intermediate case. Clause 1 (line 165) says: "if all remaining candidates have equal first-place counts... all of them are co-winners." Clause 172 (Edge cases) says: "All remaining candidates tied for lowest: batch elimination would empty `remaining`. In this case, declare all current `remaining` candidates as co-winners." These are non-equivalent for an election with, say, 3 candidates where A=1, B=1, C=1. Clause 1 (Majority/Terminal tie) implies the game ends because everyone is tied for *first*. Clause 172 (Batch elimination/Empty set) implies the game ends because everyone is tied for *lowest*. While the outcome (co-winners) might be the same, the *logic path* for the `per_round_counts` and `eliminated_by_round` arrays is ambiguous. A formal verifier (Lean/Quint) needs to know if the "tie" happened because of the `Majority termination` check (Clause 2) or the `Recursive case/Batch elimination` failure (Clause 172).
- **Cite**: > `1. Base case ... if all remaining candidates have equal first-place counts in the current round, all of them are co-winners.` AND `172. All remaining candidates tied for lowest: ... declare all current remaining candidates as co-winners`.
- **Fix recommendation**: Unify the tie-handling logic into a single hierarchical check: (1) Check for Majority, (2) If no majority, check if `count.values().unique().len() == 1` (Universal Tie); (3) Otherwise, perform Batch Elimination.

### 3. Underspecified `EnclaveImage` Symbol Type [cosmetic]
- **Category**: under-specification
- **Affected**: Section 2.5, Type signatures (line 110/Context Appendix)
- **What's wrong**: `EnclaveImage` is defined as `Bytes → Bytes`. In the context of Section 4.9 and Block E1, it is used to represent the software image being verified. However, the spec does not clarify if `EnclaveImage` refers to the *binary blob* of the enclave, the *measurement (MRTD/RTMR)*, or a *cryptographic hash* of the image. Given that Section 2.5 (line 139) discusses Borsh encoding for ballots, the lack of a specific type/structure for `EnclaveImage` (e.g., `struct EnclaveImage { measurement: Bytes, signature: Sig }`) makes the "image-identity" binding in B10/B8 slightly loose for implementation.
- **Cite**: > `EnclaveImage : Bytes → Bytes (image_extract)`
- **Fix recommendation**: Define `EnclaveImage` as a specific record type containing the measurement and necessary attestation metadata, rather than an opaque function on `Bytes`.

### 4. Potential Composition Failure in `fires_at_transition` for B6 [serious]
- **Category**: composition failure
- **Affected**: Section 2.5, Transaction trace model (lines 149-150) and B6 (line 344)
- **What's wrong**: The `fires_at_transition(σ → σ′)` model defines the transition as being parameterized by a *multiset* of transactions. B6 (line 344) attempts to attribute changes to `∃ tx ∈ fires_at_transition(...)`. However, the spec does not define how the `next.ballots[k]` value (the *result* of the transition) is mapped back to a specific `tx.payload` when the transition is composed of *multiple* transactions in the same block (e.g., two `SubmitBallot` calls for the same candidate, where the second overwrites the first). If `fires_at_transition` contains `{tx1, tx2}`, and `next.ballots[k]` is the value from `tx2`, the existential `∃ tx` is satisfied, but the spec doesn't explicitly constrain that the *value* in `next.ballots[k]` must be the specific `tx.encrypted_preferences` of the *witnessing* transaction for that specific state change, which is critical for the `∀-per-key` strength of B6.
- **Cite**: > `A state transition σ → σ′ is parameterized by the multiset of transactions fires_at_transition(σ → σ′)` AND `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), ... next.ballots[k] = tx.encrypted_preferences)`
- **Fix recommendation**: Strengthen the trace model to define the transition as a sequential application of the multiset, ensuring that for any state variable `v`, the value `next.v` is the result of the *last* transaction in the trace that mutated `v`.

## Slice-local summary
- Critical: 0
- Serious: 3
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-SLICE