# gemma-4-26b-a4b — slice off-chain-witness

## Cross-section reads
- Section 2.5 / Block E1: Defines `EnclaveImage` as an image extraction function (`Bytes → Bytes`).
- Section 3.2 / B10: Defines `B10_lean` using `EnclaveImage(...) = ...`, treating the symbol as the enclave's logic/behavior.
- Section 6.3: Defines the `dstack-KMS` trust boundary for `privkey` provenance.

## Attacks on this slice

### 1. Symbol Overloading / Type Ambiguity [serious]
- **Category**: ambiguity
- **Affected**: §8.7 Step 1, Step 3; Section 3.2 (B10_lean); Context Appendix
- **What's wrong**: The symbol `EnclaveImage` is used for two non-equivalent mathematical objects, creating an ambiguity that prevents the discharge of `B10_lean`. In the Context Appendix/Section 2.5, `EnclaveImage` is defined as an *extraction function* ($Bytes \to Bytes$) used for identity binding (Step 4). However, in the `B10_lean` definition (Section 3.2) and the discharge Step 3, `EnclaveImage(...)` is used as the *logic/behavioral function* of the enclave ($Inputs \to Output$). Step 1 and Step 2 do not resolve this; they assume the symbol represents the logic, but the `image-identity-binding` (Step 4) requires the symbol to represent the extraction. A formal proof cannot discharge `B10_lean` if the symbol's type/domain is not pinned down to the behavioral model.
- **Cite**: > `EnclaveImage : Bytes → Bytes (image_extract)` (Appendix) vs `EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(...)` (Section 3.2/8.7 Step 3).
- **Fix recommendation**: Use distinct symbols for the extraction function (e.g., `extract_image`) and the behavioral model (e.g., `enclave_logic`).

### 2. Compositional Gap: Underspecified Key Provenance [critical]
- **Category**: composition failure
- **Affected**: §8.7 Step 6; Section 3.2 (B10)
- **What's wrong**: Step 6 claims to conclude `B10`, but the discharge components are insufficient to satisfy the full conjunction of the `B10` invariant. `B10` (Section 3.2) requires the existence of a `privkey` such that `dstack_kms_derived(privkey, contract_addr)` holds. The discharge components (Lean `B10_lean`, image-binding, and `B8`) only witness the *logic* of the image, the *correspondence* of the identity, and the *attestation* of the tally. None of these components witness the *provenance/derivation* of the specific `privkey` used in the execution. The `dstack-KMS` link is an operational trust assumption (Section 6.3) that the discharge attempts to absorb into Step 6, but because it is not part of the mathematical witness provided by `B10_lean`, `image-identity-binding`, or `B8`, the compositional conclusion `B10` is logically incomplete.
- **Cite**: > `B10 ← B10_lean ∧ image-identity-binding ∧ B8` (Section 8.7 Step 6) vs `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ ...` (Section 3.2/B10).
- **Fix recommendation**: Explicitly include the `dstack-KMS` provenance as a fourth, non-mathematical/operational discharge component in the `B10` composition, or admit that `B10` is an operational claim rather than a purely mathematical one.

### 3. Hand-waving of Stage-1 Soundness for Adversarial Inputs [serious]
- **Category**: under-specification
- **Affected**: §8.7 Step 1, Step 3; Section 2.5 (Stage 1)
- **What's wrong**: `B10_lean` is defined as a universal quantification over *all* `raw_ballots`, which includes untrusted/adversarial ciphertexts. However, Step 1 (the premise for `B10_lean`) explicitly admits that the underlying `Ecies.roundtrip` (from Quartz) does not bound adversarial/malformed ciphertexts because it lacks an authentication/AEAD property. Since `B10_lean` (Step 3) is the composition of Step 1 and Step 2, `B10_lean` inherits this soundness gap. The spec effectively hand-waves the correctness of the `decrypt_and_validate` partition for the very domain (`raw_ballots`) that the `B10_lean` theorem claims to cover.
- **Cite**: > `Ecies.roundtrip proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts.` (Section 8.7 Step 1/Caveat).
- **Fix recommendation**: Either tighten the `B10_lean` domain to "well-formed/authenticated ciphertexts" (which shifts the burden to the attestation layer) or include an AEAD/authentication requirement in the Stage-1 spec.

## Slice-local summary
- Critical: 1
- Serious: 2
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-SLICE