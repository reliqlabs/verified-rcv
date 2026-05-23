# Design notes

This Quint model maps §2.5 Block 1 to `instantiate`, Blocks 2 and 4 to `timeAdvanceToStart` and `timeAdvanceToEnd`, Block 3 to `submitBallot`, Block 5 to `closeAndTally`, Block 6 to `publishResult`, and Block E1 to a separate `enclaveTally` action. `terminalStutter` keeps the resolved sink runnable for long sampled traces without changing state.

The model uses a small fixed instantiation in `main.qnt`: candidates `A`, `B`, `C`, `START_AT = 2`, `END_AT = 5`. Ballots are abstract encrypted strings keyed by candidate address. Optional `tally_result` and `enclave_computed_tally` use `{ present, value }` records, not Quint options.

The v0.3.2 tightening is represented explicitly. `ballots_at_end` is a snapshot written only by `timeAdvanceToEnd`; `b2_no_late_ballots_snapshot` checks that once `time >= end_at` and the snapshot exists, `ballots == ballots_at_end`. The enclave boundary is also explicit: `enclave_consumed_ballots`, `enclave_computed_tally`, and `enclave_has_session_key` are independent state variables written by `enclaveTally`, then projected on-chain by `publishResult`. `b10_tally_correctness_shadow` checks that the published value equals `tallySpec(ballots_at_end, candidates)` and that the enclave consumed the frozen snapshot.

Structural invariants S1 through S4, S6 through S10 are encoded as pointwise state predicates. S5 is treated as a code-discipline fact and omitted from the composite invariant. B1, B5, B6, and B7 are mostly enforced by action guards and the resolved stutter; only a simple terminal shadow is included in `all_invariants`. B8 and B9 cannot be modeled cryptographically in Quint, so B8 is represented as the classical shadow that any chain tally came from an enclave-computed tally. B9's probabilistic bound is omitted.

The IRV recursion is intentionally abstracted to `tallySpec`, a deterministic well-formed tally summary over submitted ballot keys. It preserves the schema-level conservation, per-round count sum, non-voter relation, and cross-layer equality needed for protocol checks, but it does not attempt to verify ranked-choice elimination arithmetic. That arithmetic belongs in the Lean/off-chain proof layer for `B10_lean`.

Witness invariants are negated reachability claims: resolution reachable, ballot submission reachable, and end-at crossing reachable.
