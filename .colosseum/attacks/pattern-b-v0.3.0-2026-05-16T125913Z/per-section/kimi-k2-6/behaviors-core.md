# kimi-k2-6 — slice behaviors-core

## Cross-section reads
- Section 2.5 Block 6 (`publish_result` error mapping) — to verify the ground-truth error codes for `derived state ≠ Tallying` and well-formedness checks.
- Section 2.5 `TallyResult` schema comment — to verify the "parallel to per_round_counts" definition of `eliminated_by_round`.
- Section 6.1 trust boundary — to verify the block-time monotonicity assumption (non-decreasing, not strictly increasing).

## Attacks on this slice

### 1. `publish_result` error mapping contradicts Block 6 [critical]
- **Category**: contradiction
- **Affected**: 2.4 Explicit failures table, rows `VotingStillOpen`, `AlreadyResolved`, `NotInTallyingState`
- **What's wrong**: The table maps `publish_result` before `end_at` to `VotingStillOpen` and `publish_result` after `tally_result.is_some()` to `AlreadyResolved`. However, Block 6's ground-truth error mapping assigns `derived state ≠ Tallying → NotInTallyingState` for any `publish_result` call where the state is not Tallying, which includes both Voting (before `end_at`) and Resolved (after `tally_result.is_some()`). The table therefore contradicts the structured behavior block and is internally inconsistent: the `NotInTallyingState` row says "see `AlreadyResolved`" for Resolved, but the `AlreadyResolved` row's cause description doesn't align with Block 6's mapping.
- **Cite**: > `| VotingStillOpen | close_and_tally or publish_result before end_at | Yes — wait for end_at |` and `| AlreadyResolved | Any state-mutating handler after tally_result.is_some() | No — election is terminal |` and `| NotInTallyingState | publish_result called when derived state is not Tallying | Yes if state was Created/Voting (wait); No if state was Resolved (terminal — see AlreadyResolved) |`
- **Fix recommendation**: Align the Explicit failures table with Block 6: `publish_result` in any non-Tallying state returns `NotInTallyingState`. Remove `publish_result` from the `VotingStillOpen` cause and remove the "see `AlreadyResolved`" note from the `NotInTallyingState` row. If `AlreadyResolved` is desired for Resolved state, update Block 6 to check `tally_result.is_some()` first.

### 2. Happy path `eliminated_by_round` omits final-round empty list [serious]
- **Category**: contradiction
- **Affected**: 2.1 Happy path, output block
- **What's wrong**: The happy path output shows `per_round_counts` with 3 entries (rounds 1–3) but `eliminated_by_round` with only 2 entries. Section 2.5 defines `eliminated_by_round` as "parallel to per_round_counts", implying identical length and indexing. Round 3 eliminated no one, so the parallel array should include `[]` as its third entry. The omission creates ambiguity about whether implementers should omit empty elimination rounds or maintain strict parallelism.
- **Cite**: > `per_round_counts    = [ {A: 2, B: 0, C: 1, D: 1, E: 1}, {A: 2, C: 1, D: 1, E: 1}, {A: 5} ]` and `eliminated_by_round = [[B], [C, D, E]]`
- **Fix recommendation**: Append `[]` to `eliminated_by_round` in the happy path output, or amend the 2.5 schema comment to clarify that "parallel" permits omitting rounds with no eliminations.

### 3. `ballots_dropped = N` uses undefined symbol [serious]
- **Category**: ambiguity
- **Affected**: 2.2 Boundary cases, "All ballots dropped as malformed"
- **What's wrong**: The text states `ballots_dropped = N` without defining `N`. An implementer cannot determine whether `N` equals `len(candidates)`, the number of submitted ballots, or some other value. Since the case asserts `non_voters = ∅`, all candidates must have submitted a ballot, so `N` should equal `len(candidates)`, but this is never made explicit.
- **Cite**: > `All ballots dropped as malformed (ballots_tallied = 0, ballots_dropped > 0) — same shape as all-abstain, but non_voters = ∅ and ballots_dropped = N.`
- **Fix recommendation**: Replace `N` with `len(candidates)` and explicitly state that this case assumes every candidate submitted exactly one malformed ballot.

### 4. Missing mixed abstain + malformed boundary case [serious]
- **Category**: coverage gap
- **Affected**: 2.2 Boundary cases
- **What's wrong**: The boundary cases cover all-abstain, all-dropped, and some-abstain-with-valid-ballots, but omit the reachable case where some candidates abstain, some submit malformed ballots, and some submit valid ballots. This mixed case exercises the conservation invariant `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)` in a non-trivial way and is the natural composition of the existing partial cases. Its absence leaves implementers without a worked example for the most general non-homogeneous input.
- **Cite**: > Boundary cases list: "Single candidate", "Two candidates, both vote, terminal 1-1 tie", "All candidates abstain", "Some candidates abstain", "All ballots dropped as malformed".
- **Fix recommendation**: Add a worked example with 4 candidates: 1 abstains, 1 submits malformed, 2 submit valid ballots, showing expected `ballots_tallied`, `ballots_dropped`, `non_voters`, and the IRV trace.

### 5. `InvalidTally` claims batch-elim check that Block 6 omits [serious]
- **Category**: contradiction
- **Affected**: 2.4 Explicit failures, `InvalidTally` row
- **What's wrong**: The `InvalidTally` cause description includes "batch-elim policy violation" as a checked condition. However, Block 6's well-formedness predicate — the actual on-chain validation gate — does not include any clause verifying that eliminations followed the batch-elimination rule. It only checks monotonicity, count conservation, set relations, and winner well-formedness. A tally that eliminates only one of two tied-lowest candidates would pass Block 6's checks but violates the stated batch-elim policy, meaning the chain would accept an invalid tally.
- **Cite**: > `InvalidTally { reason } | Result fails well-formedness (winner ∉ candidates, counts don't conserve, elimination sequence inconsistent, set-relation violation, batch-elim policy violation) | ...`
- **Fix recommendation**: Either add an explicit batch-elimination validation clause to Block 6 (e.g., "for each round i, eliminated_by_round[i] equals the set of candidates with minimum count in per_round_counts[i]") or remove "batch-elim policy violation" from the `InvalidTally` cause description.

### 6. Round 1 redistribution description is misleading [cosmetic]
- **Category**: ambiguity
- **Affected**: 2.1 Happy path, Round 1 row
- **What's wrong**: The text says "Redistribute B's ballot ... which already counts B's first-place vote — no movement." This is misleading because B had zero first-place votes in Round 1; there was no B-first-place vote to redistribute. The ballot was already counted for A. The phrasing implies a redistribution operation was performed and found no movement, when in fact no redistribution was needed.
- **Cite**: > `B has 0 first-place votes (unique lowest). Eliminate B. Redistribute B's ballot: B's first surviving choice is A (since A > B > … after dropping B is A > C > D > E), which already counts B's first-place vote — no movement.`
- **Fix recommendation**: Clarify that B had zero first-place votes, so no redistribution occurs; the ballot was already counted for A.

### 7. `start_at` edge case assumes strict per-block time advance [cosmetic]
- **Category**: edge case
- **Affected**: 2.3 Edge cases, "Instantiation with start_at close to env.block.time"
- **What's wrong**: The text claims the contract "enters Voting state on the very next block after start_at". This assumes block time strictly increases every block. The trust boundary in Section 6.1 only assumes non-decreasing monotonicity. If block time stalls across multiple blocks, the contract may remain in Created longer than stated.
- **Cite**: > `Instantiation with start_at close to env.block.time — admissible as long as start_at > env.block.time; the contract enters Voting state on the very next block after start_at.`
- **Fix recommendation**: Replace with "once a block with env.block.time ≥ start_at is produced".

## Slice-local summary
- Critical: 1
- Serious: 4
- Cosmetic: 2

## VERDICT (slice-local): BREAKS-AT-SLICE