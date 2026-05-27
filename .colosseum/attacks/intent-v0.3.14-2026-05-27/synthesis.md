# Synthesis — v0.3.14 intent-adversarial fan-out (2026-05-27)

**Panel** (4 voices, cross-family via gateway):
- Claude Opus 4.7 (Anthropic, focused crypto-attacks lens via Agent subagent) — file: `crypto-voice.md`
- gemini-2-5-flash (Google) — file: `gemini-2-5-flash.md` (truncated at 238 tokens, partial)
- cloudflare-100-cf-nvidia-nemotron-3-120b-a12b (NVIDIA) — file: `nemotron-3-120b-a12b.md` (truncated at 3000 tokens, 3 attacks)
- gpt-oss-120b (OpenAI-open) — file: `gpt-oss-120b.md` (complete, 4 attacks)

kimi-k2-6 (Moonshot) was dispatched but returned `text: null` with reasoning-only tokens (3000 reasoning tokens, 0 output). Excluded from panel.

True multi-voice cross-family diversity achieved (4 families: Anthropic + Google + NVIDIA + OpenAI-open). Methodology limitations honestly recorded: gemini + nemotron hit upstream token cutoffs; kimi response surface was reasoning-tokens-only.

## Cross-voice convergence matrix

| Finding | Claude | gemini | nemotron | gpt-oss | Votes | Synthesis severity |
|---|---|---|---|---|---|---|
| Registration ReportData omits `names_hash` | M1 | — | #3 (partial) | #2 (critical) | **3/4** | **MAJOR** (defense-in-depth; not critical because publish-side binding + B11 immutability already foreclose tamper via stored-vs-runtime check) |
| No DST tag on internal hashes (ballots_hash, names_hash) | M2 | — | #2 | — | **2/4** | **MINOR** (defense-in-depth; future foot-gun if hashes exposed independently) |
| Pairing (addr_i, name_i) not directly bound by names_hash alone | — | — | #1 | — | 1/4 | **INFORMATIONAL** (mitigated by ballots_hash declaration-order + tally_body address-keyed binding; one voice flagged the lens-output gap, panel agrees binding exists indirectly; doc clarification suffices) |
| B11 enforcement is structural (no UpdateNames handler) — fragile to migration | — | — | — | #3 | 1/4 | **MINOR** (architectural risk for future contract upgrades; explicit `names_locked: bool` flag or §6.x migration-discipline note recommended) |
| Visual confusability accepted by chain | — | #1 | — | — | 1/4 | **NOT NEW** (already documented as scope-bounded in §6.4 per author's intent-adversarial AB.4 pass; gemini voice did not see the §6.4 clarifying paragraph) |
| Names_hash preimage 32-bit overflow → collision | — | — | — | #1 (claimed critical) | 1/4 | **FALSE POSITIVE** (Borsh canonical-encoding injectivity property + chain gas/state-size limits foreclose; u32 max ≈ 4B values is structurally unreachable. Worth one-line bound statement to close the lens) |
| Host lying about names → indefinite publication block | — | — | — | #4 | 1/4 | **NOT NEW** (covered by failure mode 4.9 deterministic-malformed-tally deadlock; same DoS surface as host not running enclave at all) |
| N=0 / empty-names edge case | — | — | — | (implicit) | — | **PRE-EMPTED** (Block 1 requires `len(candidates) ≥ 1`, foreclosed) |

## Findings to incorporate pre-commit

**Cross-voice-confirmed (worth a spec change):**

**F1 (MAJOR, 3-voice consensus) — Extend registration ReportData to bind `names_hash`.**

Current v0.3.14: registration ReportData = `SHA-256(enclave_pubkey ‖ Borsh(contract_addr) ‖ u64_LE(election_id)) ‖ DST_PUBKEY` (32+32).

Proposed: registration ReportData = `SHA-256(enclave_pubkey ‖ Borsh(contract_addr) ‖ u64_LE(election_id) ‖ names_hash) ‖ DST_PUBKEY` (32+32).

Rationale: parallels the v0.3.12 N22 fix that triple-bound the registration. Adding `names_hash` to the registration binding makes tamper detectable at CreateElection time (earlier than publish-time). The publish-side binding catches it eventually, but the registration-side binding is "audit-clear" — a registration quote attests to the names the enclave saw at handshake time, not merely at publish time.

Estimated code cost: ~30 lines (contract.rs `verify_registration_quote` reads stored names_hash, includes in expected hash; enclave attestation.rs `produce_registration_artifacts` takes candidate_names and includes names_hash in its preimage). Existing `RegistrationQuoteWrongElection` error covers the rejection.

**F2 (MINOR, 2-voice consensus) — Add DST tags inside `ballots_hash` and `names_hash` preimages.**

Current v0.3.14: both have the structurally-identical shape `SHA-256(u32_LE(N) ‖ Borsh(...) ‖ ...)`.

Proposed: `ballots_hash = SHA-256("verified-rcv:ballots:v1" ‖ u32_LE(N) ‖ ...)` and `names_hash = SHA-256("verified-rcv:names:v1" ‖ u32_LE(N) ‖ ...)`.

Rationale: domain separation. The outer canonical_serialization already framings the two hashes at fixed offsets, but if either hash ever gets exposed independently (cross-protocol use, partial serialization dump), the inner DST prevents substitution. Cheap defense-in-depth. ~10 lines of code.

**F3 (MINOR, single voice but valid) — Explicit B11 structural-enforcement clause.**

Current v0.3.14: B11 says "structural via no UpdateNames handler" in prose.

Proposed: add an explicit invariant in `state.rs` and §6.x trust-boundary: future contract migrations MUST preserve the no-UpdateNames-handler property; any migration that introduces a name-mutation path is a behavioral break requiring MAJOR version bump (per v0.3.0 rubric).

Estimated code cost: documentation only (one §6.x clause, one doc-comment block).

**F4 (INFORMATIONAL, doc clarification) — Pairing (addr_i, name_i) binding explanation.**

Current v0.3.14: names_hash binds names; ballots_hash binds addresses-in-declaration-order; tally_body binds addresses via Vec<Addr> + Maps. Pairing is indirectly bound.

Proposed: §2.5 add one paragraph stating the pairing-binding pattern explicitly. No code change.

**Pre-empted / NOT incorporated:**

- F-overflow (gpt-oss #1): u32 overflow is structurally unreachable given input bounds.
- F-DoS-host (gpt-oss #4): same surface as failure mode 4.9.
- F-visual-confusable (gemini): documented scope-bound in §6.4.

## Methodology validation events from this pass

1. **Multi-voice fan-out paid for itself.** Without it, the author-self-review (single voice = Claude implementation author) caught visual-confusability but missed the registration-binding asymmetry (3-voice cross-confirmation) and DST separation (2-voice cross-confirmation). The methodology back-port (`colosseum-adversarial` SKILL) is validated.

2. **Cross-family diversity matters.** The same shape of finding (registration-names asymmetry) surfaced across three different model families (Anthropic + NVIDIA + OpenAI-open). One family alone would have been at risk of family-priors blind spots.

3. **False positives exist and synthesis adjudicates.** gpt-oss raised a critical-rated overflow attack that's structurally pre-empted. The synthesis step (this document) is the load-bearing adjudication.

4. **Token limits matter.** Three of four voices truncated. The fan-out script should be tightened to request <2K-token responses, OR the fan-out should use streaming + extension on truncation. Documented as a v0.4 ask gap (extend Ask T to specify max-tokens AND request-mode-streaming where supported).

5. **Author-self-review is not a substitute** even when the author is methodology-disciplined. I (the implementation-author agent) applied the 4 named lenses inline and produced one finding (visual-confusability). The cross-family panel surfaced 4 distinct findings with stronger severity. AB's discipline ("the stage operator MUST be a different agent than the code-implementation author") is empirically validated by this pass: the author missed 3 of 4 findings the cross-family panel caught.

## Recommendation

Incorporate F1 + F2 + F3 + F4 into v0.3.14 pre-commit. Re-run the v0.4 lenses against the strengthened intent. If the panel re-pass converges to zero new findings, lock v0.3.14. Otherwise iterate.

Estimated incremental cost: ~50 LOC across contract.rs + attestation.rs + intent.md sections + 3-5 new test assertions.
