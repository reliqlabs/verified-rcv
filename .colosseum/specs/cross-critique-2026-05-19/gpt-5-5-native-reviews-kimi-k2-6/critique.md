# Cross-critique: gpt-5-5-native reviews kimi-k2-6

## Q1. Most material structural divergence

The most material divergence is `b8_attestation_shadow`. My spec encodes a checkable classical shadow, `if (tally_result.present) enclave_computed_tally.present else true`, and includes it in `all_invariants`. The target defines `val b8_attestation_shadow = true` and does not include it in `all_invariants`.

This matters because B8 is the protocol-side bridge between the published chain value and the off-chain attested computation. Quint cannot model TDX, zkdcap, or SHA-256 soundness directly, but the intent still expects a classical shadow rather than a tautology. The target's `publishResult` guard does enforce `enclave_computed_tally.present`, so reachable traces still have the desired shape. But the named invariant itself claims nothing, which weakens the spec as a verification artifact and makes drift in `publishResult` invisible to `all_invariants`.

## Q2. Apparent defect in target spec

Defect: `b8_attestation_shadow = true` is a tautological behavioral invariant. The design notes say B8 is represented as the shadow that any chain tally came from an enclave-computed tally, but the code does not encode that predicate. A checkable form would be at least `if (tally_result.present) enclave_computed_tally.present else true`, preferably also tying the published value to `enclave_computed_tally.value`. Typechecking and sampled invariant runs cannot detect this because `true` is invariant under every transition.

## Q3. Change to your own spec after reading target

I would replace my `s9_elimination_monotonicity` with the target's indexed shape rather than `tally_result.value.eliminated_by_round.length() == 0`. My predicate is only valid because my current `tallySpec` never emits eliminations. The target's version better matches intent S9: if a candidate is eliminated in round `i`, later `per_round_counts[j]` must not contain that candidate. That predicate would survive a future nontrivial IRV tally model.

## Optional notes

The target's `witness_end_at_crossing = not(phase == "Tallying")` witnesses reachability of the Tallying phase, not specifically the snapshot condition `time >= end_at and end_snapshot_taken`. It is acceptable for the named end crossing in this model because `timeAdvanceToEnd` is the only Tallying entry, but the witness is less precise than it could be.
