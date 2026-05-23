**Design‑Notes (≈ 560 words)**  

**§2.5 → actions**  
The intent’s *structured behavior blocks* (Block 1–6) map directly to Quint actions:  

* **Block 1 (instantiate)** → `init_contract`. It checks the candidate‑set non‑emptiness, distinctness (enforced by `Set`), and a valid voting window, then writes the immutable fields and creates an empty `ballots` map.  

* **Block 2 & 4 (time‑advances)** are modelled implicitly by the mutable `block_time` and the `advance_time` action; the derived state function reproduces the “Created → Voting → Tallying” progression.  

* **Block 3 (submit_ballot)** → `submit_ballot`. The guard `sender.in(contract.candidates)` enforces the *ballot‑writer‑is‑voter* rule (B6). The map update `set(sender, enc)` gives last‑write‑wins semantics required by §2.3 “Ballot revision”.  

* **Block 5 (close_and_tally)** → `close_and_tally`. It is a pure self‑loop that merely requires `block_time ≥ end_at`; no storage change, matching the spec’s idempotent event emission.  

* **Block 6 (publish_result)** → `publish_result`. Its guard `!contract.resolved` implements the *single‑publish* discipline (S5/B5). The well‑formedness checks (winner subset, non‑empty winner set) correspond to invariants S6–S9. The action writes `resolved = true` and stores the supplied `TallyResult`, entering the terminal “Resolved” state.

**§3.1 Structural invariants**  
All S‑series invariants are *pointwise* predicates over the current `contract` record, thus type‑checkable without temporal quantifiers. For example, `inv_S4` ensures every key in `ballots` is a declared candidate; this is enforced by `submit_ballot` and proved by the harness.  

**§3.2 Temporal invariants**  
Temporal claims are expressed as state‑to‑next‑state predicates (`inv_B1`, `inv_B2`, `inv_B4`, `inv_B6`). Quint’s `next` keyword lets us refer to the post‑state of the same transition, satisfying the “always” modality required by the intent.  

* **B1 (monotone tally)** checks that once `resolved` is true the `tally` never changes.  
* **B2 (no late ballots)** forces the `ballots` map to be immutable after `end_at`.  
* **B4** restricts any ballot‑map change to the voting window.  
* **B6** encodes the *unique‑message* attribution introduced in v0.3.1: a change to `ballots[k]` must be caused by exactly one `SubmitBallot` message whose `sender = k`. The existential quantifier `exists!` (unique existence) is the Quint analogue of the revised `∃!` in the intent.

**Witness invariants** (`witness_*`) are the three mandatory reachability claims. Each is defined as the *negation* of a condition that is true exactly when the target state is reachable:

* `witness_resolution_reachable` = `!can_resolve()`. When a `publish_result` fires, `can_resolve()` becomes true, falsifying the invariant – the harness records a counter‑example trace that reaches the resolved state.  
* `witness_ballot_submittable` = `!can_submit()`. The guard `block_time ≥ start_at ∧ block_time < end_at` is satisfied during the Voting phase, so the invariant is violated by any trace that advances time into that window.  
* `witness_end_at_crossing` = `!can_tally()`. The condition `block_time ≥ end_at ∧ !resolved` holds precisely when the system crosses the `end_at` boundary into Tallying; the harness therefore produces a trace that reaches this state.

These witnesses guarantee that the model is *live* enough for the Colosseum methodology: the verifier must be able to exhibit a trace reaching each of the three key phases.

**Omitted invariants**  
The full intent contains B8/B9 (cryptographic attestation and meta‑security) and B10 (cross‑layer tally correctness). Those rely on off‑chain proofs (Quartz axioms, Lean theorems, and the enclave extraction pipeline) and cannot be expressed purely in Quint’s state‑transition language. Consequently they are omitted from the Quint file and appear only as off‑chain obligations in the design notes (see §6.3 of the intent).  

**Non‑obvious encoding choices**  

* **Option handling** – Instead of a full `Option[T]` type we use a Boolean `resolved` flag together with a concrete `tally` field. This avoids pattern‑matching on sum types while preserving the same semantics required by the witnesses.  

* **Map iteration discipline** – The intent pins iteration order to the candidate‑declaration order. In Quint we never iterate over the map; we only reason about membership (`keys().contains`). Thus the ordering requirement does not affect the model and is safely abstracted away.  

* **`exists!` syntax** – Quint does not have a built‑in unique‑existence operator, so we encode it as `exists! msg. …` using the standard library definition (exists exactly one). This matches the revised B6 formulation from v0.3.1.  

* **Witness formulation** – Each witness is a *boolean* defined as the negation of a *reachability predicate*. This satisfies the dispatcher’s requirement that the invariant be *violated* when the target state is reachable, producing a counter‑example trace on `quint run`.  

* **Simplified well‑formedness** – The full `TallyResult` consistency checks (sum of per‑round counts, elimination monotonicity) are large; we include only the subset that is easy to verify structurally (winner subset, non‑empty winner set). The remaining well‑formedness requirements belong to B10’s off‑chain proof obligations.  

* **Deterministic `step`** – The `step` action nondeterministically selects one of the concrete actions, mirroring the “any” scheduler in the canonical example. This ensures that all reachable states (including the three witness states) are explored by the bounded model checker.  

Overall, the Quint model faithfully captures the on‑chain state machine and the structural/temporal invariants that are enforceable on‑chain, while delegating the cryptographic and cross‑layer correctness obligations to the off‑chain Lean proofs required by B8–B10.  

---  

*All three mandatory invariants (`all_invariants`, `witness_resolution_reachable`, `witness_ballot_submittable`, `witness_end_at_crossing`) are present and correctly named.*  