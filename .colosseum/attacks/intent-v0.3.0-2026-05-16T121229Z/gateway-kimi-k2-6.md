# kimi-k2-6 (gateway) — verified-rcv revised intent adversarial pass

- **Elapsed**: 124.8s
- **Finish reason**: length
- **Usage**: {"prompt_tokens": 25540, "completion_tokens": 8192, "total_tokens": 33732, "prompt_tokens_details": {"cached_tokens": 0}}

---

The user wants me to act as a hostile spec reviewer for the verified-rcv intent document v0.3.0. I need to find specification-layer attacks, grounded in specific text, across the emphasized categories and all other categories.

Let me carefully read through the document and identify attacks.

Key areas to focus on per instructions:
1. temporal_state_mismatch: B6 was re-formulated to ∀-per-key. Does it have a state/temporal mismatch? Section 3.2 tagging discipline.
2. impossibility-hypothesis-vacuity: B8 clause (c), Quartz commitHashE inheritance (Section 6.2), B9's image_registration_honest, B10's cross-layer decomposition.
3. disjunction-vs-decomposition: B10 cross-layer factorization and B9's negligibility decomposition.
4. composition_failure: Section 6.2 Quartz inheritance table, de-retracted status, commitHashE row "NOT CONSUMED".
5. canonical_serialization pinned to Borsh in Section 2.5.
6. transaction trace model in Section 2.5 and B6's tie-back.
7. Tally_spec vs IRV_spec split and arities.
8. S5 restated as set-once write-discipline.
9. B10_lean row in Section 3.2.
10. B9 conditioned on image_registration_honest.
11. Section 6.2 [2026-05-15 — de-retracted] status block.
12. Section 8.7's 6-numbered-step decomposition.
13. SemVer header and revision history classification.

Let me go through systematically.

**Attack 1: Temporal-State Mismatch on B6**

B6 is tagged `temporal` in Section 3.2. The statement is:
`always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`

Wait, the tag is `temporal`, but the quantification is over `fires_at_transition(σ → σ′)`. The document says in Section 3.2 preamble: "state invariants discharge at every state independently; temporal properties require explicit history quantification downstream."

But B6's formulation uses `next.ballots[k] ≠ ballots[k]` which is a next-state relation, and `fires_at_transition(σ → σ′)`. This is indeed a temporal property (it relates current state to next state). However, the question is whether the ∀-per-key formulation introduced a state/temporal mismatch.

Wait, the emphasis says: "Specifically: B6 was re-formulated to ∀-per-key in the v0.3.0 revision — did the new shape introduce a state/temporal mismatch?"

Let me look at B6 carefully. It says:
`always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), ...)`

The `fires_at_transition(σ → σ′)` is defined in Section 2.5 transaction trace model as "the multiset of transactions that fired in the block taking σ to σ′".

But here's a potential issue: B6 is tagged `temporal`, but the property is actually about the *transition*, not a pure state property. However, the document explicitly says B6 is temporal. Is there a mismatch?

Wait, the attack category `temporal_state_mismatch` is: "an invariant tagged `state` that actually quantifies over history (or vice versa)." So I need to find something tagged `state` that quantifies over history, or tagged `temporal` that is actually state-evaluable (or vice versa).

Actually, looking at B6: it's tagged `temporal`. It quantifies over `next.ballots` and `fires_at_transition`. That's clearly temporal. But is there a state/temporal mismatch in the *other* direction?

What about S5? S5 is tagged as derived, in Section 3.1 (structural invariants). It says: "`tally_result: Option<TallyResult>` is set-once: no handler writes to `tally_result` after it has been set to `Some(_)`. Evaluable on a single state by inspecting the handler index, not by quantifying over successor states."

Wait, "Evaluable on a single state by inspecting the handler index" — is this actually true? To know that "no handler writes to `tally_result` after it has been set to `Some(_)`", you need to know the program/code. But the invariant is about the *state* satisfying that the write-discipline holds. Actually, the document claims it's pointwise evaluable by inspecting the handler index. But to evaluate it on a single state, you'd need to know which handlers are defined — that's a static property of the contract, not a dynamic property of the state. But the document says it's pointwise per the preamble.

However, S5 is in Section 3.1 (Structural invariants). The preamble says "No quantification over operations or time." But S5's statement "no handler writes to `tally_result` after it has been set to `Some(_)`" — if interpreted as a property of the code, it's static. If interpreted as a property of the state trajectory, it's temporal. The document claims it's state-evaluable by inspecting the handler index. This seems like a stretch but maybe acceptable.

Wait, the emphasis asks specifically about B6's ∀-per-key formulation. Let me look for a mismatch there.

B6: `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ...)`

The issue might be that B6 is tagged `temporal`, but the `∀ k` quantification over *all* addresses means that for addresses where `next.ballots[k] = ballots[k]`, the implication is vacuously true. But the existential `∃ tx ∈ fires_at_transition` — if multiple ballots change in one block, there must be multiple transactions. But CosmWasm executes transactions sequentially in a block, not atomically in a single transition. The `fires_at_transition(σ → σ′)` is a multiset of transactions. 

Actually, the document says "at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order." So a transition can include multiple `SubmitBallot` transactions from different senders. B6 handles this: for each key that changed, there exists a tx in the firing set.

But is there a state/temporal mismatch? The tag is `temporal`. The property uses `next.ballots` and `fires_at_transition`. That's temporal. No mismatch in tagging.

Wait, could B6 be mis-tagged? No, it's correctly temporal. The question is whether the new shape introduced a mismatch. Perhaps the `∀ k` makes it look like a state invariant (quantifying over all keys in a state), but it's actually temporal. But the tag is temporal, so that's fine.

Let me look elsewhere for temporal_state_mismatch.

What about S10? "resolution implies past end_at" — tagged as derived structural. Statement: `if tally_result.is_some(), then env.block.time ≥ end_at`. This is pointwise evaluable on a state. But the document says it's derived from B3 (temporal causal version). S10 is correctly tagged state because it's just an implication about current state. No mismatch.

What about B10_lean? Tagged `off-chain`. Not state or temporal.

Wait, the emphasis says: "Section 3.2 tagging discipline is the central test. Specifically: B6 was re-formulated to ∀-per-key in the v0.3.0 revision — did the new shape introduce a state/temporal mismatch?"

Maybe the issue is that B6's `∀ k ∈ Addr` quantifies over the entire address space, which includes non-candidates. But `next.ballots[k]` is only defined for candidates? Actually, `ballots: Map<Addr, Vec<u8>>` — absent means no vote. So `ballots[k]` is defined for all `k` (as an option). The property says if the value changed, then there was a SubmitBallot tx from `k`. But Block 3 requires `msg.sender ∈ candidates`. So if `k ∉ candidates`, `next.ballots[k]` can never change (because Block 3 forbids it). But B6 doesn't restrict `k` to candidates. For non-candidates, the antecedent `next.ballots[k] ≠ ballots[k]` is always false, so it's vacuously true. That's fine.

But wait — is there a temporal/state mismatch in B6's formulation? The `fires_at_transition(σ → σ′)` is defined in Section 2.5. B6 says `∃ tx ∈ fires_at_transition(σ → σ′)`. But `fires_at_transition` is defined as "the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking σ to σ′". This is a temporal concept. B6 is tagged temporal. So no mismatch.

Hmm, let me reconsider. Maybe the mismatch is that B6 is tagged `temporal`, but the `∀-per-key` form actually makes it a *state* invariant about the transition relation? No, the tag `temporal` is correct.

Wait — could the issue be that B6 is in the "Behavioral invariants" table with tag `temporal`, but the property actually mixes state and temporal in a way that makes it neither? Or perhaps the `∀ k` should be bounded to `ballots.keys` or candidates?

Actually, looking at the instruction emphasis: "did the new shape introduce a state/temporal mismatch?" Maybe the answer is no, but I should check carefully.

Let me look at B8. Tagged `temporal`. It says `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result tx that fired carried...)`. This refers to "the publish_result tx that fired". This is temporal.

What about B10? Tagged `cross-layer`. It says `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey...)`. This is temporal in shape (always, next, etc.) but tagged cross-layer. The document explicitly says "Tag is `cross-layer`, not `temporal`". Is this a mismatch? The definition of temporal_state_mismatch is "an invariant tagged `state` that actually quantifies over history (or vice versa)." B10 quantifies over history (`always`, `next`) but is tagged `cross-layer`, not `state`. The mismatch would be if it's tagged `state` but is temporal, or tagged `temporal` but is state. B10 is tagged `cross-layer` and is temporal. The preamble says behavioral invariants are tagged state or temporal. B10 is an exception. The document justifies this. So maybe not an attack.

But wait — B10_lean is tagged `off-chain`. That's also not state/temporal. The preamble doesn't mention off-chain. But the document explicitly added it.

Let me look for impossibility-hypothesis-vacuity.

**Attack 2: Impossibility-Hypothesis Vacuity in B8 clause (c)**

B8 clause (c): `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`.

The attestation is a DstackAttestation. The document says in Section 6.2 that `commitHashE` is "NOT CONSUMED by verified-rcv". But B8(c) uses SHA-256 on Borsh serialization. 

Where is the impossibility? Look at B10's decomposition: `B10 ← B10_lean ∧ image-identity-binding ∧ B8`.

B8 clause (c) binds the attestation to the tally. But the `canonical_serialization` is pinned to Borsh. However, the document says "Two implementations of the enclave that produce different byte representations for the same logical input therefore disagree on B8(c) and one of them is non-compliant."

Wait, the impossibility-hypothesis vacuity category: "a hypothesis in the spec is provably false in the surrounding category, making the theorem vacuously true."

Look at B9: `always (image_registration_honest → Pr[B8 violated ...] ≤ ... )`

The hypothesis is `image_registration_honest`. Is this provably false? No, it's an operational precondition.

Look at B8: it depends on Quartz's `tdxVerifier` and `groth16Verifier`. The document says these are classically-over-strong. Specifically, the table in 6.2 says:
- `tdxVerifier`: (d) classically-over-strong-single-negligibility (sound) + preconditional (complete)
- `groth16Verifier`: (d) classically-over-strong-doubled-negligibility

The "classically-over-strong" sub-tag means the spec states an axiom or theorem as a pure Prop when honesty requires a precondition. This is exactly the `preconditional over-strength` category. But the question asks about `impossibility-hypothesis vacuity`.

Wait, the emphasis says: "Examine B8 clause (c), the de-retracted Quartz commitHashE inheritance (Section 6.2 status block was UPDATED in v0.3.0), B9's image_registration_honest hypothesis (newly added in v0.3.0), and B10's cross-layer decomposition into B10_lean ∧ image-identity-binding ∧ B8."

For B8 clause (c): The attested user_data is SHA-256(...). The document says "32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot; upper half reserved for domain-separation tag (implementation detail)." 

Is there a vacuity issue? If the UserData slot is 64 bytes, and we embed a 32-byte digest in the lower half, then the attested user_data is 64 bytes. But B8(c) says `attested user_data = SHA-256(...)`. SHA-256 is 32 bytes. So the attested user_data (64 bytes) cannot equal SHA-256 (32 bytes). This is a type/length mismatch! The attested user_data is the full 64-byte slot, but the hash is 32 bytes. The document mentions "upper half reserved for domain-separation tag", so the actual attested user_data is `domain_tag ‖ hash` or something. But B8(c) states equality with just the hash. This makes B8(c) unsatisfiable — the attested user_data is 64 bytes, SHA-256 is 32 bytes. Therefore, the antecedent of B8 (next.tally_result.is_some() ...) can never be satisfied with a valid attestation that matches B8(c), or rather, B8(c) as written is impossible to satisfy because the types don't match.

Wait, let me read carefully: "attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))". And the note says: "32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot; upper half reserved for domain-separation tag (implementation detail)."

So the actual attested user_data is 64 bytes, but B8(c) says it equals a 32-byte hash. This is a contradiction. The enclave will put a 64-byte value (with domain separation), but B8(c) requires it to equal a 32-byte hash. This makes B8(c) impossible to satisfy. Thus any theorem relying on B8(c) as a hypothesis is vacuously true? Or rather, B8 as stated is false — no attestation can satisfy it because of the length mismatch.

Actually, the note says "32-byte digest embedded in the lower half... upper half reserved". This implies the attested user_data contains the hash in the lower half, but is not *equal* to the hash. B8(c) should say something like "the lower 32 bytes of attested user_data equal SHA-256(...)" or "attested user_data contains SHA-256(...) at offset 0". But as written, it's equality. This is a critical specification error.

Severity: critical. B8 is the core attestation binding. If B8(c) is impossible, the entire attestation chain is broken.

Suggested defense: Change B8(c) to "the lower 32 bytes of attested user_data equal SHA-256(...)" or define `attested user_data` as the lower half, or specify the exact 64-byte construction.

**Attack 3: Impossibility-Hypothesis Vacuity in B9's decomposition**

B9 says: `Pr[B8 violated ...] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)`

The hypothesis `image_registration_honest` is a precondition. But look at the summands. The document says in Section 6.2 that Quartz's `cross_component_session_bind_negl` was de-retracted and now def-ties win events. However, the `tdxVerifier` and `groth16Verifier` bundles are tagged as classically-over-strong. 

Specifically, the `tdxVerifier` row says: "(d) classically-over-strong-single-negligibility (sound) + preconditional (complete)". This means the soundness axiom is stated as a pure Prop (no adversary parameter, no preconditions about honest platform) when it should be preconditional. If the axiom is classically over-strong, it might be unsatisfiable in the standard model (e.g., it claims soundness unconditionally, but TDX quotes can be forged if the platform is compromised). However, the document says the Quartz spec has this sub-tag. Verified-rcv inherits it.

But the impossibility attack: B9's bound includes `Adv_circuit_eq(n)`. The document says in Section 6.2 that `groth16Verifier` has a "reference DCAP circuit-equivalence theorem" as its discharge path. But the `circuit_eq` hypothesis — is it possible that `Adv_circuit_eq` is actually 0 or vacuous? No, circuit equivalence is a concrete statement.

Wait, look at the `commitHashE` row. It says "(d) pigeonhole-impossible". The bundle is listed but NOT CONSUMED. However, if any other hypothesis in B9 or B8 is pigeonhole-impossible, that would be vacuity. But the document explicitly says verified-rcv does NOT consume `commitHashE`.

What about B10's cross-layer decomposition: `B10 ← B10_lean ∧ image-identity-binding ∧ B8`.

If `image-identity-binding` is not a formal hypothesis but a build-pipeline claim, then the "theorem" B10 might be vacuously true because `image-identity-binding` is never formally discharged. But that's not impossibility-hypothesis vacuity; that's just informal.

Let me look at B8 clause (c) again — that's the strongest attack.

**Attack 4: Disjunction-vs-Decomposition Collapse in B10**

Section 8.7 says: `B10 ← B10_lean ∧ image-identity-binding ∧ B8`.

Is this a decomposition or a disjunction? It's a conjunction. But the attack category `disjunction-vs-decomposition collapse` is about a hardness assumption that is genuinely disjunctive being stated as monolithic, OR a decomposition that fails to decompose.

The emphasis says: "Examine the B10 cross-layer factorization and B9's negligibility decomposition."

For B10: The factorization is `B10_lean ∧ image-identity-binding ∧ B8`. But B10 is a cross-layer claim. The "decomposition" into three conjuncts is honest — each is necessary. However, is `image-identity-binding` actually a formal hypothesis? The document says: "This is an extraction-soundness + build-reproducibility obligation that lives at the build pipeline + deployment ledger layer, not the Lean layer. It cannot be a Lean theorem because the Lean spec models the binary; the binding claim is about the modelling discipline."

So the "decomposition" claims that B10 follows from three things, but one of them (`image-identity-binding`) is not a theorem/proposition in the formal system — it's a meta-level claim about build reproducibility. This means the decomposition is not a formal disjunction or decomposition within the proof calculus; it's a hand-wavy composition where one leg is outside the logic. This is a composition failure, but also potentially a disjunction-vs-decomposition issue if the intent is to present this as a formal factorization.

For B9: The negligibility decomposition is 4 summands. The document says: "5 summands in earliest draft → 4 after Round 3a first-pass adversary (dropped Adv_KMS_leakage/confidentiality and Adv_image_registration/operational per Attacks 17+18) → still 4 here, but with the Adv_image_registration failure mode now made explicit as the LHS precondition image_registration_honest rather than absorbed silently."

The question is: are these summands actually independent? The bound is a sum of advantages. But `Adv_tdxVerifier_sound`, `Adv_groth16_KS`, `Adv_circuit_eq`, `Adv_commitTally_CR` — are these the correct decomposition? The document says the first three are inherited from Quartz's `cross_component_session_bind_negl`. But in Section 6.2, the `groth16Verifier` row is tagged "(d) classically-over-strong-doubled-negligibility". This suggests there might be two summands there, not one. The "doubled-negligibility" sub-tag means the bundle contains two independent negligibility hypotheses that are conflated. If `Adv_groth16_KS` and `Adv_circuit_eq` are actually two separate assumptions, but B9 lists them as separate summands, that's fine. But if the `tdxVerifier` bundle's classically-over-strong-single-negligibility actually hides a precondition that makes the hypothesis unsatisfiable, then the sum is vacuous.

Wait, the `disjunction-vs-decomposition` category specifically mentions: "a hardness assumption that is genuinely disjunctive — e.g., 'Groth16 knowledge-soundness and circuit-vs-reference equivalence' — is stated as a single monolithic hypothesis. The proof passes; the trust claim is misrepresented as one assumption when honesty requires two. Variant: a hypothesis stated as a single negligibility bound when the underlying advantage decomposes into a sum."

In Section 6.2, `groth16Verifier` is tagged `(d) classically-over-strong-doubled-negligibility`. This means the Quartz spec bundles TWO negligibility assumptions into one record, but they are classically over-strong. B9 separates them into `Adv_groth16_KS` and `Adv_circuit_eq`. So B9 does decompose them. That seems correct.

But what about the `tdxVerifier`? It's tagged `(d) classically-over-strong-single-negligibility (sound) + preconditional (complete)`. The "single-negligibility" might mean soundness and completeness are bundled, but completeness is preconditional. B9 only includes `Adv_tdxVerifier_sound`. It doesn't include a completeness term. Is that correct? The attestation needs both soundness and completeness. If the completeness is preconditional (only holds for honest platforms), and B9 doesn't account for completeness failures, then the decomposition is incomplete. But the document says completeness is preconditional.

Actually, the attack here might be that B9's 4-summand bound is presented as a complete decomposition, but the `tdxVerifier` bundle actually contributes *two* summands (soundness and completeness), while B9 only includes one (`Adv_tdxVerifier_sound`). The completeness precondition is omitted. However, the document says completeness is "preconditional" — meaning it's an assumption on the environment, not a cryptographic advantage. So maybe it's fine.

But wait — the `tdxVerifier` row says "(d) classically-over-strong-single-negligibility (sound) + preconditional (complete)". The "single-negligibility" refers to soundness. Completeness is separate and preconditional. B9 only bounds soundness failures. But what about completeness? If the verifier is not complete (i.e., it rejects valid attestations), that's a liveness issue, not an integrity issue. B9 is about `Pr[B8 violated]`, which is an integrity property (a bad attestation is accepted). Completeness doesn't affect integrity. So omitting completeness from the bound is correct.

Let me look at B10 decomposition again. Section 8.7 step 6 says: "Composition — B10: from B10_lean (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey..."

The issue: B10 is stated as `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(...)))`.

The decomposition `B10 ← B10_lean ∧ image-identity-binding ∧ B8` is not a formal implication in any single logic. B10_lean is a Lean theorem (universal quantification over all inputs). Image-identity-binding is a build-pipeline claim. B8 is a temporal invariant. Combining them to get B10 requires a meta-logical step that is not formally justified in the document. Specifically, B10_lean talks about `EnclaveImage(raw_ballots, candidates, privkey)`, but B10 talks about the specific enclave image that produced the attestation in B8. The link between them is `image-identity-binding`, which says the registered MRTD matches the Lean-proven binary. But B8 says the attested enclave identity matches the registered image. So chaining them: attested image = registered image = Lean-proven binary. But `image-identity-binding` is not a theorem; it's a build claim. The document admits this. So the "decomposition" is actually a disjunction of trust assumptions across different layers, presented as a conjunction. The attack is that this is not a decomposition within a single formal system, but a cross-domain handoff. However, the category `disjunction-vs-decomposition collapse` is more about conflating disjunctive assumptions into a single monolithic hypothesis. Here, the issue might be the opposite: a genuine conjunction presented as a formal decomposition when one leg is non-formal.

Actually, the category description says: "a hardness assumption that is genuinely disjunctive ... is stated as a single monolithic hypothesis." I don't see that in B10. B10 is a conjunction.

Let me reconsider B9. The document says: "The first three summands are inherited from Quartz's now-content-bearing cross_component_session_bind_negl". But in Section 6.2, the Quartz inheritance table lists `tdxVerifier` and `groth16Verifier` as separate bundles. The `cross_component_session_bind_negl` is not listed as a separate inherited claim; it's mentioned in the B9 text and the status block. Is `cross_component_session_bind_negl` a single theorem that bundles multiple assumptions? If B9 decomposes it into 3 summands, that's good. But if the Quartz-side theorem is actually a disjunction (e.g., "either tdx is sound OR groth16 is KS"), then decomposing it into a sum would be wrong. The document doesn't suggest it's a disjunction. So maybe no attack here.

**Attack 5: Composition Failure in Section 6.2 Quartz Inheritance**

The document says `commitHashE` is "NOT CONSUMED by verified-rcv" and "the row is retained in the table for traceability... but the verified-rcv compose ledger will record this inheritance row's downstream consumer count as zero."

But the table lists `commitHashE` with discharge path "VCVio randomOracle + [Fintype UserData] carrier refinement". The sub-tag is "(d) pigeonhole-impossible". The document says: "that bundle's (d) pigeonhole-impossible sub-tag would make any inheritance vacuous."

If verified-rcv does not consume it, why is it in the table? The document says "for traceability". But composition failure is about "cross-project inheritance claims that don't actually compose at the layer claimed." The `commitHashE` row is explicitly marked as not consumed. Is there a composition failure elsewhere?

Look at the withdrawn inheritance row: `DstackKeyManager` was removed because `grep` returned zero hits. Good.

But what about the de-retracted status? The document says: "verified-rcv inherits B8/B9 from a now-contented substrate." But Section 6.2 status block says the Quartz substrate was de-retracted. However, the document also says: "Known v0.3 caveats inherited from Quartz: Asks 6+7 were added... documented form in the Quartz PR; enforced form is gated on the executable-layer decision."

This means the de-retracted substrate has "documented form" but not "enforced form" for asks 6 and 7. Verified-rcv claims to inherit from a content-bearing substrate, but the substrate's content is not fully enforced. This is a composition failure: verified-rcv's B9 depends on Quartz's def-tied lifts, but those lifts have caveats (documented but not enforced). The inheritance claim is premature.

Specifically, the document says: "The verified-rcv compose ledger (forthcoming) will record this dependency graph and pick up bundle-cardinality drift signals from the Quartz ledger." But the ledger is "forthcoming" — it doesn't exist yet. So the inheritance is not actually composed.

Also, the `commitHashE` row: even though it's marked not consumed, the table still lists it as an "explicit upstream dependency that the verified-rcv compose ledger must record." But if it's not consumed, it's not a dependency. Including it creates confusion.

More importantly, look at B8: "Clauses (a) and (b) are inherited from the Quartz attestation spec". But the Quartz inheritance table lists `tdxVerifier` and `groth16Verifier` as inherited claims. However, B8 clause (c) and (d) are verified-rcv-specific. The composition theorem B8 depends on these. But the table says `tdxVerifier` is classically-over-strong-single-negligibility. This means the hypothesis is unsound to assume as a pure Prop. B8 states B8 as a deterministic implication (classical Prop). The document admits this: "Classical-Prop shadow note: B8 is stated as a deterministic implication; the cryptographic content lives in B9's negligibility budget."

This is a composition failure: B8 is stated as a categorical implication (`always (P → Q)`), but its inherited content (tdxVerifier soundness) is only true up to a negligible probability. The composition between the deterministic B8 and the probabilistic B9 is broken because B8 claims determinism while B9 admits probabilistic failure. Consumers reading B8 as a categorical implication (Section 6.6) are misled. The document acknowledges this but doesn't fix it.

**Attack 6: Canonical Serialization Ambiguity**

Section 2.5 says canonical serialization is pinned to Borsh. But does it explain why Borsh over alternatives? No. Are there ambiguity-free guarantees? The document says:
- `canonical_serialization(contract_addr ‖ tally_body)` denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`
- `tally_body` is serialized field-by-field in declaration order
- `Vec<Addr>` and `Map<Addr, Nat>` entries are emitted in candidate-declaration order

But Borsh serialization of `Map<Addr, Nat>` — is the length prefix included? Borsh typically includes a length prefix for vectors and maps. The document says "entries are emitted in candidate-declaration order". But a Map in Borsh is typically serialized as a sequence of key-value pairs, but if it's a `Map` type (like `BTreeMap`), the order might be sorted by key, not insertion order. The document says "candidate-declaration order", but Borsh's default serialization for Rust's `BTreeMap` sorts by key. If the implementation uses a map type that Borsh serializes in sorted order, but the document says candidate-declaration order, there is a mismatch.

Specifically, the document says: "`Vec<Addr>` and `Map<Addr, Nat>` entries are emitted in **candidate-declaration order** (matching the ordering discipline above), making `borsh_bytes(tally_body)` a deterministic function of the logical value."

But Borsh does not guarantee this for all map types. If `Map<Addr, Nat>` is implemented as a `HashMap`, Borsh serialization order is non-deterministic. If it's a `BTreeMap`, it's sorted by key. The document assumes candidate-declaration order, but Borsh alone does not enforce this. The spec must require the implementation to use an ordered map (like `Vec<(Addr, Nat)>` or an ordered map) and serialize in that order. Otherwise, two compliant implementations could produce different serializations for the same logical value if they choose different map representations.

This is an under-specification: the intent says deterministic-by-candidate-declaration-order, but Borsh alone doesn't guarantee it for maps.

**Attack 7: Transaction Trace Model and B6 Tie-Back**

Section 2.5 transaction trace model says: "A state transition σ → σ′ is parameterized by the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking σ to σ′ — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order."

But B6 says: `∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`.

The issue: CosmWasm executes transactions sequentially, each potentially producing a different intermediate state. The `fires_at_transition` model treats the block as a single transition from σ to σ′ with a multiset of transactions. But if transactions are executed sequentially, the state after the first tx is an intermediate state, not σ′. The `next.ballots[k]` in B6 refers to the final state σ′. However, if two transactions both submit ballots for the same candidate k (e.g., a candidate submits twice in one block), the first tx sets `ballots[k] = tx1.encrypted_preferences`, and the second tx overwrites it with `tx2.encrypted_preferences`. In the final state σ′, `ballots[k] = tx2.encrypted_preferences`. B6 requires `∃ tx ∈ fires_at_transition, tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`. This is satisfied by tx2. Good.

But what if a transaction from sender k is included in the block, but another transaction from sender k is also included? The document says "at most one per slot per sender per block". Wait, "at most one per slot per sender per block" — does this mean a sender can have multiple transactions in a block if they are in different slots? Or is it restricted to one per sender per block? The document says "at most one per slot per sender per block". This is ambiguous. In CosmWasm, a sender can typically have multiple transactions in a block. If they do, B6's existential is satisfied by the last one (since next.ballots[k] equals the last write). But B6 says "the change is attributable to a SubmitBallot transaction in that transition". It doesn't require uniqueness. So that's fine.

But the trace model says "ordered by CosmWasm's deterministic execution order." B6 doesn't use the order. The `fires_at_transition` is a multiset. But B6's ∀-per-key form might fail if a ballot changes due to a transaction that is NOT a SubmitBallot? No, only Block 3 writes to ballots. So that's fine.

Wait, the emphasis asks: "does B6's ∀-per-key formulation correctly tie back to this trace model?" The trace model defines `fires_at_transition(σ → σ′)`. B6 uses it. But does B6 correctly handle the case where `next.ballots[k] ≠ ballots[k]` but the change was caused by a transaction that is not in `fires_at_transition`? No, because all state changes come from transactions in the block.

However, there is a subtle issue: the trace model says "at most one per slot per sender per block". But B6 quantifies over `∀ k ∈ Addr`. If a sender submits a ballot, `msg.sender = k`. But what if a contract (not a candidate) calls `submit_ballot` on behalf of a candidate via a wrapper? In CosmWasm, `msg.sender` is the direct caller. If a candidate uses a smart contract wallet, `msg.sender` is the contract address, not the candidate's address. Block 3 requires `msg.sender ∈ candidates`. So a smart contract wallet that is not itself a candidate cannot submit a ballot. But if a candidate is a smart contract, its address is in `candidates`, and `msg.sender` is that contract address. That's fine.

But the trace model's `tx.msg.sender` — is this the same as `msg.sender` in Block 3? Yes.

Actually, the real issue with B6 and the trace model: B6 is tagged `temporal`, but the trace model paragraph says "Behavioral invariants (B-series) that quantify over `∃ tx ∈ fires_at_transition(σ → σ′)` are reasoning about *that transition's* firing set, not the global history." This is exactly what B6 does. So it seems correct.

Wait, the emphasis says: "did the new shape introduce a state/temporal mismatch?" Maybe the mismatch is that B6 uses `fires_at_transition(σ → σ′)` which is defined as part of the transaction trace model, but the trace model says "downstream Quint encodes this as an action label on each transition; downstream Lean encodes it as a relation parameter on `step : State → Tx → State → Prop`." In the Lean encoding, `step` takes a *single* `Tx`, not a multiset. So B6's `∃ tx ∈ fires_at_transition` cannot be directly encoded in the Lean `step` relation as written, because `step` is parameterized by a single tx, not a multiset. The document says "downstream Lean encodes it as a relation parameter