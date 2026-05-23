# gemma-4-26b-a4b — verified-rcv intent v0.3.0 adversarial pass (Pattern B, per-section dispatch)

- **Pass**: Pattern B calibration (4th attack against v0.3.0 — same intent, OpenCode + file-access subagent dispatch)
- **Slices dispatched**: 9

---

## Slice: scope

*Elapsed: 919.0s, 3,008 chars.*

# gemma-4-26b-a4b — slice scope — System scope, identity, and external contracts

## Cross-section reads
- Section 1/5: Notes that legitimacy of election instances is off-chain, contradicting the "trust" requirement in Section 6.4.
- Section 1/5: Notes the lack of on-chain candidate-consent gates, which impacts the "honesty" requirement in Section 6.4.
- Section 1/5: Notes the lack of formal verification for upstream Quartz axioms, which impacts the reliability claim in Section 6.6.
- Section 1/6: Notes chain trust for ZK modules and registration, which impacts the reliability claim in Section 6.6.

## Attacks on this slice
### 1. Section 6.4 Over-specification of Caller Trust [serious]
- **Category**: over-specification
- **Affected**: Section 6.4 / line 524
- **What's wrong**: Section 6.4 states the instantiator "is trusted to declare candidates honestly" (524). However, Section 1/5 explicitly states that the legitimacy of any election instance is off-chain (23) and that there is "no on-chain candidate-consent gate" (452). Since the contract does not verify whether candidates have consented to run, the instantiator is not "trusted" to do something the contract doesn't care about; they are simply permitted to provide any valid chain addresses, and the contract will proceed regardless of their "honesty" regarding candidate legitimacy.
- **Cite**: > Is trusted to declare candidates honestly (verified-rcv does not gate this; legitimacy is off-chain per Section 5).
- **Fix recommendation**: Remove the "is trusted to" requirement for honesty regarding candidate legitimacy, as the contract explicitly disclaims gating on consent. Frame it instead as an operational assumption for the instantiator.

### 2. Section 6.6 Over-specification of Output Reliability [serious]
- **Category**: over-specification
- **Affected**: Section 6.6 / line 540
- **What's wrong**: Section 6.6 claims the Output Contract provides reliability on attestation (540). However, Section 1/5/6.1 admit the underlying attestation correctness (Quartz/zkdcap) and chain-side registration are outside the `verified-rcv` scope/verification (21, 465, 478, 480). A consumer relying on Section 6.6 for security is assuming something the spec explicitly disclaims (Section 5, 465). If the chain/Quartz are compromised, the Output Contract claim (540) is violated on-chain, yet the `verified-rcv` spec *claims* to provide this reliability as part of its defined output contract.
- **Cite**: > Any party reading `tally_result` from chain state can rely on: ... `tally_result.is_some()` ⇒ a valid `DstackAttestation` was supplied ... (540)
- **Fix recommendation**: Temper the claim in Section 6.6 to state that the observer can rely on the *on-chain evidence* of attestation, rather than the underlying cryptographic/operational validity of the attestation itself, which is explicitly delegated to upstream components.

## Slice-local summary
- Critical: 0
- Serious: 2
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-SLICE
---

## Slice: behaviors-core

**ERROR** (788.4s): exit=0: [0m
> spec-adversary · google/gemma-4-26b-a4b
[0m
[0m→ [0mRead .colosseum/intent.md

---

## Slice: behaviors-types

*Elapsed: 108.7s, 5,996 chars.*

# gemma-4-26b-a4b — slice behaviors-types

## Cross-section reads
- Section 3.1/3.2: Used to verify the temporal/state distinction for B6, B8, and B10.
- Section 4.9: Used to ground the liveness implications of `InvalidTally`.

## Attacks on this slice
### 1. Underspecified Map Iteration for `dropped_voters` [serious]
- **Category**: under-specification
- **Affected**: Section 2.5, Stage 1 (line 159)
- **What's wrong**: The spec states `dropped_voters` are ordered by "iteration order over `raw_ballots` (deterministic per Map iteration discipline)". However, the spec does not explicitly define the `Map<Addr, Vec<u8>>` iteration discipline for the contract state. While Section 134 attempts to anchor ordering to "candidate-declaration order" for `TallyResult` fields, it does not explicitly mandate that the underlying `Map` implementation (or its serialization/iterator) follows this order for *all* iteration-dependent outputs. If the `Map` implementation (e.g., a standard CosmWasm/Rust `BTreeMap`) uses address-sorting instead of declaration-order, the `dropped_voters` list will diverge from the `non_voters` list (which *is* explicitly ordered by declaration order in line 158), creating an inconsistent audit trail for the election results.
- **Cite**: > `dropped_voters` ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline).
- **Fix recommendation**: Explicitly mandate that `raw_ballots` (and any `Map<Addr, ...>` used in the tally) are iterated according to the `candidates` declaration order.

### 2. Ambiguous `IRV_spec` Termination for Co-winners [serious]
- **Category**: ambiguity
- **Affected**: Section 2.5, Stage 2 (lines 165, 172)
- **What's wrong**: The spec provides two distinct "tie" termination rules that are not clearly reconciled for the intermediate case. Clause 1 (line 165) says: "if all remaining candidates have equal first-place counts... all of them are co-winners." Clause 172 (Edge cases) says: "All remaining candidates tied for lowest: batch elimination would empty `remaining`. In this case, declare all current `remaining` candidates as co-winners." These are non-equivalent for an election with, say, 3 candidates where A=1, B=1, C=1. Clause 1 (Majority/Terminal tie) implies the game ends because everyone is tied for *first*. Clause 172 (Batch elimination/Empty set) implies the game ends because everyone is tied for *lowest*. While the outcome (co-winners) might be the same, the *logic path* for the `per_round_counts` and `eliminated_by_round` arrays is ambiguous. A formal verifier (Lean/Quint) needs to know if the "tie" happened because of the `Majority termination` check (Clause 2) or the `Recursive case/Batch elimination` failure (Clause 172).
- **Cite**: > `1. Base case ... if all remaining candidates have equal first-place counts in the current round, all of them are co-winners.` AND `172. All remaining candidates tied for lowest: ... declare all current remaining candidates as co-winners`.
- **Fix recommendation**: Unify the tie-handling logic into a single hierarchical check: (1) Check for Majority, (2) If no majority, check if `count.values().unique().len() == 1` (Universal Tie); (3) Otherwise, perform Batch Elimination.

### 3. Underspecified `EnclaveImage` Symbol Type [cosmetic]
- **Category**: under-specification
- **Affected**: Section 2.5, Type signatures (line 110/Context Appendix)
- **What's wrong**: `EnclaveImage` is defined as `Bytes → Bytes`. In the context of Section 4.9 and Block E1, it is used to represent the software image being verified. However, the spec does not clarify if `EnclaveImage` refers to the *binary blob* of the enclave, the *measurement (MRTD/RTMR)*, or a *cryptographic hash* of the image. Given that Section 2.5 (line 139) discusses Borsh encoding for ballots, the lack of a specific type/structure for `EnclaveImage` (e.g., `struct EnclaveImage { measurement: Bytes, signature: Sig }`) makes the "image-identity" binding in B10/B8 slightly loose for implementation.
- **Cite**: > `EnclaveImage : Bytes → Bytes (image_extract)`
- **Fix recommendation**: Define `EnclaveImage` as a specific record type containing the measurement and necessary attestation metadata, rather than an opaque function on `Bytes`.

### 4. Potential Composition Failure in `fires_at_transition` for B6 [serious]
- **Category**: composition failure
- **Affected**: Section 2.5, Transaction trace model (lines 149-150) and B6 (line 344)
- **What's wrong**: The `fires_at_transition(σ → σ′)` model defines the transition as being parameterized by a *multiset* of transactions. B6 (line 344) attempts to attribute changes to `∃ tx ∈ fires_at_transition(...)`. However, the spec does not define how the `next.ballots[k]` value (the *result* of the transition) is mapped back to a specific `tx.payload` when the transition is composed of *multiple* transactions in the same block (e.g., two `SubmitBallot` calls for the same candidate, where the second overwrites the first). If `fires_at_transition` contains `{tx1, tx2}`, and `next.ballots[k]` is the value from `tx2`, the existential `∃ tx` is satisfied, but the spec doesn't explicitly constrain that the *value* in `next.ballots[k]` must be the specific `tx.encrypted_preferences` of the *witnessing* transaction for that specific state change, which is critical for the `∀-per-key` strength of B6.
- **Cite**: > `A state transition σ → σ′ is parameterized by the multiset of transactions fires_at_transition(σ → σ′)` AND `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), ... next.ballots[k] = tx.encrypted_preferences)`
- **Fix recommendation**: Strengthen the trace model to define the transition as a sequential application of the multiset, ensuring that for any state variable `v`, the value `next.v` is the result of the *last* transaction in the trace that mutated `v`.

## Slice-local summary
- Critical: 0
- Serious: 3
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-SLICE
---

## Slice: state-invariants

*Elapsed: 162.8s, 2,491 chars.*

# gemma-4-26b-a4b — slice state-invariants — Structural (pointwise) invariants S1..S10

## Cross-section reads
- Section 2.5: Defines the `TallyResult` schema, which contains specific set-relation and length constraints for `dropped_voters`, `non_voters`, and `ballots_dropped` that are not captured in the structural invariants.

## Attacks on this slice
### 1. Ambiguous/Under-specified "Handler Index" for S5 [serious]
- **Category**: ambiguity
- **Affected**: S5 (line 326)
- **What's wrong**: S5 attempts to restate the temporal write-discipline (B1) as a pointwise state invariant by claiming it is "Evaluable on a single state by inspecting the handler index". However, no `handler_index` or similar variable (e.g., `last_executed_handler`) is defined in the contract state variables in Section 2.5 (lines 112-119). Without an explicit state variable tracking the last mutation, it is impossible to evaluate whether a write-discipline has been respected by inspecting the state `σ` alone. The invariant is either non-evaluable on the defined state, or the spec under-specifies the necessary state to support its own claim of pointwise evaluability.
- **Cite**: > S5 (derived) | terminality of resolution (state-shape) | ... Evaluable on a single state by inspecting the handler index, not by quantifying over successor states.

### 2. Coverage gap for `dropped_voters` and `non_voters` identity [serious]
- **Category**: coverage gap
- **Affected**: S1..S10 (Section 3.1)
- **What's wrong**: Section 2.5 defines critical set-relation and cardinality constraints for `dropped_voters` and `non_voters` within the `TallyResult` schema: `dropped_voters ⊆ ballots.keys`, `len(dropped_voters) = ballots_dropped`, and `non_voters = candidates \ ballots.keys`. While S7 uses `len(non_voters)` and `ballots_dropped` to check total conservation, the structural invariants (S1..S10) fail to mandate the *identity* of these sets. A state could satisfy S1..S10 while `dropped_voters` contains addresses not in `ballots.keys`, or `non_voters` contains candidates who *did* vote, violating the core schema defined in Section 2.5.
- **Cite**: > `dropped_voters: Vec<Addr>, // voters whose ballots were dropped; ⊆ ballots.keys; len = ballots_dropped` (Section 2.5, line 129); `non_voters: Vec<Addr>, // candidates who did not submit a ballot; = candidates \ ballots.keys` (Section 2.5, line 130).

## Slice-local summary
- Critical: 0
- Serious: 2
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-SLICE
---

## Slice: temporal-invariants

*Elapsed: 175.1s, 3,554 chars.*

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
---

## Slice: failure-modes

*Elapsed: 75.9s, 6,026 chars.*

# gemma-4-26b-a4b — slice failure-modes

## Cross-section reads
- Section 2.5: Used to verify the `InvalidTally` error mapping and the `well-formedness` predicate mentioned in 4.9.
- Section 3.1/3.2: Used to verify the liveness/integrity implications of failure modes (e.g., B1, B2, B10).
- Section 5: Used to check the consistency of "Non-Goals" against listed failure modes (e.g., coordination resistance).
- Section 6: Used to verify the boundaries of trust (e.g., chain, hardware, KMS) against the failure causes listed in 4.1, 4.2, and 6.1/6.3.

## Attacks on this slice
### 1. Underspecified liveness/integrity boundary for `publish_result` [serious]
- **Category**: under-specification
- **Affected**: §4.1, §4.2, §4.3a, §4.3b
- **What's wrong**: The failure modes for hardware/KMS compromise (4.1, 4.2) and attestation mismatch (4.3a, 4.3b) are categorized as *confidentiality*, *integrity*, or *liveness* based on the *immediate* technical effect, but they fail to account for the systemic liveness/integrity violation caused by the *lack of recovery*. For example, in 4.3a (vkey unregistered), the spec claims liveness loss because `publish_result` fails, but it doesn't explicitly state that the *entire election instance* becomes an unrecoverable liveness sink for the chain, which is a critical operational failure mode distinct from a transient network partition (4.6). The severity tagging for 4.1/4.2 ignores the liveness violation that occurs when an election *must* proceed but cannot because the keys are compromised/leaked, effectively freezing the state transition for that specific election instance.
- **Cite**: > ### 4.1 Enclave hardware compromise — *confidentiality + potentially integrity* ... ### 4.3a Attestation vkey unregistered — *liveness*
- **Fix recommendation**: Update the severity/effect descriptions for 4.1, 4.2, and 4.3a to explicitly include the liveness violation (terminal election deadlock) caused by the "no admin recovery" design decision.

### 2. Incomplete failure mode coverage for `submit_ballot` liveness/integrity [serious]
- **Category**: coverage gap
- **Affected**: §4.1..§4.9
- **What's wrong**: The failure modes focus heavily on the *tallying* phase (post `end_at`) and the *attestation/publication* phase, but they are silent on liveness/integrity failures during the *voting window* (pre `end_at`). Specifically, there is no failure mode for "Contract state corruption" or "Storage exhaustion" during the `submit_ballot` phase. While Section 6.1 assumes chain state persistence, an adversary could target the contract's ability to accept ballots (e.g., via gas-limit exhaustion on specific `submit_ballot` transactions or chain-level state bloat) which leads to an election where the `non_voters` set (Section 8.2) is artificially inflated, violating the intent of a fair election without triggering an `InvalidTally` or `AttestationFailed` error.
- **Cite**: (The entire §4 range lacks any mention of `submit_ballot` failure modes).
- **Fix recommendation**: Add a failure mode for "Voting window liveness/availability" (e.g., chain-level denial of service on `submit_ballot` transactions) to ensure the liveness/integrity implications are understood for the entire election lifecycle, not just the tallying phase.

### 3. Soundness/Liveness mismatch in `deterministic-malformed-tally` deadlock [serious]
- **Category**: ambiguity
- **Affected**: §4.9
- **What's wrong**: Section 4.9 attempts to bridge the gap between an integrity failure (wrong tally) and a liveness failure (deadlock). However, the cause/effect description is ambiguous regarding *why* the deadlock is liveness-critical rather than just an integrity violation. It states: "a well-formed but incorrect tally is a 4.4 integrity loss... a malformed tally is a 4.9 liveness loss." This assumes the chain *knows* the difference between "wrong" and "malformed" based on the `InvalidTally` error. If an attacker can cause the enclave to produce a *well-formed* but *incorrect* tally (4.4), they achieve an integrity violation; if the enclave produces something the chain *rejects* (4.9), they achieve liveness loss. The spec fails to explicitly state the adversary's goal here: an attacker may *prefer* 4.9 (deadlock) over 4.4 (wrong result) to deny service, or vice versa. The classification of the *same* underlying cause (enclave bug) as two different failure types based on the chain's error response is a classification of *observer effect*, not underlying failure, which obscures the true risk profile.
- **Cite**: > ### 4.9 Deterministic-malformed-tally deadlock — *liveness* ... A well-formed but incorrect tally is a 4.4 integrity loss (chain accepts wrong result); a malformed tally is a 4.9 liveness loss (chain rejects everything).
- **Fix recommendation**: Reframe 4.9 not as a distinct failure mode, but as the *Liveness manifestation of an Enclave Software Bug (4.4)*, clearly stating that the liveness/integrity outcome is determined by whether the bug results in a violation of the `well-formedness` predicate (Block 6).

### 4. Misplaced severity for `voter address compromise` [cosmetic]
- **Category**: triviality
- **Affected**: §4.7
- **What's wrong**: The severity for 4.7 is listed as `integrity-per-voter`. Given the "no admin / creator recovery" and "last-write-wins" semantics explicitly defined in Section 5, the impact of a single voter's key compromise is contained to that specific voter. Labeling it as an `integrity` failure for the *system* is slightly misleading; it is an integrity violation of the *voter's intent*, which the spec correctly identifies as "out of scope" for the contract layer.
- **Cite**: > ### 4.7 Voter address compromise — *integrity-per-voter*
- **Fix recommendation**: Change the severity tag to `intent-integrity` or move it to a sub-category to clarify that the *system* integrity (the tallying of valid ballots) remains uncompromised.

## Slice-local summary
- Critical: 0
- Serious: 3
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-SLICE
---

## Slice: trust-quartz

*Elapsed: 775.0s, 4,396 chars.*

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
---

## Slice: scenarios

*Elapsed: 139.7s, 3,557 chars.*

# gemma-4-26b-a4b — slice scenarios

## Cross-section reads
- §3.1 / §3.2: Used to identify the set of B-series temporal invariants that require witnessing (B1, B2, B3, B4, B5, B6, B7, B8, B9, B10).
- §8.7: Used to confirm that the witness for B10 is located outside the target slice.

## Attacks on this slice
### 1. Coverage Gap (B1, B2, B3, B7) [serious]
- **Category**: coverage gap
- **Affected**: §8.1..§8.6
- **What's wrong**: The target slice fails to provide witnessing scenarios for critical temporal invariants B1 (tally_result monotone-once-set), B2 (no late ballots), B3 (no premature tally), and B7 (terminal-state immutability). While §8.1 mentions the `NotInVotingWindow` error (addressing B2) and §8.5 witnesses the rejection of `publish_result` (addressing B1/B5), none of the scenarios explicitly demonstrate the violation/rejection of an attempt to submit a late ballot (B2) or start a tally prematurely (B3). Furthermore, the persistence of the `Resolved` state (B1/B7) is only indirectly addressed via the rejection of `publish_result`, rather than through an explicit witness of state terminality.
- **Cite**: > B1 | tally_result monotone-once-set | temporal | `always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))`
- **Fix recommendation**: Add specific scenarios for: (1) an attempt to `submit_ballot` after `end_at` being rejected (witness for B2); (2) an attempt to `close_and_tally` before `end_at` being rejected (witness for B3); and (3) an explicit mention/step in the `Resolved` state showing no further transitions are possible for any valid handler (witness for B1/B7).

### 2. Coverage Gap (B10) [serious]
- **Category**: coverage gap
- **Affected**: §8.1..§8.6
- **What's wrong**: The core correctness obligation, B10 (tally-correctness), is not witnessed within the target slice. The witness for B10 (the cross-layer composition of `B10_lean`, image-identity, and B8) is explicitly located in §8.7, which falls outside the defined `TARGET_SLICE`.
- **Cite**: > B10 | tally-correctness | cross-layer | `always (next.tally_result.is_some() ∧ tally_result.is_none() → ...)`
- **Fix recommendation**: This is an architectural/organization issue; the `TARGET_SLICE` definition in the calling context should be expanded to include §8.7, or the intent document should clearly group all B-series witnesses together.

### 3. Under-specification (B1/B5/B7 witness completeness) [cosmetic]
- **Category**: under-specification
- **Affected**: §8.5
- **What's wrong**: Scenario 8.5 witnesses the *rejection* of a second `publish_result` transaction, which protects B1 (monotone-once-set) and B5 (at most once). However, the scenario does not explicitly verify the *persistence* of the `tally_result` value (the "always" part of B1/B7) against the specific claim that `next.tally_result = tally_result`. It shows the transaction reverts, which implies no change, but doesn't explicitly tie the outcome to the persistence of the *existing* value in the context of the temporal claim.
- **Cite**: > B1 | tally_result monotone-once-set | temporal | `always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))`
- **Fix recommendation**: Update the "Step-by-step" in §8.5 to explicitly state that after the revert, `tally_result` remains exactly equal to the original `T1`, explicitly linking the rejection to the persistence requirement of B1.

## Slice-local summary
- Critical: 0
- Serious: 2
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-SLICE
---

## Slice: off-chain-witness

*Elapsed: 758.1s, 4,387 chars.*

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
