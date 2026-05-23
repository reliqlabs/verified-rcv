# Synthesis — intent v0.3.0 (3rd adversarial pass)

- **Date**: 2026-05-16
- **Target**: `verified-rcv/.colosseum/intent.md` at v0.3.0
- **Inputs**: 5 voices with substantive output across 23 per-slice reports + 1 full-spec subagent report
- **Method**: per-section (Pattern B) dispatch via OpenCode `spec-adversary` agent + Claude Code Agent subagent for the Claude voice. See `colosseum-adversarial` SKILL.md v0.3 Step 6 for synthesis format.

## Voice roster + source

| Voice | Family | Inference | Mode | Output |
|---|---|---|---|---|
| claude-opus-4-7 | Anthropic | Claude Code Agent subagent | inlined full | 14 attacks (2C + 10S + 2c); see `../intent-v0.3.0-2026-05-16T121229Z/claude.md` |
| kimi-k2-6 | Moonshot | gateway (Pattern B sequential) | per-slice ×9 | ~49 attacks across 9 slice files |
| mistral-small-4-119b-2603 | Mistral | LM Studio (Pattern B sequential) | per-slice ×1 | 4 attacks on state-invariants (smoke; full panel not run) |
| gpt-oss-120b | OpenAI-OSS | gateway (Pattern B parallel) | per-slice ×4 | ~25 attacks across 4 slice files; 5 slices stalled (not retried) |
| gemma-4-26b-a4b | Google | LM Studio (Pattern B sequential) | per-slice ×8 | ~24 attacks across 8 slice files; behaviors-core abandoned (3× timeout) |

**Excluded from synthesis** (per v0.3 Ask C / Round 3a calibration):
- `glm-4-7-flash` — degenerate-loop behavior on behaviors-core (113K chars of fake enumerated attacks); 30B "flash" tier voice retired from gateway adversarial dispatch.
- `qwen3.6-27b` — never produced clean output under Pattern B dispatch; needs separate triage.

## Section A: Overlap matrix — findings ordered by descending voice support

### T1. `EnclaveImage` symbol is undefined and arity-inconsistent [CRITICAL — 5 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 2 (full-spec) | critical |
| kimi | temporal-invariants #4; behaviors-types #5; off-chain-witness #1 | critical / serious / critical |
| gpt-oss | behaviors-types #1 ("Missing secret-key input"); temporal-invariants #5 ("arity mismatch") | critical / contradiction |
| gemma | temporal-invariants #1 ("Symbol Contradiction"); behaviors-types #3; off-chain-witness #1 | critical / cosmetic / serious |
| mistral | (not dispatched for this slice) | — |

**Consensus**: the CONTEXT_APPENDIX and §2.5 describe `EnclaveImage : Bytes → Bytes` (1-ary, byte-level extraction). §3.2 B10_lean and §8.7 step 3 use it as `EnclaveImage(raw_ballots, candidates, privkey)` (3-ary, semantic). No bridging definition. As stated, `B10_lean := EnclaveImage(..) = Tally_spec(..)` is dischargeable by `rfl` if a proof author sets `EnclaveImage := Tally_spec`. The B10 cross-layer chain pivots on a symbol whose type is unsettled.

**Consensus fix**: §2.5 must introduce a typed `EnclaveImage` (either rename the 1-ary extraction to `extract_enclave_model : Bytes → EnclaveModel` and give `EnclaveModel.semantics : RawBallots → CandidateSet → PrivKey → TallyResult`, or explicitly overload and define a coercion). §8.7 step 4 must tie image-identity-binding to *this* `EnclaveImage` symbol, not to a free name.

### T2. `image_registration_honest` is an undefined predicate in a formal modality [CRITICAL — 5 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 5 (full-spec) | critical |
| kimi | temporal-invariants #2; trust-quartz #2; scope #1 (consumer-side) | critical / critical / serious |
| gpt-oss | temporal-invariants #4 | ambiguity / over-spec |
| gemma | temporal-invariants #2 | serious |
| (kimi also calls this out in 3 separate slices) | | |

**Consensus**: B9's antecedent `image_registration_honest` is named but never formally defined. §6.1's last bullet describes the trust intent in operational prose ("not a cryptographic property reducible to a security parameter") but does not give the predicate state-evaluation semantics. With no definition, B9 reduces to "if [unspecified condition] then bound" — epistemically equivalent to no bound. The conditioning was supposed to lift failure mode 4.3b out of the negligibility budget; substituting an undefined predicate relocates the problem rather than fixing it.

**Consensus fix**: define `image_registration_honest(σ) ≡ chain_registered_vkey(σ) = canonical_verified_rcv_vkey ∧ chain_registered_mrtd(σ) = canonical_verified_rcv_mrtd` (or equivalent precise form), with explicit evaluation semantics. State B9's quantifier shape (∀ PPT 𝒜, ∀ n, σ satisfying precondition ⇒ …) so the bound is a well-formed cryptographic claim, not LTL.

### T3. B10 cross-layer factorization is missing load-bearing conjuncts [CRITICAL — 4 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 7 (full-spec — step 6 hand-waves existential) | serious |
| kimi | temporal-invariants #3; off-chain-witness #2 (smuggled 4th conjunct); off-chain-witness #5 (input-fidelity gap) | critical / serious / serious |
| gpt-oss | temporal-invariants #6 (factorisation lacks definition) | under-spec |
| gemma | off-chain-witness #2 (Compositional Gap: Underspecified Key Provenance) | critical |

**Consensus**: the advertised factorization `B10 ← B10_lean ∧ image-identity-binding ∧ B8` is **insufficient**. The actual B10 statement carries an existential `∃ privkey, dstack_kms_derived(privkey, contract_addr) ∧ ...` that requires:
- A `dstack-KMS-trust` conjunct (the privkey actually came from dstack KMS for this contract) — currently smuggled into Step 6 as "instantiated by".
- An `input-fidelity` conjunct (the enclave's *actual* input was `ballots@end_at` and the contract's `candidates`, not host-provided fakes). A malicious host could feed empty ballots; the correct image would correctly compute `Tally_spec(∅, candidates, privkey)`; B8 would attest; B10 would be violated.

**Consensus fix**: rewrite §8.7 closing pattern to `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack-kms-trust(§6.3) ∧ input-fidelity`. Add a numbered step that discharges input-fidelity (e.g., TDX-reported input hash inclusion in the attestation, or a chain-inclusion proof embedded in the attestation envelope). Mark `dstack-kms-trust` as an axiom-level conjunct, not an "instantiated by" hand-wave.

### T4. S5's "set-once write-discipline" is not pointwise-evaluable as claimed [SERIOUS — 5 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 8 (full-spec) | serious |
| kimi | state-invariants #1 ("S5 is a code property, not a pointwise state invariant") | serious |
| mistral | state-invariants #1 ("temporal-by-construction") | critical |
| gpt-oss | state-invariants #1 + #2 (temporal-state mismatch + ambiguity of pointwise evaluability) | serious / serious |
| gemma | state-invariants #1 ("Ambiguous 'Handler Index'") | serious |

**Consensus**: the v0.3.0 reformulation of S5 — "evaluable on a single state by inspecting the handler index" — still embeds a temporal claim. "After it has been set" and "Block 6's Requires gates the write" are properties of the contract code, not σ. §3.1's preamble ("Properties evaluable on contract state at any reachable moment. No quantification over operations or time.") does not admit handler-set inspection as pointwise. The reformulation moved the temporal quantification from "over successor states" to "over the handler index" — different shadow, same shape.

**Consensus fix**: pick one of three paths: (a) move S5 to §3.2 as a temporal write-discipline corollary of B1; (b) amend §3.1's preamble to admit "handler-set inspection without trajectory quantification"; or (c) restate S5 as a pure type-shape claim like "`tally_result` is `None` or `Some(_)` at every reachable state" and shed the write-discipline content (which lives properly in Block 6's Requires). Mistral elevated this to critical; the consensus tier is serious-with-critical-shadow.

### T5. `Tally_spec` is not a deterministic function — Map iteration discipline is undefined [CRITICAL — 4 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 4 | serious (semantic determinism) |
| kimi | behaviors-types #1 (Map iteration); #2 (Borsh Map type ambiguity); #6 (IRV_spec output ordering) | critical / critical / serious |
| gpt-oss | behaviors-types #2 (unspecified iteration order) | serious |
| gemma | behaviors-types #1 (underspecified Map iteration for dropped_voters) | serious |

**Consensus**: §2.5 Stage 1 says `dropped_voters` is "ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline)" but **never defines that discipline**. The CosmWasm `Map` iteration is over storage-key lexicographic order, not candidate-declaration order — and the `TallyResult` schema's order discipline ("candidate-declaration order") applies to its output Vec/Map fields, not its input iteration. Two honest implementations can produce different `dropped_voters` orderings, both compliant. B8(c)'s SHA-256 over Borsh-encoded `tally_body` then produces different digests for the same logical tally, breaking attestation binding. B10's `tally_result = Tally_spec(...)` equality is not even well-typed unless the input iteration is pinned.

**Consensus fix**: pin `dropped_voters`'s order to a value derivable from `candidates` (e.g., "candidate-declaration-order restricted to `raw_ballots.keys` that failed validation"). OR make Stage 1 take `raw_ballots: Vec<(Addr, Ciphertext)>` in a chain-deterministic order. Additionally specify the concrete Rust type for `Map<Addr, Nat>` fields (e.g., `Vec<(Addr, Nat)>` populated in candidate-declaration-order; forbid `BTreeMap`/`HashMap` whose Borsh encoding diverges).

### T6. B6 trace model is silent on absent-key semantics and concurrent-tx behavior [SERIOUS — 4 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 3 (absent-key comparison) | serious |
| kimi | temporal-invariants #6 (LTL modality binding); behaviors-types #3 (slot semantics + multi-msg-per-tx) | serious / serious |
| gpt-oss | behaviors-types #4 (incomplete trace ordering); temporal-invariants #1 (concurrent SubmitBallot) | serious / serious |
| gemma | behaviors-types #4 (composition failure for fires_at_transition) | serious |

**Consensus**: B6's `next.ballots[k] ≠ ballots[k]` predicate depends on `ballots[k]` semantics for absent k (`None`? `⊥`? panic?), which is unspecified. The trace model's "at most one per slot per sender per block" is undefined for CosmWasm where a single chain tx can carry multiple `MsgExecuteContract` messages with the same `msg.sender`. The "slot" concept is never defined. B6's existential admits correlation but not causation: an unauthorized writer to `ballots[k]` is admissible if a co-occurring SubmitBallot from `k` exists.

**Consensus fix**: add to the trace-model paragraph: "for absent k, `ballots[k]` lifts to `⊥`/`None`; B6 quantifies over `ballots.keys ∪ next.ballots.keys` with `Option<Vec<u8>>` comparison." Replace "transactions" with "contract messages (`ExecuteMsg` payloads)" and drop "slot." Strengthen B6's existential to assert *uniqueness*: the named tx is the unique writer to `k` in the firing transition.

### T7. Borsh canonical encoding is under-specified at the `Addr` / `Nat` leaf level [SERIOUS — 3 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 9 | serious |
| kimi | behaviors-types #2 (overlaps T5; same root cause one level down) | critical |
| gpt-oss | temporal-invariants #2 (B8 clause c byte-encoding ambiguity) | under-spec |

**Consensus**: the `canonical_serialization` paragraph pins to Borsh and defines structural composition (`borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`) and field-order/declaration-order, but does NOT pin concrete Borsh encodings for `Addr` (bech32-string? raw bytes? length-prefixed UTF-8?) or `Nat` (u64? u128? endianness?). Two compliant implementations can pick different leaf encodings → different B8(c) digests → attestation mismatch.

**Consensus fix**: pin `Addr` to "Borsh `String`: u32-LE length prefix + UTF-8 bytes of bech32 representation"; pin `Nat` to a concrete uXX with explicit endianness (u128-LE). Or reference an external Borsh schema document by version.

### T8. Section 6.2 de-retraction status block over-claims substrate readiness; `commitHashE` consumer-count is wrong [SERIOUS — 3 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 6 (de-retraction composition fragility); Attack 12 (commitHashE consumer-count) | serious / serious |
| kimi | trust-quartz #1 (de-retracted block over-claims); trust-quartz #6 (commitHashE Fintype mismatch) | critical / serious |
| gemma | trust-quartz #1 (Unverified lynchpin assumption); #2 (Unanchored Quartz inheritance); #3 (binding hash fragmentation) | critical / serious / serious |

**Consensus**: the §6.2 status block claims "the retraction has been resolved" but the very same block names Quartz Asks 6+7 as documented-not-enforced, including "7 of 8 lifts over-bundled" — without disclosing whether `cross_component_session_bind_negl` (the lift verified-rcv inherits from) falls in the 7-overbundled or 1-properly-bundled cohort. Verified-rcv has not independently audited the 8 Quartz commits cited. The `commitHashE` "NOT CONSUMED" note is honest about the theorem but wrong about the interface: verified-rcv consumes the `UserData` slot semantics from Quartz under a different name (`Adv_commitTally_CR`), and the compose-ledger's "consumer count zero" hides this structural dependency. Additionally, `commitHashE`'s `[Fintype UserData]` carrier does not apply to verified-rcv's variable-length input space — a technical incompatibility the note omits.

**Consensus fix**: (a) downgrade the status banner to `[partially de-retracted]`; (b) name the Quartz cycle index of `cross_component_session_bind_negl` and disclose its Ask-6 bundling status; (c) mark the verified-rcv compose-ledger entry as `inheritance: provisional-pending-Quartz-ask-6` if applicable; (d) restate `commitHashE` non-consumption with both reasons (pigeonhole-impossible theorem + `Fintype` carrier mismatch) and explicitly note that the *UserData slot interface* is consumed structurally even though the theorem isn't.

### T9. B9 is mis-tagged and quantifier-malformed as a cryptographic claim [SERIOUS — 3 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 13 (probability not well-formed without ∀ PPT 𝒜 / ∀ n quantifiers) | serious |
| kimi | temporal-invariants #5 (temporal tag on probabilistic claim); #7 (Adv_circuit_eq is correctness not advantage) | serious / serious |
| gpt-oss | temporal-invariants #4 (overlaps with T2 on precondition + tag) | ambiguity |

**Consensus**: B9 is tagged `temporal` but its body is a probabilistic meta-security statement, not an LTL property. The Pr[…] has no explicit quantifier structure (no `∀ PPT 𝒜, ∀ n` or `∃ negl`). `Adv_circuit_eq` is a binary correctness assumption (the circuit either correctly encodes the TDX-quote-validity statement or it doesn't), not a probabilistic advantage decreasing with `n` — including it in a sum of negligible functions collapses correctness into security. The threat model (what the adversary controls) is also unstated.

**Consensus fix**: retag B9 to `meta-security` or `probabilistic` (extend the tag enumeration as already needed for `cross-layer` / `off-chain`). Restate with explicit quantifiers: `∀ PPT 𝒜 controlling chain mempool + publish_result payloads, ∀ n, image_registration_honest(σ) ⇒ Pr[B8 violated at σ→σ′ by 𝒜] ≤ ...`. Remove `Adv_circuit_eq` from the negligibility sum; state it as a standalone correctness hypothesis (the discharge path stays — `reference DCAP circuit-equivalence theorem` — but it's not a probability summand).

### T10. B8 clause (d) "matches the registered enclave image" is ambiguous; phrasings drift across §3.2 / Block 6 / §6.1 [SERIOUS — 3 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| claude | Attack 11 | cosmetic-leaning-serious |
| kimi | temporal-invariants #8 | serious |
| gpt-oss | temporal-invariants #3 | under-spec |

**Consensus**: three different phrasings ("registered", "expected", "the chain stores") for what should be one on-chain registry. "Matches" is undefined (exact equality? prefix? hash?). MRTD/RTMR slash-notation suggests pairing but B8(d) treats it as one. No explicit failure mode for MRTD/RTMR registry tampering separate from vkey tampering (the §6.1 bullet treats them as one).

**Consensus fix**: define a single concrete on-chain slot `(vkey: Bytes32, mrtd: Bytes32, rtmr: Bytes32)`, replace "matches" with exact equality on the relevant components, and use the same noun phrase ("registered verified-rcv enclave image") everywhere. Split §6.1's registration-integrity bullet into separate vkey-integrity and MRTD/RTMR-integrity bullets so partial-compromise scenarios are independently named.

### T11. Failure-mode coverage gaps: KMS unavailability, enclave resource exhaustion, ZK module bug, block-time regression [SERIOUS — 3 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | failure-modes #5 (KMS at instantiation); #6 (resource exhaustion); trust-quartz #3 (ZK module bug); trust-quartz #7 (block-time regression) | serious × 4 |
| gpt-oss | failure-modes #1 (KMS DoS); #2 (collision-binding break); #5 (enclave timeout) | serious × 3 |
| gemma | failure-modes #1 (`publish_result` liveness/integrity boundary); #2 (`submit_ballot` liveness/integrity) | serious × 2 |
| (gpt-oss also flags §4.8 "not a failure" placement) | failure-modes #6 | serious |

**Consensus**: §4 lacks failure modes for at least four load-bearing scenarios: (a) DstackKeyManager unavailable at instantiation → `instantiate` fails; (b) enclave resource exhaustion during tabulation on large candidate sets → enclave aborts without attestation; (c) ZK module verification algorithm bug → `ProofVerifyGnark` returns false on valid proofs or true on invalid ones; (d) block-time non-monotonicity (proposer regression) → derived state oscillation breaks B3/B4. Additionally, §4.8 "not a failure" consumes a numbered slot in the failure-mode list, breaking automated extraction; content belongs in §5 (Non-Goals).

**Consensus fix**: add four new failure-mode sub-sections (KMS unavailability at instantiation; enclave resource exhaustion; ZK module algorithmic bug; block-time regression). Move §4.8's content to §5 and renumber if desired.

### T12. Scenario witnesses §8.1–§8.6 don't cover B1, B2, B3, B7; §8.6 doesn't actually witness B6; B10 is in §8.7 not §8.1–§8.6 [SERIOUS — 2 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | scenarios #3 (B1/B2/B3/B7 missing); #4 (§8.6 doesn't exercise B6's write-attribution because no write occurs); #5 (§8.4 incomplete B4 — only lower bound); #6 (B10 outside slice); #7 (§8.3 witnesses no B-series) | serious × 5 |
| gemma | scenarios #1 (B1/B2/B3/B7 coverage gap); #2 (B10 coverage gap); #3 (B1/B5/B7 witness completeness) | serious × 3 |

**Consensus**: the scenarios slice fails to deliver on its advertised witness coverage. §8.6 ("Impersonation rejected") shows a rejection (no ballot is written), which makes B6's antecedent false and the implication vacuously satisfied — it doesn't exercise B6's load-bearing content. §8.4 only shows the pre-`start_at` rejection, missing the post-`end_at` rejection that B4 also bounds. §8.3 (deadlock) consumes no B-series invariant. B10 is in §8.7, not §8.1–§8.6.

**Consensus fix**: (a) add §8.X "Result immutability" (B1 + B7 witness via post-Resolution mutation attempt rejected); (b) add §8.X "Ballot store frozen after end_at" (B2); (c) add §8.X "Premature tally rejected" (B3); (d) retitle §8.6 as a Block-3-Requires witness or add a positive B6 witness alongside it; (e) extend §8.4 to cover post-`end_at` rejection too; (f) update the slice's advertised coverage to point at §8.7 for B10.

### T13. §6.6 output contract overstates categorical guarantees; well-formedness vs semantic correctness gap omitted [SERIOUS — 2 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | scope #1 (output contract states categorical reliance when B9 requires `image_registration_honest`); #4 (well-formedness vs correctness gap) | serious / serious |
| gemma | scope #2 (Over-specification of Output Reliability) | serious |

**Consensus**: §6.6 tells consumers they "can rely on" B8's attestation binding and the S6–S9 well-formedness invariants. Two problems: (i) the reliance is unconditional, but B9's bound is conditioned on `image_registration_honest` (T2); if 4.3b fires, the guarantee voids and §6.6 doesn't warn. (ii) §2.5 explicitly distinguishes syntactic well-formedness (enforced by the chain) from semantic correctness (enforced by the enclave software); §4.4 names "enclave software bug" as an in-scope failure where the chain accepts a well-formed but incorrect tally. §6.6 doesn't surface this — a consumer reading only §6.6 would believe the published tally is semantically correct.

**Consensus fix**: condition the §6.6 bullets on `image_registration_honest`. Add an explicit bullet to "What callers cannot infer": "That the tally is semantically correct (equals `Tally_spec` applied to the frozen ballots). Well-formedness is syntactic; failure mode 4.4 admits well-formed-but-incorrect tallies."

### T14. AEAD missing from Stage 1 — adversarial-ciphertext soundness gap [SERIOUS — 2 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | off-chain-witness #3 (Step 1 lemma stronger than its own caveat admits) | serious |
| gemma | off-chain-witness #3 (Hand-waving of Stage-1 Soundness for Adversarial Inputs) | serious |

**Consensus**: §8.7 step 1's caveat admits that `Ecies.roundtrip` proves only honest-ciphertext roundtrip, not adversarial-soundness. The Stage-1 spec defines the decoder as "ECIES-decrypt → parse → validate as permutation" — no AEAD layer. A malicious voter can craft a ciphertext that decrypts to a valid-looking permutation under the enclave's key but was never their honest ballot. Step 1's lemma "implementation returns the partition matching the spec" is, as stated, unsound without an honest-ciphertext precondition.

**Consensus fix**: add an honest-ciphertext precondition to the Stage-1 lemma, OR strengthen Stage 1 with an AEAD layer (e.g., wrap the ECIES with a per-candidate-key MAC keyed by the candidate's secret), and update B6's load-bearing role accordingly (Block 3's `msg.sender ∈ candidates` is *not* sufficient to bind ciphertext to sender intent).

### T15. §4.9 deadlock detection assumes unstated enclave retry behavior [SERIOUS — 2 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | failure-modes #7 | serious |
| gemma | failure-modes #3 (Soundness/Liveness mismatch) | serious |

**Consensus**: §4.9's claim of "infinite rejection loop" + detection via "repeated `InvalidTally` rejection events" presumes the enclave retries `publish_result` indefinitely after rejection. Block E1 specifies no retry policy. A one-shot enclave produces exactly one `InvalidTally` event and then silence → silent deadlock, detection mechanism fails.

**Consensus fix**: amend Block E1 to specify the enclave's retry discipline ("the enclave retries `publish_result` indefinitely with documented backoff until success or operator intervention").

### T16. Severity-tag taxonomy is undefined for "integrity-per-voter", "canonicality", etc. [COSMETIC — 3 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | failure-modes #3 (undefined severity classes) | serious |
| gpt-oss | failure-modes #4 (ambiguous severity tagging §4.5) | ambiguity |
| gemma | failure-modes #4 (Misplaced severity for voter address compromise) | cosmetic |

**Consensus**: §4 mixes "confidentiality / integrity / liveness" with ad-hoc tags like "integrity-per-voter" and "canonicality" that aren't defined anywhere. Methodology hygiene issue, not a soundness issue.

**Consensus fix**: enumerate the C/I/L base classes in §4's preamble; map every sub-tag explicitly ("integrity-per-voter" → "integrity with per-voter scope"; "canonicality" → either "liveness (no single agreed state)" or "integrity (different forks admit different winners)" with a split if needed).

### T17. Tag taxonomy expanded silently: `cross-layer`, `off-chain` not defined in §3.2 preamble [COSMETIC — 2 voices]

| Voice | Slice / Attack | Severity |
|---|---|---|
| kimi | temporal-invariants #10 | cosmetic |
| (claude touches this implicitly in Attack 13 about B9 tag) | | — |

**Consensus**: §3.2 preamble enumerates `state`/`temporal` only. B10 is tagged `cross-layer` and B10_lean is `off-chain`. Silent expansion; methodology contract not updated.

**Consensus fix**: §3.2 preamble must enumerate `state`, `temporal`, `cross-layer`, `off-chain`, `meta-security` (per T9), each with its discharge discipline. Section 3.1 has its own preamble shift owed by T4.

## Section B: False positives identified

Single-voice findings other voices implicitly refuted or didn't surface, screened for plausibility:

### F1. `ballots@end_at` notation is undefined [claude Attack 10] — single-voice

Claude flagged `ballots@end_at` in B10's RHS as undefined notation. Other voices read B10 and didn't flag this — most accepted the contextual reading "the value of `state.ballots` at the moment block-time first reached `end_at`." Claude is correct that the notation is informal and would not pass type-checking in a downstream Lean spec. **Disposition**: keep in the revision punch list as a serious editorial item (define the notation in §2.5 or replace with `state.ballots` and rely on B2). Not a false positive but not multi-voice-confirmed either; promotes from "single-voice but deep."

### F2. SemVer header mis-classifies v0.3.0 as MAJOR [claude Attack 1] — single-voice

Claude's argument: every v0.3.0 change strengthens or clarifies; nothing is removed or weakened by the rubric's MAJOR definition. The B9-conditioning change is the borderline case (conditioning narrows the unconditional claim) but the doc itself describes the prior bound as "unconditionally false," which would be MINOR or PATCH. **Disposition**: methodology-relevant but single-voice; the rubric in the SemVer header is one of v0.3 candidate Ask N (now `methodology-v0.4-candidates.md`). Worth fixing in the revision but doesn't block intent verdict.

### F3. "smart contract" identity understates off-chain scope [kimi scope #2] — single-voice

Kimi flagged §1's "Name: verified-rcv — instant-runoff voting smart contract" as understating the actual system. Other voices read §1 and didn't flag this. The §1 scope bullet immediately below names all three components; a reader who stops at the name bullet would be misled but the doc-as-a-whole isn't. **Disposition**: cosmetic editorial; keep in punch list at low priority.

### F4. "Caller contract `is trusted` is toothless" [kimi scope #5 cosmetic, gemma scope #1 serious] — disputed severity

Voices disagree on severity. Both agree the phrasing is misleading but kimi calls it cosmetic and gemma calls it serious (Over-specification of Caller Trust). **Disposition**: methodology-disagreement-worthy (Section C below); the fix is the same (rephrase to "is unverified"), the severity is judgment.

### F5. §4 preamble contradicts §4.6 on recoverability [kimi failure-modes #1 critical] — single-voice

Kimi flagged a clean internal contradiction: §4 preamble says "no failure here is auto-recoverable from within the election instance" while §4.6 says "Enclave re-publishes once reconnected... retries are safe." Other voices read §4 and didn't surface this. **Disposition**: ground-truth-true contradiction (verified by reading the text); not a false positive — promote to the revision punch list as a critical-on-its-merits even though single-voice. The other voices likely missed it due to slicing.

### F6. Stale "5-summand" reference in §8.1 [kimi scenarios #1 critical] — single-voice

Kimi flagged §8.1's text "trustworthiness reduces to the 5-summand negligibility budget" against §3.2's "4-summand bound." Direct numerical inconsistency. **Disposition**: ground-truth-true; promote to revision punch list.

### F7. B8 clause (c) 64-byte vs 32-byte equality [kimi temporal-invariants #1 critical] — single-voice

Kimi flagged B8(c)'s `attested user_data = SHA-256(...)` as type-inconsistent because the implementation note says the digest is "embedded in the lower half of Quartz's 64-byte UserData slot." A 64-byte slot ≠ a 32-byte digest. Other voices read B8 and didn't surface this; they may have charitably read the equality as "matches" or "embedded equals". **Disposition**: ground-truth-true once you read the text strictly; promote to revision punch list as serious-clarification (the fix is to restate clause (c) as "the lower 32 bytes of `attested.user_data` equal SHA-256(...)").

## Section C: Methodology disagreement worth surfacing (not action items)

### D1. Severity calibration on S5 — serious vs critical

Mistral elevated T4 (S5 not pointwise) to **critical**; claude / kimi / gemma / gpt-oss called it serious. Mistral's reading: "the spec encodes a temporal invariant inside the state-shape section, violating the preamble" — a soundness-equivalent claim. Other voices read the same surface but treat the discharge complement (handler-set inspection) as substantively correct, mis-located only. This is a real depth-of-attack divergence. The consensus methodology call is serious-with-critical-shadow (T4 above). Worth noting: mistral as a *non-reasoning model* committed harder to the soundness reading than the reasoning models did. May reflect mistral's lower hedging tendency rather than a content disagreement.

### D2. F4 caller-contract phrasing — cosmetic vs serious

See F4 above. Two voices flag the same issue at different severities. The methodology recommendation is to bias toward the more severe reading when phrasing-of-trust is involved (trust phrasings drift into downstream specs); but the fix is identical, so the severity tag is mostly for prioritization.

### D3. Scope of "structured behavior block" attacks vs "type signatures" attacks

Kimi behaviors-types slice surfaced 6 attacks; gpt-oss behaviors-types slice surfaced 6 attacks; gemma behaviors-types slice surfaced 4. The overlap is *narrow*: all three converged on Map iteration determinism + EnclaveImage missing definition, but each voice surfaced its own additional attacks. This is methodology-healthy — the slice covers a lot of substrate and per-voice attention is needed for depth. Suggests that 3-voice minimum on dense-substrate slices is operationally about right.

### D4. Gemma flagged S10 invariant under timestamp re-orgs separately

Gpt-oss state-invariants #4 (S10 under blockchain timestamp re-orgs) overlaps with kimi trust-quartz #7 (block-time monotonicity has no failure mode), but they're scoped differently. Gpt-oss reads S10's *truth value* under re-orgs (could regress); kimi reads the *failure mode coverage*. Both are sound. The fix (T11.d above) addresses both.

## Section D: Revision punch list for v0.3.1

In priority order (critical → serious → cosmetic, multi-voice → single-voice within tier):

**Critical (4):**

1. **Define `EnclaveImage` symbol with explicit type signature** (T1, 5 voices). §2.5 introduces the typed symbol; §3.2 B10_lean and §8.7 step 3 reference *that* definition. Image-identity-binding (§8.7 step 4) explicitly ties the registered binary's MRTD/RTMR to the source that extracts to `EnclaveImage`.
2. **Define `image_registration_honest` as a state predicate** (T2, 5 voices). §6.1 supplies the formal predicate; B9 is rewritten with explicit `∀ PPT 𝒜, ∀ n` quantifier structure (folds T9).
3. **Add `dstack-kms-trust` + `input-fidelity` to the B10 decomposition** (T3, 4 voices). §8.7 closing pattern becomes 4-or-5 conjunct; a new numbered step witnesses input-fidelity (TDX-reported input hash inclusion, or chain-inclusion proof).
4. **Pin `Tally_spec` to a deterministic function** (T5, 4 voices). §2.5 Stage 1 takes `Vec<(Addr, Ciphertext)>` in chain-deterministic order (or pin `dropped_voters` to candidate-declaration-order restricted to keys); concrete Rust type for `Map<Addr, Nat>` fields is `Vec<(Addr, Nat)>`.

**Serious (12):**

5. **Resolve S5's location** (T4): move to §3.2 as a B-series temporal write-discipline OR amend §3.1 preamble to admit handler-set inspection.
6. **Fix B6 trace-model semantics** (T6): define absent-key lift; replace "transactions" with "contract messages"; drop "slot"; strengthen existential to uniqueness.
7. **Pin Borsh leaf encodings for `Addr` / `Nat`** (T7).
8. **Downgrade §6.2 status banner to `partially de-retracted`** (T8): disclose `cross_component_session_bind_negl`'s Ask-6 bundle status; restate `commitHashE` non-consumption with both reasons; mark verified-rcv inheritance of the UserData slot interface.
9. **B9 quantifier + tag fix** (T9, folded with T2): retag `meta-security`; explicit `∀ PPT 𝒜, ∀ n`; remove `Adv_circuit_eq` from negligibility sum, restate as correctness hypothesis.
10. **B8(d) registry definition + consistent phrasing** (T10): define `(vkey, mrtd, rtmr)` as one on-chain slot; replace "matches" with component-wise equality; split §6.1 registration-integrity into vkey-integrity and MRTD/RTMR-integrity.
11. **Add four failure modes** (T11): KMS unavailability at instantiation; enclave resource exhaustion; ZK module algorithmic bug; block-time regression. Move §4.8 to §5.
12. **Add missing scenarios** (T12): B1+B7, B2, B3 witnesses; B10 cross-reference to §8.7; retitle/extend §8.4 and §8.6.
13. **Condition §6.6 on `image_registration_honest`** (T13); add "What callers cannot infer: semantic correctness."
14. **AEAD or honest-ciphertext precondition for Stage 1** (T14).
15. **Specify enclave retry policy in Block E1** (T15).
16. **Restate B8(c) as lower-32-byte equality** (F7).
17. **Fix §4 preamble + §4.6 contradiction** (F5).
18. **Fix §8.1 "5-summand" → "4-summand"** (F6).

**Cosmetic-ish (5):**

19. **Define severity-tag taxonomy** (T16): C/I/L base + named sub-tags.
20. **Define tag taxonomy in §3.2/§3.1 preambles** (T17): include `cross-layer`, `off-chain`, `meta-security`.
21. **Define `ballots@end_at`** or replace with `state.ballots` + reliance on B2 (F1).
22. **SemVer label review** (F2): reclassify v0.3.0 as MINOR or expand MAJOR rubric to admit "rewriting a previously-vacuous invariant."
23. **Identity wording** (F3): name the system, not the contract.

## Verdict

**BREAKS-AGAIN.** Verdict aggregated as: SURVIVES requires zero criticals + zero major serious gaps. v0.3.0 has 4 cross-voice-confirmed criticals and 12 cross-voice-confirmed serious findings. The criticals are not editorial — they touch the load-bearing core of B10 (the central correctness obligation) and B9 (the central probabilistic claim).

**Per-voice verdict roll-up**:

| Voice | Verdict |
|---|---|
| claude (full-spec) | BREAKS-AGAIN |
| kimi-k2-6 (9-slice) | 9× BREAKS-AT-SLICE |
| mistral (1-slice smoke) | BREAKS-AT-SLICE |
| gpt-oss (4-slice partial) | 4× BREAKS-AT-SLICE |
| gemma (8-slice, 1 abandoned) | 8× BREAKS-SLICE |

Six voices, zero SURVIVES. No close call.

**Revision-to-v0.3.1 next step**: apply criticals 1-4 + seriouses 5-18; cosmetic items 19-23 in same pass if scope-cheap. Re-attack v0.3.1 with the same panel under Pattern B sequential dispatch. Goedel-class excluded; glm-4-7-flash excluded; gpt-oss may be downgraded to "best-effort" if gateway transient outages persist. Estimated revision scope: ~10-15K-token diff against current intent.md; ~25K target intent size after fixes.

The verdict is decisive enough that the v0.3.1 revision can proceed without further triangulation. Mistral, gpt-oss, and gemma did not need to complete their full slice panels to reach this conclusion — the 4 confirmed-critical themes are independently load-bearing.
