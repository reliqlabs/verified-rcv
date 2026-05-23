# Cross-critique: gpt-5-5-native reviews canonical

## Q1. Most material structural divergence

The most material divergence is the published `TallyResult` shape. My spec kept the intent-level fields `per_round_counts: List[str -> int]` and `eliminated_by_round: List[List[str]]`, then encoded S8 and S9, weakly but explicitly, over those fields. The canonical replaces both with `rounds_played: int` and moves IRV-specific correctness out of the Quint state shape.

This matters because §2.5 and §3.1 make per-round counts and elimination monotonicity part of the published audit artifact, not merely off-chain proof metadata. If the protocol model omits those fields, Quint cannot express malformed round counts, candidate reappearance after elimination, or serialization-order obligations for those vectors. The canonical's `well_formed_tally` is cleaner for finite search, but it no longer models the full chain-side payload described by intent v0.3.2.

## Q2. Apparent defect in canonical

Defect: S8 and S9 are not actually encoded. The canonical `TallyResult` has no `per_round_counts` or `eliminated_by_round`, so `inv_published_well_formed` cannot check “each `per_round_counts[i]` sums to `ballots_tallied`” or “eliminated candidates never reappear.” Replacing those fields with `rounds_played` is technically valid Quint and still passes validation, but it weakens the intent schema and makes malformed audit traces unrepresentable rather than rejected.

## Q3. Change to your own spec after reading canonical

I would adopt the canonical's derived-phase structure and ghost snapshot naming. Specifically, replace my stored `phase`, `start_at`, `end_at`, and explicit `instantiate` transition with `block_time`, `contract_tally_result`, and `derived_phase(block_time, contract_tally_result)`, while keeping the full intent-level `TallyResult` fields. That would reduce phase/state skew and align B2 with v0.3.2's snapshot discipline.

## Optional notes

The canonical's B2 treatment is materially better than mine because it uses `ghost_ballots_at_end_at` as an explicit checkable anchor instead of relying on `time >= end_at` plus equality after a one-shot transition. The source comment still says intent v0.3.1, while the review target is v0.3.2; likely documentation drift, but worth fixing. The parameterized module also relies on `main.qnt` for candidate non-emptiness, distinctness, and a valid time window, rather than asserting S1-S3 inside `all_invariants`.
