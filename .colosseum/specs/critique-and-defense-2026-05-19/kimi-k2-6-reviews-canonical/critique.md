# Cross-critique: kimi-k2-6 reviews canonical

## Q1. Most material structural divergence

The canonical spec drops `per_round_counts` and `eliminated_by_round` from the `TallyResult` type entirely, replacing them with a single synthetic `rounds_played: int`. The intent's §2.5 `TallyResult` schema explicitly lists both fields as load-bearing for auditability, and §3.1 invariants S8 (per-round count consistency) and S9 (elimination monotonicity) are defined over them. By omitting the fields, the canonical spec cannot express S8 or S9 as stated in the intent; it substitutes weaker checks on `rounds_played` that do not capture the same property. This is not a modeling simplification (like using `Set[Addr]` instead of `Vec<Addr>`); it is a structural omission that removes two intent-mandated invariants from the checkable state space.

A secondary but related divergence is the canonical's use of a `pure def derived_phase` function instead of a stored `phase` variable. This is actually more faithful to the intent (which calls phase "derived" from `env.block.time` and `tally_result`), and it eliminates an entire class of phase-consistency bugs that my stored-variable approach admits.

## Q2. Apparent defect in canonical

The canonical spec's `well_formed_tally` predicate omits the check `t.dropped_voters.size() == t.ballots_dropped`. The intent's §2.5 `TallyResult` schema defines `dropped_voters: Vec<Addr>` with the invariant `len(dropped_voters) = ballots_dropped` (implied by the schema and reinforced by §3.1 S7's set-relation clauses). In the canonical spec, `enclave_tally` constructs tallies with `dropped_voters: Set()` unconditionally, while `ballots_dropped` is drawn from `oneOf(Set(0, 1))`. A state with `ballots_dropped = 1` and `dropped_voters = Set()` therefore passes `well_formed_tally` and `is_tally_spec_output`, is published via `publish_result`, and is accepted as a reachable state — despite violating the intent's structural requirement that every dropped ballot be accounted for by a voter address. The typechecker cannot catch this because `size()` and `ballots_dropped` are both `int`-typed and the predicate simply omits the equality.

## Q3. Change to your own spec after reading canonical

I would replace my stored `phase` variable with a `pure def derived_phase` function computed from `time`, `start_at`, `end_at`, and `tally_result.present`, matching the canonical's approach. The intent explicitly describes phase as derived (§2.5: "Derived states (from `env.block.time` and `tally_result`)"), not as stored contract state. Eliminating the stored variable removes a source of divergence between `phase` and the variables it is supposed to track, and it collapses my separate `timeAdvanceToStart` / `timeAdvanceToEnd` actions into a single `tick` action that is both simpler and more faithful to the intent's implicit time model.

## Optional notes

The canonical's nondeterministic `enclave_tally` action (generating tally shapes and filtering via `is_tally_spec_output`) is a better encoding strategy than my deterministic `trivialTally`. It lets the model checker explore the space of well-formed tallies without requiring a full IRV implementation in Quint, which is appropriate for a protocol-layer spec whose job is to check chain-side invariants, not to verify the tally algorithm itself (that is B10_lean's job).

The canonical's `ghost_ballots_at_end_at` ghost-variable discipline for B2 is exactly what the intent v0.3.2 A3 requires, and my spec's `ballots_at_end` + `end_snapshot_taken` achieves the same effect with slightly different naming. No material difference there.

One place where the canonical is weaker than my spec: my `s7_tally_conservation` invariant checks the full partition equation `ballots_tallied + ballots_dropped + non_voters.length() == candidates.length()`, while the canonical's `well_formed_tally` checks the same equation. Both are correct; the canonical's version is embedded in a predicate rather than exposed as a standalone invariant, which is a stylistic choice rather than a structural gap.

STATUS: ok
