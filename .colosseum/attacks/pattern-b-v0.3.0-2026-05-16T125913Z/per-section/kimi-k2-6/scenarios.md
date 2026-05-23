# kimi-k2-6 — slice scenarios

## Cross-section reads
- Section 3.2 (B9 definition) — to verify the 4-summand bound vs 8.1's "5-summand" claim.
- Section 6.2 (B9 decomposition lineage) — to confirm the 4-summand count and the removal of the 5th summand in prior revision.

## Attacks on this slice

### 1. Stale B9 summand count in 8.1 contradicts Section 3.2 [critical]
- **Category**: contradiction
- **Affected**: §8.1, Trust claim consumed line
- **What's wrong**: §8.1 states the trustworthiness of the published result "reduces to the 5-summand negligibility budget defined in Section 3." Section 3.2's B9, however, is explicitly a "4-summand bound, conditioned on the operational precondition `image_registration_honest`." The 5th summand (`Adv_image_registration`) was removed per Round 3a first-pass adversary Attack 18 and the decomposition lineage in §6.2 confirms "4 after Round 3a first-pass adversary (dropped `Adv_KMS_leakage`... and `Adv_image_registration`)". The scenario text is stale relative to the invariant definition, creating a direct numerical contradiction that misleads downstream spec writers about the security budget cardinality.
- **Cite**: > "Trust claim consumed: B8 + B9 — the published result is bound to the verified-rcv enclave's attested computation, and its trustworthiness reduces to the 5-summand negligibility budget defined in Section 3."
- **Fix recommendation**: Change "5-summand" to "4-summand" in §8.1 and §8.2, and add a parenthetical note naming the dropped summand so readers understand the revision history.

### 2. B9 is a probabilistic bound, not witnessable by a narrative trace [serious]
- **Category**: under-specification
- **Affected**: §8.1 and §8.2, Trust claim consumed lines
- **What's wrong**: §8.1 and §8.2 claim to "consume" B9 as a trust claim witnessed by the scenario. B9 is a negligibility-budget inequality: `Pr[B8 violated by a polynomial-time adversary] ≤ sum of negligible functions`. A single happy-path narrative trace cannot witness, test, or demonstrate a probability bound over all polynomial-time adversaries. Claiming B9 is "consumed" by a functional scenario conflates a cryptographic security statement with a functional test case, misleading downstream verification about what kind of artifact actually discharges B9 (it requires a proof, not a scenario).
- **Cite**: > "Trust claim consumed: B8 + B9 — the published result is bound to the verified-rcv enclave's attested computation, and its trustworthiness reduces to the 5-summand negligibility budget defined in Section 3."
- **Fix recommendation**: Remove B9 from the "Trust claim consumed" lines in §8.1 and §8.2. Replace with a note that B9 is a proof obligation (discharged by the Lean/Quartz composition), not a scenario-level witness. B8 alone is the correct trace-level claim for these scenarios.

### 3. B1, B2, B3, and B7 lack scenario witnesses in §8.1–§8.6 [serious]
- **Category**: coverage gap
- **Affected**: §8.1–§8.6 overall; specifically B1, B2, B3, B7 from §3.2
- **What's wrong**: The slice is characterized as "witnesses for B1..B6, B10," yet no scenario in §8.1–§8.6 explicitly witnesses B1 (tally_result monotone-once-set), B2 (no late ballots — ballot store frozen at end_at), B3 (no premature tally — causal gate that publish_result requires env.block.time ≥ end_at), or B7 (terminal-state immutability — Resolved is a sink). B1 is mentioned only as a supporting assertion in §8.5's B5 witness; B2 is conflated with B4 in §8.1's passing mention of late-ballot rejection; B3 and B7 are entirely absent. A scenario section that omits nearly half of the B-series invariants it is supposed to witness leaves significant gaps in downstream test and spec coverage.
- **Cite**: > (slice identifier) "scenarios — Concrete scenarios §8.1..§8.6 (witnesses for B1..B6, B10)"
- **Fix recommendation**: Add explicit scenario witnesses: (a) a "Result immutability" scenario showing a post-Resolution mutation attempt rejected (B1 + B7); (b) a "Ballot store frozen after end_at" scenario showing that ballots do not change even if a late submit_ballot is attempted (B2); (c) a "Premature tally rejected" scenario showing publish_result before end_at rejected (B3).

### 4. §8.6 does not actually witness B6 — no ballot write occurs [serious]
- **Category**: under-specification
- **Affected**: §8.6, title and Trust claim consumed line
- **What's wrong**: §8.6 is titled "Impersonation attempt rejected (witness for B6)" and claims to consume B6. B6 states: `∀ k, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`. The scenario shows a non-candidate (`Eve`) attempting to submit a ballot and being rejected. No ballot is written; `next.ballots = ballots`. Therefore the antecedent of B6 is false, and the implication is vacuously satisfied — the scenario does not exercise the actual load-bearing content of B6 (attribution of a ballot write to its submitting transaction). The positive case (a candidate successfully submits, and the written ballot is attributable to their transaction) is absent. The "stolen-key variant" paragraph even admits the contract cannot verify the voter's intent, further undermining the claim that this scenario witnesses B6's writer-voter binding.
- **Cite**: > "Given a active election in Voting state, when a non-candidate chain address attempts to submit a ballot, the contract rejects with NotACandidate; the on-chain msg.sender check makes voter-identity forgery impossible at the contract layer." ... "Transaction reverts atomically with ContractError::NotACandidate. ballots is unchanged; no event is emitted." ... "Trust claim consumed: B6 (ballot writer is the ballot voter)"
- **Fix recommendation**: Retitle §8.6 as "Non-candidate ballot rejection (Block 3 precondition witness)" and add a new §8.X scenario showing a candidate successfully submitting a ballot and the ballot being attributable to that transaction (B6 positive witness). Alternatively, reframe the existing §8.6 as witnessing Block 3's Requires clause, not B6.

### 5. §8.4 incomplete B4 witness — only lower time bound is shown [serious]
- **Category**: under-specification
- **Affected**: §8.4, title and step-by-step
- **What's wrong**: §8.4 is titled "Premature voting attempt rejected (witness for B4)". B4 is `always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)` — a two-sided time bound. The scenario only demonstrates the lower bound: a candidate attempts to vote before `start_at` and is rejected. It does not demonstrate the upper bound: a candidate attempting to vote at or after `end_at` and being rejected. While §8.1 mentions late ballots are rejected, that mention is not framed as a B4 witness. A scenario claiming to witness a two-sided temporal bound should cover both sides, or explicitly disclaim the partial coverage.
- **Cite**: > "Given a freshly instantiated contract in Created state, when a candidate attempts to submit a ballot before start_at, the contract rejects the transaction with NotInVotingWindow; no state mutates." ... B4: "always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)"
- **Fix recommendation**: Expand §8.4 to include a step showing a late-ballot rejection after `end_at`, or split into two sub-scenarios (pre-start_at and post-end_at) each explicitly tied to the corresponding conjunct of B4.

### 6. B10 witness is outside the slice in §8.7, contradicting slice characterization [serious]
- **Category**: coverage gap
- **Affected**: §8.1–§8.6 scope vs §8.7
- **What's wrong**: The slice identifier characterizes §8.1–§8.6 as "witnesses for B1..B6, B10." However, B10's witness scenario is explicitly placed in §8.7 ("B10 witness — cross-layer discharge"), which is outside the slice. B10 is not mentioned in §8.1–§8.6 at all. This means the slice fails to witness B10 despite the characterization claiming it does. The separation is intentional per the text ("B10 is the only temporal invariant without a Section-8 chain-side witness scenario, by construction"), but the slice identifier ignores this architectural fact.
- **Cite**: > (slice identifier) "scenarios — Concrete scenarios §8.1..§8.6 (witnesses for B1..B6, B10)" ... §8.7: "B10 witness — cross-layer discharge (Lean + image-identity binding + B8)"
- **Fix recommendation**: Update the slice identifier to accurately reflect that B10 is witnessed in §8.7, not §8.1–§8.6. If §8.1–§8.6 must witness B10, add a cross-reference in §8.1 pointing to §8.7 as the B10 witness location.

### 7. §8.3 witnesses no B-series invariant [serious]
- **Category**: coverage gap
- **Affected**: §8.3 overall
- **What's wrong**: §8.3 ("Deadlock via missing enclave publish") is inside a slice characterized as "witnesses for B1..B6, B10," yet it consumes no B-series invariant. Its trust claim is "the explicit failure mode 4.6 + Section 5 non-goal 'no admin / creator recovery role'." While deadlock-as-spec'd is a valid design decision, including a scenario that witnesses zero B-series invariants inside a slice whose purpose is B-series witness coverage dilutes the traceability matrix. Downstream test derivation from this scenario will not produce any B-series invariant test cases.
- **Cite**: > "Trust claim consumed: the explicit failure mode 4.6 + Section 5 non-goal 'no admin / creator recovery role'. The deadlock is part of the spec, not a bug."
- **Fix recommendation**: Either (a) move §8.3 outside the B-series witness slice and characterize it as a "failure mode scenario" rather than an invariant witness, or (b) add an explicit B-series linkage — e.g., note that §8.3 is the negative witness for the absence of a liveness invariant (Section 5: "No liveness invariant binds publish_result"), which is a meta-witness for the B-series design boundary.

## Slice-local summary
- Critical: 1
- Serious: 6
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE