# Cross-critique: gpt-5-5-native reviews magistral-medium-native

## Q1. Most material structural divergence

The most material divergence is the Stage 1 / Stage 2 boundary. My spec gives `decrypt_and_validate` a `DecryptedSet` containing `valid`, `dropped_voters`, and `non_voters`, and gives `IRV_spec` a narrower `IRVResult` containing only `winners`, `per_round_counts`, `eliminated_by_round`, and `ballots_tallied`. The target has `DecryptedSet` with only `valid` and `dropped`, while `IRV_spec` returns a full `TallyResult`.

This matters because §2.5 assigns `non_voters` to Stage 1: `non_voters := candidates \ raw_ballots.keys`, emitted in candidate-declaration order. Stage 2 is specified as the IRV recursion over already-valid ballots and candidates, producing only the IRV fields plus `ballots_tallied`. Letting `IRV_spec` produce `non_voters` moves voter-accounting across the stage boundary and weakens the claim that `Tally_spec` is the transparent composition described in §2.5.

## Q2. Apparent defect in target spec

The target `Tally_spec` does not thread `non_voters` from Stage 1. Its `DecryptedSet` has no `non_voters` field, and the final record update only overrides `dropped_voters` and `ballots_dropped`:

```lean
{ r with
  dropped_voters  := d.dropped
  ballots_dropped := d.dropped.length }
```

Therefore `t.non_voters` in the final `TallyResult` is whatever opaque `IRV_spec d.valid candidates` returned. That is semantically wrong against §2.5, which says `non_voters` comes from Stage 1 and equals candidates without raw ballot keys. The theorem `s7_voter_partition` still typechecks because it is a `sorry`, but its statement is attached to a `Tally_spec` whose source of `non_voters` is not the intended source.

No tautological-shadow defect is present: `EnclaveImage` is an `axiom`, and `B10_lean` is not discharged by `rfl`, `trivial`, or `True.intro`.

## Q3. Change to your own spec after reading target

I would consider changing my `RoundCounts := List (Addr × Nat)` to the target's explicit `RoundCount` structure with `candidate` and `count` fields. The target's encoding makes theorem statements like `s8_round_counts_sum` and `s9_no_reappearance` more readable than tuple projections. This is a readability improvement only; I would keep my Stage 1 / Stage 2 result split.

## Optional notes

Both specs use plain lists for candidate sets, winners, dropped voters, and per-round count maps. That keeps the Lean file stdlib-only and close to Borsh `Vec` serialization, but it leaves distinctness, map uniqueness, and declaration-order discipline as comments or external theorem obligations rather than type-level invariants. This is probably the right level for this convergence pass, but it is an encoding-discipline axis worth recording.

The target's use of `t.eliminated_by_round[i]!` and `t.per_round_counts[j]!` is acceptable because the theorem includes bounds. My `Option`-based indexing avoids default-value semantics in the statement, but the target's bounded form is also inspectable.
