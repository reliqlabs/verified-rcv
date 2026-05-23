# Lean cross-critique meta-analysis (2026-05-20)

Six-pair cross-critique of the three voices that passed Lean fan-out (kimi-k2-6, gpt-5-5-native, magistral-medium-native). All reviews at `--variant high`. All 6 reviews landed substantive critiques; 4 wrote directly to `critique.md`, 2 (magistral's reviews) emitted to stdout and were salvaged post-hoc.

## Wall-clock

| Pair | Elapsed | Critique size |
|---|---|---|
| gpt-5.5 reviews magistral | 67s | 3046 bytes |
| gpt-5.5 reviews kimi | 82s | 2086 bytes |
| kimi reviews magistral | 206s | 3701 bytes |
| kimi reviews gpt-5.5 | 731s | 3757 bytes |
| magistral reviews kimi | (stdout) | 26 lines |
| magistral reviews gpt-5.5 | (stdout) | 19 lines |

## Convergent finding: magistral has a composition defect (and so does the canonical)

Both kimi and gpt-5.5 independently identified the same defect in magistral's spec, and magistral self-flagged the same issue when reviewing gpt-5.5's contrasting design.

**Defect**: magistral's `Tally_spec` does not thread `non_voters` from Stage 1.

```lean
structure DecryptedSet where
  valid   : List (Addr × Ballot)
  dropped : List Addr
  -- no non_voters field

def Tally_spec ... :=
  let d := decrypt_and_validate raw candidates privkey
  let r := IRV_spec d.valid candidates
  { r with
    dropped_voters  := d.dropped
    ballots_dropped := d.dropped.length }
  -- non_voters inherited from IRV_spec opaque output
```

Since `IRV_spec` receives only `d.valid` and `candidates`, it has no access to `raw_ballots.keys`, which is required to compute `non_voters = candidates \ raw_ballots.keys` per intent §2.5. The composition is semantically wrong against the intent's Stage 1 obligation.

**The canonical `specs/RcvSpec.lean` has the same defect.** My canonical's `DecryptedSet` is identical in shape to magistral's, and `Tally_spec`'s record update threads only `dropped_voters` + `ballots_dropped`. This was caught by the cross-critique even though no critic was reviewing the canonical directly — the convergent reasoning about Stage 1/Stage 2 separation makes the canonical's mistake inspectable.

## gpt-5.5's design is methodologically superior on Stage 1/2 separation

gpt-5.5's spec uses a separate `IRVResult` type for Stage 2 output, containing only `(winners, per_round_counts, eliminated_by_round, ballots_tallied)`. `Tally_spec` then composes Stage 1's `DecryptedSet` (which includes `non_voters`) with `IRVResult` to assemble the full `TallyResult`. This matches intent §2.5's "Stage 2 is the combinatorial IRV core" claim and makes a future "Stage 2 correctness" proof inspectable in isolation.

Both kimi and magistral conceded this design after reviewing gpt-5.5; the canonical and magistral did not have it.

## Convergent finding: gpt-5.5's `[i]?`-Option-pattern S9 encoding is preferred

Three of three reviewers (gpt-5.5 defending its own choice, kimi reviewing gpt-5.5, magistral reviewing gpt-5.5) agreed that gpt-5.5's S9 encoding using `[i]? = some eliminated → ...` is methodologically stronger than `[i]!`-with-bounds-check:

- Avoids reliance on partial functions (`!`) whose default-value semantics could leak into proof terms
- Encodes index validity as theorem hypothesis (existence witness), not as separate `i < length` constraint
- More robust to refactoring (out-of-bounds index produces well-typed `none` rather than panic)

Kimi and magistral both stated they would update their own specs to use the Option pattern.

## Intent under-specification signals (encoding-discipline note candidates)

Surfaced by the cross-critique even though not flagged as defects:

1. **`CandidateSet := List Addr` distinctness**: All three voices (and the canonical) use plain lists for `CandidateSet` without encoding S2's distinctness at the type level. If `cs` contains duplicates, `cs.length` overcounts relative to the set interpretation in `non_voters`. Intent S2 is a separate structural invariant, but the Lean encoding leaves it as an external proof obligation. Candidate for an encoding-discipline note: should Lean specs require `Nodup cs` as a hypothesis or use a distinct type?

2. **`ballots_tallied = d.valid.length`** not axiomatized: All three specs leave `ballots_tallied` as an opaque field of `IRV_spec`'s output. The relationship between `ballots_tallied` and the actual count of valid ballots is not stated. Candidate for an encoding-discipline note or a new axiom.

3. **"Voters = candidates" identification**: Magistral got confused by `s7_voter_partition`'s right-hand side being `cs.length` (a count of candidates) while the left-hand side counts voters. The intent treats voters and candidates as the same set (each candidate has permission to submit a ballot), but the Lean encoding doesn't make this explicit. Candidate for an intent clarification or a typed identification `Voter := Candidate`.

## No tautological-shadow defects found

Across all 3 voices:
- `EnclaveImage` declared as `axiom` (never `def ... := Tally_spec`)
- All 5 required theorems have `:= by sorry` bodies (no `rfl`, `trivial`, `True.intro`)
- Stage 1 / Stage 2 declared `opaque` (modeled, not implemented)

Cross-critique was looking for tautological shadows like the Quint round caught; none present in the Lean specs. The convergent defect was instead a composition error, which is a different defect class.

## Methodology observations

1. **Cross-critique remains load-bearing on Lean**: The convergent composition-defect finding is something fan-out + synthesis alone would not have caught. Three independent reviewers converged on the same structural issue, including reviewers self-identifying the same defect in their own spec.

2. **Smaller voice panel works for Lean**: 3 voices producing 6 pairs gave clean convergence. Quint warranted 4+ voices; Lean's narrower encoding-choice space made 3 sufficient. Ask O / colosseum-adversarial Step 6.5 should document the voice-count tradeoff (Quint: 4+; Lean: 3 minimum).

3. **Tool-use compliance varies by voice**: magistral emitted critiques to stdout instead of writing to file in both of its review pairs. Content was substantive when salvaged. Future dispatch scripts should accept stdout output as a fallback when no critique.md is emitted, or the prompt should be more emphatic about the file-write step.

4. **gpt-5.5 → kimi 731s outlier**: 12-minute critique vs 1-3min for others. Likely gateway/network hiccup, not a methodology signal. Output was the longest and richest critique (3757 bytes).

## Punch list (pending acceptance)

The cross-critique findings, in order of materiality:

- **Apply to canonical**: split Stage 2 output type. Add `IRVResult` (or `IrvOutput`) with only `(winners, per_round_counts, eliminated_by_round, ballots_tallied)`. Have `Tally_spec` compose `DecryptedSet` (with `non_voters`) and `IRVResult` to produce `TallyResult`. This fixes the convergent composition defect.
- **Apply to canonical**: adopt gpt-5.5's `[i]?`-Option-pattern S9 encoding.
- **Intent v0.3.3 candidates**: encoding-discipline note on `CandidateSet` distinctness; `ballots_tallied` axiomatization; "voters = candidates" identification.
- **Methodology back-port**: tool-use compliance footnote in Ask O dispatch protocol.

Defense round next, then re-cross-critique after canonical revision (Asks P + R).
