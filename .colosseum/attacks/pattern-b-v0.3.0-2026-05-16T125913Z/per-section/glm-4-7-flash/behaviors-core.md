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