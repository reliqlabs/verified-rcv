I'll read the failure modes section from the intent document and then attack it ruthlessly.
# glm-4-7-flash — slice failure-modes

## Cross-section reads
- Section 2.4 (Explicit failures) — defines `InvalidTally` and `AttestationFailed` error variants that ground the detection clauses in 4.3a/4.3b/4.9
- Section 3.2 B1 (tally_result monotone-once-set) — the temporal invariant that makes 4.6's idempotency claim sound
- Section 3.2 B9 (B8 negligibility-budget decomposition) — defines `image_registration_honest` precondition that 4.3b's detection implicitly relies on
- Section 5 (Non-Goals) — explicitly excludes admin recovery, which underwrites the "none for the affected election" recovery clauses across all failure modes

## Attacks on this slice

### 1. Candidate collusion is a purpose failure, not "not a failure" [serious]
- **Category**: coverage gap / under-specification
- **Affected**: 4.8 Candidate collusion / coordinated voting — *not a failure*
- **What's wrong**: The spec explicitly classifies collusion as "not a failure" and claims it's orthogonal to the cryptographic/consensus guarantees. However, the system's purpose is to produce a fair election outcome. If candidates coordinate to vote in a way that manipulates the outcome, the system fails its primary purpose even though no technical invariant is violated. The spec's "coordination resistance is orthogonal" claim is a judgment call that should be either (a) explicitly stated as a design tradeoff with a mitigation strategy, or (b) reclassified as a failure mode with appropriate handling.
- **Cite**: > "### 4.8 Candidate collusion / coordinated voting — *not a failure* ... Coordination resistance is orthogonal to the cryptographic / consensus guarantees this system provides. Explicitly listed in Section 5 (Non-Goals)."
- **Fix recommendation**: Either (a) add a "Candidate collusion" failure mode with a mitigation strategy (e.g., "Coordination resistance is achieved through public per-round counts; adversarial coordination is detectable post-tally but not preventable"), or (b) clarify in Section 5 that the system intentionally does not provide collusion resistance and this is a documented tradeoff. The current "not a failure" framing is misleading because it suggests the system's purpose is unaffected.

### 2. 4.6 Enclave network partition recovery is unsound under 4.4 [serious]
- **Category**: temporal-state mismatch
- **Affected**: 4.6 Enclave network partition — *liveness, recoverable*
- **What's wrong**: The recovery clause claims "Enclave re-publishes once reconnected. `publish_result` is idempotent by construction (B1 plus Block 6's `AlreadyResolved` rejection), so retries are safe." However, B1 states "once an election resolves, it stays resolved with the same value forever." If the enclave has already published a wrong tally (Section 4.4, enclave software bug), retries will re-submit the same wrong tally and be rejected again. The idempotency guarantee only holds for correct tallies. The recovery clause should distinguish between (a) enclave never published (retries safe) and (b) enclave already published wrong tally (retries unsafe).
- **Cite**: > "### 4.6 Enclave network partition — *liveness, recoverable* ... - **Recovery:** Enclave re-publishes once reconnected. `publish_result` is idempotent by construction (B1 plus Block 6's `AlreadyResolved` rejection), so retries are safe."
- **Fix recommendation**: Split the recovery clause into two cases: (1) If `tally_result.is_none()` at reconnect, retries are safe and idempotent. (2) If `tally_result.is_some()` but the value is wrong (detected via off-chain audit), the election is terminal and retries are unsafe. Add a detection clause: "On reconnect, enclave queries chain state; if `tally_result.is_some()` and off-chain audit confirms correctness, abort retries; otherwise proceed with safe idempotent retry."

### 3. 4.9 Deterministic-malformed-tally deadlock assumes deterministic enclave [serious]
- **Category**: edge case
- **Affected**: 4.9 Deterministic-malformed-tally deadlock — *liveness*
- **What's wrong**: The detection clause states "On-chain — repeated `InvalidTally` rejection events with identical `reason` field." This assumes the enclave produces the *same* malformed tally on every re-run. The text justifies this with "The enclave is deterministic over the ballot set; the ballot set is frozen at `end_at` (B2). Therefore re-running the enclave produces the *same* malformed tally." However, the spec does not explicitly state that the enclave is deterministic. If the enclave has non-deterministic behavior (e.g., due to timing-dependent code paths, RNG usage, or external dependencies), the malformed tally could vary between runs, making detection via identical `reason` fields unreliable.
- **Cite**: > "### 4.9 Deterministic-malformed-tally deadlock — *liveness* ... - **Detection:** On-chain — repeated `InvalidTally` rejection events with identical `reason` field. ... - **Effect:** Election deadlocked in Tallying indefinitely. Distinct from 4.6 (network partition — connectivity issue) and 8.3 (KMS unreachable — credential issue): here the chain refuses the enclave's output rather than the enclave being unable to publish."
- **Fix recommendation**: Either (a) add a precondition clause: "Assumes enclave is deterministic over the frozen ballot set (no RNG, no timing-dependent code paths)", or (b) add a failure mode for "Non-deterministic enclave behavior" that produces varying malformed tallies, making deadlock detection more complex. The current formulation silently assumes determinism without stating it as a precondition.

### 4. 4.3b Attestation vkey substitution detection requires adversary ignorance [serious]
- **Category**: edge case
- **Affected**: 4.3b Attestation vkey maliciously substituted — *integrity*
- **What's wrong**: The detection clause states "Verifiable on-chain by anyone who knows the *correct* verified-rcv vkey hash; comparison surfaces the substitution." This assumes the adversary does not know the correct vkey hash. However, if the adversary gains knowledge of the correct vkey hash (e.g., through governance compromise, insider access, or post-registration leakage), there is no on-chain detection mechanism. The detection clause should acknowledge this limitation: detection is only possible for observers who know the correct vkey hash, which may not be available to all stakeholders.
- **Cite**: > "### 4.3b Attestation vkey maliciously substituted — *integrity* ... - **Detection:** Verifiable on-chain by anyone who knows the *correct* verified-rcv vkey hash; comparison surfaces the substitution."
- **Fix recommendation**: Add a caveat: "Detection is only possible for observers who possess the correct verified-rcv vkey hash. If the adversary obtains this hash (e.g., through governance compromise), no on-chain detection mechanism exists. This is an operational risk, not a cryptographic one."

### 5. 4.4 Enclave software bug detection is circular [serious]
- **Category**: over-specification / coverage gap
- **Affected**: 4.4 Enclave software bug — *integrity* — **in scope, central methodology target**
- **What's wrong**: The detection clause states "Pre-deployment formal verification of the enclave's tally logic against the Lean and Quint specs derived from this intent document. Post-deployment, no in-system detection mechanism exists." This is circular: the spec claims the enclave's correctness is verified against the Lean and Quint specs, but those specs are derived from the intent document. If the intent document is incomplete or incorrect, the formal verification will be incomplete or incorrect. The spec does not address the possibility that the formal verification itself is flawed. This makes the "central methodology target" claim fragile.
- **Cite**: > "### 4.4 Enclave software bug — *integrity* — **in scope, central methodology target** ... - **Detection:** Pre-deployment formal verification of the enclave's tally logic against the Lean and Quint specs derived from this intent document. Post-deployment, no in-system detection mechanism exists."
- **Fix recommendation**: Add a mitigation layer: "Formal verification is performed against the Lean and Quint specs, but verification tooling and proofs are also subject to audit. Post-deployment, consider periodic off-chain audits of the attested enclave image against the verified-rcv specification. The absence of in-system detection is a documented tradeoff."

### 6. 4.1 and 4.2 mitigation hand-waves verified-rcv-specific risks [cosmetic]
- **Category**: over-specification
- **Affected**: 4.1 Enclave hardware compromise, 4.2 dstack KMS key compromise
- **What's wrong**: Both failure modes claim mitigation is "Inherited from Quartz / dstack KMS spec. Out of scope at the verified-rcv layer." However, these are verified-rcv-specific failure modes: 4.1 involves TDX vulnerabilities that are specific to the enclave hardware, and 4.2 involves dstack KMS key derivation that is specific to the dstack deployment. The "inherited" claim is misleading because these are not inherited from Quartz (which does not have TDX or dstack-specific failure modes). The mitigation should be more precise: either acknowledge these as verified-rcv-specific risks with verified-rcv-specific mitigations, or clarify the inheritance relationship more explicitly.
- **Cite**: > "### 4.1 Enclave hardware compromise — *confidentiality + potentially integrity* ... - **Mitigation:** Inherited from Quartz `tdxVerifier` soundness assumption. Out of scope at the verified-rcv layer." > "### 4.2 dstack KMS key compromise — *confidentiality* ... - **Mitigation:** Inherited from Quartz / dstack KMS spec."
- **Fix recommendation**: Clarify the inheritance relationship: "4.1's mitigation relies on Quartz's `tdxVerifier` soundness, but the specific TDX vulnerability surface is verified-rcv-specific. 4.2's mitigation relies on dstack KMS spec, which is verified-rcv-specific. These are not inherited from Quartz alone."

### 7. 4.3a Attestation vkey unregistered detection is post-deployment-only [serious]
- **Category**: edge case
- **Affected**: 4.3a Attestation vkey unregistered — *liveness*
- **What's wrong**: The detection clause states "Verifiable on-chain — any caller can query the registered vkey and compare to the expected verified-rcv image. Detectable pre-deployment." This is correct for pre-deployment, but does not address post-deployment unregistration. If the vkey is registered at instantiation but unregistered between instantiation and `publish_result`, the detection mechanism is silent until someone attempts to publish. The spec should clarify whether post-deployment unregistration is possible and how it would be detected.
- **Cite**: > "### 4.3a Attestation vkey unregistered — *liveness* ... - **Detection:** Verifiable on-chain — any caller can query the registered vkey and compare to the expected verified-rcv image. Detectable pre-deployment."
- **Fix recommendation**: Add a post-deployment detection clause: "If unregistration occurs after instantiation, detection requires an off-chain audit mechanism (e.g., periodic monitoring of the ZK module's registered vkey state). On-chain detection is only possible when `publish_result` is attempted."

## Slice-local summary
- Critical: 0
- Serious: 5
- Cosmetic: 2

## VERDICT (slice-local): BREAKS-AT-SLICE