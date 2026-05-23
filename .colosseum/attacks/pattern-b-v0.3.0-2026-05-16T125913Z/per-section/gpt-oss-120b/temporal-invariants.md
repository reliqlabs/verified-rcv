# gpt-oss-120b — slice temporal-invariants

## Cross-section reads
- None (all citations are within the target slice).

## Attacks on this slice
### 1. B6 — per‑key attribution under‑specified for concurrent SubmitBallot txs [serious]
- **Category**: under-specification  
- **Affected**: B6 clause (temporal)  
- **What's wrong**: The invariant quantifies over a *single* transition `fires_at_transition(σ → σ′)` but does not forbid multiple `SubmitBallot` transactions from different senders occurring in the same block. In that case the existential `∃ tx … tx.msg.sender = k` could be satisfied by a transaction whose `msg.sender` is *not* the key `k` that actually changed, because the clause does not require a *bijection* between changed keys and transactions. This permits a scenario where an attacker batches many ballot updates in one block, and the invariant is trivially satisfied even if some ballot changes are not attributable to the correct sender.  
- **Cite**: > 344: `| B6 | ballot writer is the ballot voter | **temporal** | always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`  
- **Fix recommendation**: Strengthen B6 to require a *one‑to‑one* mapping, e.g. `∀ k, next.ballots[k] ≠ ballots[k] → ∃! tx ∈ fires_at_transition …` or explicitly state that at most one `SubmitBallot` per address may appear in a single transition (and that the transaction trace model enforces this).

### 2. B8 clause (c) ambiguous byte‑encoding of `contract_addr ‖ tally_body` [under-specification]
- **Category**: ambiguity  
- **Affected**: B8 clause (c) (temporal)  
- **What's wrong**: The invariant requires the attested `user_data` to equal `SHA‑256(canonical_serialization(contract_addr ‖ tally_body))`, but the spec never defines the exact binary representation of `contract_addr` (e.g., bech32, raw 20‑byte, padded) nor the concatenation order when `canonical_serialization` already includes a deterministic layout for `tally_body`. Different implementations could serialize `contract_addr` differently while still satisfying the Borsh rules for `tally_body`, leading to divergent hashes and a false‑negative attestation failure.  
- **Cite**: > 346: `… (c) attested user_data = SHA‑256(canonical_serialization(contract_addr ‖ tally_body)), …`  
- **Fix recommendation**: Add an explicit definition, e.g. “`contract_addr` is serialized as its 32‑byte raw address in big‑endian order before concatenation”, and state that the concatenation is performed *outside* the Borsh serializer.

### 3. B8 clause (d) undefined “registered verified‑rcv enclave image” [under-specification]
- **Category**: under-specification  
- **Affected**: B8 clause (d) (temporal)  
- **What's wrong**: The invariant references “the registered verified‑rcv enclave image” without specifying how that registration is performed, what identifier is stored on‑chain, or how the on‑chain value is compared to the attested MRTD/RTMR. Consequently an implementation could register a placeholder hash, and the invariant would be vacuously true even if the enclave image is unrelated.  
- **Cite**: > 346: `… (d) attested enclave identity (MRTD/RTMR) matches the registered verified‑rcv enclave image)`  
- **Fix recommendation**: Define a concrete on‑chain storage slot `registered_image_hash : Bytes` set at instantiation, and require `attested_enclave_hash = registered_image_hash`. Also describe the governance process that updates this slot.

### 4. B9’s “image_registration_honest” precondition and collision‑resistance bound [ambiguity / over‑specification]
- **Category**: ambiguity, over-specification  
- **Affected**: B9 clause (temporal)  
- **What's wrong**: The predicate `image_registration_honest` is never defined; readers cannot tell whether it means “the on‑chain hash matches the intended image” or “no governance change has occurred since registration”. Moreover, the bound includes `Adv_commitTally_CR(n)`, a negligible advantage for SHA‑256 collisions, but SHA‑256 is not proven collision‑resistant for arbitrary `n`; the bound may be unrealistically strong, making the invariant impossible to satisfy in practice.  
- **Cite**: > 347: `| B9 | B8 negligibility‑budget decomposition | **temporal** | always (image_registration_honest → Pr[B8 violated …] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))`  
- **Fix recommendation**: (a) Provide a precise definition of `image_registration_honest`; (b) Replace the collision‑resistance term with a concrete assumption (e.g., “SHA‑256 behaves as a random oracle”) or remove it if the proof technique does not rely on negligible collision probability.

### 5. B10_lean arity mismatch with `EnclaveImage` type signature [contradiction]
- **Category**: contradiction  
- **Affected**: B10_lean (off‑chain)  
- **What's wrong**: The type signature in §2.5 defines `EnclaveImage : Bytes → Bytes` (a unary function). B10_lean, however, treats `EnclaveImage` as a ternary relation `EnclaveImage(raw_ballots, candidates, privkey)`. This type mismatch makes the lemma ill‑typed and unprovable in Lean, breaking the intended decomposition of B10.  
- **Cite**: > 149‑150 (type signatures): `EnclaveImage : Bytes → Bytes`  
> 349: `| B10_lean | Lean‑internal image‑IO obligation | **off‑chain** | ∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`  
- **Fix recommendation**: Either (a) redefine `EnclaveImage` as a three‑argument function `EnclaveImage : Bytes → Bytes → PrivKey → Bytes` (and update all uses), or (b) keep the unary signature and express the obligation as `EnclaveImage(raw_input) = serialize(Tally_spec(...))` where `raw_input` encodes the three arguments.

### 6. B10 cross‑layer factorisation lacks definition of “image‑identity‑binding” [under‑specification]
- **Category**: under-specification  
- **Affected**: B10 (cross‑layer)  
- **What's wrong**: B10 is expressed as `B10 ← B10_lean ∧ image‑identity‑binding ∧ B8`, yet the spec never defines the conjunct `image‑identity‑binding`. Without a formal statement, the conjunction is incomplete; an implementation could satisfy B10_lean and B8 while ignoring any binding between the on‑chain image hash and the Lean‑proved image, rendering the invariant vacuous.  
- **Cite**: > 348: `| B10 | tally‑correctness | **cross‑layer** | … ∃ privkey … ∧ next.tally_result = Some(Tally_spec(...)))` (text mentions decomposition)  
> 349: `| B10_lean …`  
> 351‑358 (cross‑project note) only mentions dependency on Quartz attestation bundles, not on a concrete binding predicate.  
- **Fix recommendation**: Introduce a predicate `image_identity_binding : (registered_image_hash = lean_proved_image_hash)` and include it explicitly in B10’s statement, with a clear on‑chain storage location for `lean_proved_image_hash`.

## Slice‑local summary
- Critical: 0  
- Serious: 4  
- Cosmetic: 0  

## VERDICT (slice‑local): BREAKS‑AT‑SLICE | SURVIVES‑SLICE | INDETERMINATE
BREAKS‑AT‑SLICE (multiple under‑specifications, ambiguities, and a type contradiction render the temporal‑invariant suite unsound).