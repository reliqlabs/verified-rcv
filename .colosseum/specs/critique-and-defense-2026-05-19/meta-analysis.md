# Critique-of-canonical + defense round meta-analysis — 2026-05-19

Round 3a #48. Two methodology experiments at `--variant high`:
1. kimi-k2-6 and gpt-5-5-native each cross-critique the handcrafted Claude canonical (`specs/rcv.qnt`, post-S7-synthesis-fix).
2. Each voice defends its own choice on the axis the other surfaced as a defect in the earlier cross-critique.

## Wall-clock

| Job | Elapsed |
|---|---|
| gpt-5.5 defends s9_elimination_monotonicity | 47s |
| gpt-5.5 reviews canonical | 65s |
| kimi reviews canonical | 157s |
| kimi defends b8_attestation_shadow | 212s |

Four jobs, all PASS, all `--variant high`. gpt-5.5 consistently faster; kimi consistently spends more reasoning tokens.

## Headline finding: both reviewers caught the same defect in the canonical

Both voices independently identified the same primary defect in the handcrafted Claude canonical:

**Canonical drops `per_round_counts` and `eliminated_by_round` from TallyResult.** The canonical's TallyResult uses a synthetic `rounds_played: int` in place of the two intent-mandated list fields. This is not a modeling simplification analogous to "Set vs List" — it removes the ability to express S8 (per-round count consistency) and S9 (elimination monotonicity) as the intent defines them. The canonical's S8 and S9 checks bound `rounds_played` between 0 and `cands.size()`, which is far weaker than the intent's structural requirements.

kimi's framing: "It is a structural omission that removes two intent-mandated invariants from the checkable state space."

gpt-5.5's framing: "S8 and S9 are not actually encoded... it weakens the intent schema and makes malformed audit traces unrepresentable rather than rejected."

This is a real defect in the canonical and needs fixing before the spec moves into #18/#19 downstream verification work.

## Secondary finding (kimi only): well_formed_tally bookkeeping gap

kimi identified that the canonical's `well_formed_tally` predicate omits the equality `t.dropped_voters.size() == t.ballots_dropped`. The canonical's `enclave_tally` constructs `dropped_voters: Set()` with `ballots_dropped ∈ {0, 1}`, so a state with `ballots_dropped == 1` and empty `dropped_voters` passes validation. This is technically valid Quint but admits the inconsistency that on-chain accounting is supposed to forbid.

Not as load-bearing as the S8/S9 defect, but worth folding into the same revision.

## Canonical wins: derived_phase

Both reviewers said they would adopt the canonical's `pure def derived_phase` approach (computing phase from `block_time` and `tally_result` rather than carrying a `phase: str` state variable). The intent explicitly describes phase as derived. The canonical is faithful; the v11 voices' stored-phase approach is a divergence the synthesis pass earlier left for revision.

This is independent cross-validation of one canonical design choice.

## Defense round: both voices conceded

The methodology question for defenses was: under high reasoning + structured framing, does the defender anchor to the original choice or honestly reassess?

**kimi defending b8_attestation_shadow:** verdict B (concede). Stated: "The critique is correct. `val b8_attestation_shadow = true` is a tautology that provides zero model-checking value." Provided a concrete fix and verified it still typechecks. Cited the intent's v0.3.2 encoding discipline A3 (snapshot invariant) as the general principle that should have applied to B8 as well.

**gpt-5.5 defending s9_elimination_monotonicity:** verdict B (concede). Stated: "The critique is correct: my S9 predicate proves only that my simplified `tallySpec` never emits eliminations, not the intent's structural monotonicity requirement." Wrote out a complete replacement Quint predicate using a fold-and-accumulator pattern over the elimination history.

Both defenses produced concrete replacement encodings, not vague acknowledgments. Both cited specific intent sections to support the concession. The defense exercise produced genuine self-improvement output that the original cross-critique alone did not.

Methodology observations from the defenders themselves:

- kimi: "I treated action-guard structure as sufficient for invariants that should be explicitly checkable. The v0.3.2 encoding discipline already stated this principle for B2, but I failed to generalize it to B8."
- gpt-5.5: "This defense prompt forced a separation between 'my model's generator only creates valid zero-elimination results' and 'the invariant rejects invalid externally supplied tally shapes.'"

Both are real meta-insights, not defensive boilerplate.

## What this run added beyond the v11 cross-critique

The v11 cross-critique caught two defects in the v11 voices' specs (b8 tautology in kimi, s9 tautology in gpt-5.5). This run caught:
- A larger defect in the canonical that neither of the v11 voices had in their own spec (the canonical's TallyResult shape is weaker than v11's).
- Cross-validation that the canonical's derived_phase approach is the right one (both v11 voices want to adopt it).
- Conceded improvement encodings for both original defects, including a fully-written Quint replacement for s9 in gpt-5.5's defense.

The run also confirmed that defense rounds are non-sycophantic when run at high reasoning with structured prompting. Both voices conceded plainly; neither tried to anchor to the original choice. The risk of defense-anchoring that I worried about earlier in the methodology design did not materialize on this pair.

## Implications

**For the canonical spec.** The S8/S9 defect is real and significant. Two options:
1. Restore `per_round_counts` and `eliminated_by_round` to the canonical's TallyResult and encode real S8/S9 predicates over them. gpt-5.5's defense already contains a working S9 predicate that can be lifted.
2. Document the omission explicitly in the canonical's header comments and design notes, marking S8/S9 as deliberately deferred to a later spec round or to the Lean math layer.

Option 1 is the right answer per the intent. The intent doesn't say S8/S9 are off-chain or out-of-scope; it lists them in §3.1 as load-bearing structural invariants. Removing them from the protocol-layer Quint spec means we are silently weakening the intent's claim.

**For the methodology back-port (task #40).** Three concrete asks for v0.4:
1. Cross-critique should run against the canonical spec, not just against the fan-out voices. The canonical is the artifact that downstream verification consumes; defects in it are the highest-leverage to find.
2. Defense rounds at `--variant high` with structured framing are non-sycophantic and produce concrete improvement output. Add them as a standard post-critique step.
3. The methodology should track which voices have reviewed which specs against which axes, so review coverage gaps are visible.

## Artifacts

- `kimi-k2-6-reviews-canonical/critique.md`
- `gpt-5-5-native-reviews-canonical/critique.md`
- `kimi-k2-6-defends-b8_attestation_shadow/defense.md`
- `gpt-5-5-native-defends-s9_elimination_monotonicity/defense.md`
- Prior round: `.colosseum/specs/cross-critique-2026-05-19/` (kimi ↔ gpt-5.5)
