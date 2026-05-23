# kimi-k2-6 (gateway) — verified-rcv revised intent adversarial pass

- **Elapsed**: 133.3s
- **Finish reason**: length
- **Usage**: {"prompt_tokens": 20863, "completion_tokens": 8192, "total_tokens": 29055, "prompt_tokens_details": {"cached_tokens": 4160}}

---

The user wants me to act as a hostile spec reviewer for the revised intent document of verified-rcv. I need to find specification-layer attacks, grounded in specific text, categorized appropriately, with severity ratings.

Key emphasis areas:
1. `temporal_state_mismatch`: Check Section 3.2 tagging discipline.
2. `impossibility-hypothesis-vacuity`: Examine B8 clause (c), Quartz `commitHashE` inheritance, Section 6.2 retracted-substrate status block.
3. `disjunction-vs-decomposition`: Examine B9's 4-summand list.
4. `composition_failure`: Cross-project inheritance claims in Section 6.2.
5. B10 (tally-correctness) — is RHS well-defined? Arity match Section 2.5?
6. Re-tagged B6 (state → temporal) — is new formulation correct?
7. B9 4-summand list (revised from 5) — did anything load-bearing get dropped?
8. Section 2.1 worked example — is arithmetic right?
9. New Section 4.9 (deterministic-malformed-tally deadlock) — is it sound?
10. New Block E1 (enclave-side state machine) — is it complete?
11. New Section 8.7 (B10 off-chain witness) — is decomposition honest?
12. B8/B9 retracted-substrate status block in Section 6.2 — is framing honest?

I need to be ruthless, cite specific text, and not invent attacks.

Let me go through the document carefully.

**Attack 1: Temporal-state mismatch on S5 and S10**
The doc says S5 and S10 are "derived from B1" and "derived from B3" respectively, and tags them as "state" invariants. But S5 says: "if tally_result.is_some() at state σ, then any successor state σ' has tally_result = σ.tally_result". This quantifies over successor states σ', which is inherently temporal (it's about the future). A state-only predicate cannot express "any successor state" without a temporal operator. Similarly S10: "if tally_result.is_some(), then env.block.time ≥ end_at" — this is actually a state predicate (at the current state, if result is some, then current time >= end_at), but wait, B3 is the temporal causal version. S10 says "resolution implies past end_at" — at the current state, if resolved, then current time >= end_at. That's actually a state invariant. But S5 quantifies over successors. The doc explicitly says S5 is "Derived from B1 (temporal causal version)". If it's derived from a temporal property, but tagged as state, and it quantifies over successors, that's a mismatch. Actually, in TLA-style, state invariants are predicates on a single state. "any successor state" requires action or temporal logic. So S5 is mis-tagged as state.

Wait, the doc says: "S5 (derived) terminality of resolution (state-shape) — if tally_result.is_some() at state σ, then any successor state σ' has tally_result = σ.tally_result." This is absolutely temporal. It's not evaluable on a single state. The doc claims it's "Pointwise evaluable" for S6-S9 but not for S5. It says "Derived from B1". So this is a clear temporal_state_mismatch.

Similarly, S10: "if tally_result.is_some(), then env.block.time ≥ end_at". This is evaluable on a single state (current time >= end_at). So S10 is correctly state. But S5 is not.

Actually, looking at the table: S5 is listed under "Structural invariants" with no explicit "state" tag in the column (the table doesn't have a tag column for structural), but the text says "Properties evaluable on contract state at any reachable moment. No quantification over operations or time." Then S5 explicitly quantifies over successor states σ'. That's a contradiction.

Category: temporal_state_mismatch or contradiction? The category list includes temporal-state mismatch: "the spec encodes a temporal property as a state-only invariant (or vice versa)." S5 is encoded as a structural/state invariant but is actually temporal.

Severity: serious or critical? It misleads downstream spec writers into thinking they can discharge S5 as a state predicate when it requires temporal logic. That could lead to a proof architecture error. I'll call it serious.

**Attack 2: B6 temporal formulation is still broken / ambiguous**
B6 is re-tagged as temporal. Statement: "always (next.ballots ≠ ballots → ∃ tx, tx.kind = SubmitBallot ∧ tx.msg.sender ∈ next.ballots ∧ next.ballots[tx.msg.sender] = tx.encrypted_preferences)"

Wait, the quantifier is ∃ tx. What is the domain of tx? Is it a transaction in the current block? In CosmWasm, a single block can contain multiple messages in a single transaction, or multiple transactions. The spec says "∃ tx" but doesn't specify tx is in the *same* block/transaction that caused the state transition. In a blockchain, multiple txs can be in a block. If next.ballots ≠ ballots, there must exist a tx in the block. But the invariant says ∃ tx ... — if there are multiple submit_ballot txs in one block, then next.ballots would be the result of applying all of them sequentially. The existential quantifier over a single tx is insufficient: the state change could be caused by *two* SubmitBallot txs in the same block. The invariant as written would require a single tx that accounts for the entire difference, which might not exist (if two different candidates submit ballots in the same block, no single tx accounts for both writes).

Actually, let's parse it carefully: `next.ballots ≠ ballots → ∃ tx, tx.kind = SubmitBallot ∧ tx.msg.sender ∈ next.ballots ∧ next.ballots[tx.msg.sender] = tx.encrypted_preferences`. This says: if the ballot map changes, there exists a transaction such that... the sender is in the next ballots and the next ballots at that sender equal the tx's preferences. But if two ballots are submitted in the same block by A and B, then next.ballots differs from ballots by two keys. The existential quantifier only asserts there exists *at least one* tx. That part is fine — there does exist at least one tx (in fact two). But does it capture that *every* change is attributable to a tx? No, it doesn't say ∀ k ∈ next.ballots \ ballots.keys, ∃ tx ... It only says ∃ tx ... So it's actually too weak: it allows a change where some keys are written by non-SubmitBallot means, as long as at least one SubmitBallot tx exists. Wait, but the antecedent is just next.ballots ≠ ballots. If a malicious handler writes to ballots, but a SubmitBallot tx also happens in the same block, the existential is satisfied. So B6 is trivially weak? Let's check.

Actually, in CosmWasm, state transitions are per-transaction (or per-message). The `next` state in a temporal logic over blocks might mean the state at the next block. If multiple txs are in a block, the `next` state reflects all of them. B6 as written only requires that *some* SubmitBallot tx exists in that block; it does not require that *all* changes to ballots are from SubmitBallot txs. So an attacker could have a separate handler that mutates ballots, as long as a SubmitBallot tx is also included in the same block, and B6 would still be satisfied.

But wait, the doc says "every change to the ballot store is attributable to a submit_ballot tx". The formal statement does NOT say that. It says "if next differs from current, then there exists a tx...". This is a classic logic error: ∃ does not give attribution for all changes.

Category: Under-specification? Or Triviality? It's under-specification: it intends to say every write is by a SubmitBallot tx, but actually only asserts existence of at least one such tx. This is a weakness.

Severity: serious. It could allow a spec where another handler writes ballots.

Suggested defense: Change to ∀ k, if next.ballots[k] ≠ ballots[k] then ∃ tx ... or better, require that the only handler that writes ballots is Block 3.

Wait, but B6 is meant to be the temporal claim Block 3 enforces. If Block 3 is the only block that writes to ballots, then B6 should express that. The current formulation is insufficient.

**Attack 3: B9 decomposition is actually a disjunction-vs-decomposition collapse or impossibility-hypothesis vacuity**
B9 says: `Pr[B8 violated ...] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)`

The doc says this is a 4-summand bound inherited from Quartz's `cross_component_session_bind_negl`. But Section 6.2 says the Quartz substrate is retracted: "Quartz's `cross_component_session_bind_negl` ... was retracted as content-free ... proofs go through via negligible_of_le + negligible_add ... and reduce to negligible f → negligible f tautologies".

So B9 inherits a retracted substrate. But more importantly, B9 is stated as a single inequality with a sum of four terms. However, the doc notes that the Quartz content is missing. But is there a disjunction-vs-decomposition issue? The four summands correspond to different failure modes: TDX quote validity, Groth16 knowledge soundness, circuit equivalence, and commit hash collision resistance. These are independent assumptions. The bound is a sum. The category `disjunction-vs-decomposition` says: "a hardness assumption that is genuinely disjunctive ... is stated as a single monolithic hypothesis." Here B9 is explicitly a sum of four terms, which looks like decomposition. But wait — the attack category is about a hypothesis stated as a single negligibility bound when the underlying advantage decomposes into a sum. B9 does decompose into a sum. So maybe it's fine? But the prompt says "Examine B9's 4-summand list — did anything load-bearing get dropped?"

Let's check what was dropped: `Adv_KMS_leakage` and `Adv_image_registration`. The doc says KMS leakage is a confidentiality summand, not integrity. But B8 is about attestation binding (integrity). However, if KMS leaks the private key, an adversary can decrypt ballots (confidentiality) but the doc claims integrity remains intact. Is that true? If the adversary has the enclave private key, they can decrypt ballots, but can they forge attestations? The attestation is about the enclave's code identity, not about the private key. So KMS leakage alone doesn't let you forge an attestation. So dropping it from B9 (which bounds B8 violation) seems correct.

But what about `Adv_image_registration`? The doc says it's operational, not cryptographic. But B8 clause (d) requires "attested enclave identity matches the registered verified-rcv enclave image". If the registration is maliciously substituted (failure mode 4.3b), then B8 is violated: the attestation verifies, but the registered image is wrong. Does B9 cover this? B9's summands are all cryptographic advantages parameterized by n. Image registration substitution is an operational/governance failure. B9 explicitly excludes it. But B8 is violated in this case. So B9 does NOT bound the probability of B8 violation in the operational substitution scenario. The doc admits this in Section 6.1: "Enclave-image registration integrity ... is an operational assumption ... Failure mode: 4.3b". So B9 is only a partial bound: it bounds cryptographic failures, but not operational ones. The doc says B9 is the "negligibility-budget decomposition for B8". But B8 can be violated by 4.3b (vkey maliciously substituted), which is outside B9's budget. So B9 is not actually a complete decomposition of B8's violation probability. This is a disjunction-vs-decomposition collapse: the true failure modes of B8 include both cryptographic breaks and operational substitution. B9 presents a 4-summand bound that omits the operational disjunct, implying the sum covers the full probability. But the text says "B9 is stated as a deterministic implication; the cryptographic content lives in B9's negligibility budget." The operational substitution is a separate trust boundary item. However, B9 is labeled as the bound for B8 violation. If B8 can be violated non-negligibly by operational substitution, then B9's bound is false (or at least misleading) because the total probability is not bounded by the sum of those four negligible functions.

Wait, the doc says: "Classical-Prop shadow note: B8 is stated as a deterministic implication; the cryptographic content lives in B9's negligibility budget. Consumer-facing claims in Section 6.6 ('Output contract') that read B8 as a categorical implication should be understood as classical-Prop shadows of the probabilistic B9 form."

But if operational substitution is possible, B8 is not probabilistically bounded by B9 alone. The doc acknowledges this in 6.1 and 4.3b. So B9 is an incomplete decomposition. The category `disjunction-vs-decomposition` fits: the hardness assumption is genuinely disjunctive (cryptographic break OR operational substitution), but B9 states only the cryptographic sum. The "fix is to take the decomposed form as the hypothesis and prove the union bound." Here, the operational substitution is not a negligible function; it's a trust boundary. But the spec should be honest that B9 does not cover all B8 violations.

Actually, the category description says: "a hardness assumption that is genuinely disjunctive — e.g., 'Groth16 knowledge-soundness and circuit-vs-reference equivalence' — is stated as a single monolithic hypothesis. The proof passes; the trust claim is misrepresented as one assumption when honesty requires two." Here, B9 is at least a sum of four, so it's decomposed. But the operational substitution is a fifth disjunct that's omitted. So maybe the attack is that B9's 4-summand list dropped a load-bearing summand: `Adv_image_registration` was dropped but should have been retained as a non-negligible operational risk term, or B9 should not claim to bound B8 violation probability without it.

Alternatively, maybe the attack is that B9 is vacuous because the Quartz substrate is retracted. Let's examine that.

**Attack 4: Impossibility-hypothesis vacuity on B9 / B8**
Section 6.2 says: "Quartz's `cross_component_session_bind_negl` ... was retracted as content-free ... proofs go through via `negligible_of_le` + `negligible_add` ... and reduce to `negligible f → negligible f` tautologies under adversarial instantiation. The lifts have the shape of a security reduction but not the content."

B9 inherits this. The doc says: "The verified-rcv intent doc retains B8/B9 with this status declaration rather than dropping the cross-project composition."

But B9 states: `Pr[B8 violated ...] ≤ sum of four negligible functions`. If the Quartz theorems that define `Adv_tdxVerifier_sound`, etc., are content-free (no defining equations, just function symbols), then the inequality is not a meaningful bound. It's a schema. But more importantly, if the hypotheses in the Quartz theorems are impossible (e.g., `tdxVerifier` record is classically-over-strong), then any theorem proven from them is vacuously true or unsound to assume.

The prompt specifically asks to examine "B8 clause (c), Quartz `commitHashE` inheritance, and the Section 6.2 retracted-substrate status block" for impossibility-hypothesis vacuity.

Looking at Section 6.2 inheritance table:
- `commitHash` collision-resistance on `UserDataCommit → UserData` is tagged as `(d) pigeonhole-impossible (lift restates as concrete-hash collision-resistance)`.

The category `impossibility-hypothesis vacuity` says: "a hypothesis in the spec is provably false in the surrounding category, making the theorem vacuously true." The example given: "for any H : A ↪ B, if H has a collision then ..." — embedding makes antecedent unsatisfiable.

In the inheritance table, `commitHashE` bundle is tagged `(d) pigeonhole-impossible`. The doc says "lift restates as concrete-hash collision-resistance". But if the upstream Quartz spec models `UserData` as a `Fintype` (finite type) and the commit hash as a function into a smaller finite type, then collision existence is a pigeonhole certainty, not a hypothesis. If the theorem assumes collision resistance of a hash into a smaller codomain, that's impossible. The doc notes this: `(d) pigeonhole-impossible`. So the hypothesis that `commitHashE` is collision-resistant is provably false if `UserData` is larger than the hash output space (which it is, since SHA-256 is 32 bytes but `UserData` is 64 bytes per B8 clause (c) note). Wait, B8 clause (c) says: "32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot". So the hash output is 32 bytes, but `UserData` is 64 bytes. The commit function maps some larger domain to 64-byte `UserData`, but the hash is SHA-256 (32 bytes) embedded in 64 bytes. Collision resistance of SHA-256 is a standard assumption. But the Quartz `commitHashE` bundle might be pigeonhole-impossible because of the `Fintype` carrier refinement. The doc says the lift restates it as concrete-hash collision-resistance. But if the Lean proof still carries the `commitHashE` axiom, and that axiom is pigeonhole-impossible (i.e., false in the standard model because the type is finite and the function is not injective), then any theorem assuming it is vacuously true. The honest restatement is the negligible-collision-advantage of the concrete hash.

So the attack: B9's 4th summand `Adv_commitTally_CR` is supposed to capture SHA-256 collision resistance, but it inherits from Quartz's `commitHashE` bundle which is tagged `pigeonhole-impossible`. If the downstream Lean spec uses the `commitHashE` axiom directly (or a theorem that assumes it), the theorem is vacuous because the axiom is false. The spec should instead assume collision-resistance of SHA-256 as a concrete cryptographic assumption, not inherit the impossible bundle.

Category: impossibility-hypothesis vacuity. Severity: critical if it means the proof is meaningless.

**Attack 5: Composition failure on Quartz inheritance**
Section 6.2 claims inheritance of ECIES roundtrip soundness as a theorem: "ECIES encrypt/decrypt roundtrip is sound — theorem (Round 1 demoted from axiom) — None needed — proven".

But the withdrawn row says: "an earlier draft listed `DstackKeyManager` as an inherited Quartz Lean artifact. `grep ...` returns zero hits — Quartz's Lean spec tree does not currently model dstack KMS."

The composition failure is: the doc claims to inherit `Specs.Quartz.Crypto.Ecies.roundtrip` as a proven theorem. But if the ECIES spec in Quartz is proven, does it actually apply to the verified-rcv usage? The doc doesn't specify the exact parameters (curve, key derivation function, etc.). More importantly, the doc withdrew the `DstackKeyManager` claim because it wasn't in Quartz. But it still claims ECIES roundtrip is inherited. Is there a mismatch between the ECIES spec in Quartz and the ECIES used in verified-rcv? The doc says "Depends on Quartz's `Specs.Quartz.Crypto.Ecies.roundtrip` for the decryption-correctness half (Section 6.2 inheritance row 3 — a theorem, not an axiom)." But if the Quartz theorem is about a generic ECIES construction, and verified-rcv uses a specific instantiation (e.g., secp256k1 vs secp256r1, specific KDF, specific AEAD), the composition might fail if the instantiation doesn't match the theorem's preconditions. The doc doesn't specify the ECIES parameters, so this is a composition gap.

But maybe stronger: Section 6.2 says B8/B9 inherit from Quartz's `tdxVerifier` and `groth16Verifier` bundles, but the substrate is retracted. The doc says "The verified-rcv intent doc retains B8/B9 with this status declaration rather than dropping the cross-project composition." This is a composition failure: the cross-project inheritance claims don't compose because the upstream content is retracted. The doc acknowledges it but still labels B9 as inheriting from Quartz. The methodology should catch that the composition theorem is broken.

Category: composition_failure. Severity: serious.

**Attack 6: B10 arity / RHS mismatch (despite sanity pass)**
Let's check B10: `always (next.tally_result.is_some() ∧ tally_result.is_none() → next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey)))`

Section 2.5 defines `Tally_spec(raw_ballots, candidates, enclave_privkey) → TallyResult`. So B10 calls it with three arguments: `ballots@end_at` (which is `Map<Addr, Vec<u8>>`), `candidates` (`Vec<Addr>`), and `enclave_privkey` (`PrivKey`). That matches the signature.

But wait: `Tally_spec` is defined as returning a `TallyResult`. However, `Tally_spec` internally uses `enclave_privkey` to decrypt. But `enclave_privkey` is NOT stored on-chain. The contract state does not contain `enclave_privkey`; it contains `enclave_pubkey`. So B10 references `enclave_privkey` as if it's available in the chain state. But the chain state only has the public key. The intent doc says in Block E1: "enclave_privkey: PrivKey — retrieved from DstackKeyManager after attesting". So `enclave_privkey` is not a contract state variable.

B10 is a chain-side temporal invariant? It says `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey))`. But `enclave_privkey` is not in the contract state. How can a chain-side invariant reference a value not present in the state? It could be existentially quantified: there exists a privkey such that ... But B10 doesn't quantify it. It treats it as a free variable. In a chain-side spec, this is undefined unless the privkey is somehow observable. The doc says B10 is witnessed off-chain. But B10 is listed in Section 3.2 "Behavioral invariants" alongside chain-side invariants B1-B9. If B10 is meant to be a chain-side property, referencing `enclave_privkey` is a state mismatch. If it's meant to be an off-chain property, it shouldn't be in the same "behavioral invariants" table that governs chain state.

Actually, the doc says: "B10 is the only temporal invariant without a Section-8 chain-side witness scenario, by construction (B10 quantifies over enclave-image behavior, not chain state)." So the doc acknowledges it's different. But it still places it in the chain-side invariants table. The RHS references `enclave_privkey`, which is not a chain state variable. This is an ambiguity/under-specification: where does `enclave_privkey` come from in the evaluation of B10? The chain doesn't know it. The invariant is not evaluable on chain state.

Category: Under-specification or Refinement mismatch. The spec references a variable not in the state, making it impossible to evaluate. Severity: serious.

Suggested defense: Existentially quantify `enclave_privkey` and add a clause linking it to the on-chain `enclave_pubkey` (e.g., `∃ privkey, KeyPair(pubkey, privkey) ∧ ...`). Or move B10 out of the chain-side invariants.

**Attack 7: Block E1 completeness — missing failure mode for partial decryption**
Block E1 says: "Stage 1 — decrypt_and_validate: for each (addr, ciphertext) ∈ ballots, ECIES-decrypt under enclave_privkey, parse as Vec<Addr>, validate as a permutation of candidates."

What if ECIES-decrypt succeeds but produces a plaintext that is not a valid UTF-8 string or not a valid Addr encoding? The doc says "parse as Vec<Addr>" — but what parsing format is used? JSON? Borsh? Protobuf? The intent doc doesn't specify the serialization format for the decrypted plaintext. This is an under-specification: two different enclave implementations could use different serialization formats and produce different `valid_ballots` from the same ciphertexts, leading to different tallies. The spec says "parse as Vec<Addr>" but doesn't define the encoding. This is a coverage gap / under-specification.

Category: Under-specification. Severity: serious.

**Attack 8: Section 2.1 arithmetic check**
Let's recompute the example carefully.

Ballots:
- A: A > B > C > D > E
- B: A > B > C > D > E
- C: C > A > B > D > E
- D: D > C > A > B > E
- E: E > C > A > B > D

Round 1 first places:
- A: 2 (A and B)
- B: 0
- C: 1
- D: 1
- E: 1

Lowest is B (0). Eliminate B. Redistribute B's ballot. B's ballot is A > B > C > D > E. After dropping B, top surviving is A. A already counts B's first-place vote? Wait, B's ballot was already counted for A in Round 1 (since B ranked A first). So after eliminating B, B's ballot stays with A. So Round 2 counts:
- A: 2 (A's own + B's)
- C: 1
- D: 1
- E: 1

Lowest is C, D, E tied at 1. Batch eliminate all three. Redistribute:
- C's ballot: C > A > B > D > E. After dropping C, B, D, E (all eliminated), surviving is A. So goes to A.
- D's ballot: D > C > A > B > E. After dropping D, C, B, E, surviving is A. Goes to A.
- E's ballot: E > C > A > B > D. After dropping E, C, B, D, surviving is A. Goes to A.

Round 3:
- A: 5

A wins. This matches the doc. The arithmetic is correct.

But wait: the doc says in Round 1: "B has 0 first-place votes (unique lowest). Eliminate B. Redistribute B's ballot: B's first surviving choice is A (since A > B > … after dropping B is A > C > D > E), which already counts B's first-place vote — no movement." This is correct.

Round 2: "C, D, E tied for lowest at 1 each. Batch-eliminate all three." Correct.

Redistribute C's ballot: "C's ballot C > A > B > D > E after dropping C/B/D/E → A → A absorbs." Wait, after dropping C, B, D, E, the remaining candidate is A. Yes.

Redistribute D's ballot: "D's ballot D > C > A > B > E after dropping D/C/B/E → A → A absorbs." Yes.

Redistribute E's ballot: "E's ballot E > C > A > B > D after dropping E/C/B/D → A → A absorbs." Yes.

Round 3: A holds 5. Correct.

So arithmetic is right.

**Attack 9: Section 4.9 soundness — deterministic malformed tally deadlock**
Section 4.9 says: "The enclave is deterministic over the ballot set; the ballot set is frozen at end_at (B2). Therefore re-running the enclave produces the same malformed tally and publish_result rejects again. Infinite rejection loop."

Is this sound? The doc says "deterministic enclave with frozen ballots will re-produce the same InvalidTally rejection on retry; recovery requires deploying a corrected enclave image and re-registering."

But what if the enclave is not deterministic? The spec assumes determinism but doesn't state it as an invariant. Block E1 says "deterministic enclave" in the failure mode bullet, but is determinism guaranteed? The enclave runs software; if it has a bug that depends on timing, memory layout, or external entropy, it might produce different malformed tallies on different runs. The doc assumes determinism without stating it as a requirement. If the enclave is non-deterministic, it might eventually produce a well-formed tally by chance (if the bug is e.g., a race condition). The deadlock argument fails.

Category: Under-specification (missing determinism requirement) or Edge case. Severity: serious.

Suggested defense: Add an explicit requirement that the enclave image is deterministic (no external entropy, no timing dependencies) or state that the deadlock argument holds only under determinism.

**Attack 10: B8 clause (c) — SHA-256 collision resistance hypothesis**
B8 clause (c): attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body)).

B9 includes `Adv_commitTally_CR` for this. But what is `canonical_serialization`? The doc doesn't define it. Different canonical serialization schemes (e.g., JSON with sorted keys vs protobuf vs Borsh) produce different byte strings. If the enclave and the chain use different canonical serialization schemes, the hash won't match. The intent doc says "canonical_serialization" but doesn't specify the format. This is an under-specification that could lead to a composition failure between enclave and chain.

Category: Under-specification. Severity: serious.

**Attack 11: Block 6 well-formedness check — missing permutation validation**
Block 6 Requires: "tally is well-formed: winners ⊆ candidates, 1 ≤ len(winners) ≤ len(candidates), per-round counts internally consistent (each per_round_counts[i] has all entries summing to ballots_tallied), elimination sequence is monotone, conservation: ballots_tallied + ballots_dropped + len(non_voters) = len(candidates), set relations..."

Wait, the conservation equation is wrong. Let's check: `ballots_tallied` is the number of ballots successfully decrypted and validated. `ballots_dropped` is the number of ballots that failed. `non_voters` are candidates who did not submit a ballot. The total number of candidates is `len(candidates)`. But the equation says: `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)`. This is dimensionally wrong: `ballots_tallied + ballots_dropped` is the total number of ballot submissions (including malformed ones). `non_voters` is the number of candidates who didn't submit. The sum of ballot submissions + non-voters does not equal the number of candidates. It equals the number of candidates who submitted (valid or invalid) plus the number who didn't submit. But `ballots_tallied + ballots_dropped` counts the total number of entries in `raw_ballots` (i.e., number of voters who submitted). `len(non_voters)` counts candidates not in `raw_ballots.keys`. So `ballots_tallied + ballots_dropped = |raw_ballots.keys|`. And `|raw_ballots.keys| + len(non_voters) = len(candidates)`? Yes, because `non_voters = candidates \ raw_ballots.keys`. So the equation is actually `|raw_ballots.keys| + |candidates \ raw_ballots.keys| = |candidates|`. This is true. But the equation uses `ballots_tallied + ballots_dropped` to mean `|raw_ballots.keys|`. Is that guaranteed? Yes, by definition in Stage 1: every entry in `raw_ballots` is either successfully decrypted (counted in `ballots_tallied`) or dropped (counted in `ballots_dropped`). So `ballots_tallied + ballots_dropped = len(raw_ballots)`. And `non_voters = candidates \ raw_ballots.keys`. So the equation holds.

Wait, but is `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)` correct? Let's check with Section 2.2 boundary case: "Some candidates abstain (0 < len(ballots) < len(candidates))". Here `ballots` means `raw_ballots`. Suppose 2 candidates out of 5 submit. `raw_ballots.keys` has size 2. `ballots_tallied` could be 2, `ballots_dropped` 0. `non_voters` has size 3. Sum = 2 + 0 + 3 = 5 = len(candidates). Yes.

But what if a candidate submits multiple times? `raw_ballots` is a Map, so last-write-wins. Each candidate can have at most one entry. So `|raw_ballots.keys|` ≤ `len(candidates)`. So the equation holds.

However, the well-formedness check in Block 6 does NOT check that `dropped_voters` and `non_voters` are disjoint from `winners` or that `winners` are actually surviving candidates from the final round. It checks `winners ⊆ candidates` but not that winners are consistent with `per_round_counts` and `eliminated_by_round`. For example, a malformed tally could have `winners = [A]` but `per_round_counts` final round shows `{A: 2, B: 2}` (tie) and `eliminated_by_round` empty. The well-formedness predicate as listed doesn't catch that `winners` must equal the set of candidates in the final round of `per_round_counts` (or the co-winners from the final round). The doc says "winner well-formedness" checks subset and length, but not consistency with per-round counts.

Wait, the doc says in Block 6: "tally is well-formed: winners ⊆ candidates, 1 ≤ len(winners) ≤ len(candidates); per-round counts internally consistent (each per_round_counts[i] has all entries summing to ballots_tallied); elimination sequence is monotone; conservation; set relations". It does NOT say that the final round's candidates equal `winners`, or that `winners` is exactly the set of candidates in the last entry of `per_round_counts`. This is a coverage gap: a tally could pass all listed checks but have `winners` that don't match the final round.

Actually, is that covered by "elimination sequence is monotone"? No, that only says no reappearance. It doesn't say the winners are the survivors.

Category: Under-specification (well-formedness predicate too weak). Severity: serious.

Suggested defense: Add a well-formedness clause: `winners` must equal the candidate set of the final round in `per_round_counts`, or if the final round is absent (e.g., immediate termination), `winners` must match the base case.

Wait, in the boundary case "All candidates abstain", `per_round_counts = [{c: 0 for c in candidates}]`, `eliminated_by_round = []`, `winners = candidates`. Here the final round has all candidates, and winners = all candidates. So the clause should be: `winners = per_round_counts[last].keys()`.

But what about the single candidate case? `per_round_counts = [{that_candidate: ballots_tallied}]`, `winners = [that_candidate]`. OK.

What about two candidates terminal tie? `per_round_counts = [{A:1, B:1}]`, `winners = [A, B]`. OK.

So yes, `winners` should equal the key set of the last round's count map. The spec omits this.

**Attack 12: B8 clause (c) — 32-byte digest in 64-byte UserData slot**
B8 clause (c): "32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot; upper half reserved for domain-separation tag (implementation detail)."

The doc says the hash is SHA-256. But it also says the upper half is reserved for domain-separation tag. If the enclave includes a domain-separation tag in the upper half, and the chain checks that `attested user_data = SHA-256(...)`, but the attested user_data is 64 bytes (32-byte hash + 32-byte tag), then the equality doesn't hold: the chain is comparing a 64-byte value to a 32-byte hash. The doc says "attested user_data = SHA-256(canonical_serialization(...))". But if the attested user_data includes the domain-separation tag, then the equality is false. The chain would need to check that the lower 32 bytes equal the hash. The doc says "32-byte digest embedded in the lower half... upper half reserved". But the formal statement in B8 says "attested user_data = SHA-256(...)". This is a contradiction: the attested user_data is 64 bytes, SHA-256 is 32 bytes.

Category: Contradiction. Severity: critical.

Wait, is it critical? It means B8 as written can never be satisfied if the enclave sets the upper half to anything non-zero. The chain would reject all valid attestations. Or if the chain actually checks only the lower half, then B8 is mis-stated. Either way, the spec is wrong.

Let's check the text: "attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body)) (B8 clause (c)). Clauses (a) and (b) are inherited... clauses (c) and (d) are verified-rcv-specific deployment bindings. Note on (c): the hash is explicitly SHA-256... 32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot; upper half reserved for domain-separation tag (implementation detail)."

So B8 clause (c) as written is `att