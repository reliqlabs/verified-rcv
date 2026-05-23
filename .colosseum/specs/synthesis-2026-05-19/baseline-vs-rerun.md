# Baseline rerun comparison — same intent, fresh sessions

Round 3a #46. Tests whether fresh agent sessions (no prior history) converge more than the first run.

## Setup

Same three target voices as first round, same intent (v0.3.1 unchanged), fresh opencode sessions per voice. claude-opus-4-7 hit an unrelated opencode config bug (`temperature` parameter sent to a model that rejects it) and was abandoned in 13s total; comparison reduces to two voices.

| Voice | Run 1 | Run 2 (v10) | rcv.qnt Δ |
|---|---|---|---|
| kimi-k2-6 | v4-agent-iterate, 270L | v10-baseline-rerun, 280L | +10L (+3.7%) |
| gpt-5.5-native | v8-gpt-5-5-retry, 168L | v10-baseline-rerun, 148L | -20L (-12%) |

## Same-voice variance (fresh session, same intent)

### kimi v4 → kimi v10

State variables:
- v4: `initialized: bool` plus runtime `instantiate(set, s, e)` action that sets `candidates`/`start_at`/`end_at`
- v10: explicit `phase: str` string-enum state with values `"Uninitialized"/"Created"/"Voting"/"Tallying"/"Resolved"`, plus separate `timeAdvanceToStart`/`timeAdvanceToEnd` actions
- Both keep `candidates`/`start_at`/`end_at` as `var`, both run instantiate as a transition

Type model:
- v4: 4-field `TallyResult` (winners, ballots_tallied, ballots_dropped, non_voters)
- v10: 7-field `TallyResult` adds per_round_counts, eliminated_by_round, dropped_voters

Invariant content:
- v10 added real per-round count consistency (S8) and elimination monotonicity (S9) predicates, computed over the new TallyResult fields. v4 had these omitted with explanatory comments.
- Both still classical shadows on B1, B2, B4-B10 (mostly `true`).
- Both partition equation on S7.

Verdict: same lineage, more state and more predicates in v10. Roughly +10L. Structural choices on what gets shadowed vs predicate-checked stayed similar. The new `phase: str` introduces a different time-advance shape but does not change which invariants are real vs shadow.

### gpt-5.5 v8 → gpt-5.5 v10

State variables:
- v8: `ballots: str -> str` (Map of ciphertexts), `ballots_at_end: str -> str` (B2 snapshot), `last_action: str` (ghost tag), `tally_result: OptTally`
- v10: `ballots: Set[Addr]` (just voter set, no payload), no ballots_at_end, no last_action, has `close_events: int` counter (weird), `tally_result: MaybeTally`

Type model:
- v8: 5-field TallyResult, payload was opaque ciphertext
- v10: 4-field TallyResult, voter set with no payload

Invariant content:
- v8 had behavioral_shadows checking `if (now >= END_AT) ballots == ballots_at_end`. v10 dropped this entirely.
- v8 had `tally_well_formed` predicate with full S7+S8 checks. v10 keeps S6/S7/S10 with proper predicates; S8/S9 are now trivial `true`.
- v10 still partition equation on S7.
- v10 added `resolved_stutter` action (same as v8 had).

Verdict: same voice, simpler in v10. The B2 anchor (`ballots_at_end`) and the ciphertext payload both got dropped. This is genuinely worse on the invariant-coverage axis. Same model, same intent, fresh session: gpt-5.5 v10 made different abstraction-layer choices than v8.

## Cross-voice convergence in v10 (the actual experiment)

| Divergence (from synthesis diff) | v10 kimi | v10 gpt-5.5 | Same? |
|---|---|---|---|
| A1 Param shape (const vs var) | hybrid: `const` declared, but copied to `var` in init | `const` only | partial: kimi still hybrid |
| A2 Enclave layer modeling | absent (shadow only) | absent (no enclave state) | yes, both absent |
| A3 B2 frozen-ballots encoding | shadow `true` | dropped entirely | yes, both absent |
| A4 Tally search vs pin | pin to `trivialTally(cands)` | pin to `fallbackTally` | yes, both pin |
| B1 S7 partition equation | partition ✓ | partition ✓ | yes |
| B2 resolved_stutter action | absent (relies on phase guard) | present | no |
| B3 ballot type | `Map[str, str]` (with payload) | `Set[Addr]` (no payload) | no |

## Run 1 vs Run 2: did convergence move?

Cross-voice comparison on the load-bearing divergences:

| Divergence | Run 1 (v4/v8/handcraft) | Run 2 (v10) | Convergence delta |
|---|---|---|---|
| A1 params | 1 var (kimi v4), 2 const (Claude, gpt-5.5 v8) | 1 hybrid (kimi v10), 1 const (gpt-5.5 v10) | no change |
| A2 enclave | 1 deep (Claude), 1 flag (kimi v4), 1 absent (gpt-5.5 v8) | 2 absent (both v10) | actually got worse: v8 gpt-5.5's flag is gone |
| A3 B2 anchor | 1 ghost (Claude), 1 shadow (kimi v4), 1 real (gpt-5.5 v8) | 2 absent (both v10) | worse: lost gpt-5.5's real anchor |
| A4 tally search | 1 search (Claude), 2 pin (kimi, gpt-5.5) | 2 pin (both v10) | same |
| B1 S7 partition | 1 wrong (Claude), 2 right (kimi, gpt-5.5) | 2 right (both v10) | same |

**Net: fresh-state alone produced LESS convergence on the structural ambiguities, not more.** v10 gpt-5.5 actually shed two encoding elements that v8 had (B2 anchor, ciphertext payload), drifting away from the deeper modeling the synthesis pass had favored.

## Why didn't it converge?

The structural choices that diverge across voices are not driven by intent ambiguity that fresh-state could "rediscover" consistently. They are driven by:

1. **Modeling-discipline priors** the voice brings to the task. gpt-5.5 has a "minimum-viable spec" prior: produce the smallest model that satisfies the listed invariants. kimi has a "shadow what we can't really check" prior. These priors hold across runs.

2. **Stochasticity in model decoding** at relevant decision points. Whether gpt-5.5 v10 included `ballots_at_end` or not is plausibly a low-margin choice that flipped between sessions. Re-running multiple times for the same voice would show this directly.

3. **The intent really is leaving choices open** at A2 (enclave layer) and A3 (B2 encoding). Two voices both choosing "skip the enclave layer" in v10 is evidence that the intent does not require it. It does not show convergence is possible without further tightening.

## What the baseline tells us

The convergence hypothesis "fresh sessions converge more" is empirically rejected. Same intent, same voice, fresh session produces meaningful drift within a voice and no additional convergence across voices.

If the goal is convergence, the lever is the intent itself, not the session state. Specifically the A2 (enclave) and A3 (B2) tightenings from the synthesis diff are the next test — that's the v11 experiment.

If the goal is signal extraction, the methodology already works: divergence is information about where the intent leaves choices open, and the synthesis pass resolves them into a canonical spec with documented reasoning.

## Files preserved as audit trail

- `.colosseum/specs/v4-agent-iterate/per-voice/kimi-k2-6/` — kimi run 1
- `.colosseum/specs/v8-gpt-5-5-retry/per-voice/gpt-5-5-native/` — gpt-5.5 run 1
- `.colosseum/specs/v10-baseline-rerun/per-voice/{kimi-k2-6, gpt-5-5-native}/` — fresh-state run 2
- `specs/rcv.qnt` — synthesis canonical (with kimi's S7 partition equation adopted)
- `specs/rcv.qnt.pre-synthesis` — Claude original before S7 fix
