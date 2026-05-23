# mistral-small-4-119b-2603 (local) — verified-rcv revised intent adversarial pass

- **Elapsed**: 270.6s
- **Finish reason**: stop
- **Usage**: {"prompt_tokens": 26879, "completion_tokens": 4891, "total_tokens": 31770, "completion_tokens_details": {"reasoning_tokens": 0}}

---

```
VERDICT: BREAKS-AGAIN
```

---

# 1. **Temporal-state mismatch in B6's ∀-per-key formulation**

**Category:** `temporal_state_mismatch`

**Severity:** critical

**Affected location:**
- Intent doc Section 3.2 (B6)
- Intent doc Section 2.5 (transaction trace model paragraph)

**What's wrong:**
B6 claims:
> `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`

This is a **temporal property** that quantifies over *history* (the firing transition `σ → σ′`), yet it is tagged as a state invariant in Section 3.2 and Section 3.1's preamble forbids quantification over operations or time for state invariants.

The transaction trace model in Section 2.5 defines `fires_at_transition(σ → σ′)` as a set of transactions that fired *at that specific state transition*, but B6's statement does not tie the existential `tx` to the *actual firing set* — it only claims existence somewhere in history. This is a **state/temporal mismatch**: the property as written is either:
- A state invariant (false): "for every key whose ballot changed, there exists *some* SubmitBallot transaction in the entire history where `msg.sender = k`", which is trivially true but meaningless, or
- A temporal property (correct intent): "for every key whose ballot changed at transition `σ → σ′`, the change is attributable to a SubmitBallot transaction *in that same transition* whose `msg.sender = k`", which requires quantifying over the firing set.

The spec claims the latter but writes the former — a **mis-encoding of intent as state**.

**Cite:**
- Section 3.2 B6 line: "always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), ...)"
- Section 2.5 transaction trace model paragraph: "Behavioral invariants (B-series) that quantify over `∃ tx ∈ fires_at_transition(σ → σ′)` are reasoning about *that transition's* firing set, not the global history."
- Section 3.1 preamble: "Properties that are always true. If any of these is violated, the system is broken regardless of input." — state invariants are pointwise-evaluable; temporal properties require explicit history quantification.

**Suggested defense:**
Rewrite B6 as a temporal property:
> `always (∀ k ∈ Addr, ∃ tx ∈ fires_at_transition(σ → σ′), (tx.kind = SubmitBallot ∧ tx.msg.sender = k) ⇒ next.ballots[k] = tx.encrypted_preferences)`

This ties the ballot change directly to a transaction in the firing set, matching the intent.

---

# 2. **Impossibility-hypothesis vacuity in B9's `image_registration_honest` precondition**

**Category:** `impossibility-hypothesis-vacuity`

**Severity:** critical

**Affected location:**
- Intent doc Section 3.2 (B9)
- Intent doc Section 6.1 (trust boundary)

**What's wrong:**
B9 states:
> `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + ...)`

But Section 6.1 states:
> "verified-rcv trusts [...] **Enclave-image registration integrity** — the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."

This trust assumption **cannot be enforced by the chain** — it is an *operational* assumption about governance or deployment, not a cryptographic property. The precondition `image_registration_honest` is therefore **not provable within the system's trust model** — it is either:
- Always true (if we assume governance never maliciously substitutes keys), making B9 vacuous, or
- False (if governance *can* substitute keys maliciously), making B9's LHS 1 and the inequality false.

B9 is **not a negligibility budget** — it is a conditional statement whose condition cannot be guaranteed by the system.

**Cite:**
- Section 3.2 B9 line: "always (image_registration_honest → Pr[B8 violated ...])"
- Section 6.1 bullet: "Enclave-image registration integrity — [...] not subject to undetected post-registration modification." (emphasis added)
- Section 4.3b failure mode: "Attestation vkey maliciously substituted — *integrity*" — this is the operational failure that `image_registration_honest` is supposed to rule out, but it is not ruled out by the chain.

**Suggested defense:**
Either:
- Remove `image_registration_honest` and fold its failure mode (vkey substitution) into B9's negligibility budget as a fifth summand `Adv_vkey_substitution(n)`, or
- State explicitly that `image_registration_honest` is an **operational precondition** outside the cryptographic model, and B9's inequality only holds *under that assumption*.

---

# 3. **Disjunction-vs-decomposition collapse in B9's negligibility budget**

**Category:** `disjunction-vs-decomposition`

**Severity:** critical

**Affected location:**
- Intent doc Section 3.2 (B9)
- Intent doc Section 6.2 (Quartz inheritance table)

**What's wrong:**
B9 states:
> `Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)`

But Section 6.2 states:
> "the Quartz substrate `cross_component_session_bind_negl` (and 7 other `_negl` lifts) **was retracted as content-free on 2026-05-14** [...] The refactored lifts def-tie win events to concrete probability expressions, providing the content phase that was missing."

This means:
- The first three summands in B9 are **not negligibility bounds** — they were previously vacuous assumptions.
- The inequality is **not a union bound over independent failure modes** — it is a sum of terms that may not be disjoint or even meaningful.

B9 collapses a **disjunctive security claim** (each failure mode is independent) into a **monolithic sum**, but the terms are not proven to be disjoint or even well-defined after the Quartz retraction.

**Cite:**
- Section 3.2 B9 line: "always (image_registration_honest → Pr[B8 violated ...] ≤ Adv_tdxVerifier_sound(n) + ...)"
- Section 6.2 status block: "the Quartz substrate `cross_component_session_bind_negl` [...] was retracted as content-free on 2026-05-14 [...] The refactored lifts def-tie win events to concrete probability expressions"

**Suggested defense:**
Rewrite B9 as:
> `Pr[B8 violated at the next state by a polynomial-time adversary] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n) + Adv_vkey_substitution(n)`

with explicit justification that the summands are disjoint failure modes, and `Adv_vkey_substitution(n)` is a fifth term covering the operational vkey-substitution failure mode (Section 4.3b).

---

# 4. **Composition failure: `commitHashE` row marked "NOT CONSUMED" but dependency table lists it**

**Category:** `composition_failure`

**Severity:** serious

**Affected location:**
- Intent doc Section 6.2 (Quartz inheritance table)
- Intent doc Section 3.2 (B8 clause (c))

**What's wrong:**
Section 6.2 states:
> "Note A — `commitHashE` is listed but unused [...] verified-rcv's hash-collision-resistance hypothesis is `Adv_commitTally_CR` (B9 fourth summand), bounding SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`. It does **not** consume Quartz's `commitHashE` bundle"

But Section 6.2 also states:
> "verified-rcv inherits — without re-proving — the following Quartz spec-layer claims."

and lists `commitHashE` as an inherited claim.

This is a **composition failure**: the dependency table claims inheritance, but the Note explicitly denies consumption. The downstream compose ledger cannot reconcile whether `commitHashE` is a dependency or not.

**Cite:**
- Section 6.2 inheritance table row: "`commitHashE` — `Specs.Quartz.Crypto.UserDataCommit (commitHashE bundle)`"
- Section 6.2 Note A: "`commitHashE` is listed but unused [...] It does **not** consume Quartz's `commitHashE` bundle"

**Suggested defense:**
Remove the `commitHashE` row from the inheritance table, or clarify in Note A that "verified-rcv does not inherit `commitHashE`; it uses its own SHA-256 collision-resistance hypothesis."

---

# 5. **Refinement mismatch: B10's existential quantifier over `privkey` is ill-typed**

**Category:** `refinement_mismatch`

**Severity:** critical

**Affected location:**
- Intent doc Section 3.2 (B10)
- Intent doc Section 6.3 (dstack / TDX hardware trust)

**What's wrong:**
B10 states:
> `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))`

But:
- The chain only stores `enclave_pubkey`, not `privkey`.
- `dstack_kms_derived(privkey, contract_addr)` is an **off-chain** trust boundary (Section 6.3).
- The existential quantifier ranges over an **off-chain value**, but B10 is tagged as a `cross-layer` invariant — not a chain-side temporal property.

This is a **refinement mismatch**: B10 claims a chain-side property about the published tally, but its witness requires an off-chain existential quantifier over `privkey`. The chain cannot exhibit this witness — it can only accept a tally whose attestation binds to the registered `enclave_pubkey`.

**Cite:**
- Section 3.2 B10 line: "always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, ...)"
- Section 6.3 bullet: "verified-rcv trusts [...] dstack KMS honesty — the KMS releases the per-election private key only after verifying the requesting enclave's attestation."

**Suggested defense:**
Rewrite B10 as:
> `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result tx that fired carried a DstackAttestation whose attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body)) and whose attested enclave identity matches the registered verified-rcv image)`

This ties B10 to B8's attestation predicate, not an existential quantifier over `privkey`.

---

# 6. **Preconditional over-strength in B8's classical-Prop shadow note**

**Category:** `preconditional_over_strength`

**Severity:** serious

**Affected location:**
- Intent doc Section 3.2 (B8)
- Intent doc Section 6.6 (Output contract)

**What's wrong:**
Section 3.2 B8 states:
> "B8 is stated as a deterministic implication; the cryptographic content lives in B9's negligibility budget."

Section 6.6 states:
> "Any party reading `tally_result` from chain state can rely on [...]"

But Section 6.1 states:
> "verified-rcv trusts [...] Enclave-image registration integrity — the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."

This trust assumption is **not a cryptographic property** — it is an operational assumption about governance. Stating B8 as a **deterministic implication** (i.e., "if the vkey is correctly registered, then B8 holds") over-strengthens the precondition — it implies that vkey substitution is impossible, which contradicts Section 4.3b's failure mode.

**Cite:**
- Section 3.2 B8 line: "B8 is stated as a deterministic implication"
- Section 6.1 bullet: "Enclave-image registration integrity — [...] not subject to undetected post-registration modification."
- Section 4.3b failure mode: "Attestation vkey maliciously substituted — *integrity*"

**Suggested defense:**
Rewrite B8 as:
> "always (the publish_result tx that fired carried a DstackAttestation whose [...] attested enclave identity matches the registered verified-rcv image **under the operational assumption that vkey substitution did not occur**)"

and state explicitly that this is a **conditional trust claim**, not a deterministic implication.

---

# 7. **Canonical serialization ambiguity: Borsh discipline not enforced in Section 2.5**

**Category:** `under-specification`

**Severity:** serious

**Affected location:**
- Intent doc Section 2.5 (Canonical serialization block)

**What's wrong:**
Section 2.5 states:
> "canonical_serialization(contract_addr ‖ tally_body) denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`"

But:
- `borsh_bytes` is not defined in the intent doc — it is a CosmWasm convention, but the spec does not state which version of Borsh or how it handles edge cases (e.g., variable-length integers, canonical ordering).
- The IRV_spec's `per_round_counts` and `eliminated_by_round` are maps — Borsh's serialization of maps is not deterministic unless the map keys are sorted, but Section 2.5 does not state that `Map<Addr, Nat>` is serialized in candidate-declaration order.

This leaves **ambiguity** about whether two compliant enclave implementations will produce the same `tally_body` bytes for the same logical tally.

**Cite:**
- Section 2.5 line: "canonical_serialization(contract_addr ‖ tally_body) denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`"
- Section 2.5 `TallyResult` schema: "`per_round_counts: Vec<Map<Addr, Nat>>` — indexed by round; entry per surviving candidate"

**Suggested defense:**
Add a subsection:
> "Canonical serialization discipline: `TallyResult` fields are serialized in declaration order (Section 2.5). Maps are serialized with keys in candidate-declaration order; Vec entries use the same ordering discipline. Borsh version: `v1.2.0` (CosmWasm-native)."

---

# 8. **S5 restatement as set-once write-discipline does not preserve B1's trajectory claim**

**Category:** `refinement_mismatch`

**Severity:** serious

**Affected location:**
- Intent doc Section 3.1 (S5)

**What's wrong:**
Section 3.1 states:
> "S5 (derived) — terminality of resolution (state-shape): `tally_result: Option<TallyResult>` is set-once: no handler writes to `tally_result` after it has been set to `Some(_)`. This is a *write-discipline* property of the contract's handler set (only Block 6 writes `tally_result`, and Block 6's Requires `tally_result.is_none()` gates the write)."

But Section 3.2 B1 states:
> "`always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))` — once an election resolves, it stays resolved with the same value forever"

S5 as restated is a **state invariant** — it does not claim that `tally_result` remains unchanged across transitions. B1's trajectory claim is **not preserved** by S5's restatement.

**Cite:**
- Section 3.1 S5 line: "set-once write-discipline property [...] no handler writes to `tally_result` after it has been set"
- Section 3.2 B1 line: "`always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))`"

**Suggested defense:**
Add a new invariant:
> "B1_redux — `always (tally_result.is_some() → ∀ σ′ reachable from σ, tally_result(σ′) = tally_result(σ))`"

---

# 9. **B10_lean row added but not tied to Lean spec**

**Category:** `under-specification`

**Severity:** serious

**Affected location:**
- Intent doc Section 3.2 (B10_lean row)

**What's wrong:**
Section 3.2 adds:
> "B10_lean — Lean-internal image-IO obligation — `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`"

But:
- The intent doc does not state what `EnclaveImage` is — it is a Lean-extracted model, but the spec does not define its semantics or how it relates to the on-chain `TallyResult`.
- There is no pointer to a Lean theorem, spec file, or proof artifact — it is listed as a row but not tied to any downstream deliverable.

This is **under-specification** — the claim exists, but its formal substrate is not described.

**Cite:**
- Section 3.2 B10_lean line: "`B10_lean` — Lean-internal image-IO obligation [...]"

**Suggested defense:**
Add a subsection:
> "B10_lean — Lean theorem: `theorem B10_lean (raw_ballots candidates privkey : ...) : EnclaveImage raw_ballots candidates privkey = Tally_spec raw_ballots candidates privkey` — see `verified-rcv/lean/B10_lean.lean`."

---

# 10. **SemVer header does not match revision history**

**Category:** `under-specification`

**Severity:** cosmetic

**Affected location:**
- Intent doc header (SemVer classification)

**What's wrong:**
The SemVer header states:
> "v0.3.0 — MAJOR (0.x.0): invariant removed / weakened / trust boundary widened"

But:
- Section 2 states that `TallyResult` schema was added in v0.2.1 — this is a **new invariant**, not removal/weakening.
- Section 3.2 states that `B10_lean` was added in v0.3.0 — this is a **new invariant**, not removal/weakening.

The header mis-classifies the bumps — v0.3.0 should be MINOR (new invariants), not MAJOR.

**Cite:**
- Intent doc header: "v0.3.0 — MAJOR (0.x.0): invariant removed / weakened"
- Section 2 revision history: "`TallyResult` schema (added per Round 3a first-pass adversary Attack 34 — earlier draft inferred this from worked examples only)"
- Section 3.2 revision history: "`B10_lean` row added in Section 3.2 — does it correctly capture the Lean-internal image-IO obligation"

**Suggested defense:**
Update the SemVer header:
> "v0.3.0 — MINOR (0.0.x): new invariant / trust boundary narrowed"

---

# Summary

| Severity      | Count |
|---------------|-------|
| critical      | 5     |
| serious       | 4     |
| cosmetic      | 1     |

---

# META

**Categories attacked:**
- `temporal_state_mismatch` (1)
- `impossibility-hypothesis-vacuity` (2)
- `disjunction-vs-decomposition` (3)
- `composition_failure` (4)
- `refinement_mismatch` (5, 8, 9)
- `preconditional_over_strength` (6)
- `under_specification` (7, 10)

**Categories not attacked:**
- `ambiguity`: Not enough evidence to surface a concrete ambiguity; the document is verbose but mostly precise.
- `coverage_gap`: The intent doc covers all major invariants, failure modes, and trust boundaries; no obvious gaps.
- `edge_case`: The document already lists boundary cases (e.g., Section 2.2, 2.3); no new edge cases surfaced.
- `temporal_state_mismatch` (other instances): Only B6 was targeted; other invariants appear correctly tagged.
- `canonical_serialization`: Attacked under `under_specification`; no further angles.

**Artifacts not provided:**
- The Quartz Lean tree (needed to verify the de-retraction status of `cross_component_session_bind_negl`).
- The verified-rcv Lean tree (needed to verify `B10_lean`).
- The compose ledger (needed to verify dependency tracking).

**Estimated confidence:**
high — all attacks are grounded in specific text and intent mismatches. The document is detailed, but the adversarial angles above are structural flaws in how invariants, trust boundaries, and composition claims are stated.