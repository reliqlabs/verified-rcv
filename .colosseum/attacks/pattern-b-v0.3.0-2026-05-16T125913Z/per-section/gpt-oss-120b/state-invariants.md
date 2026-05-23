# gpt-oss-120b — slice state-invariants

## Cross-section reads
- B2 (temporal invariant “no late ballots”) – shows that ballot‑immutability after `end_at` is only expressed temporally, yet a state‑shape invariant is missing from this slice.

## Attacks on this slice
### 1. Temporal‑state mismatch in S5 [serious]
- **Category**: temporal-state mismatch
- **Affected**: S5 (derived) – line 326‑327
- **What's wrong**: S5 is presented as a *structural* (state‑only) invariant: “`tally_result: Option<TallyResult>` is set‑once … Evaluable on a single state by inspecting the handler index”. The property actually constrains *future* writes, i.e. it is a temporal guarantee about the absence of any later transition that mutates `tally_result`. A pure state predicate cannot capture “no handler writes after the value becomes `Some`” without referring to code. Hence the clause violates the slice’s own preamble (“Properties evaluable on contract state at any reachable moment. No quantification over operations or time.”) and is a classic temporal‑state mismatch.

- **Cite**: > **`tally_result: Option<TallyResult>` is set‑once: no handler writes to `tally_result` after it has been set to `Some(_)`.** … *Evaluable on a single state by inspecting the handler index* (lines 326‑327).

- **Fix recommendation**: Move S5 to the **Behavioral invariants** section and tag it `temporal`. Replace the wording with a temporal formula (e.g. “always (tally_result.is_some() → always (tally_result = tally_result))” or reference B1) and keep a separate structural invariant that merely states “`tally_result` is either `None` or a well‑formed `TallyResult`”.

### 2. Ambiguity of pointwise evaluability for S5 [serious]
- **Category**: ambiguity
- **Affected**: S5 – line 326‑327
- **What's wrong**: The clause claims pointwise evaluability “by inspecting the handler index”. The handler index is *metadata* about the contract’s code, not part of the on‑chain state. A verifier that only has access to the contract’s storage cannot decide the invariant without loading the source. This creates an ambiguous interpretation: either the invariant is meant to be a meta‑property (outside the model) or it is mistakenly classified as a state invariant. The spec does not clarify how a state‑only checker should treat this clause.

- **Cite**: > **Evaluable on a single state by inspecting the handler index** (lines 326‑327).

- **Fix recommendation**: Either (a) remove the “pointwise evaluable” claim and place the invariant in a separate *code‑discipline* section, or (b) provide an explicit formal definition that can be checked purely from storage (e.g., “`tally_result` is `None` or, if `Some`, the contract’s code contains exactly one `publish_result` transition that writes it”).

### 3. Coverage gap: missing state‑shape invariant for ballot immutability [serious]
- **Category**: coverage gap
- **Affected**: structural invariants slice (absence of a clause)
- **What's wrong**: The intent and behavioral invariants (B2) assert that after `end_at` the ballot map is frozen (“no late ballots”). However, there is **no corresponding structural invariant** in §3.1 that captures this *state‑shape* property (e.g., “`ballots` is immutable once `env.block.time ≥ end_at`”). The omission allows a future implementation to satisfy all listed S‑clauses while still permitting a handler that mutates `ballots` after the voting window, violating the intended security model without triggering any S‑clause failure.

- **Cite**: (implicit – the slice ends at line 332; no entry mentions ballot immutability).

- **Fix recommendation**: Add a derived structural invariant, e.g.:

  ```
  | S11 (derived) | ballot immutability after voting | if `env.block.time ≥ end_at` then `ballots` is read‑only (no handler writes to `ballots`).
  ```

  Tag it as `state‑only` and ensure it is pointwise evaluable (no code inspection needed).

### 4. Potential violation of S10 under blockchain timestamp re‑orgs [serious]
- **Category**: edge case
- **Affected**: S10 – line 331
- **What's wrong**: S10 states “if `tally_result.is_some()`, then `env.block.time ≥ end_at`”. This assumes that block timestamps are monotonic after the resolution transition. In practice, a chain can experience a *re‑org* where a later block with a lower timestamp replaces the block containing the `publish_result` transaction, causing `env.block.time` at the new tip to be earlier than `end_at`. The invariant would then be falsified even though the contract behaved correctly at the time of the transition. The spec does not bound timestamps nor mention re‑org safety, leaving a critical edge case uncovered.

- **Cite**: > **if `tally_result.is_some()`, then `env.block.time ≥ end_at`.** (line 331).

- **Fix recommendation**: Strengthen S10 to reference the *timestamp at the transition* rather than the current tip, e.g.:

  ```
  if `tally_result.is_some()` then the timestamp of the block in which `tally_result` became `Some` satisfies `≥ end_at`.
  ```

  Alternatively, add a clause that the chain guarantees monotonic timestamps for finalized blocks, and note that S10 holds under that guarantee.

## Slice-local summary
- Critical: 0  
- Serious: 4  
- Cosmetic: 0  

## VERDICT (slice-local): BREAKS-AT-SLICE | SURVIVES-SLICE | INDETERMINATE
BREAKS-AT-SLICE