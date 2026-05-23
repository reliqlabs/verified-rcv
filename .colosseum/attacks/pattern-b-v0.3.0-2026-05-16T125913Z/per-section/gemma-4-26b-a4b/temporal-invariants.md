# gemma-4-26b-a4b — slice temporal-invariants

## Cross-section reads
- Section 2.5: Used to verify the `fires_at_transition` trace model for B6 and the `EnclaveImage` type signature.

## Attacks on this slice
### 1. Symbol Contradiction: `EnclaveImage` Type/Usage Mismatch [critical]
- **Category**: contradiction
- **Affected**: B10_lean (line 349) / Section 2.5 (line 116/117, 151)
- **What's wrong**: The symbol `EnclaveImage` is used in `B10_lean` as a function of three arguments `(raw_ballots, candidates, privkey)` that returns the result of `Tally_spec(...)` (a `TallyResult`). However, Section 2.5 explicitly defines `EnclaveImage : Bytes → Bytes` (described as an `image_extract` function). This is a critical type contradiction: the symbol cannot simultaneously be an image extraction/identifier (returning `Bytes`) and the functional execution of the enclave logic (returning `TallyResult`). Furthermore, applying a `Bytes → Bytes` function to three distinct arguments is syntactically invalid.
- **Cite**: > `B10_lean ... ∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(...)` (line 349); `EnclaveImage : Bytes → Bytes (image_extract)` (Context Appendix / Section 2.5)
- **Fix recommendation**: Rename the symbol in `B10_lean` to something representing the functional execution of the image, such as `ExecuteEnclaveImage`, and define its type signature in Section 2.5 to match the `Tally_spec` inputs and output (e.g., `ExecuteEnclaveImage : Map<Addr, Vec<u8>> × Vec<Addr> × PrivKey → TallyResult`).

### 2. Underspecified Precondition: `image_registration_honest` [serious]
- **Category**: under-specification
- **Affected**: B9 (line 347)
- **What's wrong**: The entire negligibility claim for the B8 violation (the core of the attestation security) is conditioned on `image_registration_honest`. While the text notes this refers to Section 6.1, the formal definition of this precondition is missing from the visible specification. Since the validity of the 4-summand decomposition (and whether it avoids trivialization by excluding governance/substitution attacks) depends entirely on the boundaries of this precondition, the invariant is currently ungrounded.
- **Cite**: > `always (image_registration_honest → Pr[B8 violated ...])` (line 347)
- **Fix recommendation**: Provide a formal, mathematical definition of `image_registration_honest` in Section 6.1 (or nearby) that explicitly defines the set of failure modes excluded from the negligibility bound (e.g., "The chain-side registered `zkdcap_vkey` and MRTD/RTMR for the `verified-rcv` contract instance have not been modified by any transaction since the election `instantiate` block").

### 3. Ambiguous Factorization: `image-identity-binding` [cosmetic]
- **Category**: ambiguity
- **Affected**: B10 (line 348)
- **What's wrong**: `B10` is factorized as `B10_lean ∧ image-identity-binding ∧ B8`. While the intent is clear, `image-identity-binding` is not explicitly defined as a standalone invariant in the list; it appears to be an informal label for Clause (d) of `B8`. This creates ambiguity as to whether the factor is an independent property or merely a pointer to `B8(d)`.
- **Cite**: > `B10 = B10_lean ∧ image-identity-binding ∧ B8` (line 348)
- **Fix recommendation**: Explicitly label Clause (d) of `B8` as `image-identity-binding` in the text, or define `image-identity-binding` as a formal sub-property of `B8`.

## Slice-local summary
- Critical: 1
- Serious: 1
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE