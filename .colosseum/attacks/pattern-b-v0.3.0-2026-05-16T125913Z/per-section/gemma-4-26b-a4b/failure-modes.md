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