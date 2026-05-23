# Claude adversarial pass — verified-rcv revised intent

- **Voice**: claude-opus-4-7 (Agent subagent, file-access)
- **Target**: .colosseum/intent.md (revised)
- **Output dir**: .colosseum/attacks/intent-revised-2026-05-14T200029Z/

## Findings

### CRITICAL

#### 1. S5 is mis-tagged: a successor-state-quantified property listed as a "structural invariant" (temporal_state_mismatch)

- **Location:** Section 3.1, row S5 (line 305).
- **What's wrong:** Section 3.1's preamble (line 297) declares: "Properties evaluable on contract state at any reachable moment. No quantification over operations or time." S5's statement explicitly quantifies over successor states: "if `tally_result.is_some()` at state σ, then any successor state σ' has `tally_result = σ.tally_result`." The phrase "any successor state σ'" is a temporal quantifier; S5 is a binary-state property, not pointwise-evaluable. The annotation `(derived)` and the prose "listed for explicit state-shape reference per Round 3a first-pass adversary Attack 26" are an attempted defense, but the **statement itself** still carries the σ → σ' quantification that Section 3.1 forbids by its own preamble. This is the exact failure mode the `temporal_state_mismatch` attack category names: a temporal claim shoved into a state-only invariant slot. By contrast, S10 ("`tally_result.is_some()` ⇒ `env.block.time ≥ end_at`") is genuinely pointwise; it survives.
- **Cite:**
  - Preamble at line 297: "*No quantification over operations or time.*"
  - S5 at line 305: "*if `tally_result.is_some()` at state σ, then any successor state σ' has `tally_result = σ.tally_result`*"
- **Suggested fix:** Either (a) move S5 to Section 3.2 with a `temporal` tag and call it explicitly a corollary of B1; or (b) restate S5 as a *type-shape* property (e.g., "`tally_result: Option<TallyResult>` is `set-once-then-immutable` per the storage schema") and let B1 carry the trajectory claim. Option (b) keeps a structural row; option (a) is honest. The current form fails Section 3.1's own gating rule.

#### 2. B9 inherits Quartz's `cross_component_session_bind_negl` for the "first three summands" — but that theorem is about session-pubkey-binding, not tally-binding, and Quartz exports no 3-summand lift with the claimed shape (composition_failure)

- **Location:** Section 3.2, row B9 (line 326); Section 6.2 inheritance table (line 467 row 2 and the substrate-status block at lines 473–479).
- **What's wrong:** B9's bound is `Pr[B8 violated] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)`, and the row text says "**Inherited from Quartz's `cross_component_session_bind_negl` for the first three summands.**" Cross-checked against `/Users/mvid/Development/reliq/quartz/.colosseum/ledger.md` and `Specs/Quartz/Protocol/ProtocolVCVioQuad.lean`:
  - `cross_component_session_bind_negl` is a **5-summand** quadruple-bundle lift: `commitHashE-CR + commitHashBytesE-CR + tdxVerifier + groth16-KS + circuit-eq`. It exists in service of *session-pubkey-binding* (`Specs.Quartz.Protocol.ProtocolVCVioQuad`), not generic TDX-quote-binding-arbitrary-user_data.
  - To "inherit the first three summands" of `cross_component_session_bind_negl`, verified-rcv would need to pick `commitHashE-CR + commitHashBytesE-CR + tdxVerifier` (Quartz's first three) — but the intent doc instead claims to pick `tdx + groth16-KS + circuit-eq`. Those are **summands #3, #4, #5** of Quartz's quad, not its first three.
  - Even renumbered, the three crypto summands cannot be extracted as an independent reusable union bound from `cross_component_session_bind_negl`. They are entangled with the Quartz session-pubkey + UserDataCommit construction (the theorem's witness predicate is "the adversary forged a session-pubkey commitment"). The Quartz ledger's index lists exactly which lifts decompose how: the dual lift `handshake_sound_negl` is 2 summands (tdx + groth16-combined, **not** split into KS + circuit-eq); only `cross_component_session_bind_negl` exposes the KS/circuit-eq split — and only under the full 5-summand entangled form.
  - Net: there is no Quartz Lean theorem of the shape "TDX-DCAP-attestation binds arbitrary user_data, with 3-summand decomposition tdx + groth16-KS + circuit-eq, content-bearing in the absence of a commit-hash construction." The intent doc's inheritance claim points at a theorem that doesn't exist with the claimed signature.
- **Cite:**
  - Intent B9 row, line 326: "*Inherited from Quartz's `cross_component_session_bind_negl` for the first three summands.*"
  - Quartz ledger composition map shows `cross_component_session_bind_negl` decomposes as 5 summands including both commitHash flavors (lines 120-145 of `/Users/mvid/Development/reliq/quartz/.colosseum/ledger.md`).
  - Quartz ledger Step 5 doubled-negligibility split: "**Quadruple → 5 summands** (1): `cross_component_session_bind`" (ledger composition-shape-per-theorem section).
- **Suggested fix:** Either (a) restate B9 as inheriting a different Quartz theorem — likely a new `verify_tdx_quote_binds_user_data_negl` that doesn't currently exist upstream (block on Quartz, add to compose ledger as v0.3 upstream ask); or (b) drop the "inherited" claim, name `Adv_tdxVerifier_sound`, `Adv_groth16_KS`, `Adv_circuit_eq` as verified-rcv-internal hardness assumptions parameterised against Quartz's *axioms* (not its lifts), and accept that the cryptographic content depends on Quartz's content-phase refactor landing. The substrate-status block at 6.2 already acknowledges the underlying retraction, but does not address the more specific issue that the **named theorem** B9 inherits from doesn't have the claimed decomposition shape.

#### 3. B6's existential is satisfiable by an unrelated historical write — the formulation does not bind `tx` to the actual `next.ballots ≠ ballots` transition (under-specification)

- **Location:** Section 3.2, row B6 (line 323).
- **What's wrong:** B6 is stated as `always (next.ballots ≠ ballots → ∃ tx, tx.kind = SubmitBallot ∧ tx.msg.sender ∈ next.ballots ∧ next.ballots[tx.msg.sender] = tx.encrypted_preferences)`. The existential is over **any** `tx`, with no clause forcing `tx` to be the transition that fired between the two states. The conjuncts only require:
  1. `tx.kind = SubmitBallot`
  2. `tx.msg.sender ∈ next.ballots`
  3. `next.ballots[tx.msg.sender] = tx.encrypted_preferences`
  Conjuncts 2-3 say "*some* historical SubmitBallot tx is consistent with *some* entry in next.ballots". After any honest history of SubmitBallot writes, this is trivially satisfied by exhibiting any one of the historical writers. A byzantine mutation that, say, sets `ballots[Mallory] := corrupted_blob` (where Mallory was never the writer) does not falsify B6, because B6 only asks "does there exist *some* SubmitBallot tx writing *some* key" — Alice's prior honest write still witnesses the existential.
  The intended property is: **the specific transition** between σ and σ' was caused by a SubmitBallot tx whose `msg.sender` is the key being written. That requires the formulation to bind the transition action explicitly — e.g., `next = step(σ, tx) ∧ tx.kind = SubmitBallot ∧ next.ballots = σ.ballots[tx.msg.sender ↦ tx.encrypted_preferences]`, or a fluent over the action label of the firing step.
  The revision note at the end of the row claims "B6 is the temporal claim Block 3 enforces" — but the temporal formulation given does not actually express that claim. The first-pass adversary's Attack 6 caught the state/temporal tag mis-mismatch; the re-tag fixed the *label* but the *statement* still does not capture the writer-binding semantics.
- **Cite:**
  - Intent B6 row, line 323: "*`always (next.ballots ≠ ballots → ∃ tx, tx.kind = SubmitBallot ∧ tx.msg.sender ∈ next.ballots ∧ next.ballots[tx.msg.sender] = tx.encrypted_preferences)`*"
  - Methodology agent system prompt, `temporal_state_mismatch` definition: "*nothing forces the accept event to causally depend on the vkey being set. The shape of the property is wrong for the shape of the spec.*"
- **Suggested fix:** Re-state B6 to bind the existential to the firing transition: `always (next.ballots ≠ ballots → ∃ tx, fires(tx) ∧ tx.kind = SubmitBallot ∧ ∃ k, σ.ballots ≠ next.ballots only at k ∧ k = tx.msg.sender ∧ next.ballots[k] = tx.encrypted_preferences)`. Or equivalently, encode B6 as a relation on (σ, action, σ') triples rather than (σ, σ') pairs. The methodology's `temporal_state_mismatch` discipline is exactly meant to catch this. The first-pass fix re-tagged the property but did not re-state it correctly; this is a regression-after-fix.

### SERIOUS

#### 4. B9's LHS `Pr[B8 violated]` is not bounded by its RHS once the dropped `Adv_image_registration` summand is properly accounted for (preconditional over-strength)

- **Location:** Section 3.2 row B9 (line 326); Section 6.1 line 458; failure mode 4.3b (line 366).
- **What's wrong:** B9 bounds `Pr[B8 violated at the next state by a polynomial-time adversary] ≤ <4 crypto summands>`. B8 has four clauses, including (d) "*attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image*". Failure mode 4.3b describes a concrete way to violate (d) without breaking any cryptographic primitive: governance-compromise or operator-error substitutes a malicious vkey on-chain; an attestation for a *different* (malicious) image now validates against the registered vkey. None of the 4 crypto summands bound this event. The previous draft's `Adv_image_registration` summand was dropped (justifiably) on the grounds that "operational fault rate, not a cryptographic advantage parameterised by security parameter `n`" — but **dropping the summand from the bound without conditioning the LHS leaves the inequality false**: an adversary who substitutes the vkey has success probability ≈ 1 (not negligible), and the RHS is a sum of negligibles. The honest restatement is `Pr[B8 violated | enclave-image registration honest] ≤ <4 crypto summands>` with "enclave-image registration honest" as an explicit precondition. Section 6.1 line 458 captures the assumption as a trust-boundary item, which is correct — but B9's LHS does not reference this precondition. The bound, as stated, is false.
- **Cite:**
  - B9 RHS, line 326: lists 4 summands, none operational.
  - B9 explanation, line 326: "*(ii) `Adv_image_registration` (Attack 18 — operational fault rate, not a cryptographic advantage parameterised by security parameter `n`; recorded instead as a trust-boundary assumption in Section 6.1).*"
  - 4.3b, line 366: concrete attack path that violates B8(d) without breaking any crypto primitive.
- **Suggested fix:** Condition B9 on the trust-boundary precondition: `always (image_registration_honest → Pr[B8 violated] ≤ <4 crypto summands>)`. Alternatively, lift "image_registration_honest" into a hypothesis on the composed theorem at the Lean layer.

#### 5. Section 6.2 inheritance row 4 (`commitHashE` collision-resistance) is operationally dead — verified-rcv's commit hash is its own SHA-256, not Quartz's `commitHashE` (triviality / coverage gap)

- **Location:** Section 6.2 inheritance table, row 4 (line 469); contrast B9 row, line 326.
- **What's wrong:** The Section 6.2 table lists "`commitHash` collision-resistance on `UserDataCommit → UserData` | `Specs.Quartz.Crypto.UserDataCommit` (`commitHashE` bundle)" as an inherited Quartz claim. But B9 explicitly says verified-rcv's relevant collision-resistance hypothesis is `Adv_commitTally_CR`, a verified-rcv-internal SHA-256 bound on `(contract_addr ‖ tally_body)` — explicitly **not** Quartz's `commitHashE`. The revision note at B9 makes this even sharper: "*the missing summand earlier drafts silently inherited via the Quartz pigeonhole-impossible `commitHashE` axiom*" — i.e., the prior version was wrongly relying on `commitHashE`, and the revision corrected this by introducing `Adv_commitTally_CR` instead. So `commitHashE` should not still appear in the inheritance table; nothing in verified-rcv consumes it. Listing it as "inherited" is operationally vacuous and misleading about the trust dependencies. The row may be intended as "we inherit Quartz's general pattern of how to express hash-CR", but that's not what an inheritance table means — an inheritance table lists *consumed* upstream claims.
- **Cite:**
  - Section 6.2 row 4, line 469.
  - B9 row, line 326: "*Verified-rcv-specific for the fourth: `Adv_commitTally_CR` bounds collision resistance of `SHA-256(canonical_serialization(contract_addr ‖ tally_body))` as the binding hash in B8 clause (c) — the missing summand earlier drafts silently inherited via the Quartz pigeonhole-impossible `commitHashE` axiom (Attack 5).*"
- **Suggested fix:** Remove the `commitHashE` row from the Section 6.2 table (since verified-rcv does not consume it), or annotate the row "**NOT CONSUMED** — see B9 `Adv_commitTally_CR` for the actual verified-rcv hash-CR hypothesis." The withdrawn DstackKeyManager row already sets the precedent for honest withdrawal.

#### 6. Section 8.7 Step 3 mis-labels a Lean-internal composition as "B10" — B10 is a chain-side temporal claim, not a Lean image-IO claim (composition_failure / refinement mismatch)

- **Location:** Section 8.7 numbered list, step 3 (line 664); contrast Section 3.2 B10 statement (line 327) and the "Trust claim consumed" line at 669.
- **What's wrong:** Step 3 reads: "**Composition theorem — B10**: by composition of the above two, the image's externally observable input-output relation `(raw_ballots, candidates, privkey) ↦ tally` equals `Tally_spec(raw_ballots, candidates, privkey)`, which is what B10 asserts." But B10 (line 327) actually states a temporal property about the chain's `next.tally_result`: `always (next.tally_result.is_some() ∧ tally_result.is_none() → next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey)))`. The chain's published tally and the image's IO relation are not the same thing: the chain accepts a tally bound by attestation (B8), and only if the attested image is the verified-rcv image does the chain's tally equal the image's output. So `Stage-1 lemma ∧ Stage-2 theorem ⇒ image-IO = Tally_spec` (call this Lean-B10'), but `Lean-B10' + image-identity-binding (Step 4) + B8 (chain-side) ⇒ B10`. Step 3 ascribes the name "B10" to the Lean-internal corollary, which is a refinement mismatch: the Lean theorem and the temporal invariant share a name but assert different things. The wrap-up line 669 — "Composition pattern: `B10 ← (Stage-1 lemma ∧ Stage-2 theorem)` at the Lean layer + `B8` at the chain layer" — is *more honest* and explicitly includes B8 in the discharge, but the numbered structure at lines 660-665 marks Step 3 alone as "Composition theorem — B10" and demotes Step 4 (image-identity binding) to "*not* a Lean theorem — captured separately in the compose ledger as a build/deployment discipline item." Step 4 is load-bearing for B10's actual claim, but the structure presents it as a side-channel; the methodology's `composition_failure` discipline asks specifically for this kind of cross-layer dependency to be made explicit.
- **Cite:**
  - Section 3.2 B10 statement, line 327.
  - Section 8.7 step 3, line 664: "*Composition theorem — B10: by composition of the above two, the image's externally observable input-output relation … equals `Tally_spec(…)`, which is what B10 asserts.*"
  - Section 8.7 step 4, line 665: "*Image identity binding … This is an extraction-soundness / build-reproducibility obligation, not a Lean theorem.*"
- **Suggested fix:** Rename Step 3 to "Lean-internal composition: image-IO = `Tally_spec`" and add an explicit Step 5 (or fold into Step 4) that names the cross-layer composition for B10: `B10 ← (Lean composition) ∧ (build-reproducibility / image-binding) ∧ B8`. The intent doc may also want to introduce a separate name (e.g., `B10_lean`) for the Lean-internal half so they're not collapsed.

#### 7. Section 8.7 Stage-1 lemma's dependency on `Ecies.roundtrip` covers honest ciphertexts only — Stage 1's correctness requires reasoning about adversarial / malformed ciphertexts (composition_failure)

- **Location:** Section 8.7 step 1 (line 662); Section 6.2 inheritance row 3 (line 468); cross-ref Section 2.5 Stage 1 (line 134) and Section 2.3 malformed-ballot edge case (line 70).
- **What's wrong:** Step 1 says the Stage-1 lemma's correctness "Depends on Quartz's `Specs.Quartz.Crypto.Ecies.roundtrip` for the decryption-correctness half." Cross-checked at `/Users/mvid/Development/reliq/quartz/proofs/lean/Specs/Quartz/Crypto/Ecies.lean:133`: `theorem roundtrip (sk : PrivKey) (pt : Plaintext) : decrypt sk (encrypt (keyOf sk) pt) = pt`. This says: *honestly produced ciphertexts decrypt to the original plaintext.* But Stage 1 (Section 2.5, line 134-136) operates on `raw_ballots : Map<Addr, Vec<u8>>` — arbitrary byte blobs submitted by ECIES-encrypting (or not) under whatever the voter chose to encrypt under. The malformed-ballot edge case (Section 2.3, lines 70-74) covers four failure modes, including "Decryption fails (corrupted ciphertext, wrong key) → dropped." `Ecies.roundtrip` does NOT bound the behavior of `decrypt sk (random_bytes)` — and the Quartz `Ecies` spec does not appear to expose an authenticated-decryption property (no MAC / AEAD claim). If the ECIES variant is not authenticated, an adversary could submit a ciphertext that decrypts to *some* arbitrary plaintext, potentially even a valid permutation of `candidates`, and Stage 1 would accept it. The Stage-1 lemma needs (a) a decryption-failure-on-malformed-ciphertext property (authenticated decryption / AEAD), AND (b) a parse-validation property (the parser rejects non-permutations). `Ecies.roundtrip` alone is insufficient.
- **Cite:**
  - Section 8.7 step 1, line 662: "*Depends on Quartz's `Specs.Quartz.Crypto.Ecies.roundtrip` for the decryption-correctness half (Section 6.2 inheritance row 3 — a theorem, not an axiom).*"
  - `/Users/mvid/Development/reliq/quartz/proofs/lean/Specs/Quartz/Crypto/Ecies.lean:133`: `theorem roundtrip (sk : PrivKey) (pt : Plaintext) : decrypt sk (encrypt (keyOf sk) pt) = pt`
  - Section 2.3 malformed-ballot edge case, line 70: "Decryption fails (corrupted ciphertext, wrong key) → dropped."
- **Suggested fix:** Either (a) specify that the ECIES variant is authenticated (AEAD, e.g., ECIES-AES-GCM) and add a Quartz dependency on an `Ecies.auth_decrypt_unforgeability` theorem (which currently doesn't appear to exist — block on Quartz, add to compose ledger); or (b) re-specify Stage 1 so its correctness obligation is "decrypt-or-drop is a deterministic function of (ciphertext, sk)" and decouple the *correctness* property from any soundness claim about which ciphertexts decrypt to what (in which case Stage 1 is correct definitionally, and the security claim moves to the integrity story — but then the well-formedness check is the only thing standing between adversarial ciphertext and accepted tally, which the intent doc must spell out).

#### 8. Section 2.5 IRV algorithm has dead-code / redundant edge-case branch (over-specification / ambiguity)

- **Location:** Section 2.5 lines 142-152 (IRV_spec algorithm and edge cases).
- **What's wrong:** Step 1 of the IRV algorithm specifies the base case as: "*if `|remaining| = 1`, the sole remaining candidate is the winner; if all remaining candidates have equal first-place counts in the current round, **all of them are co-winners**.*" This covers both `|remaining| = 1` AND the all-tied-multi-remaining case as termination. Then the "Edge cases" subsection at lines 150-152 adds: "**All remaining candidates tied for lowest**: batch elimination would empty `remaining`. In this case, declare all current `remaining` candidates as co-winners (per "ties → multi-winner" policy)." This is the same case Step 1 already handles — "all tied for lowest" ⊆ "all remaining have equal first-place counts." The edge-case bullet is dead code (it could only fire if Step 1 had been skipped, but Step 1 fires before the recursive case). Worse: the edge-case bullet is written as a clarification of Step 3 (the batch-elimination step), suggesting it is reached *after* failing Step 1 — but a careful reading shows it's actually subsumed by Step 1. A spec writer porting this to Lean / Quint will either (a) duplicate the case and produce two unreachable branches, or (b) collapse one and risk dropping a witness. Resolution should be explicit.
- **Cite:**
  - Section 2.5 step 1, line 144: "*Base case (single candidate or terminal-tie): if `|remaining| = 1`, the sole remaining candidate is the winner; if all remaining candidates have equal first-place counts in the current round, all of them are co-winners.*"
  - Section 2.5 edge case, line 151: "*All remaining candidates tied for lowest: batch elimination would empty `remaining`. In this case, declare all current `remaining` candidates as co-winners…*"
- **Suggested fix:** Remove the edge-case bullet (it's subsumed) OR re-state Step 1 to handle only `|remaining| = 1`, with the all-tied case being handled exclusively as the edge case in Step 3's failure-to-make-progress branch. Either is internally consistent; the current form has both, and they overlap.

#### 9. B10's RHS references `enclave_privkey`, an off-chain quantity, in a statement quantifying over chain states — implicit dependency on an unobservable parameter (refinement mismatch)

- **Location:** Section 3.2, B10 statement (line 327).
- **What's wrong:** B10 reads `always (next.tally_result.is_some() ∧ tally_result.is_none() → next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey)))`. The first three of `Tally_spec`'s arguments — `ballots@end_at`, `candidates` — are chain-observable. The third, `enclave_privkey`, is **never observable on-chain**; the chain has only `enclave_pubkey`. The statement, taken literally, quantifies over chain states (the `always` modality) but references a value that no chain state contains. Either the statement is meant to be parameterised by an external (off-chain) constant `enclave_privkey` — in which case the temporal-logic frame is over chain × (off-chain enclave context) — or the statement is vacuous/ill-typed in any chain-only model. Section 8.7 step 4 (image-identity binding) gestures at this — the binary running in the enclave is bound by attestation, and that binary internally holds `enclave_privkey` — but B10 doesn't carry that binding.
- **Cite:**
  - Section 3.2 B10, line 327: `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, enclave_privkey))`.
  - Section 2.5 line 132: `Tally_spec(raw_ballots, candidates, enclave_privkey) → TallyResult`.
- **Suggested fix:** Make the off-chain dependency explicit. Either (a) re-state B10 as `… → ∃! privkey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))`, with the existence/uniqueness of `privkey` being part of the dstack KMS trust assumption (Section 6.3); or (b) factor B10 into a Lean-side statement and a chain-side statement, with the off-chain context made first-class. Approach (b) matches the 8.7-style two-layer discharge but should be reflected in B10's *statement*, not only its commentary.

### COSMETIC

#### 10. B2 uses `ballots@end_at` notation without defining it — circular if read literally

- **Location:** Section 3.2 row B2 (line 319).
- **What's wrong:** B2 reads `always (env.block.time ≥ end_at → ballots = ballots@end_at)`. The notation `ballots@end_at` is presumably "the value of `ballots` at the first state where `env.block.time ≥ end_at`," but the intent doc doesn't establish this notational convention. Read literally, `ballots = ballots@end_at` is the trivial identity if `ballots@end_at := ballots` (current value). Read substantively (the value at the freeze point), the property is well-formed but depends on an implicit definition of subscript-at-time. Same notation is reused in B10 (line 327). For an informal intent doc this is fine, but downstream Lean / Quint encoders will need an explicit definition.
- **Cite:** Lines 319, 327.
- **Suggested fix:** Add a one-line notational convention: "`x@t` denotes the value of state variable `x` at the first state σ with `σ.env.block.time ≥ t`."

#### 11. Section 2.1 worked example's claim that Round 1 "no movement" is technically correct but pedagogically confusing

- **Location:** Section 2.1, Round 1 row of the tally table (line 32).
- **What's wrong:** The Round 1 cell says: "B has 0 first-place votes (unique lowest). Eliminate B. Redistribute B's ballot: B's first surviving choice is A (since `A > B > …` after dropping B is `A > C > D > E`), which already counts B's first-place vote — no movement." But B's ballot was originally `A > B > C > D > E` (per line 23: "B's ballot: A > B > C > D > E"), so the elimination of B (the candidate) doesn't move B's ballot's first-rank — A was already B's first choice. The phrasing "B's first surviving choice is A (since `A > B > …` after dropping B is `A > C > D > E`)" is correct but reads as if dropping B changed A's position; in fact A was the first rank to begin with. Minor confusion, but the example is the primary witness for Section 2.5's batch-elim semantics — clarity matters.
- **Suggested fix:** Re-word: "B's ballot already had A as its first choice; no redistribution movement is required."

#### 12. Section 8.2's tally trace dropped the second batch-elimination round that Section 2.5 would emit

- **Location:** Section 8.2 (line 567-593); trace at lines 580-583.
- **What's wrong:** Section 8.2 has 2 ballots: A's `A > B > C > D > E` and C's `C > A > B > D > E`. Round 1: A=1, B=0, C=1, D=0, E=0. The min count is 0, tied among B, D, E. Section 8.2's trace then jumps directly to "Round 2: A and C tied 1-1. Terminal tie → both co-winners." But by Section 2.5 Step 1 ("if all remaining candidates have equal first-place counts in the current round, **all of them are co-winners**"), Round 2 should fire the equal-counts base case before getting to a "terminal tie" framing. The end state (A and C are co-winners) is the same, but the description "terminal tie" is informal; the spec's actual termination condition is "all equal first-place counts in the round." Internal consistency would benefit from using the spec's terminology.
- **Suggested fix:** Replace "Terminal tie → both co-winners" with "All remaining candidates have equal first-place counts; per Section 2.5 step 1 base case, all remaining are co-winners."

## Summary

| Severity | Count |
|---|---|
| Critical | 3 |
| Serious | 6 |
| Cosmetic | 3 |

## VERDICT

BREAKS-AGAIN
