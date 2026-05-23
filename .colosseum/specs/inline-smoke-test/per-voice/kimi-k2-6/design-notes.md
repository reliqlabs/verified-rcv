**Block-to-action mapping (§2.5)**
- Block 1 (`instantiate`) → `instantiate` action: sets `initialized`, seeds constants.
- Block 2/4 (time advance) → `tick` action: increments `block_time`; derived states (Created/Voting/Tallying) are guards on other actions.
- Block 3 (`submit_ballot`) → `submit_ballot(sender,prefs)`: requires `START_AT ≤ block_time < END_AT` and `sender ∈ candidates`.
- Block 5 (`close_and_tally`) → `close_and_tally` no-op action: requires Tallying, preserves state.
- Block 6 (`publish_result`) → `publish_result`: deterministically computes `tally_spec(ballots, CANDIDATE_LIST)` and stores it. This encodes B10’s classical shadow: the chain only accepts the semantically correct result.
- Block E1 (enclave) is elided; its effect is the pure def `tally_spec`.

**Invariant encoding**
- **S-series (structural):** S1–S3 are const-level predicates; S4, S6–S10 are state predicates evaluated over current `tally_result` and `ballots`. S5 (handler-set write discipline) is enforced by construction—only `publish_result` writes `tally_result` and only when `None`.
- **B-series (temporal):** Encoded via explicit history variables (`prev_ballots`, `prev_tally`, `prev_block_time`, `last_action`, `last_sender`) updated on every transition. B1 (`tally_result` immutable once set) becomes `prev_tally.isSome() ⇒ tally_result == prev_tally`. B2 (frozen ballots after `end_at`) becomes `prev_block_time ≥ END_AT ⇒ ballots == prev_ballots`. B3/B4 use the same history pattern. B6 uses `last_action`/`last_sender` to attribute each ballot change to a unique `SubmitBallot` event.
- **B8/B9/B10:** Quint cannot model cryptography or probabilities. B8 is reduced to the classical shadow `tally_result.isSome() ⇒ REGISTRY_HONEST`, enforced by the `publish_result` guard. B9 is omitted entirely (meta-security/probabilistic). B10 is enforced by construction: `publish_result` stores exactly `Some(tally_spec(...))`, making the tally correctness a deterministic state更新 rather than an off-chain existential.
- **B10_lean:** Omitted—Lean discharge is outside Quint’s scope.

**Omissions**
- Encryption/decryption is abstracted away: ballots are stored as plaintext `List[str]` rankings; the ECIES layer is trusted to round-trip correctly via the off-chain Quartz theorem.
- `DstackAttestation`, `zkdcap`, and SHA-256 hashing are not modeled; B8 clauses (a)–(d) collapse to the boolean `REGISTRY_HONEST`.
- Canonical serialization order and `ballots@end_at` are implicit in the deterministic pure def `tally_spec`; no explicit `end_at` snapshot variable is needed because B2 guarantees immutability.
- `Set<str>` is used instead of `Vec<Addr>` where declaration order is not load-bearing for the Quint model checker (structural invariants care about membership, not serialization).

**Witnesses**
- Each witness is the negation of the desired reachability property, so `quint run` produces a violating trace that demonstrates the state is reachable.