**§2.5 block → action mapping**
- Block 1 (instantiate) → const parameters bound in `main.qnt`; `init` sets empty state.
- Block 2/4 (time advance) → `tick` increments discrete time and snapshots `ballots` into `ballots_at_end` the first time `END_AT` is crossed.
- Block 3 (submit_ballot) → `submit_ballot(voter, ballot)` guarded on voting window and voter membership.
- Block 5 (close_and_tally) omitted (no-op event emission).
- Block 6 (publish_result) → `publish_result` computes `tally_spec(ballots)` and, if well-formed, sets `tally_result.present = true`.
- Block E1 (enclave tally) is pure-def `tally_spec`: a deterministic plurality-like count over first preferences (simplified from full IRV to avoid recursion).

**§3.1 / §3.2 invariant encoding**
- Structural: S1–S3 are const well-formedness; S4 is ballot-key containment; S6–S7 are tally-result winner-subset and count-conservation; S10 is time-after-end.
- Temporal shadows via history variables:
  - B1 → `was_resolved` implies `tally_result.present`.
  - B2 → `end_crossed` implies `ballots == ballots_at_end`.
  - B3 → `was_resolved` implies `end_crossed`.
  - B4–B7 encoded structurally by action guards (ballots written only during voting; publish_result gated on `not(was_resolved)`; no exit from resolved).
  - B8/B9 omitted (crypto/meta-security beyond Quint scope).
  - B10 → `tally_result` value equals `tally_spec(ballots)`.
- S5, S8, S9 omitted to stay within output budget.

**Non-obvious encoding choices**
- No `Option` type: Quint lacks it, so `MaybeTally` record with `present` flag used.
- `var` declarations placed before state-dependent `val` invariants; ordering matters for Quint scoping.
- Recursive IRV replaced by a fold-based plurality aggregator to eliminate general recursion, which Quint may not support in `pure def`.
- Map update uses `.set`, not `.put` (Quint convention per canonical `reactors.set(...)`).
- Set subset encoded as `exclude(...).size() == 0` because `.subset` operator is not in the canonical idiom.
- Ballots in `submit_ballot` hardcoded to `CANDIDATES` (a valid full ranking) to avoid nondeterministic ballot generation.
- `nondet` in `step` wrapped in `all { ... }` body for parser safety.

**Witnesses**
- `witness_resolution_reachable` (`not(was_resolved)`): violated by `publish_result`.
- `witness_ballot_submittable` (`ballots` empty): violated by `submit_ballot`.
- `witness_end_at_crossing` (`not(end_crossed)`): violated by `tick` crossing `END_AT`.