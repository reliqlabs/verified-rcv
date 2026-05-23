# kimi-k2-6 — slice temporal-invariants

## Cross-section reads
- §2.5 transaction trace model (lines 149-150) — to verify `fires_at_transition(σ → σ′)` definition and binding discipline for B6.
- §2.5 Block 6 / Block E1 (lines 253-310) — to verify B8(c) user_data construction and B10's `Tally_spec` reference.
- §6.1 trust boundary (lines 471-480) — to confirm `image_registration_honest` is operational prose, not a formal predicate.
- §6.2 Quartz inheritance table (lines 486-510) — to verify `Adv_circuit_eq` provenance and `groth16Verifier` bundle tagging.
- §8.7 B10 witness decomposition (lines 684-701) — to verify the factorization `B10 ← B10_lean ∧ image-identity-binding ∧ B8`.

## Attacks on this slice

### 1. B8 clause (c) asserts equality between 64-byte attested user_data and 32-byte SHA-256 digest [critical]
- **Category**: contradiction
- **Affected**: B8 clause (c), line 346
- **What's wrong**: B8(c) states `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`. The same line's implementation note states the SHA-256 digest is "embedded in the lower half of Quartz's 64-byte `UserData` slot; upper half reserved for domain-separation tag." A 64-byte value cannot equal a 32-byte value. The "implementation detail" parenthetical does not resolve the contradiction—it confirms it. Downstream encoders will either enforce the equality (and reject valid attestations) or enforce the 64-byte construction (and violate B8(c)).
- **Cite**: > `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))` … `32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot; upper half reserved for domain-separation tag (implementation detail).`
- **Fix recommendation**: Restate B8(c) as a sub-field equality: `the lower 32 bytes of attested user_data equal SHA-256(...)`, and make the domain-separation tag part of the canonical serialization or an explicit separate clause.

### 2. B9's `image_registration_honest` precondition is a free predicate with no formal definition [critical]
- **Category**: under-specification
- **Affected**: B9, line 347
- **What's wrong**: B9 wraps `image_registration_honest` in an `always` modality and treats it as an antecedent of a formal probability bound. The slice describes it only as an operational assumption: "the chain-side registered verified-rcv vkey + MRTD have not been substituted post-registration; Section 6.1 trust boundary." Cross-reading Section 6.1 confirms it is "an operational assumption on the deployment / governance surface … not a cryptographic property reducible to a security parameter." Because no state predicate, transition relation, or formal typing is supplied, B9 reduces to "if [undefined], then bound holds." It is not evaluable by downstream tools and provides no falsifiable constraint.
- **Cite**: > `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ …)` — "conditioned on the operational precondition `image_registration_honest` (the chain-side registered verified-rcv vkey + MRTD have not been substituted post-registration; Section 6.1 trust boundary)."
- **Fix recommendation**: Define `image_registration_honest(σ)` as a concrete state predicate (e.g., `chain_registered_vkey(σ) = expected_vkey ∧ chain_registered_mrtd(σ) = expected_mrtd`) and remove it from the `always` modality, restating B9 as a conditional meta-theorem under a static assumption.

### 3. B10 cross-layer factorization omits input-integrity and KMS-derivation conjuncts [critical]
- **Category**: composition failure
- **Affected**: B10 (line 348) and Section 8.7 factorization (line 701)
- **What's wrong**: The slice claims `B10 ← B10_lean ∧ image-identity-binding ∧ B8`. B10_lean proves the enclave image computes `Tally_spec` correctly for *any* inputs. Image-identity-binding proves the on-chain registered image equals the Lean-proven binary. B8 proves the attestation binds the published tally to the registered image. None of the three conjuncts witness that the enclave actually received `ballots@end_at` and `candidates` as inputs, nor that the `privkey` satisfying `dstack_kms_derived(privkey, contract_addr)` was used. A malicious host could feed manipulated ballots to a legitimate enclave image; the enclave would correctly compute `Tally_spec(fake_ballots, candidates, privkey)`, B8 would verify, and B10 would be violated despite all three conjuncts holding. The factorization is therefore insufficient for the claimed conclusion.
- **Cite**: > `B10 ← B10_lean ∧ image-identity-binding ∧ B8` … `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))` … `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr)`
- **Fix recommendation**: Add an input-integrity conjunct (e.g., `enclave_input = ballots@end_at ‖ candidates`) and a KMS-derivation conjunct to the factorization, or weaken B10 to quantify over the enclave's actual inputs.

### 4. B10_lean's `EnclaveImage` symbol is undefined and arity-mismatched with the type appendix [critical]
- **Category**: under-specification
- **Affected**: B10_lean, line 349
- **What's wrong**: B10_lean introduces `EnclaveImage(raw_ballots, candidates, privkey)` as a 3-ary function symbol representing the enclave's input-output relation. This symbol is not defined anywhere in the intent document prior to this line. The CONTEXT_APPENDIX types `EnclaveImage : Bytes → Bytes` (a 1-ary function on bytes). There is no bridging definition reconciling these arities or types. B10_lean is therefore a free-symbol assertion with no grounding in the document's type system.
- **Cite**: > `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`
- **Fix recommendation**: Define `EnclaveImage` in §2.5 with its full 3-ary type signature, or replace the symbol with an explicit lambda over the enclave's observable behavior.

### 5. B9 is tagged `temporal` but its body is a probabilistic meta-security statement, not a temporal property [serious]
- **Category**: temporal-state mismatch
- **Affected**: B9, line 347
- **What's wrong**: The slice preamble states that "temporal properties require explicit history quantification downstream" and warns that "the wrong tag silently mis-encodes intent." B9's body is `always (image_registration_honest → Pr[B8 violated … by a polynomial-time adversary] ≤ Σ Adv_i(n))`. The inner `Pr[…]` ranges over adversaries and random coins; it is a cryptographic security statement, not a property of system states or execution trajectories. Tagging it as `temporal` mis-encodes intent: downstream tools will treat B9 as an LTL-style execution invariant when it is actually a meta-theoretic negligibility bound.
- **Cite**: > `B9 | B8 negligibility-budget decomposition | **temporal** | always (image_registration_honest → Pr[B8 violated …] ≤ …)`
- **Fix recommendation**: Retag B9 as `meta-security` or `probabilistic`, or move it out of the behavioral-invariant table entirely into a separate security-budget section.

### 6. B6's ∀-per-key formula mixes LTL modalities with free transition parameters, weakening causal attribution [serious]
- **Category**: ambiguity
- **Affected**: B6, line 344
- **What's wrong**: B6 states `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), …)`. The `always` modality quantifies over states, but `fires_at_transition(σ → σ′)` references explicit transition parameters `σ` and `σ′` that are not bound by the modality. The formula is therefore not well-formed in standard LTL and creates an ambiguity in downstream encoding (Quint action labels vs Lean step relations). Moreover, the existential only requires correlation, not causation: if some unauthorized tx also writes to `ballots[k]` and the final value happens to equal a co-occurring `SubmitBallot` from `k`, B6 is satisfied even though the authorized tx did not cause the write.
- **Cite**: > `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`
- **Fix recommendation**: Bind the transition explicitly using an action modality (e.g., `[_]_{fires_at_transition}`) or restate B6 as a pure LTL formula over `step` relations. Strengthen the existential to assert that the identified tx is the *unique* writer to `k` in that transition.

### 7. B9's 4-summand decomposition conflates a correctness theorem with a security advantage [serious]
- **Category**: disjunction-vs-decomposition collapse
- **Affected**: B9, line 347; Section 6.2 inheritance table, line 489
- **What's wrong**: B9 decomposes its bound into four summands, including `Adv_circuit_eq(n)`. Section 6.2 states the discharge path for `groth16Verifier` includes a "reference DCAP circuit-equivalence theorem." Circuit equivalence is a binary correctness property (the circuit either correctly encodes the TDX quote validity relation or it does not); it is not a probabilistic security advantage that decreases with a security parameter `n`. By including it in a sum of negligible advantages, B9 collapses a correctness assumption into a security bound, misrepresenting the logical structure. The honest form would separate correctness assumptions (circuit equivalence, image-identity-binding) from cryptographic advantage bounds.
- **Cite**: > `Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)` … `reference DCAP circuit-equivalence theorem`
- **Fix recommendation**: Remove `Adv_circuit_eq` from the negligibility sum and state it as a standalone correctness hypothesis. Restate B9 as a conditional bound under the conjunction of correctness assumptions.

### 8. B8 clause (d) "matches the registered verified-rcv enclave image" is ambiguous [serious]
- **Category**: ambiguity
- **Affected**: B8 clause (d), line 346
- **What's wrong**: B8(d) requires that "attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image." The slice never defines what "matches" means (exact equality? prefix? hash? composite?). Nor does it define the type of the "registered verified-rcv enclave image" — is it an MRTD value, an RTMR value, a pair, a hash, or a build artifact? Section 6.1 calls it "enclave-image identity (MRTD / RTMR)" with a slash, suggesting disjunction or pairing, but B8(d) treats it as a single registered value. This ambiguity makes the clause unenforceable on-chain.
- **Cite**: > `(d) attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image`
- **Fix recommendation**: Define the registered value as a concrete on-chain storage slot (e.g., `chain_registered_mrtd: Bytes32`) and replace "matches" with exact equality: `attested_mrtd = chain_registered_mrtd`.

### 9. Cross-project dependency note promises future ledger recording but supplies no formal propagation mechanism [serious]
- **Category**: coverage gap
- **Affected**: Cross-project dependency note, lines 351-358
- **What's wrong**: The note states that B8 depends on Quartz axioms and that "Any drift in either of those bundles' cardinality at the Quartz level … should propagate as a downstream alert to verified-rcv's ledger." It then says "The verified-rcv compose ledger (forthcoming) will record this dependency explicitly." There is no formal mechanism, invariant, or procedural rule in the slice that ensures this propagation actually happens. The dependency is a hand-waved future promise, not a specified behavior.
- **Cite**: > `The verified-rcv compose ledger (forthcoming) will record this dependency explicitly.`
- **Fix recommendation**: Replace the forward-looking promise with a concrete specification of how bundle-cardinality drift is detected and propagated (e.g., a CI gate, a ledger schema, or a periodic diff check against Quartz's bundle registry).

### 10. Tag preamble does not define `cross-layer` or `off-chain`, silently expanding the tag taxonomy [cosmetic]
- **Category**: ambiguity
- **Affected**: B10 (line 348), B10_lean (line 349), and the tag preamble (line 335)
- **What's wrong**: The slice preamble explicitly limits the tag taxonomy to `state` and `temporal`, stating the distinction is "load-bearing per Colosseum methodology." B10 is tagged `cross-layer` and B10_lean is tagged `off-chain` — neither of which is defined in the preamble. This silently expands the taxonomy without updating the methodological contract, creating ambiguity about discharge obligations for these invariants.
- **Cite**: > `The **state**/**temporal** tag is load-bearing per Colosseum methodology: state invariants discharge at every state independently; temporal properties require explicit history quantification downstream.`
- **Fix recommendation**: Update the preamble to enumerate all valid tags (`state`, `temporal`, `cross-layer`, `off-chain`, `probabilistic`) and their discharge disciplines.

## Slice-local summary
- Critical: 4
- Serious: 5
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE

The slice contains four critical flaws: a type contradiction in B8(c) (64-byte vs 32-byte equality), an undefined precondition in B9 that makes the bound epistemically vacuous, a compositionally insufficient factorization for B10 that omits input-integrity and KMS-derivation witnesses, and a free function symbol in B10_lean with no grounding in the document's type system. These are compounded by five serious issues including a temporal-state mismatch on B9, ambiguous causal attribution in B6, a category error in B9's decomposition, ambiguous matching semantics in B8(d), and an unfulfilled coverage gap in the cross-project dependency note. The slice does not survive adversarial scrutiny.