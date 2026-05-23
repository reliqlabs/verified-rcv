# Synthesis — verified-rcv revised intent, Round 3a re-adversarial pass

- **Run ID**: `intent-revised-2026-05-14T200029Z`
- **Target**: `.colosseum/intent.md` (revised; 700 lines after sanity-pass fixes)
- **Voices dispatched**: 8 — 7 completed structured reports, 1 errored
- **Synthesis written**: 2026-05-15, in-session (Claude Code, Opus 4.7)
- **Consensus verdict**: **BREAKS-AGAIN** — 7/7 surviving voices vote BREAKS, multi-voice criticals exist

This is orchestrator output (not a model report). It's the structured digest of the 7 verbatim per-voice reports in this run dir.

---

## 1. Voice roster + verdict

| Voice | Harness path | Elapsed | Finish | Verdict | Visible content |
|---|---|---|---|---|---|
| claude-opus-4-7 (Agent subagent, file-access) | claude-code | 339s | stop | **BREAKS-AGAIN** | 27 KB, 3 critical + 6 serious + 3 cosmetic |
| mistral-small-4-119b-2603 | lm-studio local | 211s | stop | **BREAKS** | 17 KB, 3 critical + 6 serious + 1 cosmetic |
| qwen3.6-27b-mlx | lm-studio local | 864s | stop | **BREAKS** | 11 KB, 3 critical + 3 serious |
| google/gemma-4-26b-a4b | lm-studio local | 2455s | length | **BREAKS** | 102 KB nominal — usable content only first ~220 lines (after which model entered degenerate repetition loop), 2 critical + 2 serious |
| kimi-k2-6 | gateway (Moonshot) | 133s | length | **BREAKS** (implicit from content; truncated mid-thought) | 35 KB raw reasoning stream, 12 attacks visible — model truncated before producing a final verdict line |
| glm-4-7-flash | gateway (Zhipu) | 84s | stop | **BREAKS** (template-echo header; content unambiguous) | 8 KB, 3 critical + 1 serious |
| gpt-oss-120b | gateway (OSS) | 53s | stop | **BREAKS-AGAIN** | 15 KB, 3 critical + 7 serious |
| goedel-prover-v2-32b | lm-studio local | 852s | length | **ERROR** | Degenerated into tautology loops — theorem-prover specialist unsuited for general adversarial review |

Failure-shape observations recorded in `gateway-bugs-2026-05-14.md`: gateway-wide ~240s Cloudflare cap drove kimi to truncation; Anthropic-route Cloudflare 524 at ~127s (Bug 4) prevented gateway-claude voices entirely (Claude voice ran via Agent subagent instead, which is the methodology-preferred path anyway).

---

## 2. Overlap matrix — findings ordered by multi-voice support

Voices marked **C** (critical), **S** (serious), **c** (cosmetic) per that voice's own severity rating. Themes ordered by descending number of voices flagging the same underlying issue.

### Theme 1 — **B9 vacuity / inherited-from-retracted-substrate** (7/7 voices)

| Voice | Finding |
|---|---|
| claude | #4 (S) — B9 LHS preconditional over-strength: dropped `Adv_image_registration` summand without conditioning LHS on `image_registration_honest`, so bound is *false* given 4.3b path |
| mistral | #3 (S) — B9 not a well-formed temporal property; RHS terms undefined |
| qwen | #3 (C) — B9 inherits retracted Quartz substrate; mathematically empty |
| glm | #2 (C) — B9 vacuous; bound's components retracted or undefined |
| gpt-oss | #5 (S) — bound rests on retracted Quartz theorems; ≤ undefined is vacuously true |
| kimi | attack 3 + 4 — disjunction-vs-decomposition collapse + impossibility-hypothesis-vacuity |
| gemma | #1 (C) — **triviality**: "Pr ≤ undefined" is meaningless |

**Consensus**: B9's current formulation cannot survive intact. Possible defenses (all voices converge on a subset of these):
- (a) restate B9 conditioned on `image_registration_honest` (Claude #4)
- (b) name the summands as verified-rcv-internal hardness assumptions decoupled from Quartz's retracted lifts (gpt-oss, glm, qwen)
- (c) drop B9 until Quartz content-phase lands (multiple voices' fallback)
- The Quartz def-tying refactor that landed today (per the Quartz-agent feedback) addresses the upstream retraction; Section 6.2 status block is now stale and the de-retraction is the natural path forward

### Theme 2 — **B10 chain-side / off-chain mis-classification** (6/7 voices flag, 1 explicitly disagrees)

| Voice | Finding |
|---|---|
| claude | #9 (S) — B10's RHS references `enclave_privkey`, which is not in chain state; implicit off-chain dependency unaccounted for |
| mistral | #9 (S) — B10's witness is entirely off-chain, doesn't satisfy methodology's trust model for a chain-side temporal invariant |
| qwen | #2 (C) — `enclave_privkey` not a chain state variable; chain cannot evaluate B10 |
| glm | #3 (C) — temporal-state-mismatch; B10 is a property of enclave computation, not chain state evolution |
| kimi | attack 6 (S) — `enclave_privkey` referenced as free variable in chain-side invariant |
| gpt-oss | #10 (S) — `ballots@end_at` only B2-guaranteed, not state-evaluable before `end_at` |
| **gemma** | **Explicitly verified B10 is correct** — "Tally_spec arity matches; B10 is a temporal property" |

**Note on disagreement**: Gemma's "B10 is fine" claim is shallower than the others' critiques. Other voices accept B10's syntactic shape but flag that the *witness path* (Section 8.7) is off-chain by construction while the invariant lives in Section 3.2's chain-side table. The Section 8.7 fix from the prior sanity-pass acknowledged this but didn't resolve the typing tension. The methodology-correct shape is probably: factor B10 into a Lean-side `B10_lean` (image-IO obligation) and a chain-side `B10_chain` (`tally_result = Some(t) → ∃ image-binding ∧ B8 verified for t`), then prove `B10_lean + B10_chain ⇒ B10`. Claude #9's existential / Section-6.3-dependency framing is the cleanest concrete fix.

### Theme 3 — **B6 existential under-specified / not bound to firing transition** (6/7 voices flag, 1 explicitly disagrees)

| Voice | Finding |
|---|---|
| claude | #3 (C) — `∃ tx, ...` doesn't bind `tx` to the actual `next.ballots ≠ ballots` transition; satisfied by any historical SubmitBallot write |
| mistral | #1 (C) — mis-tagged as state in prior draft (mostly addressed by re-tag, but mistral notes the formulation remains weak) |
| qwen | #5 (S) — references `tx.kind` and `tx.msg.sender` which aren't part of the state model; transaction trace not defined |
| glm | #1 (S) — last-write-wins overwrite scenario: when B overwrites A's ballot, B6's existential is satisfied by *any* prior SubmitBallot, so the invariant is too weak to detect the overwrite-by-different-sender corruption |
| gpt-oss | #1 (C) + #7 (S) — existential is still "state-only in disguise" because `tx` is undefined in the state model; needs ∀-per-key formulation |
| kimi | attack 2 (S) — `∃ tx` quantifier doesn't give attribution for *all* changes; multiple-tx-per-block case violates implicit intent |
| **gemma** | **Explicitly verified B6 is correct** — "sophisticated and correct observation; correctly tagged temporal" |

**Consensus fix**: convert B6 from `(next.ballots ≠ ballots → ∃ tx, ...)` to a `∀-per-key` form binding the transition: `∀ k, next.ballots[k] ≠ ballots[k] → ∃ tx, fires(tx) ∧ tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`. This requires introducing an explicit transaction-trace model in the spec (kimi + qwen + gpt-oss converge). Gemma's "B6 is fine" agrees the *intent* is captured by the re-tag but doesn't engage with the existential-strength argument the others make.

### Theme 4 — **S5 mis-tagged as state (quantifies over successor states)** (4/7 voices)

| Voice | Finding |
|---|---|
| claude | #1 (C) — Section 3.1's preamble forbids "quantification over operations or time" but S5 says "any successor state σ′ has tally_result = σ.tally_result" — that's an explicit temporal quantifier |
| mistral | #7 (c — cosmetic) |
| gpt-oss | #2 (S) — should move to behavioral invariants table or be explicitly tagged as a corollary of B1 |
| kimi | attack 1 (S) |

**Consensus fix**: either (a) restate S5 as a *type-shape* property (`tally_result` is set-once-then-immutable) without successor-state quantification, or (b) move S5 to Section 3.2 with a `temporal` tag and an explicit `derived-from-B1` annotation. The current annotation "Derived from B1 (temporal causal version); listed for explicit state-shape reference per Round 3a first-pass adversary Attack 26" is a *defense* but doesn't fix the formulation, which still violates Section 3.1's gating rule by its plain words.

### Theme 5 — **Section 6.2 inheritance table vs. retracted-substrate status block contradicts itself** (3/7 voices)

| Voice | Finding |
|---|---|
| glm | #4 (C) — direct contradiction: table claims valid inheritance, status block immediately below says claims are retracted |
| gpt-oss | #4 (C) — DstackKeyManager row was correctly withdrawn, but B8 still depends on it; if removed, composition is unprovable |
| gemma | #3 (S) — inheriting "retracted/content-free" claims is not inheritance; it's a placeholder |

**Consensus fix**: with Quartz's def-tying refactor having landed, the retracted-substrate banner is *now stale*. Update Section 6.2 to either (a) reflect the de-retraction with a new status block citing the Quartz cycle-6.4-through-6.11 commits, or (b) if waiting for upstream PR to be reviewed, keep the substrate-status block as historical and add a "current state: de-retraction in flight" subblock. Closely related to the Quartz-agent feedback you relayed about Asks 6+7.

### Theme 6 — **`canonical_serialization` undefined** (3/7 voices)

| Voice | Finding |
|---|---|
| mistral | #10 (S) |
| qwen | #4 (S) |
| kimi | attack 10 (S) |

**Consensus fix**: pin a specific serialization scheme. Borsh (CosmWasm-native) is the natural choice; alternatively a JSON canonicalization with lexicographic key ordering. Adds one paragraph to Section 2.5 or Section 6.6.

### Theme 7 — **B9 missing operational/vkey-substitution disjunct** (3/7 voices, including some Theme-1 overlap)

| Voice | Finding |
|---|---|
| claude | #4 (S, same as Theme 1 entry) — B9's LHS is unconditional, so 4.3b path violates B8 outside the budget |
| qwen | #6 (S) — disjunction-vs-decomposition: B9 omits operational disjunct |
| kimi | attack 3 (covers same ground) |

**Consensus fix**: condition B9 on `image_registration_honest` precondition (Section 6.1 trust boundary). This is the cleanest formal fix; matches Claude's #4 specific suggestion.

### Theme 8 — **`commitHashE` row in Section 6.2 is operationally dead** (2/7 voices)

| Voice | Finding |
|---|---|
| claude | #5 (S) — verified-rcv consumes `Adv_commitTally_CR` (SHA-256), not Quartz's `commitHashE` (pigeonhole-impossible); the row should be removed or annotated as not-consumed |
| gemma | #2 (C) — the SHA-256 "lift" is anchored in a vacuous Quartz axiom (commitHashE is pigeonhole-impossible) |

**Consensus fix**: remove or annotate the `commitHashE` row in Section 6.2 inheritance table. Gemma's stronger version: ensure no downstream proof obligation actually consumes the Quartz `commitHashE` bundle — only the verified-rcv-internal SHA-256 collision-resistance hypothesis (`Adv_commitTally_CR`).

### Theme 9 — **Stage 1 decryption-soundness needs more than `Ecies.roundtrip`** (1/7 — Claude alone)

| Voice | Finding |
|---|---|
| claude | #7 (S) — `Ecies.roundtrip` only proves honestly-encrypted plaintext roundtrips. The Stage-1 lemma needs an *authenticated-decryption* property (AEAD / unforgeability under chosen ciphertext) to bound adversarial ciphertexts. If the ECIES variant isn't authenticated, an adversary can submit a ciphertext that decrypts to a different valid permutation than what they intended. |

**Single-voice but deep**: file-access-grounded; Claude actually grep'd Quartz's Ecies.lean and confirmed only `roundtrip` exists upstream. Either (a) specify authenticated ECIES, (b) move the security claim from "decryption correctness" to "deterministic decrypt-or-drop", or (c) flag as a v0.3 upstream ask on Quartz.

### Theme 10 — **Section 8.7 Step 3 mis-labels Lean-internal composition as "B10"** (1/7 — Claude alone)

| Voice | Finding |
|---|---|
| claude | #6 (S) — Step 3 says "by composition of Stage-1 + Stage-2, image-IO = Tally_spec, which is what B10 asserts." But B10 is a chain-side temporal property; the Lean composition gives Lean-B10' (image-IO equality), and B10 follows only with image-identity binding (Step 4, demoted to "not a Lean theorem") + B8. |

**Single-voice but methodology-sharp**: connects to Theme 2. Fix: rename Step 3 to "Lean-internal composition: image-IO = `Tally_spec`" and add an explicit B10 ← Lean-comp ∧ image-binding ∧ B8 step.

### Theme 11 — **IRV `IRV_spec` Step 1 base case subsumes the edge case** (1/7 — Claude alone)

| Voice | Finding |
|---|---|
| claude | #8 (S) — Step 1 base case ("if all remaining have equal first-place counts → co-winners") subsumes the edge-case bullet ("all remaining candidates tied for lowest → co-winners"). Spec writers porting to Lean/Quint will either duplicate or risk dropping a witness. |

**Single-voice; concrete editorial fix.**

### Theme 12 — **Section 4.9 enclave determinism is unstated requirement** (2/7 voices)

| Voice | Finding |
|---|---|
| mistral | #6 (S) — assumes enclave deterministic without stating it; non-deterministic enclave could escape the deadlock |
| kimi | attack 9 (S) — same |

**Consensus fix**: add "enclave image is deterministic over `(raw_ballots, candidates, privkey)`; no external entropy, no timing dependencies" as a Section 6.3 trust assumption or as an explicit Block E1 requirement.

### Theme 13 — **Block E1 doesn't specify decrypted-plaintext parsing format** (2/7 voices)

| Voice | Finding |
|---|---|
| kimi | attack 7 (S) — Stage 1 says "parse as Vec<Addr>" but doesn't define the encoding; different enclave implementations could disagree |
| mistral | overlap with #10 (canonical_serialization) |

**Fix**: same Borsh / JSON canonicalization decision as Theme 6; should pin once and reference from both places.

### Theme 14 — **Block 6 well-formedness missing winners-equals-last-round check** (1/7 — kimi alone)

| Voice | Finding |
|---|---|
| kimi | attack 11 (S) — `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)` doesn't ensure `winners = keys(per_round_counts[last])`. A malformed tally could pass well-formedness but have `winners` disconnected from the final round. |

**Single-voice but concrete**: add `winners = keys(per_round_counts[last])` to Block 6 Requires.

### Theme 15 — **B8 clause (c) 32-byte digest in 64-byte UserData slot may not actually equate** (1/7 — kimi alone, truncated before conclusion)

| Voice | Finding |
|---|---|
| kimi | attack 12 (C — truncated) — B8 says `attested user_data = SHA-256(...)`, but UserData is 64 bytes (32-byte digest + 32-byte domain-separation tag per the prior note). The equality cannot literally hold if upper 32 bytes are non-zero. Either the chain checks only lower 32 bytes (B8 mis-stated) or chain rejects all valid attestations (B8 unsatisfiable). |

**Single-voice but worth verifying**: the spec wording is "32-byte digest embedded in the lower half ... upper half reserved for domain-separation tag (implementation detail)". The implementation-detail clause is doing work the formal statement doesn't pick up. Fix: either restate B8(c) as `lower_half(attested user_data) = SHA-256(...)`, or define UserData as the structured pair (digest, tag) and the equality as projection.

### Theme 16 — **Intent vs Spec coverage gap: TEE doesn't remove trust in tallier** (1/7 — gemma alone)

| Voice | Finding |
|---|---|
| gemma | #4 (S) — Section 1 says "TEE + attestation removes trust in any single tallier", but Block 6 only verifies syntax + attestation. An enclave bug producing a well-formed-but-wrong tally is accepted. The TEE doesn't remove trust; it shifts it to enclave software correctness. |

**Honest spec admits this** (Section 4.4 + Section 8.7), but gemma is correct that Section 1's framing oversells. **Editorial fix**: rephrase Section 1 to "TEE + attestation + pre-deployment formal verification of the enclave image removes trust in any single tallier (with the enclave-image-correctness obligation made explicit and discharged off-chain via B10's Lean proof)."

---

## 3. False positives (single-voice findings refuted by other voices or by spec re-reading)

These were surfaced and explicitly contradicted; do **not** treat as required revisions:

| Voice | Finding | Refutation |
|---|---|---|
| qwen | #1 (C) — Section 2.1 arithmetic is wrong (claims A should be 3 in Round 2) | **kimi attack 8 + gemma "final check" + manual recomputation all confirm**: B's ballot was `A > B > C > D > E`, so B's first-place vote was already counted for A in Round 1. Eliminating B doesn't redistribute (top choice unchanged). Spec is correct. Qwen confused the redistribution semantics. |
| mistral | #2 (C) — B10 arity mismatch (claims `ballots@end_at` is decrypted, mismatching `Tally_spec(raw_ballots, ...)`) | **Spec is correct**: `ballots@end_at` refers to the *contract storage map* at `end_at` — i.e., `Map<Addr, Vec<u8>>` (encrypted). `Tally_spec` Stage 1 then does the decryption. Mistral mis-parsed which side of the decryption boundary `ballots@end_at` sits on. |
| gpt-oss | #3 (C) — Block 6 self-contradicts ("emits another event but is harmless" vs `AlreadyResolved → error`) | **Spec is correct**: those two sentences are in *different blocks*. Block 5 (`close_and_tally`) is the idempotent self-loop with the "harmless" comment; Block 6 (`publish_result`) is the outbound transition that rejects with `AlreadyResolved`. Gpt-oss conflated the two blocks. |
| mistral | #5 (S) — B8 is tagged temporal but its content includes instantaneous checks | **Misreading the methodology**: B8's `always (next.tally_result.is_some() ∧ tally_result.is_none() → ...)` is genuinely temporal (a property of transitions). The checks *within* the implication's conclusion happen instantaneously at the firing transition, which is how temporal logic over actions normally works. Other voices treat B8's tag as correct. |
| mistral | #8 (S) — Section 6.2 inheritance table doesn't include TDX-quote-structure or zkdcap-proof-format | **Scope-correct**: the intent doc inherits Quartz's lifts; it's not the place to restate Quartz's own structures. Mistral is asking the doc to be self-contained where the methodology explicitly says it's a *composition* doc. |

---

## 4. Methodology disagreement worth surfacing (not a revision target)

- **Gemma vs the field on B6 + B10**: gemma explicitly affirms B6 and B10 are correctly formulated, while 5–6 other voices flag them. Gemma's reasoning is shallower (just notes "the temporal tag is correct") and doesn't engage with the existential-binding (B6) or off-chain-witness (B10) critiques the others develop. This is a *depth-of-attack* divergence, not a methodology disagreement. The other voices' critiques win on substance.
- **B9 fix path is open**: voices converge on "B9 is broken" but diverge on whether to (a) condition on `image_registration_honest`, (b) restate without Quartz inheritance, or (c) drop entirely. With the Quartz def-tying refactor having landed today, option (d) "absorb the Quartz de-retraction and tighten Section 6.2" becomes the practical winner. This is a methodology call.

---

## 5. Revision punch list — ordered by criticality + multi-voice support

Each entry: priority tier (CRITICAL | SERIOUS | EDITORIAL), voices supporting, concrete edit.

### CRITICAL (multi-voice + load-bearing)

1. **B9 — restate to handle the de-retracted Quartz substrate** (7/7 voices)
   - Replace the Section 6.2 retracted-substrate status block with a *de-retracted* status block citing the Quartz cycle-6.4-through-6.11 commits + Asks 6+7.
   - Condition B9's LHS on `image_registration_honest` (Claude #4); leaves the bound true.
   - Tighten the inheritance row references in 6.2 to the specific Quartz `_negl` lifts post-def-tying (Quartz ledger now identifies them concretely).
   - Decide and document: keep all 4 summands (Adv_tdx, Adv_groth16_KS, Adv_circuit_eq, Adv_commitTally_CR) or restate as verified-rcv-internal hardness assumptions. Recommend keeping inheritance now that the substrate has content.

2. **B6 — bind the existential to the firing transition + per-key universal** (6/7 voices)
   - Restate: `always (∀ k, next.ballots[k] ≠ ballots[k] → ∃ tx, fires(tx) ∧ tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`.
   - Add a "Transaction trace model" note in Section 2.5 explicitly: `tx` is a CosmWasm message with `kind`, `msg.sender`, `payload` fields; downstream Quint spec carries this as a typed action label.

3. **B10 + Section 8.7 — factor into chain-side and Lean-side halves** (6/7 voices)
   - Rename Section 8.7 Step 3 to "Lean-internal composition: image-IO = `Tally_spec`" (call this `B10_lean`).
   - Add explicit `B10 ← B10_lean ∧ image-identity-binding ∧ B8` derivation step.
   - Either (a) move B10's *current* statement (with `enclave_privkey`) to a new Section 3.3 "Cross-layer obligations" with a `cross-layer` tag, leaving Section 3.2 with only chain-evaluable properties; or (b) existentially quantify `enclave_privkey` and link to Section 6.3 dstack-KMS-honesty assumption (Claude #9's preferred form).
   - **Decision needed**: (a) is the more methodology-aligned fix.

4. **S5 — either restate without successor-state quantifier or move to behavioral invariants** (4/7 voices)
   - Recommended: restate as a *type-shape* property in Section 3.1: "`tally_result` is set-once-then-immutable per storage schema (no handler writes after `Some(_)`)" — then B1 carries the trajectory claim.
   - Alternative: move S5 to Section 3.2 as `temporal`, tagged `(derived-from-B1)`.

### SERIOUS (multi-voice or single-voice-but-deep)

5. **Section 6.2 — update for Quartz de-retraction** (3/7 voices, also fold in your Quartz-agent feedback)
   - Replace `[2026-05-14] B8/B9 substrate status` block with `[2026-05-15] B8/B9 substrate de-retraction: Quartz cycle-6.4 through cycle-6.11 implemented def-tying refactor; Asks 6+7 added; PR shape now "v0.2 — 7 asks + Round A adversarial review"`. Cite the Quartz ledger paragraph.
   - Re-check inheritance row 1 (`tdxVerifier`) and row 2 (`groth16Verifier`) against the post-refactor Quartz `_negl` lifts — they should now have content.

6. **canonical_serialization — pin a specific scheme** (3/7 voices)
   - Add to Section 2.5: "Canonical serialization of `tally_body` uses Borsh per CosmWasm v2.0 spec. Canonical serialization of `contract_addr ‖ tally_body` is `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`."
   - Same definition resolves Theme 13 (Block E1 decrypted-plaintext parsing format).

7. **`commitHashE` row in Section 6.2** (2/7 voices)
   - Either remove the row (verified-rcv doesn't consume it; uses `Adv_commitTally_CR`), or annotate it `**NOT CONSUMED — see B9 `Adv_commitTally_CR` for the actual hash-CR hypothesis**`. The withdrawn `DstackKeyManager` row sets precedent.

8. **Section 4.9 — state enclave determinism as explicit requirement** (2/7 voices)
   - Add to Section 6.3 dstack/TDX trust: "Enclave image is deterministic over its input `(raw_ballots, candidates, privkey)`; no external entropy, no timing dependencies. This is an obligation on the verified-rcv enclave image, discharged at pre-deployment formal verification (Section 8.7)."

9. **Block 6 well-formedness — add winners-equals-last-round check** (1/7 voice, structurally tight)
   - Add to Block 6 Requires: `winners = candidates_in_round(per_round_counts.last())`. Concretely: `winners` equals the key set of the final `per_round_counts` entry. Already implicit in "elimination monotonicity"; making it explicit closes the gap kimi flagged.

10. **B8 clause (c) — 32-byte-digest-in-64-byte-slot** (1/7 voice, kimi truncated)
    - Restate as `lower_half(attested user_data) = SHA-256(canonical_serialization(contract_addr ‖ tally_body)) ∧ upper_half(attested user_data) = domain_separation_tag`.
    - Verify against Quartz's actual UserData encoding before merging the edit.

11. **B10's Stage-1 lemma — strengthen ECIES dependency to authenticated decryption** (1/7 voice — Claude, file-access-grounded)
    - Add to Section 8.7 Step 1: "Depends on Quartz's `Specs.Quartz.Crypto.Ecies.roundtrip` *plus* an authenticated-decryption / AEAD unforgeability property. The latter does not currently appear in Quartz; this is a v0.3 upstream ask. Until it lands, Stage 1 is stated as 'deterministic decrypt-or-drop' (correctness, not soundness)."

12. **Section 8.7 Step 3 — rename to disambiguate from B10** (1/7 voice — Claude)
    - "Composition theorem — B10" → "Lean-internal composition: image-IO = `Tally_spec`". Pairs with revision #3.

### EDITORIAL (cosmetic but tightens grounded reading)

13. **IRV_spec Step 1 base case subsumes the edge case bullet** (Claude #8): remove the redundant edge-case bullet or restructure Step 1 to handle only `|remaining| = 1` with the all-tied case in Step 3's failure-to-progress branch.

14. **Section 1 framing — "TEE removes trust" oversells** (gemma #4): rephrase to "TEE + attestation + pre-deployment formal verification of the enclave image", acknowledging the B10 obligation is the load-bearing piece.

15. **`ballots@end_at` notation** (Claude #10 cosmetic): add a one-line convention to Section 3.2 preamble: "`x@t` denotes the value of state variable `x` at the first state σ with `σ.env.block.time ≥ t`."

16. **Section 2.1 Round 1 prose** (Claude #11 cosmetic): re-word "B's first surviving choice is A — no movement" to "B's ballot already had A as its first choice; no redistribution movement is required."

17. **Section 8.2 trace terminology** (Claude #12 cosmetic): replace "Terminal tie → both co-winners" with "All remaining candidates have equal first-place counts; per Section 2.5 step 1 base case, all remaining are co-winners."

---

## 6. Methodology observations — this pass as v0.3 evidence

These don't affect the intent doc; they're inputs to the methodology back-port queue.

- **Ensemble depth-of-attack heterogeneity is real**: gemma vs. the rest on B6 + B10 is a clean example of one voice declaring "fine" where five others develop substantive critiques. The methodology should treat "voice affirms" as a *weaker* signal than "voice critiques without rebuttal" — affirmations don't necessarily mean the affirming voice engaged with the depth of the attack.
- **Theorem-prover specialist exclusion**: goedel-prover-v2-32b degenerated into Lean-style tautology loops. Confirms the prior session's observation: theorem-prover-trained models don't generalize to spec adversarial review; they pattern-match the prompt as a goal and try to prove it instead of attack it. v0.3 SKILL.md should warn against this.
- **Gateway timeout shape varies per upstream route** (Bugs 3 + 4 in `gateway-bugs-2026-05-14.md`): tooling needs to record `(http_status, elapsed, finish_reason)` per voice so synthesis can distinguish "voice refused / failed" from "voice agreed".
- **Manifest-pattern dispatch via `colosseum_run.py` (v0.3 prototype landed)** enables OpenCode-side non-Claude voices on long-output adversarial work, sidestepping the gateway 240s + Anthropic-route 127s caps that bit this pass. Round 3b would benefit from running the next pass via the manifest + OpenCode harness for non-Claude voices.
- **Voice-distribution-aware synthesis**: with 7+ voices, a verdict-bucket count is one signal but loses information about *which findings* clustered. The overlap-matrix shape this synthesis uses (theme → voices supporting → suggested fix) generalizes better and should become the SKILL.md-prescribed synthesis layout.

---

## 7. Recommended action

The verdict is **BREAKS-AGAIN**, with 4 critical revision items (#1-#4 in the punch list) load-bearing. Methodology says: revise → re-attack → only-then-Quint.

**Battery-friendly path forward** (no LM calls needed for the revision itself):

1. Apply punch-list items #1–#4 (the criticals) to `intent.md` — call this revision 3 (after first draft + Round-3a first-pass revision).
2. Fold in items #5 (Quartz de-retraction) and #6 (canonical_serialization) since both touch the same sections and are cheap.
3. Defer items #7–#17 to a follow-up revision pass if time permits, but they aren't blocking.
4. **Defer re-attack until on power** — the revision delta is large enough to warrant a fresh fan-out, not just a sanity-check sub-pass. This is the right time to test the manifest + OpenCode harness pattern in anger.

After re-attack survives: Quint spec (task #16) → Lean (#17) → verify pyramid (#18) → compose ledger (#19) → comparator pass against Quartz's existing IRV (#20).
