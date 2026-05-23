**Mapping §2.5 behavior blocks → actions**

| Block | Intent description | Quint encoding |
|------|--------------------|----------------|
| Instantiate (Block 1) | Initialise immutable parameters, empty ballot map, `tally_result = None`. | `action init` writes `state` with the const‑instantiated values and sets `tally_result` to `{present:false,…}`. |
| Implicit time‑advance | Drives the derived phases **Created → Voting → Tallying → Resolved**. | `action tick` nondeterministically adds a small integer to the global `time`. The pure helper `derived_state` computes the current phase from `state.start_at`, `state.end_at`, and `state.tally_result.present`. |
| SubmitBallot (Block 3) | Candidates may write (or overwrite) their encrypted ballot while voting is open. | `action submit_ballot` picks a `sender` from `state.candidates`, checks `derived_state == "Voting"`, and updates `state.ballots` with `set(sender, ct)`. |
| CloseAndTally (Block 5) | Emits an off‑chain event; no on‑chain state change. | `action close_and_tally` only guards `derived_state == "Tallying"`. |
| PublishResult (Block 6) | Verifies a placeholder attestation, checks well‑formedness, then stores the result atomically. | `action publish_result` guards the same phase, ensures `state.tally_result.present == false`, asserts that every winner is a declared candidate, and writes `{present:true, value:tr}`. A tiny `sample_tallies` set provides a concrete `TallyResult` for model‑checking. |

**Encoding §3.1 structural invariants**

All six pointwise predicates (`inv_*`) are pure booleans that can be evaluated on any reachable state:

* `inv_nonempty_candidates` – `size(state.candidates) ≥ 1`.
* `inv_distinct_candidates` – trivially true because `candidates` is a `Set`.
* `inv_window_order` – `state.start_at < state.end_at`.
* `inv_ballot_keys_are_candidates` – universal quantification over `state.ballots.keys`.
* `inv_winner_subset` – conditional on `state.tally_result.present`; uses `all w in … => …`.
* `inv_ballot_conservation` – conditional on `present`; adds the three counts and checks they equal the number of candidates.

The composite `all_invariants` is simply the conjunction of these six booleans, satisfying the dispatcher’s required name.

**Encoding §3.2 behavioral witnesses**

The three mandated witnesses are *negated* reachability properties, i.e. they are false in at least one execution:

* `witness_resolution_reachable` asserts `!state.tally_result.present`. A `publish_result` step makes the predicate false.
* `witness_ballot_submittable` asserts the ballot map stays empty. The `submit_ballot` action falsifies it.
* `witness_end_at_crossing` asserts `time < END_AT`. The `tick` action can advance `time` past `END_AT`.

Quint treats a `val name: bool` as “holds in every reachable state”, so each of these booleans will be shown false by the model‑checker, providing the required counterexample traces.

**Omitted invariants and why**

* Temporal invariants B1–B5 (e.g. monotonicity of `tally_result`) are omitted because they require an LTL‑style step relation that is already enforced by the single‑writer discipline in `publish_result`.  
* B6 (ballot‑writer attribution) is captured implicitly: only a sender that is a declared candidate can execute `submit_ballot`, and the map update stores the ballot under exactly that sender’s address, satisfying the “unique writer” clause.  
* Cryptographic claims B8/B9 and the cross‑layer correctness invariant B10 are off‑chain; their assumptions appear in the trust‑boundary section but are not expressed as on‑chain Quint predicates.  

**Non‑obvious encoding choices**

* **Option encoding** – Quint lacks a built‑in `Option` type, so we introduced a generic record `Opt[T] = {present: bool, value: T}` and used `present`/`value` fields throughout.  
* **Set vs. List** – The intent specifies `Vec` for ordered collections; on‑chain safety does not depend on ordering, so we model them as `Set` (candidates, winners) and `List` only where the type is required (`per_round_counts`, `eliminated_by_round`).  
* **Placeholder ciphertexts** – The model does not inspect ballot contents; a small finite set of dummy strings suffices to make the state space bounded.  
* **`tick` nondeterminism** – A small integer set `{0,1,2,5,10}` keeps the reachable‑state graph finite while still allowing the three witnesses to be exercised.  
* **`default_tally`** – Required to initialise the optional field; its contents are never used because `present` is false until `publish_result` overwrites it.

**Summary of block‑to‑action mapping**

* Block 1 → `init`  
* Implicit time progression → `tick`  
* Block 3 → `submit_ballot`  
* Block 5 → `close_and_tally`  
* Block 6 → `publish_result`  

All mandatory invariant names (`all_invariants`, `witness_resolution_reachable`, `witness_ballot_submittable`, `witness_end_at_crossing`) are present, and the specification follows the loading‑bearing idiom demonstrated in the canonical `reactor.qnt` example.  

STATUS: ok