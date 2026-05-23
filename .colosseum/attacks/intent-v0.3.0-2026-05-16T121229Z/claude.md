# claude (claude-opus-4-7 via Agent subagent) — intent v0.3.0 adversarial pass

- **Pass**: 3rd adversarial (vs v0.3.0)
- **Date**: 2026-05-16
- **Channel**: Agent subagent (file access)

---

## Attacks

### 1. SemVer header mis-classifies v0.3.0 as MAJOR [severity: serious]

- **Affected**: Top header / Revision history
- **Category**: contradiction (internal classification rule vs applied label)
- **What's wrong**: The header defines MAJOR as "invariant removed / weakened / trust boundary widened / behavior previously specified becomes unspecified." But the v0.3.0 entry lists "B6 ∀-per-key, B10 cross-layer, B9 conditioned on `image_registration_honest`, S5 restated as set-once-write-discipline … canonical_serialization pinned to Borsh." Every one of these *strengthens* or *clarifies* — B6 binds an attribution that was previously satisfiable by any historical tx (formalization tightening), B9's antecedent adds a precondition (which *narrows* the unconditional claim — but the doc itself describes the prior bound as "unconditionally *false*"), and Borsh-pinning is a strict refinement of "canonical_serialization." None of these matches the MAJOR rubric — they match the MINOR or PATCH rubrics. The one defensible candidate for MAJOR — "B9 conditioned ⇒ B9 weakened" — is explicitly described by the doc as fixing a previously *false* claim, i.e., not weakening a real claim but acknowledging a vacuous/false one. By the doc's own rules, this should be MINOR ("trust boundary narrowed / new precondition surfaced") or a PATCH train. Either the rubric is wrong, or the label is wrong. This is the second consecutive MAJOR (v0.2.0 → v0.3.0) where the doc's classification trips its own rule, and that is exactly the failure mode Ask N exists to prevent.
- **Cite**:
  > - MAJOR (0.x.0): invariant removed / weakened / trust boundary widened / behavior previously specified becomes unspecified
  > - MINOR (0.0.x where x adds): new invariant / trust boundary narrowed / new failure mode / new scenario
  > …
  > - v0.3.0 — 2026-05-15 MAJOR: revision against Round 3a re-adversarial fan-out (6 multi-voice findings … — B6 ∀-per-key, B10 cross-layer, B9 conditioned on `image_registration_honest`, S5 restated as set-once-write-discipline, Section 6.2 de-retraction, canonical_serialization pinned to Borsh).
- **Fix recommendation**: Either reclassify v0.3.0 as MINOR (preserving the rubric) and explain why each item is a strengthening/clarification rather than a weakening; or expand the MAJOR rubric to include "previously-stated invariant is rewritten because the prior statement was provably false/vacuous" — and call that out explicitly so future readers can audit the same situation again. Without one of those fixes, the SemVer header is decorative.

### 2. `B10_lean` uses an undefined function symbol `EnclaveImage` [severity: critical]

- **Affected**: Section 3.2 B10_lean row; Section 8.7 step 3
- **Category**: under-specification / triviality risk
- **What's wrong**: B10_lean asserts `EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(...)`. Nowhere in the intent doc is `EnclaveImage` typed, defined, or sourced. Section 8.7 step 3 calls it "the *Lean-extracted* model of the enclave binary," but says nothing about *which* extraction tool, what semantics the model assigns, or what side-effects/non-termination/panic behavior the function symbol absorbs. As stated, `B10_lean` is *whatever* relation a future Lean spec author defines `EnclaveImage` to be — including the trivial `EnclaveImage := Tally_spec`, which discharges B10_lean by `rfl`. The methodology asked the intent to fix the prior "step 3 conflated B10_lean with B10" problem (theme 10). The fix introduces a *new* unconstrained symbol that re-opens the same hole one layer down. Image-identity-binding (step 4) is described as "extraction-soundness + build-reproducibility" but it only binds the *MRTD/RTMR* to the binary — not the binary to the Lean function symbol. There is no link between "the binary whose MRTD is registered" and "the `EnclaveImage` symbol B10_lean is about" that this doc establishes. So the whole B10 chain is one defined symbol short of being tight.
- **Cite**:
  > B10_lean | Lean-internal image-IO obligation | **off-chain** | `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)` — the enclave image's externally observable input-output relation equals `Tally_spec`.
  > …
  > 3. **Lean-internal composition — `B10_lean`**: by composition of the Stage-1 lemma and Stage-2 theorem, the enclave image's externally observable input-output relation satisfies `EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`. This is a Lean theorem about the *Lean-extracted* model of the enclave binary, parameterised over `privkey`.
- **Fix recommendation**: Specify the extraction substrate (e.g., "Aeneas-extracted Rust model" or "hand-written Lean translation of the Rust source with stated extraction discipline"), and make explicit that `EnclaveImage` is a *fixed* term in the Lean spec sourced from that extraction — not a name the proof author may instantiate. Add a sentence that ties image-identity-binding to `EnclaveImage` (e.g., "the registered MRTD/RTMR is the build hash of the Rust crate from which `EnclaveImage` was extracted"). Without that link, step 4 and step 3 are talking about two different `EnclaveImage`s.

### 3. B6's transaction-trace model is silent on what `next.ballots[k] ≠ ballots[k]` means for keys not previously present [severity: serious]

- **Affected**: Section 3.2 B6, Section 2.5 transaction trace model paragraph
- **Category**: edge case / ambiguity
- **What's wrong**: B6 reads `next.ballots[k] ≠ ballots[k]`. The contract storage `ballots: Map<Addr, Vec<u8>>` is a partial map — keys are "absent" before first write. The intent doc does not specify the comparison semantics for absent keys: is `ballots[k]` = `None`/`⊥`/`empty bytes` for absent `k`? The Section 2.5 schema comment "absent = no vote" implies `None`, but the predicate `next.ballots[k] ≠ ballots[k]` is written as a value-level comparison, not an option-level comparison. Two non-equivalent readings:
  - **(A)** `ballots[k] := None` for absent k; `next.ballots[k] := Some(bytes)` after submit; the inequality fires on first write. B6 is sound.
  - **(B)** `ballots[k]` is undefined (panics) for absent k; the predicate is ill-formed at first write. B6 is vacuous or non-evaluable on the very transition it's meant to attribute.
  The trace model paragraph specifies `tx.msg.sender: Addr`, `tx.kind`, payload structure — but does *not* specify the lift to handle absent keys. This is the exact category of edge-case silence that B6's prior ∃-form was found broken on; rewriting to ∀-per-key without nailing the absent-key reading reopens the same hole.
- **Cite**:
  > B6 | ballot writer is the ballot voter | **temporal** | `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`
  > …
  > `ballots: Map<Addr, Vec<u8>>` — candidate → encrypted_preferences; absent = no vote
- **Fix recommendation**: Either reformulate B6 as a quantification over `ballots.keys ∪ next.ballots.keys` with an `Option<Vec<u8>>` lift, or add a sentence in the trace-model paragraph: "for absent k, `ballots[k]` is treated as `⊥`/`None`; B6's inequality holds whenever k transitions from absent-to-present, present-to-different-value, or (vacuously, since map entries are not deleted) present-to-absent."

### 4. `Tally_spec` / `IRV_spec` split is honest at arity but loses semantic determinism [severity: serious]

- **Affected**: Section 2.5 (Stage 1 + composition); Section 3.2 B10 / B10_lean
- **Category**: composition_failure / under-specification
- **What's wrong**: The split is presented as a tightening fix for the prior arity mismatch (mistral A2), but Stage 1's specification leaks non-determinism into `Tally_spec` that B10 then quantifies over equality with. Specifically: Stage 1 says `dropped_voters` is "ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline)" — but the *input* `raw_ballots: Map<Addr, Vec<u8>>` does *not* have a declared iteration discipline anywhere in this doc. The CosmWasm `Map` iteration is over the *storage key prefix* and varies across SDK / chain configurations. So `Tally_spec(raw_ballots, candidates, privkey)` is **not a deterministic function of its logical inputs** — it depends on storage layout. Two implementations of the enclave with different `Map`-iteration discipline produce different `dropped_voters` orderings, both compliant with the Section 2.5 spec. B8 clause (c)'s SHA-256 over `Borsh(tally_body)` then produces different digests for the *same* logical input. B10 (and B10_lean) is `tally_result = Tally_spec(...)` — but `Tally_spec` is not a function unless the iteration order is pinned. The Borsh paragraph fixes the *encoding* but doesn't pin the *order of dropped_voters*. The TallyResult schema's order-discipline note pins **only candidate-declaration order**; it explicitly does not say "dropped_voters are emitted in some order derivable from candidates." Result: a real bit-level disagreement is admitted under the spec.
- **Cite**:
  > - `dropped_voters` ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline).
  > …
  > Ordering of entries within each map and within `Vec<Addr>` fields is **deterministic-by-candidate-declaration-order** (the order in `candidates` at instantiation); this matters for downstream Lean / Quint encoding to produce reproducible serialization.
- **Fix recommendation**: Either (i) pin `dropped_voters` to a deterministic order *derivable from the candidates list* (e.g., "in `candidates`-order, restricted to those that appear in `raw_ballots.keys` and failed validation"), or (ii) make Stage 1 take a *list* of `(Addr, ciphertext)` pairs in a chain-deterministic order and define iteration over the list, declaring the chain's iteration as part of the public contract. Without one of these, `Tally_spec` is not a function and B10's equality is not even well-typed.

### 5. `image_registration_honest` is an undefined and unconditioned predicate [severity: critical]

- **Affected**: Section 3.2 B9; Section 6.1 last bullet
- **Category**: impossibility-hypothesis-vacuity (anti-form) / under-specification
- **What's wrong**: The B9 conditioning was supposed to carve out a real threat model by lifting failure mode 4.3b out of the negligibility budget. But `image_registration_honest` is not given a formal definition anywhere — Section 6.1's last bullet describes the *trust* in image-registration integrity, but the predicate `image_registration_honest` appears as a free symbol in B9's antecedent. Two failure modes:
  - **(A)** If `image_registration_honest` is *defined* as something close to "vkey + MRTD on-chain are honest at every state on the trajectory," B9's antecedent is a temporal predicate, not a pointwise one — but B9 is sitting in the temporal-invariant table and the inner Pr[...] is at "the next state," so the antecedent should also be evaluated then. The doc doesn't say.
  - **(B)** If `image_registration_honest` is operationally undefined, B9 reduces to: "if some out-of-band condition we haven't formalized holds, then the bound holds" — which is the same epistemic status as having no bound at all. The doc previously moved `Adv_image_registration` out of B9 *because* the operational fault had no cryptographic-advantage form; substituting an undefined predicate in the antecedent doesn't repair that — it relocates the unspecified content.
  The doc names the predicate, points at Section 6.1 in parens, and stops. B9 is the central probabilistic claim of the intent doc; the antecedent it pivots on must be defined to the same standard.
- **Cite**:
  > B9 | B8 negligibility-budget decomposition | **temporal** | `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))` — **4-summand bound, conditioned on the operational precondition** `image_registration_honest` (the chain-side registered verified-rcv vkey + MRTD have not been substituted post-registration; Section 6.1 trust boundary).
- **Fix recommendation**: Make `image_registration_honest` a defined predicate over chain state and time, e.g., `image_registration_honest(σ) ≡ chain_registered_vkey(σ) = canonical_verified_rcv_vkey ∧ chain_registered_mrtd(σ) = canonical_verified_rcv_mrtd`, and state its evaluation time: "evaluated at σ for the transition σ → σ′ in B9." Otherwise the conditioning is decorative.

### 6. Section 6.2 de-retraction status block is composition-fragile and possibly tautological [severity: serious]

- **Affected**: Section 6.2 `[2026-05-15 — de-retracted]`
- **Category**: composition_failure / preconditional over-strength
- **What's wrong**: The block claims the Quartz upstream "implemented the **def-tying refactor across all 8 protocol-layer `_negl` lifts** (cycles 6.4 through 6.11)" and concludes verified-rcv inherits B8/B9 "from a now-content-bearing substrate." Three problems:
  - **(i)** The text references cycles 6.4–6.11 and Quartz's *now-content-bearing* `cross_component_session_bind_negl` — but it provides no path-to-evidence other than "8 commits at `/Users/mvid/Development/reliq/quartz/.colosseum/changes/2026-05-14T*-cycle-6.{4..11}-*.md`". Critically, the block also notes "**Asks 6+7** were added during the cycle-6.4-through-6.11 work" and that Ask 6 says the *prior plan over-bundled 7 of 8 lifts*. So 7 of 8 of the very lifts named as the de-retraction substrate were over-bundled. The status block does not disclose whether `cross_component_session_bind_negl` is one of the 7 over-bundled lifts or the 1 properly-bundled lift. If it is in the over-bundled 7, the de-retraction is itself a flawed unit that Ask 6 will rework — i.e., the substrate is provisionally content-bearing pending another Quartz cycle. Verified-rcv inheriting from a substrate that the upstream's own follow-up ask flags as over-bundled is a known-fragile composition.
  - **(ii)** "*documented* form in the Quartz PR; *enforced* form is gated on the executable-layer decision" — Ask 7 (degenerate-zero-advantage cycle intent declaration) is documented-only. The doc says cycles 6.7 and 6.8 "produced lifts with `failAdv 𝒜 n = 0` identically." If `cross_component_session_bind_negl` consumes a 6.7/6.8 lift, the bound flowing into B9 includes a 0-advantage summand — which is mathematically free but methodologically suspect: "intentionally degenerate within scope" is not stated as ascertained for these particular cycles.
  - **(iii)** The Section 6.2 row 1 (`tdxVerifier`) carries the sub-tag "(d) classically-over-strong-single-negligibility (sound) + preconditional (complete)." The "preconditional (complete)" half implies *some* precondition for the completeness side — but verified-rcv's B9 uses `Adv_tdxVerifier_sound(n)` only, and the doc doesn't comment on whether the completeness precondition has surface here. If the verified-rcv enclave depends on the *completeness* side at any point (e.g., honest enclaves get valid attestations such that `publish_result` succeeds — the liveness face), B9 silently picks up a Quartz-side precondition that is nowhere visible in this intent.
- **Cite**:
  > **The retraction has been resolved upstream on 2026-05-14/15.** Quartz's Lean tree implemented the **def-tying refactor across all 8 protocol-layer `_negl` lifts** (cycles 6.4 through 6.11; 8 commits at `/Users/mvid/Development/reliq/quartz/.colosseum/changes/2026-05-14T*-cycle-6.{4..11}-*.md`).
  > …
  > **Known v0.3 caveats inherited from Quartz** (per the Quartz-agent feedback on the implementation):
  > - **Asks 6+7** were added during the cycle-6.4-through-6.11 work: (6) per-conjunct failure-mode analysis as bundle-count source (the prior plan over-bundled 7 of 8 lifts); (7) degenerate-zero-advantage cycles must declare intent (cycles 6.7 and 6.8 produced lifts with `failAdv 𝒜 n = 0` identically …
- **Fix recommendation**: Add a single line in 6.2 naming which of the 8 lifts `cross_component_session_bind_negl` *is* (cycle index) and whether it falls in the "1 properly-bundled" or "7 over-bundled" cohort; mark the verified-rcv compose-ledger entry as `inheritance: provisional-pending-Quartz-ask-6` if the latter. Separately, surface the preconditional-completeness clause of `tdxVerifier` as an explicit verified-rcv liveness-side assumption (or document why it is not consumed).

### 7. Section 8.7 step 6 hand-waves the existential instantiation [severity: serious]

- **Affected**: Section 8.7 step 6; Section 3.2 B10
- **Category**: triviality / under-specification of discharge
- **What's wrong**: Step 6 says: "from `B10_lean` (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived `privkey` (Section 6.3): the published `tally_result` equals `Tally_spec(ballots@end_at, candidates, privkey)` …" This is the *conclusion* of B10, restated, with the word "instantiated by" doing the work of an actual proof step. Critically, B10's existential is `∃ privkey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))`. To witness the existential you need a `privkey` value and a proof that *that specific privkey* was dstack-KMS-derived for this contract. Step 6 says it gets the privkey from Section 6.3 — but Section 6.3 is a **trust boundary**, not a Lean theorem; "dstack KMS honesty" is an *assumption*. So step 6 discharges B10's existential by trusting the assumption "dstack KMS gave us the right key." This is not a methodology failure per se — the trust is named — but step 6 is presented as a *composition* step, not as an existential witnessed by an additional trust claim. The result is that B10 silently depends on **`dstack_kms_derived` being decidable / inspectable in the discharge layer**, which the doc never establishes. If `dstack_kms_derived` is itself only an assumption, then B10 reduces to "B10_lean + image-binding + B8 + dstack-trust ⇒ B10" — a 4-link chain — but step 6 presents it as a 3-link chain.
- **Cite**:
  > 6. **Composition — B10**: from `B10_lean` (steps 1+2+3) + image-identity-binding (step 4) + B8 (step 5), with the existential quantifier in B10 instantiated by the dstack-KMS-derived `privkey` (Section 6.3): the published `tally_result` equals `Tally_spec(ballots@end_at, candidates, privkey)` for the dstack-KMS-derived `privkey`. This is the B10 statement from Section 3.2.
  > …
  > B10 | … | `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`
- **Fix recommendation**: Rename the section header's composition pattern from "`B10 ← B10_lean ∧ image-identity-binding ∧ B8`" to "`B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack-KMS-trust(6.3)`" — explicitly carrying the trust dependency as one of the conjuncts. Otherwise step 6's "instantiated by" is a hand-wave that hides a fourth link.

### 8. S5's set-once write-discipline is not pointwise-evaluable as claimed [severity: serious]

- **Affected**: Section 3.1 S5
- **Category**: temporal_state_mismatch (residual)
- **What's wrong**: S5 was reformulated to escape Section 3.1's "no quantification over operations or time" preamble. The new form is described as "Evaluable on a single state by inspecting the handler index, not by quantifying over successor states." But the predicate "no handler writes to `tally_result` after it has been set to `Some(_)`" is *itself* a temporal claim — "no handler … writes … after." Inspecting the handler index gives you the set of handlers, but you cannot evaluate "Block 6's Requires `tally_result.is_none()` gates the write" as a state-only predicate over the contract state σ — it requires inspecting the *handler's source*, which is a meta-property of the *contract code*, not σ. This is a different category of "pointwise" than what Section 3.1 promises: Section 3.1 says state invariants are properties of σ, not of the handler set. S5 has moved from quantifying over successor states to quantifying over the handler index — but the preamble doesn't admit handler-index quantification as "pointwise." The new form is still a temporal-claim shadow, just at a different level. If S5 stays in the "structural" table, the preamble needs an explicit "handler-set inspection is in scope" clause; if not, S5 should move to B-table as a temporal write-discipline.
- **Cite**:
  > Properties evaluable on contract state at any reachable moment. No quantification over operations or time.
  > …
  > S5 (derived) | terminality of resolution (state-shape) | **`tally_result: Option<TallyResult>` is set-once: no handler writes to `tally_result` after it has been set to `Some(_)`.** This is a *write-discipline* property of the contract's handler set (only Block 6 writes `tally_result`, and Block 6's Requires `tally_result.is_none()` gates the write). Evaluable on a single state by inspecting the handler index, not by quantifying over successor states.
- **Fix recommendation**: Either rename the section preamble to "Properties evaluable on contract state OR on the static handler set without trajectory quantification," or relocate S5 to Section 3.2 as a temporal write-discipline corollary of B1. The current placement contradicts the preamble.

### 9. Borsh pinning is incomplete — does not specify Addr / Nat encodings [severity: serious]

- **Affected**: Section 2.5 canonical serialization paragraph
- **Category**: under-specification
- **What's wrong**: The paragraph pins canonical_serialization to Borsh but defines only the structural composition: `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`, "field-by-field in declaration order," "candidate-declaration order" within Vec/Map. What it does *not* specify is the concrete Borsh encoding for `Addr` and `Nat`. Borsh has multiple defensible encodings for `Addr` (bech32 string, raw bytes, length-prefixed bytes — which one?) and for `Nat` (variable-length u64? u128? big-endian? little-endian?). CosmWasm's `Addr` is `String` under the hood; Borsh would encode it as a length-prefixed UTF-8 byte string, but **the doc never says this**. The same ambiguity that "canonical_serialization" left open is reproduced one layer down. Two compliant Borsh implementations can pick different `Addr`/`Nat` representations and B8(c)'s SHA-256 still disagrees. The patch was incomplete.
- **Cite**:
  > **Canonical serialization** … every appearance of `canonical_serialization(x)` in this doc … refers to **Borsh** serialization per the CosmWasm-native discipline. Concretely:
  > - `canonical_serialization(contract_addr ‖ tally_body)` denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`, where `‖` is byte concatenation.
  > - `tally_body` is the `TallyResult` struct above, serialized field-by-field in declaration order; `Vec<Addr>` and `Map<Addr, Nat>` entries are emitted in **candidate-declaration order** (matching the ordering discipline above), making `borsh_bytes(tally_body)` a deterministic function of the logical value.
- **Fix recommendation**: Add concrete pinning for `Addr` (e.g., "Borsh `String`: u32-LE length prefix + UTF-8 bytes of the bech32 representation") and `Nat` (e.g., "u128 little-endian"). Or reference an external Borsh schema document and pin its version. The current paragraph is one layer of indirection short.

### 10. B10's `ballots@end_at` is undefined in the chain-side state language [severity: serious]

- **Affected**: Section 3.2 B10
- **Category**: ambiguity / under-specification
- **What's wrong**: B10's RHS is `Tally_spec(ballots@end_at, candidates, privkey)`. The notation `ballots@end_at` does not appear in Section 2.5's state-variable list, the transaction-trace model, or any operator definition. By context, it means "the value of `ballots` at the moment `env.block.time` first reached `end_at`" — but this is a *history-dependent* lookup, not a state-σ value. B2 establishes that `ballots = ballots@end_at` for all σ in Tallying/Resolved (because no further writes occur post-end_at), so the two are extensionally equal *given B2 holds*. But B10 cannot lean on B2 in its definition — it's stating the equality that B2's truth lets you simplify. As written, B10 says "the published tally equals `Tally_spec` applied to a quantity that does not exist as a chain-σ value." This is the same class of error B6's prior form had: referencing data that requires a trace-level model without naming the trace. The fix added a transaction-trace model paragraph for B6 — it is not used for B10.
- **Cite**:
  > B10 | … | `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`
- **Fix recommendation**: Either define `ballots@end_at` formally — "the value of `state.ballots` at the unique chain state σ_first where σ_first satisfies env.block.time ≥ end_at and the predecessor does not" — or replace it with `state.ballots` (the present-state value) and rely on B2 to argue they're equal. The latter is cleaner; the doc has B2 specifically for this purpose.

### 11. B8 clause (d) references "registered verified-rcv enclave image" without specifying the registry [severity: cosmetic-leaning-serious]

- **Affected**: Section 3.2 B8 clause (d)
- **Category**: under-specification / cross-reference
- **What's wrong**: B8 clause (d) is "attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image." Where the registration *lives* is left to inference from Section 6.1's "Enclave-image registration integrity" bullet, which itself says "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR)." But this is one bullet describing *both* a vkey registry and an enclave-image-identity registry. Are these the same on-chain object? Different? Section 4.3a/4.3b only talk about the vkey registry. There is no explicit "MRTD/RTMR registry" in the trust-boundary list separately. Block 6's Requires "attested enclave identity (MRTD / RTMR) matches the expected `verified-rcv` enclave image" uses the word "expected," not "registered." Three slightly-different phrasings ("registered," "expected," "the chain stores") for what should be one object. A reader trying to nail down "is the MRTD on-chain mutable? immutable? subject to the same 4.3b vector?" cannot answer from this doc.
- **Cite**:
  > B8 | … (d) attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image)
  > …
  > Block 6 Requires: … attested enclave identity (MRTD / RTMR) matches the expected `verified-rcv` enclave image
  > …
  > 6.1: the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification.
- **Fix recommendation**: Pick one phrasing, define the registry once (e.g., "the on-chain enclave-image-identity slot, a tuple `(vkey, mrtd, rtmr)` registered at instantiation and immutable thereafter modulo failure mode 4.3b"), and use the same noun phrase everywhere. Currently three references describe the same artifact with three different adjectives.

### 12. Block 6's `commitHashE` row "NOT CONSUMED" is honest but the reason is wrong [severity: serious]

- **Affected**: Section 6.2 Note A
- **Category**: contradiction / preconditional over-strength
- **What's wrong**: Note A says Quartz's `commitHashE` bundle is not consumed because "that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous." The reasoning then offered: verified-rcv uses `Adv_commitTally_CR` bounding SHA-256 collision-resistance on `Borsh(contract_addr ‖ tally_body)`. But *what is the difference between* `commitHashE` and SHA-256-on-Borsh-bytes? Both are commitment hashes. The doc claims `commitHashE` is pigeonhole-impossible (i.e., its formal Lean statement makes collision impossible due to type cardinality, not concrete hash hardness), and that verified-rcv replaces this by stating SHA-256 CR directly. Fine. **But verified-rcv's B8 clause (c) embeds the digest in "the lower half of Quartz's 64-byte `UserData` slot; upper half reserved for domain-separation tag (implementation detail)."** So verified-rcv *is* consuming Quartz's UserData slot — and the function mapping (`contract_addr`, `tally_body`) into that 64-byte slot is morally exactly `commitHashE`. The doc says it's not consuming `commitHashE`, but it's consuming the *same role* under a different name — and B9 includes `Adv_commitTally_CR` as a fresh negligibility summand, sidestepping Quartz's pigeonhole-impossible form by stating SHA-256 CR fresh. This is consistent on the *summand* but **inconsistent on the consumer-count claim**: the verified-rcv compose ledger entry "downstream consumer count is zero" for `commitHashE` is false if "consume" includes "use the slot it commits into." More precisely: verified-rcv inherits the *interface* of `commitHashE` (the `UserData` slot's semantics) without inheriting the *theorem*. Reporting consumer count as zero hides this.
- **Cite**:
  > **Note A — `commitHashE` is listed but unused** … verified-rcv's hash-collision-resistance hypothesis is `Adv_commitTally_CR` (B9 fourth summand), bounding SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`. It does **not** consume Quartz's `commitHashE` bundle — that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous. The row is retained in the table for traceability (other Quartz-downstream specs may consume it), but the verified-rcv compose ledger will record this inheritance row's downstream consumer count as zero.
  > …
  > B8 … **Note on (c)**: the hash is explicitly SHA-256 … 32-byte digest embedded in the lower half of Quartz's 64-byte `UserData` slot; upper half reserved for domain-separation tag (implementation detail).
- **Fix recommendation**: Either (a) declare that verified-rcv consumes the *UserData slot interface* from Quartz (a structural, not theorem-level, inheritance) and the *concrete collision-resistance assumption* fresh as `Adv_commitTally_CR` — making both layers explicit — or (b) state that the lower-32-byte slot is verified-rcv's own private domain and Quartz's UserData semantics are not inherited. The current "consumer count zero" claim is too clean and misrepresents the structural dependency.

### 13. B9's `Pr[B8 violated at the next state]` is not a well-formed probability without a security parameter scoping [severity: serious]

- **Affected**: Section 3.2 B9
- **Category**: ambiguity / preconditional over-strength
- **What's wrong**: B9 says `Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)`. The summands are parameterized by `n` (security parameter) but the LHS is not. What is the distribution over which `Pr[...]` is taken? What is the role of `n` for the LHS — is it "for all `n`, ..."? Is there a "for all PPT 𝒜, exists a negligible function bounded by these summands"? The standard cryptographic form would be `∀ PPT 𝒜, ∃ negl, ∀ n: Pr[...] ≤ ...(n)`. The doc writes it as a deterministic-looking inequality with `n` only on the RHS. Without the quantifier, the inequality is over a single sample at a single `n`, which is what a non-cryptographer reader would assume — and which is *not* a meaningful claim (a single trial bound says nothing about hardness). Sister concern: "the next state by a polynomial-time adversary" — adversary *of what*? An adversary controlling which inputs (chain mempool? enclave inputs? both?). The threat model isn't quantified into the probability expression.
- **Cite**:
  > B9 | … | `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n))`
- **Fix recommendation**: Restate B9 with explicit quantifier structure: "∀ PPT 𝒜 controlling the chain mempool and `publish_result` payloads, ∀ n, image_registration_honest ⇒ Pr[B8 violated at the next transition by 𝒜] ≤ Σᵢ Adv_i(n)." Or punt by saying the formal cryptographic form lives downstream of Quartz's de-tied lifts and this intent-level statement is informal — but say so. As written, the bound's status as a cryptographic claim is ambiguous.

### 14. "fires_at_transition" is silent on Instantiate firing [severity: cosmetic]

- **Affected**: Section 2.5 transaction trace model paragraph
- **Category**: edge case / coverage gap
- **What's wrong**: The trace model lists `tx.kind ∈ {Instantiate, SubmitBallot, CloseAndTally, PublishResult}` and says "at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order." But `Instantiate` is the contract-creation operation — there is no `σ` before instantiation, and CosmWasm models instantiation as a contract-creation message at the module level (a `MsgInstantiateContract` from outside the contract's storage). What does `fires_at_transition(σ_pre_instantiation → σ_after_instantiation)` mean when `σ_pre_instantiation` doesn't exist in the contract's state space? The doc puts Instantiate in the trace alphabet, but never grounds the "before" state. This matters because B6/B8/B10 quantify with `always (...)` — meaning "at all reachable states" — and the very first reachable state is post-Instantiation. The trace model implicitly assumes contract-creation is also a state transition, but with no σ_pre to quantify against. Minor for B6/B8/B10 (they're guarded by post-instantiation predicates that fail at σ_init anyway), but the trace model paragraph should be tight.
- **Cite**:
  > A transaction `tx` is a record with at least `tx.kind ∈ {Instantiate, SubmitBallot, CloseAndTally, PublishResult}` … A state transition `σ → σ′` is parameterized by the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking `σ` to `σ′` …
- **Fix recommendation**: Add a sentence: "Instantiation is modelled as a transition from a designated `σ⊥` (uninhabited contract slot) to `σ_init`; B-series invariants of the form `always P` are read as 'P at all σ in the post-σ_init trajectory.'" Otherwise `always (B6)` is undefined at σ_init.

---

## Summary
- Critical: 2 (Attack 2: undefined `EnclaveImage` symbol; Attack 5: undefined `image_registration_honest` predicate)
- Serious: 10 (Attacks 1, 3, 4, 6, 7, 8, 9, 10, 12, 13)
- Cosmetic: 2 (Attacks 11, 14)

## VERDICT: BREAKS-AGAIN
