# Cross-critique meta-analysis — 2026-05-19

Round 3a methodology dogfood. After v11 produced two near-isomorphic specs through intent tightening, this experiment tested whether voice-on-voice critique (with high reasoning effort) surfaces signal that single-voice generation misses.

## Setup

- Two voices: `burnt/kimi-k2-6` and `openai/gpt-5.5` (native).
- Both pass v11 validation (typecheck=0, safety holds, 3/3 witnesses violated).
- Both opencode invocations used `--variant high` to apply provider-specific high reasoning effort. Confirmed via session DB: `"variant":"high"` recorded.
- Cross-critique prompt asked three focused questions per pair:
  1. Most material structural divergence
  2. Apparent defect in target spec (technically valid but semantically empty)
  3. Change reviewer would make to its own spec after reading target

## Wall-clock

| Pair | Elapsed |
|---|---|
| gpt-5.5 reviewing kimi | 42s |
| kimi reviewing gpt-5.5 | 700s |

The 16x difference is mostly reasoning-token count under `--variant high`. gpt-5.5's reasoning was tight; kimi spent significantly more reasoning tokens. Both produced equivalently structured outputs.

## Defect convergence

The strongest signal is that **both voices independently identified the same tautological-shadow defect class**, but on different axes.

| Defect | Identified by | In target spec |
|---|---|---|
| `b8_attestation_shadow = true` (tautology where a checkable predicate was claimed in design notes) | gpt-5.5 | kimi |
| `s9_elimination_monotonicity = if (present) eliminated_by_round.length() == 0` (vacuously true because tallySpec always returns empty eliminations) | kimi | gpt-5.5 |

Both defects are typecheck-clean and pass `quint run --invariant=all_invariants` because the predicate body evaluates to `true`. Standard validation does not catch them. Cross-critique catches both. The convergence on the same defect *class* across both reviewers, identified on different specific predicates, is the methodology signal cross-critique was designed to produce.

## Symmetric self-improvement

Each reviewer admitted a weakness in its own spec after reading the target:

- gpt-5.5 (Q3): "I would replace my `s9_elimination_monotonicity` with the target's indexed shape. My predicate is only valid because my current `tallySpec` never emits eliminations. The target's version better matches intent S9 and would survive a future nontrivial IRV tally model."
- kimi (Q3): "I would add the B7 terminal shadow and B8 attestation shadow to my `all_invariants` composite, replacing their current `true` definitions. The target encoded these better because it recognized that B7 and B8 admit meaningful classical-Prop shadows that can be checked as state predicates."

These are symmetric improvements: each voice is willing to adopt what the other did better. The combination of (a) self-defect-identification and (b) target-strength-adoption produces a third spec that takes the strongest encoding from each — without the synthesis pass having to make those calls externally.

## Sycophancy assessment

The earlier methodology note flagged sycophancy as the main risk for general "peer review of full specs." Empirically that did not happen on this pair.

- kimi opened with: "The target's all_invariants is therefore strictly stronger and more faithful to the intent's cross-layer composition structure."
- gpt-5.5 stated: "[kimi's named invariant] itself claims nothing, which weakens the spec as a verification artifact."

Both reviewers stated weaknesses in the target plainly. Both also stated weaknesses in their own spec plainly. The structured-prompt framing (Q1/Q2/Q3 with explicit instruction that Q2 is asking about a defect) appears to push past the default validate-and-agree posture.

## What signal did cross-critique add over synthesis-alone?

The v11 synthesis pass would not have caught either tautological-shadow defect because the synthesis pass works at the structural level: state variables, action sets, witness invariants. Behavioral-shadow encoding is below the diff threshold the synthesis pass operates at. Both defects required reading the predicate bodies of the behavioral shadows and reasoning about what they actually claim.

Cross-critique surfaced two real, otherwise-invisible defects in two passing specs. The cost was one extra dispatch round (about 700s wall-clock for the slow voice). That ratio is the right one for the methodology to absorb cross-critique as a standard step after fan-out + synthesis when load-bearing behavioral shadows are present.

## v0.4 back-port implications

For task #40 (Colosseum v0.4 candidate asks), cross-critique now has a worked example to anchor a skill design:

1. **Skill: `colosseum-spec-cross-critique`**. Optional pass after fan-out + synthesis. Trigger: a passing spec contains behavioral-shadow predicates declared as bare `true` or as predicates that depend on values trivially set by the spec's own pure functions. The skill dispatches each voice as reviewer of every other voice's spec, with the three-question structure above. High reasoning effort (`--variant high`) is the right default.

2. **Skill: synthesis pass should incorporate cross-critique findings.** When cross-critique identifies a defect, the synthesis pass should require the defect be addressed either by tightening the predicate or by explicitly downgrading the invariant tag (e.g., from `state` to `omitted-with-justification`).

3. **The `--variant high` configuration** that this experiment surfaced should be defaulted in any v0.4 multi-voice dispatch. Reasoning-effort=low produces specs that pass typecheck but contain the kind of empty-shadow defects cross-critique reveals. The increased wall-clock is worth the deeper output.

4. **Sycophancy mitigation through structured prompts.** The Q1/Q2/Q3 framing here worked. v0.4 cross-critique skill should mandate this kind of structured questioning, not open-ended review.

## Action items for `specs/rcv.qnt`

The canonical spec is the handcrafted Claude version (with v11 voice confirmation). It already has:
- Real B8 predicate (`inv_b8_publish_implies_registry_honest`) — not a tautology
- Real B10 predicate (`inv_b10_chain_side_projection`) — not a tautology
- S6-S9 omission (Claude opted to omit S8/S9 entirely rather than declare tautological shadows)

The two cross-critique defects do not apply to the canonical. Both v11 specs would now correct themselves toward Claude's discipline if asked to revise. No edits to `specs/rcv.qnt` required from this critique.

## Artifacts

- `kimi-k2-6-reviews-gpt-5-5-native/critique.md` — kimi's review of gpt-5.5
- `gpt-5-5-native-reviews-kimi-k2-6/critique.md` — gpt-5.5's review of kimi
- `dispatch.log` — run summary
- This file — meta-analysis and v0.4 implications
