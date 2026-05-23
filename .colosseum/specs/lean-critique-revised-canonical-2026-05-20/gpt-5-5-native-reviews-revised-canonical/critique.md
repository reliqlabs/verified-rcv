# Re-critique of revised canonical: gpt-5-5-native

## Q1. Is the fix structurally sound?

Yes. The revised canonical structurally addresses both cross-critique findings.

`IRVResult` is now a distinct Stage-2 structure with only `winners`, `per_round_counts`, `eliminated_by_round`, and `ballots_tallied`. `IRV_spec` returns `IRVResult`, not `TallyResult`, so Stage 2 no longer appears to own Stage-1 voter bookkeeping.

`DecryptedSet` now includes `non_voters`. `Tally_spec` computes `d := decrypt_and_validate raw candidates privkey`, computes `r := IRV_spec d.valid candidates`, then constructs the final `TallyResult` with IRV fields from `r`, `ballots_dropped := d.dropped.length`, `dropped_voters := d.dropped`, and `non_voters := d.non_voters`. That directly encodes the §2.5 composition split.

`s9_no_reappearance` uses `t.eliminated_by_round[i]? = some eliminated` and `t.per_round_counts[j]? = some rc`, with `i < j` retained. There is no remaining `[i]!` partial-index/default-value dependency in the theorem body.

## Q2. Did the revision introduce any new defects?

No new defect found. The refactor preserves all published fields of `TallyResult`, and each field in `Tally_spec` has the intended source after the split. In particular, `t.non_voters` in `s7_voter_partition` is now definitionally the Stage-1 `d.non_voters`, not an opaque inherited field from `IRV_spec`.

The new S9 statement keeps the strict later-round relation `i < j`, and both indexed hypotheses are Option-pattern witnesses, so there is no off-by-one regression. `IRVResult`, `TallyResult`, and `DecryptedSet` derive `Inhabited`, which is appropriate for the opaque-return use sites.

## Q3. Remaining concerns

The most material remaining under-encoding is `ballots_tallied = d.valid.length`. Intent §2.5 states Stage 2 returns `ballots_tallied := |valid_ballots|`, and §2.5 composition threads that field into `TallyResult`. The revised Lean file lets `IRV_spec` return an arbitrary `ballots_tallied` because it is opaque, then S7 and S8 reason over that arbitrary number.

This is not a regression from the revision, but it is load-bearing for the math layer. Without an inspectable obligation tying `r.ballots_tallied` to `d.valid.length`, the Stage-1 partition and Stage-2 count can drift while the top-level composition still typechecks. A future canonical should add a theorem such as `irv_ballots_tallied` or make `Tally_spec.ballots_tallied := d.valid.length` if the design wants Stage 1 to own that count.

## Optional notes

The revised canonical is better than my fan-out spec in one small axis: `RoundCount` is a named structure instead of a pair, so S9 can state key absence as `entry.candidate ≠ c` without relying on pair projection or helper predicates. That improves readability while staying stdlib-only.

STATUS: ok
