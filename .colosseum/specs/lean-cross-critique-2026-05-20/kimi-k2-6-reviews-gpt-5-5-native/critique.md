# Cross-critique: kimi-k2-6 reviews gpt-5-5-native

## Q1. Most material structural divergence

The most material divergence is in the encoding of `s9_no_reappearance` (elimination monotonicity).

- **Target (gpt-5-5-native):** Uses `Option` pattern matching:
  ```lean
  t.eliminated_by_round[i]? = some eliminated →
  t.per_round_counts[j]? = some rc →
  ...
  ¬ roundMentions c rc
  ```
  Bounds checking is internalized into the theorem hypotheses via `List.get?` equality to `some`.

- **Mine (kimi-k2-6):** Uses partial list indexing with `!`:
  ```lean
  i < j → j < t.per_round_counts.length → i < t.eliminated_by_round.length →
  ∀ c ∈ t.eliminated_by_round[i]!, ...
  ```
  Bounds are explicit separate hypotheses, and the proof relies on the partial function `List.get!` being guarded by those hypotheses.

**Why this matters:** The target's encoding is more robust. It does not depend on a partial function (`!`) whose well-definedness is separated from its use site. In the target, the fact that index `i` is valid is encoded directly as the existence of a witness `eliminated` such that `get?` returns `some eliminated`. This makes the theorem statement self-contained and less fragile during proof development. If a future refactor changes the length hypotheses, the target's statement remains well-typed and meaningful, whereas mine could produce an out-of-bounds panic in the proof term. The divergence is material because it affects the safety and maintainability of the proof obligation, not just its logical content.

## Q2. Apparent defect in target spec

I find no defect. The target spec typechecks cleanly, `EnclaveImage` is declared as an `axiom` (not a `def` equal to `Tally_spec`), `Tally_spec` is a transparent `def` that properly composes `decrypt_and_validate` and `IRV_spec` while threading Stage-1 bookkeeping, all five required theorems are present with `sorry` bodies (not `rfl` or `trivial`), and each theorem statement appears to faithfully encode the corresponding intent obligation (S6, S7, S8, S9, B10_lean). The `Option`-based indexing in S9 is arguably an improvement over partial indexing, not a defect.

## Q3. Change to your own spec after reading target

I would rewrite my `s9_no_reappearance` to use `Option` pattern matching instead of `!` indexing. Concretely:

```lean
theorem s9_no_reappearance
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    let t := Tally_spec raw cs pk
    ∀ (i : Nat) (eliminated : List Addr) (j : Nat) (rc : RoundCounts) (c : Addr),
      t.eliminated_by_round[i]? = some eliminated →
      t.per_round_counts[j]? = some rc →
      i < j →
      c ∈ eliminated →
      ¬ (∃ n, (c, n) ∈ rc) := by
  sorry
```

The target encoded this more cleanly by avoiding partial functions entirely. I would also consider extracting `roundTotal` and `roundMentions` as named helpers (as the target did) to make the theorem statements more readable, though this is secondary to the indexing change.

## Optional notes

Both specs share the same limitation: `CandidateSet := List Addr` does not encode the S2 distinctness invariant at the type level. If `cs` contains duplicates, `cs.length` overcounts relative to the set interpretation used in `non_voters`, which could in principle make `s7_voter_partition` unprovable without a distinctness hypothesis. The intent treats distinctness as S2 (a separate structural invariant), so this omission is consistent with the intent's design, but it is a shared gap.

The target's use of a `RawBallot` struct (vs my `Addr × Bytes` pair) is a stylistic difference with negligible methodological impact. Both specs correctly stay stdlib-only and avoid Mathlib, which is appropriate for this layer.

STATUS: ok
