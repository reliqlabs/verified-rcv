# Design Notes — kimi-k2-6 Quint Encoding

## §2.5 Block → Action Mapping

| Block | Action | Notes |
|-------|--------|-------|
| Block 1 (instantiate) | `instantiate(candidates_set, s, e)` | Chooses from `CANDIDATE_SETS`. Guards: `len ≥ 1`, distinctness (via `Set`), `s > time`, `e > s`. |
| Block 2 (time → start_at) | `advance_time` | Implicit; time increments by 1. `isVoting` becomes true when `time` crosses `start_at`. |
| Block 3 (submit_ballot) | `submit_ballot(sender, prefs)` | Guarded by `isVoting`, `sender.in(candidates)`, `prefs != ""`. Last-write-wins via `ballots.set`. |
| Block 4 (time → end_at) | `advance_time` | Implicit; `isTallying` when `time ≥ end_at` and `not(tally_present)`. |
| Block 5 (close_and_tally) | `close_and_tally` | Self-loop in Tallying; no storage change. |
| Block 6 (publish_result) | `publish_result` | Guarded by `isTallying` and `not(tally_present)`. Sets a trivial well-formed tally. |
| Block E1 (enclave tally) | Shadow only | IRV computation is off-chain; `publish_result` abstracts the enclave output. |

## §3.1 Structural Invariant Encoding

- **S1** (`len(candidates) ≥ 1`): `inv_S1` — `not(initialized) or candidates.size() >= 1`.
- **S2** (distinct candidates): Trivially satisfied by `Set[Addr]`; duplicates impossible.
- **S3** (`start_at < end_at`): `inv_S3` — guarded by `instantiate` precondition.
- **S4** (ballot keys ⊆ candidates): `inv_S4` — `ballot_keys.subseteq(candidates)`. Guarded by `submit_ballot` requiring `sender.in(candidates)`.
- **S5** (write-discipline): Meta-property; only `publish_result` writes `tally_present`, gated on `not(tally_present)`. Not a state predicate.
- **S6** (winner well-formedness): `inv_S6` — `winners.subseteq(candidates)` and size bounds when `tally_present`.
- **S7** (count conservation): `inv_S7` — `ballots_tallied + ballots_dropped + non_voters.size() == candidates.size()`.
- **S8–S9**: **Omitted**. Require `per_round_counts` and `eliminated_by_round`. The `TallyResult` type is minimal because full tally computation is off-chain (Block E1) and B10 is cross-layer.
- **S10** (resolution ⇒ past `end_at`): `inv_S10` — `not(tally_present) or time >= end_at`.

## §3.2 Behavioral Invariant Encoding

All behavioral invariants are classical shadows (state predicates preserved by action guards).

- **B1** (monotone-once-set): `inv_B1` — `not(tally_present) or (tally_present and attestation_valid and tally_computed)`. Guards prevent unsetting `tally_present`.
- **B2** (no late ballots): Shadow `true`. `submit_ballot` requires `isVoting`; ballots frozen after `end_at`.
- **B3** (no premature tally): Same as S10. `publish_result` requires `isTallying`.
- **B4** (no premature voting): Shadow `true`. `submit_ballot` requires `isVoting`.
- **B5** (publish_result at most once): Shadow `true`. Guard requires `not(tally_present)`.
- **B6** (ballot writer is voter): Shadow `true`. `submit_ballot` requires `sender.in(candidates)`; no other action writes `ballots`.
- **B7** (terminal immutability): Shadow `true`. When `tally_present`, only `advance_time` is available and it does not mutate storage.
- **B8** (attestation-binds-tally): `inv_B8_shadow` — `not(tally_present) or attestation_valid`. Flag set exclusively in `publish_result`.
- **B9** (negligibility budget): **Omitted**. Meta-security with probabilistic quantifiers not encodable in Quint.
- **B10** (tally-correctness): `inv_B10_shadow` — `not(tally_present) or tally_computed`. Semantic equality is cross-layer; discharged in Lean.

## Non-Obvious Choices

1. **Set vs List for candidates**: The intent uses `Vec<Addr>` for deterministic serialization. Quint uses `Set[Addr]` because ordering is irrelevant to state-machine invariants; serialization is off-chain.

2. **Partial map encoding**: `ballots: Addr -> str` is total with `""` as absent. `ballot_keys` tracks the domain, avoiding `Option` (unavailable in Quint).

3. **TallyResult minimality**: `per_round_counts` and `eliminated_by_round` are omitted. The witnesses require reachability of states, not specific tally values. The well-formedness predicate checks the structural properties the chain validates (Block 6).

4. **Ghost flags**: `attestation_valid` and `tally_computed` exist solely to give B8 and B10 classical shadows, making cross-layer invariants inspectable.

5. **Discrete time**: `time` increments by 1 per `advance_time`, abstracting block time into a bounded integer space sufficient for state-machine transitions.
