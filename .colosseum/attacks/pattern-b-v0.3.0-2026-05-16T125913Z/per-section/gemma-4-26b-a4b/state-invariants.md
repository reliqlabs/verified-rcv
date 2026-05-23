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