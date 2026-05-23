# Design Notes for magistral-medium-native Lean Spec

## Encoding Decisions

1. **Types**:
   - Used `String` for `Addr` as it's simple and matches the canonical example.
   - Used `List UInt8` for `Bytes` to represent binary data.
   - Used `List Addr` for `CandidateSet` and ballot rankings for simplicity.
   - Used `List` for `RawBallots` (as in the canonical example).

2. **Stage 1 and Stage 2**:
   - Used `opaque` for both `decrypt_and_validate` and `IRV_spec` as they model the extracted enclave behavior.
   - Used `def` for `Tally_spec` as it composes Stage 1 + Stage 2 transparently.

3. **TallyResult**:
   - Used a structure with the required fields as specified in the intent document.
   - Added `deriving Repr, Inhabited` to make the structure usable in Lean.

4. **Theorems**:
   - Implemented all required theorems with `sorry` bodies as specified.
   - Used the same names as in the canonical example for consistency.

5. **EnclaveImage**:
   - Used `axiom` for `EnclaveImage` as required, not `def EnclaveImage := Tally_spec`.

## Stdlib vs Mathlib

- Used only the Lean 4 stdlib, no Mathlib import, following the canonical example.

## Non-obvious Choices

- Used `List` for `RawBallots` instead of `Map` or other structures, as it's simple and matches the canonical example.
- Used `List Addr` for `CandidateSet` and ballot rankings for simplicity and to avoid dependencies.

## What Was Omitted

- Detailed comments on each function and theorem, as the focus is on the structure and correctness of the spec.
- Any implementation details for `decrypt_and_validate` and `IRV_spec`, as they are meant to be opaque.

The spec is designed to be a straightforward encoding of the intent document, focusing on the structure and correctness of the types and theorems rather than implementation details.