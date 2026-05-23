# Adversarial review — verified-rcv intent doc, first draft

- **Target**: `/Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md` (2026-05-14 elicitation pass)
- **Reviewer**: `colosseum-spec-adversary` (Round 3a methodology dogfood)
- **Mode**: blind to `examples/ranked-choice/**`; cross-verified Quartz inheritance claims against `proofs/lean/Specs/Quartz/**` and `.colosseum/ledger.md` only.

```
VERDICT: BREAKS  (multiple critical findings — fabricated Quartz inheritance row, retracted-substrate B9, terminal-tie scenario contradicts S6, batch-elim conservation pivot under-specified)
```

## Summary by category and severity (methodology back-port signal)

| Category | Critical | Serious | Cosmetic | Notes |
|---|---|---|---|---|
| Composition failure | 3 | 2 | 0 | Section 6.2 row 3 (`DstackKeyManager`) is fabricated; B9 inherits a retracted scaffold; summand list disagrees with Quartz |
| Impossibility-hypothesis vacuity | 1 | 1 | 0 | B8 clause (c) hash is unidentified; verified-rcv silently consumes Quartz's `(d-pigeonhole-impossible)` `commitHashE` without restatement |
| Disjunction-vs-decomposition | 2 | 1 | 0 | B9's 5 summands double-count enclave-key-leak and silently drop two summands Quartz expands |
| Temporal-state mismatch | 1 | 2 | 1 | S5/B1 redundancy is honest but S6/S7/S8/S9 are mis-tagged as `state` when they are observably temporal at write |
| Coverage gap | 2 | 3 | 0 | Section 2.1 happy path has an arithmetic error; `InvalidTally` deadlock loop missed; no "TallyResult" type defined |
| Edge case | 1 | 4 | 1 | Terminal 1-1 tie, 1-candidate abstention, ballots conservation @ N=0, `end_at` off-by-one |
| Contradiction | 2 | 1 | 0 | 2.1 round-1 row contradicts the prose ("B has 0 first-place votes"); 2.2 terminal tie says co-winners but S6/IRV semantics conflict |
| Under-specification | 1 | 3 | 0 | "Ballot revision" never resolves how the enclave deduplicates by `msg.sender`; well-formedness predicate informal |
| Over-specification | 0 | 2 | 1 | 1M gas bound, "N ≤ 20–30 candidates" estimate, and "winners ⊆ candidates ∧ len(winners) ≥ 1" forces a winner |
| Preconditional over-strength | 0 | 1 | 0 | B8 inherits `tdxVerifier.sound`'s classical-Prop form without preserving the preconditional gap |
| Triviality | 0 | 1 | 1 | B6 reduces to "Block 3 rejects non-candidates" — tautological given the Requires clause |
| Ambiguity | 0 | 3 | 1 | Section 2.5 Forbids/Requires diverge from Block 5/6 prose; "spec violation, not just a failed transaction" |
| **Totals** | **13** | **24** | **5** | 42 findings |

---

## Attack 1 — happy-path table contradicts its own prose

- **Category**: Contradiction
- **Severity**: critical
- **Scenario**: Section 2.1 (line 32) lists Round 1 counts as `A=2 B=0 C=1 D=1 E=1`. Five candidates each cast one ballot. The total first-place votes is therefore 5. But Sum(2+0+1+1+1) = 5 and the table says A has 2 first-place votes. From the listed ballots:
  - A's ballot first choice = A
  - B's ballot first choice = A ("B endorses A as first choice")
  - C's ballot first choice = C
  - D's ballot first choice = D
  - E's ballot first choice = E
  - Round 1 should read `A=2, B=0, C=1, D=1, E=1` ✓ arithmetically — but the prose immediately following says "B has 0 first-place votes (unique lowest). Eliminate B. Redistribute B's ballot ... A is next surviving choice on that ballot, but A already has B's first-place vote — no movement". B's ballot's *first* choice is already A; redistribution applies to *eliminated candidates'* ballots, but A is not eliminated. The narrative confuses "B's ballot being redistributed because B is eliminated" with "B's ballot already having A as first choice". The transition `Round 1 → Round 2` shows `A: 2 → 2, C: 1 → 2, D: 1, E: 1` — i.e., C gained 1 vote from the elimination of B. But B's ballot ranks `A > B > C > D > E`; after eliminating B, the surviving prefix is `A > C > D > E`. The next surviving choice is **A**, not C. So Round 2 should read `A: 3, C: 1, D: 1, E: 1`, not `A: 2, C: 2`.
- **Why it succeeds**: The intent doc's canonical worked example for the happy path is **arithmetically wrong**. Downstream Lean / Quint specs derived from this scenario will encode wrong tabulation semantics. The error is on the load-bearing reference example.
- **Suggested defense**: Recompute the round table by hand. Either (i) change B's ballot first choice from A to B, or (ii) recompute Round 2 with B's redistribution flowing to A (yielding `A: 3`), which then forces a different Round-3 picture.

---

## Attack 2 — Section 6.2 row 3 fabricates a Quartz artifact that does not exist

- **Category**: Composition failure
- **Severity**: critical
- **Scenario**: Section 6.2 (line 370) claims:

  > `DstackKeyManager` derives per-contract keypairs deterministically from an attested seed, releases the private key only to the matching attested enclave image | `Specs.Quartz.Crypto.Ecies` (`PrivKey`, `PubKey`, `keyOf`)

  Direct check: `grep -rn "DstackKeyManager\|KMS\|KeyManager" /Users/mvid/Development/reliq/quartz/proofs/lean/Specs/` returns **zero hits**. Quartz's Lean spec tree contains no model of dstack KMS. `Specs.Quartz.Crypto.Ecies` exposes `PrivKey : Type`, `PubKey : Type`, `keyOf : PrivKey → PubKey` — a type-level placeholder for asymmetric key derivation. Nothing about "release[s] the private key only to the matching attested enclave image" is present in any Quartz Lean artifact.
- **Why it succeeds**: The intent doc claims to *inherit* a property that has never been stated, let alone proved, in the upstream. The verified-rcv compose ledger's first cross-project dependency edge points at a non-existent vertex. Section 5's last non-goal ("No formal verification of inherited Quartz primitives at this layer. verified-rcv asserts B8 ... as depending on Quartz's `tdxVerifier` and `groth16Verifier` bundles") implicitly excludes KMS — yet B9's `Adv_KMS_leakage` summand and 4.2's "Inherited from Quartz / dstack KMS spec" both depend on it.
- **Suggested defense**: Either (a) drop the row and concede that KMS honesty is an unverified out-of-band assumption with no Lean substrate, or (b) commit to writing the Quartz-layer `Specs.Quartz.Crypto.KMS` module before B9 can be stated honestly.

---

## Attack 3 — B9's 5-summand list does not match Quartz's 5-summand list it claims to inherit

- **Category**: Disjunction-vs-decomposition collapse
- **Severity**: critical
- **Scenario**: Section 3.2 B9 (line 243) gives:

  > `≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_KMS_leakage(n) + Adv_image_registration(n)`

  and says "Three (`tdxVerifier`, Groth16-KS, circuit-equivalence) are inherited from Quartz's `cross_component_session_bind_negl` 5-summand union bound."

  But Quartz's actual `cross_component_session_bind_negl` 5-summand bound (per `proofs/lean/Specs/Quartz/Protocol/ProtocolVCVioQuad.lean` and `.colosseum/ledger.md` lines 105–124) is:

  ```
  Pr[commitHashE coll] + Pr[commitHashBytesE coll] + Pr[tdxVerifier forgery] + negligible_groth16 + negligible_circuit
  ```

  Quartz's 5 summands = {commitHashE-CR, commitHashBytesE-CR, tdxVerifier-sound, groth16-KS, circuit-eq}.
  verified-rcv's 5 summands = {tdxVerifier-sound, groth16-KS, circuit-eq, KMS-leakage, image-registration}.

  Two Quartz summands (`commitHashE` and `commitHashBytesE` collision-resistance) are silently **dropped** from the verified-rcv bound. Two new summands (`KMS_leakage`, `image_registration`) are silently **added** without provenance in any Quartz Lean theorem. The "inherited from Quartz's 5-summand union bound" prose obscures that this is a *different* 5-tuple with the same cardinality.
- **Why it succeeds**: Section 6.2 inherits `commitHashE` (row 5: "Quartz axiom bucket (d) pigeonhole-impossible (lift restates as concrete-hash collision-resistance)"). B8 clause (c) uses `hash(contract_addr, tally)` — which is a commitment hash binding tally body bytes to the contract address, structurally identical to Quartz's `commitHashBytesE`. The `commitHashBytesE` collision-resistance summand is necessary for B8 clause (c) to hold under collision attack; dropping it from B9 either contradicts B8 (the bound omits a summand B8 needs) or treats clause (c) as classical-Prop (re-importing the spec-impossible `(d-pigeonhole)` axiom). Cardinality of 5 has been preserved cosmetically; honesty requires either 4 (Quartz inheritance only) or 6 (Quartz's 5 + image_registration) or 7 (Quartz's 5 + KMS + image), depending on what is in scope.
- **Suggested defense**: Recompute the summand list from B8's clauses. B8 clauses (a)+(b) cover `tdxVerifier` + `groth16-KS` + `circuit-eq` (3 inherited). Clause (c) requires `commitHash` collision-resistance (1 inherited, currently missing). Clause (d) requires `image_registration` (1 new, operational). KMS leakage is a *confidentiality* failure mode (4.2), not an attestation-soundness summand; it does not belong in B9 because B9 is about B8 being violated — losing confidentiality does not violate B8. Either drop `KMS_leakage` from B9 or rewrite B8 to bind confidentiality (currently it does not).

---

## Attack 4 — B9 inherits a substrate that the upstream ledger has retracted as content-free

- **Category**: Composition failure (refinement mismatch sub-variant)
- **Severity**: critical
- **Scenario**: B9 says "the verified-rcv compose ledger will record the dependency graph explicitly" and cites Quartz's `cross_component_session_bind_negl` as the inheritance source. The Quartz ledger (`/Users/mvid/Development/reliq/quartz/.colosseum/ledger.md`) opens with:

  > **CORRECTION 2026-05-14**: Round A adversarial review ... returned BREAKS on the content-phase lift. The 8 `_negl` theorems are content-free tautologies — each binds its protocol-fail advantage as a free `ℝ≥0∞` function symbol with no defining equation; proofs go through via `negligible_of_le` + `negligible_add` (closure properties of `negligible`) ... The lifts have the *shape* of a security reduction but not the *content*.

  i.e., the Quartz-side artifact verified-rcv's B9 claims to inherit was *retracted on the same day* (2026-05-14) verified-rcv's intent doc was written. The intent doc's references to "Quartz's `cross_component_session_bind` 5-summand union bound" cite a theorem whose interpretation upstream is now "scaffolding pending def-tying refactor", not audit-ready content.
- **Why it succeeds**: B9 cannot be honestly stronger than its substrate. The Quartz lift currently proves `negligible f → negligible f` (vacuous closure under domination + sum). Inheriting "5-summand bound" from this gives verified-rcv a 5-summand bound whose 5 summands are all unconstrained `ℝ≥0∞` symbols — every B9 instance is satisfied by choosing all five advantages to be `λ _, 0`. The verified-rcv compose ledger would emit the same vacuity Round A surfaced in Quartz, but at one more layer of remove.
- **Suggested defense**: Either (i) defer B9 until the Quartz content-phase refactor lands and the 5 summands are tied to concrete win events, or (ii) explicitly mark B9 as `[STATUS: scaffolding-inherited-from-Quartz-scaffolding]` in the intent doc, with the same caveat the Quartz ledger now carries.

---

## Attack 5 — B8 clause (c) does not identify the hash, silently consuming Quartz's pigeonhole-impossible axiom

- **Category**: Impossibility-hypothesis vacuity
- **Severity**: critical
- **Scenario**: B8 clause (c) (line 242): "attested `user_data = hash(contract_addr, tally)`". The function `hash` is not named, not typed, not specified. Quartz models this via `commitHashE : UserDataCommit ↪ UserData` — a `Function.Embedding`, i.e., an *axiomatically injective* hash into a fixed-width 64-byte codomain. Quartz's `UserDataCommit.lean` header reads (line 41):

  > By pigeonhole, no such embedding exists once `|UserDataCommit| > |UserData|`. The bundling **does not fix** this. It surfaces it: the single axiom now visibly carries both "there is a hash" and "the hash is collision-free", which exposes the second claim as the load-bearing trust assumption.

  Quartz tags this `(d-pigeonhole-impossible)`. The honest lift restates the embedding as "negligible collision probability under random-oracle model". The verified-rcv intent doc never makes this restatement — it writes `hash(contract_addr, tally)` in classical-Prop form, no negligibility qualifier, no `[Fintype]` constraint on `tally`. The TallyResult contains `Vec<HashMap<Addr, Nat>>` (per `per_round_counts`) with N up to ~30 candidates and rounds up to ~30 — codomain |TallyResult| is open. Hashing this into a 64-byte UserData slot is *mathematically* exactly the same pigeonhole impossibility as Quartz's `commitHashE`.
- **Why it succeeds**: Either (a) the intent doc silently consumes Quartz's spec-impossible axiom and inherits the vacuity (B8's clause (c) becomes a deterministic equality on an impossible embedding — the antecedent `hash(addr, T1) = hash(addr, T2) → T1 = T2` is universally true by pigeonhole-vacuity in the standard set theoretic model); or (b) the intent doc means a *new*, verified-rcv-specific hash that has no Quartz substrate. The doc gives no indication which. Without identifying `hash` as a named cryptographic primitive with an explicit `[Fintype TallyResult]` carrier refinement and a collision-advantage bound `Adv_commitTally_CR`, B8 clause (c) is vacuously satisfiable.
- **Suggested defense**: Name the hash explicitly (e.g., "SHA-256 of canonical-serialization(contract_addr || tally)"), state its codomain as 32 bytes (or 64), state collision-resistance as a hypothesis `Adv_commitTally_CR(n)`, and add this summand to B9. The 6.2 inheritance table row 5 (`commitHashE`) cannot cover this because it commits a different structured tuple type.

---

## Attack 6 — B6 is tagged `state` but every honest formalization makes it temporal

- **Category**: Temporal-state mismatch
- **Severity**: critical
- **Scenario**: B6 (line 240): "ballot writer is the ballot voter | **state** | at every state, ∀ k ∈ ballots, the SubmitBallot tx that wrote ballots[k] had msg.sender = k. Enforced by construction of Block 3's Requires."

  The clause "the SubmitBallot tx that wrote ballots[k] had msg.sender = k" quantifies over the *history*, not over the current state. A state-only predicate evaluating `ballots[k]` cannot witness which transaction wrote it — the storage at a given state σ contains only `Vec<u8>` blobs keyed by Addr; it does not contain "the tx that wrote this entry". The honest state-only statement is `S4` (`∀ k ∈ ballots.keys, k ∈ candidates`), which is what Block 3 actually enforces. B6 *says* something stronger: the writer's identity matches the key. To make this verifiable at every state, the state must record the writer; but `state.rs` does not (and the intent doc does not require it to). The property is temporal: `always (next.ballots ≠ ballots → tx.msg.sender ∈ next.ballots ∧ next.ballots[tx.msg.sender] = tx.encrypted_preferences)`.

  Section 3.2 header explicitly warns: "The **state**/**temporal** tag is load-bearing per Colosseum methodology: state invariants discharge at every state independently; temporal properties require explicit history quantification downstream. The wrong tag silently mis-encodes intent — the failure mode the `colosseum-spec-adversary` `temporal_state_mismatch` attack category is tuned to find."

  This is the failure the author warned the reviewer to find. The tag is wrong.
- **Why it succeeds**: At Lean lift time, a state-only invariant `∀ σ ∈ ReachableStates, ∀ k ∈ σ.ballots, σ.ballots[k].writer = k` requires augmenting `σ` with a `writer : Map<Addr, Addr>` field. The intent doc does not declare this in Section 2.5's state variables. Downstream Quint encoding would either (i) discharge B6 vacuously by re-stating `S4` (losing the "writer == key" content), or (ii) add a phantom `writer` field that has no chain-storage counterpart.
- **Suggested defense**: Re-tag B6 as `temporal` and rewrite the statement in next-state-relation form: `always (k ∉ ballots ∧ k ∈ next.ballots ∧ next.ballots ≠ ballots → ∃ tx, tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`.

---

## Attack 7 — S6, S7, S8, S9 tagged structural but only hold *post-write* and so are temporal

- **Category**: Temporal-state mismatch
- **Severity**: serious
- **Scenario**: Section 3.1 lists S6–S9 under "Structural invariants" with the qualifier "Properties evaluable on contract state at any reachable moment. No quantification over operations or time." Each is gated on `if tally_result.is_some()` (S6, S7) or applies per-row of `per_round_counts` (S8, S9). The gating is fine; the issue is that the *body* of each invariant requires the `tally_result` body to *satisfy* a well-formedness predicate.

  But the well-formedness predicate is enforced *only at the publish_result transition* (Block 6's Requires). If a state σ exists with `tally_result = None`, S6–S9 hold vacuously. If σ has `tally_result = Some(T)` and the Requires were checked at the writing transition, S6–S9 hold. The state-only formulation thus reduces to "Block 6 correctly checks well-formedness before writing" — which is a *temporal* property of the write transition, not an evaluable predicate over state.

  In a Lean state-machine model, evaluating S6 at an arbitrary state σ amounts to running the well-formedness checker — a `state` invariant, technically. But under refinement to a contract-execution semantics, the only path that establishes S6 is the publish_result transition; an external observer cannot distinguish "the chain enforces this at write" from "this happens to be true" without running the well-formedness predicate. The author intended the *enforcement*, not the *evaluable*.
- **Why it succeeds**: A spec author who reads "S6: state" will write a Lean `state_invariant` predicate that checks well-formedness pointwise on every state — and the Quint/Lean discharge will succeed for every state in which `tally_result = Some(T)` only because the transition created it that way. The state-only formulation is mechanically right but encodes the wrong claim — the *temporal* claim is "every write to `tally_result` passes well-formedness", which is the load-bearing thing the spec author wants to prove.
- **Suggested defense**: Either (i) keep the `state` tag but rename to "well-formedness-pointwise-evaluable (refined from temporal write-discipline)" and reference the temporal complement; or (ii) demote S6–S9 to derived corollaries of a temporal B-class invariant `B10: always (next.tally_result.is_some() ∧ tally_result.is_none() → well_formed(next.tally_result))`.

---

## Attack 8 — terminal-tie scenario in 2.2 contradicts S6's "len(winners) ≥ 1" semantics under IRV

- **Category**: Contradiction (edge case interaction)
- **Severity**: critical
- **Scenario**: Section 2.2 (line 56): "Two candidates, both vote, terminal 1-1 tie — first round is the only round; no elimination needed. Result: `winners = [both candidates]`, `per_round_counts = [{A: 1, B: 1}]`, `eliminated_by_round = []`."

  IRV terminates when (a) some candidate has >50% of remaining ballots, or (b) only one candidate remains, or (c) all remaining candidates tie at the same vote count and batch-eliminating all of them would empty the set. Case (c) is the all-abstain scenario; it produces co-winners by the doc's convention. The "terminal 1-1 tie" *with* both candidates having voted is a different shape — neither has a majority (each has 50%), there are two candidates remaining, no batch elimination has triggered yet because neither is "lowest". The Section 2.5 Block 6 well-formedness predicate requires `per-round counts internally consistent (sum to ballots_tallied for each round)` and `elimination sequence monotone`. Nothing in Section 2.5 says "if no further elimination is possible, declare co-winners". The "terminal tie → both co-winners" rule is introduced ad-hoc in the 2.2 example without specification.

  Worse, Section 2.1's Round 3 ("C holds 3 of 5 ballots = 60% > 50%. **C wins**") uses ">50%" as the termination condition. Under that condition, "1 of 2 = 50% < 50%" — neither candidate wins. The intent doc has *two different* termination conditions: ">50% wins" in 2.1, and "fall-through to co-winners on terminal tie" in 2.2. Section 2.5 Block 6 specifies neither.
- **Why it succeeds**: An implementer reading the intent will face a choice point with no answer: either (i) ">50% only" (so 1-1 in 2-candidate election produces an error — `InvalidTally` because the loop did not converge), or (ii) "≥50% wins" (which makes 2.1 Round-3 still work — C has 60% > 50% — but also fires earlier rounds prematurely). The intent doc's example outputs assume "the algorithm has a `terminate when only 2 remain and tied → co-winners` rule" that is nowhere stated.
- **Suggested defense**: Either (i) define IRV termination explicitly (e.g., "terminate when ≥50% achieved or |remaining| = |eliminated_this_round| + |remaining_this_round| would not progress; on no-progress, declare co-winners"), or (ii) accept "1-1 terminal tie" as `InvalidTally` and update 2.2 to reflect that.

---

## Attack 9 — single-candidate boundary case violates B6 if the candidate abstains

- **Category**: Edge case
- **Severity**: serious
- **Scenario**: Section 2.2 (line 54): "Single candidate (`len(candidates) = 1`) — trivial winner; tabulation short-circuits. Result: `winners = [that_candidate]`, `per_round_counts = [{that_candidate: ballots_tallied}]`, `eliminated_by_round = []`. `non_voters` lists the sole candidate iff they did not vote."

  If the sole candidate abstains, `ballots_tallied = 0`, `non_voters = [that_candidate]`, `winners = [that_candidate]`, `per_round_counts = [{that_candidate: 0}]`. S6 holds (`winners ⊆ candidates ∧ len(winners) ≥ 1`). But S7 (`ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)`) gives `0 + 0 + 1 = 1 = len(candidates)` ✓. S8 (`per-round counts sum to ballots_tallied`) gives `0 = 0` ✓. But B6 says "ballot writer is the ballot voter" — the predicate quantifies over `k ∈ ballots`, which is empty, so it holds vacuously. The "winner with zero votes" outcome is mathematically defined but semantically: the system has declared a winner who not only didn't get a majority, but didn't get a single vote. The intent doc's Section 5 says "No threshold or quorum requirement"; the per-round count `{that_candidate: 0}` is mathematically valid but the result is "X wins with 0 votes" — surprising. More importantly, S6 + S7 + S8 do not rule this out, and the intent doc never says they should.
- **Why it succeeds**: A downstream invariant of the form "if `winners ≠ ∅`, then `∃ c ∈ winners, per_round_counts[final][c] > 0`" would catch this. No such invariant is stated.
- **Suggested defense**: Add an invariant or scenario specifying behavior when the unique winner has 0 first-place votes. Either accept as documented (the abstain-as-winner case), or add S11: `if ballots_tallied = 0 ∧ winners ≠ ∅ → winners = candidates` (forces co-winning rather than "first candidate by ordering").

---

## Attack 10 — batch-elimination tie-break under-specified at the candidate-set boundary

- **Category**: Under-specification
- **Severity**: serious
- **Scenario**: Section 2.1 Round 2: "D and E tied for lowest at 1 each. **Batch-eliminate D and E.**" Section 2.2 abstention scenario: "B, D, E tied for lowest at 0. Batch-eliminate B, D, E."

  The intent doc never specifies *when* batch elimination is safe vs unsafe. A canonical IRV rule is: "batch-eliminate the set S of lowest-tied candidates if and only if their *combined* count is still less than the next-lowest candidate's count" (otherwise eliminating them all could prematurely eliminate a winner). In Round 2 of 2.1: D = 1, E = 1, next-lowest survivor = A at 2 (or C at 2). Sum(D, E) = 2 = next-lowest = 2 — the "strictly less than" version says do NOT batch-eliminate. The "less-or-equal" version says batch-eliminate. The intent doc gives no rule. Different IRV implementations choose differently; the standard ACE-rules implementation forbids batch elimination when sum(eliminated) ≥ next-survivor count.

  Section 2.2 abstention: B=0, D=0, E=0, next-survivor = A at 1 (or C at 1). Sum(B, D, E) = 0 < 1 — batch-elim is safe under any reasonable rule.
- **Why it succeeds**: The 2.1 happy path's Round-2 batch elimination uses an unstated tie-break rule that the implementer must invent. A Lean spec generated from this intent will either guess wrong (and prove the wrong tally) or expose the gap (and refuse to encode the algorithm).
- **Suggested defense**: State the batch-elimination rule explicitly. Recommended (standard): "batch-eliminate the lowest-tied set S iff |S| candidates can be eliminated without removing all candidates AND sum(votes in S) < min(votes in candidates \ S)". Then re-derive Section 2.1's table under this rule; the current example may not satisfy it.

---

## Attack 11 — `InvalidTally` creates a second deadlock path missing from Section 8.3 and 4.6

- **Category**: Coverage gap
- **Severity**: critical
- **Scenario**: Section 2.4 (line 93) lists `InvalidTally { reason }` as recoverable "Yes — enclave bug; re-tally needed". Section 4.6 (line 311) says `publish_result` "is idempotent by construction (B1 plus Block 6's `AlreadyResolved` rejection), so retries are safe". But idempotency only applies to **successful** publishes. If the enclave produces a malformed tally (e.g., a bug in IRV tabulation produces `winners = []` violating S6, or counts that don't conserve violating S7), `publish_result` rejects with `InvalidTally`. The enclave must re-tally. But the enclave is *deterministic over the ballot set* — if the ballot set is frozen at `end_at` (per B2) and the tally function is deterministic, the enclave will produce the *same* malformed tally on retry. The contract will reject again. Infinite rejection loop.

  Section 4.6 (enclave-network-partition) and 8.3 (enclave-never-publishes) cover the case where `publish_result` never *succeeds*. But neither covers the case where `publish_result` *successfully reverts with InvalidTally on every retry*. The election deadlocks in Tallying just as definitively as 4.6/8.3, but via a different mechanism — and the intent doc's failure-mode taxonomy is silent on it.
- **Why it succeeds**: The intent doc's reasoning ("Yes — enclave bug; re-tally needed") implicitly assumes the enclave can produce a *different* tally on retry. For a deterministic enclave with frozen ballots, this is false. The recovery column is wrong: `InvalidTally` is **non-recoverable** without an enclave software update — which requires re-attestation of a new image, which is out of scope (4.4 covers only the unverified-enclave case; the verified-enclave-with-bug case is the methodology target but the recovery action is missing).
- **Suggested defense**: Add Section 4.9 — "Deterministic-malformed-tally deadlock — *liveness*". Detection: `publish_result` returns `InvalidTally` repeatedly. Recovery: same as 4.4 (deploy fresh contract with new enclave image). Update Section 2.4's `InvalidTally` "Recoverable" column to "Conditional — requires enclave image update, not in-instance".

---

## Attack 12 — `start_at == env.block.time` admissibility contradicts Block 1 Requires

- **Category**: Edge case
- **Severity**: serious
- **Scenario**: Section 2.3 (line 76): "Instantiation with `start_at` close to `env.block.time` — admissible as long as `start_at > env.block.time`; the contract enters Voting state on the very next block after `start_at`."

  Block 1 Requires (line 121): "`start_at > env.block.time`" — strict. So `start_at = env.block.time` rejects with `InvalidInstantiation`. Fine.

  But: the derived state "Created" is `env.block.time < start_at` (line 107), and "Voting" is `start_at ≤ env.block.time < end_at` (line 108). At the boundary `env.block.time = start_at`, derived state is *immediately* Voting. The "very next block" is incorrect — Voting begins at the *same* block as `env.block.time = start_at`, not the next block. The off-by-one is mid-doc.

  Worse: Block 1 enforces `start_at > env.block.time` at instantiation. If instantiation happens at block height B with `env.block.time = T_B`, and `start_at = T_B + δ` for some `δ > 0`, the contract is in Created at block B (since `T_B < start_at`). At block B+1 with `env.block.time = T_{B+1}`, if `T_{B+1} ≥ start_at`, the contract is in Voting. Whether voting can occur at block B+1 depends on whether `T_{B+1} - T_B ≥ δ`. The intent doc says "very next block" — but Cosmos block times can advance by any positive amount (typically 6s on Xion), so if `δ < block_time_increment`, the boundary is crossed mid-block; if `δ > block_time_increment`, it's crossed multiple blocks later.
- **Why it succeeds**: The "very next block" prose is wrong both ways: (a) Voting begins at `start_at == env.block.time`, not after; (b) the block at which this happens depends on `δ` vs block-time-increment, not "the very next block".
- **Suggested defense**: Replace "very next block after `start_at`" with "the first block whose `env.block.time ≥ start_at`".

---

## Attack 13 — `end_at == env.block.time` derived state is ambiguous between Voting and Tallying

- **Category**: Ambiguity (edge case)
- **Severity**: serious
- **Scenario**: Section 2.5 derived states:
  - Voting: `start_at ≤ env.block.time < end_at`
  - Tallying: `env.block.time ≥ end_at ∧ tally_result.is_none()`

  These are not in conflict — at `env.block.time = end_at`, Voting is false (strict <) and Tallying is true (≥). OK.

  But Block 5 Forbids (line 172): "`env.block.time < end_at`". And Section 2.3 edge case (line 66): "**Late ballot during Tallying** — `submit_ballot` after `end_at` is rejected with `NotInVotingWindow`. Ballot store is frozen at the `end_at` boundary."

  At the exact block where `env.block.time = end_at`: a `submit_ballot` transaction in that same block — does it succeed? Block 3 Requires "derived state is Voting" → false (since Voting is `< end_at` strict). So `submit_ballot` rejects with `NotInVotingWindow`. B2 (line 236): "`always (env.block.time ≥ end_at → ballots = ballots@end_at)`" — at `env.block.time = end_at`, `ballots` must equal `ballots@end_at`, which is *itself at this block*. So `ballots` at this block must equal `ballots@end_at`. This is tautological at `env.block.time = end_at`, but in the same block: can a `submit_ballot` transaction at this block *modify* `ballots`? Block 3 rejects, so no. Consistent.

  But: `ballots@end_at` is not defined in the intent doc. Is it the value of `ballots` at the first block where `env.block.time ≥ end_at`? Or at the last block where `env.block.time < end_at`? The two can differ if a `submit_ballot` succeeds at the boundary.
- **Why it succeeds**: `ballots@end_at` is a fictitious value; B2 requires a precise definition. In an event-sourced model, the relevant value is "the value of `ballots` immediately before any block where `env.block.time ≥ end_at`". The intent doc never defines this.
- **Suggested defense**: Define `ballots@end_at` as "the value of `ballots` at the first state σ in the execution trace where `σ.env.block.time ≥ end_at`". State explicitly that no `submit_ballot` can succeed at or after this state.

---

## Attack 14 — B2's `ballots@end_at` notation is undefined

- **Category**: Ambiguity
- **Severity**: serious
- **Scenario**: B2 (line 236): "`always (env.block.time ≥ end_at → ballots = ballots@end_at)` — once voting closes, the ballot store is frozen for all time". The `@end_at` subscript is a time-indexed reference. Quint/Lean state-machine spec languages do not have a built-in "value at time T" operator. This needs either (a) a ghost variable `ballots_at_end_at: Option<Map<Addr, Vec<u8>>>` populated at the transition to Tallying, or (b) a temporal "the value when block-time first hit end_at".
- **Why it succeeds**: Section 3.2 marks B2 `temporal` — correct tag. But the spec writer downstream will need to encode this in Quint with an explicit `last_voting_state` ghost. The intent doc says nothing about this ghost.
- **Suggested defense**: Either reformulate B2 as `always (next.env.block.time ≥ end_at ∧ env.block.time < end_at → ballots_at_end_at = ballots) ∧ always (env.block.time ≥ end_at → ballots = ballots_at_end_at)` with an explicit ghost, or rewrite as a structural property "after end_at, the only transitions that change state are publish_result". B4 (line 238) already does this — B2 is partially redundant with B4.

---

## Attack 15 — Section 2.4 `AttestationFailed` "Recoverable: Yes — enclave may re-submit" contradicts Section 4.3a/4.3b's "Recovery: None"

- **Category**: Contradiction
- **Severity**: serious
- **Scenario**: Section 2.4 (line 92): "`AttestationFailed { reason }` | TDX quote verification failure, zkdcap proof mismatch, wrong enclave identity | Yes — enclave may re-submit; persistent failures require off-chain debugging"

  Section 4.3a (line 277): "`publish_result` fails permanently with `AttestationFailed { reason: VkeyUnregistered }`. Election deadlocks; no canonical tally ever lands on-chain. Recovery: None for the affected election; deadlock is terminal."

  Section 4.3b (line 286): "Recovery: None for the affected election; bad tallies are terminal once `publish_result` succeeds."

  The taxonomies cross-classify: 2.4 says `AttestationFailed` is recoverable; 4.3a/4.3b says specific sub-causes are unrecoverable. The intent doc has two failure taxonomies that disagree.
- **Why it succeeds**: A downstream consumer of Section 2.4's table will treat `AttestationFailed` uniformly as recoverable. A downstream reader of Section 4 will treat 4.3a's cause as unrecoverable. Both are in the same intent doc.
- **Suggested defense**: Reconcile: 2.4 should say "Conditional — recoverable if cause is enclave-side (re-attest); non-recoverable if cause is chain-registry-side (4.3a) or chain-governance-side (4.3b)".

---

## Attack 16 — B8 inherits `tdxVerifier.sound` classical-Prop form without preserving the preconditional gap

- **Category**: Preconditional over-strength
- **Severity**: serious
- **Scenario**: B8 clause (a): "TDX quote validates". This consumes Quartz's `tdxVerifier.sound` projection, which Quartz's `Dstack.lean` (line 232) gives as:

  ```
  theorem verifyTdxQuote_sound (q : TdxQuote) (mr : MrEnclave) (ud : UserData) :
      verifyTdxQuote q = some (mr, ud) → was_signed_by_dstack q
  ```

  and Quartz's `Dstack.lean` honesty caveat (line 217–230) explicitly flags this as classical-Prop dropping the "negligible probability of forgery" qualifier. The truthful form requires modeling against `OracleSpec`.

  The intent doc inherits clause (a) without restating the preconditional or negligibility qualifier. B9 *partially* compensates by including `Adv_tdxVerifier_sound(n)` in the budget — but B8 itself reads as a classical-Prop entailment, which is what Quartz Round-A retracted as content-free.
- **Why it succeeds**: B8 is a "temporal" invariant whose body is a deterministic implication. A spec-layer reader sees "attestation verifies → tally is correctly bound" with no probability budget. The probability budget lives in B9. But B8 is the load-bearing trust claim — the document's Section 6.6 "Output contract" says "`tally_result.is_some()` ⇒ a valid `DstackAttestation` was supplied at `publish_result` time (B8)". This consumer-facing claim is the over-strong classical form.
- **Suggested defense**: Either (i) annotate B8 as `[classical-Prop shadow, content via B9]` so consumers know to read B9 for the actual trust claim, or (ii) restate B8 with explicit "(with probability ≥ 1 − Adv_total(n))" qualifier.

---

## Attack 17 — `Adv_KMS_leakage` is a confidentiality summand, not an attestation-binds-tally summand; it does not belong in B9

- **Category**: Disjunction-vs-decomposition
- **Severity**: serious
- **Scenario**: B9 includes `Adv_KMS_leakage(n)`. The cited failure mode is 4.2: "KMS bug or operator compromise releases the per-election keypair to a non-attested entity. Effect: Adversary decrypts ballots. **Integrity remains intact (the leaked key alone cannot produce valid attestations).**"

  Quote: "Integrity remains intact". B8 is about integrity (attestation binds tally). KMS leakage **does not violate B8**. It violates **confidentiality**, which is not the property B9 bounds.

  Adding `Adv_KMS_leakage` to B9's union bound double-counts confidentiality risk as integrity risk — it inflates the bound with an irrelevant summand. The summand is misclassified.
- **Why it succeeds**: B9's union bound reads as "probability that B8 fails", and the doc cites the 5-summand structure as inherited from Quartz. Quartz's 5 summands all relate to attestation forgery / hash collision / circuit-equivalence — *all* attack the integrity claim. KMS leakage attacks confidentiality only.
- **Suggested defense**: Remove `Adv_KMS_leakage` from B9. State KMS-leakage's failure mode as a *separate* property (e.g., B10: "confidentiality of pre-tally ballots reduces to KMS honesty + ECIES-IND-CPA + TDX confidentiality").

---

## Attack 18 — `Adv_image_registration` is not a cryptographic advantage and stating it as one is over-strength

- **Category**: Disjunction-vs-decomposition
- **Severity**: critical
- **Scenario**: B9 includes `Adv_image_registration(n)`. The accompanying prose: "(`image_registration`) is verified-rcv-specific and is a *deployment-operational* claim, not a cryptographic reduction: it asserts that off-chain processes for vkey publication and image-hash distribution reliably correspond to the on-chain registered vkey."

  The doc explicitly acknowledges this is **not a cryptographic advantage**. But B9's left-hand side is `Pr[B8 violated at the next state by a polynomial-time adversary] ≤ ...`. The right-hand side is a sum of advantages parameterized by security parameter `n`. An operational-deployment claim is not a function of `n`; it is a binary "the off-chain process is honest" or not. Including it as `Adv_image_registration(n)` is type-erasure: mixing cryptographic negligible-in-n quantities with operational fault probabilities.

  Quartz's 5-summand bound contains only cryptographic advantages. The intent doc tries to fold an operational risk into the same arithmetic. Either (i) the union bound becomes meaningless (an operational adversary either succeeds with probability 1 or 0, not negligibly), or (ii) `Adv_image_registration(n)` is a constant-in-n quantity bounded above by some operational fault rate, which makes the whole bound non-negligible.
- **Why it succeeds**: The disjunction-vs-decomposition discipline requires the bound's summands be *homogeneous in type*. Mixing cryptographic and operational risk in one union bound smuggles past the type discipline.
- **Suggested defense**: Pull `Adv_image_registration` out of B9. State it as a separate trust assumption in Section 6.1 (chain trust) or 6.4 (caller contract): "the verified-rcv vkey registered on chain matches the externally-distributed image-hash". Bound on operational fault probability is *not* a function of security parameter.

---

## Attack 19 — `Adv_tdxVerifier_sound` and `Adv_KMS_leakage` are not disjoint events

- **Category**: Disjunction-vs-decomposition
- **Severity**: serious
- **Scenario**: B9 lists `Adv_tdxVerifier_sound + Adv_KMS_leakage` as separate summands (a sum is a union bound on disjoint events, or more correctly, an upper bound via Boole's inequality). But 4.1 ("TDX vulnerability ... If the vulnerability also permits forging attestations, integrity is also lost") and 4.2 ("KMS bug or operator compromise releases the per-election keypair") are not disjoint:
  - A TDX hardware vulnerability that extracts the private key (4.1's integrity-loss sub-case) and a KMS bug that releases the same key (4.2) cover overlapping attacker capability. An adversary with the per-election private key, regardless of *how* they obtained it, can:
    (i) decrypt ballots (confidentiality loss — both 4.1 and 4.2),
    (ii) produce signed attestations *if* TDX is also compromised in 4.1's broader sense (integrity loss — 4.1 only).
  - The relevant integrity events are: (a) forge a Groth16 proof; (b) forge a TDX quote (i.e., adversary controls TDX signing key); (c) the legitimate enclave produces a wrong tally (4.4 — out of scope of B9 because B8 only requires attestation-validity, not tally-correctness — see Attack 20).
- **Why it succeeds**: If B9 already covers attacker-controls-private-key via `Adv_tdxVerifier_sound` (which models forge-the-quote), the `Adv_KMS_leakage` summand is at best redundant (same attacker capability) or at worst double-counted (same event under two names).
- **Suggested defense**: Remove `Adv_KMS_leakage` per Attack 17, or explicitly state how `tdxVerifier_sound` and `KMS_leakage` carve disjoint slices of attacker-event-space.

---

## Attack 20 — B8 does not bind the tally to a correct-execution claim, only to an attestation-validates claim

- **Category**: Coverage gap (under-specification)
- **Severity**: critical
- **Scenario**: B8 says the published tally is bound to attestation validation. It does NOT say the tally is bound to the *correct* IRV computation over the ballots. B8 clauses:
  - (a) TDX quote validates
  - (b) zkdcap proof verifies
  - (c) user_data = hash(contract_addr, tally)
  - (d) MRTD/RTMR matches registered image

  All four can hold while the tally is *wrong* (the attested enclave image has a bug — failure mode 4.4). Section 4.4 says this is "in scope, central methodology target": "Pre-deployment formal verification against the downstream Lean and Quint specs. No post-deployment mitigation; spec correctness is load-bearing."

  But B8 does not assert "the tally is what running IRV on the ballots would produce". That assertion would be a separate invariant — call it `B10: correct-tabulation`. The intent doc does not include such an invariant in Section 3.2. Section 6.6 ("Output contract") says consumers can rely on "The tally body satisfies the well-formedness invariants S6 + S7 + S8 + S9" — but S6–S9 are syntactic well-formedness, not semantic correctness. A bug-introduced wrong tally that happens to satisfy syntactic well-formedness passes all stated invariants.
- **Why it succeeds**: Section 4.4 says "spec correctness is load-bearing" — i.e., the intent doc relies on the *spec* (the Lean / Quint encoding of IRV) being correct. But the *invariant set* (B1–B9, S1–S10) does not include "the published tally equals the spec's IRV applied to ballots". Without this invariant, the methodology target (4.4) has no formal anchor.
- **Suggested defense**: Add `B10: tally-correctness` (temporal): `always (next.tally_result = Some(T) ∧ tally_result = None → T = IRV_spec(decrypt_all(ballots, enclave_privkey)))` — i.e., the published tally is the IRV-spec applied to the decrypted ballots under the enclave's private key. This is the load-bearing methodology-target invariant.

---

## Attack 21 — "instantiation never completes; redeploy with corrected parameters" is the *only* recovery; instantiator has no on-chain feedback

- **Category**: Coverage gap (under-specification)
- **Severity**: serious
- **Scenario**: Section 2.4 row 1 (`InvalidInstantiation`): "No — instantiation never completes; redeploy with corrected parameters". Section 6.4: "Cannot recover from misconfiguration — instantiation parameters are immutable."

  Implicit: the instantiator is responsible for getting the parameters right. But what about the case where instantiation *succeeds* with wrong parameters? E.g., `end_at = start_at + 1 second` — passes Block 1 Requires, but produces a 1-second voting window in which no candidate can plausibly submit. The contract enters Voting, then immediately Tallying. The deadlock if no ballots are submitted is "all candidates abstain" (2.2). The intent doc has no notion of "minimum-viable voting window".
- **Why it succeeds**: Section 2.4's `InvalidInstantiation` catches *some* parameter errors (empty candidates, past start_at, end_at ≤ start_at). It does not catch operational misuse (1-second window, 100-year window, 1-candidate election with no plausible voters). Section 5 explicitly allows these ("No threshold or quorum requirement"). So they are intentionally permitted. But the intent doc should *say* so explicitly: "all instantiations satisfying Block 1 Requires are admitted; operational meaningfulness of the resulting election is off-chain".
- **Suggested defense**: Add to Section 6.4: "Block 1 Requires are the *only* on-chain admissibility checks; the instantiator alone is responsible for choosing operationally-meaningful values (window length, candidate set, etc.). The chain provides no minimum-window or quorum check."

---

## Attack 22 — "`encrypted_preferences` non-empty" is a degenerate well-formedness check that does not validate ECIES shape

- **Category**: Under-specification
- **Severity**: serious
- **Scenario**: Block 3 Requires (line 144): "`encrypted_preferences` non-empty". Section 6.5 says "the contract does not verify encryption shape on-chain; malformed ciphertext is dropped by the enclave at tally time". An adversary submits `encrypted_preferences = [0x00]` — a single null byte, non-empty, definitely not a valid ECIES ciphertext. The contract accepts and stores it. The enclave drops it at tally time as malformed.

  This is intentional and documented. But: this means `submit_ballot` is essentially uncostly per-tx (storage of a 1-byte blob); a candidate can spam submissions overwriting their own ballot many times during the voting window. Section 4.7's "last-write-wins" handles this for legitimate users; for an adversary who has compromised the candidate's key, they can spam to ensure their fake ballot wins the last-write race against the legitimate user.

  More structurally: `EmptyBallot` exists (line 88) as `ContractError` rejecting empty `encrypted_preferences`. Why have an `EmptyBallot` error at all if the enclave will drop any malformed ballot at tally time? The reason given is "Resubmit with valid encryption". But the enclave already does that. `EmptyBallot` is over-specification: a contract-level rejection of one specific failure shape (empty bytes) when all other malformed shapes are handled at tally time. Inconsistent contract-level vs enclave-level validation.
- **Why it succeeds**: An adversary can craft a near-empty malformed ballot (1 byte, 2 bytes, etc.) that satisfies Block 3 but fails enclave decryption. The on-chain `EmptyBallot` check provides false comfort: it filters one specific bad shape but does not validate anything cryptographic.
- **Suggested defense**: Either (a) remove the `EmptyBallot` check entirely (the enclave handles all malformed shapes uniformly), or (b) add minimum-length and format-shape checks consistent with ECIES ciphertext structure.

---

## Attack 23 — Section 2.5 Block 5 ("close_and_tally") provides no liveness obligation; Section 8.3 documents the consequence but no spec captures it

- **Category**: Coverage gap
- **Severity**: serious
- **Scenario**: Block 5 (line 168): "Trigger: `ExecuteMsg::CloseAndTally {}` (any chain address may call) ... emits a Cosmos SDK event for off-chain monitoring (UI / enclave-watcher); no storage change. ... Idempotent by construction." Block 5 has no Requires beyond `derived state is Tallying`; no Forbids beyond voting still open / already resolved.

  Block 5 is *purely* a signaling primitive. It produces no state change. Section 8.3's deadlock scenario relies on Block 5 being callable but having no liveness obligation. No invariant captures "if Block 5 fires repeatedly without Block 6 firing, eventually `tally_result.is_some()`" — because no such property is intended (it would require an admin or a deadline).
- **Why it succeeds**: This is *intentional* — Section 5 non-goal: "No admin / creator recovery role." But the doc never explicitly states "no liveness invariant binds publish_result". A reader of Section 3.2 may search for a "must-eventually-resolve" temporal property and find none, but they have to do so by absence.
- **Suggested defense**: Add a non-invariant statement (Section 3.3 or in Non-Goals): "No liveness invariant binds publish_result; the system makes no claim that an election will resolve. Deadlock in Tallying is permitted by construction (Section 5 / Failure 4.6 / Scenario 8.3)."

---

## Attack 24 — `1,000,000 gas` bound is asserted, not derived; treating excess as "spec violation" is over-specification

- **Category**: Over-specification
- **Severity**: serious
- **Scenario**: Section 7 (line 420): "Every on-chain handler call MUST consume ≤ 1,000,000 gas units. ... A handler call that would exceed 1M gas is treated as a **spec violation**, not just a failed transaction — it indicates the configuration (or input size) is outside the bounds the system is designed for."

  This is asserted, not derived. The bound is "Conservative relative to Cosmos SDK block gas limits ... Round number that downstream Kani harnesses can target directly as a static-analysis bound." The intent doc treats this as a *correctness property*. But a transaction exceeding gas is *not* a correctness failure — it's a transaction that doesn't execute. The state machine is unaffected. Treating "gas overflow" as on par with "wrong winner" or "frozen ballots changed" is category-confusion.

  Worse: "N ≤ 20–30 candidates" is described as "informal" and "downstream verification (Kani / Verus harnesses on the contract) should produce a tight bound." So the intent doc explicitly admits the bound is unverified and yet brands its violation a "spec violation".
- **Why it succeeds**: Downstream Kani harnesses targeting "≤ 1M gas" will either (a) succeed for all N up to some specific value (say N=18) and then start failing — implementer sees this as a correctness failure rather than an N-dependent performance failure; or (b) treat 1M as a constant and find counterexamples for any N where the per-byte storage cost crosses the threshold. Either way, the 1M bound is a *performance* claim, not a *correctness* claim, and elevating it to a correctness invariant misuses the verification budget.
- **Suggested defense**: Demote Section 7 from "correctness-relevant performance bound" to "performance non-functional requirement". State `Pr[handler exceeds 1M gas for N ≤ 20]` separately from B1–B10 and S1–S10. Do not treat gas overflow as a spec violation.

---

## Attack 25 — Section 4.7 stolen-key recovery semantics contradict Section 6.5

- **Category**: Contradiction
- **Severity**: cosmetic
- **Scenario**: Section 4.7: "Recovery: During voting: victim regains control and resubmits (last-write-wins). After `end_at`: none." Section 6.5: "Is responsible for their own chain private key security (failure mode 4.7)."

  These agree at the contract layer (last-write-wins). But: Section 8.6 step 5 (the impersonation scenario "stolen-key impersonation") elaborates: "legitimate A may resubmit if they regain control before `end_at`". This is repeated in 4.7. Yet Section 5 non-goals: "No coercion resistance. A voter can be compelled to reveal their preferences post-tally (or coerced into voting a specific way that they later prove by revealing their private key)." This is *different* from key-theft. Coercion-resistance is about post-tally proof; key-theft is about pre-tally action. The intent doc conflates them by saying both are out-of-scope.
- **Why it succeeds**: Mild contradiction in failure-classification taxonomy. The non-goal "no coercion resistance" is broader than 4.7's "no key-management"; failing to distinguish them obscures the threat model.
- **Suggested defense**: Separate the two non-goals: "No coercion resistance (post-tally)", "No key-management at the contract layer (pre-tally key theft handled per 4.7)".

---

## Attack 26 — B1 and S5 are partially redundant; the "state-shape view"/"temporal causal version" framing leaks methodology jargon into spec content

- **Category**: Ambiguity (cosmetic)
- **Severity**: cosmetic
- **Scenario**: S5 (line 222): "terminality of resolution (state-shape) | if `tally_result.is_some()` at state σ, then any successor state σ' has `tally_result = σ.tally_result` (state-shape view; the temporal causal version is B1)". B1 (line 235): "tally_result monotone-once-set | temporal | `always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))`".

  Both encode the same property. The framing "state-shape view; the temporal causal version is B1" is methodology jargon, not spec content. The intent doc seems to be hedging: enclosing the same property twice under two tags so a reader can pick which discharge mechanism they prefer. This is fine for elicitation but the downstream Quint/Lean compose stage will have to choose one or the other.
- **Why it succeeds**: Spec-layer redundancy is acceptable in intent docs but should be flagged so the downstream lifter knows to deduplicate.
- **Suggested defense**: Mark S5 as derived: "S5 (derived from B1, state-shape projection)". Same for S10/B3.

---

## Attack 27 — Section 2.4's `NotInTallyingState` recovery "Conditional" is too vague

- **Category**: Ambiguity
- **Severity**: cosmetic
- **Scenario**: Section 2.4 row 7: "`NotInTallyingState` | `publish_result` called when derived state is not Tallying | Conditional". Conditional on what? Section 8.5 illustrates one case (already-resolved); 8.4 illustrates another (premature). The recoverability differs: pre-Tallying is recoverable (wait), post-Tallying (Resolved) is not recoverable (terminal).
- **Why it succeeds**: A consumer of the table cannot tell when retry is fruitful.
- **Suggested defense**: Replace "Conditional" with "Yes if state was Created/Voting (wait); No if state was Resolved (terminal — see `AlreadyResolved`)".

---

## Attack 28 — `len(non_voters) = candidates \ ballots.keys` is implied but not stated

- **Category**: Under-specification
- **Severity**: cosmetic
- **Scenario**: Section 2.1 result shows `non_voters = []` because all candidates voted. Section 2.2 abstention shows `non_voters = [B, D, E]`. The relationship `non_voters = candidates \ ballots.keys` is implied but never stated in Section 2.5 or 3.1.

  S7 (count conservation) requires `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)`. With `non_voters = candidates \ ballots.keys`, `len(non_voters) = len(candidates) - len(ballots.keys)`. So `ballots_tallied + ballots_dropped + len(candidates) - len(ballots.keys) = len(candidates)`, i.e., `ballots_tallied + ballots_dropped = len(ballots.keys)`. This is the actual conservation property the well-formedness check should verify.

  Without stating `non_voters = candidates \ ballots.keys`, S7 is under-determined: a malicious enclave could publish `non_voters` containing addresses *not* in `candidates`, and S7 would still hold arithmetically.
- **Why it succeeds**: Block 6 Requires lists conservation `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)` but does not require `non_voters ⊆ candidates`. A malicious tally with `non_voters = [random_addr_1, random_addr_2, random_addr_3]` (where these are not candidates) and `ballots_tallied + ballots_dropped = len(candidates) - 3` passes the arithmetic but is semantically wrong.
- **Suggested defense**: Add to Block 6 well-formedness: `non_voters ⊆ candidates` and `non_voters = candidates \ (ballots_tallied_voters ∪ ballots_dropped_voters)`.

---

## Attack 29 — "any chain address may call" `close_and_tally` is over-permissive; griefing vector unaddressed

- **Category**: Coverage gap
- **Severity**: serious
- **Scenario**: Block 5 (line 168): "any chain address may call". Idempotent emit-event-only. An adversary can spam `close_and_tally` calls at the rate the chain allows. Each call costs gas (paid by the caller) but emits an event. Off-chain enclave-watcher services subscribing to these events get DoS'd by event flood.

  Section 7's 1M-gas-per-call is per-handler, so the chain rate-limits naturally. But the enclave-watcher's off-chain processing is not bounded by the intent doc.
- **Why it succeeds**: This is operational, not on-chain. The intent doc scope is "protocol-level behavior across all three components" (Section 1) — the enclave-watcher is "all three components" (it bridges chain ↔ enclave). DoS resistance of the enclave-watcher is a coverage gap.
- **Suggested defense**: Either (i) add a Section 7 note: "Off-chain enclave-watcher rate-limits its event processing; chain rate-limiting is not sufficient"; or (ii) gate `close_and_tally` to candidate-callers only (raising an authentication concern but eliminating the spam vector).

---

## Attack 30 — Section 5 non-goals lists 13 items; Non-Goal "No formal verification of inherited Quartz primitives" conflicts with Section 4.4 "in scope, central methodology target"

- **Category**: Ambiguity (cosmetic)
- **Severity**: serious
- **Scenario**: Section 5 final non-goal: "**No formal verification of inherited Quartz primitives at this layer.** verified-rcv asserts B8 (attestation-binds-tally) as depending on Quartz's `tdxVerifier` and `groth16Verifier` bundles; it does not re-verify those bundles."

  Section 4.4 ("enclave software bug"): "*integrity* — **in scope, central methodology target**". Mitigation: "Pre-deployment formal verification of the enclave's tally logic against the Lean and Quint specs derived from this intent document."

  These are consistent at the surface (verify the *tally logic*, not the *Quartz primitives*). But Section 6.2's inheritance table includes 5 rows; row 5 (`commitHashE`) is `(d) pigeonhole-impossible (lift restates as concrete-hash collision-resistance)`. Attack 5 shows verified-rcv's hash is structurally a different commitment from Quartz's. So either (a) verified-rcv inherits and the new hash is silently using the same impossible axiom (Attack 5 vacuity), or (b) verified-rcv writes a new collision-resistance hypothesis for its own hash — in which case "no formal verification of inherited Quartz primitives" no longer covers it (it's a new primitive, not inherited).
- **Why it succeeds**: The "what's inherited vs what's verified-rcv-specific" boundary is fuzzy. The intent doc Section 6.2 claims 5 inheritances; Attack 2 shows one is fabricated; Attack 5 shows another silently consumes an impossibility-axiom; Attack 3 shows the summand list disagrees with the upstream's. The "no formal verification ... at this layer" non-goal *also* implicitly takes verified-rcv-specific primitives off the verification table.
- **Suggested defense**: Split the non-goal: "No formal verification of upstream-Quartz axioms at this layer (their honesty is the Quartz colosseum's responsibility)". State separately: "Verified-rcv-specific cryptographic primitives (e.g., `hash(contract_addr, tally)`) ARE in scope and require their own collision-resistance hypothesis."

---

## Attack 31 — `winners ⊆ candidates ∧ len(winners) ≥ 1` over-specifies in the 0-candidate impossibility case but the spec rules it out via S1

- **Category**: Over-specification (cosmetic)
- **Severity**: cosmetic
- **Scenario**: S6 demands `len(winners) ≥ 1`. S1 demands `len(candidates) ≥ 1`. So `winners ⊆ candidates` non-empty is achievable. The over-spec is on the upper bound: there's no S6 bound `len(winners) ≤ len(candidates)` — implied but unstated.
- **Why it succeeds**: A malicious tally with `winners = candidates ++ [extra_address]` would pass `winners ⊆ candidates`? No — `⊆` rules that out. OK, S6 is actually tight. Minor finding: S6 should explicitly state `len(winners) ≤ len(candidates)` or note that this is implied by `winners ⊆ candidates` plus distinctness.

  More substantively: under "terminal-tie produces co-winners" (Attack 8), winners can be up to `len(candidates)`. The intent doc never bounds this — Section 2.2's all-abstain scenario explicitly has `winners = candidates`, but Section 5 non-goal "No partial preference orderings" means every ballot ranks all candidates. So the only way to get `winners = candidates` is total abstention. Section 2.2 illustrates this; S6 permits it. Consistent.
- **Suggested defense**: Optional: add a comment to S6: `winners distinct, len(winners) ≤ len(candidates)`.

---

## Attack 32 — "Forbids: negation of each Requires clause" in Block 1 is sloppy notation; under-specifies the error mapping

- **Category**: Ambiguity
- **Severity**: serious
- **Scenario**: Block 1 (line 122): "Forbids: negation of each Requires clause". Block 6 (line 193) similarly: "Forbids: negation of each Requires clause". But Block 6's Requires has 4 clauses; the Forbids would be the disjunction of their negations. Block 6's "On precondition failure" then maps to *three* specific error variants (`NotInTallyingState`, `AttestationFailed`, `InvalidTally`) — there are 4 Requires but only 3 error variants. Which Requires-failures map to which errors?

  Specifically: "tally is well-formed" Requires includes 4 sub-clauses (winners ⊆ candidates, per-round consistent, monotone, conservation). Each can fail independently. The error is `InvalidTally { reason }` — but `reason` is unstructured (a string?). An implementer would need to choose `reason`'s schema; the intent doc does not specify.
- **Why it succeeds**: "Forbids" is the contrapositive of "Requires"; explicit listing is redundant *unless* you want to specify which error fires for which violation. The Block 1/6 prose elides this. Block 3's prose actually does map errors to specific failures (lines 152-155). Inconsistent across blocks.
- **Suggested defense**: Either (a) drop "Forbids" entries entirely (redundant with Requires) and rely on the error-mapping prose, or (b) make Block 1's and Block 6's Forbids→error mapping as explicit as Block 3's.

---

## Attack 33 — B5 "no further publish_result tx ever succeeds" cites B1 plus Block 6's `AlreadyResolved` but B1 alone suffices

- **Category**: Triviality
- **Severity**: serious
- **Scenario**: B5 (line 239): "publish_result fires at most once | temporal | always (tally_result.is_none() ∧ next.tally_result.is_some() → no further publish_result tx ever succeeds) — derives from B1 plus Block 6's `AlreadyResolved` rejection".

  B1 says `tally_result` is monotone-once-set. If `tally_result.is_some()`, any subsequent state has `tally_result = same Some(T)`. So a successful publish_result would change `tally_result` to `Some(T')` for some T' — but B1 forbids this. So B1 alone forbids further successful publish_result. B5 is a derived corollary of B1. Stating it as an independent temporal invariant adds no content beyond B1.

  Section 8.5's witness scenario relies on B5 — but the proof goes through B1 + Block 6 trivially. B5 is redundant.
- **Why it succeeds**: An invariant that adds no constraints beyond its derivation source is trivial. The intent doc lists 9 behavioral invariants; if B5 is derived from B1, the doc has 8 independent + 1 redundant. The reviewer's attack on "9 invariants" is over-stated if B5 is folded into B1.
- **Suggested defense**: Either (i) mark B5 as derived from B1 and demote to a corollary (consistent with Attack 26's treatment of S5/B1 and S10/B3 redundancy), or (ii) strengthen B5 to a *distinct* property (e.g., "publish_result tx for two distinct tally bodies is impossible" — but B1 already implies this).

---

## Attack 34 — `TallyResult` type schema is referenced everywhere but never declared

- **Category**: Coverage gap
- **Severity**: serious
- **Scenario**: `TallyResult` is referenced in Section 2.1 (worked example output), Section 2.5 (state variable `tally_result: Option<TallyResult>`), Section 3.1 (S6–S9 all refer to fields), Section 6.6 (consumer interface). Its fields are inferred from the worked example:
  - `winners: Vec<Addr>`
  - `per_round_counts: Vec<HashMap<Addr, Nat>>`
  - `eliminated_by_round: Vec<Vec<Addr>>`
  - `ballots_tallied: Nat`
  - `ballots_dropped: Nat`
  - `dropped_voters: Vec<Addr>`
  - `non_voters: Vec<Addr>`

  But the intent doc never declares this schema in Section 2.5 ("State variables") or in any types subsection. A downstream Lean encoder would have to reverse-engineer the schema from worked examples — a known anti-pattern.
- **Why it succeeds**: The schema is *the* thing that downstream specs need most. Inferring from examples loses precision (is `per_round_counts` `Vec` indexed by round, or `Map<round_number, ...>`? Are absent addresses in `per_round_counts[i]` interpreted as eliminated or as zero? `non_voters` order — sorted, insertion order, candidate-order?).
- **Suggested defense**: Add Section 2.5 sub-section "TallyResult schema" formally declaring all 7 fields with types and ordering conventions.

---

## Attack 35 — `dropped_voters` is added/incremented inconsistently with `non_voters`

- **Category**: Under-specification
- **Severity**: serious
- **Scenario**: Section 2.3 malformed-ballot edge case: "Each drop increments `ballots_dropped` and adds the offending voter's address to `dropped_voters` in the tally result." So `len(dropped_voters) = ballots_dropped` ✓. But: can the same address appear *twice* in `dropped_voters` if a candidate's last-write-wins ballot was malformed but they also tried (and were rejected on-chain for `EmptyBallot`)? The on-chain rejection means there's only one stored `ballots[k]` blob (the last-write-wins one); if it's malformed, the candidate appears once. OK.

  Can an address appear in *both* `dropped_voters` and `non_voters`? A candidate who never submitted is in `non_voters`; a candidate who submitted a malformed ballot is in `dropped_voters`. Disjoint by construction ✓ — assuming each candidate appears in at most one of `{voted-OK, voted-malformed, did-not-vote}`. Consistent.

  But: S7's conservation `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)` assumes these are disjoint. The intent doc does not state the disjointness as an invariant. A bug in the enclave could produce a tally where an address appears in both `dropped_voters` and `non_voters`; arithmetic of S7 would still hold (`ballots_dropped = 1, len(non_voters) = 1, ballots_tallied + 1 + 1 = N + 1` — wait, that would *break* S7). OK, so S7 indirectly enforces disjointness.

  Subtler bug: a candidate appears in `dropped_voters` AND there's a stored `ballots[k]` blob for them. S7 holds. But Block 6 well-formedness doesn't check `dropped_voters ⊆ ballots.keys`. So a malicious enclave could publish a tally where `dropped_voters` contains an address that never submitted (false dropping accusation). S7 still holds if conservation is maintained.
- **Why it succeeds**: Block 6 well-formedness should require `dropped_voters ⊆ ballots.keys` and `non_voters = candidates \ ballots.keys`. The first is missing.
- **Suggested defense**: Add to Block 6 well-formedness: `dropped_voters ⊆ ballots.keys`, `non_voters = candidates \ ballots.keys`, disjointness `dropped_voters ∩ non_voters = ∅`.

---

## Attack 36 — Section 1 "Specifies protocol-level behavior across all three components" promises three-component scope; sections 2–8 cover the contract layer only

- **Category**: Coverage gap
- **Severity**: serious
- **Scenario**: Section 1 (line 8): "Specifies protocol-level behavior across all three components."

  The three components are: (i) CosmWasm contract, (ii) TDX enclave performing tabulation, (iii) zkdcap attestation. Sections 2.5's behavior blocks cover (i) and (iii)'s interaction (Block 6). The enclave's *internal* behavior — what tally function it runs, how it handles malformed ballots, how it streams events — is described only in prose (Section 2.3 edge cases, Section 8.1 step 6). No enclave-level state machine, no enclave-level invariants.

  In particular: the enclave reads `ballots`, runs IRV, produces `TallyResult` + `DstackAttestation`, submits via `publish_result`. The intent doc gives no block-spec for this enclave-side state machine. Section 6.5 covers voter-contract trust; Section 6.3 covers TDX hardware trust. There's no Section "behavior block for the enclave's tally computation".

  Yet Section 4.4 ("enclave software bug") makes the enclave's spec the *load-bearing methodology target*. Without an enclave-level state machine in the intent doc, the load-bearing target has no anchor.
- **Why it succeeds**: The intent doc treats the enclave as a black-box. The methodology target (4.4) requires a white-box spec. Mismatch.
- **Suggested defense**: Add Section 2.5 sub-block: "Enclave tally computation" — input: `ballots: Map<Addr, Vec<u8>>`, `enclave_privkey: PrivKey`, `candidates: Vec<Addr>`. Operation: decrypt each ballot under privkey; check parses as permutation of candidates; run IRV (with batch-elim rule from Attack 10); produce `TallyResult` + `DstackAttestation`. Output: submitted to `publish_result`.

---

## Attack 37 — Section 6.1's "ZK module endpoint semantics" trust assumption is silent on gnark vs circom

- **Category**: Ambiguity
- **Severity**: cosmetic
- **Scenario**: Section 6.1: "ZK module endpoint semantics — `/xion.zk.v1.Query/ProofVerifyGnark` returns true iff the supplied proof verifies against the registered vkey under gnark's verification algorithm. Trusted as Xion-spec'd; not re-verified at this layer."

  Quartz CLAUDE.md says: "zkdcap verifier migration: circom `ProofVerify` → gnark `ProofVerifyGnark` endpoint. Live blocker for testnet flow." So the gnark endpoint is the *target* but not yet the *current* state. The intent doc bets on a not-yet-merged Xion chain feature. The trust assumption holds *if* the gnark endpoint ships; otherwise verified-rcv is consuming an unstable interface.
- **Why it succeeds**: The intent doc's verification target depends on an upstream migration that is "Live blocker for testnet flow". Status uncertain.
- **Suggested defense**: Annotate Section 6.1 with the upstream-blocker note and the fallback if gnark migration slips.

---

## Attack 38 — Section 2.5 "Cross-block discipline" claims "every state appears as From state in exactly one block per outbound transition" — false for Tallying

- **Category**: Contradiction
- **Severity**: cosmetic
- **Scenario**: Section 2.5 cross-block discipline (line 203): "Every state appears as **From state** in exactly one block per outbound transition; entry to Created is via Block 1 (instantiate); entry to Voting is via Block 2 (implicit time advance from Created); entry to Tallying is via Block 4 (implicit time advance from Voting); entry to Resolved is via Block 6 (publish_result)."

  Tallying appears as From state in **Block 5** (close_and_tally, self-loop) **and** Block 6 (publish_result → Resolved). Two outbound transitions from Tallying. The next sentence says "The two Tallying self-loops (Block 5 and Block 6) have non-overlapping Produces" — but Block 6's "To state" is Resolved (terminal), not Tallying. So Block 6 is *not* a self-loop on Tallying; it's an outbound transition to Resolved. The doc calls it a "self-loop" twice (line 204 + line 207); the line 207 claim "Both fire from the same `From state`" is correct, but only Block 5 is a self-loop. Block 6 is a normal outbound transition.

  Minor: the "exactly one block per outbound transition" claim still holds (Block 5 = Tallying→Tallying self-loop is one transition; Block 6 = Tallying→Resolved is another; each is in exactly one block). But calling Block 6 a "self-loop" is wrong.
- **Why it succeeds**: Terminology confusion.
- **Suggested defense**: Clarify line 207: "Block 5 is a self-loop on Tallying; Block 6 transitions Tallying → Resolved. Both share From state = Tallying; they are distinguished by storage effect (no change vs `tally_result := Some(tally)`)."

---

## Attack 39 — `Adv_circuit_eq` is inherited from Quartz's `groth16Verifier.sound` decomposition, but the intent doc never names *which* circuit

- **Category**: Composition failure
- **Severity**: serious
- **Scenario**: B9 includes `Adv_circuit_eq(n)`. Per Quartz `Zkdcap.lean` (line 67–93), `circuit_equivalence` is the bound for "the zkdcap circuit faithfully encodes Intel SGX PCK chain validation up to the hard-coded Root CA, the DCAP quote-v4 signature check, and the binding of the in-quote `report_data` / `mr_td` fields to the public inputs."

  The verified-rcv intent doc does not specify *which* zkdcap circuit. Quartz's circuit is the one in `/Users/mvid/Development/reliq/zkdcap`. verified-rcv could use the same circuit (then the same `circuit_eq` advantage applies) or a different one (then a verified-rcv-specific `circuit_eq` advantage applies). The intent doc is silent.

  Section 6.1 references gnark and a "registered vkey" but never names the circuit. Section 6.2 inherits `groth16Verifier` (Quartz's). If the circuit is the same, inheritance is honest. If different, B9 silently inherits a different bound.
- **Why it succeeds**: The verified-rcv vkey is a verified-rcv-specific operational variable (Section 4.3a: "the verified-rcv `zkdcap_vkey`"). Same vkey → same circuit → inheritance OK. Different vkey → may or may not be same circuit. Intent doc doesn't specify.
- **Suggested defense**: State explicitly in Section 6.1 or 6.2: "The zkdcap circuit verified-rcv uses is identical to Quartz's (same vkey, same gnark binary)." If different, then the circuit-equivalence claim must be re-stated for verified-rcv's specific circuit, not inherited from Quartz.

---

## Attack 40 — Block 6's "(any chain address may call)" qualifier on publish_result is *implicit*; the doc says "submitted by the enclave; identity verified via attestation"

- **Category**: Ambiguity
- **Severity**: serious
- **Scenario**: Block 6 (line 184): "Trigger: `ExecuteMsg::PublishResult { tally: TallyResult, attestation: DstackAttestation }` (submitted by the enclave; identity verified via attestation)".

  But Block 5 (line 169) is "(any chain address may call)". Block 6 lacks this qualifier — implying restriction to the enclave? But the enclave is identified by *attestation*, not by `msg.sender`. The chain has no way to gate `publish_result` on `msg.sender = enclave_address` because the enclave does not have a stable chain address. Block 6 in practice can be called by anyone who *carries* a valid `DstackAttestation` and `tally`.

  An adversary who captures the enclave's publish_result tx in-flight (e.g., via mempool inspection) could re-broadcast it with a different `msg.sender`. Does the contract care? Block 6 Requires doesn't check `msg.sender`. So *anyone* can finalize the election by replaying the enclave's attestation+tally. This is intentional (the attestation, not the `msg.sender`, is the trust anchor), but the intent doc's parenthetical "(submitted by the enclave)" obscures this.
- **Why it succeeds**: A reader of Block 6 may believe `msg.sender` is checked — leading to a wrong Lean spec. The actual semantics is "anyone with a valid attestation can finalize".
- **Suggested defense**: Replace "(submitted by the enclave; identity verified via attestation)" with "(any chain address may submit; the enclave identity is verified via the carried `attestation`, not `msg.sender`)".

---

## Attack 41 — Block 1's "DstackKeyManager-issued keypair derivable for this contract instance" is a Requires but has no failure mapping besides `InvalidInstantiation`

- **Category**: Under-specification
- **Severity**: cosmetic
- **Scenario**: Block 1 Requires line 121: "DstackKeyManager-issued keypair derivable for this contract instance". On-failure (line 127): "DstackKeyManager key derivation failure" maps to `InvalidInstantiation`.

  But the Requires/Forbids/Produces format is supposed to be specified pre-transition. The DstackKeyManager call happens *during* the instantiation transaction; checking its success is a side-effect, not a precondition on the input. This is a mid-transaction side-effect that gets retconned as a Requires. The doc smudges over the in-transaction sequencing.
- **Why it succeeds**: Cosmetic — the modeling discipline says Requires should be checked on the input before side-effects; here the "Requires" is a side-effect that can fail.
- **Suggested defense**: Refactor Block 1: Requires = (length, distinctness, time-window); Produces = (storage update + DstackKeyManager call + enclave_pubkey set, atomically; the DstackKeyManager call is a CW2-style sub-transaction whose failure reverts the instantiation).

---

## Attack 42 — Section 2.1 redistribution mechanics conflate "B has 0 first-place votes" with "B is the unique lowest"; in the 5-voter table, A also has 2 first-place votes — was the elimination really unique-lowest?

- **Category**: Edge case (contradiction with Attack 1)
- **Severity**: serious
- **Scenario**: Section 2.1 Round 1 (table line 32): "B has 0 first-place votes (unique lowest). Eliminate B."

  From the votes: A=2, B=0, C=1, D=1, E=1. B has 0. C, D, E each have 1. A has 2. B is unique-lowest at 0. OK, the prose is right for Round 1.

  But: per Attack 1, B's ballot is `A > B > C > D > E` — B's *first* choice is A. So "B has 0 first-place votes" — yes, no candidate (including B) ranked B first. The doc is consistent here at the first-place-counts level.

  However: under "redistribute B's ballot after eliminating B", the surviving ranking is `A > C > D > E`. B's first surviving choice is A. So A should gain 1 vote in Round 2: A=2 + 1 = 3. But the table shows Round 2 A=2. So either (a) B's ballot is treated as already-counted-for-A (because B's first non-self choice is A) and redistribution drops the self-vote without re-counting — but B's first-place vote went to A in Round 1 (B ranked A as first), so it's already counted; OR (b) the elimination rule does not redistribute ballots whose top non-eliminated candidate has already received the vote. The doc's prose says "no movement" — consistent with the table — but the intuition behind "no movement" is that B's ballot's first surviving rank-1 candidate (A) already absorbed B's first-place vote in Round 1.

  This is correct IRV semantics: ballots' surviving rank-1 candidate gets the vote; if eliminating a candidate shifts the surviving rank-1 to a *new* candidate, that new candidate absorbs; if the surviving rank-1 is *already* the rank-1 (because the eliminated candidate was not rank-1 for this ballot), no movement. B's ballot's rank-1 = A (not B); eliminating B does not change A's rank-1 status on this ballot; A keeps the vote.

  So Round 1 → Round 2: A = 2 (unchanged), C = 1 (unchanged), D = 1 (unchanged), E = 1 (unchanged). The table line 33 shows A=2, C=2, D=1, E=1 — *C jumps from 1 to 2*. Where did C's extra vote come from? No ballot was redistributed (only B's was redistributable; B's ballot stays on A). So C = 1, not 2. The table is wrong.

  **This is Attack 1 confirmed independently.** The Round 1 → Round 2 transition arithmetic is broken.
- **Why it succeeds**: See Attack 1.
- **Suggested defense**: See Attack 1. Round 2 should show A=2, C=1, D=1, E=1 (no movement); Round 2 then has D and E tied for lowest at 1 with C also at 1 — three-way tie; batch elimination depends on Attack 10's rule.

---

## META

### Categories I attacked

- **Composition failure**: heavily — Section 6.2's Quartz-inheritance table is the spec's load-bearing trust transfer; cross-checking against Quartz's actual Lean + ledger surfaces multiple structural defects (Attacks 2, 3, 4, 30, 39).
- **Impossibility-hypothesis vacuity**: Attack 5 (B8 clause (c) silently consumes Quartz's `(d-pigeonhole-impossible)` axiom); Attack 4 (B9 inherits a Quartz lift that Quartz itself retracted as content-free).
- **Disjunction-vs-decomposition collapse**: Attacks 3, 17, 18, 19 — the 5-summand list of B9 is heterogeneous (mixes crypto + operational), non-disjoint (`tdxVerifier_sound` overlaps `KMS_leakage`), and disagrees with the upstream's 5-summand list.
- **Temporal-state mismatch**: Attack 6 (B6, the prime example — author explicitly warned the reviewer to look here, and the tag is wrong); Attack 7 (S6–S9 are well-formedness-pointwise but encode write-discipline temporal claims).
- **Preconditional over-strength**: Attack 16 — B8 consumes Quartz's classical-Prop `tdxVerifier.sound` without re-introducing the negligibility / preconditional qualifier.
- **Coverage gap**: Attacks 11 (InvalidTally deadlock), 20 (tally-correctness invariant missing), 23 (no liveness obligation), 29 (close_and_tally griefing), 34 (TallyResult schema), 36 (no enclave-level state machine).
- **Edge case**: Attacks 8 (terminal tie semantics), 9 (single-candidate abstention), 12 (`start_at == env.block.time`), 13 (`end_at == env.block.time`), 14 (B2's @end_at notation).
- **Contradiction**: Attacks 1 (worked-example arithmetic broken), 8 (terminal-tie contradiction), 15 (recovery-classification disagreement), 38 (cross-block self-loop terminology), 42 (Attack 1 confirmed independently).
- **Ambiguity**: Attacks 13, 14, 27, 30, 32, 37, 40.
- **Over-specification**: Attack 24 (1M gas as correctness invariant), 31.
- **Under-specification**: Attacks 10 (batch-elim rule), 21 (instantiator feedback), 22 (EmptyBallot), 28 (non_voters relation), 35 (dropped_voters disjointness), 41.
- **Triviality**: Attack 33 (B5 redundant with B1).

### Categories I did not attack

- **Refinement mismatch** standalone — covered transitively in composition failure (Attack 4). Without a downstream Lean spec yet drafted, no implementation-vs-system mismatch is available to attack.

### Artifacts I wanted access to but blindness restriction prevented

- The Quartz `examples/ranked-choice/` Lean specs and Quint files. Several attacks (especially 3, 5, 8, 10, 20, 36) would be sharpened by comparing the intent doc's IRV semantics to a known-working implementation. Per the blindness policy, I did not look. **Note for methodology**: that I felt the pull to look IS evidence that the intent doc is hard to validate in isolation. The IRV algorithm's batch-elimination rule, terminal-tie handling, and TallyResult schema are not derivable from the intent doc alone — they require either a reference implementation or stronger spec content.
- The verified-rcv `colosseum-intent` elicitation transcript (would help distinguish "author hadn't decided" from "author decided wrong"). Not provided; not requested.
- A draft of the downstream Quint spec (would let me run `temporal_state_mismatch` attacks against the actual encoding rather than the tag). Not yet produced — Round 3a is at spec-draft phase.

### Estimated confidence

**High**. Several findings (Attacks 1, 2, 3, 4, 5, 6, 8) are directly cross-verified against quoted upstream artifacts (Quartz Lean files, Quartz ledger). Attacks 11, 20, 36 cite intent-doc text plus methodology principles. The remainder are concrete scenarios tied to specific quoted clauses. The number of critical findings (13) is high but justified by the load-bearing claims being mis-stated; the spec under review is a first-draft intent doc, and the most damaging findings are in the cross-project composition section (6.2, B8, B9) where the cost of mis-statement compounds downstream.

### Methodology back-port asks for v0.3

1. **Cross-project inheritance audit step**: when a spec's Section "Trust Boundaries" claims to inherit from an upstream Lean spec, the methodology should require the reviewer to literally `grep` the named artifacts. The first three critical findings (Attacks 2, 3, 4) would have been caught by a mechanical "named artifact exists in upstream tree?" + "named summand list matches upstream's?" check.

2. **Worked-example arithmetic check**: Attack 1 / Attack 42 found a 5-line table that is arithmetically wrong. A `colosseum-intent` post-elicitation step that runs each worked example through a paper-and-pencil simulator (or, for IRV-like systems, an independent reference implementation) would catch this before the adversary sees it. The intent doc treats Section 2.1 as the canonical "if a spec writer reads only this section, they should be able to characterize the system's input-output relation" — and the canonical example is broken.

3. **Tag-confidence discipline for state/temporal**: Section 3.2 explicitly invites the adversary to attack the state/temporal tags. The methodology might formalize this as a mandatory "for each invariant, write down what state-extension would make the state-only formulation honest; if the extension is non-empty, the tag should be temporal". Attack 6 (B6) and Attack 7 (S6–S9) both turn on this discipline.

4. **Multi-summand bound provenance**: when a spec claims to inherit an N-summand bound from upstream, methodology should require:
   - List the upstream's N summands by name.
   - List the downstream's N summands by name.
   - Justify each addition (new claim, new layer) and each subtraction (collapsed, out of scope).
   Attack 3 caught a 5→5 silent substitution; without a methodology checklist, this would have shipped.

5. **Retraction-propagation**: the Quartz ledger was retracted *on the same day* this intent doc was written. Downstream specs depending on retracted upstream artifacts should automatically inherit a retraction flag. Currently nothing in the intent doc references the Quartz retraction; the verified-rcv author was either unaware or chose not to mention. Methodology should require the compose ledger to surface upstream-ledger retractions before the downstream spec is committed.
