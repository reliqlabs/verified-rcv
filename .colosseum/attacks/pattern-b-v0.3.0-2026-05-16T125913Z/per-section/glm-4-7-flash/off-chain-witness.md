I'll read the target slice from the intent document and then produce an aggressive attack report.
# glm-4-7-flash — slice off-chain-witness

## Cross-section reads
- Section 6.3 (dstack/TDX hardware trust) — provides the dstack-KMS-derived privkey existential witness that Step 6 silently imports
- Section 3.2 B10 (cross-layer invariant) — the target statement that Step 6 claims to discharge
- Section 3.2 B10_lean (off-chain invariant) — the Lean-internal image-IO obligation that Step 3 claims to compose
- Section 6.2 (Quartz inheritance) — documents the upstream-blocked Stage-1 soundness gap noted in Step 1

## Attacks on this slice

### 1. Stage-1 lemma is upstream-blocked, not proven [serious]
- **Category**: coverage gap / impossibility-hypothesis vacuity
- **Affected**: Step 1 (decrypt_and_validate correctness)
- **What's wrong**: Step 1 is presented as a "Stage-1 lemma" that "for every (raw_ballots, candidates, privkey), the implementation of Stage 1 returns the partition (valid_ballots, dropped_voters, non_voters) matching the spec." However, the caveat immediately below explicitly states: "Ecies.roundtrip proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts. The Stage-1 spec is therefore stated as 'deterministic decrypt-or-drop' (correctness of the partition is definitional once the decoder is fixed); the soundness claim 'a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate' requires an additional authenticated-decryption (AEAD) property. Quartz does not currently expose this property. Recorded as a v0.3 upstream ask on Quartz; verified-rcv's compose ledger flags the Stage-1 soundness obligation as upstream-blocked."

This is a **hand-waved claim presented as a proven lemma**. The decomposition asserts Step 1 is a theorem, but the text admits it's not actually proven and is upstream-blocked. This makes Step 6's claim that B10 follows from "B10_lean (steps 1+2+3)" factually incorrect — Step 1 is not a theorem.

- **Cite**: > "Stage-1 lemma — decrypt_and_validate correctness: for every (raw_ballots, candidates, privkey), the implementation of Stage 1 returns the partition (valid_ballots, dropped_voters, non_voters) matching the spec in Section 2.5. Depends on Quartz's Specs.Quartz.Crypto.Ecies.roundtrip for the decryption-correctness half (Section 6.2 inheritance row 3 — a theorem, not an axiom). **Caveat (Round 3a re-adversarial theme 9 / claude S7)**: Ecies.roundtrip proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts. The Stage-1 spec is therefore stated as 'deterministic decrypt-or-drop' (correctness of the partition is definitional once the decoder is fixed); the soundness claim 'a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate' requires an additional authenticated-decryption (AEAD) property. Quartz does not currently expose this property. Recorded as a v0.3 upstream ask on Quartz; verified-rcv's compose ledger flags the Stage-1 soundness obligation as upstream-blocked."

- **Fix recommendation**: Either (a) remove Step 1 from the B10 composition and explicitly mark it as an upstream-blocked gap that must be resolved before B10 can be discharged, or (b) restate Step 1 as a "spec-level obligation" rather than a "lemma" and adjust Step 6 to reflect that B10_lean depends on an unresolved upstream dependency.

### 2. Image identity binding is not a Lean theorem [serious]
- **Category**: triviality / over-specification
- **Affected**: Step 4 (image identity binding)
- **What's wrong**: Step 4 is presented as part of the B10 composition: "Image identity binding: the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that B10_lean is over. This is an extraction-soundness + build-reproducibility obligation that lives at the build pipeline + deployment ledger layer, not the Lean layer. It cannot be a Lean theorem because the Lean spec models the binary; the binding claim is about the modelling discipline. The verified-rcv compose ledger records this as a tracked obligation with a discrete artifact (reproducible build hash + registered MRTD/RTMR equality)."

This is **not a Lean theorem** — it's a build/ledger obligation. Yet Step 6 claims B10 follows from "B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5)". By including Step 4 as a "step" in the B10 composition, the spec misleads about what is actually proven. The decomposition is dishonest about the layering — it presents a build/ledger obligation as if it were a mathematical theorem that can be composed with Lean theorems.

- **Cite**: > "Image identity binding: the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that B10_lean is over. This is an extraction-soundness + build-reproducibility obligation that lives at the build pipeline + deployment ledger layer, not the Lean layer. It cannot be a Lean theorem because the Lean spec models the binary; the binding claim is about the modelling discipline. The verified-rcv compose ledger records this as a tracked obligation with a discrete artifact (reproducible build hash + registered MRTD/RTMR equality)."

- **Fix recommendation**: Remove Step 4 from the numbered B10 composition list. It should be documented as a separate "build/ledger obligation" that must be satisfied independently, not as a step in the B10 discharge. Update Step 6 to read: "from B10_lean (steps 1+2+3) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey." Then add a separate note: "Image-identity-binding (Step 4) is a build/ledger obligation tracked in the compose ledger; it must be satisfied independently of the Lean proof of B10_lean."

### 3. EnclaveImage symbol typing mismatch [critical]
- **Category**: preconditional over-strength / type error
- **Affected**: Step 3 (Lean-internal composition — B10_lean)
- **What's wrong**: The spec defines EnclaveImage in the CONTEXT APPENDIX as: "EnclaveImage : Bytes → Bytes (image_extract)". However, Step 3 states: "by composition of the Stage-1 lemma and Stage-2 theorem, the enclave image's externally observable input-output relation satisfies EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)."

This is a **fundamental type error**. EnclaveImage is defined as taking a single Bytes argument (the image binary), but Step 3 calls it with three arguments: raw_ballots, candidates, and privkey. The right-hand side Tally_spec also takes three arguments. The composition is ill-typed — you cannot compose a function of type Bytes → Bytes with a function of type (raw_ballots, candidates, privkey) → Tally_spec.

This is not just a notational issue — it's a **preconditional over-strength**. The spec asserts that EnclaveImage has an input-output relation that matches Tally_spec, but EnclaveImage's actual type signature doesn't allow it to take the necessary inputs. The decomposition is factually incorrect.

- **Cite**: > "EnclaveImage : Bytes → Bytes (image_extract)" (from CONTEXT APPENDIX) and > "by composition of the Stage-1 lemma and Stage-2 theorem, the enclave image's externally observable input-output relation satisfies EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)."

- **Fix recommendation**: Either (a) redefine EnclaveImage to take the correct type signature (e.g., EnclaveImage : (raw_ballots, candidates, privkey) → Tally_spec) and update the CONTEXT APPENDIX definition, or (b) restate Step 3 to use a different function name (e.g., EnclaveImageIO : (raw_ballots, candidates, privkey) → Tally_spec) and clarify that EnclaveImage is the binary extraction function, not the IO relation. The latter is more honest about the layering.

### 4. Step 6 does not actually conclude B10 [serious]
- **Category**: impossibility-hypothesis vacuity
- **Affected**: Step 6 (Composition — B10)
- **What's wrong**: Step 6 claims: "from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey. This is the B10 statement from Section 3.2."

This claim is **vacuously true** because the premises are not actually satisfied:
1. Step 1 is not a theorem (it's upstream-blocked)
2. Step 4 is not a Lean theorem (it's a build/ledger obligation)
3. Step 5 is just a reference to B8, which is already covered elsewhere
4. The existential quantifier instantiation "with the dstack-KMS-derived privkey (Section 6.3)" is not justified — where does this privkey come from? The spec doesn't show how the existential is instantiated in the Lean proof.

The decomposition is dishonest about what's actually proven. Step 6 presents a composition that appears to discharge B10, but the components are either unproven or not the right type of thing to compose.

- **Cite**: > "from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey. This is the B10 statement from Section 3.2."

- **Fix recommendation**: Either (a) remove Step 6 entirely and replace it with a honest statement that B10 is **not yet discharged** — the composition is blocked by the upstream-blocked Stage-1 lemma and the missing image-identity-binding proof, or (b) restructure the decomposition to explicitly separate what is proven (Steps 2+3) from what is not (Steps 1+4+5), and mark B10 as "pending upstream dependencies." The honest approach is (a) — the current Step 6 is a misleading claim that B10 follows from the composition when it doesn't.

### 5. Fourth trust link (dstack-KMS) not honestly admitted [serious]
- **Category**: under-specification
- **Affected**: Step 6 (Composition — B10)
- **What's wrong**: Step 6 includes the phrase "with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3)". This is a **fourth trust link** that is not honestly admitted in the decomposition. The decomposition presents B10 as a composition of three things: B10_lean, image-identity-binding, and B8. But actually, there's a fourth thing: the dstack-KMS-derived privkey that instantiates the existential quantifier in B10.

Section 6.3 states: "verified-rcv trusts: dstack KMS honesty — the KMS releases the per-election private key only after verifying the requesting enclave's attestation." This is an operational trust assumption that is not part of the honest decomposition. The spec doesn't show how the existential quantifier in B10 is instantiated — it just asserts it is, without explaining the mechanism.

This is **under-specification** — the existential quantifier in B10 is not discharged honestly. The decomposition hides the dstack-KMS trust link as an afterthought in Step 6, rather than making it an explicit component of the composition.

- **Cite**: > "from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey."

- **Fix recommendation**: Either (a) make the dstack-KMS-derived privkey an explicit step in the composition (e.g., Step 4a: "Privkey instantiation — the dstack-KMS-derived privkey is retrieved from the KMS after attestation, satisfying the existential quantifier in B10"), or (b) remove the existential quantifier from B10 and make the privkey a free variable in the Lean proof (i.e., B10_lean should be parameterized over the privkey, and the instantiation happens outside the Lean proof). The honest approach is (a) — the current Step 6 hides the dstack-KMS trust link as an afterthought.

## Slice-local summary
- Critical: 1 (EnclaveImage symbol typing mismatch)
- Serious: 4 (Stage-1 lemma upstream-blocked, image identity binding not a Lean theorem, Step 6 doesn't conclude B10, dstack-KMS trust link not honestly admitted)
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE

The slice is fundamentally broken. The EnclaveImage type mismatch makes Step 3 factually incorrect. The upstream-blocked Stage-1 lemma and non-Lean image-identity-binding make Step 6's claim that B10 follows from the composition vacuously true. The dstack-KMS trust link is hidden as an afterthought rather than being honestly admitted. The decomposition is dishonest about what is actually proven and what is not.