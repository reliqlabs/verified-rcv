# kimi-k2-6 — slice failure-modes

## Cross-section reads
- Block 1 (instantiate) — to verify DstackKeyManager failure at instantiation is not covered in §4.
- Block E1 (enclave tally computation) — to check retry semantics and determinism claims in §4.9.
- §2.4 (Explicit failures) — to confirm `InvalidInstantiation` KMS case is a per-handler error, not a system-level failure mode.

## Attacks on this slice

### 1. §4 intro and §4.6 directly contradict on recoverability [critical]
- **Category**: contradiction
- **Affected**: §4 preamble + §4.6
- **What's wrong**: The preamble states as a universal rule: "Per the no-admin-recovery design decision, **no failure here is auto-recoverable from within the election instance** — recovery always means deploying a fresh election with corrected configuration." Yet §4.6's Recovery clause says: "Enclave re-publishes once reconnected. `publish_result` is idempotent by construction ... so retries are safe." This is auto-recovery within the same election instance without any fresh deployment. A universal quantifier in the preamble is falsified by a concrete counter-example three subsections later.
- **Cite**: > "Per the no-admin-recovery design decision, **no failure here is auto-recoverable from within the election instance** — recovery always means deploying a fresh election with corrected configuration."  
  > "Recovery: Enclave re-publishes once reconnected. `publish_result` is idempotent by construction (B1 plus Block 6's `AlreadyResolved` rejection), so retries are safe."
- **Fix recommendation**: Either weaken the preamble to "no failure here is auto-recoverable except §4.6 (network partition)", or reclassify §4.6 recovery as requiring a fresh election instance (which would be false to the actual design). The honest fix is to scope the preamble: "no failure here is auto-recoverable from within the election instance **by administrative action**" — but then §4.6 is also auto-recoverable without admin action, so the exception must be named explicitly.

### 2. §4.3a detection claim contradicts its own cause clause [serious]
- **Category**: contradiction
- **Affected**: §4.3a
- **What's wrong**: The Cause includes "or was unregistered between instantiation and publish_result" — a runtime event. The Detection claim says "Detectable pre-deployment". A runtime unregistration that happens after deployment cannot be detected pre-deployment. The detection clause is therefore too strong and contradicts the temporal scope of the cause.
- **Cite**: > "Cause: The verified-rcv `zkdcap_vkey` was never registered on the chain's ZK module under the expected slot, or was unregistered between instantiation and `publish_result`."  
  > "Detection: Verifiable on-chain — any caller can query the registered vkey and compare to the expected verified-rcv image. Detectable pre-deployment."
- **Fix recommendation**: Split into two sub-modes: (a) never registered — detectable pre-deployment; (b) unregistered at runtime — detectable only by on-chain monitoring after instantiation. Or weaken Detection to "Verifiable on-chain; the never-registered case is detectable pre-deployment."

### 3. §4.7 and §4.5 introduce undefined severity classes [serious]
- **Category**: ambiguity
- **Affected**: §4.7 ("integrity-per-voter"), §4.5 ("canonicality")
- **What's wrong**: The document never defines a severity taxonomy beyond confidentiality / integrity / liveness. §4.7 tags "integrity-per-voter" and §4.5 tags "canonicality" without explaining how these relate to the three base classes. "Integrity-per-voter" appears to be a subset of integrity (one voter's ballot is corrupted); "canonicality" appears to be a liveness/integrity hybrid (fork ambiguity). Because the classes are undefined, a downstream spec writer cannot tell whether a mitigation for "integrity" also covers "integrity-per-voter", or whether "canonicality" needs separate test harnesses.
- **Cite**: > "Voter address compromise — *integrity-per-voter*"  
  > "Chain consensus halt or fork — *liveness or canonicality*"
- **Fix recommendation**: Map every tag to the base C/I/L taxonomy. "integrity-per-voter" → "integrity (per-voter scope)". "canonicality" → either "liveness" (if the issue is lack of a single agreed state) or "integrity" (if different forks admit different winners), or split the failure mode into two sub-modes with distinct tags.

### 4. §4.8 is not a failure mode but consumes a numbered slot [serious]
- **Category**: coverage gap
- **Affected**: §4.8
- **What's wrong**: The section title is "Failure Modes". §4.8 is explicitly labeled "not a failure". Including it in the enumerated failure modes list breaks the boundary of the section and makes automated extraction unreliable. A methodology that requires completeness checking of failure modes cannot distinguish §4.8 from actual failures without parsing the free-text subtitle. The honest place for this content is §5 (Non-Goals), where "No coordination / collusion resistance" is already listed.
- **Cite**: > "Candidate collusion / coordinated voting — *not a failure*"
- **Fix recommendation**: Remove §4.8 from the Failure Modes section and consolidate its content into §5 (Non-Goals), which already contains the matching bullet.

### 5. Missing failure mode: DstackKeyManager unavailability at instantiation [serious]
- **Category**: coverage gap
- **Affected**: §4 (absent)
- **What's wrong**: Block 1 (instantiate) Requires "DstackKeyManager-issued keypair derivable for this contract instance". If the KMS is unavailable at instantiation time, the contract creation fails with `InvalidInstantiation`. This is a system-level liveness failure (election never starts) distinct from §4.2 (KMS compromise, which assumes the KMS was available and later leaked) and §4.6 (network partition of the enclave post-instantiation). The methodology should require that any Block Requires clause with an external dependency has a corresponding failure mode.
- **Cite**: > (Block 1) "Requires: ... DstackKeyManager-issued keypair derivable for this contract instance"
- **Fix recommendation**: Add §4.x "KMS unavailability at instantiation — *liveness*" with cause "DstackKeyManager unreachable during `instantiate`", effect "contract creation fails; election never starts", detection "failed instantiation transaction", mitigation "operational KMS health checks".

### 6. Missing failure mode: enclave resource exhaustion during tabulation [serious]
- **Category**: coverage gap
- **Affected**: §4 (absent)
- **What's wrong**: Block E1 runs ECIES decryption plus IRV recursion inside a TDX enclave. For large candidate sets the working set could exceed enclave memory or CPU time limits. The enclave would abort without producing an attestation, causing a liveness failure that is distinct from §4.6 (network partition — the enclave is running but cannot reach the chain). The intent does not bound `len(candidates)`, so this failure is admissible. The methodology should require acknowledging resource-bound failures for unbounded inputs.
- **Cite**: > (Block E1) "Stage 1 — `decrypt_and_validate`: for each `(addr, ciphertext) ∈ ballots` ... Stage 2 — `IRV_spec`: run the IRV recursion on `(valid_ballots, candidates)`"
- **Fix recommendation**: Add §4.x "Enclave resource exhaustion — *liveness*" with cause "candidate set or ballot count exceeds enclave memory/CPU limits", effect "enclave aborts without attestation; election deadlocks", mitigation "pre-deployment bench-marking + candidate-set size cap".

### 7. §4.9 deadlock soundness relies on unstated enclave retry semantics [serious]
- **Category**: under-specification
- **What's wrong**: §4.9 claims "re-running the enclave produces the same malformed tally and `publish_result` rejects again. Infinite rejection loop." and "Detection: On-chain — repeated `InvalidTally` rejection events". However, Block E1 does not specify that the enclave retries at all. If the enclave is a one-shot process (run once, submit, exit on any error), there is exactly one `InvalidTally` event and then silence. The "infinite loop" and "repeated events" claims are only true under an unstated retry policy. Without that policy, the deadlock is silent and the detection mechanism fails.
- **Cite**: > "The enclave is deterministic over the ballot set; the ballot set is frozen at `end_at` (B2). Therefore re-running the enclave produces the *same* malformed tally and `publish_result` rejects again. Infinite rejection loop."  
  > "Detection: On-chain — repeated `InvalidTally` rejection events with identical `reason` field."
- **Fix recommendation**: Add an explicit requirement in Block E1 that the enclave must retry `publish_result` indefinitely (or with a documented backoff) until success or operator intervention. Without this, §4.9's detection claim is ungrounded.

### 8. §4.1 "potentially integrity" is ambiguous under the stated threat model [serious]
- **Category**: ambiguity
- **Affected**: §4.1
- **What's wrong**: §4.1 tags the failure as "confidentiality + potentially integrity". The text says integrity is lost only "if the vulnerability also permits forging attestations". But the Mitigation says this is "Inherited from Quartz `tdxVerifier` soundness assumption" — meaning that within the threat model, forging attestations is bounded by a negligible advantage. Therefore integrity loss is either out-of-scope (contingent on an upstream assumption) or impossible under the threat model. The "potentially" tag is misleading because it suggests integrity loss is an in-scope risk comparable to confidentiality loss.
- **Cite**: > "Effect: Adversary decrypts all on-chain ballots. If the vulnerability also permits forging attestations, integrity is also lost."  
  > "Mitigation: Inherited from Quartz `tdxVerifier` soundness assumption. Out of scope at the verified-rcv layer."
- **Fix recommendation**: Split into two sub-modes or re-tag: "confidentiality (in-scope); integrity (out-of-scope — contingent on `tdxVerifier` soundness failure, see Quartz spec)". Alternatively, if integrity is considered in-scope, the mitigation must address it at the verified-rcv layer, which the text says is out of scope.

## Slice-local summary
- Critical: 1
- Serious: 7
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE
The slice contains a critical self-contradiction between the universal preamble and §4.6's recoverability, multiple undefined severity tags, a non-failure occupying a failure-mode slot, and two significant coverage gaps (instantiation-time KMS unavailability, enclave resource exhaustion). Additionally, §4.9's deadlock detection claim is under-specified because it assumes unstated enclave retry behavior. The slice does not survive adversarial scrutiny.