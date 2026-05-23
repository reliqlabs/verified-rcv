# glm-4-7-flash — slice scenarios

## Cross-section reads
- Section 2.5 (transaction trace model, lines 149-150) — defines `fires_at_transition(σ → σ′)` for B6's witness
- Section 3.2 behavioral invariants (lines 337-349) — defines B1-B6 statements to be witnessed
- Section 8.7 (lines 684-701) — describes B10's off-chain witness structure

## Attacks on this slice

### 1. Coverage gap for B1, B2, B3 [serious]
- **Category**: coverage gap
- **Affected**: §8.1 through §8.6 (no explicit witness for B1, B2, B3)
- **What's wrong**: The six concrete scenarios (8.1–8.6) explicitly witness only B4, B5, and B6. B1 (tally_result monotone-once-set), B2 (no late ballots), and B3 (no premature tally) are not given explicit witnessing scenarios. B1 is mentioned in 8.5's trust claim but labeled as B5; B2 is mentioned in 8.1 step 4 but not as a witness; B3 is mentioned in 8.1 step 5 but not as a witness. The intent document's Revision Log (line 724) claims "all temporal invariants retain scenario-level witnesses," but this is false for B1, B2, B3.
- **Cite**: 
  - §8.1 trust claim: "Trust claim consumed: B8 + B9"
  - §8.4 trust claim: "Trust claim consumed: B4 (no premature voting)"
  - §8.5 trust claim: "Trust claim consumed: B5 (publish_result fires at most once)"
  - §8.6 trust claim: "Trust claim consumed: B6 (ballot writer is the ballot voter)"
  - §3.2 B1 statement (line 339): `always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))`
  - §3.2 B2 statement (line 340): `always (env.block.time ≥ end_at → ballots = ballots@end_at)`
  - §3.2 B3 statement (line 341): `always (next.tally_result.is_some() ∧ tally_result.is_none() → env.block.time ≥ end_at)`
- **Fix recommendation**: Add three new scenarios (or expand existing ones) that explicitly witness B1, B2, and B3:
  - **8.7 (new)**: "Tally_result monotone-once-set" — election resolves, then a second `publish_result` is attempted (from any address, with or without attestation). The contract rejects with `AlreadyResolved`; the original `tally_result` is unchanged. This explicitly witnesses B1's "once resolved, stays resolved" temporal claim.
  - **8.8 (new)**: "Late ballot rejected" — election in Tallying state, a candidate submits a ballot after `end_at`. The contract rejects with `NotInVotingWindow`; `ballots` remains frozen at `end_at`. This explicitly witnesses B2's ballot-frozen claim.
  - **8.9 (new)**: "Premature tally rejected" — election in Voting state, a candidate calls `close_and_tally`. The contract rejects with `VotingStillOpen`; `tally_result` remains `None`. This explicitly witnesses B3's causal requirement that tallying requires `end_at`.

### 2. Under-specification of B4 witness [serious]
- **Category**: under-specification
- **Affected**: §8.4 (Premature voting attempt rejected)
- **What's wrong**: §8.4 explicitly witnesses B4 ("Trust claim consumed: B4"), but the scenario only demonstrates one of B4's three temporal conditions: voting before `start_at`. B4's statement (§3.2 line 342) quantifies over all transitions where `next.ballots ≠ ballots`, requiring `start_at ≤ env.block.time < end_at`. The scenario does not witness B4 for:
  - Voting after `end_at` (which is B2's domain, but B4's temporal condition still applies)
  - Voting during Tallying or Resolved states (which should also be rejected by B4's time-derived-state check)
  - Voting at the exact `end_at` boundary (edge case not covered)
- **Cite**: 
  - §8.4 trust claim: "Trust claim consumed: B4 (no premature voting)"
  - §8.4 step 4: "At `T0 + 1h`, block time passes `end_at`; contract enters Tallying. Late `submit_ballot` calls are now rejected with `NotInVotingWindow`."
  - §3.2 B4 statement (line 342): `always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)`
- **Fix recommendation**: Expand §8.4 to include a second sub-scenario (e.g., "8.4b") demonstrating B4's rejection of voting during Tallying or Resolved states, or explicitly state that §8.4's witness is incomplete and the remaining cases are covered by B2/B3/B1. The intent should be honest about which scenarios witness which invariants.

### 3. Under-specification of B5 witness [serious]
- **Category**: under-specification
- **Affected**: §8.5 (Double-publish attempt rejected)
- **What's wrong**: §8.5 explicitly witnesses B5 ("Trust claim consumed: B5"), but the scenario only demonstrates one edge case: a second `publish_result` after the election has already reached Resolved. B5's statement (§3.2 line 343) is a temporal property about the firing of `publish_result` transitions: `always (tally_result.is_none() ∧ next.tally_result.is_some() → no further publish_result tx ever succeeds)`. The scenario assumes the election is already Resolved, but B5 is about the *transition* firing, not just the terminality of the result. The spec does not witness B5 for:
  - Multiple `publish_result` calls during Tallying (which should be rejected by B3/B1)
  - `publish_result` during Voting (rejected by B3)
  - The idempotent nature of `close_and_tally` (Block 5) which is unrelated to B5
- **Cite**: 
  - §8.5 trust claim: "Trust claim consumed: B5 (publish_result fires at most once)"
  - §8.5 step 2: "Some time later, the enclave (or an attacker) submits a second `publish_result` carrying `T2` (possibly different from `T1`) and a valid attestation."
  - §8.5 step 3: "Block 6 evaluates: derived state is **Resolved** (because `tally_result.is_some()`), not Tallying."
  - §3.2 B5 statement (line 343): `always (tally_result.is_none() ∧ next.tally_result.is_some() → no further publish_result tx ever succeeds)`
- **Fix recommendation**: Expand §8.5 to include a sub-scenario demonstrating B5's rejection of multiple `publish_result` calls during Tallying (e.g., "8.5b"), or explicitly state that B5's witness is incomplete and the remaining cases are covered by B1/B3. The intent should be honest about which scenarios witness which invariants.

### 4. Under-specification of B6 witness [serious]
- **Category**: under-specification
- **Affected**: §8.6 (Impersonation attempt rejected)
- **What's wrong**: §8.6 explicitly witnesses B6 ("Trust claim consumed: B6"), but the primary scenario only covers non-candidate addresses. B6's statement (§3.2 line 344) is the `∀-per-key` form: `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`. The scenario does not witness B6 for:
  - Last-write-wins overwrites where one candidate's tx mutates another's slot (the `∀-per-key` form requires each key change to be attributable to a tx with `msg.sender = k`)
  - Multiple `SubmitBallot` txs in the same transition (the `fires_at_transition` quantifier requires the existential to range over the firing set, not the global history)
  - The stolen-key variant is mentioned as a "variant" but not as a primary witness
- **Cite**: 
  - §8.6 trust claim: "Trust claim consumed: B6 (ballot writer is the ballot voter)"
  - §8.6 step 3: "Block 3 evaluates: `msg.sender == Eve`. Requires clause `msg.sender ∈ candidates` fails because `Eve ∉ [A, B, C]`."
  - §8.6 variant: "**Variant — stolen-key impersonation**: an adversary who *has* compromised candidate A's private key (failure mode 4.7) can submit a ballot as A."
  - §3.2 B6 statement (line 344): `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`
  - §2.5 transaction trace model (line 149): "A state transition `σ → σ′` is parameterized by the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking `σ` to `σ′`"
- **Fix recommendation**: Add a sub-scenario (e.g., "8.6b") demonstrating B6's `∀-per-key` witness for last-write-wins overwrites (e.g., candidate A submits a ballot, then candidate B submits a ballot, overwriting A's slot). The scenario should show that the final `ballots[A]` is attributable to B's tx, not A's tx. Alternatively, explicitly state that B6's witness is incomplete and the remaining cases are covered by the `msg.sender ∈ candidates` check in Block 3.

### 5. Edge case witnesses missing [serious]
- **Category**: edge case
- **Affected**: §8.1 through §8.6 (no explicit edge-case witnesses for abstention, premature voting during Tallying/Resolved, late ballots, boundary conditions)
- **What's wrong**: The scenarios do not explicitly witness several edge cases mentioned in Section 2.3 (edge cases):
  - **Abstention-heavy elections during voting window**: Section 2.2 boundary case mentions "Some candidates abstain (`0 < len(ballots) < len(candidates)`)", but no scenario explicitly witnesses B4 for abstention-heavy voting windows.
  - **Premature voting during Tallying/Resolved**: Section 2.3 edge case mentions "Late ballot during Tallying" (B2), but no scenario explicitly witnesses B4 for voting during Tallying or Resolved states.
  - **Late ballots during Tallying**: Section 2.3 edge case mentions "Late ballot during Tallying" (B2), but no scenario explicitly witnesses B2 for late ballots during Tallying.
  - **Boundary conditions**: Section 2.3 edge case mentions "Instantiation with `start_at` close to `env.block.time`" and "All ballots dropped as malformed", but no scenario explicitly witnesses B4/B2 for these edge cases.
- **Cite**: 
  - §2.3 edge cases (lines 81-92): "Late ballot during Tallying", "Ballot revision", "Malformed ballot content", "Attestation race", "Enclave never publishes", "Multiple `close_and_tally` calls", "Instantiation with `start_at` close to `env.block.time`", "Identical encrypted-ballot bytes from two voters"
  - §2.2 boundary cases (lines 75-77): "Some candidates abstain (`0 < len(ballots) < len(candidates)`) — tally proceeds with submitted ballots only"
  - §3.2 B4 statement (line 342): `always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)`
  - §3.2 B2 statement (line 340): `always (env.block.time ≥ end_at → ballots = ballots@end_at)`
- **Fix recommendation**: Add scenarios or sub-scenarios that explicitly witness B4 and B2 for the edge cases mentioned in §2.3. For example:
  - **8.10 (new)**: "Abstention-heavy voting window" — election in Voting state with 3 abstaining candidates. A candidate submits a ballot. The contract accepts it because `start_at ≤ env.block.time < end_at`. This witnesses B4 for abstention-heavy elections.
  - **8.11 (new)**: "Late ballot during Tallying" — election in Tallying state. A candidate submits a ballot after `end_at`. The contract rejects with `NotInVotingWindow`; `ballots` remains frozen. This witnesses B2 for late ballots during Tallying.
  - **8.12 (new)**: "Ballot at end_at boundary" — election at `env.block.time = end_at`. A candidate submits a ballot. The contract accepts it because `start_at ≤ env.block.time < end_at` (the condition is strict on the upper bound). This witnesses B4 for the boundary condition.

## Slice-local summary
- Critical: 0
- Serious: 5
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE