# Cross-critique: kimi-k2-6 reviews magistral-medium-native

## Q1. Most material structural divergence

The target conflates Stage 2 output with the full `TallyResult` by typing `IRV_spec` as returning `TallyResult` directly, rather than a separate Stage-2-specific type (e.g., `IrvOutput`). In the target, `IRV_spec` must populate every `TallyResult` field, including `non_voters`, `dropped_voters`, and `ballots_dropped`, even though those are Stage-1 bookkeeping concepts. The intent (Section 2.5) explicitly separates Stage 2 as the "combinatorial IRV core" producing only `(winners, per_round_counts, eliminated_by_round, ballots_tallied)`, with `Tally_spec` assembling the full result by threading Stage-1 outputs.

This matters because it blurs the methodology's intended proof decomposition: a future proof author should be able to verify Stage-2 correctness (the IRV algorithm) independently of Stage-1 decryption/validation. By making `IRV_spec` return the full `TallyResult`, the target forces Stage-2 to be concerned with fields it has no business knowing about, and makes it impossible for `Tally_spec` to act as a transparent assembly layer.

Compounding this, the target's `DecryptedSet` omits the `non_voters` field that the intent says Stage 1 produces. So `Tally_spec` has no `non_voters` value from Stage 1 to thread into the result.

## Q2. Apparent defect in target spec

The target's `Tally_spec` does not override the `non_voters` field:

```lean
def Tally_spec ... :=
  let d := decrypt_and_validate raw candidates privkey
  let r := IRV_spec d.valid candidates
  { r with dropped_voters := d.dropped, ballots_dropped := d.dropped.length }
```

Since `non_voters` is not in the `with` override, it is inherited unchanged from `r` (the output of `IRV_spec`). But `IRV_spec` only receives `d.valid` and `candidates`; it has no access to `raw_ballots.keys`, which is required to compute `non_voters = candidates \ raw_ballots.keys` per the intent's Stage 1 definition. If `IRV_spec` attempts to approximate `non_voters` as `candidates \ d.valid.keys`, it would misclassify dropped voters as non-voters (a candidate who submitted a malformed ballot is in `raw_ballots.keys` but not in `valid.keys`, so they are a dropped voter, not a non-voter).

This means `s7_voter_partition` makes a conservation claim about a `non_voters` value that `Tally_spec` does not actually source from the correct Stage-1 computation. The theorem is not vacuous (it is a meaningful claim about the opaque `IRV_spec`), but the composition is semantically wrong with respect to the intent's Stage-1/Stage-2 separation.

## Q3. Change to your own spec after reading target

Adopt the target's `RoundCount` named-struct approach instead of my `Addr × Nat` tuple for `RoundCounts`. In the target, `rc.candidate` and `rc.count` are self-documenting, whereas my `p.fst` and `p.snd` in `s8_round_counts_sum` and `s9_no_reappearance` require the reader to remember tuple ordering. The struct adds clarity without changing the mathematical content.

## Optional notes

Both specs share a secondary gap: neither axiomatizes that `ballots_tallied = d.valid.length`, so the S8/S9 theorems rely on an opaque `ballots_tallied` value whose relationship to the actual valid ballot count is unstated. This is less severe than the target's `non_voters` issue because `ballots_tallied` is at least produced by the correct stage (Stage 2).

The intent's Stage-1/Stage-2 separation is load-bearing for the methodology's proof decomposition. The target's design would make it harder to state and prove a "Stage-2 correctness" lemma in isolation, because Stage 2 is entangled with the full `TallyResult` schema.

STATUS: ok
