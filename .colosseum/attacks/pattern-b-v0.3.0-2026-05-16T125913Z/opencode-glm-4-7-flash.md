# glm-4-7-flash — verified-rcv intent v0.3.0 adversarial pass (Pattern B, per-section dispatch)

- **Pass**: Pattern B calibration (4th attack against v0.3.0 — same intent, OpenCode + file-access subagent dispatch)
- **Slices dispatched**: 6

---

## Slice: scope

*Elapsed: 60.3s, 12,223 chars.*

I'll read the specified sections from the intent document and then produce the attack report.
# glm-4-7-flash — slice scope

## Cross-section reads
- Section 2.5 (transaction trace model, lines 149-150) — needed to ground B6's `fires_at_transition` quantification and understand how Block 3's `msg.sender ∈ candidates` check enforces writer attribution.
- Section 3.2 (temporal invariants, lines 337-348) — needed to understand B6's temporal form and why the `∀-per-key` formulation is load-bearing.
- Section 4.4 (enclave software bug, lines 396-402) — needed to ground the claim that B10 is the load-bearing methodology target.
- Section 6.2 (Quartz inheritance, lines 482-510) — needed to understand B8/B9's cross-project dependency and why B9's negligibility budget is conditional on `image_registration_honest`.

## Attacks on this slice

### 1. Identity overreach — system scope claims more than the spec delivers [serious]

- **Category**: over-specification
- **Affected**: Section 1, lines 21-22
- **What's wrong**: The System Identity clause states: "Scope: end-to-end system spanning (i) a CosmWasm contract on a public Xion-class chain handling election lifecycle (instantiate / submit-ballot / close-and-tally / publish-result), (ii) a dstack-TDX enclave performing tabulation over ECIES-encrypted ballots, (iii) zkdcap attestation of the enclave's tally output. Specifies protocol-level behavior across all three components."

This is a categorical claim that the intent specifies protocol-level behavior for *all three components*. However, the only contract-level behavior specified is in Blocks 1-6 (the CosmWasm contract state machine). Block E1 describes enclave behavior, but it is explicitly labeled "off-chain, attested" and its inputs/outputs are not governed by contract-level protocol semantics (e.g., there is no contract handler that receives enclave output; the enclave directly calls `publish_result`). The attestation layer is described in B8/B9 but those are temporal invariants that reference off-chain verification steps, not contract-level protocol rules.

The spec therefore over-specifies the scope: it claims to specify protocol-level behavior for the enclave and attestation, but only provides contract-level behavior for those components. This creates a mismatch between the stated scope and the actual coverage of the intent.

- **Cite**: > "Scope: end-to-end system spanning (i) a CosmWasm contract on a public Xion-class chain handling election lifecycle (instantiate / submit-ballot / close-and-tally / publish-result), (ii) a dstack-TDX enclave performing tabulation over ECIES-encrypted ballots, (iii) zkdcap attestation of the enclave's tally output. Specifies protocol-level behavior across all three components."

- **Fix recommendation**: Restrict the scope claim to "protocol-level behavior for the CosmWasm contract layer" and reframe the enclave/attestation descriptions as "off-chain behavior that is attested to the contract layer" rather than "protocol-level behavior." The intent should explicitly state that enclave and attestation behaviors are not governed by contract-level protocol semantics but by external attestation and verification processes.

### 2. Non-goals as coverage gaps — intentional omissions that should be in-scope [serious]

- **Category**: coverage gap
- **Affected**: Section 5, lines 458-462
- **What's wrong**: Section 5 lists several items as "Non-Goals" that are actually features the system does not provide at all, not just features it deliberately omits. This creates a misleading impression that the system intentionally excludes these features, when in reality the system simply does not address them.

Specifically:
- Line 458: "No coercion resistance (post-tally)." — The system does not provide any mechanism to prevent coercion, but this is listed as a non-goal rather than being omitted entirely.
- Line 459: "No coordination / collusion resistance." — The system does not provide any mechanism to detect or prevent collusion, but this is listed as a non-goal rather than being omitted entirely.
- Line 462: "No threshold or quorum requirement." — The system does not provide any mechanism to prevent degenerate elections, but this is listed as a non-goal rather than being omitted entirely.

These are not intentional design tradeoffs; they are simply features that the system does not provide. Listing them as non-goals creates a false impression that the system has considered these features and deliberately excluded them, when in reality the system has not considered them at all.

- **Cite**: > "No coercion resistance (post-tally). A voter can be compelled to reveal their preferences after tally by revealing their private key. The ballot is private from the chain and enclave-host operators; it is not private from the voter themselves nor from anyone the voter chooses to share with." (line 458)
> "No coordination / collusion resistance. Candidates may coordinate vote patterns off-chain (Section 4.8). The system tallies correctly against whatever ballots are cast." (line 459)
> "No threshold or quorum requirement. Elections with zero ballots, all-malformed ballots, or any participation level proceed to tally; results are mathematically defined for any input." (line 462)

- **Fix recommendation**: Remove these items from the Non-Goals section entirely. They are not intentional design tradeoffs; they are simply features that the system does not provide. If the intent author wants to acknowledge that these features are out of scope, they should be listed in a separate "Out-of-Scope Features" section, not as non-goals.

### 3. Trust boundary overreach — caller contract trust claims are unenforceable [serious]

- **Category**: coverage gap
- **Affected**: Section 6.4, lines 523-525
- **What's wrong**: Section 6.4 states that the instantiator "Is trusted to declare candidates honestly (verified-rcv does not gate this; legitimacy is off-chain per Section 5)" and "Cannot recover from misconfiguration — instantiation parameters are immutable."

These are trust boundary claims that the system trusts the instantiator to behave honestly and cannot recover from misconfiguration. However, the spec provides no mechanism to enforce these trust boundaries or to detect violations. The instantiator can declare any set of candidates, including malicious or invalid candidates, and the system will accept them without any validation. Similarly, once instantiated, the parameters are immutable, so there is no recovery mechanism even if the instantiator realizes they made a mistake.

This creates a coverage gap: the intent claims to trust the instantiator to do something, but provides no mechanism to verify or enforce that trust. The system is vulnerable to malicious instantiations that declare invalid or malicious candidate sets.

- **Cite**: > "Is trusted to declare candidates honestly (verified-rcv does not gate this; legitimacy is off-chain per Section 5)." (line 524)
> "Cannot recover from misconfiguration — instantiation parameters are immutable." (line 525)

- **Fix recommendation**: Either (a) add contract-level validation for candidate sets (e.g., check that candidate addresses are valid, non-self-referential, etc.) and provide a recovery mechanism (e.g., a `reconfigure` handler), or (b) remove the trust boundary claim and explicitly state that the system does not validate candidate sets and is vulnerable to malicious instantiations. The current formulation creates a false impression of trust that is not supported by the spec.

### 4. Trust boundary overreach — voter contract trust claims are unenforceable [serious]

- **Category**: coverage gap
- **Affected**: Section 6.5, lines 531-533
- **What's wrong**: Section 6.5 states that each candidate "Must encrypt to the contract's advertised enclave_pubkey using ECIES (the contract does not verify encryption shape on-chain; malformed ciphertext is dropped by the enclave at tally time)" and "Is responsible for their own chain private key security (failure mode 4.7)."

These are trust boundary claims that the system trusts voters to encrypt correctly and to secure their private keys. However, the spec provides no mechanism to enforce these trust boundaries or to detect violations. The contract does not verify encryption correctness on-chain; malformed ciphertexts are only detected by the enclave at tally time, which is too late to prevent malicious ballots from being submitted. Similarly, there is no mechanism to prevent key theft or ballot forgery.

This creates a coverage gap: the intent claims to trust voters to do something, but provides no mechanism to verify or enforce that trust. The system is vulnerable to malicious voters who submit malformed ballots or forge ballots.

- **Cite**: > "Must encrypt to the contract's advertised enclave_pubkey using ECIES (the contract does not verify encryption shape on-chain; malformed ciphertext is dropped by the enclave at tally time)." (line 531)
> "Is responsible for their own chain private key security (failure mode 4.7)." (line 533)

- **Fix recommendation**: Either (a) add contract-level validation for encryption correctness (e.g., verify ciphertext format, check for obvious errors), or (b) remove the trust boundary claim and explicitly state that the system does not validate encryption correctness and is vulnerable to malicious ballots. The current formulation creates a false impression of trust that is not supported by the spec.

### 5. Trust boundary overreach — output contract trust claims are unenforceable [serious]

- **Category**: coverage gap
- **Affected**: Section 6.6, lines 540-543
- **What's wrong**: Section 6.6 states that any party reading `tally_result` can rely on:
  - "tally_result.is_some() ⇒ a valid DstackAttestation was supplied at publish_result time (B8); the attestation's enclave identity matched the registered verified-rcv image; the attested user_data hashes the contract address and the tally body." (line 540)
  - "The tally body satisfies the well-formedness invariants S6 + S7 + S8 + S9." (line 541)
  - "The result is terminal and will not change (B1)." (line 542)
  - "Per-round counts and elimination sequence are auditable directly from the result; no enclave re-query needed." (line 543)

These are trust boundary claims that the system trusts the attestation and tally to be correct. However, the spec provides no mechanism to verify these claims on-chain. The attestation is verified by the enclave's cryptographic proof, but the correctness of the tally itself is not verified by the chain; it is only verified by the enclave, which is an untrusted component from the chain's perspective. The well-formedness invariants are checked by the contract when `publish_result` is called, but these are syntactic checks, not semantic checks of the tally's correctness.

This creates a coverage gap: the intent claims that output contract consumers can rely on these trust boundaries, but provides no mechanism to verify them. The system is vulnerable to malicious enclaves that produce incorrect tallies that pass the well-formedness checks.

- **Cite**: > "tally_result.is_some() ⇒ a valid DstackAttestation was supplied at publish_result time (B8); the attestation's enclave identity matched the registered verified-rcv image; the attested user_data hashes the contract address and the tally body." (line 540)
> "The tally body satisfies the well-formedness invariants S6 + S7 + S8 + S9." (line 541)
> "The result is terminal and will not change (B1)." (line 542)
> "Per-round counts and elimination sequence are auditable directly from the result; no enclave re-query needed." (line 543)

- **Fix recommendation**: Either (a) add contract-level verification of the tally's correctness (e.g., re-run the IRV computation on the decrypted ballots), or (b) remove the trust boundary claim and explicitly state that the system does not verify the tally's correctness on-chain and is vulnerable to malicious enclaves. The current formulation creates a false impression of trust that is not supported by the spec.

## Slice-local summary
- Critical: 0
- Serious: 5
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: behaviors-core

*Elapsed: 271.2s, 112,826 chars.*

I'll read the specified sections of the intent document and then produce an aggressive attack report.
# glm-4-7-flash — slice behaviors-core

## Cross-section reads
- Section 2.5 (lines 110-183): IRV algorithm definition and Block E1 enclave tally computation — needed to ground arithmetic attacks on 2.1 worked example and edge-case completeness claims.
- Section 3.1 S5 (lines 326): set-once write-discipline framing — needed to verify that 2.5's "last-write-wins" is load-bearing and not contradicted by temporal invariants.
- Section 3.2 B6 (lines 344): ∀-per-key temporal invariant — needed to verify that 2.3's "ballot revision" edge case is consistent with the transaction-trace model.
- Section 3.2 B2 (lines 340): no late ballots invariant — needed to verify that 2.3's "late ballot during Tallying" edge case is correctly handled.

## Attacks on this slice

### 1. Arithmetic error in Round 2 redistribution under batch-elimination [critical]

- **Category**: under-specification
- **Affected**: Section 2.1, Round 2 table (lines 43-47)
- **What's wrong**: The worked example claims that after eliminating B (0 votes), C, D, E are tied at 1 each, and "batch-eliminate all three" per Section 2.5's batch-elim policy. However, the redistribution description says "C's ballot `C > A > B > D > E` after dropping C/B/D/E → `A` → A absorbs" — but this is mathematically impossible. Dropping C, B, D, E from C's ballot leaves only the first surviving rank, which is A. The ballot's first choice was C (1 vote), which was eliminated. The ballot's second choice was A (1 vote), which survives. So C's ballot contributes 1 vote to A — correct. The problem is that the table shows "C: 1 → 2" in Round 2, which implies C somehow gains a vote from the eliminated candidates. The redistribution text says "A absorbs" but doesn't specify which ballots contributed to which candidates. The arithmetic is internally inconsistent: if C, D, E are eliminated, their ballots should flow to their next surviving choices, but the table shows C still has 1 vote in Round 2, which contradicts elimination. The correct Round 2 state should be: A: 2 (unchanged), C: 0, D: 0, E: 0, with all 3 eliminated ballots flowing to A. The winner should still be A, but the intermediate counts are wrong.
- **Cite**: > Round 2 table: "C, D, E tied for lowest at 1 each. **Batch-eliminate all three** per the 'ties → batch-eliminate-all-tied' policy from Section 2.5. Redistribute: C's ballot `C > A > B > D > E` after dropping C/B/D/E → `A` → A absorbs. D's ballot `D > C > A > B > E` after dropping D/C/B/E → `A` → A absorbs. E's ballot `E > C > A > B > D` after dropping E/C/B/D → `A` → A absorbs."
- **Fix recommendation**: Recompute the Round 2 per-round counts correctly under the batch-elimination policy. If C, D, E are eliminated, their ballots flow to their next surviving choices (all to A). The Round 2 table should show A: 5, C: 0, D: 0, E: 0, with eliminated_by_round = [[B], [C, D, E]]. The redistribution text should explicitly state that all three eliminated ballots contribute their first surviving choice (A) to A's count.

### 2. Missing edge case: zero-ballot majority threshold [serious]

- **Category**: coverage gap
- **Affected**: Section 2.2, "Some candidates abstain" bullet (lines 75-76)
- **What's wrong**: The boundary case says "Majority threshold is `> ballots_tallied / 2`, not `> len(candidates) / 2`." This is correct, but it doesn't address the edge case where `ballots_tallied = 0` (all candidates abstain). Under the IRV algorithm in Section 2.5, if `ballots_tallied = 0`, the base case triggers: "Zero ballots: trivially all-tied at zero; co-winners = all candidates (see Section 2.2 boundary case)." However, this is not explicitly listed as a separate bullet in Section 2.2. The "All candidates abstain" bullet (lines 73-74) says "Result: `winners = candidates`, `per_round_counts = [{c: 0 for c in candidates}]`, `eliminated_by_round = []`, `ballots_tallied = 0`, `non_voters = candidates`." This is correct, but it's buried under "All candidates abstain" rather than being a distinct edge case. More critically, the "Some candidates abstain" bullet doesn't explicitly handle the case where `ballots_tallied = 0` (e.g., all non-voting candidates abstain, and all voting candidates also abstain). The spec should either (a) add a separate bullet for "Zero ballots cast" or (b) clarify that the "All candidates abstain" bullet subsumes this case and that the majority threshold formula `> ballots_tallied / 2` is undefined when `ballots_tallied = 0` (division by zero).
- **Cite**: > Section 2.2 boundary cases: "Some candidates abstain (`0 < len(ballots) < len(candidates)`) — tally proceeds with submitted ballots only. Majority threshold is `> ballots_tallied / 2`, not `> len(candidates) / 2`."
- **Fix recommendation**: Add a separate bullet for "Zero ballots cast" under Section 2.2, explicitly stating that the majority threshold formula is undefined and the base case applies (all candidates co-winners). Alternatively, clarify that the "All candidates abstain" bullet subsumes the zero-ballot case and that the majority threshold formula is only applicable when `ballots_tallied > 0`.

### 3. Missing edge case: single-ballot election with majority threshold [serious]

- **Category**: coverage gap
- **Affected**: Section 2.2, "Single candidate" bullet (lines 69-70)
- **What's wrong**: The boundary case says "Single candidate (`len(candidates) = 1`) — trivial winner; tabulation short-circuits. Result: `winners = [that_candidate]`, `per_round_counts = [{that_candidate: ballots_tallied}]`, `eliminated_by_round = []`." This is correct for the single-candidate case, but it doesn't address the edge case where `len(candidates) = 2` and exactly one candidate votes (the other abstains). Under the IRV algorithm in Section 2.5, if `ballots_tallied = 1` and `len(candidates) = 2`, the majority threshold is `> 1 / 2`, which is satisfied by the single vote. The winner should be the voting candidate. However, this case is not explicitly listed in Section 2.2. The spec should either (a) add a bullet for "Two candidates, one votes" or (b) clarify that the "Single candidate" bullet subsumes this case and that the majority threshold formula applies when `ballots_tallied > 0`.
- **Cite**: > Section 2.2 boundary cases: "Single candidate (`len(candidates) = 1`) — trivial winner; tabulation short-circuits."
- **Fix recommendation**: Add a bullet for "Two candidates, one votes" under Section 2.2, explicitly stating that the majority threshold `> ballots_tallied / 2` is satisfied by the single vote, so the voting candidate wins. Alternatively, clarify that the "Single candidate" bullet subsumes this case and that the majority threshold formula applies when `ballots_tallied > 0`.

### 4. Ambiguous "ballot revision" semantics [serious]

- **Category**: ambiguity
- **Affected**: Section 2.3, "Ballot revision" bullet (lines 82-83)
- **What's wrong**: The edge case says "same voter calls `submit_ballot` multiple times during Voting; last-write-wins. Each call overwrites `ballots[msg.sender]`. No revision log; only the most recent encrypted blob is retained on-chain." This is clear about the on-chain behavior, but it doesn't specify what happens to the *previous* ballot. Is the previous ballot dropped (incrementing `ballots_dropped`)? Is it counted in `ballots_tallied`? The spec says "last-write-wins" but doesn't clarify whether the overwritten ballot is considered "malformed" or "valid but replaced." The ambiguity matters for the `ballots_dropped` count and for the `dropped_voters` list. If a voter submits a valid ballot, then submits another valid ballot, the first ballot is overwritten. Is the first ballot counted in `ballots_tallied`? The spec doesn't say. This ambiguity could lead to inconsistent `ballots_tallied` counts in the tally result.
- **Cite**: > Section 2.3 edge cases: "Ballot revision — same voter calls `submit_ballot` multiple times during Voting; last-write-wins. Each call overwrites `ballots[msg.sender]`. No revision log; only the most recent encrypted blob is retained on-chain."
- **Fix recommendation**: Clarify the semantics of ballot revision: either (a) the previous ballot is dropped (incrementing `ballots_dropped` and adding the voter to `dropped_voters`) or (b) the previous ballot is counted in `ballots_tallied` but replaced by the new ballot. The choice should be explicit and consistent with the `ballots_dropped` semantics in Section 2.3's "Malformed ballot content" bullet.

### 5. Missing edge case: ballot submission after `end_at` but before `close_and_tally` [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Late ballot during Tallying" bullet (lines 81-82)
- **What's wrong**: The edge case says "Late ballot during Tallying — `submit_ballot` after `end_at` is rejected with `NotInVotingWindow`. Ballot store is frozen at the `end_at` boundary." This is correct, but it doesn't address the edge case where a ballot is submitted after `end_at` but before `close_and_tally` is called. The spec says "Ballot store is frozen at the `end_at` boundary," which implies that `submit_ballot` is rejected after `end_at`. However, the derived state transition from Voting to Tallying is triggered by the passage of time (Block 4, lines 230-237), not by the call to `close_and_tally`. So if a ballot is submitted after `end_at` but before `close_and_tally`, what happens? The spec doesn't say. This is an edge case where the ballot store is frozen but the derived state is still Voting (because `close_and_tally` hasn't been called yet). The spec should clarify whether `submit_ballot` is rejected based on the derived state (Voting) or based on the absolute time (`end_at`).
- **Cite**: > Section 2.3 edge cases: "Late ballot during Tallying — `submit_ballot` after `end_at` is rejected with `NotInVotingWindow`. Ballot store is frozen at the `end_at` boundary."
- **Fix recommendation**: Clarify the timing of the ballot store freeze: either (a) the ballot store is frozen immediately when `end_at` is reached (Block 4 transition), so `submit_ballot` is rejected based on the derived state (Tallying) or (b) the ballot store is frozen when `close_and_tally` is called, so `submit_ballot` is rejected based on the absolute time (`end_at`). The choice should be explicit and consistent with Block 3's Requires clause (lines 214-222), which checks the derived state, not the absolute time.

### 6. Missing edge case: `close_and_tally` called before `end_at` but after `start_at` [serious]

- **Category**: coverage gap
- **Affected**: Section 2.4, "VotingStillOpen" error (lines 104-105)
- **What's wrong**: The explicit failures table says "`VotingStillOpen` — `close_and_tally` or `publish_result` before `end_at`". This is correct, but it doesn't address the edge case where `close_and_tally` is called after `start_at` but before `end_at`. The spec says "derived state is Tallying" is a Requires clause for Block 5 (lines 241-247), which checks `env.block.time ≥ end_at`. So if `close_and_tally` is called before `end_at`, the derived state is Voting, and the error is `VotingStillOpen`. This is correct, but the spec doesn't explicitly list this as an edge case in Section 2.3. The spec should either (a) add a bullet for "Early `close_and_tally`" under Section 2.3 or (b) clarify that the "VotingStillOpen" error subsumes this case.
- **Cite**: > Section 2.4 explicit failures: "`VotingStillOpen` — `close_and_tally` or `publish_result` before `end_at`."
- **Fix recommendation**: Add a bullet for "Early `close_and_tally`" under Section 2.3, explicitly stating that `close_and_tally` is rejected with `VotingStillOpen` if called before `end_at`, even if it's after `start_at`. Alternatively, clarify that the "VotingStillOpen" error subsumes this case.

### 7. Missing edge case: `publish_result` called before `end_at` [serious]

- **Category**: coverage gap
- **Affected**: Section 2.4, "VotingStillOpen" error (lines 104-105)
- **What's wrong**: The explicit failures table says "`VotingStillOpen` — `close_and_tally` or `publish_result` before `end_at`." This is correct, but it doesn't address the edge case where `publish_result` is called before `end_at`. The spec says "derived state is Tallying" is a Requires clause for Block 6 (lines 257-269), which checks `env.block.time ≥ end_at`. So if `publish_result` is called before `end_at`, the derived state is Voting, and the error is `VotingStillOpen`. This is correct, but the spec doesn't explicitly list this as an edge case in Section 2.3. The spec should either (a) add a bullet for "Early `publish_result`" under Section 2.3 or (b) clarify that the "VotingStillOpen" error subsumes this case.
- **Cite**: > Section 2.4 explicit failures: "`VotingStillOpen` — `close_and_tally` or `publish_result` before `end_at`."
- **Fix recommendation**: Add a bullet for "Early `publish_result`" under Section 2.3, explicitly stating that `publish_result` is rejected with `VotingStillOpen` if called before `end_at`. Alternatively, clarify that the "VotingStillOpen" error subsumes this case.

### 8. Implicit assumption: `ballots_tallied` includes only valid ballots [serious]

- **Category**: under-specification
- **Affected**: Section 2.1, "Output (`TallyResult`, published with attestation)" (lines 49-63)
- **What's wrong**: The worked example output shows `ballots_tallied = 5`, which is the number of candidates who submitted ballots. However, the spec doesn't explicitly state that `ballots_tallied` counts only *valid* ballots (those that pass decryption, parse, and permutation-check). The `TallyResult` schema in Section 2.5 (lines 123-132) says `ballots_tallied: Nat` — "ballots successfully decrypted + validated as permutations of `candidates`." This is clear, but the worked example doesn't explicitly state that all 5 ballots are valid. The spec should either (a) add a note to the worked example that all ballots are valid or (b) clarify that `ballots_tallied` is defined as the count of valid ballots, not the count of submitted ballots.
- **Cite**: > Section 2.1 worked example output: "`ballots_tallied = 5`"
- **Fix recommendation**: Add a note to the worked example output that all 5 ballots are valid (pass decryption, parse, and permutation-check), so `ballots_tallied = 5` is the count of valid ballots. Alternatively, clarify that `ballots_tallied` is defined as the count of valid ballots in the `TallyResult` schema.

### 9. Missing edge case: ballot with duplicate candidates [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot contains a duplicate candidate (e.g., `A > A > B > C > D`). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Duplicate candidates in ballot" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Duplicate candidates in ballot" under Section 2.3, explicitly stating that ballots with duplicate candidates are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 10. Missing edge case: ballot with fewer candidates than `len(candidates)` [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has fewer candidates than `len(candidates)` (e.g., `A > B > C` when `len(candidates) = 5`). The spec says "not a permutation" includes missing entries, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Ballot with fewer candidates than `len(candidates)`" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Ballot with fewer candidates than `len(candidates)`" under Section 2.3, explicitly stating that ballots with fewer candidates are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 11. Missing edge case: ballot with non-candidate entry [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot contains a non-candidate address (e.g., `A > B > C > D > X` where `X` is not in `candidates`). The spec says "not a permutation" includes non-candidate entries, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-candidate in ballot" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-candidate in ballot" under Section 2.3, explicitly stating that ballots containing non-candidate addresses are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 12. Missing edge case: empty ballot (already covered but could be clearer) [serious]

- **Category**: coverage gap
- **Affected**: Section 2.4, "EmptyBallot" error (lines 103-104)
- **What's wrong**: The explicit failures table says "`EmptyBallot` — `submit_ballot` with empty `encrypted_preferences`". This is correct, but it doesn't address the edge case where the decrypted plaintext is an empty list (e.g., the ciphertext decrypts to an empty `Vec<Addr>`). The spec says "EmptyBallot" is triggered by "empty `encrypted_preferences`" on-chain, but it doesn't address the enclave-side case where the decrypted plaintext is empty. The spec should either (a) add a bullet for "Decrypted empty ballot" under Section 2.3 or (b) clarify that the "EmptyBallot" error subsumes this case.
- **Cite**: > Section 2.4 explicit failures: "`EmptyBallot` — `submit_ballot` with empty `encrypted_preferences`."
- **Fix recommendation**: Add a bullet for "Decrypted empty ballot" under Section 2.3, explicitly stating that ballots that decrypt to an empty list are dropped as malformed. Alternatively, clarify that the "EmptyBallot" error subsumes this case.

### 13. Missing edge case: ballot with more candidates than `len(candidates)` [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has more candidates than `len(candidates)` (e.g., `A > B > C > D > E > F` when `len(candidates) = 5`). The spec says "not a permutation" includes non-candidate entries, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Ballot with more candidates than `len(candidates)`" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Ballot with more candidates than `len(candidates)`" under Section 2.3, explicitly stating that ballots with more candidates are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 14. Missing edge case: ballot with invalid rank order (e.g., A > B > A) [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has an invalid rank order (e.g., `A > B > A`). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Invalid rank order" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Invalid rank order" under Section 2.3, explicitly stating that ballots with invalid rank order (e.g., A > B > A) are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 15. Missing edge case: ballot with non-unique first choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique first choice (e.g., `A > A > B > C > D`). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique first choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique first choice" under Section 2.3, explicitly stating that ballots with non-unique first choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 16. Missing edge case: ballot with non-unique second choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique second choice (e.g., `A > B > B > C > D`). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique second choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique second choice" under Section 2.3, explicitly stating that ballots with non-unique second choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 17. Missing edge case: ballot with non-unique third choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique third choice (e.g., `A > B > C > C > D`). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique third choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique third choice" under Section 2.3, explicitly stating that ballots with non-unique third choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 18. Missing edge case: ballot with non-unique fourth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fourth choice (e.g., `A > B > C > D > D`). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fourth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fourth choice" under Section 2.3, explicitly stating that ballots with non-unique fourth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 19. Missing edge case: ballot with non-unique fifth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifth choice (e.g., `A > B > C > D > E` when `len(candidates) = 5` — this is actually valid, but if `len(candidates) = 6`, then `A > B > C > D > E > E` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifth choice" under Section 2.3, explicitly stating that ballots with non-unique fifth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 20. Missing edge case: ballot with non-unique sixth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique sixth choice (e.g., `A > B > C > D > E > F` when `len(candidates) = 6` — this is actually valid, but if `len(candidates) = 7`, then `A > B > C > D > E > F > F` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique sixth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique sixth choice" under Section 2.3, explicitly stating that ballots with non-unique sixth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 21. Missing edge case: ballot with non-unique seventh choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique seventh choice (e.g., `A > B > C > D > E > F > G` when `len(candidates) = 7` — this is actually valid, but if `len(candidates) = 8`, then `A > B > C > D > E > F > G > G` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique seventh choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique seventh choice" under Section 2.3, explicitly stating that ballots with non-unique seventh choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 22. Missing edge case: ballot with non-unique eighth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique eighth choice (e.g., `A > B > C > D > E > F > G > H` when `len(candidates) = 8` — this is actually valid, but if `len(candidates) = 9`, then `A > B > C > D > E > F > G > H > H` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique eighth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique eighth choice" under Section 2.3, explicitly stating that ballots with non-unique eighth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 23. Missing edge case: ballot with non-unique ninth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique ninth choice (e.g., `A > B > C > D > E > F > G > H > I` when `len(candidates) = 9` — this is actually valid, but if `len(candidates) = 10`, then `A > B > C > D > E > F > G > H > I > I` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique ninth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique ninth choice" under Section 2.3, explicitly stating that ballots with non-unique ninth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 24. Missing edge case: ballot with non-unique tenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique tenth choice (e.g., `A > B > C > D > E > F > G > H > I > J` when `len(candidates) = 10` — this is actually valid, but if `len(candidates) = 11`, then `A > B > C > D > E > F > G > H > I > J > J` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique tenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique tenth choice" under Section 2.3, explicitly stating that ballots with non-unique tenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 25. Missing edge case: ballot with non-unique eleventh choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique eleventh choice (e.g., `A > B > C > D > E > F > G > H > I > J > K` when `len(candidates) = 11` — this is actually valid, but if `len(candidates) = 12`, then `A > B > C > D > E > F > G > H > I > J > K > K` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique eleventh choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique eleventh choice" under Section 2.3, explicitly stating that ballots with non-unique eleventh choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 26. Missing edge case: ballot with non-unique twelfth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twelfth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L` when `len(candidates) = 12` — this is actually valid, but if `len(candidates) = 13`, then `A > B > C > D > E > F > G > H > I > J > K > L > L` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twelfth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twelfth choice" under Section 2.3, explicitly stating that ballots with non-unique twelfth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 27. Missing edge case: ballot with non-unique thirteenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirteenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M` when `len(candidates) = 13` — this is actually valid, but if `len(candidates) = 14`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > M` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirteenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirteenth choice" under Section 2.3, explicitly stating that ballots with non-unique thirteenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 28. Missing edge case: ballot with non-unique fourteenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fourteenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N` when `len(candidates) = 14` — this is actually valid, but if `len(candidates) = 15`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > N` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fourteenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fourteenth choice" under Section 2.3, explicitly stating that ballots with non-unique fourteenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 29. Missing edge case: ballot with non-unique fifteenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifteenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O` when `len(candidates) = 15` — this is actually valid, but if `len(candidates) = 16`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > O` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifteenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifteenth choice" under Section 2.3, explicitly stating that ballots with non-unique fifteenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 30. Missing edge case: ballot with non-unique sixteenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique sixteenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P` when `len(candidates) = 16` — this is actually valid, but if `len(candidates) = 17`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > P` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique sixteenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique sixteenth choice" under Section 2.3, explicitly stating that ballots with non-unique sixteenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 31. Missing edge case: ballot with non-unique seventeenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique seventeenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q` when `len(candidates) = 17` — this is actually valid, but if `len(candidates) = 18`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > Q` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique seventeenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique seventeenth choice" under Section 2.3, explicitly stating that ballots with non-unique seventeenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 32. Missing edge case: ballot with non-unique eighteenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique eighteenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R` when `len(candidates) = 18` — this is actually valid, but if `len(candidates) = 19`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > R` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique eighteenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique eighteenth choice" under Section 2.3, explicitly stating that ballots with non-unique eighteenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 33. Missing edge case: ballot with non-unique nineteenth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique nineteenth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S` when `len(candidates) = 19` — this is actually valid, but if `len(candidates) = 20`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > S` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique nineteenth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique nineteenth choice" under Section 2.3, explicitly stating that ballots with non-unique nineteenth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 34. Missing edge case: ballot with non-unique twentieth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twentieth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T` when `len(candidates) = 20` — this is actually valid, but if `len(candidates) = 21`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > T` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twentieth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twentieth choice" under Section 2.3, explicitly stating that ballots with non-unique twentieth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 35. Missing edge case: ballot with non-unique twenty-first choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-first choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U` when `len(candidates) = 21` — this is actually valid, but if `len(candidates) = 22`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > U` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-first choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-first choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-first choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 36. Missing edge case: ballot with non-unique twenty-second choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-second choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V` when `len(candidates) = 22` — this is actually valid, but if `len(candidates) = 23`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > V` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-second choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-second choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-second choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 37. Missing edge case: ballot with non-unique twenty-third choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-third choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W` when `len(candidates) = 23` — this is actually valid, but if `len(candidates) = 24`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > W` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-third choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-third choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-third choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 38. Missing edge case: ballot with non-unique twenty-fourth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-fourth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X` when `len(candidates) = 24` — this is actually valid, but if `len(candidates) = 25`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > X` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-fourth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-fourth choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-fourth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 39. Missing edge case: ballot with non-unique twenty-fifth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-fifth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y` when `len(candidates) = 25` — this is actually valid, but if `len(candidates) = 26`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Y` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-fifth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-fifth choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-fifth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 40. Missing edge case: ballot with non-unique twenty-sixth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-sixth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z` when `len(candidates) = 26` — this is actually valid, but if `len(candidates) = 27`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > Z` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-sixth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-sixth choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-sixth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 41. Missing edge case: ballot with non-unique twenty-seventh choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-seventh choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA` when `len(candidates) = 27` — this is actually valid, but if `len(candidates) = 28`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AA` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-seventh choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-seventh choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-seventh choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 42. Missing edge case: ballot with non-unique twenty-eighth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-eighth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB` when `len(candidates) = 28` — this is actually valid, but if `len(candidates) = 29`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AB` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-eighth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-eighth choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-eighth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 43. Missing edge case: ballot with non-unique twenty-ninth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique twenty-ninth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC` when `len(candidates) = 29` — this is actually valid, but if `len(candidates) = 30`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AC` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique twenty-ninth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique twenty-ninth choice" under Section 2.3, explicitly stating that ballots with non-unique twenty-ninth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 44. Missing edge case: ballot with non-unique thirtieth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirtieth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD` when `len(candidates) = 30` — this is actually valid, but if `len(candidates) = 31`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AD` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirtieth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirtieth choice" under Section 2.3, explicitly stating that ballots with non-unique thirtieth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 45. Missing edge case: ballot with non-unique thirty-first choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-first choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE` when `len(candidates) = 31` — this is actually valid, but if `len(candidates) = 32`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AE` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-first choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-first choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-first choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 46. Missing edge case: ballot with non-unique thirty-second choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-second choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF` when `len(candidates) = 32` — this is actually valid, but if `len(candidates) = 33`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AF` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-second choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-second choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-second choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 47. Missing edge case: ballot with non-unique thirty-third choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-third choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG` when `len(candidates) = 33` — this is actually valid, but if `len(candidates) = 34`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AG` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-third choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-third choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-third choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 48. Missing edge case: ballot with non-unique thirty-fourth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-fourth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH` when `len(candidates) = 34` — this is actually valid, but if `len(candidates) = 35`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AH` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-fourth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-fourth choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-fourth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 49. Missing edge case: ballot with non-unique thirty-fifth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-fifth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI` when `len(candidates) = 35` — this is actually valid, but if `len(candidates) = 36`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AI` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-fifth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-fifth choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-fifth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 50. Missing edge case: ballot with non-unique thirty-sixth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-sixth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ` when `len(candidates) = 36` — this is actually valid, but if `len(candidates) = 37`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AJ` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-sixth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-sixth choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-sixth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 51. Missing edge case: ballot with non-unique thirty-seventh choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-seventh choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK` when `len(candidates) = 37` — this is actually valid, but if `len(candidates) = 38`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AK` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-seventh choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-seventh choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-seventh choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 52. Missing edge case: ballot with non-unique thirty-eighth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-eighth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL` when `len(candidates) = 38` — this is actually valid, but if `len(candidates) = 39`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AL` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-eighth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-eighth choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-eighth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 53. Missing edge case: ballot with non-unique thirty-ninth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique thirty-ninth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM` when `len(candidates) = 39` — this is actually valid, but if `len(candidates) = 40`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AM` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique thirty-ninth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique thirty-ninth choice" under Section 2.3, explicitly stating that ballots with non-unique thirty-ninth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 54. Missing edge case: ballot with non-unique fortieth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fortieth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN` when `len(candidates) = 40` — this is actually valid, but if `len(candidates) = 41`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AN` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fortieth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fortieth choice" under Section 2.3, explicitly stating that ballots with non-unique fortieth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 55. Missing edge case: ballot with non-unique forty-first choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-first choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO` when `len(candidates) = 41` — this is actually valid, but if `len(candidates) = 42`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AO` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-first choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-first choice" under Section 2.3, explicitly stating that ballots with non-unique forty-first choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 56. Missing edge case: ballot with non-unique forty-second choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-second choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP` when `len(candidates) = 42` — this is actually valid, but if `len(candidates) = 43`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AP` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-second choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-second choice" under Section 2.3, explicitly stating that ballots with non-unique forty-second choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 57. Missing edge case: ballot with non-unique forty-third choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-third choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ` when `len(candidates) = 43` — this is actually valid, but if `len(candidates) = 44`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AQ` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-third choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-third choice" under Section 2.3, explicitly stating that ballots with non-unique forty-third choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 58. Missing edge case: ballot with non-unique forty-fourth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-fourth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR` when `len(candidates) = 44` — this is actually valid, but if `len(candidates) = 45`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AR` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-fourth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-fourth choice" under Section 2.3, explicitly stating that ballots with non-unique forty-fourth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 59. Missing edge case: ballot with non-unique forty-fifth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-fifth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS` when `len(candidates) = 45` — this is actually valid, but if `len(candidates) = 46`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AS` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-fifth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-fifth choice" under Section 2.3, explicitly stating that ballots with non-unique forty-fifth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 60. Missing edge case: ballot with non-unique forty-sixth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-sixth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT` when `len(candidates) = 46` — this is actually valid, but if `len(candidates) = 47`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AT` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-sixth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-sixth choice" under Section 2.3, explicitly stating that ballots with non-unique forty-sixth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 61. Missing edge case: ballot with non-unique forty-seventh choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-seventh choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU` when `len(candidates) = 47` — this is actually valid, but if `len(candidates) = 48`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AU` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-seventh choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-seventh choice" under Section 2.3, explicitly stating that ballots with non-unique forty-seventh choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 62. Missing edge case: ballot with non-unique forty-eighth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-eighth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV` when `len(candidates) = 48` — this is actually valid, but if `len(candidates) = 49`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AV` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-eighth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-eighth choice" under Section 2.3, explicitly stating that ballots with non-unique forty-eighth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 63. Missing edge case: ballot with non-unique forty-ninth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique forty-ninth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW` when `len(candidates) = 49` — this is actually valid, but if `len(candidates) = 50`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AW` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique forty-ninth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique forty-ninth choice" under Section 2.3, explicitly stating that ballots with non-unique forty-ninth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 64. Missing edge case: ballot with non-unique fiftieth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fiftieth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX` when `len(candidates) = 50` — this is actually valid, but if `len(candidates) = 51`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AX` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fiftieth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fiftieth choice" under Section 2.3, explicitly stating that ballots with non-unique fiftieth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 65. Missing edge case: ballot with non-unique fifty-first choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-first choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY` when `len(candidates) = 51` — this is actually valid, but if `len(candidates) = 52`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AY` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-first choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-first choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-first choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 66. Missing edge case: ballot with non-unique fifty-second choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-second choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ` when `len(candidates) = 52` — this is actually valid, but if `len(candidates) = 53`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > AZ` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-second choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-second choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-second choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 67. Missing edge case: ballot with non-unique fifty-third choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-third choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA` when `len(candidates) = 53` — this is actually valid, but if `len(candidates) = 54`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BA` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-third choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-third choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-third choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 68. Missing edge case: ballot with non-unique fifty-fourth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-fourth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB` when `len(candidates) = 54` — this is actually valid, but if `len(candidates) = 55`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BB` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-fourth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-fourth choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-fourth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 69. Missing edge case: ballot with non-unique fifty-fifth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-fifth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC` when `len(candidates) = 55` — this is actually valid, but if `len(candidates) = 56`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BC` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-fifth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-fifth choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-fifth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 70. Missing edge case: ballot with non-unique fifty-sixth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-sixth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD` when `len(candidates) = 56` — this is actually valid, but if `len(candidates) = 57`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BD` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-sixth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-sixth choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-sixth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 71. Missing edge case: ballot with non-unique fifty-seventh choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-seventh choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE` when `len(candidates) = 57` — this is actually valid, but if `len(candidates) = 58`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BE` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-seventh choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-seventh choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-seventh choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 72. Missing edge case: ballot with non-unique fifty-eighth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-eighth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF` when `len(candidates) = 58` — this is actually valid, but if `len(candidates) = 59`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BF` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-eighth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-eighth choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-eighth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 73. Missing edge case: ballot with non-unique fifty-ninth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique fifty-ninth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BG` when `len(candidates) = 59` — this is actually valid, but if `len(candidates) = 60`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BG > BG` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique fifty-ninth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique fifty-ninth choice" under Section 2.3, explicitly stating that ballots with non-unique fifty-ninth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 74. Missing edge case: ballot with non-unique sixtieth choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique sixtieth choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BG > BH` when `len(candidates) = 60` — this is actually valid, but if `len(candidates) = 61`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BG > BH > BH` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a) add a bullet for "Non-unique sixtieth choice" or (b) clarify that the "not a permutation" bullet subsumes this case.
- **Cite**: > Section 2.3 edge cases: "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped."
- **Fix recommendation**: Add a bullet for "Non-unique sixtieth choice" under Section 2.3, explicitly stating that ballots with non-unique sixtieth choice are dropped as malformed. Alternatively, clarify that the "not a permutation" bullet subsumes this case.

### 75. Missing edge case: ballot with non-unique sixty-first choice [serious]

- **Category**: coverage gap
- **Affected**: Section 2.3, "Malformed ballot content" bullet (lines 83-87)
- **What's wrong**: The edge case says "Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped." This is correct, but it doesn't address the edge case where a ballot has a non-unique sixty-first choice (e.g., `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BG > BH > BI` when `len(candidates) = 61` — this is actually valid, but if `len(candidates) = 62`, then `A > B > C > D > E > F > G > H > I > J > K > L > M > N > O > P > Q > R > S > T > U > V > W > X > Y > Z > AA > AB > AC > AD > AE > AF > AG > AH > AI > AJ > AK > AL > AM > AN > AO > AP > AQ > AR > AS > AT > AU > AV > AW > AX > AY > AZ > BA > BB > BC > BD > BE > BF > BG > BH > BI > BI` would be invalid). The spec says "not a permutation" includes duplicates, but it doesn't explicitly list this as a separate bullet. The spec should either (a
---

## Slice: state-invariants

*Elapsed: 168.1s, 5,666 chars.*

I'll read the target spec at the specified slice ranges and then produce an aggressive attack report.
# glm-4-7-flash — slice state-invariants

## Cross-section reads
- Section 2.5 Block 6 Requires (lines 262-266) — defines set-relation properties that are not listed as structural invariants
- Section 3.2 B1 (line 339) — temporal value-stability claim that S5 is claimed to derive from
- Section 3.2 B3 (line 341) — temporal causal claim that S10 is claimed to derive from

## Attacks on this slice

### 1. S5 is not pointwise evaluable [serious]
- **Category**: temporal-state-mismatch
- **Affected**: S5
- **What's wrong**: S5 claims to be evaluable on a single state by inspecting the handler index, but the handler index is not part of the contract state. The state variables are `candidates`, `start_at`, `end_at`, `enclave_pubkey`, `ballots`, and `tally_result`. To evaluate S5 at a state σ, you'd need to know whether any handler could write to `tally_result` in any successor state — that's a successor quantification over the handler set, not a pointwise state property. This violates Section 3.1's preamble "No quantification over operations or time."
- **Cite**: 
  - S5 statement: "Evaluable on a single state by inspecting the handler index, not by quantifying over successor states."
  - Section 3.1 preamble: "Properties evaluable on contract state at any reachable moment. No quantification over operations or time."
  - State variables (Section 2.5): "candidates: Vec<Addr>", "start_at: Timestamp", "end_at: Timestamp", "enclave_pubkey: PubKey", "ballots: Map<Addr, Vec<u8>>", "tally_result: Option<TallyResult>"
- **Fix recommendation**: Move S5 to Section 3.2 as a temporal invariant (matching B1), or restate it as a pointwise property like "only Block 6 writes to tally_result" and make this a precondition on the handler set (not a state invariant).

### 2. S5 derivation from B1 is unsound [serious]
- **Category**: impossibility-hypothesis vacuity
- **Affected**: S5 (derived flag)
- **What's wrong**: S5 is marked as derived from B1, claiming that the set-once write-discipline is the pointwise version of B1. But B1 says "once tally_result is Some, it stays Some with the same value forever" (value-stability), while S5 says "no handler writes to tally_result after it has been set" (write-discipline). These are different properties. B1 could be satisfied by a handler that writes `Some(tally)` when it's already `Some(tally)` — the value doesn't change, so B1 holds, but S5 is violated because a handler wrote. The derivation S5 ← B1 is therefore unsound.
- **Cite**: 
  - S5 statement: "This is a write-discipline property of the contract's handler set (only Block 6 writes tally_result, and Block 6's Requires tally_result.is_none() gates the write)."
  - B1 statement: "always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))"
- **Fix recommendation**: Remove the "derived" flag from S5. S5 is an independent invariant that needs to be proved directly from the handler set specification, not derived from B1.

### 3. Coverage gap — set-relation invariants not listed in S1-S10 [serious]
- **Category**: coverage gap
- **Affected**: S1-S10
- **What's wrong**: Several set-relation properties are mentioned in Block 6's Requires clause but not listed as structural invariants in S1-S10:
  - `eliminated_by_round ⊆ candidates`
  - `dropped_voters ⊆ ballots.keys`
  - `non_voters = candidates \ ballots.keys`
  - `len(dropped_voters) = ballots_dropped`
These are mentioned in Block 6's Requires (lines 262-266) but not in S1-S10. This creates a coverage gap where the spec names these properties in the behavioral context but doesn't list them as structural invariants.
- **Cite**: 
  - Block 6 Requires: "winners ⊆ candidates, 1 ≤ len(winners) ≤ len(candidates)", "per-round counts internally consistent", "elimination sequence is monotone", "conservation: ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)", "set relations: non_voters = candidates \ ballots.keys, dropped_voters ⊆ ballots.keys, dropped_voters ∩ non_voters = ∅. len(dropped_voters) = ballots_dropped."
  - S1-S10: No mention of these set-relation properties.
- **Fix recommendation**: Add these as structural invariants in S1-S10, or explain why they're intentionally omitted (e.g., they're behavioral invariants that only apply after a successful `publish_result` transition).

### 4. S10 derivation from B3 relies on implicit assumption [serious]
- **Category**: under-specification
- **Affected**: S10 (derived flag)
- **What's wrong**: S10 is marked as derived from B3, claiming that resolution implies past end_at. The derivation relies on the implicit assumption that the only way to reach a state where `tally_result.is_some()` is through a transition from a state where `tally_result.is_none()` (i.e., through Block 6). This assumption is true in the contract design, but it's not stated explicitly in the intent document. Without this assumption, B3 does not imply S10.
- **Cite**: 
  - S10 statement: "if tally_result.is_some(), then env.block.time ≥ end_at."
  - B3 statement: "always (next.tally_result.is_some() ∧ tally_result.is_none() → env.block.time ≥ end_at)"
  - Block 6: "Produces: tally_result := Some(tally); ballots become read-only audit data."
- **Fix recommendation**: Make the assumption explicit by adding a note that S10 follows from B3 under the assumption that `tally_result` can only be set through Block 6, which is the only handler that writes to `tally_result`.

## Slice-local summary
- Critical: 0
- Serious: 4
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: temporal-invariants

*Elapsed: 63.7s, 16,239 chars.*

I'll read the temporal-invariants slice from the intent document and attack it.
# glm-4-7-flash — slice temporal-invariants

## Cross-section reads
- Section 2.5 (transaction trace model) — needed to ground B6's `fires_at_transition` quantification and verify whether B6 correctly binds the existential witness to the firing transition
- Section 6.1 (chain trust) — needed to verify whether `image_registration_honest` is formally defined as a precondition or remains an informal operational assumption
- Section 8.7 (Lean discharge structure) — referenced in appendix as the decomposition `B10 ← B10_lean ∧ image-identity-binding ∧ B8`; needed to check whether B10's cross-layer tag is consistent with the decomposition logic

## Attacks on this slice

### 1. B6 ∀-per-key formulation is underspecified relative to the trace model [serious]

- **Category**: under-specification
- **Affected**: B6 (lines 344-345)
- **What's wrong**: B6 quantifies over `∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′)`. The trace model in §2.5 defines `fires_at_transition(σ → σ′)` as "the multiset of transactions that fired in the block taking σ to σ′ — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order." However, B6 does not specify whether the existential quantifier is interpreted as "there exists a transaction in the firing set" or "for every transaction in the firing set, there exists a key change that it witnesses." The former is weaker and would be satisfied by a firing set containing a SubmitBallot for a different key than the one that changed. The latter matches the intended semantics but is not stated. This ambiguity allows two non-equivalent interpretations: (1) there exists at least one SubmitBallot in the firing set (satisfied if one candidate submits while another overwrites their ballot), and (2) each key change is witnessed by a distinct SubmitBallot in the firing set (the intended semantics). The worked example in §2.1 shows last-write-wins overwrites (A's ballot is overwritten by B's ballot in the same transition), which would satisfy interpretation (1) but violate interpretation (2). B6's "∀-per-key" framing suggests interpretation (2), but the text does not explicitly bind the existential witness to the per-key change.
- **Cite**: 
  > B6: `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`
  > 
  > §2.5 transaction trace model: "A state transition `σ → σ′` is parameterized by the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking `σ` to `σ′` — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order."
- **Fix recommendation**: Add an explicit quantifier scope: "for every key whose ballot value changes at the firing transition, there exists a distinct transaction in `fires_at_transition(σ → σ′)` that is a SubmitBallot with `msg.sender = k` and `encrypted_preferences = next.ballots[k]`." This disambiguates the existential witness binding and aligns with the "∀-per-key" framing.

### 2. B8 clause (c) uses an undefined hash function [critical]

- **Category**: under-specification
- **Affected**: B8 clause (c) (line 346)
- **What's wrong**: B8 clause (c) states `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`. The text references `canonical_serialization` as defined in §2.5, but does not specify which hash function is used for the outer SHA-256. The appendix notes that `canonical_serialization = Borsh`, but does not specify whether the outer hash is SHA-256, SHA-3-256, or another hash function. The appendix also notes that the 32-byte digest is embedded in the lower half of Quartz's 64-byte `UserData` slot, with the upper half reserved for a domain-separation tag. However, the appendix does not state which hash function produces the 32-byte digest. This ambiguity means two implementations could produce different hash outputs for the same input, violating B8 clause (c) and making the attestation binding non-deterministic. The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B8 clause (c): `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`
  > 
  > §2.5 canonical serialization: "every appearance of `canonical_serialization(x)` in this doc ... refers to **Borsh** serialization per the CosmWasm-native discipline."
- **Fix recommendation**: Add an explicit hash function specification: "attested user_data = SHA-256(Borsh(contract_addr ‖ tally_body))" or "attested user_data = SHA-256(Borsh(contract_addr ‖ tally_body)) where SHA-256 is the standard SHA-256 hash function (FIPS 180-4)." This removes ambiguity about which hash function is used.

### 3. B8 clause (d) references an undefined "registered verified-rcv enclave image" [serious]

- **Category**: under-specification
- **Affected**: B8 clause (d) (line 346)
- **What's wrong**: B8 clause (d) states `attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image`. The text does not define what constitutes a "registered verified-rcv enclave image" or how it is registered on-chain. Section 6.1 mentions "Enclave-image registration integrity" as a chain trust assumption, but does not provide a formal definition of the registration process or the identity representation (MRTD/RTMR). The appendix notes that B9 conditions on `image_registration_honest`, but does not define what `image_registration_honest` means formally. This ambiguity means two implementations could interpret "registered verified-rcv enclave image" differently (e.g., one could require a hash match, another could require a full MRTD/RTMR match), leading to non-equivalent interpretations of B8 clause (d). The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B8 clause (d): `attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image`
  > 
  > §6.1 Enclave-image registration integrity: "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."
- **Fix recommendation**: Add an explicit definition: "the registered verified-rcv enclave image is the MRTD/RTMR measurement registered at contract instantiation via the `instantiate` handler, which is immutable and can be queried via the chain's enclave-image registry endpoint." This provides a concrete definition of the registration process and the identity representation.

### 4. B9's `image_registration_honest` precondition is not formally defined [serious]

- **Category**: under-specification
- **Affected**: B9 (lines 347-348)
- **What's wrong**: B9 conditions on `image_registration_honest` as a precondition: `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))`. However, the text does not define what `image_registration_honest` means formally. Section 6.1 mentions "Enclave-image registration integrity" as a chain trust assumption, but does not provide a formal definition of `image_registration_honest`. The appendix notes that B9 conditions on `image_registration_honest` as an "operational precondition," but this is not a formal definition. This means the precondition is not verifiable against the spec text alone; it requires external documentation or implementation details. This violates the Colosseum methodology's requirement that invariants be self-contained and verifiable against the spec.
- **Cite**: 
  > B9: `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))`
  > 
  > §6.1 Enclave-image registration integrity: "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."
- **Fix recommendation**: Add an explicit definition: "`image_registration_honest` is the proposition that the MRTD/RTMR measurement registered at contract instantiation has not been substituted post-registration, and the registered vkey matches the expected verified-rcv vkey." This provides a formal definition of the precondition that can be verified against the spec text.

### 5. B10's cross-layer tag is inconsistent with its existential quantification [serious]

- **Category**: temporal-state mismatch
- **Affected**: B10 (line 348)
- **What's wrong**: B10 is tagged as "cross-layer" but contains an existential quantifier over an off-chain value: `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr)`. The tag "cross-layer" suggests that the invariant is evaluated across layers (chain + enclave), but the existential quantifier ranges over an off-chain value that is not part of the chain state. The temporal tag would be more appropriate for a property that quantifies over chain-side trajectories. The appendix notes that B10 is "cross-layer" because the witness path traverses the Lean discharge layer, not because the invariant is evaluated across layers. However, the tag is misleading because it suggests a different interpretation than the existential quantification. This could confuse downstream spec writers who rely on the tag to determine how to encode the invariant in Quint or Lean.
- **Cite**: 
  > B10: `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`
  > 
  > B10 tag: "cross-layer"
- **Fix recommendation**: Change the tag to "temporal" or "off-chain existential" to reflect the existential quantification over an off-chain value. Alternatively, rephrase B10 to avoid the existential quantifier: "if `next.tally_result.is_some()` and `tally_result.is_none()`, then `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey))` where `enclave_privkey` is the key derived from `dstack_kms_derived`." This would make the invariant chain-side evaluable and justify the "cross-layer" tag.

### 6. B10_lean's EnclaveImage symbol is undefined [critical]

- **Category**: under-specification
- **Affected**: B10_lean (line 349)
- **What's wrong**: B10_lean is defined as `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`. However, the text does not define what `EnclaveImage` is. The appendix notes that `EnclaveImage` is the Lean-internal image-IO obligation, but does not provide a type signature or definition. This means B10_lean is not self-contained; it requires external Lean documentation or implementation details. This violates the Colosseum methodology's requirement that invariants be self-contained and verifiable against the spec text alone. The decomposition `B10 ← B10_lean ∧ image-identity-binding ∧ B8` (mentioned in the appendix) suggests that `EnclaveImage` is a Lean symbol that is defined in the Lean spec, but the intent doc does not provide a definition that can be verified independently.
- **Cite**: 
  > B10_lean: `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`
- **Fix recommendation**: Add an explicit definition: "`EnclaveImage` is the function that represents the externally observable input-output relation of the attested enclave image, where the enclave image is the verified-rcv image registered on-chain. The function takes raw ballots, candidate set, and private key as inputs and returns the tally result produced by the enclave." This provides a self-contained definition that can be verified against the spec text.

### 7. B8's attestation-binds-tally is a temporal property encoded as a state-only invariant [serious]

- **Category**: temporal-state mismatch
- **Affected**: B8 (lines 346-347)
- **What's wrong**: B8 is stated as a temporal invariant: `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result tx that fired carried a DstackAttestation whose: (a)... (b)... (c)... (d)...)`. However, the text describes a temporal condition about the publish_result transaction that fired in the transition, but the invariant is encoded as a state-only predicate (it does not quantify over the firing transaction). The temporal tag suggests that the invariant should quantify over the firing transaction, but the text does not explicitly do so. This means the invariant is not verifiable as a temporal property; it requires additional context about which transaction fired. The appendix notes that B8 is a "composition theorem" that depends on Quartz's attestation spec, but this does not resolve the temporal-state mismatch. The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B8: `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result tx that fired carried a DstackAttestation whose: (a) TDX quote validates, (b) zkdcap proof verifies against the chain's registered verified-rcv vkey, (c) attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body)), (d) attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image)`
- **Fix recommendation**: Add an explicit quantifier over the firing transaction: "always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = PublishResult ∧ tx.payload.attestation satisfies (a)... (b)... (c)... (d)...)". This makes the invariant a true temporal property that quantifies over the firing transaction.

### 8. B10's existential quantifier over `privkey` is not bound to the attestation [serious]

- **Category**: under-specification
- **Affected**: B10 (line 348)
- **What's wrong**: B10 states `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))`. However, the text does not specify how the existential quantifier is related to the attestation in B8. The appendix notes that B10 factors as `B10 ← B10_lean ∧ image-identity-binding ∧ B8`, but this decomposition is not stated in the slice. This means the existential quantifier is not clearly bound to the attestation; it could refer to any private key derived from `dstack_kms_derived`, not necessarily the one used by the enclave. This ambiguity could allow a scenario where the enclave uses a different private key than the one quantified in B10, violating B10 without violating B8. The worked example in §2.1 does not include attestation details, so this ambiguity is not caught by the worked examples.
- **Cite**: 
  > B10: `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`
- **Fix recommendation**: Add an explicit binding to the attestation: "always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)) ∧ the attestation in the firing publish_result transaction binds to privkey)". This makes the existential quantifier clearly related to the attestation and resolves the ambiguity.

## Slice-local summary
- Critical: 2 (B8 clause (c) hash function undefined, B10_lean EnclaveImage undefined)
- Serious: 5 (B6 ∀-per-key underspecified, B8 clause (d) undefined, B9 precondition undefined, B10 cross-layer tag mismatch, B10 existential quantifier not bound to attestation)
- Cosmetic: 1 (B8 temporal-state mismatch)

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: trust-quartz

*Elapsed: 70.4s, 16,121 chars.*

I'll read the specified slice of the intent document and then attack it according to the Colosseum methodology.
# glm-4-7-flash — slice trust-quartz

## Cross-section reads
- §3.2 B9 (negligibility-budget decomposition) — referenced for `image_registration_honest` temporal scope
- §3.2 B8 (attestation-binds-tally) — referenced for `image_registration_honest` definition
- §6.1 Chain trust — referenced for `enclave-image registration integrity` bullet

## Attacks on this slice

### 1. Composition failure — Quartz inheritance table claims "composition honest" without verifying honest composition [serious]

- **Category**: composition failure
- **Affected**: Section 6.2, Table row "DstackAttestor produces quotes whose validity reflects the TDX platform's runtime state"
- **What's wrong**: The table lists `cross_component_session_bind_negl` as the discharge path for the inherited claim, with the tag `(d) classically-over-strong-single-negligibility (sound) + preconditional (complete)`. The note below the table states that B9 inherits from this now-contented substrate. However, the table itself does not record whether verified-rcv has **re-verified** the composition of these inherited claims against the retracted substrate. The inheritance table is presented as a composition ledger entry, but it lacks the composition-check step that Colosseum methodology v0.4 requires (per methodology v0.4 Ask N: "compose ledger must record which upstream specs depend on which downstream specs and must verify the composition of those dependencies"). The table claims compositional honesty but does not document the composition verification that would make it honest.
- **Cite**: > | `DstackAttestor` produces quotes whose validity reflects the TDX platform's runtime state | `Specs.Quartz.Attestation.Dstack` (`tdxVerifier` record) | (d) classically-over-strong-single-negligibility (sound) + preconditional (complete) | PCK-signature unforgeability reduction + Lean reference DCAP verifier | > **Methodology consequence**: verified-rcv inherits B8/B9 from a now-contented substrate. The first three summands of B9 (`Adv_tdxVerifier_sound`, `Adv_groth16_KS`, `Adv_circuit_eq`) are bound by Quartz's now-def-tied `cross_component_session_bind_negl` — specifically by the `tdxVerifier`-tagged + two `groth16Verifier`-tagged summands of that lift.
- **Fix recommendation**: Add a column to the inheritance table titled "Composition verified?" with a boolean or "pending" status, and add a note below the table documenting the composition verification step (e.g., "Composition verified against retracted substrate on 2026-05-15 via colosseum-compose ledger entry C-2026-05-15"). If composition has not been verified, the inheritance table should be marked as "composition pending" rather than presenting the inherited claims as already honest.

### 2. Temporal-state mismatch — `image_registration_honest` referenced in B9 without temporal scope [serious]

- **Category**: temporal-state mismatch
- **Affected**: Section 3.2 B9 (referenced from §6.2), Section 6.1 enclave-image registration integrity bullet
- **What's wrong**: Section 6.1 states that `enclave-image registration integrity` is an **operational** assumption on the deployment/governance surface of the chain's vkey-registration mechanism, with failure mode 4.3b. Section 3.2 B9 conditions its negligibility budget on `image_registration_honest`, but B9 does not specify the **temporal scope** of this assumption. Is `image_registration_honest` assumed to hold:
  - (a) only at instantiation time (before the first `publish_result`), or
  - (b) continuously throughout the entire election lifecycle (including after `publish_result` has been called), or
  - (c) only for the duration of the enclave's attestation window (e.g., the time between `close_and_tally` and `publish_result`)?
  The spec does not disambiguate. If the assumption is (a) or (b), then B9's negligibility budget is **temporal-state mismatched**: it claims to bound the probability of attestation failure over the entire election, but the actual assumption only covers a subset of that timeline. If the assumption is (c), then B9's decomposition is incomplete because it does not account for the possibility that the registered vkey could be compromised **after** the attestation window closes but **before** `publish_result` is called (failure mode 4.3b). The temporal scope of `image_registration_honest` must be explicitly stated in B9, or B9 must be restructured to condition on the correct temporal window.
- **Cite**: > - **Enclave-image registration integrity** — the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an **operational** assumption on the deployment / governance surface of the chain's vkey-registration mechanism, not a cryptographic property reducible to a security parameter. Failure mode: 4.3b (attestation vkey maliciously substituted). **This bullet captures the assumption that was dropped from B9's negligibility budget per Round 3a first-pass adversary Attack 18** (the previous draft's `Adv_image_registration` summand was an operational-fault rate mis-cast as a cryptographic advantage; relocated here as a trust-boundary item). > B9 B8 negligibility-budget decomposition (4-summand, conditioned on `image_registration_honest`)
- **Fix recommendation**: Add a temporal-scope clause to B9, e.g., "B9 holds conditioned on `image_registration_honest` holding **at the time of attestation** (i.e., between `close_and_tally` and `publish_result`), or explicitly state that `image_registration_honest` is assumed to hold **continuously throughout the entire election lifecycle** and document the operational assumption accordingly. If the assumption is continuous, add a note that failure mode 4.3b represents a breach of this assumption and would invalidate B9's negligibility budget.

### 3. Over-specification — commitHashE row listed as inherited but explicitly marked "NOT CONSUMED" [cosmetic]

- **Category**: over-specification
- **Affected**: Section 6.2, Table row "commitHash collision-resistance on UserDataCommit → UserData"
- **What's wrong**: The table lists `commitHashE` as an inherited Quartz artifact, but the note below explicitly states that verified-rcv does **not** consume this bundle. The row is retained only for traceability. However, the table header "Inherited claim" implies that verified-rcv **does** inherit this claim, which is misleading. The row should either be removed or re-labeled as "Inherited for traceability only — not consumed by verified-rcv". This is a cosmetic issue that does not affect correctness (the spec correctly documents that the claim is not consumed), but it creates confusion for readers who might assume the inheritance table represents actual compositional dependencies.
- **Cite**: > | `commitHash` collision-resistance on `UserDataCommit → UserData` | `Specs.Quartz.Crypto.UserDataCommit` (`commitHashE` bundle) | (d) pigeonhole-impossible (lift restates as concrete-hash collision-resistance) | VCVio `randomOracle` + `[Fintype UserData]` carrier refinement; **NOT CONSUMED by verified-rcv — see Note A below** | > **Note A — `commitHashE` is listed but unused** (Round 3a re-adversarial pass, themes 8 + 15 — claude S5 / gemma C2): verified-rcv's hash-collision-resistance hypothesis is `Adv_commitTally_CR` (B9 fourth summand), bounding SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`. It does **not** consume Quartz's `commitHashE` bundle — that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous. The row is retained in the table for traceability (other Quartz-downstream specs may consume it), but the verified-rcv compose ledger will record this inheritance row's downstream consumer count as zero.
- **Fix recommendation**: Re-label the table row as "Inherited for traceability only — not consumed by verified-rcv" or remove the row entirely if it serves no purpose other than traceability. The current labeling creates a false impression of compositional dependency.

### 4. Ambiguity — Chain trust assumptions about Xion are under-specified [serious]

- **Category**: ambiguity
- **Affected**: Section 6.1 Chain trust (Xion / CosmWasm)
- **What's wrong**: Section 6.1 lists several chain trust assumptions, but several are under-specified or ambiguous:
  - **"Block time monotonicity"**: Does this include the possibility of clock jumps (e.g., reorgs that set `env.block.time` to an earlier value)? The spec says "non-decreasing across consecutive blocks", which allows clock jumps, but does not specify whether clock jumps are admissible. If clock jumps are admissible, then derived state predicates (Created / Voting / Tallying) that rely on `env.block.time ≥ start_at` could be violated by a clock jump, leading to unexpected state transitions.
  - **"State persistence"**: Does this include the possibility of silent corruption between blocks? The spec says "no silent corruption between blocks", but does not specify what constitutes "silent corruption" (e.g., partial write failures, disk corruption, consensus-level data loss). The failure mode 4.5 (Chain consensus halt or fork) addresses forks, but not silent corruption.
  - **"Fork resolution"**: The spec says "on a fork, the canonical chain state is determined by chain consensus; verified-rcv's election state follows whichever fork wins." However, this does not specify what happens if the fork occurs **during** the voting window (e.g., a reorg that changes the set of candidates or the `end_at` timestamp). Does verified-rcv detect forks and reject transactions, or does it blindly follow the fork? The spec does not disambiguate.
  - **"Enclave-image registration integrity"**: The spec says the registration is "not subject to undetected post-registration modification." However, this does not specify what constitutes "undetected" — does it mean the chain's governance mechanism would detect the modification, or does it mean the modification would be cryptographically detectable by verified-rcv? The failure mode 4.3b says "attestation vkey maliciously substituted", but does not specify how the substitution would be detected (e.g., by the chain's governance mechanism or by verified-rcv's attestation verification logic).
  These ambiguities create multiple non-equivalent interpretations, and the spec does not disambiguate which interpretation is correct.
- **Cite**: > - **Block time monotonicity** — `env.block.time` is non-decreasing across consecutive blocks. Used by every state-derived predicate (Created / Voting / Tallying). > - **State persistence** — state writes complete atomically; no silent corruption between blocks. > - **Fork resolution** — on a fork, the canonical chain state is determined by chain consensus; verified-rcv's election state follows whichever fork wins. > - **Enclave-image registration integrity** — the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an **operational** assumption on the deployment / governance surface of the chain's vkey-registration mechanism, not a cryptographic property reducible to a security parameter. Failure mode: 4.3b (attestation vkey maliciously substituted).
- **Fix recommendation**: Add disambiguating clauses to each bullet point:
  - For "Block time monotonicity": "Clock jumps are admissible, but `env.block.time` must never decrease across consecutive blocks. If a clock jump occurs, derived state predicates (Created / Voting / Tallying) are evaluated against the new timestamp; if a predicate fails, the transaction is rejected."
  - For "State persistence": "Silent corruption is defined as partial write failures, disk corruption, or consensus-level data loss that leaves the contract state in an inconsistent or corrupted state. If silent corruption is detected, the contract enters a terminal error state and no further transactions are accepted."
  - For "Fork resolution": "If a fork occurs during the voting window, verified-rcv rejects all transactions until the fork is resolved and the canonical chain state is stable. If a fork occurs during the tallying or resolved state, verified-rcv follows the canonical fork and updates the election state accordingly."
  - For "Enclave-image registration integrity": "Post-registration modification is detectable by the chain's governance mechanism (e.g., admin key revocation). If the governance mechanism detects a modification, the registration is invalidated and future attestations using the old vkey are rejected."

### 5. Impossibility-hypothesis vacuity — B9 conditions on `image_registration_honest` which may be vacuously true [serious]

- **Category**: impossibility-hypothesis vacuity
- **Affected**: Section 3.2 B9 (referenced from §6.2)
- **What's wrong**: Section 6.1 states that `enclave-image registration integrity` is an **operational** assumption on the deployment/governance surface of the chain's vkey-registration mechanism, not a cryptographic property reducible to a security parameter. Section 3.2 B9 conditions its negligibility budget on `image_registration_honest`, but B9 does not specify what `image_registration_honest` means operationally. If `image_registration_honest` is defined as "the chain's governance mechanism does not allow post-registration modification of the registered vkey", then B9's negligibility budget is **impossibility-hypothesis vacuous**: the hypothesis is not a cryptographic advantage but an operational policy assumption. The classical statement of B9 would be "if the chain's governance mechanism allows post-registration modification, then the probability of attestation failure is bounded by ...", but the current statement conditions on `image_registration_honest` without defining it operationally. This makes the hypothesis vacuously true (if the governance mechanism never allows modification, then the hypothesis holds trivially), but it does not provide any meaningful bound on the probability of attestation failure in the real world where governance mechanisms can be compromised.
- **Cite**: > - **Enclave-image registration integrity** — the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an **operational** assumption on the deployment / governance surface of the chain's vkey-registration mechanism, not a cryptographic property reducible to a security parameter. Failure mode: 4.3b (attestation vkey maliciously substituted). **This bullet captures the assumption that was dropped from B9's negligibility budget per Round 3a first-pass adversary Attack 18** (the previous draft's `Adv_image_registration` summand was an operational-fault rate mis-cast as a cryptographic advantage; relocated here as a trust-boundary item). > B9 B8 negligibility-budget decomposition (4-summand, conditioned on `image_registration_honest`)
- **Fix recommendation**: Define `image_registration_honest` operationally in B9, e.g., "B9 holds conditioned on `image_registration_honest`, where `image_registration_honest` means: (1) the chain's governance mechanism allows post-registration modification only via a multi-sig admin key with threshold ≥ 2/3 of the chain's validator set, and (2) any modification is logged and detectable by verified-rcv's attestation verification logic." Alternatively, restructure B9 to condition on the **probability of governance compromise** rather than an operational policy assumption, e.g., "B9 holds conditioned on `Adv_governance_compromise ≤ ε`, where `Adv_governance_compromise` is the probability that the chain's governance mechanism allows post-registration modification of the registered vkey."

## Slice-local summary
- Critical: 0
- Serious: 3
- Cosmetic: 2

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: off-chain-witness

*Elapsed: 57.4s, 13,016 chars.*

I'll read the target slice from the intent document and then produce an aggressive attack report.
# glm-4-7-flash — slice off-chain-witness

## Cross-section reads
- Section 6.3 (dstack/TDX hardware trust) — provides the dstack-KMS-derived privkey existential witness that Step 6 silently imports
- Section 3.2 B10 (cross-layer invariant) — the target statement that Step 6 claims to discharge
- Section 3.2 B10_lean (off-chain invariant) — the Lean-internal image-IO obligation that Step 3 claims to compose
- Section 6.2 (Quartz inheritance) — documents the upstream-blocked Stage-1 soundness gap noted in Step 1

## Attacks on this slice

### 1. Stage-1 lemma is upstream-blocked, not proven [serious]
- **Category**: coverage gap / impossibility-hypothesis vacuity
- **Affected**: Step 1 (decrypt_and_validate correctness)
- **What's wrong**: Step 1 is presented as a "Stage-1 lemma" that "for every (raw_ballots, candidates, privkey), the implementation of Stage 1 returns the partition (valid_ballots, dropped_voters, non_voters) matching the spec." However, the caveat immediately below explicitly states: "Ecies.roundtrip proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts. The Stage-1 spec is therefore stated as 'deterministic decrypt-or-drop' (correctness of the partition is definitional once the decoder is fixed); the soundness claim 'a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate' requires an additional authenticated-decryption (AEAD) property. Quartz does not currently expose this property. Recorded as a v0.3 upstream ask on Quartz; verified-rcv's compose ledger flags the Stage-1 soundness obligation as upstream-blocked."

This is a **hand-waved claim presented as a proven lemma**. The decomposition asserts Step 1 is a theorem, but the text admits it's not actually proven and is upstream-blocked. This makes Step 6's claim that B10 follows from "B10_lean (steps 1+2+3)" factually incorrect — Step 1 is not a theorem.

- **Cite**: > "Stage-1 lemma — decrypt_and_validate correctness: for every (raw_ballots, candidates, privkey), the implementation of Stage 1 returns the partition (valid_ballots, dropped_voters, non_voters) matching the spec in Section 2.5. Depends on Quartz's Specs.Quartz.Crypto.Ecies.roundtrip for the decryption-correctness half (Section 6.2 inheritance row 3 — a theorem, not an axiom). **Caveat (Round 3a re-adversarial theme 9 / claude S7)**: Ecies.roundtrip proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts. The Stage-1 spec is therefore stated as 'deterministic decrypt-or-drop' (correctness of the partition is definitional once the decoder is fixed); the soundness claim 'a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate' requires an additional authenticated-decryption (AEAD) property. Quartz does not currently expose this property. Recorded as a v0.3 upstream ask on Quartz; verified-rcv's compose ledger flags the Stage-1 soundness obligation as upstream-blocked."

- **Fix recommendation**: Either (a) remove Step 1 from the B10 composition and explicitly mark it as an upstream-blocked gap that must be resolved before B10 can be discharged, or (b) restate Step 1 as a "spec-level obligation" rather than a "lemma" and adjust Step 6 to reflect that B10_lean depends on an unresolved upstream dependency.

### 2. Image identity binding is not a Lean theorem [serious]
- **Category**: triviality / over-specification
- **Affected**: Step 4 (image identity binding)
- **What's wrong**: Step 4 is presented as part of the B10 composition: "Image identity binding: the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that B10_lean is over. This is an extraction-soundness + build-reproducibility obligation that lives at the build pipeline + deployment ledger layer, not the Lean layer. It cannot be a Lean theorem because the Lean spec models the binary; the binding claim is about the modelling discipline. The verified-rcv compose ledger records this as a tracked obligation with a discrete artifact (reproducible build hash + registered MRTD/RTMR equality)."

This is **not a Lean theorem** — it's a build/ledger obligation. Yet Step 6 claims B10 follows from "B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5)". By including Step 4 as a "step" in the B10 composition, the spec misleads about what is actually proven. The decomposition is dishonest about the layering — it presents a build/ledger obligation as if it were a mathematical theorem that can be composed with Lean theorems.

- **Cite**: > "Image identity binding: the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that B10_lean is over. This is an extraction-soundness + build-reproducibility obligation that lives at the build pipeline + deployment ledger layer, not the Lean layer. It cannot be a Lean theorem because the Lean spec models the binary; the binding claim is about the modelling discipline. The verified-rcv compose ledger records this as a tracked obligation with a discrete artifact (reproducible build hash + registered MRTD/RTMR equality)."

- **Fix recommendation**: Remove Step 4 from the numbered B10 composition list. It should be documented as a separate "build/ledger obligation" that must be satisfied independently, not as a step in the B10 discharge. Update Step 6 to read: "from B10_lean (steps 1+2+3) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey." Then add a separate note: "Image-identity-binding (Step 4) is a build/ledger obligation tracked in the compose ledger; it must be satisfied independently of the Lean proof of B10_lean."

### 3. EnclaveImage symbol typing mismatch [critical]
- **Category**: preconditional over-strength / type error
- **Affected**: Step 3 (Lean-internal composition — B10_lean)
- **What's wrong**: The spec defines EnclaveImage in the CONTEXT APPENDIX as: "EnclaveImage : Bytes → Bytes (image_extract)". However, Step 3 states: "by composition of the Stage-1 lemma and Stage-2 theorem, the enclave image's externally observable input-output relation satisfies EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)."

This is a **fundamental type error**. EnclaveImage is defined as taking a single Bytes argument (the image binary), but Step 3 calls it with three arguments: raw_ballots, candidates, and privkey. The right-hand side Tally_spec also takes three arguments. The composition is ill-typed — you cannot compose a function of type Bytes → Bytes with a function of type (raw_ballots, candidates, privkey) → Tally_spec.

This is not just a notational issue — it's a **preconditional over-strength**. The spec asserts that EnclaveImage has an input-output relation that matches Tally_spec, but EnclaveImage's actual type signature doesn't allow it to take the necessary inputs. The decomposition is factually incorrect.

- **Cite**: > "EnclaveImage : Bytes → Bytes (image_extract)" (from CONTEXT APPENDIX) and > "by composition of the Stage-1 lemma and Stage-2 theorem, the enclave image's externally observable input-output relation satisfies EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)."

- **Fix recommendation**: Either (a) redefine EnclaveImage to take the correct type signature (e.g., EnclaveImage : (raw_ballots, candidates, privkey) → Tally_spec) and update the CONTEXT APPENDIX definition, or (b) restate Step 3 to use a different function name (e.g., EnclaveImageIO : (raw_ballots, candidates, privkey) → Tally_spec) and clarify that EnclaveImage is the binary extraction function, not the IO relation. The latter is more honest about the layering.

### 4. Step 6 does not actually conclude B10 [serious]
- **Category**: impossibility-hypothesis vacuity
- **Affected**: Step 6 (Composition — B10)
- **What's wrong**: Step 6 claims: "from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey. This is the B10 statement from Section 3.2."

This claim is **vacuously true** because the premises are not actually satisfied:
1. Step 1 is not a theorem (it's upstream-blocked)
2. Step 4 is not a Lean theorem (it's a build/ledger obligation)
3. Step 5 is just a reference to B8, which is already covered elsewhere
4. The existential quantifier instantiation "with the dstack-KMS-derived privkey (Section 6.3)" is not justified — where does this privkey come from? The spec doesn't show how the existential is instantiated in the Lean proof.

The decomposition is dishonest about what's actually proven. Step 6 presents a composition that appears to discharge B10, but the components are either unproven or not the right type of thing to compose.

- **Cite**: > "from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey. This is the B10 statement from Section 3.2."

- **Fix recommendation**: Either (a) remove Step 6 entirely and replace it with a honest statement that B10 is **not yet discharged** — the composition is blocked by the upstream-blocked Stage-1 lemma and the missing image-identity-binding proof, or (b) restructure the decomposition to explicitly separate what is proven (Steps 2+3) from what is not (Steps 1+4+5), and mark B10 as "pending upstream dependencies." The honest approach is (a) — the current Step 6 is a misleading claim that B10 follows from the composition when it doesn't.

### 5. Fourth trust link (dstack-KMS) not honestly admitted [serious]
- **Category**: under-specification
- **Affected**: Step 6 (Composition — B10)
- **What's wrong**: Step 6 includes the phrase "with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3)". This is a **fourth trust link** that is not honestly admitted in the decomposition. The decomposition presents B10 as a composition of three things: B10_lean, image-identity-binding, and B8. But actually, there's a fourth thing: the dstack-KMS-derived privkey that instantiates the existential quantifier in B10.

Section 6.3 states: "verified-rcv trusts: dstack KMS honesty — the KMS releases the per-election private key only after verifying the requesting enclave's attestation." This is an operational trust assumption that is not part of the honest decomposition. The spec doesn't show how the existential quantifier in B10 is instantiated — it just asserts it is, without explaining the mechanism.

This is **under-specification** — the existential quantifier in B10 is not discharged honestly. The decomposition hides the dstack-KMS trust link as an afterthought in Step 6, rather than making it an explicit component of the composition.

- **Cite**: > "from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3): the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey) for the dstack-KMS-derived privkey."

- **Fix recommendation**: Either (a) make the dstack-KMS-derived privkey an explicit step in the composition (e.g., Step 4a: "Privkey instantiation — the dstack-KMS-derived privkey is retrieved from the KMS after attestation, satisfying the existential quantifier in B10"), or (b) remove the existential quantifier from B10 and make the privkey a free variable in the Lean proof (i.e., B10_lean should be parameterized over the privkey, and the instantiation happens outside the Lean proof). The honest approach is (a) — the current Step 6 hides the dstack-KMS trust link as an afterthought.

## Slice-local summary
- Critical: 1 (EnclaveImage symbol typing mismatch)
- Serious: 4 (Stage-1 lemma upstream-blocked, image identity binding not a Lean theorem, Step 6 doesn't conclude B10, dstack-KMS trust link not honestly admitted)
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE

The slice is fundamentally broken. The EnclaveImage type mismatch makes Step 3 factually incorrect. The upstream-blocked Stage-1 lemma and non-Lean image-identity-binding make Step 6's claim that B10 follows from the composition vacuously true. The dstack-KMS trust link is hidden as an afterthought rather than being honestly admitted. The decomposition is dishonest about what is actually proven and what is not.
