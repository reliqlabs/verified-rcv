# Multi-voice Quint spec synthesis — 2026-05-19

Round 3a task #45. Resolves the divergence between three validating
voices into one canonical spec at `specs/rcv.qnt`. Produces a list of
intent-revision targets for v0.4.

## Voices under synthesis

| Voice | Path | rcv.qnt | main.qnt | Notes |
|-------|------|---------|----------|-------|
| Claude (handcrafted) | `specs/rcv.qnt` | 453L | 16L | Anchors at parent-agent dispatch. Deepest model. |
| kimi-k2-6 (agent-iterate via gateway) | `.colosseum/specs/v4-agent-iterate/per-voice/kimi-k2-6/rcv.qnt` | 270L | 11L | Shadow-style behavioral invariants. |
| gpt-5.5 (native OpenAI via opencode) | `.colosseum/specs/v8-gpt-5-5-retry/per-voice/gpt-5-5-native/rcv.qnt` | 168L | 8L | Most compact. last_action discipline. |

All three pass: `quint typecheck = 0`, `all_invariants` holds, 3/3
witness invariants violated under `--max-steps=30 --max-samples=100`.

## Divergence catalogue

Format: divergence ID, classification (A=intent-undefined, B=encoding-latitude, C=outlier), per-voice choice, proposed resolution.

### A1. Parameterization shape (candidates, start_at, end_at)

Classification: A (intent-undefined).

- Claude: `const CANDIDATES: List[str]`, `const START_AT: int`, `const END_AT: int`. Bound at `import rcv(...)`.
- kimi: `var candidates: Set[Addr]`, `var start_at: int`, `var end_at: int` plus explicit `instantiate(candidates_set, s, e)` action that sets them.
- gpt-5.5: same as Claude. `const CANDIDATES: Set[str]`.

Intent §2.5 Block 1 ("instantiate") describes a one-shot initialization event with the candidate set and timestamps. It does not state whether the Quint model should treat these as compile-time constants (one model per election) or as state variables set by an instantiate transition.

Resolution: keep `const` (Claude/gpt-5.5 agreement). Finite model checking is cleaner with parameter binding at `import`; the canonical Quint examples (`reactor.qnt`, `minimmit`) use this pattern. kimi's runtime instantiation transition adds a transition without adding invariant signal.

Intent-revision target (v0.4): clarify that Block 1 parameters are bound at instantiation and modeled as `const` in the Quint protocol model, not as runtime-mutable state.

### A2. Enclave-layer state modeling

Classification: A (intent-undefined).

- Claude: three state variables, `enclave_has_session_key: bool`, `enclave_consumed_ballots: Addr -> Ciphertext`, `enclave_computed_tally: OptionalTally`. Dedicated `enclave_tally(t)` action with guard `is_tally_spec_output(t, ghost_ballots_at_end_at)`. `publish_result` reads `enclave_computed_tally`.
- kimi: two boolean flags, `attestation_valid: bool`, `tally_computed: bool`. Both set in `publish_result` itself. No enclave-side state variable.
- gpt-5.5: no enclave-side state. `publish_result` computes `valid_tally_for(ballots)` inline.

Intent §2.5 Block E1 describes an off-chain enclave action that computes Tally_spec over the chain-frozen ballot set and produces an attestation. §3.2 B10 requires "enclave_input_fidelity" (consumed = ballots@end_at). To check B10's chain-side projection invariant the model must distinguish what the chain published from what the enclave consumed.

Resolution: keep Claude's enclave-layer modeling. kimi's flag-based shadow and gpt-5.5's collapsed model both lose the ability to express the chain-side B10 projection as a real predicate (consumed_ballots == ghost snapshot). Both reduce B10 to a boolean flag that asserts itself.

Intent-revision target (v0.4): state that the protocol-layer Quint model MUST include enclave-side state (consumed ballots, computed tally, session-key liveness) sufficient to express B10's chain-side projection as a state predicate, not a flag.

### A3. B2 frozen-ballots encoding

Classification: A (intent-undefined).

- Claude: ghost variables `ghost_ballots_at_end_at: Addr -> Ciphertext` and `ghost_ballots_at_end_at_captured: bool`. `tick(dt)` action captures the snapshot on the first tick that crosses END_AT. State invariant `inv_b2_ballots_at_end_at_frozen` checks (i) snapshot is stable and (ii) chain ballots subset of snapshot.
- kimi: pure shadow `inv_B2 = not(tally_present) or true`. Relies on `submit_ballot` action-guard requiring `isVoting` to make late writes structurally impossible.
- gpt-5.5: regular state variable `ballots_at_end: str -> str` (no ghost prefix), captured in `tick` action when crossing END_AT, also in `close_and_tally`. Behavioral shadow: `if (now >= END_AT) ballots == ballots_at_end else true`.

Intent §3.2 B2 says "ballots@end_at is frozen at the Voting → Tallying boundary." This is a temporal property. Action-guard disablement (kimi) is the weakest encoding (it relies on the action set's structural completeness, not a checkable predicate). Snapshot-based encoding (Claude, gpt-5.5) provides a real invariant.

Resolution: keep Claude's ghost-var snapshot. Adopt gpt-5.5's tighter invariant phrasing (`if (now >= END_AT) ballots == ballots_at_end`) as the stronger form. The ghost-var prefix in Claude is a discipline marker; rename if desired but the structure stays.

Intent-revision target (v0.4): require B2 to be encoded as a checkable invariant over a snapshot variable (ghost or regular), not pure action-guard shadow. The shadow form is insufficient as a B2 proof obligation.

### A4. Tally search vs deterministic pin

Classification: A (intent-undefined).

- Claude: `step` nondeterministically picks a TallyResult shape (winners, counts, rounds), then `enclave_tally(t)` action-guard filters via `is_tally_spec_output(t, ghost_ballots_at_end_at)`.
- kimi: deterministic single tally inside `publish_result`: `{ winners: candidates, ballots_tallied: ballot_keys.size(), ballots_dropped: 0, non_voters: candidates.exclude(ballot_keys) }`.
- gpt-5.5: deterministic via `valid_tally_for(ballots)` pure function with similar shape.

Intent §3.1 S6–S9 are well-formedness predicates on a published tally; the actual IRV computation is off-chain (Block E1) and is the subject of B10_lean (a math-spec obligation, not a Quint one). The Quint model only needs to assert the published tally is a well-formed Tally_spec output.

Resolution: keep Claude's nondet search. Exercising the model checker over the tally space catches more well-formedness bugs than pinning to one value. Adopt gpt-5.5's `valid_tally_for(bs)` as a useful helper for the witness case, but the publish path should remain search-style.

Intent-revision target (v0.4): clarify that S6–S9 are well-formedness predicates the chain checks at publish time, and the model checker is expected to exercise the tally space via nondet search.

### B1. S7 partition equation vs bound inequality

Classification: B (encoding latitude, pick the stronger).

- Claude: `ballots_tallied + ballots_dropped <= cands.size()` (inequality).
- kimi: `ballots_tallied + ballots_dropped + non_voters.size() == candidates.size()` (partition equation).
- gpt-5.5: same as kimi (partition equation).

Intent §3.1 S7 reads as a partition statement (every candidate is in exactly one of tallied/dropped/non-voter). 2 of 3 voices read it that way.

Resolution: ADOPT kimi/gpt-5.5 partition equation. Tighter constraint, catches accounting bugs Claude's bound misses.

### B2. resolved_stutter action

Classification: B (encoding latitude).

- Claude: no explicit stutter action. Relies on the fact that no action has its guard satisfied once `tally_result.present`.
- kimi: same as Claude.
- gpt-5.5: explicit `resolved_stutter` action whose only guard is `tally_result.present` and which is a no-op self-loop.

Quint requires `step` to remain enabled in some sense. Both approaches typecheck and validate. The explicit stutter is more defensively obvious.

Resolution: optional. Adopt gpt-5.5's explicit `resolved_stutter` if cleaner; not load-bearing.

### B3. Total map vs partial map for ballots

Classification: B (encoding latitude).

- Claude: `Addr -> Ciphertext` (Quint Map, partial).
- kimi: `Addr -> str` total via `ADDR_POOL.mapBy(a => "")` plus separate `ballot_keys: Set[Addr]` set to track the live domain.
- gpt-5.5: same as Claude (Map, partial).

Resolution: keep Claude's partial-map encoding. kimi's parallel-set workaround is more state without invariant gain.

### B4. last_action ghost tag

Classification: B (encoding latitude).

- gpt-5.5: `last_action: str`, set in every action.
- Claude/kimi: not present.

Useful for debugging counterexamples but not load-bearing for the invariant set we have. Skip for the canonical.

### C1. kimi `initialized` flag and instantiate action

Classification: C (outlier choice driven by A1 above).

If we accept A1's resolution (const parameters), the `initialized` flag and `instantiate` action both disappear. No state invariant ties to them outside guarding the rest of kimi's actions.

Not adopting.

### C2. Witness phrasings (cosmetic only)

All three voices phrase witnesses as negations of reachable states. The exact phrasing differs (`not(tally_result.present)` vs `ballot_keys.size() == 0` vs `not(ballots.keys().size() > 0)`). All three pass the witness check. No synthesis action needed.

## Proposed edits to `specs/rcv.qnt`

Concrete change set:

1. S7 partition equation (B1).
   Replace the `<=` bound in `well_formed_tally` with the partition equation. Specifically change:
   ```
   t.ballots_tallied + t.ballots_dropped <= cands.size(),
   ```
   to:
   ```
   t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == cands.size(),
   ```

2. Optional: add an explicit `resolved_stutter` action (B2). Skippable if validation still passes without it.

3. Add a note in the rcv.qnt header listing which choices came from synthesis vs original authorship (audit trail).

No other structural changes. Claude's deeper encoding stays as the canonical because the divergence analysis above showed it provides the most checkable signal on the load-bearing properties (A2 enclave, A3 B2 anchor, A4 tally search).

## Intent-revision targets for v0.4

Distinct from the spec edits; these go into a future intent revision pass and inform the colosseum-spec-synthesis skill skeleton.

1. (from A1) Specify that Block 1 parameters are bound at instantiate-time and modeled as `const` in the Quint protocol model.

2. (from A2) Specify that the protocol-layer Quint model MUST include enclave-side state (consumed ballots, computed tally, session-key liveness) sufficient to express B10 as a state predicate, not a boolean flag shadow.

3. (from A3) Specify that B2 (frozen ballots at end_at) MUST be encoded as a state invariant over a snapshot variable, not pure action-guard shadow.

4. (from A4) Clarify that S6–S9 are well-formedness predicates the chain checks at publish time; the model checker is expected to exercise the tally space via nondet search.

5. (from B1) Restate S7 as a partition equation to remove the bound-vs-equation ambiguity that produced the 1/3 vs 2/3 split.

## Audit trail discipline

This synthesis report is the artifact that justifies why `specs/rcv.qnt`'s canonical form is what it is. Future readers (including #18 verify, #19 compose, #20 comparator) can check the per-divergence reasoning here without re-deriving it.

The three input specs remain in `.colosseum/specs/v4-agent-iterate/`, `.colosseum/specs/v8-gpt-5-5-retry/`, and the Claude original is preserved as `specs/rcv.qnt.pre-synthesis` (created when the edits are applied).
