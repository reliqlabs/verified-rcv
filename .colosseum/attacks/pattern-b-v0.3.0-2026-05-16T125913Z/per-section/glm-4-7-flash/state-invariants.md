I'll read the target spec at the specified slice ranges and then produce an aggressive attack report.
# glm-4-7-flash — slice state-invariants

## Cross-section reads
- Section 2.5 Block 6 Requires (lines 262-266) — defines set-relation properties that are not listed as structural invariants
- Section 3.2 B1 (line 339) — temporal value-stability claim that S5 is claimed to derive from
- Section 3.2 B3 (line 341) — temporal causal claim that S10 is claimed to derive from

## Attacks on this slice

### 1. S5 is not pointwise evaluable [serious]
- **Category**: temporal-state-mismatch
- **Affected**: S5
- **What's wrong**: S5 claims to be evaluable on a single state by inspecting the handler index, but the handler index is not part of the contract state. The state variables are `candidates`, `start_at`, `end_at`, `enclave_pubkey`, `ballots`, and `tally_result`. To evaluate S5 at a state σ, you'd need to know whether any handler could write to `tally_result` in any successor state — that's a successor quantification over the handler set, not a pointwise state property. This violates Section 3.1's preamble "No quantification over operations or time."
- **Cite**: 
  - S5 statement: "Evaluable on a single state by inspecting the handler index, not by quantifying over successor states."
  - Section 3.1 preamble: "Properties evaluable on contract state at any reachable moment. No quantification over operations or time."
  - State variables (Section 2.5): "candidates: Vec<Addr>", "start_at: Timestamp", "end_at: Timestamp", "enclave_pubkey: PubKey", "ballots: Map<Addr, Vec<u8>>", "tally_result: Option<TallyResult>"
- **Fix recommendation**: Move S5 to Section 3.2 as a temporal invariant (matching B1), or restate it as a pointwise property like "only Block 6 writes to tally_result" and make this a precondition on the handler set (not a state invariant).

### 2. S5 derivation from B1 is unsound [serious]
- **Category**: impossibility-hypothesis vacuity
- **Affected**: S5 (derived flag)
- **What's wrong**: S5 is marked as derived from B1, claiming that the set-once write-discipline is the pointwise version of B1. But B1 says "once tally_result is Some, it stays Some with the same value forever" (value-stability), while S5 says "no handler writes to tally_result after it has been set" (write-discipline). These are different properties. B1 could be satisfied by a handler that writes `Some(tally)` when it's already `Some(tally)` — the value doesn't change, so B1 holds, but S5 is violated because a handler wrote. The derivation S5 ← B1 is therefore unsound.
- **Cite**: 
  - S5 statement: "This is a write-discipline property of the contract's handler set (only Block 6 writes tally_result, and Block 6's Requires tally_result.is_none() gates the write)."
  - B1 statement: "always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))"
- **Fix recommendation**: Remove the "derived" flag from S5. S5 is an independent invariant that needs to be proved directly from the handler set specification, not derived from B1.

### 3. Coverage gap — set-relation invariants not listed in S1-S10 [serious]
- **Category**: coverage gap
- **Affected**: S1-S10
- **What's wrong**: Several set-relation properties are mentioned in Block 6's Requires clause but not listed as structural invariants in S1-S10:
  - `eliminated_by_round ⊆ candidates`
  - `dropped_voters ⊆ ballots.keys`
  - `non_voters = candidates \ ballots.keys`
  - `len(dropped_voters) = ballots_dropped`
These are mentioned in Block 6's Requires (lines 262-266) but not in S1-S10. This creates a coverage gap where the spec names these properties in the behavioral context but doesn't list them as structural invariants.
- **Cite**: 
  - Block 6 Requires: "winners ⊆ candidates, 1 ≤ len(winners) ≤ len(candidates)", "per-round counts internally consistent", "elimination sequence is monotone", "conservation: ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)", "set relations: non_voters = candidates \ ballots.keys, dropped_voters ⊆ ballots.keys, dropped_voters ∩ non_voters = ∅. len(dropped_voters) = ballots_dropped."
  - S1-S10: No mention of these set-relation properties.
- **Fix recommendation**: Add these as structural invariants in S1-S10, or explain why they're intentionally omitted (e.g., they're behavioral invariants that only apply after a successful `publish_result` transition).

### 4. S10 derivation from B3 relies on implicit assumption [serious]
- **Category**: under-specification
- **Affected**: S10 (derived flag)
- **What's wrong**: S10 is marked as derived from B3, claiming that resolution implies past end_at. The derivation relies on the implicit assumption that the only way to reach a state where `tally_result.is_some()` is through a transition from a state where `tally_result.is_none()` (i.e., through Block 6). This assumption is true in the contract design, but it's not stated explicitly in the intent document. Without this assumption, B3 does not imply S10.
- **Cite**: 
  - S10 statement: "if tally_result.is_some(), then env.block.time ≥ end_at."
  - B3 statement: "always (next.tally_result.is_some() ∧ tally_result.is_none() → env.block.time ≥ end_at)"
  - Block 6: "Produces: tally_result := Some(tally); ballots become read-only audit data."
- **Fix recommendation**: Make the assumption explicit by adding a note that S10 follows from B3 under the assumption that `tally_result` can only be set through Block 6, which is the only handler that writes to `tally_result`.

## Slice-local summary
- Critical: 0
- Serious: 4
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE