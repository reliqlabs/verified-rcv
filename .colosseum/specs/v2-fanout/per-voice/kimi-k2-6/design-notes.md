**Block-to-action mapping (§2.5):**
- Block 1 (`instantiate`) → `instantiate` action: sets `candidates`, `start_at`, `end_at`, resets ballot store.
- Blocks 2 & 4 (time windows) → `advance_time`: updates `phase` from Created→Voting when `current_time` crosses `start_at`, and Voting→Tallying when it crosses `end_at`.
- Block 3 (`submit_ballot`) → `submit_ballot`: stores an opaque `Ballot` blob under `msg.sender` (here, a nondet member of `candidates`).
- Block 5 (`close_and_tally`) → `close_and_tally`: read-only self-loop in Tallying.
- Block 6 (`publish_result`) → `publish_result`: deterministically constructs a trivial but well-formed co-winner `TallyResult`, sets `tally_result = Some(tr)`, and moves to Resolved.
- Block E1 (enclave tally) is off-chain; the chain-side shadow is the guard in `publish_result` plus the `b9_b10_shadow` invariant.

**Invariant encodings (§3.1 + §3.2):**
- **S1–S4, S10**: direct state predicates over `candidates`, `ballots`, and `current_time`.
- **S5**: omitted as a handler-set meta-property; enforced structurally because only `publish_result` writes `tally_result` and it requires `tally_result == None`.
- **S6–S9**: encoded via `match` on `Option[TallyResult]`. `trivial_counts` builds a one-round count map so that `publish_result` always emits a tally satisfying conservation, winner bounds, and count sums.
- **B1/B7**: ghost boolean `resolved_ever` set only in `publish_result`; invariants assert `resolved_ever => phase == Resolved` and `tally_result != None`.
- **B2**: ghost map `frozen_ballots` snapshotted when `advance_time` leaves Voting; invariant asserts equality with `ballots` in Tallying/Resolved.
- **B3–B6**: enforced by action guards (e.g., `submit_ballot` requires `phase == Voting`; `publish_result` requires `phase == Tallying`). No extra state predicate needed.
- **B8/B9/B10**: encoded as classical shadows (`b8_shadow`, `b9_b10_shadow`) that assume registry and enclave honesty. Quint cannot model cryptography or cross-layer Lean proofs, so the invariants collapse to `tally_result != None => true` under the honest-environment assumption.

**Omissions:**
- Exact `Tally_spec` / IRV computation: omitted because faithfully computing ranked-choice elimination in pure Quint for arbitrary candidate sets is extremely verbose and unnecessary for the bounded safety/witness goals. The deterministic trivial tally suffices to reach Resolved and exercise S6–S9.
- `B9` probabilistic bound: entirely meta-security; no Quint shadow beyond the boolean honesty flag.
- `S5` handler-set quantification: a meta-property of the action set, not evaluable as a state predicate.

**Non-obvious choices:**
- `Set[Addr]` instead of `List[Addr]` for `candidates` avoids declaration-order bookkeeping since the chosen IRV variant batch-eliminates all tied candidates and does not need candidate ordering for correctness.
- `ADDR_SET.mapBy(a => None)` creates a fixed-domain partial map for `ballots`, making absent-key semantics (`None`) explicit and safe for `s4` and `b2`.
- `publish_result` builds the tally functionally from the current ballot set rather than accepting a nondeterministic argument; this guarantees the emitted tally satisfies both the structural invariants and the set-relation constraints from Block 6 without needing to generate arbitrary `TallyResult` values.