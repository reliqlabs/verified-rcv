# v11 result — intent tightening drives convergence

Round 3a #47. After v10 established that fresh-state alone does not converge voices, v11 tested whether explicit intent tightening on the two genuinely ambiguous structural axes (A2 enclave-layer modeling, A3 B2 snapshot encoding) closes the gap.

## What changed in intent v0.3.2

Two encoding-discipline notes added.

**A2 (enclave-layer modeling) — appended to §2.5 Block E1:** the protocol-layer model MUST include three enclave-side state variables (`enclave_consumed_ballots`, `enclave_computed_tally`, `enclave_has_session_key`) with their own transitions, so B10 chain-side projection is checkable as a state predicate rather than a boolean flag.

**A3 (B2 frozen-ballots) — appended to §3.2 B2 row:** the protocol-layer model MUST encode B2 as a state invariant over a snapshot variable. Action-guard disablement alone is insufficient.

Intent version bumped 0.3.1 → 0.3.2 (PATCH per the SemVer rubric, since no invariant content changed).

## Spec dimensions

| Voice | v10 (untightened) rcv.qnt | v11 (tightened) rcv.qnt | Δ |
|---|---|---|---|
| kimi-k2-6 | 280L | 358L | +78L (+28%) |
| gpt-5.5-native | 148L | 271L | +83% |

Both voices grew under the tightened intent. The added lines are predominantly the new enclave-side state, the enclaveTally transition, and the B10 projection invariant.

## Convergence: v10 vs v11

| Divergence axis | v10 cross-voice agreement | v11 cross-voice agreement |
|---|---|---|
| Enclave state variables | both absent | **both present, same 3 names** (`enclave_consumed_ballots`, `enclave_computed_tally`, `enclave_has_session_key`) |
| B2 snapshot variable | both absent | **both present, same name** (`ballots_at_end`) |
| B2 invariant body | both absent | **identical** (`if (time >= end_at ∧ end_snapshot_taken) ballots == ballots_at_end else true`) |
| Snapshot capture | n/a | **same trigger** (`timeAdvanceToEnd` captures `ballots_at_end = ballots`, sets `end_snapshot_taken = true`) |
| enclaveTally transition | n/a | **structurally identical** (guard on `phase == "Tallying" ∧ end_snapshot_taken ∧ session_key ∧ not(computed)`, writes `consumed_ballots = ballots_at_end` and `computed_tally`) |
| publishResult guard | varied | **same** (require `computed.present` and `consumed == ballots_at_end`) |
| Tally function | `trivialTally` vs `fallbackTally` | `trivialTally(cands, raw)` vs `tallySpec(raw, cands)` — same body, name + arg-order differ |
| Phase model | kimi: phase string; gpt-5.5: derived predicates | both use phase string with same value set |
| S7 partition | both right | both right |
| Helper actions | divergent | both have `unchanged_static`, `unchanged_enclave` |
| Behavioral shadows (B1, B4–B7) | both declare some, slightly different | minor: kimi declares trivial `true` for B1/B4/B5/B6; gpt-5.5 omits; gpt-5.5 declares `b7_terminal_shadow` and `b8_attestation_shadow` with content; kimi declares both as bare `true` |

## Reading the result

The voices' specs in v11 are essentially isomorphic protocol models. State vectors match, action sets match, the load-bearing invariants (B2, B10, plus all S1–S10) have the same shape across both. The remaining divergence is in shadow-declaration style (which behavioral invariants get a trivial `true` shadow vs being omitted from the composite) and a few cosmetic naming differences (`trivialTally` vs `tallySpec`).

The convergence hypothesis is empirically confirmed: when the intent specifies the encoding requirements on its ambiguous axes, voices converge.

A second observation matters for methodology: the convergence is partly "directed compliance" rather than independent rediscovery. My intent edit literally enumerated the three required enclave variable names. Both voices copied them verbatim. That is correct behavior — the intent is the spec authority — but it shifts the methodology weight from "voices arrive at the same structure independently" to "voices comply with the same explicit instruction." The signal from fan-out under a fully-prescribed intent is then mainly about edge cases the intent doesn't cover (declarative style, helper-function organization, shadow declarations) rather than load-bearing structure.

## Methodology implication

Tightening the intent on a real ambiguity is the correct lever when load-bearing structural divergence is observed across voices. Tightening too much collapses the fan-out diversity that adversarial review depends on. The right balance is to tighten on axes where divergence would be a defect (B10 must be checkable; B2 must be a snapshot) and leave latitude on axes where divergence is encoding signal (declarative style, helper organization).

This run gives evidence for that specifically: A2 and A3 should be tightened in the intent permanently. The remaining divergences in v11 are exactly the encoding-latitude signal the methodology wants to preserve.

## Spec choice for downstream verification

Both v11 specs are technically valid, and the handcrafted Claude spec at `specs/rcv.qnt` (already synthesized with the S7 fix from #45) is more conservative on a few axes. For #18 verify and #19 ledger, three options:

1. Keep `specs/rcv.qnt` (Claude handcraft + S7 synthesis fix) as canonical. The v11 voices' independent confirmation of A2 enclave state and B2 snapshot variables is now documented evidence that Claude's choices are aligned with the methodology.

2. Adopt one of the v11 specs (kimi or gpt-5.5) as canonical. They are simpler, more uniform with the v0.3.2 intent's explicit prescriptions.

3. Re-synthesize Claude's spec against v11 to absorb the few v11 improvements (`end_snapshot_taken` flag is cleaner than `ghost_ballots_at_end_at_captured`; `unchanged_enclave` helper action saves repetition).

Recommendation: option 3. Light re-synthesis pass against v11 to absorb the cleaner pieces, keeping Claude's deeper structural choices (Set vs List for candidates, nondet tally search, more comprehensive S6-S9 predicates). This makes `specs/rcv.qnt` reflect both the original synthesis and the v11 multi-voice confirmation of the load-bearing choices.

## Artifacts

- v11 kimi: `.colosseum/specs/v11-intent-tightened/per-voice/kimi-k2-6/`
- v11 gpt-5.5: `.colosseum/specs/v11-intent-tightened/per-voice/gpt-5-5-native/`
- v10 baseline (for comparison): `.colosseum/specs/v10-baseline-rerun/per-voice/{kimi-k2-6, gpt-5-5-native}/`
- Intent v0.3.2: `.colosseum/intent.md`
- Original synthesis: `.colosseum/specs/synthesis-2026-05-19/diff.md`
- Baseline comparison: `.colosseum/specs/synthesis-2026-05-19/baseline-vs-rerun.md`
