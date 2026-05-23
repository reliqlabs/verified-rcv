# gemma-4-26b-a4b — slice trust-quartz

## Cross-section reads
- Section 3.2 (B9, B10) for the usage of `image_registration_honest` and the negligibility budget.
- Section 4.2 for the KMS failure mode related to `DstackKeyManager`.

## Attacks on this slice

### 1. Unverified lynchpin assumption [critical]
- **Category**: coverage gap
- **Affected**: Section 6.2, Quartz inheritance table (DstackKeyManager row)
- **What's wrong**: The entire security model of `verified-rcv` rests on the premise that the `DstackKeyManager` releases the per-election private key *only* to the matching attested enclave image. However, the spec explicitly admits that this claim is not an inherited Quartz theorem and instead characterizes it as an "unverified out-of-band assumption" (Section 6.2, line 495). Because this binding is the lynchpin for the `B10` (tally-correctness) obligation, the formal chain of trust is fundamentally broken; the spec provides a mathematical proof for the *logic* of the tally, but the *connection* between that logic and the physical enclave is based on an assumption the spec cannot verify or inherit.
- **Cite**: > `DstackKeyManager ... The claim "private key released only to matching attested enclave image" is an unverified out-of-band assumption on the dstack KMS implementation itself, not an inherited Quartz theorem.` (Section 6.2, line 495)
- **Fix recommendation**: The lynchpin assumption must be promoted from an "out-of-band assumption" to a formal requirement of the `DstackKeyManager` spec, which must then be included in the Quartz inheritance table as an inherited theorem to close the coverage gap.

### 2. Unanchored Quartz inheritance [serious]
- **Category**: composition failure
- **Affected**: Section 6.2, status block `[2026-05-15 — de-retracted]`
- **What's wrong**: `verified-rcv` inherits the "now-def-tied" summands of Quartz (Section 6.2, line 503) to resolve the retraction of the `_negl` lifts. However, the spec fails to anchor this inheritance to a specific Quartz version, commit hash, or release tag that contains the `def-tying` refactor. Since the status of the Quartz substrate is volatile (having undergone retraction and refactoring within the same week), `verified-rcv`'s correctness is non-deterministic; it depends on the underlying Quartz dependency having been updated to the post-retraction state. A consumer of this spec cannot verify the composed claim without knowing exactly which version of Quartz provides the "content-bearing" summands.
- **Cite**: > `verified-rcv inherits B8/B9 from a now-contented substrate.` (Section 6.2, line 503)
- **Fix recommendation**: Explicitly anchor the inheritance to a specific Quartz version or commit hash (e.g., `inherits Quartz v0.2.1+ [commit: <hash>]`) to ensure the composed trust claim is stable and verifiable.

### 3. Compositional fragmentation of binding hash [serious]
- **Category**: composition failure
- **Affected**: Section 6.2, Quartz inheritance table (`commitHashE` row)
- **What's wrong**: `verified-rcv` explicitly breaks the compositional chain of trust by opting out of Quartz's `commitHashE` binding, citing its "pigeonhole-impossible" sub-tag as making inheritance vacuous (Section 6.2, line 493). It instead defines its own replacement, `Adv_commitTally_CR`, for the `tally_body`. This creates a fragmented trust model where the attestation `UserData` is composed of two distinct, non-unified binding mechanisms: the Quartz `commitHashE` for the `UserDataCommit` and the `verified-rcv`-internal SHA-256 for the `tally_body`. This fragmentation prevents a single, monolithic hardness assumption from governing the entire attestation payload, potentially allowing an adversary to exploit the interface between these two disparate binding logics.
- **Cite**: > `It does not consume Quartz's commitHashE bundle ... the verified-rcv compose ledger will record this inheritance row's downstream consumer count as zero.` (Section 6.2, line 493)
- **Fix recommendation**: Unify the binding mechanism by ensuring the `tally_body` is included within the domain of the Quartz `commitHashE` bundle, or explicitly define the compositional theorem that proves the union of these two distinct binding assumptions provides the required security for the total `UserData`.

## Slice-local summary
- Critical: 1
- Serious: 2
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-SLICE