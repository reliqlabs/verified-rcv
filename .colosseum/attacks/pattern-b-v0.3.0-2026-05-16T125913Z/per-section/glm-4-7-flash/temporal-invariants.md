I'll read the temporal-invariants slice from the intent document and attack it.
# glm-4-7-flash — slice temporal-invariants

## Cross-section reads
- Section 2.5 (transaction trace model) — needed to ground B6's `fires_at_transition` quantification and verify whether B6 correctly binds the existential witness to the firing transition
- Section 6.1 (chain trust) — needed to verify whether `image_registration_honest` is formally defined as a precondition or remains an informal operational assumption
- Section 8.7 (Lean discharge structure) — referenced in appendix as the decomposition `B10 ← B10_lean ∧ image-identity-binding ∧ B8`; needed to check whether B10's cross-layer tag is consistent with the decomposition logic

## Attacks on this slice

### 1. B6 ∀-per-key formulation is underspecified relative to the trace model [serious]

- **Category**: under-specification
- **Affected**: B6 (lines 344-345)
- **What's wrong**: B6 quantifies over `∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′)`. The trace model in §2.5 defines `fires_at_transition(σ → σ′)` as "the multiset of transactions that fired in the block taking σ to σ′ — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order." However, B6 does not specify whether the existential quantifier is interpreted as "there exists a transaction in the firing set" or "for every transaction in the firing set, there exists a key change that it witnesses." The former is weaker and would be satisfied by a firing set containing a SubmitBallot for a different key than the one that changed. The latter matches the intended semantics but is not stated. This ambiguity allows two non-equivalent interpretations: (1) there exists at least one SubmitBallot in the firing set (satisfied if one candidate submits while another overwrites their ballot), and (2) each key change is witnessed by a distinct SubmitBallot in the firing set (the intended semantics). The worked example in §2.1 shows last-write-wins overwrites (A's ballot is overwritten by B's ballot in the same transition), which would satisfy interpretation (1) but violate interpretation (2). B6's "∀-per-key" framing suggests interpretation (2), but the text does not explicitly bind the existential witness to the per-key change.
- **Cite**: 
  > B6: `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`
  > 
  > §2.5 transaction trace model: "A state transition `σ → σ′` is parameterized by the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking `σ` to `σ′` — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order."
- **Fix recommendation**: Add an explicit quantifier scope: "for every key whose ballot value changes at the firing transition, there exists a distinct transaction in `fires_at_transition(σ → σ′)` that is a SubmitBallot with `msg.sender = k` and `encrypted_preferences = next.ballots[k]`." This disambiguates the existential witness binding and aligns with the "∀-per-key" framing.

### 2. B8 clause (c) uses an undefined hash function [critical]

- **Category**: under-specification
- **Affected**: B8 clause (c) (line 346)
- **What's wrong**: B8 clause (c) states `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`. The text references `canonical_serialization` as defined in §2.5, but does not specify which hash function is used for the outer SHA-256. The appendix notes that `canonical_serialization = Borsh`, but does not specify whether the outer hash is SHA-256, SHA-3-256, or another hash function. The appendix also notes that the 32-byte digest is embedded in the lower half of Quartz's 64-byte `UserData` slot, with the upper half reserved for a domain-separation tag. However, the appendix does not state which hash function produces the 32-byte digest. This ambiguity means two implementations could produce different hash outputs for the same input, violating B8 clause (c) and making the attestation binding non-deterministic. The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B8 clause (c): `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`
  > 
  > §2.5 canonical serialization: "every appearance of `canonical_serialization(x)` in this doc ... refers to **Borsh** serialization per the CosmWasm-native discipline."
- **Fix recommendation**: Add an explicit hash function specification: "attested user_data = SHA-256(Borsh(contract_addr ‖ tally_body))" or "attested user_data = SHA-256(Borsh(contract_addr ‖ tally_body)) where SHA-256 is the standard SHA-256 hash function (FIPS 180-4)." This removes ambiguity about which hash function is used.

### 3. B8 clause (d) references an undefined "registered verified-rcv enclave image" [serious]

- **Category**: under-specification
- **Affected**: B8 clause (d) (line 346)
- **What's wrong**: B8 clause (d) states `attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image`. The text does not define what constitutes a "registered verified-rcv enclave image" or how it is registered on-chain. Section 6.1 mentions "Enclave-image registration integrity" as a chain trust assumption, but does not provide a formal definition of the registration process or the identity representation (MRTD/RTMR). The appendix notes that B9 conditions on `image_registration_honest`, but does not define what `image_registration_honest` means formally. This ambiguity means two implementations could interpret "registered verified-rcv enclave image" differently (e.g., one could require a hash match, another could require a full MRTD/RTMR match), leading to non-equivalent interpretations of B8 clause (d). The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B8 clause (d): `attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image`
  > 
  > §6.1 Enclave-image registration integrity: "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."
- **Fix recommendation**: Add an explicit definition: "the registered verified-rcv enclave image is the MRTD/RTMR measurement registered at contract instantiation via the `instantiate` handler, which is immutable and can be queried via the chain's enclave-image registry endpoint." This provides a concrete definition of the registration process and the identity representation.

### 4. B9's `image_registration_honest` precondition is not formally defined [serious]

- **Category**: under-specification
- **Affected**: B9 (lines 347-348)
- **What's wrong**: B9 conditions on `image_registration_honest` as a precondition: `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))`. However, the text does not define what `image_registration_honest` means formally. Section 6.1 mentions "Enclave-image registration integrity" as a chain trust assumption, but does not provide a formal definition of `image_registration_honest`. The appendix notes that B9 conditions on `image_registration_honest` as an "operational precondition," but this is not a formal definition. This means the precondition is not verifiable against the spec text alone; it requires external documentation or implementation details. This violates the Colosseum methodology's requirement that invariants be self-contained and verifiable against the spec.
- **Cite**: 
  > B9: `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))`
  > 
  > §6.1 Enclave-image registration integrity: "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."
- **Fix recommendation**: Add an explicit definition: "`image_registration_honest` is the proposition that the MRTD/RTMR measurement registered at contract instantiation has not been substituted post-registration, and the registered vkey matches the expected verified-rcv vkey." This provides a formal definition of the precondition that can be verified against the spec text.

### 5. B10's cross-layer tag is inconsistent with its existential quantification [serious]

- **Category**: temporal-state mismatch
- **Affected**: B10 (line 348)
- **What's wrong**: B10 is tagged as "cross-layer" but contains an existential quantifier over an off-chain value: `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr)`. The tag "cross-layer" suggests that the invariant is evaluated across layers (chain + enclave), but the existential quantifier ranges over an off-chain value that is not part of the chain state. The temporal tag would be more appropriate for a property that quantifies over chain-side trajectories. The appendix notes that B10 is "cross-layer" because the witness path traverses the Lean discharge layer, not because the invariant is evaluated across layers. However, the tag is misleading because it suggests a different interpretation than the existential quantification. This could confuse downstream spec writers who rely on the tag to determine how to encode the invariant in Quint or Lean.
- **Cite**: 
  > B10: `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`
  > 
  > B10 tag: "cross-layer"
- **Fix recommendation**: Change the tag to "temporal" or "off-chain existential" to reflect the existential quantification over an off-chain value. Alternatively, rephrase B10 to avoid the existential quantifier: "if `next.tally_result.is_some()` and `tally_result.is_none()`, then `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey))` where `enclave_privkey` is the key derived from `dstack_kms_derived`." This would make the invariant chain-side evaluable and justify the "cross-layer" tag.

### 6. B10_lean's EnclaveImage symbol is undefined [critical]

- **Category**: under-specification
- **Affected**: B10_lean (line 349)
- **What's wrong**: B10_lean is defined as `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`. However, the text does not define what `EnclaveImage` is. The appendix notes that `EnclaveImage` is the Lean-internal image-IO obligation, but does not provide a type signature or definition. This means B10_lean is not self-contained; it requires external Lean documentation or implementation details. This violates the Colosseum methodology's requirement that invariants be self-contained and verifiable against the spec text alone. The decomposition `B10 ← B10_lean ∧ image-identity-binding ∧ B8` (mentioned in the appendix) suggests that `EnclaveImage` is a Lean symbol that is defined in the Lean spec, but the intent doc does not provide a definition that can be verified independently.
- **Cite**: 
  > B10_lean: `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`
- **Fix recommendation**: Add an explicit definition: "`EnclaveImage` is the function that represents the externally observable input-output relation of the attested enclave image, where the enclave image is the verified-rcv image registered on-chain. The function takes raw ballots, candidate set, and private key as inputs and returns the tally result produced by the enclave." This provides a self-contained definition that can be verified against the spec text.

### 7. B8's attestation-binds-tally is a temporal property encoded as a state-only invariant [serious]

- **Category**: temporal-state mismatch
- **Affected**: B8 (lines 346-347)
- **What's wrong**: B8 is stated as a temporal invariant: `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result tx that fired carried a DstackAttestation whose: (a)... (b)... (c)... (d)...)`. However, the text describes a temporal condition about the publish_result transaction that fired in the transition, but the invariant is encoded as a state-only predicate (it does not quantify over the firing transaction). The temporal tag suggests that the invariant should quantify over the firing transaction, but the text does not explicitly do so. This means the invariant is not verifiable as a temporal property; it requires additional context about which transaction fired. The appendix notes that B8 is a "composition theorem" that depends on Quartz's attestation spec, but this does not resolve the temporal-state mismatch. The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B8: `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result tx that fired carried a DstackAttestation whose: (a) TDX quote validates, (b) zkdcap proof verifies against the chain's registered verified-rcv vkey, (c) attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body)), (d) attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image)`
- **Fix recommendation**: Add an explicit quantifier over the firing transaction: "always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = PublishResult ∧ tx.payload.attestation satisfies (a)... (b)... (c)... (d)...)". This makes the invariant a true temporal property that quantifies over the firing transaction.

### 8. B10's existential quantifier over `privkey` is not bound to the attestation [serious]

- **Category**: under-specification
- **Affected**: B10 (line 348)
- **What's wrong**: B10 states `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))`. However, the text does not specify how the existential quantifier is related to the attestation in B8. The appendix notes that B10 factors as `B10 ← B10_lean ∧ image-identity-binding ∧ B8`, but this decomposition is not stated in the slice. This means the existential quantifier is not clearly bound to the attestation; it could refer to any private key derived from `dstack_kms_derived`, not necessarily the one used by the enclave. This ambiguity could allow a scenario where the enclave uses a different private key than the one quantified in B10, violating B10 without violating B8. The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B10: `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`
- **Fix recommendation**: Add an explicit binding to the attestation: "always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)) ∧ the attestation in the firing publish_result transaction binds to privkey)". This makes the existential quantifier clearly related to the attestation and resolves the ambiguity.

## Slice-local summary
- Critical: 2 (B8 clause (c) hash function undefined, B10_lean EnclaveImage undefined)
- Serious: 5 (B6 ∀-per-key underspecified, B8 clause (d) undefined, B9 precondition undefined, B10 cross-layer tag mismatch, B10 existential quantifier not bound to attestation)
- Cosmetic: 1 (B8 temporal-state mismatch)

## VERDICT (slice-local): BREAKS-AT-SLICE