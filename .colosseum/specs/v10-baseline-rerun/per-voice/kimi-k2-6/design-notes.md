# Design Notes — RCV Quint Spec (kimi-k2-6 voice)

## State-machine mapping (§2.5)

| Intent block | Quint action | Notes |
|---|---|---|
| Block 1: instantiate | `instantiate` | Guards on `phase == "Uninitialized"`, sets constants, moves to `Created`. |
| Block 2: time advances to start_at | `timeAdvanceToStart` | Jumps `time` from any value `< start_at` directly to `start_at`, moving phase to `Voting`. |
| Block 3: submit_ballot | `submitBallot(voter, enc)` | Guarded on `phase == "Voting"`; voter must be in `CANDIDATES`; appends encrypted ballot to `ballots` map. |
| Block 4: time advances to end_at | `timeAdvanceToEnd` | Jumps `time` to `end_at`, moving phase to `Tallying`. |
| Block 5: close_and_tally | `closeAndTally` | Idempotent self-loop in `Tallying`. No state change. |
| Block 6: publish_result | `publishResult` | Guarded on `phase == "Tallying"` and `not(tally_result.present)`. Sets `tally_result.present = true`, phase to `Resolved`. |

A stuttering action (time tick) is included in `step` to allow arbitrary time progression without phase changes.

## Invariant encoding (§3.1 + §3.2)

**Structural invariants (§3.1):**

- **S1–S3** (`s1_nonempty_candidates`, `s2_distinct_candidates`, `s3_well_ordered_window`): Guarded by `phase != "Uninitialized"` so they do not fire on the empty initial state.
- **S4** (`s4_ballot_keys_are_candidates`): Uses `ballots.keys().forall(...)` with a `listContains` helper since Quint has no native `List.contains`.
- **S5** (`s5_terminality`): Omitted as a direct invariant; terminality is enforced by action guards (no outgoing transitions from `Resolved`).
- **S6** (`s6_winner_wellformed`): Checks winners list is non-empty, within bounds, and all elements are valid candidates. Uses `foldl` because Quint's `forall` only works on sets.
- **S7** (`s7_tally_conservation`): Verifies `ballots_tallied + ballots_dropped + non_voters.length() == candidates.length()`.
- **S8** (`s8_per_round_consistency`): Each round's count map sums to `ballots_tallied`. Uses a `sumValues` helper with `Set.fold`.
- **S9** (`s9_elimination_monotonicity`): Eliminated candidates do not reappear in later rounds. Nested `foldl`/`forall` over lists and sets.
- **S10** (`s10_resolution_implies_past_end`): If `tally_result.present`, then `time >= end_at`.

**Behavioral invariants (§3.2):**

- **B1–B2, B4–B7**: Shadowed as `true` because they depend on ballot decryption semantics or voter identity that this chain-side spec does not model.
- **B3** (`b3_no_premature_tally`): Directly encoded as `not(tally_result.present and time < end_at)`.
- **B8, B10**: Shadowed as `true`. B8 (attestation) and B10 (tally correctness) are cryptographic properties that Quint cannot express; they are noted as classical shadows.
- **B9**: Omitted entirely. The negligibility bound on coercion resistance is a probabilistic off-chain claim; Quint has no probability mechanism.

## Omissions and simplifications

1. **No actual IRV computation**: The spec uses a `trivialTally` that produces a well-formed dummy result (all-zero counts, first candidate as winner). This is sufficient for the chain-side protocol invariants (S6–S10) but does not verify tally correctness (B10). A separate Lean or Verus layer would prove the IRV algorithm.
2. **Encrypted ballots are opaque strings**: The `ballots` map stores `str -> str` (voter -> ciphertext). Decryption and preference extraction are not modeled.
3. **No generic Option type**: Quint lacks parametric types, so `TallyResultOpt` is a concrete record `{ present: bool, value: TallyResult }`.
4. **No Set-to-List conversion**: Quint has no `Set.toList()`. Where lists are needed, they are constructed directly or omitted.
5. **No recursive definitions**: Quint rejects recursive `pure def`. The IRV rounds are not computed recursively; instead, the tally is generated non-deterministically (here, trivially).

## Non-obvious choices

- **Phase as a string**: Rather than a sum type (which Quint does not support well), phase is a `str` with known values `"Uninitialized"`, `"Created"`, `"Voting"`, `"Tallying"`, `"Resolved"`. Action guards check these strings.
- **Time jumps instead of unit ticks**: `timeAdvanceToStart` and `timeAdvanceToEnd` jump directly to the boundary times. This reduces state-space explosion while preserving the ordering invariants.
- **Witness invariants are negations**: `witness_resolution_reachable = not(phase == "Resolved")`. When `quint run` reports "Invariant violated", it means the positive state is reachable, which is the desired witnessing behavior.
- **`all_invariants` is a conjunction of S1–S4, S6–S10, and B3**: S5 and B1–B2, B4–B10 are either structurally enforced by the state machine or shadowed as `true`.
