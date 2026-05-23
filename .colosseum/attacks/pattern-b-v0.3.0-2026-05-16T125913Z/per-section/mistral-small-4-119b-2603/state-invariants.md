# mistral-small-4-119b-2603 — slice state-invariants

## Cross-section reads
- Pointer to S5/S10 restatements in §3.1 (lines 326–331): the slice under attack is exactly these lines; cross-section reads are for orientation only and not cited.

## Attacks on this slice
### 1. S5 write-discipline framing is still temporal-by-construction [critical]
- **Category**: temporal-state mismatch
- **Affected**: S5 restatement lines 326–327 (state-shape section)
- **What's wrong**: S5 claims to be a *pointwise* write-discipline property ("only Block 6 writes `tally_result`, gated on `is_none()`"), yet its justification still quantifies over *all successors* of a state where `tally_result.is_some()`. The invariant body reads:
  > "no handler writes to `tally_result` after it has been set to `Some(_)`"
  and the justification paragraph continues:
  > "This is a write-discipline property of the contract's handler set (only Block 6 writes `tally_result`, and Block 6's Requires `tally_result.is_none()` gates the write)."
  The phrase "after it has been set" and "any successor σ′" (in the preamble's intent) are temporal quantifiers, not pointwise evaluations. The spec therefore encodes a *temporal* invariant inside the state-shape section, violating the preamble "No quantification over operations or time" and the S5 restatement's own claim to be pointwise.
- **Cite**: > "S5 (derived) | terminality of resolution (state-shape) | **`tally_result: Option<TallyResult>` is set-once: no handler writes to `tally_result` after it has been set to `Some(_)`**. This is a *write-discipline* property of the contract's handler set (only Block 6 writes `tally_result`, and Block 6's Requires `tally_result.is_none()` gates the write)."
- **Fix recommendation**: Replace the temporal wording with a pure handler-index claim:
  - Add to §3.1: "S5 (derived) | terminality of resolution (state-shape) | `tally_result` is written at most once across all handlers; only Block 6 writes to `tally_result`, and its Requires clause ensures the write occurs only when `tally_result.is_none()`."
  - Remove any mention of "after it has been set" or successor quantification. Keep the handler-index justification in a single sentence without temporal quantifiers.

### 2. S10 derivation from B3 is not a state-shape invariant [serious]
- **Category**: coverage gap / refinement mismatch
- **Affected**: S10 lines 331–332 (state-shape section)
- **What's wrong**: S10 claims "if `tally_result.is_some()`, then `env.block.time ≥ end_at`" and labels it as a state-shape invariant. This is not evaluable on contract state alone; it requires reading `env.block.time` (chain environment) and comparing to `end_at` (contract storage). The state-shape section preamble explicitly says "Properties evaluable on contract state at any reachable moment." S10 therefore violates the preamble and is not a structural invariant. It should either be moved to §3.2 (behavioral/temporal) or restated as a state-shape claim about *stored* variables only (e.g., "the contract stores `end_at` and `tally_result`, and if `tally_result.is_some()` then the stored `end_at` is ≤ current block time" — but even that requires temporal reasoning).
- **Cite**: > "S10 (derived) | resolution implies past end_at | if `tally_result.is_some()`, then `env.block.time ≥ end_at`. **Derived from B3** (temporal causal version); listed for explicit state-shape reference per Round 3a first-pass adversary Attack 26."
- **Fix recommendation**: Move S10 to §3.2 under a new temporal invariant:
  - Add in §3.2: "B10½ | resolution implies stored end_at ≤ block_time | `always (tally_result.is_some() → env.block.time ≥ end_at)` — derived from B3 and evaluable on the stored `end_at` variable plus chain environment."
  - Alternatively, if you insist on keeping it in §3.1, restate as a state-shape claim about stored variables only:
    "S10 (derived) | resolution implies end_at ≤ block_time at publish time | if `tally_result.is_some()`, then the stored `end_at` field satisfies `end_at ≤ env.block.time` at the moment of write (Block 6)." This still requires temporal reasoning to discharge, so §3.2 is the honest placement.

### 3. S6–S9 annotations do not cleanly separate state-shape from temporal complements [serious]
- **Category**: refinement mismatch
- **Affected**: S6–S9 lines 327–330 (state-shape section)
- **What's wrong**: Each of S6–S9 includes an annotation:
  > "Pointwise evaluable per S6's note."
  and then references a temporal complement (e.g., "the temporal write-discipline complement (each `publish_result` write satisfies this) is captured by Block 6's Requires + B10's correctness obligation"). This blurs the line between state-shape invariants and their temporal complements. The state-shape section should list only pointwise-evaluable claims on stored variables; temporal complements belong in §3.2 or in the handler Requires clauses.
- **Cite**: > "S6 | winner well-formedness | if `tally_result.is_some()`, then `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`. **Pointwise evaluable**; the temporal write-discipline complement (each `publish_result` write satisfies this) is captured by Block 6's Requires + B10's correctness obligation (Round 3a first-pass adversary Attack 7)."
- **Fix recommendation**: For each S6–S9:
  - Keep only the pointwise state-shape claim in §3.1 (e.g., "S6 | winner well-formedness | `winners` field, when present, satisfies `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`").
  - Move the temporal complement to a separate bullet under each invariant's justification, clearly labeled "Temporal complement (handled by Block 6 Requires)" and remove the cross-reference to B10's correctness (which is a temporal claim, not a handler Requires).
  - Alternatively, drop the "Pointwise evaluable" note and simply state each invariant as a pointwise claim on stored fields, without mixing in temporal complements.

### 4. S5/S10 restatements omit the handler-index table promised by "set-once write-discipline" [cosmetic]
- **Category**: triviality
- **Affected**: S5/S10 lines 326–332 (state-shape section)
- **What's wrong**: The S5 restatement claims to be a "write-discipline" property but does not enumerate which handlers write `tally_result` nor their preconditions. The table promised by "set-once write-discipline" is missing; the invariant body does not list handlers at all. This makes the claim weaker than intended and harder to discharge in downstream specs.
- **Cite**: > "S5 (derived) | terminality of resolution (state-shape) | **`tally_result: Option<TallyResult>` is set-once: no handler writes to `tally_result` after it has been set to `Some(_)`**. This is a *write-discipline* property of the contract's handler set (only Block 6 writes `tally_result`, and Block 6's Requires `tally_result.is_none()` gates the write)."
- **Fix recommendation**: Add a short handler-index table inline:
  - "S5 (derived) | terminality of resolution (state-shape) |
    Handlers:
    - Block 6 (`publish_result`): writes `tally_result` only when `tally_result.is_none()` (Requires).
    - All other handlers: no write to `tally_result`.
    Therefore `tally_result` is written at most once."

## Slice-local summary
- Critical: 1
- Serious: 2
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE