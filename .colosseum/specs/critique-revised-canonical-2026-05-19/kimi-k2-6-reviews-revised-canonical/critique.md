# Re-critique of revised canonical: kimi-k2-6

## Q1. Is the S8/S9 fix structurally sound?

**Yes.** The revision correctly restores the intent-mandated predicates.

- **S8 (`s8_round_counts_sum`)**: folds over `per_round_counts` and asserts `sum_round(m) == t.ballots_tallied` for each round map `m`. This exactly matches the intent clause "each `per_round_counts[i]` has all entries summing to `ballots_tallied`".

- **S9 (`s9_no_reappearance`)**: walks rounds in order with an accumulator of already-eliminated candidates. At round `i` it checks that the keys of `per_round_counts[i]` are disjoint from the eliminated-so-far set, then adds `eliminated_by_round[i]` to the accumulator. This correctly encodes: a candidate eliminated at round `i` may appear as a key in `per_round_counts[i]` (the round they were eliminated) but cannot appear in any `per_round_counts[j]` for `j > i`. The ordering of the check (disjointness before accumulation) is the correct encoding of the intent's strict inequality `j > i`.

- **`dropped_voters.size() == ballots_dropped`**: added to `well_formed_tally` as required. The prior canonical allowed `ballots_dropped = 1` with an empty `dropped_voters` set; that gap is now closed.

No defects found in the S8/S9/dropped_voters fix itself.

## Q2. Did the revision introduce any new defects?

**Yes: a state-space gap when exactly 4 candidates submit ballots.**

The revised `step` action generates two nondet tally shapes for `enclave_tally`:

- `t_zero_round`: `ballots_tallied = bt_tallied` where `bt_tallied ∈ {0, 1, 2, 3, 5}`. `well_formed_tally` enforces conservation `ballots_tallied + ballots_dropped + non_voters.size() == candidates.size()`. With `ballots_dropped = 0` and `non_voters.size() = 5 - |voters|`, this requires `bt_tallied == |voters|`.
- `t_one_round`: `ballots_tallied = 0`, which requires `|voters| == 5` to satisfy conservation.

Therefore:
- `|voters| ∈ {0, 1, 2, 3, 5}`: at least one shape is valid, `enclave_tally` can fire, resolution is reachable.
- `|voters| = 4`: neither shape satisfies conservation (`bt_tallied = 4` is not in the nondet set, and `t_one_round` requires 5 voters). `enclave_tally` is disabled for all generated `t`, so `publish_result` can never fire.

This is a modeling artifact introduced by the revision's bounded nondet tally search. The prior canonical (v11) used `trivialTally` which always produced a conservation-satisfying tally. The revised canonical's shape enumeration is richer (it exercises S9 via `t_one_round`), but the omission of `bt_tallied = 4` from the `t_zero_round` search set creates a coverage hole. The witness checks still pass because the model checker samples traces with other voter counts, but the spec cannot reach resolution from any state where exactly 4 of 5 candidates voted.

**Fix**: add `4` to the `bt_tallied` nondet set, or generate `bt_tallied` dynamically as `ghost_ballots_at_end_at.keys().size()` instead of sampling from a hardcoded set.

## Q3. Remaining concerns

The single most material remaining concern is **B1 (tally_result monotone-once-set) encoded as action-guard disablement only, with no state-side check.**

The revised canonical relies entirely on `publish_result`'s guard `not(contract_tally_result.present)` to ensure the tally is written at most once. There is no state invariant of the form "if `contract_tally_result.present`, then no action in the current step mutates it." If a future action were added (or an existing action were modified) that writes to `contract_tally_result` without the `not(present)` guard, the model checker would have no invariant to flag the violation.

By contrast, B2 is encoded as a checkable state invariant (`inv_b2_ballots_at_end_at_frozen`) over a ghost snapshot variable, per the v0.3.2 encoding discipline. B1 deserves the same treatment: a ghost variable `ghost_published_tally` (or equivalent) capturing the tally at the moment of publication, plus an invariant `if (contract_tally_result.present) then contract_tally_result == ghost_published_tally`, would provide defense-in-depth against action-set drift.

B6 (ballot-writer-equals-voter) is also unencoded as a state predicate, but its violation scope is narrower (a single ballot slot is corrupted) compared to B1 (the entire election result is mutable).

## Optional notes

- The revised canonical's use of `derived_phase` as a pure function (rather than a stored `phase` variable) is cleaner than v11's approach and eliminates a class of phase-desynchronization bugs.
- The `registry` state variable and `image_registration_honest` predicate are a genuine improvement over v11; they make B8's classical-Prop shadow checkable rather than vacuously `true`.
- Witness coverage could be strengthened by adding `witness_close_and_tally_fires` and `witness_enclave_tally_fires` to ensure the idempotent and off-chain actions are actually exercised, not just skipped on all sampled traces.

STATUS: ok
