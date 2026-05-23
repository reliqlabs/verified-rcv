# gpt-oss-120b — slice behaviors‑core

## Cross-section reads
- None needed; all attacks are grounded within the target slice.

## Attacks on this slice
### 1. Missing result fields in boundary‑case examples [critical]
- **Category**: under‑specification  
- **Affected**: §2.2 Boundary cases – “Single candidate” and “Two candidates, both vote” examples  
- **What's wrong**: The worked examples give only `winners`, `per_round_counts`, and `eliminated_by_round`. They omit the mandatory `ballots_dropped`, `dropped_voters`, and `non_voters` fields required by the `TallyResult` schema (see §2.5). An implementation that follows the examples verbatim could produce a `TallyResult` lacking these fields, violating invariant S7 (tally count conservation) and S8 (per‑round count consistency).  
- **Cite**: > “- **Single candidate (`len(candidates) = 1`)** … Result: `winners = [that_candidate]`, `per_round_counts = [{that_candidate: ballots_tallied}]`, `eliminated_by_round = []`.” (lines 69‑71)  
> “- **Two candidates, both vote, terminal 1‑1 tie** … Result: `winners = [both candidates]`, `per_round_counts = [{A: 1, B: 1}]`, `eliminated_by_round = []`.” (lines 71‑72)  
- **Fix recommendation**: Extend each boundary‑case example to list all `TallyResult` fields, e.g. `ballots_dropped = 0`, `dropped_voters = []`, `non_voters = []` (or the appropriate set), ensuring conformity with the schema and downstream invariants.

### 2. Unspecified tie‑elimination when all survivors tie with non‑zero votes [serious]
- **Category**: coverage gap / edge case  
- **Affected**: §2.3 Edge cases – “Late ballot …”, “Ballot revision …”, “Malformed ballot …” list; no entry for the situation where after some eliminations the remaining candidates all have equal *non‑zero* first‑place counts, and batch‑elimination would empty the set.  
- **What's wrong**: The spec’s batch‑elimination rule (Section 2.5) says “eliminate all candidates tied for lowest”. If the remaining set is `{X, Y}` each with 2 votes, the rule would eliminate both, leaving no candidates – an undefined state not covered by any example. Implementations may either (a) declare both co‑winners (contradicting the “lowest‑count” rule) or (b) abort with an unspecified error, leading to divergent behaviours.  
- **Cite**: > “### 2.3 Edge cases” (lines 79‑93) – the listed edge cases do not mention this scenario.  
- **Fix recommendation**: Add an explicit edge‑case description (e.g., “All remaining candidates tie with equal non‑zero first‑place votes → declare all as co‑winners”) and update the batch‑elimination policy to handle the empty‑set outcome gracefully.

### 3. Ambiguous error semantics for out‑of‑window ballot submission [serious]
- **Category**: ambiguity  
- **Affected**: §2.3 Edge case “Late ballot during Tallying” and Block 3 precondition failures.  
- **What's wrong**: The edge case states that a ballot submitted after `end_at` is rejected with `NotInVotingWindow`. Block 3’s `submit_ballot` handler also maps any submission outside the voting window (including *pre‑window* submissions) to the same `NotInVotingWindow` error. Consequently, a caller cannot distinguish whether a ballot was early or late, leaking timing information and making debugging harder. Moreover, the intent’s “Late ballot” description suggests a distinct failure mode for late submissions, which the spec does not honour.  
- **Cite**: > “- **Late ballot during Tallying** — `submit_ballot` after `end_at` is rejected with `NotInVotingWindow`.” (line 81)  
> “- **Requires:** … `env.block.time ≥ start_at` … **Forbids:** … `env.block.time < start_at` … **On precondition failure:** … state ≠ Voting → `NotInVotingWindow`.” (lines 215‑227)  
- **Fix recommendation**: Introduce a separate error variant, e.g., `LateBallot`, for submissions after `end_at`, while retaining `NotInVotingWindow` for pre‑window attempts. Update the edge‑case description and Block 3’s error mapping accordingly.

## Slice‑local summary
- Critical: 1  
- Serious: 2  
- Cosmetic: 0  

## VERDICT (slice‑local): BREAKS‑AT‑SLICE | SURVIVES‑SLICE | INDETERMINATE
BREAKS‑AT‑SLICE (the slice contains critical and serious under‑specifications and ambiguities that invalidate the worked‑example contract between intent and spec).