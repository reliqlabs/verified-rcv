# Re-cross-critique of revised Lean canonical — meta-analysis (2026-05-20)

After the cross-critique caught a composition defect (Stage 1/2 conflation; non_voters not threaded) and an encoding upgrade (Option-pattern for S9) and the canonical was revised, this round verifies the fix and checks for new defects.

## Wall-clock

| Voice | Elapsed | Critique size |
|---|---|---|
| magistral-medium-native | 44s | 1775 bytes |
| gpt-5-5-native | 109s | 2830 bytes |
| kimi-k2-6 | 260s | 3460 bytes |

## Fix verdict: structurally sound, all 3 voices agree

All three reviewers verified the revision correctly addresses both cross-critique findings:

- `IRVResult` is a distinct Stage-2 structure with only `(winners, per_round_counts, eliminated_by_round, ballots_tallied)`. `IRV_spec` returns `IRVResult`, not `TallyResult`.
- `DecryptedSet` now carries `non_voters : List Addr` from Stage 1.
- `Tally_spec` composes correctly: Stage-2 fields from `r : IRVResult`; Stage-1 fields (`ballots_dropped`, `dropped_voters`, `non_voters`) from `d : DecryptedSet`; `ballots_dropped` computed as `d.dropped.length`.
- `s9_no_reappearance` uses `[i]? = some ...` Option pattern with `i < j` retained; no `[i]!` remains.

All three reviewers also confirmed no new defects introduced by the refactor. The methodology pattern worked: re-cross-critique after a canonical revision caught no regressions, while the fix correctly closed the prior defects.

## Convergent remaining concern: CandidateSet distinctness

Two of three reviewers (kimi, magistral) picked the same most-material remaining concern as their Q3:

`CandidateSet := List Addr` does not encode S2's distinctness at the type level. Concrete consequence (kimi's argument): `s7_voter_partition` claims `ballots_tallied + ballots_dropped + non_voters.length = cs.length`. If `cs` contains duplicates, `cs.length` overcounts and the conservation equation fails even for honest Stage 1 and Stage 2. The theorem as stated is too strong — it claims the equation holds for all `cs` including duplicate-containing ones, which doesn't match the intent's qualified claim.

Fix options:
- Add a `cs.Nodup` hypothesis to S6, S7, S8, S9 (lightweight, no type change)
- Bundle `CandidateSet` as `{ cs : List Addr // cs.Nodup }` (heavier, but distinctness becomes a type invariant)
- Encoding-discipline note in intent v0.3.3 that downstream Lean specs MUST require `Nodup` on `CandidateSet`

## Second remaining concern: ballots_tallied axiomatization (gpt-5.5)

gpt-5.5 picked a different Q3: there is no constraint linking `r.ballots_tallied` (where `r := IRV_spec d.valid candidates`) to `d.valid.length`. Per intent §2.5, Stage 2 returns `ballots_tallied := |valid_ballots|`, but `IRV_spec` is opaque, so `r.ballots_tallied` can be any `Nat` while the composition still typechecks.

Fix options:
- Add a Stage-2 correctness axiom: `axiom irv_ballots_tallied : ∀ valid cs, (IRV_spec valid cs).ballots_tallied = valid.length`
- Or have `Tally_spec` set `ballots_tallied := d.valid.length` directly (overriding the IRV output), which makes Stage 1 own the count
- Encoding-discipline note in intent v0.3.3 that this Stage-2 obligation must be axiomatized

Both remaining concerns are intent-level under-specifications surfaced by the cross-critique cycle, not canonical-spec bugs.

## kimi's bonus encoding-discipline candidate

In optional notes, kimi flagged a length relation between `per_round_counts` and `eliminated_by_round` that is implicit in §3.1's worked example but not encoded:

`eliminated_by_round.length = per_round_counts.length - 1` (typical case)
`0 = 1 - 1` (all-abstain boundary case)

A well-formedness predicate encoding this could close a small gap in S9's preconditions and the structural reading of round counts. Candidate for intent v0.3.3 or a v0.4 ask.

## Methodology observations

1. **Ask R (re-cross-critique) cycle worked end-to-end on Lean**: the cross-critique caught a real composition defect; the fix was applied; the re-cross-critique verified the fix was sound AND surfaced 2 new intent-level gaps. No regressions introduced. Same pattern as the Quint Ask R cycle (2026-05-19) which caught a state-space coverage hole. Consistent evidence that the post-revision check is load-bearing.

2. **Voice-count economy**: 3 voices producing the re-cross-critique gave converging signal on the most material remaining concern (2/3 agreement on CandidateSet distinctness). Smaller panel sufficient when the divergence axis is intent-level.

3. **magistral handled tool-use correctly this time**: critique.md was written to file successfully (44s, smallest critique but on-task). The prior dispatch's stdout-only failure mode did not recur. May have been a context-window or stage-of-conversation effect rather than a stable voice limitation.

4. **Convergent intent-level concerns are Ask Q-shaped**: both remaining concerns (CandidateSet distinctness, ballots_tallied axiomatization) are convergent across voices on the same axis (Stage 2 under-specification of its output relation to Stage 1). Per Ask Q (intent-tightening on encoding-discipline as the convergence lever), these are candidates for an intent v0.3.3 patch with new encoding-discipline notes, rather than spec-level patches.

## Punch list

In order of materiality:

1. **Intent v0.3.3 PATCH candidate**: add encoding-discipline notes A4 (CandidateSet distinctness — downstream Lean specs MUST require `Nodup` on `CandidateSet`) and A5 (Stage 2 `ballots_tallied` axiom — must be tied to `valid.length`). Convergent across voices; matches Ask Q methodology.
2. **Canonical patch** (optional, applies whether or not intent is patched): add `Nodup cs` hypothesis to S6–S9 theorem statements; add `irv_ballots_tallied` axiom. Lightweight, no type signatures change.
3. **kimi's length-relation candidate** (queued for v0.4 or intent v0.3.3): encode `eliminated_by_round.length = per_round_counts.length - 1` (with all-abstain boundary case) as a well-formedness predicate.

## Final canonical state

`specs/RcvSpec.lean` now has:

- Distinct `IRVResult` (Stage 2 output) and `TallyResult` (composed)
- `DecryptedSet` includes `non_voters` from Stage 1
- `Tally_spec` transparently assembles Stage 1 + Stage 2 outputs
- `s9_no_reappearance` uses Option-pattern indexing
- 5 theorems with `sorry` bodies; `EnclaveImage` as axiom; no tautological shadows

Lean typecheck: 0 errors, 5 sorries (expected). Ready for downstream proof work (B10_lean discharge by a Lean-specialist model when Leanstral lands; #19 integration ledger entry per `compose-ledger.md` schema).
