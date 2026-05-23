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