# Cross-critique: kimi-k2-6 reviews gpt-5-5-native

## Q1. Most material structural divergence

The target includes checkable classical-shadow predicates for **B7** (`b7_terminal_shadow`) and **B8** (`b8_attestation_shadow`) inside `all_invariants`, while my spec trivializes both to `true`. This is the most material divergence because it directly changes what the composite invariant claims about cross-layer and terminal behavior.

- `b7_terminal_shadow` in the target checks `phase == "Resolved" ⇒ tally_result.present`. This is a runtime-checkable consequence of B7 (Resolved is a sink) that my model omits entirely.
- `b8_attestation_shadow` in the target checks `tally_result.present ⇒ enclave_computed_tally.present`. This encodes the classical shadow of B8: any chain-side tally must have been preceded by an enclave-side computation. My spec leaves B8 as `true`, collapsing the chain↔enclave boundary that the v0.3.2 encoding discipline (A2) explicitly requires.

The target’s `all_invariants` is therefore strictly stronger and more faithful to the intent’s cross-layer composition structure. My spec would accept a state where `phase == "Resolved"` but `tally_result.present == false` (if such a state were reachable through a future action change), or where `tally_result` appears without `enclave_computed_tally`. The target catches both.

## Q2. Apparent defect in target spec

**S9 (`elimination_monotonicity`) is trivially encoded as a tautology.**

The target defines:
```
val s9_elimination_monotonicity = if (tally_result.present) tally_result.value.eliminated_by_round.length() == 0 else true
```

Since `tallySpec` always returns `eliminated_by_round: List()`, this predicate is always `true` and never exercises the structural constraint the intent actually requires: "if a candidate appears in `eliminated_by_round[i]`, they appear in no `per_round_counts[j]` for `j > i`." The typechecker cannot catch this because the expression is syntactically valid and well-typed; it is semantically vacuous.

The defect matters because S9 is listed as a load-bearing structural invariant in §3.1 of the intent. If the tally function is ever refined (e.g., in a follow-up spec round that replaces `tallySpec` with a real IRV implementation), the target’s S9 will silently pass even when elimination sequences violate monotonicity. A faithful encoding should express the full constraint, even if it is vacuously satisfied by the current trivial tally.

## Q3. Change to your own spec after reading target

I would add the **B7 terminal shadow** and **B8 attestation shadow** to my `all_invariants` composite, replacing their current `true` definitions:

```
val b7_terminal_shadow = if (phase == "Resolved") tally_result.present else true
val b8_attestation_shadow = if (tally_result.present) enclave_computed_tally.present else true
```

The target encoded these better because it recognized that B7 and B8, while temporal/cross-layer in the intent, admit meaningful classical-Prop shadows that can be checked as state predicates. Including them makes the protocol model observably stronger without requiring LTL support. I would keep my more detailed S9 encoding (which attempts the full monotonicity check) but adopt the target’s B7/B8 shadow discipline.

## Optional notes

Both specs correctly implement the v0.3.2 encoding disciplines A2 (enclave-side state for B10 projection) and A3 (B2 over a snapshot variable). The target’s `witness_end_at_crossing` uses `not(time >= end_at and end_snapshot_taken)` rather than my phase-based witness; both are valid reachability negations, though the target’s is slightly more precise because it witnesses the crossing condition directly rather than the phase label.

The target omits S5 from `all_invariants` per its design notes, treating it as a handler-set meta-property. This is defensible under the intent’s §3.1 preamble amendment, but it means the Quint model does not check write-discipline at runtime. A future round might add a ghost variable tracking whether `tally_result` has ever been written, to make S5 checkable without full LTL.

STATUS: ok
