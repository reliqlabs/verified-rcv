# gemma-4-26b-a4b — verified-rcv intent v0.3.0 adversarial pass (Pattern B, per-section dispatch)

- **Pass**: Pattern B calibration (4th attack against v0.3.0 — same intent, OpenCode + file-access subagent dispatch)
- **Slices dispatched**: 1

---

## Slice: state-invariants

*Elapsed: 62.4s, 4,154 chars.*

# colosseum-spec-adversary — slice state-invariants

## Cross-section reads
- `2.5 Structured behavior blocks` for Block 6 requirements and $TallyResult$ schema to validate S6, S7, S8, S9.
- `3.2 Behavioral invariants` for B1/B3 to validate the S5/B1 distinction.
- `4.9 Deterministic-malformed-tally deadlock` for context on $InvalidTally$.

## Attacks on this slice
### 1. S5/B1 redundancy and potential for refinement mismatch [serious]
- **Category**: `temporal-state mismatch`
- **Affected**: S5, B1
- **What's wrong**: The spec attempts to solve a previous "successor quantification" error in S5 by reframing it as a "set-once write-discipline" property evaluable on a single state. However, it explicitly delegates the actual "once, always" trajectory property to B1 in the temporal section. This creates a risk where an implementation might satisfy S5 (the handler doesn't write to the slot if it's full) but fail B1 (due to a different handler or an unexpected state transition not covered by the "set-once" logic). While mathematically distinct, in a verification context, having two invariants covering the same logical property with different "tags" often leads to refinement mismatches where a developer proves S5 but the B1 property fails in the actual temporal model.
- **Cite**: > S5 (derived) | terminality of resolution (state-shape) | **`tally_result: Option<TallyResult>` is set-once: no handler writes to `tally_result` after it has been set to `Some(_)`.** ... The trajectory claim "once `Some`, always `Some` with the same value" is **B1**'s temporal form
- **Fix recommendation**: Consolidate the property. Keep B1 as the primary temporal invariant for "once, always" and define S5 purely as a structural property of the *transition set* or remove it to avoid dual-specification overhead that complicates formal proofs.

### 2. S10/B3 logical dependency gap [cosmetic]
- **Category**: `under-specification`
- **Affected**: S10
- **What's wrong**: S10 is stated as a derived structural invariant: "if `tally_result.is_some()`, then `env.block.time ≥ end_at`." It claims to be derived from B3 (the temporal causal version). However, S10 is a property of the *state*, whereas B3 is a property of the *transition*. A state could satisfy S10 (the result exists and time is past `end_at`) even if a transition violated B3 (the result was set *before* `end_at`). While the system's security relies on B3, S10 provides no actual "safety" if B3 is broken; it's a post-hoc observation.
- **Cite**: > S10 (derived) | resolution implies past end_at | if `tally_result.is_some()`, then `env.block.time ≥ end_at`. **Derived from B3**
- **Fix recommendation**: Acknowledge that S10 is a *safety property* that relies on the *liveness/causality* of B3, or move it entirely to the temporal section to ensure it's treated as a property of the execution trace.

### 3. S8/S9 Underspecification: Zero-ballot/Empty-set behavior [serious]
- **Category**: `edge case`
- **Affected**: S8, S9
- **What's wrong**: The structural invariants for round consistency (S8) and elimination monotonicity (S9) assume a non-empty set of surviving candidates/counts. In the "All candidates abstain" boundary case (Section 2.2, line 73), all candidates are co-winners with 0 votes. For $i=0$, $per\_round\_counts[0]$ contains all candidates with 0. For subsequent rounds, the set might be empty or behave erratically depending on how "surviving" is indexed in an empty-set context. S8/S9 do not explicitly define behavior for $ballots\_tallied = 0$. A verifier might find a vacuous satisfaction where an empty $per\_round\_counts$ satisfies "all entries sum to 0" but fails the intended IRV logic.
- **Cite**: > S8 | per-round count consistency | each `per_round_counts[i]` has all entries summing to `ballots_tallied`.
- **Fix recommendation**: Explicitly constrain S8/S9 for the $ballots\_tallied > 0$ case, or define the behavior of empty/zero-sum maps in the $TallyResult$ schema to ensure invariants are not vacuously satisfied in abort/abstain scenarios.

## Slice-local summary
- Critical: 0
- Serious: 2
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE
