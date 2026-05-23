# kimi-k2-6 — slice trust-quartz

## Cross-section reads
- §3.2 B9 (lines 347) — to verify `image_registration_honest` is used as a formal antecedent in an `always`-modality probability bound with 4 summands.
- §4.1 (lines 364-370) — to confirm TDX compromise effect includes "forging attestations" (integrity loss).
- §4.3a/4.3b (lines 380-394) — to confirm failure modes cover vkey registration issues, not ZK verification-algorithm bugs.
- Block 1 (§2.5, lines 185-200) — to confirm `instantiate` stores only `candidates`, `start_at`, `end_at`, `enclave_pubkey`; no vkey or MRTD/RTMR is stored.

## Attacks on this slice

### 1. De-retracted status block over-claims substrate readiness [critical]
- **Category**: composition failure
- **Affected**: §6.2 B8/B9 substrate status block `[2026-05-15 — de-retracted]`
- **What's wrong**: The banner proclaims the retraction "resolved upstream on 2026-05-14/15" and asserts "verified-rcv inherits B8/B9 from a now-contented substrate." But the caveats immediately below admit that Quartz asks 6 and 7 "have documented form in the Quartz PR; enforced form is gated on the executable-layer decision." This means the substrate is only partially ready — critical methodology obligations (per-conjunct failure-mode analysis and degenerate-zero-advantage intent declaration) are documented but not enforced. Furthermore, the document cites commit paths in the Quartz repo but provides no evidence that verified-rcv independently audited those commits. The "now-contented" claim is a statement about Quartz's state, not verified-rcv's verification of it. By presenting a clean resolution while hiding unenforced gaps in caveats, the spec misleads readers about the solidity of its cryptographic foundation.
- **Cite**: > "The retraction has been resolved upstream on 2026-05-14/15." ... "These asks have documented form in the Quartz PR; enforced form is gated on the executable-layer decision."
- **Fix recommendation**: Restate the status banner as `[2026-05-15 — partially de-retracted]` and add an explicit caveat that verified-rcv has not independently re-verified the Quartz refactor; the inheritance claim is provisional pending enforcement of asks 6+7 and a verified-rcv-side audit.

### 2. `image_registration_honest` is undefined operational prose wrapped in B9's formal modality [critical]
- **Category**: ambiguity
- **Affected**: §6.1 Enclave-image registration integrity bullet (line 480)
- **What's wrong**: The bullet describes the assumption in operational prose: "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an operational assumption on the deployment / governance surface... not a cryptographic property reducible to a security parameter." But B9 (§3.2) uses `image_registration_honest` as a formal antecedent in `always (image_registration_honest → Pr[B8 violated ...] ≤ ...)`. The slice never defines `image_registration_honest` as a state predicate, transition relation, or formal typing — it is governance prose masquerading as a logical symbol. Because it is undefined, B9 reduces to "if [unspecified operational condition], then bound holds" — epistemically equivalent to having no bound at all for the operational-failure case. The trust boundary section fails to supply the formal definition that B9 requires.
- **Cite**: > "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an **operational** assumption on the deployment / governance surface of the chain's vkey-registration mechanism, not a cryptographic property reducible to a security parameter."
- **Fix recommendation**: Define `image_registration_honest` as a concrete evaluable predicate, e.g., `chain_registered_vkey(σ) = expected_vkey ∧ chain_registered_mrtd(σ) = expected_mrtd`, with explicit evaluation semantics (evaluated at the `publish_result` transition, not under an `always` modality). Alternatively, restate B9 as a conditional meta-theorem under a static assumption rather than a temporal invariant.

### 3. ZK module "iff" assumption lacks failure mode coverage [serious]
- **Category**: coverage gap
- **Affected**: §6.1 ZK module endpoint semantics bullet (line 478)
- **What's wrong**: The bullet states: "`/xion.zk.v1.Query/ProofVerifyGnark` returns true iff the supplied proof verifies against the registered vkey under gnark's verification algorithm." The "iff" is a strong functional-correctness claim — no false positives, no false negatives. But the document lists no failure mode for "ZK module verification algorithm is buggy" or "gnark implementation diverges from spec." Failure modes 4.3a and 4.3b cover vkey registration issues, not the verification algorithm itself. If the ZK module has a bug that accepts invalid proofs, B8 clause (b) is violated with high probability, yet this is not captured by B9's negligibility budget or any failure mode. The Non-Goals section (§5) excludes "formal verification of upstream-Quartz axioms," but the ZK module is an Xion chain component, not a Quartz artifact — so the exclusion does not apply. This is an unacknowledged, load-bearing assumption.
- **Cite**: > "ZK module endpoint semantics — `/xion.zk.v1.Query/ProofVerifyGnark` returns true iff the supplied proof verifies against the registered vkey under gnark's verification algorithm. Trusted as Xion-spec'd; not re-verified at this layer."
- **Fix recommendation**: Add a failure mode for ZK module algorithmic compromise (e.g., "ZK verification bug — accepts invalid proofs"), or weaken the "iff" to "is intended to return true iff" with an explicit caveat that module correctness is assumed and not verified at this layer.

### 4. Groth16 inheritance row conflates two distinct B9 summands [serious]
- **Category**: disjunction-vs-decomposition collapse
- **Affected**: §6.2 Quartz inheritance table, row 2 (`verifyGroth16`, line 489)
- **What's wrong**: The table lists a single row for `verifyGroth16` with discharge path "ArkLib Groth16 KS reduction + reference DCAP circuit-equivalence theorem." But B9's negligibility bound contains TWO separate Groth16-related summands: `Adv_groth16_KS(n)` and `Adv_circuit_eq(n)`. The status block confirms this decomposition: "the `tdxVerifier`-tagged + two `groth16Verifier`-tagged summands of that lift." A single table row with a combined discharge path obscures the mapping. A reader cannot tell whether `verifyGroth16` is one claim with two reduction steps, or whether two separate Quartz claims are collapsed into one row. The honest form would decompose this into named summands matching B9's budget.
- **Cite**: > "`verifyGroth16` accepts a proof iff it is a valid Groth16 proof of the encoded TDX-quote-validity statement under the registered vkey" ... "ArkLib Groth16 KS reduction + reference DCAP circuit-equivalence theorem"
- **Fix recommendation**: Split the `verifyGroth16` row into two rows — one for `Adv_groth16_KS` (Groth16 knowledge-soundness) and one for `Adv_circuit_eq` (DCAP circuit-equivalence) — each with its own discharge path and explicit mapping to the B9 summand it supports.

### 5. TDX integrity assumption is not connected to B9's precondition structure [serious]
- **Category**: composition failure
- **Affected**: §6.3 TDX integrity bullet (line 515)
- **What's wrong**: §6.3 states "TDX integrity — the TDX platform isolates the enclave's memory from the untrusted host... Failure mode 4.1." Cross-reading §4.1 confirms that a TDX vulnerability "also permits forging attestations" — i.e., B8 can be violated. But B9 (§3.2) has no precondition for TDX integrity and no summand for TDX compromise. If TDX is compromised, a polynomial-time adversary can forge attestations with probability ≈ 1, which is not bounded by the 4 negligible summands. §6.3 does not flag that B9's bound implicitly depends on this assumption. The trust boundary presents TDX integrity as a standalone item without explaining its load-bearing role for B9, creating a dangerous gap: a reader might believe B9 holds regardless of TDX state.
- **Cite**: > "TDX integrity — the TDX platform isolates the enclave's memory from the untrusted host; the enclave's runtime state is not observable to the host operator. Failure mode 4.1."
- **Fix recommendation**: Add an explicit note to §6.3: "B9's negligibility bound is only valid under this assumption; TDX compromise is outside the cryptographic model and would falsify the bound." Alternatively, add a `tdx_integrity_honest` precondition to B9.

### 6. commitHashE NOT CONSUMED reason omits finite-type mismatch [serious]
- **Category**: under-specification
- **Affected**: §6.2 Quartz inheritance table row 4 (`commitHashE`, line 491) and Note A (line 493)
- **What's wrong**: Note A explains that `commitHashE` is NOT CONSUMED because "that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous." But the table's discharge path says "VCVio `randomOracle` + `[Fintype UserData]` carrier refinement." The `Fintype UserData` assumption means the user data space is finite. Verified-rcv's `Adv_commitTally_CR` bounds SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`, where the input space is NOT finite (contract addresses and tally bodies vary in size). The document never mentions this finite-type mismatch as a reason for non-consumption. The stated reason is a methodological classification issue; the unstated reason is a technical incompatibility. By omitting the type mismatch, the document prevents readers from assessing whether `commitHashE` could be adapted for verified-rcv or whether the two hash assumptions are fundamentally incomparable.
- **Cite**: > "VCVio `randomOracle` + `[Fintype UserData]` carrier refinement; **NOT CONSUMED by verified-rcv — see Note A below**" ... "that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous."
- **Fix recommendation**: Add to Note A: "Additionally, `commitHashE`'s `[Fintype UserData]` carrier does not apply to verified-rcv's variable-length `Borsh(contract_addr ‖ tally_body)` inputs, making the bundle technically incompatible even if the sub-tag were content-bearing."

### 7. Block time monotonicity lacks failure mode [serious]
- **Category**: coverage gap
- **Affected**: §6.1 Block time monotonicity bullet (line 475)
- **What's wrong**: The bullet assumes "`env.block.time` is non-decreasing across consecutive blocks" and notes this is "Used by every state-derived predicate (Created / Voting / Tallying)." But there is no failure mode for non-monotonic block time. In Cosmos SDK, block timestamps are proposer-determined and can go backwards (malicious or buggy proposer). If `env.block.time` decreases, a contract in Tallying or Resolved could revert to Voting or Created, breaking temporal invariants B3 (no premature tally) and B4 (no premature voting). The document acknowledges fork resolution as a trust boundary but not block-time regression as a failure scenario. This is a coverage gap — the assumption is load-bearing but unguarded.
- **Cite**: > "Block time monotonicity — `env.block.time` is non-decreasing across consecutive blocks. Used by every state-derived predicate (Created / Voting / Tallying)."
- **Fix recommendation**: Add a failure mode for non-monotonic block time (e.g., "Block time regression — proposer produces block with earlier timestamp, causing derived-state oscillation"), or explicitly state that this assumption is unguarded and chain-dependent.

### 8. Enclave-image registration integrity conflates vkey and MRTD/RTMR [serious]
- **Category**: disjunction-vs-decomposition collapse
- **Affected**: §6.1 Enclave-image registration integrity bullet (line 480)
- **What's wrong**: The bullet says "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification." This treats vkey and MRTD/RTMR as a single "registration." But they are used in different B8 clauses (vkey in clause (b) for zkdcap verification; MRTD/RTMR in clause (d) for enclave identity matching) and likely stored in different chain mechanisms. They can be compromised independently: governance could substitute the vkey while leaving the MRTD intact, or vice versa. Lumping them together means a partial compromise is not covered. Also, "at instantiation" is misleading — Block 1 stores only `candidates`, `start_at`, `end_at`, `enclave_pubkey`; the vkey and MRTD are assumed to exist on-chain, not stored by the contract.
- **Cite**: > "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."
- **Fix recommendation**: Decompose into two separate bullets — "Vkey registration integrity" and "Enclave-image identity (MRTD/RTMR) registration integrity" — each with its own failure mode reference. Clarify that "at instantiation" means "assumed correct on-chain at the time of contract instantiation," not "stored by the contract during instantiation."

## Slice-local summary
- Critical: 2
- Serious: 6
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE

The trust-quartz slice contains two critical issues and six serious issues. The de-retracted status block over-claims the readiness of the Quartz substrate while hiding unenforced methodology gaps (Attack 1). The `image_registration_honest` predicate is undefined operational prose that B9 treats as a formal antecedent, making the bound unverifiable (Attack 2). Additionally, the ZK module assumption lacks a failure mode (Attack 3), the Groth16 inheritance row collapses two summands into one (Attack 4), TDX integrity is disconnected from B9's precondition structure (Attack 5), the commitHashE non-consumption reason is incomplete (Attack 6), block time monotonicity has no failure mode (Attack 7), and the registration integrity bullet conflates two independently compromisable values (Attack 8). These flaws materially weaken the spec's honesty about its trust boundaries and cross-project composition.