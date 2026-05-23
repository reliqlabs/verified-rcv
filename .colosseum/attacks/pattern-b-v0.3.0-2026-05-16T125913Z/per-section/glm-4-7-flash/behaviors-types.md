# glm-4-7-flash — slice behaviors-types

## Cross-section reads
- Section 6.2 (Quartz inheritance table) — to verify whether `DstackKeyManager` is actually a Quartz artifact (it's not; this informs the Tally_spec arity attack).
- Section 8.7 (B10 witness) — to confirm that `EnclaveImage` is not defined in the intent doc (it's used as a Lean-internal symbol).

## Attacks on this slice

### 1. Tally_spec arity over-specification [serious]
- **Category**: over-specification
- **Affected**: Section 2.5, line 153
- **What's wrong**: The type signature declares `Tally_spec : EncryptedBallots × Schedule × ResolverPolicy → TallyResult`, but the actual function signature in the prose (lines 297–300) and the B10 restatement (line 738) use `(ballots, candidates, enclave_privkey)`. The Schedule and ResolverPolicy types are not used in the function body; they are artifacts of an earlier draft that were never removed from the type signature. This mismatches the actual contract behavior and creates a type-level mismatch between the intent doc and downstream Quint/Lean encodings.
- **Cite**: > `Tally_spec : EncryptedBallots × Schedule × ResolverPolicy → TallyResult` (line 153)
- **Fix recommendation**: Change the type signature to `Tally_spec : EncryptedBallots × CandidateSet × PrivKey → TallyResult` to match the actual function signature used throughout the section. This eliminates the type-level mismatch and prevents downstream encodings from trying to model non-existent parameters.

### 2. Map iteration order under-specification [serious]
- **Category**: under-specification
- **Affected**: Section 2.5, lines 134, 159
- **What's wrong**: The spec states that `per_round_counts[i]` entries and `Vec<Addr>` fields are "deterministic-by-candidate-declaration-order" and that `borsh_bytes(tally_body)` is a deterministic function of the logical value. However, Borsh itself does not mandate Map iteration order; it serializes maps as key-value pairs in whatever order the implementation chooses. The spec describes the intended ordering as a property but does not require it as an invariant, leaving room for two compliant-looking enclave implementations to disagree on B8(c)’s hash.
- **Cite**: > `per_round_counts[i]` lists only surviving candidates for round `i`; addresses absent from `per_round_counts[i]` are interpreted as eliminated before round `i`. Ordering of entries within each map and within `Vec<Addr>` fields is **deterministic-by-candidate-declaration-order` (line 134); `borsh_bytes(tally_body)` is a deterministic function of the logical value (line 138)
- **Fix recommendation**: Add an explicit structural invariant in Section 3.1: `∀ i, ∀ c ∈ candidates, c appears in per_round_counts[i] iff c is in the surviving set for round i, and the Map iteration order for each per_round_counts[i] is exactly candidate-declaration-order`. This pins the ordering as a required invariant rather than a descriptive property.

### 3. IRV_spec semantics under-specification [serious]
- **Category**: under-specification
- **Affected**: Section 2.5, lines 161–176
- **What's wrong**: The spec names `IRV_spec` as a function but only provides a prose narrative of the algorithm. The narrative relies on external knowledge of Australian IRV rules (batch elimination, ties → multi-winner fallback) rather than specifying the algorithm formally. This creates a gap between the intent doc and downstream formal encodings: the Lean theorem `IRV_spec` correctness (Section 8.7, step 2) must be written against a formal specification, but the intent doc does not provide one.
- **Cite**: > `Stage 2 — IRV_spec(valid_ballots, candidates) → (winners, per_round_counts, eliminated_by_round, ballots_tallied)` (line 161) followed by prose description (lines 163–176)
- **Fix recommendation**: Either formalize the IRV algorithm in a separate specification document (e.g., a Lean theorem or Quint property) or add a reference to a canonical formal specification (e.g., Mathlib’s `IRV` module). The intent doc should not rely on external knowledge as the specification.

### 4. Borsh pin over-specification [serious]
- **Category**: over-specification
- **Affected**: Section 2.5, lines 136–140
- **What's wrong**: The spec claims that Borsh serialization makes `borsh_bytes(tally_body)` a deterministic function of the logical value, but Borsh does not mandate Map iteration order. The spec says "Vec<Addr> and Map<Addr, Nat> entries are emitted in candidate-declaration order" but this is a requirement on the implementation, not a guarantee from Borsh itself. The claim that Borsh guarantees determinism is therefore over-specification: two implementations that follow Borsh but differ in Map iteration order would produce different byte representations for the same logical value, violating B8(c).
- **Cite**: > `canonical_serialization(contract_addr ‖ tally_body)` denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`, where `‖` is byte concatenation. `tally_body` is the `TallyResult` struct above, serialized field-by-field in declaration order; `Vec<Addr>` and `Map<Addr, Nat>` entries are emitted in **candidate-declaration order** (matching the ordering discipline above), making `borsh_bytes(tally_body)` a deterministic function of the logical value. (lines 136–138)
- **Fix recommendation**: Remove the claim that Borsh guarantees determinism. Replace it with an explicit requirement that the implementation must emit Map entries in candidate-declaration order, and add the corresponding structural invariant in Section 3.1 (see Attack 2).

### 5. fires_at_transition under-specification for B6 [critical]
- **Category**: under-specification
- **Affected**: Section 2.5, lines 149, 344
- **What's wrong**: The transaction trace model defines `fires_at_transition(σ → σ′)` as a **multiset** of transactions (line 149). However, B6 requires a **single** transaction to witness each key change: `∀ k, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`. A multiset does not have a unique witness; if two SubmitBallot transactions fire in the same block, the existential quantifier could be satisfied by either, but the spec does not require that the witness transaction is the one that actually performed the write. This creates a gap between the model and the invariant: the model allows scenarios where the existential is satisfied by a transaction that did not cause the change.
- **Cite**: > `fires_at_transition(σ → σ′)` is parameterized by the multiset of transactions that fired in the block taking `σ` to `σ′` (line 149); B6: `∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences` (line 344)
- **Fix recommendation**: Change the model to use a **set** instead of a multiset, or add an additional constraint that the witness transaction is the one that actually performed the write. For example: `∀ k, next.ballots[k] ≠ ballots[k] → ∃! tx ∈ fires_at_transition(σ → σ′) such that tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`. This ensures that the witness is unique and corresponds to the actual write.

### 6. EnclaveImage free variable [serious]
- **Category**: under-specification
- **Affected**: Section 3.2, line 349
- **What's wrong**: The spec uses `EnclaveImage` as a symbol in the B10_lean invariant but never defines it. It is used as if it is a type or a function, but there is no definition of what `EnclaveImage` is in the intent doc. This creates a gap between the intent doc and the Lean spec: the Lean theorem `B10_lean` must reference a formal definition of `EnclaveImage`, but the intent doc does not provide one.
- **Cite**: > `B10_lean | off-chain | ∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)` (line 349)
- **Fix recommendation**: Define `EnclaveImage` as a type or function in Section 2.5, or add a reference to a formal definition in the Lean spec. For example: "EnclaveImage is the type of enclave binary images; the Lean spec models this as a function from raw_ballots, candidates, and privkey to Tally_result." This makes the symbol's meaning explicit in the intent doc.

## Slice-local summary
- Critical: 1 (fires_at_transition model)
- Serious: 5 (Tally_spec arity, Map iteration order, IRV_spec semantics, Borsh pin, EnclaveImage free variable)
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE