# Re-cross-critique of revised canonical — meta-analysis (2026-05-19)

After the first cross-critique caught two defects in the canonical (S8/S9 trivial encoding, dropped_voters bookkeeping gap) and the canonical was revised to address them, this round verifies the fix and checks for new defects.

## Wall-clock

| Voice | Elapsed |
|---|---|
| gpt-5.5 reviewing revised canonical | 127s |
| kimi reviewing revised canonical | 277s |

## S8/S9 fix verdict: structurally sound, both voices agree

Both reviewers verified the S8 (`s8_round_counts_sum`) and S9 (`s9_no_reappearance`) predicates match the intent's claims. kimi specifically validated the disjointness-before-accumulation ordering in S9 as the correct encoding of `j > i` (strict inequality, allowing a candidate to appear in `per_round_counts[i]` on the round they were eliminated but not afterward). Both also confirmed the `dropped_voters.size() == ballots_dropped` addition closes the gap from the first round.

## NEW defect found by the revision (kimi)

The revision introduced a state-space coverage hole. The nondet tally search in `step`:

```quint
nondet bt_tallied = oneOf(Set(0, 1, 2, 3, CANDIDATES.length()))
```

For a 5-candidate election (`CANDIDATES.length() == 5`), this gives `bt_tallied ∈ {0, 1, 2, 3, 5}` — missing 4. Combined with `well_formed_tally`'s partition equation, when exactly 4 of 5 candidates have submitted ballots, neither shape (`t_zero_round` needs `bt_tallied = 4`; `t_one_round` needs 5 voters) satisfies validation. `enclave_tally` is disabled at that state, so resolution is unreachable from any trace where exactly 4 voters submitted.

Fix applied: replaced the hardcoded set with `0.to(CANDIDATES.length()).fold(Set(), (acc, n) => acc.union(Set(n)))`, which produces `{0, 1, 2, 3, 4, 5}` for the 5-candidate case and naturally covers the full voter-count space. Comment added pointing back to this critique.

This is a clean methodology signal: re-cross-critique caught a defect the revision itself introduced. Without this round, the 4-voter coverage hole would have shipped silently and only surfaced if a future trace happened to land in that state.

## Scope warning found by gpt-5.5

`is_tally_spec_output` is named like an IRV correctness check, but it only checks chain-observable bookkeeping. Concrete witness from `quint run`: a resolved state with `per_round_counts = [{A: 3, others: 0}]` (A has strict majority) and `winners = candidate_set` passes validation, even though the IRV outcome should be `winners = {A}`. The predicate's name overstates what's checked.

Fix applied: added a `SCOPE WARNING` comment above `is_tally_spec_output` documenting that this predicate covers only the chain-observable subset of Tally_spec; IRV semantic correctness is the discharge obligation of B10_lean (intent §3.2). Predicate name retained because renaming would propagate through many call sites and the comment alone is sufficient to prevent misreading downstream.

## Remaining concerns (not applied, queued for v0.4 back-port)

**kimi's B1 ghost-variable hardening recommendation**: B1 (tally_result monotone-once-set) is currently encoded as `publish_result`'s action-guard disablement only. There is no state-side invariant of the form "if `contract_tally_result.present`, then no action mutates it." If a future action were added that writes to `contract_tally_result` without the `not(present)` guard, the model checker would have no invariant to flag the violation.

kimi's proposed fix: add a `ghost_published_tally` capturing the tally at publication, plus an invariant `if (contract_tally_result.present) then contract_tally_result == ghost_published_tally`. This is defense-in-depth, not load-bearing for current behavior, and the same pattern applies to B5/B6/B7 (all action-guard-only).

Queued for v0.4 back-port as a methodology pattern: when a behavioral invariant could be encoded as either action-guard or ghost-variable+state-predicate, prefer the latter as defense against action-set drift.

## Methodology observation

The re-cross-critique workflow:
1. Cross-critique finds defects → 2. Apply fixes → 3. Re-cross-critique → 4. Catches new defects introduced by the fix

Worked end-to-end this round. Step 4 caught a real coverage hole that step 2 added. The marginal cost of running step 3 is small (~5 minutes wall-clock); the marginal information value is high (one real defect plus one scope-name correction).

For v0.4: re-cross-critique should be a documented step after every canonical revision that touches load-bearing predicates. The pattern is "synthesize → fix → re-attack until stable."

## Final canonical state

`specs/rcv.qnt` now has:

- TallyResult matching intent §2.5 schema (`per_round_counts`, `eliminated_by_round`, no synthetic `rounds_played`)
- Real S8 and S9 predicates (not bounds)
- `dropped_voters.size() == ballots_dropped` consistency check
- Full `bt_tallied` range coverage in `step` nondet search
- Documented scope of `is_tally_spec_output` (chain-observable subset only; IRV semantics is B10_lean's job)

typecheck=0, all_invariants holds, 3/3 witnesses violated. Ready for downstream verification work (#17 Lean spec composes off this canonical; #18 verify pyramid if applicable; #19 integration ledger).
