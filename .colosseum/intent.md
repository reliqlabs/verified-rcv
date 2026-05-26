# Intent: verified-rcv

> Colosseum intent document. The human-anchored source of truth for what this system should do. Every downstream spec, proof, and test is bounded by the quality of this document.

**Document version: 0.3.11** (SemVer per Colosseum methodology v0.4 candidate Ask N — see `colosseum/methodology-v0.4-candidates.md`). Version-bump classification:
- MAJOR (0.x.0): invariant removed / weakened / trust boundary widened / behavior previously specified becomes unspecified, OR a previously-stated invariant is rewritten because the prior statement was provably false/vacuous (per v0.3.1 Round 3a synthesis F2 — rubric extended to cover this case)
- MINOR (0.0.x where x adds): new invariant / trust boundary narrowed / new failure mode / new scenario / new defined symbol or predicate
- PATCH (0.0.x where x clarifies): notation cleanup / status block update / cross-reference fix / worked-example arithmetic clarification with correct answer preserved

**Revision history**:
- v0.1.0 — 2026-05-14 initial elicitation pass via `colosseum-intent` skill.
- v0.2.0 — 2026-05-14 MAJOR: revision against Round 3a first-pass adversary (13 critical + selected serious findings from `.colosseum/attacks/intent-first-draft-2026-05-14.md`).
- v0.2.1 — 2026-05-14 PATCH: 4 sanity-pass fixes (B10 arity tightening, Block E1 cross-references, Section 6.1 image-registration bullet, Section 8.7 added).
- v0.3.0 — 2026-05-15 MAJOR (per the extended MAJOR rubric above, since v0.3.0 rewrote B9 from a previously-vacuous unconditional bound to a conditioned one): revision against Round 3a re-adversarial fan-out (6 multi-voice findings from `.colosseum/attacks/intent-revised-2026-05-14T200029Z/synthesis.md` — B6 ∀-per-key, B10 cross-layer, B9 conditioned on `image_registration_honest`, S5 restated as set-once-write-discipline, Section 6.2 de-retraction, canonical_serialization pinned to Borsh).
- v0.3.1 — 2026-05-16 MINOR: revision against Round 3a 3rd adversarial pass (subagent dispatch + claude subagent; 4 critical + 12 serious themes from `.colosseum/attacks/pattern-b-v0.3.0-2026-05-16T125913Z/synthesis.md`). MINOR because the pass adds 4 new failure modes (§4.10-§4.13), 3 new scenarios (§8.8-§8.10), and several new defined symbols/predicates (`EnclaveImage` typing, `image_registration_honest` predicate, `ballots@end_at` definition, `input_fidelity` conjunct, `dstack_kms_trust` axiom). No invariant is weakened. The 4 critical fixes (EnclaveImage typing, `image_registration_honest` definition, B10 conjuncts, Tally_spec determinism) close soundness gaps in the prior load-bearing claims by adding precision, not by removing constraints.
- v0.3.2 — 2026-05-19 PATCH: encoding-discipline tightenings to §2.5 Block E1 and §3.2 B2 per the v10 baseline rerun finding (`.colosseum/specs/synthesis-2026-05-19/baseline-vs-rerun.md`). Adds two protocol-layer-model-checker constraints (A2: enclave-side state required for B10 projection to be checkable; A3: B2 must be encoded as a checkable invariant over a snapshot variable, not action-guard shadow). PATCH because no invariant content changes, only the encoding-discipline guidance for downstream Quint/TLA+ models is made explicit. Tests the convergence hypothesis: do voices that previously diverged on these axes converge once the intent specifies the encoding requirement.
- v0.3.3 — 2026-05-20 PATCH: encoding-discipline tightenings to §3.1 S2 and §2.5 Stage 2 per the Lean cross-critique cycle (`.colosseum/specs/lean-cross-critique-2026-05-20/meta-analysis.md` + `.colosseum/specs/lean-critique-revised-canonical-2026-05-20/meta-analysis.md`). Adds two downstream-Lean-spec constraints (A4: `CandidateSet` distinctness MUST be a hypothesis or bundled type-level invariant in downstream Lean specs, since S7's conservation equation fails for duplicate-containing `cs`; A5: downstream Lean specs declaring `IRV_spec` opaque MUST axiomatize `ballots_tallied = |valid_ballots|`, since the opaque declaration does not capture this Stage-2 obligation). PATCH because no invariant content changes; both notes make existing intent claims (S2 distinctness; Stage 2 ballots_tallied definition) inspectable at the Lean encoding layer. Convergent across the 3 Lean voices' re-cross-critique (kimi + magistral picked A4; gpt-5.5 picked A5).
- v0.3.4 — 2026-05-23 PATCH: documents the implicit length relation between Stage 2's `eliminated_by_round` and `per_round_counts` lists (kimi's optional note from the re-cross-critique meta-analysis Q3). The relation `eliminated_by_round.length = per_round_counts.length` holds in the typical case (one elimination per non-terminal round + a final round that records the winning count without further elimination), with the all-abstain boundary case being `0 = 1` round (single zero-count round, no eliminations) — wait, that's `eliminated_by_round.length = 0, per_round_counts.length = 1`, so the relation is `per_round_counts.length = eliminated_by_round.length + 1` in non-trivial cases and `per_round_counts.length = eliminated_by_round.length` only when batch elimination produces multi-winner co-winners (terminal tie). Encoded as a clarifying note in §2.5 Stage 2 plus encoding-discipline note A6 (downstream Lean specs MAY but are not required to encode this as a well-formedness predicate). PATCH because no invariant content changes; the relation is a derivable property of the existing IRV algorithm.
- v0.3.11 — 2026-05-26 MINOR: B6 closure — `enclave_input_fidelity` link 7 (§8.7) is now discharged via the cheapest path documented in v0.3.9: extend `canonical_serialization` with a `ballots_hash` field. The full canonical_serialization layout is now `Borsh(contract_addr) ‖ Borsh(chain_id) ‖ u64_LE(election_id) ‖ ballots_hash[32] ‖ tally_body`. `ballots_hash` is 32 raw bytes (no length prefix; fixed size) of `SHA-256(u32_LE(included_count) ‖ for each included entry: Borsh(voter: String) ‖ Borsh(ciphertext: bytes))`. Iteration is in candidate-declaration order: walk `candidates`, for each candidate with a corresponding `BALLOTS` entry, emit the pair; candidates without ballots are skipped (they appear in `non_voters` downstream). Implementations: contract-side `compute_ballots_hash(candidates: &[Addr], ballots_view: &[(Addr, HexBinary)]) -> [u8; 32]` lives in `crates/contract/src/contract.rs`; runtime-side `compute_ballots_hash(candidates: &[String], entries: &[(String, Vec<u8>)]) -> [u8; 32]` lives in `crates/enclave/src/attestation.rs`. Cross-test `compute_ballots_hash_contract_vs_runtime_byte_identical` asserts byte-equality. **§8.7 link 7 reformulated** from "open: choose between (i) extended `report_data` schema OR (ii) chain-inclusion proof" to "**discharged via option (i)**: `ballots_hash` is bound into the publish-quote ReportData via `canonical_serialization`. A host that substitutes raw_ballots produces a `ballots_hash` different from the chain's `compute_ballots_hash(election.candidates, BALLOTS.range(...))`; the chain's `verify_publish_quote` then fails `AttestationCommitMismatch`." **Methodology consequence**: B10's 5-link composition (Lean + image-binding + B8 + dstack-KMS-trust + enclave_input_fidelity) is now fully chain-witnessed for links 5+7 (B8 via gnark + B6 via ballots_hash), with links 1+2+3 (the Lean discharge) still pending B10_lean's `sorry` and link 4 (image-identity-binding) still requiring reproducible build infrastructure. The remaining trust gap is fully on the off-chain side; the chain catches every input-fidelity and output-binding violation modulo SHA-256 collision-resistance. **MINOR per the v0.3.0 rubric**: trust boundary narrowed (link 7 moves from open methodology obligation to chain-verified theorem); new symbol added (`ballots_hash`, `compute_ballots_hash`); no invariant weakened. **B6 closes the last contract-side gap from the v0.3.10 deployment-readiness audit** (remaining items B1/B2/B3/B4/B5/B7 are operational/build/math layer, not contract).
- v0.3.10 — 2026-05-26 MINOR: audit re-review N2 + N3 + N4 + N5 + N6 remediations (companion bundle to v0.3.9 N1). Per-finding changes: **N2 (admin pubkey substitution residual)** — `UpdateRegistry` (the v0.3.8 M3 immediate-rotation handler) is removed and replaced with a 3-handler timelock flow: `ProposeRegistryUpdate` (admin-only; stores `PendingRegistry { registry, apply_after = now + config.registry_update_delay_seconds }`), `FinalizeRegistryUpdate` (permissionless after `apply_after`), `CancelRegistryUpdate` (admin-only; discards pending). The `Config` gets a new field `registry_update_delay_seconds: u64`; production deployments should set this to a meaningful value (e.g., 86400 = 1 day) so voters can observe a pending rotation via the new `QueryMsg::PendingRegistry` query and react before the new registry lands. Permissionless finalize prevents a misbehaving admin from soft-bricking a pending update. With the timelock + visibility, the residual attack surface narrows from "admin can swap mid-flight" to "admin proposes, waits the timelock, finalizes." Combined with v0.3.9 N1 B8(e) (which requires the substituted pubkey to come from a TDX enclave whose code matches the registry), the remaining surface is "admin runs an alternate VM with MRTD/RTMR matching the registry, proposes an update, waits out the timelock, finalizes" — meaningfully harder than the v0.3.8 baseline. **N3 (silent tally clobbering on re-CreateElection)** — `CreateElection` from `Resolved` phase now archives the prior `TALLY_RESULT` into a new `HISTORICAL_TALLIES: Map<u64, TallyResult>` storage map keyed by `election_id` before clearing for the next election. New `QueryMsg::HistoricalTally { election_id }` lets consumers pin by `election_id` and resolve the result indefinitely. **N4 (cross-chain replay defense for canonical_serialization)** — `canonical_serialization` layout changes from `Borsh(contract_addr) ‖ u64_LE(election_id) ‖ tally_body` to `Borsh(contract_addr) ‖ Borsh(chain_id) ‖ u64_LE(election_id) ‖ tally_body`. The contract reads `env.block.chain_id` and passes it to the commit-hash computation; the runtime takes a `chain_id` field on `TallyRequest`. Defense-in-depth — without this, a tally bound by `canonical_serialization` on chain A could be replayed on chain B with the same `(contract_addr, election_id, tally_body)` if both chains happened to share addresses; the chain_id binding prevents that. Cross-test extended to assert bytewise difference under `chain_id` variation. **N5 (CI safety gates)** — `.github/workflows/ci.yml` now runs: (a) default-features build + test, (b) verification-feature test (Kani harness types), (c) clippy with `-D warnings` on the workspace, (d) `cargo audit --deny warnings`, (e) release wasm build + `cosmwasm-check`, (f) symbol-grep on the wasm artifact to verify neither `mock_attestation` nor `verification` symbols leaked into the production build. The grep is an honest "deferred is panic" enforcement (audit Ask Z). Clippy passing required allowing 6 Aeneas-extractability stylistic lints on the `enclave-core` crate (the Lean-extraction target is deliberately non-idiomatic Rust; idiomatic clippy fixes would break extraction) and a `[lints.rust] check-cfg = ['cfg(kani)']` declaration on the contract crate (Kani's external cfg is now known to rustc). **N6 (state.rs doc-comments stale)** — refreshed `state.rs` doc-comments across `Election`, `EnclaveImageRegistry`, `Config`, `PendingRegistry` (new), `TALLY_RESULT`, `HISTORICAL_TALLIES` (new) to reflect M1 phase gating + M3 registry update path + v0.3.9 N1 B8(e) pubkey provenance binding + v0.3.10 N2 timelock + v0.3.10 N3 archival. Methodology consequence: N1+N2 together close most of the admin trust surface from the v0.3.8 audit; the remaining surface (alternate-VM-with-matching-MRTD scenario) is now governance-style rather than cryptographic and is appropriately addressed at the operational layer (multi-sig admin, governance-locked admin transfer, etc., out of scope for verified-rcv contract itself). **MINOR per the v0.3.0 rubric**: trust boundary narrowed (admin can no longer immediately rotate the registry; voter detection window mandatory), new symbols added (`PendingRegistry`, `HISTORICAL_TALLIES`, `chain_id` in canonical_serialization, three new ExecuteMsg variants, two new QueryMsg variants, `registry_update_delay_seconds` on Config), no invariant weakened. The `UpdateRegistry` removal would otherwise be MAJOR ("behavior previously specified becomes unspecified") but the replacement is strict refinement of the rotation path with strictly more constraints, so MINOR holds. **Asks Y + Z + AA from v0.3.8 are now fully honored**: contract-and-runtime adversarial fan-out (Y) drove N1 + N2; deferred-is-panic (Z) is enforced in CI grep + the chain-side `verify_gnark_proof_via_xion` returns hard error in production builds; negative-test discipline (AA) is satisfied by the 45-contract-test suite covering every new rejection path (gnark layout, ReportData mismatch, commit_hash mismatch, measurement mismatch, TCB unaccepted, registration vs publish DST cross-purpose replay, timelock not-expired, timelock no-pending, pending-already-set, archive-on-resolved, chain_id divergence).
- v0.3.9 — 2026-05-26 MINOR: trust boundary narrowing. The audit re-review (N1) noted that v0.3.8 closed the *envelope-format* gaps (M2 election_id, M3 registry shape, M4 declaration order) but left B8 clauses (a), (b), (d) discharged only by a documented future-work TDX-quote-parser plus a SHA-256 commit-hash binding on `user_data` (B8(c)) — i.e., the chain provided **consistency** of the published `(tally, user_data)` pair but not **soundness** of the attestation. The audit also noted (N1.c) that pubkey provenance had no cryptographic discharge — admin could substitute an arbitrary enclave pubkey. v0.3.9 closes all three with a single direct call to Xion's `xion.zk.v1.Query/ProofVerifyGnark` from inside the verified-rcv contract, replacing the `DstackEnvelope` wrapper entirely. **Trust boundary narrowed** (per the MINOR rubric): B8(a), B8(b), B8(d) move from "deferred axiom, no on-chain witness" to "verified in-contract every publish." Pubkey provenance (a new clause B8(e), v0.3.9) gets the same gnark-witnessed binding via `ReportData[0..32] = SHA-256(enclave_pubkey)` at `CreateElection`. Per-block effects: (1) §2.5 adds the **gnark public_inputs byte layout** as a normative table — 306 fr-elements × 32 BE bytes = 9,792 bytes for the verified-rcv reference DCAP circuit (one byte per `uints.U8`, MrTd at `[0..1536]`, Rtmr0..3 at `[1536..7680]`, ReportData at `[7680..9728]`, TcbStatus at `[9728..9760]`, Timestamp at `[9760..9792]`). (2) §2.5 adds the **`ReportData` 64-byte layout** with purpose-tagged halves: registration ReportData = `SHA-256(enclave_pubkey) ‖ DST_VERIFIED_RCV_ENCLAVE_PUBKEY_V1_PADDED` (32+32); publish ReportData = `commit_hash ‖ DST_VERIFIED_RCV_TALLY_V1_PADDED` (32+32, where `commit_hash = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))` per v0.3.8 M2). Domain separation between registration and publish quotes prevents cross-purpose replay even though both use the same circuit + same vkey. (3) §2.5 Block 6 (`publish_result`) signature changes from `{tally, attestation: DstackAttestation}` to `{tally, proof: Vec<u8>, public_inputs: Vec<u8>}`; Block 6's Requires reformulated in terms of direct gnark verification + measurements + ReportData equality. (4) §2.5 Block E1 outputs `(tally, proof, public_inputs)` instead of `DstackAttestation`. (5) §6.1 registry shape changes: `vkey: Bytes32` → `vkey_name: String` (a reference to the on-chain `xion.zk` vkey table; the vkey itself lives in the chain's ZK module, not in verified-rcv's contract). `mrtd` and `rtmr` remain 48 bytes (TDX SHA-384); the registry now distinguishes `rtmr1` and `rtmr2` (mandatory) from `rtmr0` and `rtmr3` (optional, may be left unbound by setting to None — see zkdcap-verifier's `check_rtmr0` flag pattern). (6) §3.2 B8 clauses reformulated: (a) and (b) collapse into "the gnark proof verifies via `ProofVerifyGnark` against the registry's `vkey_name`"; (c) restated as "the commit_hash extracted from `public_inputs.ReportData[0..32]` equals `SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))`"; (d) restated as "the (MrTd, Rtmr1, Rtmr2, optionally Rtmr0/Rtmr3) tuple extracted from `public_inputs` equals the on-chain registry's measurements"; (e) added — "the `enclave_pubkey` registered at `CreateElection` matches `public_inputs.ReportData[0..32]` of the registration quote, where the registration quote was verified by the same gnark verification primitive". (7) §3.2 B9 unchanged in structure (3-summand bound stands); `Adv_commitTally_CR` continues to bound SHA-256 collision-resistance on `canonical_serialization(contract_addr ‖ election_id ‖ tally_body)` as before. (8) §6.2 inheritance table updated: the `groth16Verifier` row's discharge path is now "by Xion `xion.zk.v1.Query/ProofVerifyGnark` semantics" (trusted per §6.1 Chain Trust) rather than "ArkLib Groth16 KS reduction + reference DCAP circuit-equivalence theorem"; the reduction now lives at the *chain layer*, not at the verified-rcv layer. The `circuit_equivalence_honest` precondition for B9 now reads as a property of the registered `vkey_name`'s vkey content (i.e., that vkey corresponds to the correct DCAP circuit). (9) §8.7 link 5 (B8) reworded for the gnark verification path; links 6 (`dstack_kms_trust`) and 7 (`enclave_input_fidelity`) unchanged. (10) §4.3a/4.3b reframed in vkey-name terms (the substituted artifact is now the named vkey row in `xion.zk`, not a contract-storage byte field). Methodology consequence: B8 is no longer an *axiom* of the verified-rcv layer — it's a *theorem* discharged by the conjunction of `ProofVerifyGnark` correctness (§6.1 chain trust) + measurement equality (chain-side bytewise check) + ReportData equality (chain-side bytewise check) + circuit correctness (`circuit_equivalence_honest`, the vkey content matches the DCAP semantic). The trust surface moves from "attestation-shim plus deferred future work" to "chain ZK module correctness plus operational vkey registration honesty." Asks Z + AA from v0.3.8 are honored (no chain-side stub-as-Ok; negative tests for each new rejection path). **MINOR per the v0.3.0 rubric**: trust boundary narrowed (B8 a/b/d move from inherited-axiom to chain-verified-theorem), new symbols added (gnark public_inputs byte layout, ReportData purpose-tagged layout, B8(e) pubkey-provenance clause), no invariant weakened, no behavior previously specified becomes unspecified (DstackEnvelope is *replaced* by a more concrete spec, not removed). The `DstackEnvelope` removal would otherwise be MAJOR-classified ("behavior previously specified becomes unspecified") but the replacement is strict refinement of the verification path, so the MINOR classification holds.
- v0.3.8 — 2026-05-26 PATCH+MINOR-ish: substantial audit-finding remediation pass (3 critical + 5 major). The contract at HEAD did NOT honor B8/B10 guarantees the intent advertised: `AttestationEnvelope::Mock` was reachable in production builds; the `Dstack` variant only checked `user_data.len() == 64`; `enclave_pubkey` was accepted from admin without shape validation; `CreateElection` clobbered an active election; `canonical_serialization` omitted `election_id` so cross-election attestation replay was viable; `EnclaveImageRegistry` had no shape validation or update path; `check_tally_well_formed` didn't enforce candidate-declaration order on per-round counts; the runtime's `HealthResponse.mrtd_hex` returned the dstack `compose_hash` not the TDX MRTD. **Five out of six independent attack strategies reached the same critical findings.** Encoding-discipline notes added: (1) `Adv_groth16_KS` summand bundling cross-reference (carried from v0.3.7). (2) `canonical_serialization` now defined as `Borsh(contract_addr) ‖ u64_LE(election_id) ‖ tally_body` per intent §2.5; the `election_id` insertion is the M2 audit fix that closes the cross-election replay. (3) §3.2 B8(d) commit-hash field formally specified at byte-level; the chain MUST verify the lower 32 bytes of `user_data` equal `SHA-256(canonical_serialization(...))`. (4) §6.1 registry shape pinned: `mrtd` and `rtmr` MUST be 48 bytes each (TDX SHA-384 measurements); `vkey` MUST be non-empty. (5) §6.1 registry update path: admin-only `UpdateRegistry` execute message, gated on no active election. (6) §2.5 Block 1 alternate path: `CreateElection` is admin-only AND gated on previous election being in `Created` or `Resolved` phase (not Voting/Tallying). (7) §3.1 S6-S9 declaration-order pin: chain-side well-formedness verification MUST also verify subset-in-declaration-order for `winners`, `dropped_voters`, `non_voters`, per-row of `per_round_counts` and `eliminated_by_round`. **Process finding**: the adversarial fan-out was applied to the intent/spec layers but NOT to the contract or runtime code; future cycles must run `colosseum-spec-adversary` against every code artifact that ships into the trust boundary (Ask Y in colosseum/methodology-v0.4-candidates.md). **Mock attestation discipline**: any code path that an intent invariant depends on must either be fully implemented or compile out / panic. Stub-as-Ok is forbidden (Ask Z). **Negative-test discipline**: for every invariant `Ok ⇒ P`, the harness suite MUST also include `¬P ⇒ Err` (Ask AA). All eight findings have been remediated in code; 25 contract tests + 16 runtime tests + 4 cross-tests + 11 IRV-math tests pass. The `B10_lean_irv` discharge remains pending (multi-day Aeneas refinement) but the trust chain it anchors no longer has chain-side escape hatches.
- v0.3.7 — 2026-05-26 PATCH: substrate clarifications from the Quartz audit (2026-05-26 read of `/Users/mvid/Development/reliq/quartz`). Three clarifications, no invariant content changes: (1) `Adv_groth16_KS` summand bundling — Quartz's `groth16Verifier` lift is "doubled-negligibility" (Groth16-KS on BN254 + R1CS↔reference-DCAP circuit equivalence) per `ZkdcapVCVio.lean`; verified-rcv's B9 retag (v0.3.1) split the doubled into `Adv_groth16_KS` (one summand) + `circuit_equivalence_honest` (one binary precondition). The framings are semantically equivalent under degenerate-negligibility interpretation of circuit equivalence, but the audit notes the bundling should be inspectable. §6.2 now states this cross-reference explicitly. (2) Added two more v0.3 caveats inherited from Quartz to §6.2's caveat list: `IsPPT` predicate is partially-closed (Quartz Round A finding #5; polynomial-adversary notion not substantively instantiated yet, cycle 6.14 queued), and `Ecies.roundtrip` is proven at spec-layer abstraction only (`Ciphertext := PubKey × Plaintext`, deterministic — real randomness + curve operations are not modelled). (3) Confirmed: `DstackKeyManager` withdrawal (intent v0.2.1 / §6.2 line 627) and `image_registration_honest` verified-rcv-internal labeling (§6.1, ledger line 80) are already correct per the audit — no patches needed at those sites; the audit's risk recommendations on those were prophylactic. PATCH because no invariant content changes; just inspectability of inherited assumptions.
- v0.3.6 — 2026-05-25 PATCH: encoding-discipline tightening to §3.1 S8 and S9 per the Round 3e Stage-2 proof-attempt finding A8. Concretizing math `IRV_spec` (Round 3e) made visible that S8's per-round conservation (`Σ counts = ballots_tallied`) and S9's elimination monotonicity (eliminated candidates do not reappear in later rounds) are BOTH false without a ballot-validity hypothesis: a ballot whose ranking is disjoint from `cs` (or empty) yields `count_at_index = 0` everywhere, triggering the all-abstain branch with `per_round_counts[k] = cs.map(c => {c, 0})` — and at deeper recursion `k > 0`, that branch reintroduces previously-eliminated candidates into a later round (S9 violation) AND yields `Σ = 0 ≠ ballots_tallied = valid.length > 0` (S8 violation). Intent §2.5 Stage 1 validates `valid_ballots` as permutations of `cs`; downstream Lean specs declaring S8/S9 over an abstract `valid : List (Addr × Ballot)` MUST propagate that constraint as a hypothesis: `∀ (_, b) ∈ valid_ballots, ∀ c ∈ cs, c ∈ b.ranking` (every ballot covers `cs`, the weakest condition that makes the all-abstain branch unreachable for non-empty `valid`). Adds A8 alongside A4/A5/A7. PATCH because no invariant content changes; just propagates the Stage 1 partition discipline (validated permutation) to the Stage 2 obligation statements. Pattern identical to A7: opaque-IRV_spec hid a soundness gap that concretization exposed.
- v0.3.5 — 2026-05-24 PATCH: encoding-discipline tightening to §3.1 S6 per the Round 3e concreteness pass finding. Concretizing the math `IRV_spec` in `specs/RcvSpec.lean` (replacing the prior `opaque` declaration with a concrete recursive definition mirroring §2.5's algorithm) made visible that the previously-axiomatized `irv_winners_shape : 1 ≤ winners.length` claim is UNPROVABLE for the edge case `cs.length = 0` — `[].Nodup = True` holds vacuously, so the prior axiom statement (parameterized only over `cs.Nodup`) was inconsistent at that boundary. The intent already enforces `len(candidates) ≥ 1` at Block 1 (a Requires clause whose violation reverts instantiation), so S6 only ever applies to elections that passed Block 1 — but the downstream Lean spec didn't propagate this dependency. Adds encoding-discipline note A7 (downstream Lean specs MUST include `1 ≤ cs.length` or equivalent non-empty hypothesis on every S6/`irv_winners_shape` statement, since `cs.Nodup` alone does not preclude `cs = []`). PATCH because no invariant content changes; just makes the implicit Block 1 → S6 dependency explicit at the Lean encoding layer. Pattern is the same as A4 (cs.Nodup propagation) and A5 (ballots_tallied propagation) — opaque-IRV-spec assumptions made invisible until concretization surfaced them.

See the Revision Log section near the end for full per-bump notes.

## 1. System Identity

(Refined per v0.3.1 subagent-dispatch synthesis F3 — single-voice from kimi scope #2: the v0.3.0 "instant-runoff voting smart contract" framing understates the off-chain scope. The system has three load-bearing components — the chain contract is one of them, not the system as a whole.)

- **Name:** verified-rcv — a three-component instant-runoff voting *system*, comprising (i) a CosmWasm chain contract, (ii) a dstack-TDX enclave, and (iii) a zkdcap attestation pipeline. The word "contract" alone refers to component (i) only; "verified-rcv" names the whole.
- **Scope:** end-to-end system spanning (i) a CosmWasm contract on a public Xion-class chain handling election lifecycle (instantiate / submit-ballot / close-and-tally / publish-result), (ii) a dstack-TDX enclave performing tabulation over ECIES-encrypted ballots, (iii) zkdcap attestation of the enclave's tally output. Specifies protocol-level behavior across all three components.
- **Purpose:** enable a public, verifiable IRV election among a declared candidate set, where individual ballots remain private (ECIES-sealed to the enclave's per-election keypair) while elimination rounds and the winner are publicly auditable. Ranked-choice tabulation eliminates spoiler-effect strategic pressure on candidate-voters; the TEE + attestation removes trust in any single tallier.
- **Electorate:** the candidate set itself — only declared candidates may submit ballots. Anyone may instantiate a contract; legitimacy of any given instance is off-chain. The full per-round vote-count output makes participation (or non-participation) of each candidate publicly inspectable.

## 2. Behaviors

Concrete input/output pairs across typical, boundary, and edge cases. Each behavior is a worked example. If a spec writer reads only this section, they should be able to characterize the system's input-output relation.

### 2.1 Happy path

**Setup:** election instantiated with 5 candidates: `A, B, C, D, E` (distinct chain addresses). Voting window `[t0, t1]`. At `t = t0 + δ`, all 5 candidates have submitted ballots.

**Ballots submitted** (each candidate's full ranking, ECIES-encrypted under the per-election enclave pubkey):

- A's ballot: `A > B > C > D > E`
- B's ballot: `A > B > C > D > E` (B endorses A as first choice)
- C's ballot: `C > A > B > D > E`
- D's ballot: `D > C > A > B > E`
- E's ballot: `E > C > A > B > D`

**At `t > t1`, tally proceeds** (revised 2026-05-14 per Round 3a first-pass adversary Attacks 1 + 42 — earlier draft had an arithmetic error in Round 2):

| Round | A | B | C | D | E | Action |
|-------|---|---|---|---|---|--------|
| 1 | 2 | 0 | 1 | 1 | 1 | B has 0 first-place votes (unique lowest). Eliminate B. Redistribute B's ballot: B's first surviving choice is A (since `A > B > …` after dropping B is `A > C > D > E`), which already counts B's first-place vote — no movement. |
| 2 | 2 | — | 1 | 1 | 1 | C, D, E tied for lowest at 1 each. **Batch-eliminate all three** per the "ties → batch-eliminate-all-tied" policy from Section 2.5. Redistribute: C's ballot `C > A > B > D > E` after dropping C/B/D/E → `A` → A absorbs. D's ballot `D > C > A > B > E` after dropping D/C/B/E → `A` → A absorbs. E's ballot `E > C > A > B > D` after dropping E/C/B/D → `A` → A absorbs. |
| 3 | 5 | — | — | — | — | A is the sole remaining candidate. A holds 5 of 5 ballots = 100%. **A wins.** |

**Output (`TallyResult`, published with attestation):**

```
winners             = [A]
per_round_counts    = [
  {A: 2, B: 0, C: 1, D: 1, E: 1},
  {A: 2,        C: 1, D: 1, E: 1},
  {A: 5                          }
]
eliminated_by_round = [[B], [C, D, E]]
ballots_tallied     = 5
ballots_dropped     = 0
dropped_voters      = []
non_voters          = []
```

**Why the new winner differs from the original draft**: the earlier draft showed C winning via a Round-2 transition `C: 1 → 2` that had no valid redistribution path (B's elimination flowed to A, not C). The arithmetically correct trace under the stated batch-elimination policy produces A as the winner. Both rules (batch-eliminate-all-tied, majority-or-sole-survivor termination) are exercised by this corrected example.

### 2.2 Boundary cases

- **Single candidate (`len(candidates) = 1`)** — trivial winner; tabulation short-circuits. Result: `winners = [that_candidate]`, `per_round_counts = [{that_candidate: ballots_tallied}]`, `eliminated_by_round = []`. `non_voters` lists the sole candidate iff they did not vote.

- **Two candidates, both vote, terminal 1-1 tie** — first round is the only round; no elimination needed. Result: `winners = [both candidates]`, `per_round_counts = [{A: 1, B: 1}]`, `eliminated_by_round = []`.

- **All candidates abstain (`len(ballots) = 0` at `end_at`)** — every candidate has 0 first-place votes; all tied for lowest; batch elimination would empty the set, so they are instead declared co-winners. Result: `winners = candidates`, `per_round_counts = [{c: 0 for c in candidates}]`, `eliminated_by_round = []`, `ballots_tallied = 0`, `non_voters = candidates`. Result is mathematically defined but practically meaningless; UI is responsible for surfacing as "no result" if desired.

- **Some candidates abstain (`0 < len(ballots) < len(candidates)`)** — tally proceeds with submitted ballots only. Majority threshold is `> ballots_tallied / 2`, not `> len(candidates) / 2`. `non_voters` enumerates the abstaining candidates by address.

- **All ballots dropped as malformed (`ballots_tallied = 0`, `ballots_dropped > 0`)** — same shape as all-abstain, but `non_voters = ∅` and `ballots_dropped = N`. Visible on-chain that ballots were cast but each was rejected at decryption / parse / permutation-check.

### 2.3 Edge cases

- **Late ballot during Tallying** — `submit_ballot` after `end_at` is rejected with `NotInVotingWindow`. Ballot store is frozen at the `end_at` boundary.
- **Ballot revision** — same voter calls `submit_ballot` multiple times during Voting; last-write-wins. Each call overwrites `ballots[msg.sender]`. No revision log; only the most recent encrypted blob is retained on-chain.
- **Malformed ballot content** (detected by the enclave at tally time):
  - Decryption fails (corrupted ciphertext, wrong key) → dropped.
  - Decrypted plaintext does not parse as `Vec<Addr>` → dropped.
  - Parsed list is not a permutation of `candidates` (contains duplicate, missing, or non-candidate entry) → dropped.
  - Each drop increments `ballots_dropped` and adds the offending voter's address to `dropped_voters` in the tally result.
- **Attestation race** — enclave produces a valid attestation for tally `T`. Before `publish_result` is mined, no ballots can change (voting is closed). The enclave's read is consistent. No race possible by construction.
- **Enclave never publishes** — election stays in Tallying indefinitely. No admin recovery; no auto-restart. The election is effectively dead. Accepted as a design tradeoff this round (see Non-Goals).
- **Multiple `close_and_tally` calls** — handler is purely event-emit + validation; second call emits another event, no state change, no error. Idempotent by construction.
- **Instantiation with `start_at` close to `env.block.time`** — admissible as long as `start_at > env.block.time`; the contract enters Voting state on the very next block after `start_at`.
- **Identical encrypted-ballot bytes from two voters** — unrelated to correctness; each ballot is bound to its submitting candidate by `msg.sender`. No deduplication concern.

### 2.4 Explicit failures

Consolidated `ContractError` variants. Each rejects the transaction atomically; no state mutates on failure.

| Error | Cause | Recoverable? |
|-------|-------|--------------|
| `InvalidInstantiation { reason }` | Empty/duplicate candidates, `start_at` in the past, `end_at ≤ start_at`, DstackKeyManager key derivation failure | No — instantiation never completes; redeploy with corrected parameters |
| `NotInVotingWindow` | `submit_ballot` outside `[start_at, end_at)` | Yes if pre-window; No if post |
| `NotACandidate` | `submit_ballot` from address not in `candidates` | No — non-candidates cannot vote in this election |
| `EmptyBallot` | `submit_ballot` with empty `encrypted_preferences` | Yes — resubmit with valid encryption |
| `VotingStillOpen` | `close_and_tally` or `publish_result` before `end_at` | Yes — wait for `end_at` |
| `AlreadyResolved` | Any state-mutating handler after `tally_result.is_some()` | No — election is terminal |
| `NotInTallyingState` | `publish_result` called when derived state is not Tallying | Yes if state was Created/Voting (wait); No if state was Resolved (terminal — see `AlreadyResolved`) (Round 3a first-pass adversary Attack 27) |
| `AttestationFailed { reason }` | TDX quote verification failure, zkdcap proof mismatch, wrong enclave identity, or chain-side vkey unregistered/misregistered | Conditional — recoverable if cause is enclave-side (re-attest); non-recoverable if cause is chain-registry-side (Section 4.3a) or chain-governance-side (Section 4.3b). Round 3a first-pass adversary Attack 15 reconciliation. |
| `InvalidTally { reason }` | Result fails well-formedness (winner ∉ candidates, counts don't conserve, elimination sequence inconsistent, set-relation violation, batch-elim policy violation) | Conditional — requires enclave image update, not in-instance. A deterministic enclave with frozen ballots will re-produce the same `InvalidTally` rejection on retry; recovery requires deploying a corrected enclave image and re-registering. See Section 4.9 for the deadlock case. Round 3a first-pass adversary Attack 11 reframe. |

### 2.5 Structured behavior blocks

**State variables (contract storage):**

- `candidates: Vec<Addr>` — declared at instantiation, immutable
- `start_at, end_at: Timestamp`
- `enclave_pubkey: PubKey` — generated at instantiation via DstackKeyManager
- `ballots: Map<Addr, Vec<u8>>` — candidate → encrypted_preferences; absent = no vote
- `tally_result: Option<TallyResult>` — None until published; immutable once set

**`TallyResult` schema** (added per Round 3a first-pass adversary Attack 34 — earlier draft inferred this from worked examples only):

```
struct TallyResult {
    winners: Vec<Addr>,                       // 1 ≤ len ≤ len(candidates); ⊆ candidates
    per_round_counts: Vec<Map<Addr, Nat>>,    // indexed by round; entry per surviving candidate
    eliminated_by_round: Vec<Vec<Addr>>,      // candidates eliminated in each round (parallel to per_round_counts)
    ballots_tallied: Nat,                     // ballots successfully decrypted + validated as permutations of `candidates`
    ballots_dropped: Nat,                     // ballots that failed decryption / parse / permutation-check at the enclave
    dropped_voters: Vec<Addr>,                // voters whose ballots were dropped; ⊆ ballots.keys; len = ballots_dropped
    non_voters: Vec<Addr>,                    // candidates who did not submit a ballot; = candidates \ ballots.keys
}
```

`per_round_counts[i]` lists only surviving candidates for round `i`; addresses absent from `per_round_counts[i]` are interpreted as eliminated before round `i`. Ordering of entries within each map and within `Vec<Addr>` fields is **deterministic-by-candidate-declaration-order** (the order in `candidates` at instantiation); this matters for downstream Lean / Quint encoding to produce reproducible serialization.

**Canonical serialization** (added per Round 3a re-adversarial pass — themes 6+13, mistral A10 / qwen A4 / kimi A10 / kimi A7; **leaf-encoding pins added per v0.3.1 Round 3a subagent-dispatch synthesis T7** — 3 voices flagged the prior version as under-specified at the `Addr` / `Nat` level; **`election_id` added v0.3.8 M2; `chain_id` added v0.3.10 N4 for cross-chain replay defense; `ballots_hash` added v0.3.11 B6 to discharge §8.7 link 7 enclave_input_fidelity**): every appearance of `canonical_serialization(x)` in this doc (most notably B8 clause (c) and Block E1's attestation construction) refers to **Borsh** serialization per the CosmWasm-native discipline. Concretely:
- `canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body)` denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(chain_id) ‖ u64_LE(election_id) ‖ ballots_hash[32] ‖ borsh_bytes(tally_body)`, where `‖` is byte concatenation. The `chain_id` field (v0.3.10 N4) sits between `contract_addr` and `election_id`. The `ballots_hash` field (v0.3.11 B6) is **32 raw bytes** (no length prefix; fixed size) sitting between `election_id` and `tally_body`, computed as `SHA-256(u32_LE(included_count) ‖ for each entry in candidate-declaration order: Borsh(voter: String) ‖ Borsh(ciphertext: bytes))`. The contract reads `env.block.chain_id` + computes `ballots_hash` from `BALLOTS.range(...)` at PublishResult time; the runtime computes `ballots_hash` from its received `raw_ballots`. Divergence → `AttestationCommitMismatch` on chain.
- `tally_body` is the `TallyResult` struct above, serialized field-by-field in declaration order. `Vec<Addr>` fields and `Map<Addr, Nat>` fields are serialized as the concrete Rust type `Vec<(Addr, Nat)>` (for Maps) or `Vec<Addr>` (for Vecs), populated in **candidate-declaration order**. **Implementations are required to use ordered-sequence types (`Vec<T>`); `BTreeMap` and `HashMap` are explicitly forbidden for tally-output fields** because their Borsh encodings diverge from declaration-order on non-trivial inputs.
- **Leaf encodings**: `Addr` is serialized as Borsh `String` (u32-LE length prefix followed by UTF-8 bytes of the bech32 representation). `Nat` is serialized as `u64` little-endian. These pins make `borsh_bytes(tally_body)` a deterministic function of the logical value at the byte level.
- The decrypted-plaintext format for ballots in Block E1 Stage 1 is **the same Borsh encoding** of `Vec<Addr>` (the voter's preference list); the parser in Stage 1 is `borsh_decode::<Vec<Addr>>` followed by the permutation-of-`candidates` validation.
Two implementations of the enclave that produce different byte representations for the same logical input therefore disagree on B8(c) and one of them is non-compliant.

**Gnark verification primitive — chain-side direct query** (added v0.3.9 per N1 audit re-review). The chain verifies enclave attestations via a **direct gRPC query** to the Xion `xion.zk` module from inside the verified-rcv contract:

```
QueryRequest::Grpc {
    path: "/xion.zk.v1.Query/ProofVerifyGnark",
    data: encode(QueryVerifyGnarkRequest {
        proof:         <gnark Groth16 proof bytes>,
        public_inputs: <306 fr-element BE bytes; see layout below>,
        vkey_name:     <chain-registered xion.zk vkey row name; see §6.1>,
        vkey_id:       0,
    }),
}
```

The chain returns `ProofVerifyGnarkResponse { verified: bool }`. **The verified-rcv contract no longer embeds a `DstackEnvelope` wrapper**; `publish_result` and `create_election` (the multi-election form of instantiate) carry `proof` and `public_inputs` directly. The `xion.zk.v1.Query/ProofVerifyGnark` semantics are trusted per §6.1 Chain Trust (the chain's ZK module is responsible for correct gnark Groth16 BN254 verification against the registered vkey).

**Gnark public_inputs byte layout — normative** (added v0.3.9). For the zkdcap reference DCAP gnark circuit (`/Users/mvid/Development/reliq/zkdcap/circuits/dcap-gnark/circuit/types.go:99-108`), the `public_inputs` blob is **9,792 bytes**, encoding 306 BN254 fr field elements in **declaration order** as concatenated 32-byte big-endian values (per Xion proto `QueryVerifyGnarkRequest.public_inputs`):

| Field | Type | fr-element count | Byte range | Notes |
|---|---|---:|---|---|
| MrTd | `[48]uints.U8` | 48 | `[0..1536]` | TDX MRTD (48-byte SHA-384 measurement) |
| Rtmr0 | `[48]uints.U8` | 48 | `[1536..3072]` | TDX RTMR0 |
| Rtmr1 | `[48]uints.U8` | 48 | `[3072..4608]` | TDX RTMR1 |
| Rtmr2 | `[48]uints.U8` | 48 | `[4608..6144]` | TDX RTMR2 |
| Rtmr3 | `[48]uints.U8` | 48 | `[6144..7680]` | TDX RTMR3 |
| ReportData | `[64]uints.U8` | 64 | `[7680..9728]` | TDX `report_data` (purpose-tagged 32+32 — see ReportData layout below) |
| TcbStatus | `frontend.Variable` | 1 | `[9728..9760]` | Merged platform+QE TCB severity (0..6, lower = better; 6 = Revoked rejected by circuit) |
| Timestamp | `frontend.Variable` | 1 | `[9760..9792]` | Witness timestamp (u64, seconds-since-epoch) |

**Per-element decoding discipline**: each `uints.U8` is encoded as a single fr field element whose value is the byte's integer value in `[0, 256)`. In 32-byte BE serialization, this means the **31 high bytes MUST be zero** and the byte value sits at offset `i*32 + 31`. The verified-rcv contract enforces this invariant on every verify — any element with a non-zero high byte is rejected as `GnarkPublicInputNotU8 { elem_idx }`. This makes the contract self-validating: if the gnark layout assumption is wrong, the first verify fails fast rather than silently accepting malformed inputs.

For `TcbStatus` and `Timestamp` (full-range frontend.Variable), the contract reads the value as a u64 BE from the last 8 bytes of the 32-byte chunk, asserting the high 24 bytes are zero (bounds check; both values fit in u64 by circuit construction).

**`ReportData` 64-byte layout — purpose-tagged** (added v0.3.9 per N1 audit re-review). The TDX `report_data` field is 64 bytes. verified-rcv splits it into two domain-separated halves:

```
ReportData = data[0..32] ‖ DST_TAG[32..64]
```

where:

- **`data[0..32]`** carries a SHA-256 digest whose preimage depends on the quote purpose:
  - **Registration quote** (issued by the enclave at boot or pre-`CreateElection`): `data = SHA-256(enclave_pubkey)` where `enclave_pubkey` is the 33-byte compressed secp256k1 public key the enclave will use as the ECIES recipient identity. Binding via SHA-256 (collision-resistance hypothesis `Adv_pubkey_CR` carried as a degenerate case of `Adv_commitTally_CR`; same primitive, same hardness).
  - **Publish quote** (issued by the enclave at tally completion): `data = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))` per v0.3.8 M2 — unchanged from v0.3.8 except for residence: was lower 32 bytes of `user_data`; is now `ReportData[0..32]` in the gnark public_inputs blob.

- **`DST_TAG[32..64]`** carries a 32-byte purpose tag, zero-padded if the ASCII label is shorter:
  - **Registration quote**: `DST_VERIFIED_RCV_ENCLAVE_PUBKEY_V1` (the 34-byte ASCII label is truncated to 32, or — equivalent and used in code — `DST_VERIFIED_RCV_PUBKEY_V1` (26 bytes) zero-padded to 32. Choice is pinned in §3.2 B8(e) and the implementation.).
  - **Publish quote**: `DST_VERIFIED_RCV_TALLY_V1` (25 ASCII bytes, zero-padded to 32) — same as v0.3.8.

Domain separation between registration and publish quotes prevents cross-purpose replay (an attacker who obtained a publish quote cannot reuse its proof+public_inputs as a registration quote, even though both quotes use the same gnark circuit + same vkey, because `data[0..32]` and `DST_TAG[32..64]` will disagree with the registration verification path).

**Encoding discipline (v0.3.9)**: the verified-rcv contract computes the expected `ReportData` value from its own inputs (`SHA-256(enclave_pubkey)` for registration or `SHA-256(canonical_serialization(...))` for publish) and compares to the extracted `data[0..32]` byte-for-byte; the `DST_TAG[32..64]` is compared to a hard-coded 32-byte constant. Both comparisons are constant-length and constant-time-safe. Mismatch produces `AttestationReportDataMismatch { expected, actual }` (publish) or `AttestationPubkeyBindingMismatch` (registration).

**Derived states (from `env.block.time` and `tally_result`):**

- **Created**: `env.block.time < start_at`
- **Voting**: `start_at ≤ env.block.time < end_at`
- **Tallying**: `env.block.time ≥ end_at ∧ tally_result.is_none()`
- **Resolved**: `tally_result.is_some()` (terminal)

**Transaction trace model** (added per Round 3a re-adversarial pass — theme 3 prerequisite; **revised per v0.3.1 subagent-dispatch synthesis T6** — 4 voices flagged absent-key semantics and slot-vs-message ambiguity in the prior form). The state machine has an implicit *contract-message trace* alongside the state variables above. A contract message `msg` is the unit of state transition: each chain transaction (`MsgExecuteContract` / `MsgInstantiateContract`) carries one or more such messages, each delivered to the contract handler. A contract message has at least `msg.kind ∈ {Instantiate, SubmitBallot, CloseAndTally, PublishResult}`, `msg.sender: Addr` (the direct caller of the contract — the `MessageInfo.sender` value in CosmWasm semantics), and a kind-specific payload (`msg.encrypted_preferences` for `SubmitBallot`; `msg.tally + msg.attestation` for `PublishResult`).

A state transition `σ → σ′` is parameterized by the **ordered list** `fires_at_transition(σ → σ′)` of contract messages that fired in the block taking `σ` to `σ′`, in CosmWasm's deterministic execution order. **Instantiation is modelled as the transition from a designated `σ⊥` (uninhabited contract slot, before contract creation) to `σ_init`** — B-series invariants of the form `always P` are read as "P at all σ in the post-σ_init trajectory."

**Absent-key semantics**: `ballots: Map<Addr, Vec<u8>>` is a partial map. For `k ∉ ballots.keys`, the value `ballots[k]` lifts to `⊥` (`None`). Value-level comparisons `next.ballots[k] ≠ ballots[k]` in B6 and similar are read as `Option<Vec<u8>>` inequality: the predicate holds whenever `k` transitions between any pair of `None / Some(v) / Some(v')`. Map entries are not deleted in this spec, so the relevant case is absent-to-present (first write) and present-to-different (overwrite).

**Uniqueness**: when a B-series invariant of the form `∃ msg ∈ fires_at_transition(σ → σ′), P(msg)` is used to attribute a state change to a specific message, the existential is read as requiring `msg` to be the **unique** message in that firing set whose write produced the state change. If multiple messages co-occurring in the same firing set could have produced the same change, B6 is read with the additional uniqueness condition: there is exactly one such `msg`, and that `msg` is the named writer.

Downstream Quint encodes this as an action label on each transition; downstream Lean encodes it as a relation parameter on `step : State → Msg → State → Prop` (Lean operates on a single-message step relation; `fires_at_transition` is the trace projection that aggregates per-block messages). This model is the formal substrate for B6's writer attribution, B8's attestation binding, and any other invariant whose witness is the firing message rather than a static state property.

**Frozen ballot lookup — `ballots@end_at`** (added per v0.3.1 subagent-dispatch synthesis F1): the notation `ballots@end_at` denotes the value of `state.ballots` at the unique chain state `σ_first` where `env.block.time(σ_first) ≥ end_at` and `env.block.time(predecessor of σ_first) < end_at`. By B2 (no late ballots), `ballots = ballots@end_at` at every σ in the Tallying or Resolved derived state, so the two are extensionally equal at the moment B10 is evaluated. The notation is retained because B10's RHS is morally about "the ballot set the enclave saw at tally time"; downstream Lean specs may equivalently use `state.ballots` paired with a B2 lemma rewrite.

**`EnclaveImage` type signature** (added per v0.3.1 subagent-dispatch synthesis T1 — 5 voices flagged the symbol as undefined in v0.3.0). `EnclaveImage` is the **semantics of the Lean-extracted model of the enclave binary**, with type signature:

```
EnclaveImage : RawBallots × CandidateSet × PrivKey → TallyResult
```

where `RawBallots = Map<Addr, Vec<u8>>`, `CandidateSet = Vec<Addr>`, `PrivKey = Bytes32` (or the concrete dstack-KMS-derived key type). `EnclaveImage` is the externally-observable input-output relation of the enclave binary, modeled in Lean. It is a **fixed term** in the downstream Lean spec, sourced from the enclave binary via a documented extraction discipline (e.g., Aeneas Rust-to-Lean extraction, or hand-written Lean translation of the Rust source with stated extraction-soundness obligations). Image-identity-binding (§8.7 step 4) ties the chain-registered MRTD/RTMR to the build artifact of the Rust crate from which `EnclaveImage` was extracted; a proof author may **not** instantiate `EnclaveImage` freely (e.g., setting `EnclaveImage := Tally_spec` to discharge B10_lean by `rfl` is forbidden — the symbol is reserved for the extracted model). The earlier 1-ary `EnclaveImage : Bytes → Bytes` reference (the byte-level extraction step) is renamed to `extract_enclave_model : Binary → LeanTerm` — see §8.7 step 4 for its role.

**IRV algorithm — the specification B10 references:**

The full tally pipeline is **`Tally_spec(raw_ballots, candidates, enclave_privkey) → TallyResult`** — the function B10 names. It composes two stages:

**Stage 1 — `decrypt_and_validate(raw_ballots, candidates, enclave_privkey) → (valid_ballots: Map<Addr, Vec<Addr>>, dropped_voters: Vec<Addr>, non_voters: Vec<Addr>)`**:

- Iterate `raw_ballots` in **candidate-declaration order** restricted to `raw_ballots.keys`: for each `addr ∈ candidates` such that `addr ∈ raw_ballots.keys`, retrieve `ciphertext = raw_ballots[addr]`, ECIES-decrypt under `enclave_privkey`, parse as `Vec<Addr>`, validate as a permutation of `candidates`. On success, `valid_ballots[addr] := decrypted_preferences`. On any failure (decrypt / parse / not-a-permutation), `addr` is appended to `dropped_voters` and the entry is omitted from `valid_ballots`. **The candidate-declaration-order iteration discipline is load-bearing** (added per v0.3.1 subagent-dispatch synthesis T5 — 4 voices flagged the prior "Map iteration discipline" wording as undefined): the underlying `Map` iteration in CosmWasm storage is lexicographic by storage key, which differs from candidate-declaration order. Pinning iteration to the candidates list makes Stage 1 a deterministic function of its logical inputs and ensures both `dropped_voters` ordering and `valid_ballots` population order are reproducible across honest implementations.
- `non_voters := candidates \ raw_ballots.keys`, emitted in **candidate-declaration order**.
- `dropped_voters` ordered by the candidate-declaration-order iteration above.

**Stage 2 — `IRV_spec(valid_ballots, candidates) → (winners, per_round_counts, eliminated_by_round, ballots_tallied)`**:

Defined recursively over rounds:

1. **Base case** (single candidate or terminal-tie): if `|remaining| = 1`, the sole remaining candidate is the winner; if all remaining candidates have equal first-place counts in the current round, **all of them are co-winners**.
2. **Majority termination**: if some candidate `c` has strictly more than half the current round's first-place votes (`count[c] > sum(count) / 2`), `c` wins.
3. **Recursive case** (batch elimination): otherwise, let `min_count = min(count.values())`; let `losers = {c : count[c] = min_count}`. Eliminate all candidates in `losers`. Redistribute each loser's ballots: for each ballot whose current top-ranked candidate is in `losers`, advance to the next surviving rank. Recurse on `remaining \ losers`.

`ballots_tallied := |valid_ballots|` (the count of ballots fed into the recursion).

**Encoding discipline (v0.3.3 from lean-cross-critique-2026-05-20 finding A5)**: downstream Lean specs that declare `IRV_spec` as `opaque` (or `axiom`) MUST also declare an axiom tying `ballots_tallied` to `|valid_ballots|`. The shape is:

```lean
axiom irv_ballots_tallied :
  ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
    (IRV_spec valid cs).ballots_tallied = valid.length
```

Without this axiom, an opaque `IRV_spec` can return any `Nat` for `ballots_tallied` while the composition still typechecks, leaving S7 and S8 making claims about a value that is not constrained by Stage 2's specification. The axiom restores the Stage-2 obligation that this paragraph's `:=` defines. Downstream Quint specs that nondeterministically choose a `ballots_tallied` value in the protocol model's tally action implicitly enforce this via the action's well-formedness conjunct; the explicit axiom is the Lean-side equivalent.

**Length relation between `eliminated_by_round` and `per_round_counts`** (added v0.3.4 from lean-critique-revised-canonical-2026-05-20 meta-analysis Q3 / kimi optional notes): the IRV recursion produces `per_round_counts.length` rounds total. In each non-terminal round, exactly one batch of candidates is eliminated and recorded in `eliminated_by_round`; the terminal round (either majority-found or single-candidate-remaining or all-remaining-tied) does NOT add an entry to `eliminated_by_round`. Therefore:

- **Typical case** (majority found or single-candidate-remaining at round N): `eliminated_by_round.length = per_round_counts.length - 1`. The terminal round records the winning count without recording further elimination.
- **Terminal-tie boundary case** (all remaining tied at round N, batch-elimination would empty remaining → co-winners): `eliminated_by_round.length = per_round_counts.length - 1` STILL holds because round N's count map is recorded but no further elimination happens.
- **All-abstain boundary case** (`|valid_ballots| = 0`): `per_round_counts = [{c: 0 for c in candidates}]` (single round, all candidates with zero votes), `eliminated_by_round = []`. So `eliminated_by_round.length = 0`, `per_round_counts.length = 1`, relation `per_round_counts.length = eliminated_by_round.length + 1` holds.

In all cases: **`per_round_counts.length = eliminated_by_round.length + 1`**.

**Encoding discipline (v0.3.4 from lean-critique-revised-canonical-2026-05-20 Q3 finding A6)**: downstream Lean specs MAY encode this length relation as a well-formedness predicate alongside S6–S9. It is not strictly required (S9's `i < j` formulation handles index bounds via Option-pattern matching), but encoding it strengthens the structural reading of `IRV_spec`'s output and closes a small gap in S9's preconditions. If encoded, the axiom shape would be:

```lean
axiom irv_round_eliminated_lengths :
  ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
    (IRV_spec valid cs).per_round_counts.length =
      (IRV_spec valid cs).eliminated_by_round.length + 1
```

A6 is recommended but not mandatory; the structural well-formedness theorems S6–S9 are dischargeable without it (verified-rcv's `specs/RcvSpec.lean` proves them against the 4 Stage-1/Stage-2 axioms added in v0.3.3 + the existing `irv_ballots_tallied`).

Edge cases:
- **All remaining candidates tied for lowest**: batch elimination would empty `remaining`. In this case, declare all current `remaining` candidates as co-winners (per "ties → multi-winner" policy).
- **Zero ballots**: trivially all-tied at zero; co-winners = all candidates (see Section 2.2 boundary case).

This is the canonical IRV variant used by Australian federal parliament (full-preferential, batch-elimination by lowest tied set). The algorithm terminates in at most `len(candidates)` rounds.

**Composition (Tally_spec)**: `Tally_spec(raw_ballots, candidates, enclave_privkey)` returns the `TallyResult` whose:
- `(winners, per_round_counts, eliminated_by_round, ballots_tallied)` fields come from `IRV_spec(valid_ballots, candidates)`,
- `ballots_dropped := len(dropped_voters)`, `dropped_voters`, `non_voters` come from Stage 1.

`Tally_spec` is the function B10 (Section 3.2) refers to; `IRV_spec` is its inner combinatorial core, isolated so the Lean proof obligation cleanly factors into a Stage-1 decryption-correctness lemma plus a Stage-2 IRV-correctness theorem (the bulk of the methodology work).

**Batch-elimination safety note** (Round 3a first-pass adversary Attack 10): the chosen variant always batch-eliminates all candidates tied at the lowest count, including the case where this would produce an "early co-winner" outcome (sum of eliminated ≥ next-survivor count). This is the **batch-elim-with-multi-winner-fallback** rule, intentionally simpler than the ACE-rules variant (which forbids batch elimination when `sum(eliminated) ≥ min(survivor)`). The simpler variant is chosen because it matches the user's "ties → multi-winner" decision policy from the elicitation pass.

#### Block 1: instantiate

- **From state:** N/A (contract creation)
- **Trigger:** `instantiate { candidates, start_at, end_at }`
- **Requires:**
  - `len(candidates) ≥ 1`
  - all candidate addresses distinct
  - `start_at > env.block.time`
  - `end_at > start_at`
  - DstackKeyManager-issued keypair derivable for this contract instance
- **Forbids:** negation of each Requires clause
- **Produces:**
  - stored: `candidates`, `start_at`, `end_at`, `enclave_pubkey`
  - `ballots = ∅`, `tally_result = None`
- **To state:** Created
- **On precondition failure:** `ContractError::InvalidInstantiation { reason }`; instantiation atomically reverts.

#### Block 2: time advances to start_at *(implicit, no handler)*

- **From state:** Created
- **Trigger:** chain block produced with `env.block.time ≥ start_at` (passive)
- **Requires:** `env.block.time ≥ start_at`
- **Forbids:** (none)
- **Produces:** no storage change; derived state advances.
- **To state:** Voting

#### Block 3: submit_ballot

- **From state:** Voting
- **Trigger:** `ExecuteMsg::SubmitBallot { encrypted_preferences: Vec<u8> }`
- **Requires:**
  - derived state is Voting
  - `msg.sender ∈ candidates`
  - `encrypted_preferences` non-empty
- **Forbids:**
  - derived state ≠ Voting
  - `msg.sender ∉ candidates`
  - empty `encrypted_preferences`
- **Produces:** `ballots[msg.sender] := encrypted_preferences` (last-write-wins)
- **To state:** Voting (self-loop; storage update only)
- **On precondition failure:**
  - state ≠ Voting → `NotInVotingWindow`
  - non-candidate → `NotACandidate`
  - empty preferences → `EmptyBallot`

#### Block 4: time advances to end_at *(implicit, no handler)*

- **From state:** Voting
- **Trigger:** chain block produced with `env.block.time ≥ end_at` (passive)
- **Requires:** `env.block.time ≥ end_at`
- **Forbids:** (none)
- **Produces:** no storage change; from this point, ballots are immutable.
- **To state:** Tallying

#### Block 5: close_and_tally

- **From state:** Tallying
- **Trigger:** `ExecuteMsg::CloseAndTally {}` (any chain address may call)
- **Requires:** derived state is Tallying
- **Forbids:**
  - `env.block.time < end_at`
  - `tally_result.is_some()`
- **Produces:** emits a Cosmos SDK event for off-chain monitoring (UI / enclave-watcher); no storage change.
- **To state:** Tallying (self-loop; idempotent — re-invocation emits another event but is harmless)
- **On precondition failure:**
  - voting still open → `VotingStillOpen`
  - already resolved → `AlreadyResolved`

#### Block 6: publish_result

- **From state:** Tallying
- **Trigger:** `ExecuteMsg::PublishResult { tally: TallyResult, proof: Vec<u8>, public_inputs: Vec<u8> }` (v0.3.9 N1 amendment — the prior `DstackAttestation` wrapper is removed; the gnark proof and its public_inputs are carried directly). **Any chain address may submit**; the enclave identity is verified via the carried `(proof, public_inputs)`, not via `msg.sender` (Round 3a first-pass adversary Attack 40). This is intentional: the trust anchor is the gnark attestation, not the on-chain sender; an adversary who replays the enclave's `(tally, proof, public_inputs)` payload from the mempool to a different `msg.sender` still finalizes the *same* result, so the replay is a no-op (replay-protection by `B5` derived-from-`B1` plus Block 6's `AlreadyResolved` rejection).
- **Requires:**
  - derived state is Tallying
  - `public_inputs.len() == 9_792` (per §2.5 gnark public_inputs byte layout)
  - **gnark proof verifies via `xion.zk.v1.Query/ProofVerifyGnark`** against the on-chain registry's `vkey_name` (see B8 clauses (a) + (b), v0.3.9 form)
  - the (MrTd, Rtmr1, Rtmr2, optionally Rtmr0, optionally Rtmr3) tuple extracted from `public_inputs` equals the corresponding on-chain registry fields (component-wise, per §6.1 registry layout) — see B8 clause (d)
  - `ReportData[0..32]` extracted from `public_inputs` equals `SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))` — see B8 clause (c)
  - `ReportData[32..64]` extracted from `public_inputs` equals `DST_VERIFIED_RCV_TALLY_V1` zero-padded to 32 bytes — see §2.5 ReportData layout
  - `TcbStatus` extracted from `public_inputs` is in the registry's `accepted_tcb_statuses` set (defaults: `{0 UpToDate, 1 SWHardeningNeeded, 2 ConfigurationNeeded, 3 ConfigurationAndSWHardeningNeeded}`; rejects `4 OutOfDate`, `5 OutOfDateConfigurationNeeded`, `6 Revoked` — the gnark circuit additionally hard-rejects `Revoked` inside the proof, so 6 should never reach this check)
  - tally is well-formed:
    - `winners ⊆ candidates`, `1 ≤ len(winners) ≤ len(candidates)`
    - per-round counts internally consistent (each `per_round_counts[i]` has all entries summing to `ballots_tallied`)
    - elimination sequence is monotone (no candidate reappears in `per_round_counts[j]` for `j > i` if they appear in `eliminated_by_round[i]`)
    - conservation: `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)`
    - **set relations** (Round 3a first-pass adversary Attack 28 + 35): `non_voters = candidates \ ballots.keys`, `dropped_voters ⊆ ballots.keys`, `dropped_voters ∩ non_voters = ∅`. `len(dropped_voters) = ballots_dropped`.
- **Forbids:** the negation of each Requires clause (with explicit error mapping in `On precondition failure` below per Round 3a first-pass adversary Attack 32)
- **Produces:** `tally_result := Some(tally)`; ballots become read-only audit data.
- **To state:** Resolved (terminal)
- **On precondition failure** (explicit Requires-clause → error mapping; v0.3.9 expanded mapping for gnark errors):
  - derived state ≠ Tallying → `NotInTallyingState`
  - `public_inputs.len() ≠ 9_792` → `GnarkPublicInputsLength { got, expected: 9_792 }`
  - any gnark `uints.U8` element has non-zero high bytes → `GnarkPublicInputNotU8 { elem_idx }`
  - any gnark `frontend.Variable` element exceeds u64 range → `GnarkPublicInputOutOfRange { elem_idx }`
  - `xion.zk.v1.Query/ProofVerifyGnark` returns `verified=false` → `ProofVerificationFailed`
  - extracted measurements (MrTd / Rtmr1 / Rtmr2 / Rtmr0-if-bound / Rtmr3-if-bound) differ from registry → `AttestationMeasurementMismatch { field, expected, actual }`
  - `ReportData[0..32]` differs from `SHA-256(canonical_serialization(...))` → `AttestationCommitMismatch { expected, actual }`
  - `ReportData[32..64]` differs from `DST_VERIFIED_RCV_TALLY_V1` padded → `AttestationDomainTagInvalid`
  - `TcbStatus` not in `accepted_tcb_statuses` → `AttestationTcbStatusUnaccepted { status }`
  - well-formedness fails (any sub-clause) → `InvalidTally { reason: WellFormednessClauseN }`

**Tally-correctness obligation** (B10): well-formedness above is *syntactic* — the published tally satisfies the structural invariants. **Semantic correctness** — i.e., `tally = Tally_spec(ballots, candidates, enclave_privkey)` (Section 2.5) — is enforced *by the enclave software*, not by the chain. The chain cannot independently verify B10 without re-running the IRV computation and decrypting the ballots (which it lacks the key for). The chain accepts B10 *on the strength of the enclave attestation* (B8): if the attested image is the verified-rcv image, and the verified-rcv image has been formally verified to implement `Tally_spec`, then B10 holds. Section 4.4 ("enclave software bug") is the failure mode where this chain meets reality: an attested enclave with a bug produces a syntactically well-formed but semantically wrong tally, which the chain accepts.

#### Cross-block discipline

- Every state has at least one entry path: Created (via Block 1), Voting (via Block 2 from Created), Tallying (via Block 4 from Voting), Resolved (via Block 6 from Tallying). ✓
- Resolved is terminal; no outbound transitions. ✓
- Two blocks fire from `From state = Tallying`: **Block 5 is a self-loop** (no storage change; emits event); **Block 6 is an outbound transition** to Resolved. Their Produces clauses are non-overlapping and their effects are distinct (Round 3a first-pass adversary Attack 38 terminology fix).

#### Block E1: enclave tally computation (off-chain, attested)

Added per Round 3a first-pass adversary Attack 36 — earlier draft treated the enclave as a black box; Section 4.4 names it as the load-bearing methodology target.

- **From state:** off-chain enclave observes `close_and_tally` event on chain (Block 5 emitted), OR polls the chain and sees derived state has advanced to Tallying.
- **Trigger:** event detection OR poll.
- **Inputs:**
  - `ballots: Map<Addr, Vec<u8>>` — read from contract storage at the first chain state where `env.block.time ≥ end_at`. Frozen per B2.
  - `enclave_privkey: PrivKey` — retrieved from DstackKeyManager after attesting against the chain-registered verified-rcv image.
  - `candidates: Vec<Addr>` — read from contract storage.
- **Requires:**
  - Successfully attested keypair retrieval (else fail without publishing).
  - Chain state advanced past `end_at` (do not act before).
- **Produces:** (equivalently: compute `Tally_spec(ballots, candidates, enclave_privkey)` per Section 2.5, then attest + submit)
  1. **Stage 1 — `decrypt_and_validate`**: for each `(addr, ciphertext) ∈ ballots`, ECIES-decrypt under `enclave_privkey`, parse as `Vec<Addr>`, validate as a permutation of `candidates`. Build `valid_ballots`, `dropped_voters`, `non_voters` per Section 2.5 Stage 1.
  2. **Stage 2 — `IRV_spec`**: run the IRV recursion on `(valid_ballots, candidates)` per Section 2.5 Stage 2; obtain `(winners, per_round_counts, eliminated_by_round, ballots_tallied)`.
  3. Construct `TallyResult` by combining Stage-1 and Stage-2 outputs per Section 2.5 Composition; this is `Tally_spec(ballots, candidates, enclave_privkey)`.
  4. **Request TDX quote** from `tappd` (dstack's Trusted App Daemon) with `report_data = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body)) ‖ DST_VERIFIED_RCV_TALLY_V1_PADDED` (per §2.5 ReportData layout — publish quote form).
  5. **Drive the zkdcap gnark prover** (via the host-side unix-socket prover service at `/Users/mvid/Development/reliq/zkdcap/host/src/gnark.rs`) to produce `(proof, public_inputs)` for the gnark circuit at `/Users/mvid/Development/reliq/zkdcap/circuits/dcap-gnark/circuit/dcap.go`. The `public_inputs` blob is the 9,792-byte layout pinned in §2.5; the proof is a gnark-native Groth16 BN254 serialization.
  6. Submit `ExecuteMsg::PublishResult { tally, proof, public_inputs }` to the chain (Block 6). The chain verifies via `xion.zk.v1.Query/ProofVerifyGnark` per §6.1; rejection paths are enumerated in Block 6's "On precondition failure" mapping.
- **To state:** unchanged on the chain side until Block 6 fires; the enclave itself transitions from idle → ran → published.
- **On failure:**
  - Attested keypair retrieval fails → Section 4.2 (KMS leakage class) or 4.6 (network partition) failure mode; enclave does not publish; election deadlocks per 8.3.
  - Gnark prover unreachable / prover timeout / proof generation error → enclave retries with exponential backoff per §4.9 retry policy; persistent failure → effective deadlock (variant of §4.11 resource exhaustion).
  - Chain rejects `publish_result` with `ProofVerificationFailed` / `AttestationMeasurementMismatch` / `AttestationCommitMismatch` / `AttestationDomainTagInvalid` / `AttestationTcbStatusUnaccepted` → re-attest, retry. Persistent rejection: Section 4.3a/4.3b (vkey unregistered or maliciously substituted) or Section 4.1 (TDX measurements changed under operator).
  - Chain rejects `publish_result` with `GnarkPublicInputsLength` / `GnarkPublicInputNotU8` / `GnarkPublicInputOutOfRange` → prover-side encoding bug; deterministic — enclave deadlocks (variant of §4.9).
  - Chain rejects `publish_result` with `InvalidTally` → enclave bug; deterministic re-run yields same rejection. Section 4.9 deadlock.
  - **Enclave produces a syntactically well-formed but semantically wrong `tally`** (passes Block 6 Requires, gnark proof valid, but `tally ≠ Tally_spec(ballots, candidates, enclave_privkey)`) → Section 4.4 (enclave software bug — the load-bearing methodology target); B10 violation; un-detectable on-chain. Mitigated by pre-deployment formal verification of the enclave image against B10.

**B10 is the correctness obligation on Block E1.** Pre-deployment formal verification of the enclave image against B10 + the well-formedness predicate (Block 6) is the central methodology target (Section 4.4). Block E1's input-output relation — `(raw_ballots, candidates, enclave_privkey) ↦ Tally_spec(raw_ballots, candidates, enclave_privkey)` — is the discharge obligation.

**Encoding discipline for protocol-layer model checkers (v0.3.2 from synthesis-2026-05-19 baseline-vs-rerun finding A2)**: a faithful Quint / TLA+ / equivalent model of this protocol MUST include enclave-side state distinct from chain-side state. Specifically, the model includes (at minimum):

1. **`enclave_consumed_ballots`** — a snapshot of what the enclave actually decrypted, distinct from the chain's `ballots@end_at`. Differs from `ballots@end_at` iff a malicious host fed substituted inputs (failure mode 4.4 / `enclave_input_fidelity` violation).
2. **`enclave_computed_tally`** — the enclave's output, distinct from `tally_result` published on chain. Equals what `publish_result` writes iff the enclave's computation and the chain's acceptance agree.
3. **`enclave_has_session_key`** — boolean liveness flag for KMS-derived key availability, distinct from on-chain state. Required for the `dstack_kms_trust` precondition of B10.

These three variables MUST be modeled with their own transitions (`enclave_tally` action distinct from `publish_result`) so that B10's chain-side projection invariant (`tally_result.value == Tally_spec_output(enclave_consumed_ballots, candidates) ∧ enclave_consumed_ballots == ballots@end_at`) is checkable as a state predicate.

Boolean-flag shadows (e.g., `attestation_valid: bool`, `tally_computed: bool`) are INSUFFICIENT — they collapse the chain↔enclave boundary that failure mode 4.4 is defined over and reduce B10 to a self-asserting tautology rather than a checkable cross-component projection.

## 3. Invariants

Properties that are always true. If any of these is violated, the system is broken regardless of input.

### 3.1 Structural invariants

Properties evaluable on contract state at any reachable moment. **No quantification over execution trajectory.** As of v0.3.1, this section's preamble is amended (per v0.3.1 subagent-dispatch synthesis T4 — 5 voices flagged the prior S5 form) to explicitly admit **handler-set inspection** as a non-trajectory form of static quantification: a structural invariant may quantify over the static set of contract handlers (`Block 1..Block 6`) without violating the "no quantification over operations or time" rule, provided the quantification produces a property of σ (or a code-level meta-property that constrains σ via the handler's Requires clauses). The trajectory claim ("once `Some`, always `Some`") remains in §3.2 as the temporal form B1.

| # | Invariant | Statement |
|---|-----------|-----------|
| S1 | non-empty candidates | `len(candidates) ≥ 1` |
| S2 | distinct candidates | `candidates` contains no duplicate addresses. **Encoding discipline (v0.3.3 from lean-cross-critique-2026-05-20 finding A4)**: downstream Lean specs MUST encode S2's distinctness either as a `cs.Nodup` (or equivalent) hypothesis on every theorem whose statement depends on `len(candidates)` (S6, S7, S8, S9, B10_lean), or as a bundled type-level invariant on `CandidateSet` (e.g., `CandidateSet := { cs : List Addr // cs.Nodup }`). A plain `CandidateSet := List Addr` declaration without the hypothesis silently strengthens S7's conservation equation to claim it holds for all lists including duplicate-containing ones, which is unprovable; the spec then typechecks but the load-bearing claim is wrong. Downstream Quint specs use `Set[Addr]` whose deduplication is built-in and need no additional encoding, but Lean's `List Addr` does not carry that guarantee. |
| S3 | well-ordered voting window | `start_at < end_at` |
| S4 | ballot keys are candidates | `∀ k ∈ ballots.keys, k ∈ candidates` |
| S5 (derived) | terminality of resolution (handler-set property) | **`tally_result` write-discipline**: the static handler set has the property that only Block 6 (`publish_result`) writes to `tally_result`, and Block 6's Requires include `tally_result.is_none()`. Therefore, by induction on the handler set, `tally_result` is written at most once across any execution trajectory. **This is a handler-set property** (admitted per the §3.1 preamble amendment in v0.3.1), not a trajectory quantification: the property `∀ h ∈ handlers, h writes tally_result ⇒ (h = Block 6 ∧ h.Requires ⊨ tally_result.is_none())` is a meta-property of the contract code, evaluable by inspecting the handler index alone. The trajectory claim "once `Some`, always `Some` with the same value" is **B1**'s temporal form (§3.2). **Restated per v0.3.1 subagent-dispatch synthesis T4** (5 voices: claude / kimi / mistral / gpt-oss / gemma flagged the prior "state-shape" framing as residual-temporal): the new framing names the property explicitly as handler-set rather than state-shape, and §3.1's preamble is amended to admit this category. **Severity-shadow note**: mistral elevated this finding to critical (treating the residual-temporal embedding as soundness-equivalent); the consensus tier is serious-with-critical-shadow. The handler-set framing closes the residual-temporal escape route by relocating the quantification from "successor states" to "the static handler index," which §3.1 now admits. |
| S6 | winner well-formedness | if `tally_result.is_some()`, then `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`. **Pointwise evaluable**; the temporal write-discipline complement (each `publish_result` write satisfies this) is captured by Block 6's Requires + B10's correctness obligation (Round 3a first-pass adversary Attack 7). **Encoding discipline (v0.3.5 from Round 3e concreteness pass finding A7)**: downstream Lean specs MUST include `1 ≤ cs.length` (or equivalently `cs ≠ []`) as a hypothesis on every S6 / `irv_winners_shape` theorem statement, since `cs.Nodup` alone admits `cs = []` (`[].Nodup = True` holds vacuously) — and for `cs = []`, the IRV core returns `winners = []` (per §2.5 Block E1 + the defensive zero-candidates path in the extracted enclave core), violating the `1 ≤ len(winners)` clause. The intent enforces `len(candidates) ≥ 1` at Block 1 instantiate (Requires clause), so S6 only applies *post-instantiation*; downstream Lean specs that fail to thread `1 ≤ cs.length` from S1 to S6 silently inherit an inconsistent axiom-statement when `IRV_spec` is declared `opaque`. Concretizing `IRV_spec` (Round 3e) surfaces this gap by making the inconsistency type-checkable. Same propagation pattern as A4 (cs.Nodup) and A5 (ballots_tallied); the cs.length ≥ 1 / cs.Nodup pair is what intent §3.1 S1 + S2 demand jointly. Bundle-type alternative (`CandidateSet := { cs : List Addr // cs.Nodup ∧ 1 ≤ cs.length }`) is acceptable and equivalent. |
| S7 | tally count conservation | if `tally_result.is_some()`, then `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)`. Pointwise evaluable per S6's note. |
| S8 | per-round count consistency | each `per_round_counts[i]` has all entries summing to `ballots_tallied`. Pointwise evaluable per S6's note. **Encoding discipline (v0.3.6 from A8)**: downstream Lean specs MUST include `∀ (_, b) ∈ valid_ballots, ∀ c ∈ cs, c ∈ b.ranking` as a hypothesis. Without it, an exhausted-ballot (ranking ∩ cs = ∅) yields `Σ = 0 ≠ ballots_tallied`. Stage 1 enforces this via permutation validation; the hypothesis propagates that to Stage 2. |
| S9 | elimination monotonicity | if a candidate appears in `eliminated_by_round[i]`, they appear in no `per_round_counts[j]` for `j > i`. Pointwise evaluable per S6's note. **Encoding discipline (v0.3.6 from A8)**: same hypothesis as S8. Without it, the all-abstain branch can trigger at deep recursion (`total = 0` at step `k > 0` due to exhausted ballots), writing `per_round_counts[k] = cs.map(c => {c, 0})` which reintroduces every candidate (including those in `eliminated_by_round[0..k-1]`), violating monotonicity. The hypothesis makes the all-abstain branch unreachable for non-empty `valid`. |
| S10 (derived) | resolution implies past end_at | if `tally_result.is_some()`, then `env.block.time ≥ end_at`. **Derived from B3** (temporal causal version); listed for explicit state-shape reference per Round 3a first-pass adversary Attack 26. |

### 3.2 Behavioral invariants

The invariant tag is load-bearing per Colosseum methodology. As of v0.3.1, the tag taxonomy (formerly implicit) is enumerated per v0.3.1 subagent-dispatch synthesis T17:

- **state** — pointwise on σ; discharged by an unquantified predicate over chain state. Lives in §3.1.
- **temporal** — LTL-style; requires history quantification downstream (Quint action labels, Lean `step : State → Msg → State → Prop` relations).
- **cross-layer** — the witness path traverses an off-chain layer (Lean-discharge proof, build-reproducibility binding, etc.). Cannot be discharged by chain trajectories alone.
- **off-chain** — the invariant body itself lives entirely off-chain (pure Lean theorem about an extracted model). Composes with cross-layer invariants but has no chain-side witness.
- **meta-security** — a cryptographic / probabilistic claim with explicit adversary + security-parameter quantifiers. Discharged by cryptographic reduction proofs, not by trajectory inspection. **New in v0.3.1** to host B9 after the retag (T9).

The wrong tag silently mis-encodes intent — the failure mode the `colosseum-spec-adversary` `temporal_state_mismatch` attack category is tuned to find.

| # | Invariant | Tag | Statement |
|---|-----------|-----|-----------|
| B1 | tally_result monotone-once-set | **temporal** | `always (tally_result.is_some() → always (tally_result.is_some() ∧ next.tally_result = tally_result))` — once an election resolves, it stays resolved with the same value forever |
| B2 | no late ballots | **temporal** | `always (env.block.time ≥ end_at → ballots = ballots@end_at)` — once voting closes, the ballot store is frozen for all time. **Encoding discipline (v0.3.2 from synthesis-2026-05-19 baseline-vs-rerun finding A3)**: protocol-layer model checkers (Quint, TLA+, etc.) MUST encode B2 as a checkable state invariant over a snapshot variable that captures `ballots` at the moment `env.block.time` first crosses `end_at`. Pure action-guard disablement ("`submit_ballot` requires `isVoting`, therefore no late writes are possible") is insufficient as a B2 encoding — it relies on the action set being structurally complete and provides no checkable predicate. The snapshot variable may be ghost (audit-only) or regular state; the invariant body is `ballots ⊆ snapshot AND every snapshot key equals ballots[key]` or equivalent. |
| B3 | no premature tally (causal) | **temporal** | `always (next.tally_result.is_some() ∧ tally_result.is_none() → env.block.time ≥ end_at)` — the resolution transition requires voting to have closed at the moment it fires |
| B4 | no premature voting | **temporal** | `always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)` — ballots only change during the open voting window |
| B5 (derived) | publish_result fires at most once | **temporal** | `always (tally_result.is_none() ∧ next.tally_result.is_some() → no further publish_result tx ever succeeds)`. **Derived corollary of B1 + Block 6's `AlreadyResolved` rejection**; listed for explicit reference only, not as an independent constraint (Round 3a first-pass adversary Attack 33). |
| B6 | ballot writer is the ballot voter | **temporal** | `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃! msg ∈ fires_at_transition(σ → σ′), msg.kind = SubmitBallot ∧ msg.sender = k ∧ next.ballots[k] = msg.encrypted_preferences)` — *for every key whose ballot value changed at the firing transition*, the change is attributable to a **unique** `SubmitBallot` message in that transition whose `msg.sender` equals the key. The `∀-per-key + ∃!-unique-msg` form is **load-bearing** (per v0.3.1 subagent-dispatch synthesis T6 — 4 voices flagged the prior `∃` form as admitting non-causal attribution): it forces *each* key change to be attributed to a corresponding message *and* rules out scenarios where some other unauthorized message also writes to `ballots[k]` with a happen-to-match value. Comparison `next.ballots[k] ≠ ballots[k]` lifts to `Option<Vec<u8>>` inequality per §2.5's absent-key semantics (the predicate holds on absent-to-present, present-to-different, present-to-absent transitions). **Restated per Round 3a re-adversarial pass — theme 3 (claude C3, gpt-oss C1+S7, kimi A2, qwen S5, glm S1, mistral C1)** and **further sharpened per v0.3.1 subagent-dispatch synthesis T6 (claude / kimi-temporal-inv / kimi-behaviors-types / gpt-oss / gemma)**: the prior `∃ tx` form was satisfied by *any* historical message and didn't bind to the firing transition, didn't rule out unauthorized co-occurring writers, and was silent on the absent-key case. The new form (a) uses `∃!` for uniqueness, (b) uses `msg` (contract message, not chain transaction — see §2.5 trace model) to match CosmWasm semantics where one chain tx can carry many messages, (c) lifts the comparison through `Option`. The transaction-trace model `fires_at_transition(σ → σ′)` is defined in §2.5 above (transaction trace model paragraph). Block 3's Requires (`msg.sender ∈ candidates`) is what makes B6 hold; B6 is the temporal claim Block 3 enforces. |
| B7 | terminal-state immutability | **temporal** | `always (state = Resolved → always state = Resolved)` — Resolved is a sink |
| B8 | attestation-binds-tally | **temporal** | `always (next.tally_result.is_some() ∧ tally_result.is_none() → the publish_result msg that fired carried (tally, proof, public_inputs) whose: (a) public_inputs has length 9_792 and conforms to the §2.5 gnark public_inputs byte layout (uints.U8 high-byte invariant + frontend.Variable u64-range invariant), (b) /xion.zk.v1.Query/ProofVerifyGnark returns verified=true when queried with (proof, public_inputs, registry.vkey_name), (c) ReportData[0..32] extracted from public_inputs equals SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body)) AND ReportData[32..64] equals DST_VERIFIED_RCV_TALLY_V1 zero-padded to 32 bytes, (d) the (MrTd, Rtmr1, Rtmr2) tuple extracted from public_inputs equals the on-chain registry's (mrtd, rtmr1, rtmr2) — plus (Rtmr0, Rtmr3) component-wise equality where the registry has bound those slots, (e) [registration-time companion, see B8(e) row below] the enclave_pubkey field of the active election was bound at CreateElection time by the same gnark verification primitive against a registration-purpose ReportData with data[0..32] = SHA-256(enclave_pubkey))`. **v0.3.9 reformulation (audit re-review N1)**: clauses (a) through (e) are now all **chain-verified-in-contract** — every publish exercises the gnark verifier directly via /xion.zk.v1.Query/ProofVerifyGnark and the contract performs bytewise equality on extracted public_inputs fields. Trust boundary narrowed: clauses (a) and (b) move from "inherited from Quartz tdxVerifier + groth16Verifier axioms (deferred-axiom)" to "inherited from Xion's xion.zk module ProofVerifyGnark semantics (chain-trust, §6.1)"; clauses (c) and (d) move from "verified-rcv-specific deployment binding via SHA-256(canonical_serialization) over 64-byte user_data" to "bytewise extraction from gnark public_inputs (no SHA-256 indirection on the measurements / DST; SHA-256 is still load-bearing on commit_hash in clause (c) via Adv_commitTally_CR in B9). The 32-byte commit_hash occupies ReportData[0..32] (was: lower 32 bytes of user_data); the 32-byte DST tag occupies ReportData[32..64] (was: upper 32 bytes of user_data). The pre-v0.3.9 user_data construct is removed. Collision-resistance hypothesis appears in B9 as Adv_commitTally_CR (unchanged primitive: SHA-256 over canonical_serialization). **Classical-Prop shadow note** (unchanged from prior version): B8 is stated as a deterministic implication; the cryptographic content lives in B9's negligibility budget. Consumer-facing claims in Section 6.6 ("Output contract") that read B8 as a categorical implication should be understood as classical-Prop shadows of the probabilistic B9 form (Attack 16). |
| B8(e) | pubkey-provenance | **temporal** | `always (next.elections[id].is_some() ∧ elections[id].is_none() → the create_election msg that fired carried (enclave_pubkey, proof, public_inputs) whose: public_inputs verifies via /xion.zk.v1.Query/ProofVerifyGnark against registry.vkey_name AND ReportData[0..32] extracted from public_inputs equals SHA-256(enclave_pubkey) AND ReportData[32..64] equals DST_VERIFIED_RCV_PUBKEY_V1 zero-padded to 32 bytes AND extracted measurements match registry)`. **Added v0.3.9 (audit re-review N1.c — pubkey provenance)**: the v0.3.8 form of CreateElection accepted enclave_pubkey from admin without cryptographic provenance — admin could substitute an arbitrary pubkey, breaking ballot privacy (an admin-supplied pubkey whose privkey is in admin's possession lets admin decrypt all submitted ballots even before the tally; see N2 mitigation discussion). With B8(e), the enclave_pubkey at CreateElection is bound to a TDX quote that commits to it via SHA-256(enclave_pubkey) in ReportData[0..32] — admin can no longer pick the pubkey freely; the pubkey must come out of a TDX enclave whose code matches the registered MRTD. Residual risk (post-B8(e)): admin running a different VM whose MRTD matches the registry — a much narrower surface, addressable separately via per-election nonce + timelock + multisig admin (see N2 in audit-finding remediation log). The cryptographic primitive is the same SHA-256 (collision-resistance bound by `Adv_commitTally_CR` in B9, applied to the pubkey preimage rather than the tally preimage — same hardness assumption). Domain-separation tag `DST_VERIFIED_RCV_PUBKEY_V1` prevents cross-purpose replay between registration and publish quotes (they use the same circuit + vkey but different ReportData layout). |
| B9 | B8 negligibility-budget decomposition | **meta-security** | `∀ PPT 𝒜 (controlling the chain mempool and all PublishResult message payloads), ∀ security parameter n, ∀ chain state σ, ∀ candidate successor σ′ with publish_result firing in fires_at_transition(σ → σ′): image_registration_honest(σ) ∧ circuit_equivalence_honest ⇒ Pr[B8 clauses (a)+(b)+(c) violated at σ′ by 𝒜's chosen attestation] ≤ Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_commitTally_CR(n)` — **3-summand bound, conditioned on the operational precondition** `image_registration_honest(σ)` (formally defined in §6.1) and the **correctness assumption** `circuit_equivalence_honest` (the reference DCAP gnark circuit correctly encodes the TDX-quote-validity relation; the v0.3.0 inclusion of `Adv_circuit_eq` as a probability summand was a category error per v0.3.1 T9 / kimi temporal-invariants attack 7: circuit-equivalence is a binary correctness property, not a probabilistic advantage decreasing with `n`). **Probability distribution**: over 𝒜's randomness + the random coins of TDX-quote / Groth16 / SHA-256 primitives. **The conditioning is load-bearing** (Round 3a re-adversarial theme 7 / claude S4 / qwen S6 / kimi A3): without `image_registration_honest`, B9's LHS includes failure mode 4.3b (vkey substitution), which has adversarial success ≈ 1 against governance-compromise adversaries, not bounded by any negligible summand. The two Quartz-inherited summands depend on `cross_component_session_bind_negl`'s def-tied form per Quartz cycle-6.4-through-6.11 (§6.2 status block); `Adv_commitTally_CR` remains verified-rcv-internal and bounds SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)` as the binding hash in B8 clause (c). **v0.3.1 sharpening (subagent-dispatch synthesis T9)**: 3 voices flagged the prior B9 as (i) **mis-tagged temporal** when its body is a meta-security statement (now retagged), (ii) **missing the `∀ PPT 𝒜, ∀ n` quantifier structure** (now explicit), (iii) **including `Adv_circuit_eq` in a probabilistic sum** where it's a correctness assumption (now lifted to the precondition `circuit_equivalence_honest`). The summand count changed 4 → 3 in v0.3.1 not by dropping content but by promoting `Adv_circuit_eq` to a named correctness hypothesis on the LHS. **Decomposition lineage**: 5 summands in earliest draft → 4 after Round 3a first-pass adversary (dropped `Adv_KMS_leakage` and `Adv_image_registration`) → 3 summands + 2 named preconditions in v0.3.1. **Substrate provenance**: B9's two Quartz-inherited summands depend on `cross_component_session_bind_negl`'s def-tied form per Quartz cycle-6.4-through-6.11. The Quartz-side asks 6 (per-conjunct failure-mode analysis) and 7 (degenerate-zero-advantage cycle intent) shape what "def-tied" means upstream; verified-rcv's compose ledger records both. |
| B10 | tally-correctness | **cross-layer** | `always (next.tally_result.is_some() ∧ tally_result.is_none() → ∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ enclave_input_fidelity(σ, σ′) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey)))` — the published tally equals `Tally_spec` applied to the frozen ballot set with *some* private key, where: (i) the key was dstack-KMS-derived for this contract (Section 6.3 trust boundary), and (ii) the enclave actually consumed `ballots@end_at` and `candidates` as its inputs (not host-substituted values). **Tag is `cross-layer`**: the existential ranges over off-chain values, the witness path traverses the Lean discharge layer (Section 8.7), and `enclave_input_fidelity` is established by the attestation envelope rather than chain state. **v0.3.1 sharpening (subagent-dispatch synthesis T3, 4 voices: claude / kimi / gpt-oss / gemma)**: the v0.3.0 form omitted `enclave_input_fidelity`, leaving a gap where a malicious host could feed empty ballots to a correct enclave image, the enclave would correctly compute `Tally_spec(∅, candidates, privkey)`, B8 would attest, and B10 would be violated despite all three named conjuncts (B10_lean, image-identity-binding, B8) holding. The new conjunct closes that gap. **`enclave_input_fidelity(σ, σ′)`** is the predicate: the inputs the enclave consumed equal `(ballots@end_at(σ), candidates(σ))`. Discharged operationally by an input-hash field in the attestation envelope (see §8.7 step 5b); the discharge is part of B8 clause (c)'s domain-separation-tagged commitment to `(contract_addr, tally_body)` together with §8.7's new input-fidelity step. **Restated per Round 3a re-adversarial pass — theme 2 (claude S9, qwen C2, glm C3, kimi A6, mistral S9, gpt-oss S10)**: the prior formulation referenced `enclave_privkey` as a free variable in a chain-side temporal invariant. The current form (with `enclave_input_fidelity`) makes the off-chain dependencies explicit. B10 tracks the load-bearing methodology obligation (Round 3a first-pass adversary Attack 20); without it, Section 4.4's enclave-software-bug failure mode has no formal anchor. `Tally_spec` is the canonical pipeline (§2.5: Stage 1 `decrypt_and_validate` + Stage 2 `IRV_spec` + Composition); discharging B10 is the primary work of the downstream Lean spec, factored as `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack_kms_trust ∧ enclave_input_fidelity` per Section 8.7's restructured decomposition (v0.3.1 amends the 3-link composition to 5-link to make the trust + fidelity dependencies explicit). The verified-rcv compose ledger records B10 as the central correctness obligation. |
| B10_lean | Lean-internal image-IO obligation | **off-chain** | `∀ raw_ballots: RawBallots, candidates: CandidateSet, privkey: PrivKey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)` — the enclave image's externally observable input-output relation equals `Tally_spec`, where `EnclaveImage` is the **fixed Lean-extracted-model symbol** defined in §2.5 with explicit 3-ary type signature `RawBallots × CandidateSet × PrivKey → TallyResult`. A proof author **may not** instantiate `EnclaveImage` freely — it is reserved for the model produced by the documented extraction discipline. This is the **Lean-discharge half** of B10; it lives entirely off-chain and is independent of any chain state. Composes with image-identity-binding (the on-chain registered (mrtd, rtmr) match the build artifact of the Rust crate `EnclaveImage` was extracted from) + B8 (attestation binds the image to the published tally) + dstack_kms_trust (axiomatic, §6.3) + enclave_input_fidelity (attestation-envelope-witnessed, §8.7 step 5b) to give B10. **v0.3.1 sharpening (subagent-dispatch synthesis T1, 5 voices)**: the prior form referenced `EnclaveImage` as a free symbol with conflicting `Bytes → Bytes` typing in the CONTEXT_APPENDIX vs 3-ary semantic usage here. §2.5 now defines the typed symbol explicitly; §8.7 step 4 ties it to the extraction substrate. **Added per Round 3a re-adversarial pass — theme 10 (claude S6)**: the v0.2.1 Section 8.7 Step 3 conflated this with B10 itself, obscuring the cross-layer composition. Distinguishing the names + typing makes the discharge structure inspectable. |

#### Cross-project dependency note

B8 is a **composition theorem** that depends on:

- `Specs.Quartz.Attestation.Dstack.tdxVerifier` (Quartz axiom, (d) sub-tag classically-over-strong-single-negligibility)
- `Specs.Quartz.Attestation.Zkdcap.groth16Verifier` (Quartz axiom, (d) sub-tag classically-over-strong-doubled-negligibility)

Any drift in either of those bundles' cardinality at the Quartz level (per v0.2's `bundle-cardinality drift tracking`) should propagate as a downstream alert to verified-rcv's ledger. The verified-rcv compose ledger (forthcoming) will record this dependency explicitly.

## 4. Failure Modes

System-level failure scenarios distinct from Section 2.4's per-handler `ContractError` variants. Each names a cause, the resulting compromised property, detection, recovery, and mitigation.

**Severity-tag taxonomy** (added per v0.3.1 subagent-dispatch synthesis T16 — 3 voices flagged ad-hoc tags in v0.3.0). Each failure mode is tagged with one or more of the three base severity classes — **confidentiality**, **integrity**, **liveness** — optionally with a named refinement when the failure's scope is narrower than the base class. The refinements are:
- *integrity-per-voter* — integrity loss restricted to a single voter's ballot (e.g., voter-key compromise). Composes as base-class **integrity** but is narrower in scope; one voter's loss does not propagate to the tally's overall correctness if other voters' ballots are unaffected.
- *canonicality* — a sub-class of **liveness** specific to multi-fork scenarios where no single canonical chain state exists, *or* of **integrity** if different forks admit different winners. The two faces are split into 4.5a (canonicality-as-liveness) and 4.5b (canonicality-as-integrity) below.
- *recoverable* — a modifier signaling that the failure self-heals once its cause is removed, distinct from the default "no auto-recovery." 4.6 is the only such case.

**Recoverability**: per the no-admin-recovery design decision, **failures listed below are not auto-recoverable from within the election instance, with one named exception: §4.6 (enclave network partition)**, which auto-recovers when connectivity is restored because `publish_result` is idempotent. For all other failures, recovery means deploying a fresh election. **v0.3.1 sharpening (subagent-dispatch synthesis F5 — kimi failure-modes attack 1 critical, single-voice but ground-truth-true)**: the v0.3.0 preamble said "no failure here is auto-recoverable" without exception, which contradicts §4.6's explicit recoverability. The preamble is amended to name the exception explicitly.

### 4.1 Enclave hardware compromise — *confidentiality + potentially integrity*

- **Cause:** TDX vulnerability (Intel SA / side-channel) extracts the per-election private key at runtime.
- **Effect:** Adversary decrypts all on-chain ballots. If the vulnerability also permits forging attestations, integrity is also lost.
- **Detection:** Out-of-band (Intel disclosure, chain-operator anomaly detection).
- **Recovery:** None for the affected election; ballots are already exposed. Future elections require patched hardware plus chain-operator update to the registered enclave image / vkey.
- **Mitigation:** Inherited from Quartz `tdxVerifier` soundness assumption. Out of scope at the verified-rcv layer.

### 4.2 dstack KMS key compromise — *confidentiality*

- **Cause:** KMS bug or operator compromise releases the per-election keypair to a non-attested entity.
- **Effect:** Adversary decrypts ballots. Integrity remains intact (the leaked key alone cannot produce valid attestations).
- **Detection:** Out-of-band (KMS audit).
- **Recovery:** None for the affected election.
- **Mitigation:** Inherited from Quartz / dstack KMS spec.

### 4.3a Attestation vkey unregistered — *liveness*

- **Cause:** The verified-rcv `zkdcap_vkey` was never registered on the chain's ZK module under the expected slot, or was unregistered between instantiation and `publish_result`.
- **Effect:** `publish_result` fails permanently with `AttestationFailed { reason: VkeyUnregistered }`. Election deadlocks; no canonical tally ever lands on-chain.
- **Detection:** Verifiable on-chain — any caller can query the registered vkey and compare to the expected verified-rcv image. Detectable pre-deployment.
- **Recovery:** None for the affected election; deadlock is terminal.
- **Mitigation:** Pre-deployment operational check that the vkey is registered. Inherits from Quartz attestation chain.

### 4.3b Attestation vkey maliciously substituted — *integrity*

- **Cause:** A different (malicious or buggy) `zkdcap_vkey` is registered on the chain's ZK module under the expected slot — either through chain-governance compromise or operator error.
- **Effect:** `publish_result` accepts attestations from non-verified-rcv enclaves; arbitrary tallies may be published with apparent validity. Integrity is lost.
- **Detection:** Verifiable on-chain by anyone who knows the *correct* verified-rcv vkey hash; comparison surfaces the substitution.
- **Recovery:** None for the affected election; bad tallies are terminal once `publish_result` succeeds.
- **Mitigation:** Pre-deployment vkey verification + chain-governance posture. Inherits from Quartz attestation chain.

### 4.4 Enclave software bug — *integrity* — **in scope, central methodology target**

- **Cause:** The attested enclave image contains a tally bug. Attestation validates correctly because the image hash *is* what was attested; the bug lives inside the attested image.
- **Effect:** Wrong tally published; recorded as canonical on-chain.
- **Detection:** Pre-deployment formal verification of the enclave's tally logic against the Lean and Quint specs derived from this intent document. Post-deployment, no in-system detection mechanism exists.
- **Recovery:** None for the affected election.
- **Mitigation:** Pre-deployment formal verification against the downstream Lean and Quint specs. No post-deployment mitigation; spec correctness is load-bearing.

### 4.5 Chain consensus halt or fork — *liveness or canonicality*

- **Cause:** Xion chain stops producing blocks, or hard-forks during voting/tallying.
- **Effect:** Halt → liveness lost (no transactions go through). Fork → ambiguity about which fork's election state is canonical.
- **Detection:** Chain monitoring tooling.
- **Recovery:** Wait for chain operators. Fork resolution determines canonical state.
- **Mitigation:** Choice of chain. Out of scope.

### 4.6 Enclave network partition — *liveness, recoverable*

- **Cause:** Enclave functioning correctly, but cannot reach the chain (or vice versa) to submit `publish_result`.
- **Effect:** Election deadlocked until connectivity restored.
- **Detection:** Off-chain monitoring.
- **Recovery:** Enclave re-publishes once reconnected. `publish_result` is idempotent by construction (B1 plus Block 6's `AlreadyResolved` rejection), so retries are safe.
- **Mitigation:** Operational — deploy enclave with redundant connectivity. The idempotency property is the in-scope guarantee.

### 4.7 Voter address compromise — *integrity-per-voter*

- **Cause:** A candidate's chain private key is stolen.
- **Effect:** Attacker submits a ballot in the victim's name. With last-write-wins, the legitimate ballot is overwritten.
- **Detection:** Per-candidate (the victim notices their address voted in a way they did not authorize).
- **Recovery:** During voting: victim regains control and resubmits (last-write-wins). After `end_at`: none.
- **Mitigation:** Voter-side key management. Out of scope; the last-write-wins recovery semantics is the in-scope guarantee.

### 4.8 [RESERVED — moved to §5 Non-Goals in v0.3.1]

**Candidate collusion / coordinated voting** was listed here in v0.3.0 as "not a failure"; per v0.3.1 subagent-dispatch synthesis (kimi failure-modes attack 4, single-voice but ground-truth-true), a non-failure-mode entry in the failure-mode list breaks automated extraction and conflates Non-Goals with failures. The slot number is retained as RESERVED to preserve cross-references; the content has been merged into §5's "No coordination / collusion resistance" bullet.

### 4.9 Deterministic-malformed-tally deadlock — *liveness*

(Added per Round 3a first-pass adversary Attack 11.)

- **Cause:** The attested enclave image contains a bug that produces a tally violating Block 6's well-formedness predicate (Section 2.5). Examples: `winners = []` (empty winner set), per-round counts that don't conserve, `non_voters ⊄ candidates`, batch-elim policy violation. `publish_result` rejects with `InvalidTally`. The enclave is deterministic over the ballot set; the ballot set is frozen at `end_at` (B2). Therefore re-running the enclave produces the *same* malformed tally and `publish_result` rejects again. **Enclave retry policy (added per v0.3.1 subagent-dispatch synthesis T15)**: Block E1 specifies that the enclave retries `publish_result` indefinitely with exponential backoff (initial delay 30s, doubling up to 1h max) until either `publish_result` succeeds or an operator manually halts the enclave. Without this retry-policy specification, "infinite rejection loop" and its on-chain detection signature are unstated — a one-shot enclave would emit exactly one `InvalidTally` event and then silence (silent deadlock).
- **Effect:** Election deadlocked in Tallying indefinitely. Distinct from 4.6 (network partition — connectivity issue) and 4.10 (KMS unavailability at instantiation — credential issue): here the chain refuses the enclave's output rather than the enclave being unable to publish.
- **Detection:** On-chain — **repeated `InvalidTally` rejection events with identical `reason` field**, separated by the enclave's exponential-backoff delay (~30s, ~60s, ~2min, ..., up to 1h between events). The repetition + identical-reason signature distinguishes 4.9 from a one-shot 4.4 failure, and the backoff-spaced cadence distinguishes 4.9 from a buggy retry loop in unrelated code.
- **Recovery:** None for the affected election. Requires deploying a corrected enclave image (re-attestation against a different MRTD/RTMR), which makes the new image a different `verified-rcv` deployment from the chain's perspective — i.e., a fresh contract instance with the corrected image. The deadlocked instance remains as a permanent on-chain record.
- **Mitigation:** Pre-deployment formal verification of the enclave's tally logic against B10 (tally-correctness invariant) and the well-formedness predicate (Block 6). This is the same mitigation as 4.4 (enclave software bug); 4.9 is the *liveness* face of 4.4's *integrity* face. Together they bound the chain's risk surface from enclave bugs: a well-formed but incorrect tally is a 4.4 integrity loss (chain accepts wrong result); a malformed tally is a 4.9 liveness loss (chain rejects everything).

### 4.10 KMS unavailability at instantiation — *liveness*

(Added per v0.3.1 subagent-dispatch synthesis T11 — 3 voices flagged the prior absence.)

- **Cause:** Block 1's `instantiate` Requires a DstackKeyManager-issued keypair derivable for this contract instance. If the KMS is unreachable, mid-failure, or rate-limited at instantiation time, key derivation fails and contract creation reverts.
- **Effect:** Contract creation fails with `InvalidInstantiation { reason: KmsUnavailable }`. Election never starts; no on-chain state is created.
- **Detection:** Failed instantiation transaction visible to the would-be operator. Distinct from 4.2 (KMS compromise — confidentiality loss after successful instantiation) and from 4.3a (vkey unregistered — different failure surface).
- **Recovery:** Retry instantiation after the KMS is restored.
- **Mitigation:** Operational — KMS-health probe before instantiation attempt; dstack KMS observability.

### 4.11 Enclave resource exhaustion during tabulation — *liveness*

(Added per v0.3.1 subagent-dispatch synthesis T11 — 3 voices flagged the prior absence.)

- **Cause:** Block E1 runs ECIES decryption + IRV recursion inside a TDX enclave. For large candidate sets and/or ballot counts, the working set or CPU time may exceed enclave resource limits. The enclave aborts without producing an attestation. The contract has no bound on `len(candidates)` or `len(ballots)`, so this case is admissible.
- **Effect:** Election deadlocked in Tallying. The enclave fails to publish, distinct from 4.6 (chain unreachable) and 4.10 (KMS unreachable at instantiation).
- **Detection:** Off-chain monitoring of the enclave process; on-chain, the symptom is identical to 4.6 (no `publish_result` event after `end_at`) and the two are distinguishable only by enclave-side telemetry.
- **Recovery:** None for the affected election; the resource bound must be raised or the input cardinality must shrink before a corrected instance can re-attempt.
- **Mitigation:** Pre-deployment benchmarking of `Tally_spec` on representative inputs. Operational candidate-set + ballot-set caps applied at the Block 1 Requires level (out of scope for v0.3.1 spec — a future invariant `S11: len(candidates) ≤ max_candidates_param` could pin this).

### 4.12 ZK module algorithmic compromise — *integrity*

(Added per v0.3.1 subagent-dispatch synthesis T11 — Xion ZK module bug not in the prior failure-mode list.)

- **Cause:** The chain's `/xion.zk.v1.Query/ProofVerifyGnark` endpoint contains a bug: it returns `true` for invalid proofs (false positive) or `false` for valid ones (false negative). This is distinct from 4.3a/4.3b which cover vkey registration; the algorithm itself is buggy.
- **Effect:** False positive — `publish_result` accepts attestations whose ZK proof does not actually verify the TDX-quote-validity statement. B8 clause (b) is violated with adversary-controlled probability ≈ 1 against a polynomial-time adversary aware of the bug. False negative — `publish_result` rejects valid attestations; election deadlocks (4.3a-equivalent).
- **Detection:** Out-of-band — formal verification of the gnark module against the canonical zkdcap circuit specification, or third-party audit. Once a specific bug is known, on-chain detection by replaying the proof against a known-good verifier.
- **Recovery:** None for elections already finalized under the buggy module; deploy fresh elections against a patched chain.
- **Mitigation:** Inherited from Xion's ZK module spec; **not re-verified at the verified-rcv layer**. Verified-rcv assumes the gnark module is correct per §6.1 ZK module endpoint semantics; this failure mode names the residual risk.

### 4.13 Block-time non-monotonicity — *liveness / integrity*

(Added per v0.3.1 subagent-dispatch synthesis T11 — 2 voices flagged the block-time regression case.)

- **Cause:** §6.1 assumes `env.block.time` is non-decreasing across consecutive blocks. In Cosmos SDK, block timestamps are proposer-determined; a malicious or buggy proposer can produce a block with a timestamp earlier than the predecessor's.
- **Effect:** Derived-state oscillation. A contract in Tallying (`env.block.time ≥ end_at`) could revert to Voting (`start_at ≤ env.block.time < end_at`) if a subsequent block has a timestamp earlier than `end_at`. Breaks B3 (no premature tally) and B4 (no premature voting) along the regression path. If `publish_result` already fired, B1 (tally_result monotone-once-set) is preserved (the contract storage doesn't unwind), so the *value* of `tally_result` is safe — but new `submit_ballot` calls in the regressed state are accepted, corrupting `ballots@end_at`'s value retroactively unless B2's discharge is explicitly post-regression-aware.
- **Detection:** Chain-monitoring tools comparing consecutive block timestamps.
- **Recovery:** None at the verified-rcv layer; chain operators slash the misbehaving proposer (out-of-band).
- **Mitigation:** Inherited from Xion's consensus + slashing rules. **Note**: the assumption is named in §6.1 as load-bearing; this failure mode makes it an explicit risk surface rather than a silent assumption.

## 5. Non-Goals

Things this system explicitly does NOT do. Prevents over-specification and clarifies scope.

- **No admin / creator recovery role.** If the enclave never publishes (network partition, KMS failure, registry mismatch), the election deadlocks permanently. No party can restart, abort, or extend. Accepted as a design tradeoff to keep the trust surface minimal.
- **No re-tally after Resolved.** Once `tally_result.is_some()`, the result is terminal. Even if a flaw is discovered later, the on-chain record stands.
- **No on-chain candidate-consent gate.** Anyone may instantiate a contract listing any chain addresses as candidates. Legitimacy of any instance is established off-chain; the system makes participation publicly inspectable but does not gate it.
- **No vote weight, staking, or stake-weighted voting.** Every candidate's ballot has weight 1. No quadratic, conviction, or governance-token-weighted variants.
- **No multi-round elections** (primary + general, runoff with new candidates, etc.). A single contract instance handles one tally. Sequential elections require separate instances.
- **No vote delegation / proxy / liquid democracy.** Each candidate's ballot is submitted by themselves (last-write-wins); no `delegate_to(other)` mechanism.
- **No partial preference orderings.** Every ballot must rank every candidate. Ballots failing this constraint are dropped as malformed (Section 2.3 edge case).
- **No live tallying during the voting window.** Tally happens only after `end_at`. Intermediate counts are not visible on-chain or off; the enclave does not perform partial computation until the window closes.
- **No coercion resistance (post-tally).** A voter can be compelled to reveal their preferences after tally by revealing their private key. The ballot is private from the chain and enclave-host operators; it is not private from the voter themselves nor from anyone the voter chooses to share with.
- **No coordination / collusion resistance.** Candidates may coordinate vote patterns off-chain (Section 4.8). The system tallies correctly against whatever ballots are cast.
- **No key-management at the contract layer (pre-tally key theft).** If a candidate's chain private key is stolen, the attacker can submit a ballot in their name; the contract-layer cannot distinguish (Section 4.7). Note: this is distinct from post-tally coercion above. Round 3a first-pass adversary Attack 25 reframe.
- **No anonymity for ballot submitters.** Each ballot is bound on-chain to its submitting candidate's address. The on-chain transaction reveals "candidate X voted at time T"; only the *content* of the ballot is private.
- **No threshold or quorum requirement.** Elections with zero ballots, all-malformed ballots, or any participation level proceed to tally; results are mathematically defined for any input.
- **No on-chain operational-viability checks** (e.g., minimum voting window). All instantiations satisfying Block 1's Requires are admitted; the instantiator alone is responsible for choosing operationally-meaningful parameter values (window length, candidate set, etc.). The chain provides no minimum-window, maximum-candidate-count, or quorum check. Round 3a first-pass adversary Attack 21.
- **No liveness invariant binds publish_result.** The system makes no claim that an election will resolve. Deadlock in Tallying is permitted by construction (Sections 4.6, 4.9, 8.3). Round 3a first-pass adversary Attack 23.
- **No formal verification of upstream-Quartz axioms at this layer.** verified-rcv asserts B8 + B9 as depending on Quartz's `tdxVerifier` and `groth16Verifier` bundles; the honesty of those bundles is the Quartz colosseum's responsibility. Section 6.2 names this inheritance explicitly. *Verified-rcv-specific cryptographic primitives* (e.g., the SHA-256 commit hash in B8 clause (c)) **are in scope** at this layer and require their own collision-resistance hypothesis (`Adv_commitTally_CR` in B9). Round 3a first-pass adversary Attack 30 split.

## 6. Trust Boundaries

What the system assumes about its environment. Where input validation begins and ends, and what is taken on faith vs. proved.

### 6.1 Chain trust (Xion / CosmWasm)

verified-rcv trusts the underlying chain to provide:

- **Block time monotonicity** — `env.block.time` is non-decreasing across consecutive blocks. Used by every state-derived predicate (Created / Voting / Tallying).
- **Sender authentication** — `msg.sender` is the address that signed the transaction; chain consensus enforces signature validity. Block 3 (`submit_ballot`) relies on this for the "voter == candidate" check.
- **State persistence** — state writes complete atomically; no silent corruption between blocks.
- **ZK module endpoint semantics** — `/xion.zk.v1.Query/ProofVerifyGnark` returns true iff the supplied `(proof, public_inputs)` verifies against the named `vkey_name` (resolved via the on-chain `xion.zk` VKey store) under gnark's verification algorithm for Groth16 BN254. Trusted as Xion-spec'd; not re-verified at this layer. The verified-rcv contract issues this query directly via `QueryRequest::Grpc` per §2.5 "Gnark verification primitive" (v0.3.9). The vkey registration governance and the gnark verification algorithm correctness are both Xion's responsibility; verified-rcv's only obligations at this surface are (a) constructing the prost-encoded `QueryVerifyGnarkRequest` correctly and (b) extracting measurements + ReportData from `public_inputs` per §2.5 layout. **ZK module vkey-registration governance corollary**: the on-chain `xion.zk` VKey store is a chain-governance-supervised registry (Xion's `MsgRegisterVKey` or equivalent); verified-rcv assumes the named `vkey_name` is registered to the correct verified-rcv gnark vkey. Failure mode 4.3b covers post-registration substitution by chain-governance compromise; failure mode 4.12 covers gnark-module algorithmic bugs.
- **Fork resolution** — on a fork, the canonical chain state is determined by chain consensus; verified-rcv's election state follows whichever fork wins.
- **On-chain registry — verified-rcv enclave identity** (revised per v0.3.1 subagent-dispatch synthesis T2 + T10; **registry schema amended in v0.3.8 audit-finding remediation M3 (mrtd/rtmr 48-byte); further amended in v0.3.9 audit re-review N1 (vkey_name + RTMR split + accepted TCB statuses)**). The chain stores a **per-deployment registry slot** for the verified-rcv contract, holding the structured `EnclaveImageRegistry` value:

  ```
  EnclaveImageRegistry {
      vkey_name: String,                     // v0.3.9: name of xion.zk-registered gnark vkey (was Bytes32 inline)
      mrtd: [u8; 48],                        // v0.3.8 M3: TDX MRTD (SHA-384, 48 bytes); was Bytes32
      rtmr1: [u8; 48],                       // v0.3.9: split out from generic rtmr; mandatory
      rtmr2: [u8; 48],                       // v0.3.9: mandatory
      rtmr0: Option<[u8; 48]>,               // v0.3.9: optional (gnark circuit may not bind this)
      rtmr3: Option<[u8; 48]>,               // v0.3.9: optional
      accepted_tcb_statuses: Vec<u8>,        // v0.3.9: TCB severity values accepted (default: {0,1,2,3}); 4/5/6 rejected
  }
  ```

  **Schema rationale (v0.3.9)**:
  - `vkey_name: String` (was `vkey: Bytes32`): the gnark vkey content lives in the chain's `xion.zk` VKey store, not inline in the verified-rcv contract storage. The registry references the vkey by name; verified-rcv's `/xion.zk.v1.Query/ProofVerifyGnark` call resolves `vkey_name` → vkey bytes at query time. Decoupling the storage allows vkey rotation via chain-governance (`xion.zk` `MsgRegisterVKey` / `MsgUpdateVKey`) without touching verified-rcv contract storage.
  - `mrtd: [u8; 48]`, `rtmr*: [u8; 48]`: TDX measurements are SHA-384 (48 bytes). The pre-v0.3.8 `Bytes32` was incorrect for TDX; v0.3.8 M3 fixed this.
  - `rtmr1`/`rtmr2` mandatory; `rtmr0`/`rtmr3` optional: matches the zkdcap-verifier convention (`/Users/mvid/Development/reliq/zkdcap/verifier/src/contract.rs:317-337`). RTMR0 is the firmware measurement (typically operator-controlled, less informative); RTMR3 is the runtime-extended measurement (workload-dependent and sometimes left unbound for stable deployments). Setting them to `None` skips the corresponding chain-side equality check.
  - `accepted_tcb_statuses: Vec<u8>`: the gnark circuit hard-rejects TcbStatus = 6 (Revoked); the contract additionally rejects whatever the operator chooses (typically {4, 5} = OutOfDate / OutOfDateConfigurationNeeded). Default is {0..=3}.

  The registry slot is written at instantiation and updateable via the admin-only `UpdateRegistry` execute message (v0.3.8 M3), gated on no active election in Voting/Tallying phase. The registry is **assumed not subject to undetected post-registration modification** at the contract storage layer; the admin trust surface is the only modification vector (out-of-band-governed at the contract-operator layer).

  **Formal predicate `image_registration_honest(σ)`** (added per v0.3.1 subagent-dispatch synthesis T2; **amended in v0.3.9 to track the new schema**). The predicate referenced by B9 is formally:

  ```
  image_registration_honest(σ) ≡
      chain_registry(σ).vkey_name = canonical_verified_rcv_vkey_name
    ∧ xion_zk.lookup_vkey(canonical_verified_rcv_vkey_name).bytes = canonical_verified_rcv_vkey_bytes
    ∧ chain_registry(σ).mrtd  = canonical_verified_rcv_mrtd
    ∧ chain_registry(σ).rtmr1 = canonical_verified_rcv_rtmr1
    ∧ chain_registry(σ).rtmr2 = canonical_verified_rcv_rtmr2
    ∧ chain_registry(σ).rtmr0 ∈ {None, Some(canonical_verified_rcv_rtmr0)}
    ∧ chain_registry(σ).rtmr3 ∈ {None, Some(canonical_verified_rcv_rtmr3)}
  ```

  where `canonical_verified_rcv_*` are the deployment-fixed expected values (typically published in the deployment manifest / cross-verified by independent operators). The conjuncts on `rtmr0` and `rtmr3` admit the "unbound" choice as honest (operator declines to enforce; only enforces if Some). **The vkey lookup conjunct (second line, v0.3.9)** captures the indirection: `image_registration_honest` requires both (i) verified-rcv's registry points to the right `vkey_name`, AND (ii) the chain's `xion.zk` store holds the right vkey bytes under that name. Either being compromised opens failure mode 4.3b.

  The predicate is evaluated **at the chain state `σ` immediately preceding the firing transition that B9 bounds** (i.e., for `σ → σ′` with `publish_result ∈ fires_at_transition(σ → σ′)`, B9's antecedent is `image_registration_honest(σ)`). The predicate is *not* under an `always` modality in B9; B9 is a meta-security statement parameterized over `σ` (per the v0.3.1 B9 retag to `meta-security`).

  **Operational vs cryptographic**: the chain's registry-integrity guarantee is operational (chain-governance + slashing on `xion.zk`; admin-discipline on verified-rcv's `EnclaveImageRegistry`) rather than cryptographic. A governance-compromised chain can rewrite either registry; that case is precisely failure mode 4.3b and is **explicitly excluded** from B9's bound by the `image_registration_honest` antecedent. **vkey registration vs MRTD/RTMR registration vs verified-rcv-registry vkey_name pointer are conceptually one operation but may be governed by independently-compromisable surfaces** (per kimi trust-quartz attack 8); on a chain that supports independent governance of each registry component, the predicate's conjuncts must each hold independently. The v0.3.9 schema makes the surface split explicit by separating the `vkey_name` pointer (verified-rcv-governed) from the vkey content (xion.zk-governed).

  Failure modes for this trust surface: 4.3a (vkey unregistered — either `vkey_name` points at a missing xion.zk slot, or that slot is empty), 4.3b (vkey maliciously substituted — either `vkey_name` was rewritten in verified-rcv's registry, or the xion.zk-stored bytes were rewritten under the same name), 4.13 (block-time non-monotonicity also affects the consistent reading of `σ` for the predicate). **MRTD-substitution-only and RTMR-substitution-only attacks** map to 4.3b's scope; the failure mode covers the union.

### 6.2 Quartz inheritance (the cross-project dependency)

verified-rcv inherits — without re-proving — the following Quartz spec-layer claims. Each is an explicit upstream dependency that the verified-rcv compose ledger must record.

| Inherited claim | Source artifact | Quartz axiom bucket | Discharge path |
|---|---|---|---|
| `DstackAttestor` produces quotes whose validity reflects the TDX platform's runtime state | `Specs.Quartz.Attestation.Dstack` (`tdxVerifier` record) | (d) classically-over-strong-single-negligibility (sound) + preconditional (complete) | PCK-signature unforgeability reduction + Lean reference DCAP verifier |
| `verifyGroth16` accepts a proof iff it is a valid Groth16 proof of the encoded TDX-quote-validity statement under the registered vkey | `Specs.Quartz.Attestation.Zkdcap` (`groth16Verifier` record); honest formulation at `proofs/lean/Specs/Quartz/Attestation/ZkdcapVCVio.lean:98-105` | (d) classically-over-strong-doubled-negligibility | **v0.3.9 discharge path change**: verified-rcv now consumes Groth16 verification via the Xion chain's `/xion.zk.v1.Query/ProofVerifyGnark` (a chain-side primitive built on gnark Groth16 BN254). The discharge path therefore splits: (i) `Adv_groth16_KS` (cryptographic-hardness summand) is bounded by gnark's BN254 Groth16 KS, trusted as a chain-side primitive at §6.1; (ii) `circuit_equivalence_honest` (binary correctness precondition) is the assertion that the registered `vkey_name`'s vkey content corresponds to the zkdcap reference DCAP gnark circuit (`/Users/mvid/Development/reliq/zkdcap/circuits/dcap-gnark/circuit/dcap.go`) which faithfully encodes the TDX-quote-validity statement. The Quartz inheritance row is retained for traceability; the actual verification on the verified-rcv path no longer routes through Quartz's Lean `groth16Verifier` lift but through Xion's chain ZK module. **Bundling note (v0.3.7 from Quartz audit 2026-05-26, retained for context)**: Quartz's `groth16Verifier` lift is "doubled" — it bundles `negligible_groth16` (Groth16-KS on BN254) AND `negligible_circuit` (R1CS encoding ↔ reference DCAP verifier semantic equivalence). verified-rcv's §3.2 B9 retag (v0.3.1) split this doubled lift into `Adv_groth16_KS` plus `circuit_equivalence_honest`. The v0.3.9 chain-side consumption preserves the split; the cryptographic summand is now bounded by gnark's primitive rather than ArkLib's, but the hardness assumption family is unchanged (Groth16-KS on BN254 either way). |
| ECIES encrypt/decrypt roundtrip is sound | `Specs.Quartz.Crypto.Ecies.roundtrip` | **theorem** (Round 1 demoted from axiom) | None needed — proven |
| `commitHash` collision-resistance on `UserDataCommit → UserData` | `Specs.Quartz.Crypto.UserDataCommit` (`commitHashE` bundle) | (d) pigeonhole-impossible (lift restates as concrete-hash collision-resistance) | VCVio `randomOracle` + `[Fintype UserData]` carrier refinement; **NOT CONSUMED by verified-rcv — see Note A below** |

**Note A — `commitHashE` is listed but unused** (Round 3a re-adversarial pass, themes 8 + 15 — claude S5 / gemma C2; restated in v0.3.1 per T8). verified-rcv's hash-collision-resistance hypothesis is `Adv_commitTally_CR` (B9 third summand after the v0.3.1 retag), bounding SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)` *directly at this layer*. The row is retained in the table for traceability (other Quartz-downstream specs may consume it). See the substrate-status block below for the three independent reasons why this inheritance row is structurally unconsumed by verified-rcv (pigeonhole-impossible sub-tag, Fintype carrier mismatch, structural slot-interface-only consumption). The verified-rcv compose ledger records `commitHashE`'s downstream consumer count as zero from this spec; the `UserData` slot interface is recorded as a live dependency.

**Withdrawn inheritance row** (Round 3a first-pass adversary Attack 2, 2026-05-14): an earlier draft of this table listed `DstackKeyManager` as an inherited Quartz Lean artifact. `grep -rn "DstackKeyManager\|KeyManager" /Users/mvid/Development/reliq/quartz/proofs/lean/Specs/` returns zero hits — Quartz's Lean spec tree does not currently model dstack KMS. The claim "private key released only to matching attested enclave image" is an unverified out-of-band assumption on the dstack KMS implementation itself, not an inherited Quartz theorem. The row has been removed from the inheritance table; the assumption remains operational and is captured in Section 6.3 (dstack / TDX hardware trust) and Failure Mode 4.2.

#### B8/B9 substrate status `[2026-05-16 — partially de-retracted]`

**Status update (Round 3a subagent-dispatch 3rd pass, theme T8 — 3 voices: claude / kimi / gpt-oss flagged the v0.3.0 banner as over-claiming substrate readiness; restating per the v0.3.1 punch list).** The Quartz substrate `cross_component_session_bind_negl` (and 7 other `_negl` lifts) **was retracted as content-free on 2026-05-14** in Quartz's Round-A adversarial review (per the ledger's `CORRECTION 2026-05-14` banner: the 8 `_negl` theorems bound their protocol-fail advantages as free `ℝ≥0∞` function symbols with no defining equation; proofs reduced to `negligible f → negligible f` tautologies under adversarial instantiation).

**The retraction has been partially resolved upstream on 2026-05-14/15.** Quartz's Lean tree implemented the **def-tying refactor across all 8 protocol-layer `_negl` lifts** (cycles 6.4 through 6.11; 8 commits at `/Users/mvid/Development/reliq/quartz/.colosseum/changes/2026-05-14T*-cycle-6.{4..11}-*.md`). The refactored lifts def-tie win events to concrete probability expressions, providing the content phase that was missing. The PR shape is now "v0.2 — back-port 7 asks + Round A adversarial review" (4 original asks + 3 new asks 5/6/7 surfaced during implementation; same enforcement rules: documented form in the PR, enforced form deferred behind executable-layer decision).

**Why "partially" — open Ask-6 bundle status (v0.3.1 sharpening, T8)**: Quartz Ask-6 (per-conjunct failure-mode analysis as bundle-count source) is in *documented* form, not *enforced* form. Until Ask-6 is enforced executably, the bundle-cardinality of `cross_component_session_bind_negl` (currently 7 protocol-fail summands bundled under one lift) is a methodology obligation rather than a verified count. The verified-rcv compose ledger will pick up bundle-cardinality drift signals from the Quartz ledger; until then, the substrate is "content-supplied (def-tied) but bundle-shape unverified." Hence "partially de-retracted" rather than "de-retracted."

**Methodology consequence**: verified-rcv inherits B8/B9 from a now-contented substrate. After the v0.3.1 B9 retag, the negligibility sum is **3 summands** (not 4 or 5 — see §3.2 B9 restatement): `Adv_tdxVerifier_sound`, `Adv_groth16_KS`, `Adv_commitTally_CR`. The `Adv_circuit_eq` term that v0.3.0 listed as the third summand has been promoted to a **correctness precondition** of B9 (a Lean theorem about the zkdcap circuit's faithful encoding of the TDX-quote-validity statement) and is no longer part of the negligibility sum. The first two summands are bound by Quartz's now-def-tied `cross_component_session_bind_negl` — specifically by the `tdxVerifier`-tagged + `groth16Verifier`-tagged summands of that lift. The third summand (`Adv_commitTally_CR`) remains verified-rcv-internal and corresponds to SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`.

**`commitHashE` non-consumption restated (v0.3.1, T8)**: the `commitHashE` row in §6.2's inheritance table is retained for traceability but is **structurally unconsumed** by verified-rcv for three independent reasons:
- (i) **pigeonhole-impossible (d) sub-tag**: Quartz's `commitHashE` `(d) pigeonhole-impossible` classification makes the bundle a tautological lift (its win event is `false` under the bundle's own definition), so consuming it gives a vacuous bound. The Quartz colosseum's lifting refactor (cycle 6.x) does not change the `(d)` classification; only the def-tying changes.
- (ii) **Fintype carrier mismatch**: `commitHashE` is parameterised over `[Fintype UserData]` in Quartz, but verified-rcv's `UserData = (contract_addr, tally_body)` is **not** `Fintype` in the verified-rcv Lean spec (the `tally_body` field has unbounded ranges per the v0.3.1 Performance Bounds removal). Consuming `commitHashE` would require a `Fintype` carrier refinement that verified-rcv does not provide.
- (iii) **Structural UserData-slot-interface non-consumption**: verified-rcv consumes Quartz's `UserData` *slot interface* (the typed slot for binding chain-side data to enclave attestation) but does not consume `commitHashE`'s *collision-resistance-as-pigeonhole* statement. The slot interface and the pigeonhole statement are distinct artifacts in the Quartz spec; verified-rcv's compose ledger flags the slot-interface consumption as a live dependency and the pigeonhole statement as `consumer_count = 0`.

The `Adv_commitTally_CR` hypothesis in B9 is **stated directly at the verified-rcv layer** as SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`, not derived from `commitHashE`.

**Known v0.3 caveats inherited from Quartz** (per the Quartz-agent feedback on the implementation):
- **Asks 6+7** were added during the cycle-6.4-through-6.11 work: (6) per-conjunct failure-mode analysis as bundle-count source (the prior plan over-bundled 7 of 8 lifts); (7) degenerate-zero-advantage cycles must declare intent (cycles 6.7 and 6.8 produced lifts with `failAdv 𝒜 n = 0` identically; without explicit declaration, auditors cannot tell "intentionally degenerate within scope" from "structurally trivial, not a security lemma"). These asks have *documented* form in the Quartz PR; *enforced* form is gated on the executable-layer decision.
- The verified-rcv compose ledger (forthcoming) records both asks as Quartz-side methodology obligations that propagate to verified-rcv's bundle-cardinality readout.
- **`IsPPT` predicate is partially-closed** (Quartz Round A finding #5; verified-rcv audit 2026-05-26). Quartz's `_negl` lifts are parametric over a "polynomial-time adversary" notion (`IsPPT`); the predicate is currently defined as `True` in Quartz's Lean (with the partially-closing rename `*_AGAINST_UNBOUNDED_ADVERSARIES` to make the gap explicit). The polynomial-adversary notion has not been substantively instantiated. Quartz cycle 6.14 is queued for the substantive instantiation. verified-rcv inherits this caveat: B9's PPT quantifier carries the same loosening until Quartz cycle 6.14 lands. This does NOT make B9 vacuous (the negligibility-budget structure stands), but the strength of "PPT adversary" in verified-rcv's claim is precisely as strong as Quartz's `IsPPT` at the moment of consumption.
- **`Ecies.roundtrip` is at spec-layer abstraction** (verified-rcv audit 2026-05-26). Quartz's `proofs/lean/Specs/Quartz/Crypto/Ecies.lean:149-151` proves `Ecies.roundtrip` against the abstracted model where `Ciphertext := PubKey × Plaintext` and decrypt is deterministic. Real ECIES randomness, ECDH derivation, and curve operations are NOT modelled in the theorem. verified-rcv inherits the abstraction: Stage 1 decryption-correctness depends on this spec-layer roundtrip, which is sound for the abstraction's assumptions but does not constitute a verification of the implementation's ECIES at the cryptographic-primitive level. The implementation-level ECIES soundness is an honest computational assumption layered on top of Quartz's spec-layer proof.

Composition theorem: B9 (Section 3.2) formalizes the negligibility-budget decomposition for B8. The verified-rcv compose ledger (forthcoming) will record this dependency graph and pick up bundle-cardinality drift signals from the Quartz ledger.

### 6.3 dstack / TDX hardware trust

verified-rcv trusts:

- **TDX integrity** — the TDX platform isolates the enclave's memory from the untrusted host; the enclave's runtime state is not observable to the host operator. Failure mode 4.1.
- **dstack KMS honesty** — the KMS releases the per-election private key only after verifying the requesting enclave's attestation. Failure mode 4.2.
- **MRTD / RTMR accuracy** — the attested enclave-identity measurement faithfully represents the executing code. Required for B8 clause (d).

### 6.4 Caller contract (instantiator)

(Refined per v0.3.1 subagent-dispatch synthesis F4 — disputed-severity finding from kimi scope #5 + gemma scope #1: the v0.3.0 "is trusted to declare candidates honestly" phrasing was toothless — trust without a check is not a guarantee; clarified below.)

The party calling `instantiate`:

- **Must supply** a non-empty, distinct candidate set; a valid voting window with `start_at > env.block.time`; consent to the no-admin-recovery posture (the chain enforces these via Block 1's Requires).
- **Off-chain responsibility — candidate declaration**: the instantiator picks the candidate set, and the chain does not gate candidate legitimacy. Whether the addresses in `candidates` actually correspond to real, consenting, IRL candidates is *not* a property the system claims — it is a deployment-time off-chain responsibility on the instantiator and the watching public. This is **not** "trust" in the cryptographic / inheritance sense; it is the explicit scope boundary stated in §5 ("No on-chain candidate-consent gate"). Where v0.3.0 said "is trusted to declare candidates honestly," v0.3.1 reads: "no on-chain check is performed; the instantiator's declaration is the candidate set, full stop. Off-chain inspectability (publicly visible `candidates` field, publicly visible non_voters / participation pattern in the tally) is the only legitimacy signal the system provides."
- **Cannot recover** from misconfiguration — instantiation parameters are immutable; mistakes require a fresh contract instance.

### 6.5 Voter contract (candidate-submitter)

Each candidate submitting a ballot:

- **Must** encrypt to the contract's advertised `enclave_pubkey` using ECIES (the contract does not verify encryption shape on-chain; malformed ciphertext is dropped by the enclave at tally time).
- **Must** rank all candidates in their ballot (else dropped).
- **Is responsible for** their own chain private key security (failure mode 4.7).
- **Trusts** that the enclave will tally faithfully against the spec (the inverse failure mode 4.4 is the methodology target).

**AEAD / authenticated-ciphertext precondition note** (added per v0.3.1 subagent-dispatch synthesis T14 — 2 voices: claude S7 / kimi flagged the Stage-1 soundness gap). ECIES-as-typically-specified provides *passive-eavesdropper* confidentiality (semantic security against a chosen-plaintext adversary observing the ciphertext on chain) and *honestly-encrypted-plaintext roundtrip* correctness (the Quartz `Ecies.roundtrip` theorem). It does **not**, by itself, provide *authenticated decryption* — i.e., it does not bind a successfully-decrypted plaintext to the identity of the *encryptor*. A non-candidate adversary cannot make the contract accept their ballot at the contract layer (B6 blocks `msg.sender ∉ candidates`), but a non-candidate adversary who has observed candidate A's *encrypted* ballot on chain and obtained A's chain private key (4.7) can re-encrypt arbitrary plaintexts under the enclave's public key and have the enclave decrypt them at tally time. The chain-layer B6 check pins the *submitter*, not the *encryptor*; combined with `Ecies.roundtrip`, the enclave's Stage-1 partition (Section 2.5 + §8.7 step 1) is "the chain accepted this ciphertext from the named candidate, and the enclave's decryption of the chain-accepted ciphertext is well-defined." It is *not* "the named candidate authored this plaintext under their consent" — that stronger claim is *out of scope* this round and would require an AEAD-style binding the verified-rcv enclave does not currently enforce. The verified-rcv compose ledger records this as a tracked upstream ask on Quartz; for now, the Stage-1 soundness obligation is qualified as "deterministic partition over chain-accepted ciphertexts, with the encryptor-identity binding inherited only from B6's submitter check."

### 6.6 Output contract (consumers of `tally_result`)

(Restated per v0.3.1 subagent-dispatch synthesis T13 — 2 voices: kimi trust-quartz / gemma flagged the v0.3.0 form as over-claiming categorical guarantees and missing the well-formedness-vs-semantic-correctness gap.)

Any party reading `tally_result` from chain state can rely on the following — **all conditional on `image_registration_honest(σ)` holding at the firing transition** (Section 6.1; failure mode 4.3b is the negation):

- `tally_result.is_some()` ⇒ a valid gnark Groth16 proof + `public_inputs` blob was supplied at `publish_result` time and verified via `/xion.zk.v1.Query/ProofVerifyGnark` against the chain-registered `vkey_name` (B8 v0.3.9 form); the proof's `public_inputs` carried measurements (MrTd, Rtmr1, Rtmr2, optionally Rtmr0/Rtmr3) matching the on-chain `EnclaveImageRegistry`; the proof's `ReportData[0..32]` equaled `SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))`; the proof's `ReportData[32..64]` equaled the publish-purpose domain-separation tag `DST_VERIFIED_RCV_TALLY_V1` padded to 32 bytes.
- The tally body satisfies the well-formedness invariants S6 + S7 + S8 + S9 — **structural well-formedness only** (winner-membership, count-conservation, set-disjointness of eliminated rounds). Well-formedness is **not** semantic correctness: a tally can be well-formed but wrong (failure mode 4.4 — enclave software bug). The semantic-correctness guarantee is B10, and B10 itself is a cross-layer composition (Section 8.7) — its discharge structure includes `B10_lean` (Lean-internal correctness of the extracted enclave model), image-identity-binding (build-pipeline obligation), `enclave_input_fidelity` (the input the enclave saw equals `ballots@end_at`), and `dstack_kms_trust` (the per-election private key came from the dstack KMS).
- The result is terminal and will not change (B1).
- Per-round counts and elimination sequence are auditable directly from the result; no enclave re-query needed.

What callers **cannot** infer from a published `tally_result`:

- Whether the registered vkey/MRTD/RTMR was the legitimately-issued verified-rcv tuple (4.3b — out of band; equivalently, whether `image_registration_honest(σ)` actually holds for the chain σ they are reading from).
- Whether the enclave hardware was uncompromised at tally time (4.1 — out of band).
- Whether individual voter preferences were as the voter intended (4.7 — out of band).
- **Whether the tally is semantically correct** (matches the IRV specification on the given ballots). This is B10's load (Section 3.2 + Section 8.7); a well-formed `tally_result` whose attestation passed is *consistent with* B10 holding, but well-formedness alone does not imply B10. A reader who wants the semantic-correctness guarantee must additionally trust the off-chain links of B10's discharge — the `B10_lean` proof for the registered image hash, the build-pipeline ledger entry binding the registered MRTD/RTMR to that proof, and the operational `dstack_kms_trust` assumption. A reader unwilling to extend trust to those off-chain links has only well-formedness from the chain alone.
- **Whether the ZK module endpoint is algorithmically sound** (4.12 — out-of-band Xion ZK module risk surface).

## 7. Performance Bounds

**There is no correctness-relevant performance bound.** Gas consumption is a *non-functional* property of this system (Round 3a first-pass adversary Attack 24): a transaction that exceeds chain gas limits fails to execute, leaving state unchanged — which is a transaction outcome, not a violation of any invariant in Section 3. Gas overflow is not a spec violation.

The intent doc records the following gas **target** (non-functional, design-time):

- **Design target: every on-chain handler call consumes ≤ 1,000,000 gas units.** This applies to `instantiate`, `submit_ballot`, `close_and_tally`, and `publish_result`. Treated as a design-time goal for the implementation, not as a correctness invariant. A handler call that exceeds this in deployment is a configuration issue (input size out of envelope) or implementation regression, not a spec failure.

Consequences (informational):

- **Indirect cap on `len(candidates)`**: `publish_result` is the most expensive handler — it issues a gRPC query to `xion.zk.v1.Query/ProofVerifyGnark` (carrying 9,792-byte `public_inputs` + ~256-byte gnark proof; verification cost is chain-side and constant-time-in-N from verified-rcv's perspective) and writes the `TallyResult` containing `O(N²)` per-round count entries for `N` candidates. The 1M gas target translates to a practical upper limit on `N` that depends on the chain's per-byte storage gas plus the cross-module gRPC query cost. Estimating conservatively (Cosmos SDK storage ~30 gas per byte written + base contract execution + ProofVerifyGnark call), the target is met up to roughly **N ≤ 20–30 candidates**. Downstream verification (Kani / Verus harnesses on the contract) should produce a tight bound; until then, this is an informal estimate, not a verified property.

- **No real-time / latency guarantees**: the spec makes no claim about *when* a tally appears after `end_at`. The enclave may publish promptly, late, or never (failure modes 4.6, 4.9 cover the deadlock cases). The 1M gas target governs per-call cost, not end-to-end latency.

Rationale for choosing 1M:

- Conservative relative to Cosmos SDK block gas limits (typically 100M–200M for Xion-class chains), giving the contract room to coexist with other transactions in the same block.
- Round number that downstream Kani harnesses can target directly as a static-analysis bound.
- Forces the contract design toward bounded-size state structures rather than open-ended growth.

If a deployment requires `N > 30`, either raise this bound explicitly in a future intent-doc revision (with the chain-gas justification) or restructure `publish_result` to spread tally publication across multiple transactions. The latter would change the state machine (introducing a `Publishing` sub-state of `Tallying`); not addressed in this intent.

## 8. Concrete Scenarios

Narrative walkthroughs of key flows. Specs and tests derive directly from these.

### 8.1 Clean election with clear winner

> Given a 5-candidate election where all candidates vote and one candidate ultimately achieves majority, when the voting window closes and the enclave tallies, the system publishes the per-round counts and the winner; the result is terminal and the on-chain state is final.

Step-by-step:

1. Alice instantiates a contract with candidates `[A, B, C, D, E]`, `start_at = T0 + 60s`, `end_at = T0 + 1h`. DstackKeyManager issues a per-election keypair. Contract enters Created state.
2. At `T0 + 60s + δ`, block time passes `start_at`; contract enters Voting state.
3. Each candidate fetches the `enclave_pubkey` from contract state, encrypts their full preference ranking under ECIES, and calls `submit_ballot`. Contract checks `msg.sender ∈ candidates` and stores the ballot. Order of submission does not matter; last-write-wins per voter.
4. At `T0 + 1h`, block time passes `end_at`; contract enters Tallying state. Late `submit_ballot` calls are now rejected with `NotInVotingWindow`.
5. Anyone (typically the verified-rcv enclave-watcher) calls `close_and_tally`. This emits a Cosmos SDK event but does not change contract storage. The enclave-watcher service detects the event.
6. The enclave decrypts all stored ballots, validates each as a permutation of `candidates`, runs IRV tabulation (single + batch elimination per Section 2.5), produces a `TallyResult` with per-round counts, elimination sequence, and winner.
7. The enclave requests a publish-purpose TDX quote whose `report_data = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body)) ‖ DST_VERIFIED_RCV_TALLY_V1_PADDED`, drives the zkdcap gnark prover to produce `(proof, public_inputs)`, and calls `publish_result { tally, proof, public_inputs }`. The contract queries `/xion.zk.v1.Query/ProofVerifyGnark`, extracts and compares measurements + ReportData per §2.5 layout, checks tally well-formedness, and sets `tally_result = Some(tally)`. Contract enters Resolved state.
8. Any chain user queries the contract; the published `tally_result` shows: winner address, per-round counts, eliminated candidates by round, ballots tallied / dropped / non-voters.

Trust claim consumed: B8 + B9 — the published result is bound to the verified-rcv enclave's attested computation, and its trustworthiness reduces to the **3-summand** negligibility budget defined in Section 3.2 (revised per v0.3.1 F6 — the v0.3.0 text said "5-summand" before v0.3.0's B9 retag moved `Adv_circuit_eq` from the sum to a correctness precondition and dropped a stale fifth summand; the current sum is `Adv_tdxVerifier_sound + Adv_groth16_KS + Adv_commitTally_CR`).

### 8.2 Abstention-heavy election

> Given a 5-candidate election where only 2 candidates submit ballots, when the voting window closes and the enclave tallies, the system produces a result over the 2 submitted ballots; the 3 non-voting candidates appear in `non_voters` in the published output, making participation publicly inspectable.

Step-by-step:

1. Alice instantiates with `[A, B, C, D, E]`, window as before. Contract enters Voting.
2. Only A and C submit ballots. B, D, E never call `submit_ballot`.
   - A's ballot: `A > B > C > D > E`
   - C's ballot: `C > A > B > D > E`
3. At `end_at`, contract enters Tallying.
4. Enclave reads `ballots` (2 entries), tallies:

   | Round | A | B | C | D | E | Action |
   |---|---|---|---|---|---|---|
   | 1 | 1 | 0 | 1 | 0 | 0 | B, D, E tied for lowest at 0. Batch-eliminate B, D, E. |
   | 2 | 1 | — | 1 | — | — | A and C tied 1-1. Terminal tie → both co-winners. |

5. Enclave publishes:
   ```
   winners            = [A, C]
   per_round_counts   = [{A:1, B:0, C:1, D:0, E:0}, {A:1, C:1}]
   eliminated_by_round = [[B, D, E]]
   ballots_tallied    = 2
   ballots_dropped    = 0
   non_voters         = [B, D, E]
   ```
6. The on-chain result makes it visible that B, D, E did not vote (in `non_voters`). Whether this is interpreted as "no result" or "co-winners" is a UI concern; the system's output is mathematically defined.

Trust claim consumed: same as 8.1.

### 8.3 Deadlock via missing enclave publish

> Given a clean election that has reached Tallying, when the enclave fails to ever publish a result (e.g., the enclave host's KMS is permanently unreachable), the election state remains in Tallying indefinitely; no party can force resolution and no admin recovery exists.

Step-by-step:

1. Election proceeds normally through Created → Voting → Tallying as in 8.1, steps 1–5.
2. The enclave host loses connectivity to dstack KMS (operational failure outside the chain's view). The enclave cannot retrieve the per-election private key.
3. `close_and_tally` may be called any number of times; each emits an event but no state changes. Block 5 is idempotent and has no liveness obligation.
4. The enclave never calls `publish_result`. `tally_result` remains `None` indefinitely.
5. The contract state is permanently stuck: derived state is Tallying, no further transitions exist, and no party (instantiator, candidates, chain admin, enclave) has a mechanism to abort, re-trigger, or migrate.
6. Eventual recovery is **out of scope for this instance** — a fresh contract must be instantiated; the deadlocked instance remains as a permanent on-chain record of a failed election.

Trust claim consumed: the explicit failure mode 4.6 + Section 5 non-goal "no admin / creator recovery role". The deadlock is part of the spec, not a bug.

This scenario is the canonical example of why the no-admin-recovery decision is load-bearing: introducing recovery would add an admin role (and its trust claim) to the system. The methodology accepts deadlock-as-spec'd-behaviour to preserve the trust surface.

### 8.4 Premature voting attempt rejected + post-`end_at` rejected (witnesses for B4 + B2 + B3)

(Extended per v0.3.1 subagent-dispatch synthesis T12 — 2 voices: kimi scenarios / gemma scope flagged the v0.3.0 §8.4 as witnessing only the pre-`start_at` half of B4 and missing B2 + B3 entirely.)

> Given a freshly instantiated contract in Created state, when a candidate attempts to submit a ballot before `start_at`, the contract rejects with `NotInVotingWindow`. Given the same contract after `end_at`, the contract similarly rejects any `submit_ballot` call; combined, these two rejections witness B4 (no premature voting), B2 (`ballots@end_at` is frozen), and B3 (no premature tally — its `submit_ballot`-side complement).

**Step-by-step (pre-`start_at` half, witnesses B4):**

1. Alice instantiates with candidates `[A, B, C]`, `start_at = T0 + 1h`, `end_at = T0 + 2h`. Contract enters Created.
2. At `T0 + 30s` (well before `start_at`), candidate A — perhaps misreading the schedule, perhaps testing — calls `submit_ballot` with their encrypted preference list.
3. Block 3 evaluates: derived state is Created (`env.block.time < start_at`), not Voting. Requires clause "derived state is Voting" fails.
4. Transaction reverts atomically with `ContractError::NotInVotingWindow`. `ballots` remains empty; no event is emitted.
5. At `T0 + 1h + δ`, the contract enters Voting. A may now resubmit successfully.

**Step-by-step (post-`end_at` half, witnesses B2 + B3, added in v0.3.1):**

6. Election proceeds to Voting at `T0 + 1h`. Candidates A and B submit ballots at `T0 + 1h + 5min` and `T0 + 1h + 10min`. The post-Voting view of `ballots`:
   ```
   ballots = { A: ECIES(...), B: ECIES(...) }
   ```
7. At `T0 + 2h + δ` (just past `end_at`), candidate C — who has not yet voted — calls `submit_ballot` with their encrypted preference list.
8. Block 3 evaluates: derived state is Tallying (`env.block.time ≥ end_at`), not Voting. Requires clause "derived state is Voting" fails.
9. Transaction reverts atomically with `ContractError::NotInVotingWindow`. `ballots` is unchanged — *the value `ballots@end_at` (defined as the snapshot of `ballots` at the firing of the Voting → Tallying transition, per §2.5) is preserved past the transition*; B2 holds.
10. **B3 corollary**: because Block 5 (`close_and_tally`) is read-only, B3 is preserved trivially as a tally-side claim. The `submit_ballot`-side complement of B3 (no late ballot can sneak into the tallied set) is witnessed here: post-`end_at` rejection means the enclave's input set is exactly `ballots@end_at`, satisfying B3's "the input to Block E1 equals `ballots@end_at`" complement.

Trust claims consumed:
- **B4** (no premature voting) — the pre-`start_at` rejection witnesses that `ballots` does not change before `start_at`.
- **B2** (ballot frozen at `end_at`) — the post-`end_at` rejection witnesses that `ballots@end_at` is preserved as a snapshot, with the only handler that writes to `ballots` (Block 3) gating on the time-derived state predicate.
- **B3** (no premature tally — `submit_ballot`-side complement) — the post-`end_at` rejection witnesses that no late ballot is admitted into `ballots@end_at`, which the enclave reads as input to `Tally_spec`.

### 8.5 Double-publish attempt rejected (witness for B5)

> Given an election that has reached Resolved, when a second `publish_result` transaction is attempted — whether from the legitimate enclave (retry after network blip), a buggy enclave, or an adversary mimicking the enclave — the contract rejects with `AlreadyResolved`; the original `tally_result` is unchanged.

Step-by-step:

1. Election proceeds through 8.1 steps 1–7 to Resolved. `tally_result = Some(T1)`.
2. Some time later, the enclave (or an attacker) submits a second `publish_result` carrying `T2` (possibly different from `T1`) and a valid attestation.
3. Block 6 evaluates: derived state is **Resolved** (because `tally_result.is_some()`), not Tallying. Forbids clause `tally_result.is_some()` triggers.
4. Transaction reverts atomically with `ContractError::AlreadyResolved`. `tally_result` is still `Some(T1)`; no state mutates.
5. Even if the second `publish_result` carried a *legitimate* attestation for a corrected tally `T2`, it is rejected. There is no on-chain mechanism to replace `T1` with `T2`. This is the no-re-tally non-goal in action.

Trust claim consumed: B5 (publish_result fires at most once) — the temporal property that a successful publish_result transition fires at most once per contract instance. Combined with B1 (monotone-once-set), this guarantees the published result is the unique published result.

### 8.6 Ballot writer is the ballot voter (positive + negative witnesses for B6)

(Restructured per v0.3.1 subagent-dispatch synthesis T12 — 2 voices: kimi scenarios / gemma flagged the v0.3.0 §8.6 as only witnessing the *rejection* face of B6 and missing the *positive existential* face. B6's revised form (`∃! msg ∈ trace. msg is submit_ballot ∧ msg.sender = k ∧ writes ballots[k]`, see §3.2) requires both an existence witness on the positive side and a non-existence witness on the negative side.)

> Given an active election in Voting state where candidate A submits a ballot, the contract writes `ballots[A]` iff the writer is A; given the same election where a non-candidate or wrong-candidate writer attempts to write the same slot, the contract rejects. Together these witness B6's uniqueness-of-writer property.

**Step-by-step (positive existence witness, added v0.3.1):**

1. Election with candidates `[A, B, C]` is in Voting state. `ballots = {}` initially.
2. At `T0 + 1h + 5min`, candidate A signs and broadcasts a `submit_ballot` transaction with encrypted preference list `enc_A`. The chain enforces signature validity; `msg.sender == A` in the resulting tx.
3. Block 3 evaluates: `msg.sender == A`, `A ∈ candidates`, derived state is Voting. All Requires hold.
4. The handler writes `ballots[A] = enc_A` and emits `BallotSubmitted { voter: A }`. Post-tx state: `ballots = { A: enc_A }`.
5. **Uniqueness of writer (B6 positive face)**: the only handler that writes to `ballots[A]` is Block 3, and Block 3's Requires include `msg.sender ∈ candidates` — for the write to slot `A` specifically, the handler-internal binding is `ballots[msg.sender] = body`, so the writer of `ballots[A]` is exactly the signer of the tx whose `msg.sender` equals `A`. Combined with the chain's signature-verification axiom (§6.1 sender-authentication), the writer of `ballots[A]` is the holder of A's chain private key at write time. There is **exactly one** contract message in the trace with `msg.sender = A` *that actually wrote* (per the v0.3.1 retag of B6 to "∃! msg with successful write"; failed-Requires messages don't count). Last-write-wins overrides count as multiple successful writes per slot — B6's uniqueness is stated as ∃! writer-identity, not ∃! write-event; see §3.2.

**Step-by-step (negative non-candidate rejection witness, preserved from v0.3.0):**

6. A non-candidate address `Eve` — perhaps controlled by an adversary who wants to inject a ballot influencing the tally, perhaps just a misclick — calls `submit_ballot` with an encrypted preference list.
7. Block 3 evaluates: `msg.sender == Eve`. Requires clause `msg.sender ∈ candidates` fails because `Eve ∉ [A, B, C]`.
8. Transaction reverts atomically with `ContractError::NotACandidate`. `ballots` is unchanged; no event is emitted. **B6's non-existence face**: no message with `msg.sender ∉ candidates` successfully wrote to `ballots`; non-candidate writers cannot exist in the writer-set of any `ballots[k]`.

**Variant — stolen-key impersonation**: an adversary who *has* compromised candidate A's private key (failure mode 4.7) can submit a ballot as A. From the contract's perspective `msg.sender == A` is valid; the impersonation is indistinguishable from a legitimate submission. The contract layer does not detect this; it is a voter-side key-management failure handled by 4.7's recovery semantics (legitimate A may resubmit if they regain control before `end_at`).

Trust claim consumed: B6 (ballot writer is the ballot voter) — at the contract layer, the chain's signature verification + `msg.sender` semantics enforce that whoever wrote `ballots[k]` was the holder of `k`'s private key at write time. The positive + negative witnesses together cover both faces of the revised B6 (`∃! msg` form, see §3.2). The stolen-key variant clarifies that B6 is a *contract-layer* claim, not an end-to-end "intended voter" claim; the latter is out of scope (Non-Goal: "No coercion resistance" + "No key-management at the contract layer").

### 8.7 B10 witness — cross-layer discharge (5-link composition: Lean + image-binding + B8 + dstack-KMS-trust + input-fidelity)

(Restructured per v0.3.1 subagent-dispatch synthesis T3 + T1 — 4 voices on T3 (claude / kimi / gpt-oss / mistral) flagged that the v0.3.0 3-link factorization `B10_lean ∧ image-identity-binding ∧ B8` was incomplete: two additional load-bearing conjuncts — `dstack_kms_trust` (the per-election privkey actually came from the dstack KMS for the registered enclave image) and `enclave_input_fidelity` (the input the enclave saw equals `ballots@end_at`) — must be explicit, else the composition does not actually entail the B10 conclusion. 5 voices on T1 flagged that `EnclaveImage` had no explicit type signature; the §2.5 typing is now referenced here.)

> Given a candidate enclave image binary, B10 is witnessed by a **5-link cross-layer composition**: a Lean-internal correctness theorem (`B10_lean`, the image-IO obligation, typed over the §2.5 `EnclaveImage` symbol) combined with image-identity binding (the registered MRTD/RTMR on-chain matches the Lean-extracted binary), B8 (attestation chain witnesses), `dstack_kms_trust` (the per-election privkey came from the dstack KMS for the registered image), and `enclave_input_fidelity` (the input the enclave actually saw equals `ballots@end_at` plus the public Block 1 parameters). The chain alone neither verifies nor witnesses B10 directly; the discharge structure makes the cross-layer composition inspectable and assigns each layer its own obligation.

**Restated per v0.3.1 subagent-dispatch synthesis themes T1 + T3 + T10.** The v0.3.0 §8.7 distinguished `B10_lean` from `B10` (a real improvement over the pre-v0.3.0 conflation) but left two load-bearing conjuncts implicit and gave `EnclaveImage` no explicit type signature. The restructured form pins `EnclaveImage` to its §2.5 typing and makes the 5-link composition explicit.

**`EnclaveImage` typing reference (T1)**: `EnclaveImage` is defined in §2.5 (Block E1) as a deterministic function of type
```
EnclaveImage : (raw_ballots : Vec<(Addr, Ciphertext)>) → (candidates : Vec<Addr>) → (privkey : DstackPrivkey) → TallyResult
```
where the chain-deterministic order of `raw_ballots` is the candidate-declaration order over the keys of `state.ballots@end_at` (Stage 1 input discipline, §2.5). The `EnclaveImage` symbol is the **Lean-extracted model** of the deployed enclave binary, not the binary itself — the binary-to-model binding is link 4 below.

Structure of the discharge (5 links; downstream Lean + ledger deliverables, not part of this intent doc):

1. **Stage-1 lemma — `decrypt_and_validate` correctness**: for every `(raw_ballots, candidates, privkey)`, the Stage-1 implementation returns the partition `(valid_ballots, dropped_voters, non_voters)` matching the spec in Section 2.5. Depends on Quartz's `Specs.Quartz.Crypto.Ecies.roundtrip` for the decryption-correctness half (Section 6.2 inheritance row 3 — a *theorem*, not an axiom). **AEAD caveat (cross-reference to §6.5 v0.3.1 note + Round 3a re-adversarial theme 9 / claude S7)**: `Ecies.roundtrip` proves *honestly-encrypted plaintexts roundtrip*; it does not bound *adversarial / malformed ciphertexts*. The Stage-1 spec is stated as "deterministic decrypt-or-drop" over chain-accepted ciphertexts (correctness of the partition is definitional once the decoder is fixed); the soundness claim "a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate" requires an additional AEAD-style binding the verified-rcv enclave does not currently enforce. Recorded as a v0.3 upstream ask on Quartz; verified-rcv's compose ledger flags the Stage-1 soundness obligation as upstream-blocked.
2. **Stage-2 theorem — `IRV_spec` correctness**: for every `(valid_ballots, candidates)`, the Stage-2 implementation returns the `IRV_spec` output. The inductive structure is on round count; termination is by `|remaining|` strictly decreasing (or hitting the terminal-tie base case). No upstream dependency — fully internal.
3. **Lean-internal composition — `B10_lean`**: by composition of the Stage-1 lemma and Stage-2 theorem, the Lean-extracted enclave image's externally observable input-output relation satisfies `EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`. This is a Lean theorem about the *Lean-extracted* model of the enclave binary, parameterised over `privkey`. It is **not** B10 itself; it is one of B10's load-bearing components. The §2.5 typing pins the function signature; this step proves it equals `Tally_spec`.
4. **Image identity binding**: the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that `B10_lean` is over. This is an **extraction-soundness + build-reproducibility** obligation that lives at the *build pipeline + deployment ledger* layer, not the Lean layer. It cannot be a Lean theorem because the Lean spec models the binary; the binding claim is *about* the modelling discipline. The verified-rcv compose ledger records this as a tracked obligation with a discrete artifact (reproducible build hash + registered MRTD/RTMR equality, including the Lean-extraction-soundness sub-obligation: the extracted model faithfully represents the binary's deterministic input-output behavior).
5. **B8 (chain-side, already covered; v0.3.9 reformulation)**: the gnark proof verification via `/xion.zk.v1.Query/ProofVerifyGnark` (issued by the verified-rcv contract directly, per §2.5 "Gnark verification primitive") witnesses that the firing transition's tally was produced by an enclave whose code measurements (MrTd, Rtmr1, Rtmr2, and optionally Rtmr0, Rtmr3) match the registered `EnclaveImageRegistry` and whose `ReportData[0..32]` commits to `SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))` (with the publish-purpose DST in ReportData[32..64]). Conditioned on `image_registration_honest(σ)` at the firing transition (per the v0.3.1 B9 retag; v0.3.9-amended schema). **v0.3.9 narrowing**: this link no longer relies on a future-work TDX-quote-parser or a deferred Groth16 verification — both are now performed in-contract every publish via the chain's `xion.zk` module. The trust surface moves from "Quartz `tdxVerifier` + `groth16Verifier` axioms (deferred to off-chain reduction proofs)" to "Xion `xion.zk` module ProofVerifyGnark correctness (chain-trust, §6.1) + reference DCAP gnark circuit correctness (`circuit_equivalence_honest`)."
6. **`dstack_kms_trust` (operational link, added v0.3.1 T3)**: the per-election private key released by the dstack KMS to the running enclave is the key derived under the dstack KMS's per-image key-derivation discipline for the registered (MRTD, RTMR) tuple. This is an **operational assumption** on dstack-KMS implementation honesty — not a Lean theorem, not a Quartz-Lean inheritance row (the v0.2.1 withdrawal of the `DstackKeyManager` inheritance row stands; see §6.2). It is the same trust surface as failure mode 4.2 (KMS key compromise — confidentiality face) viewed from its integrity-of-keying-discipline face: KMS-honest ⇒ the privkey the running enclave saw is bound to the registered image identity. Without this conjunct, link 5 (B8) would witness "*some* image with the attested identity ran" but not "the image that ran got the *correct* privkey," and `EnclaveImage(_, _, privkey)` with an attacker-supplied privkey could disagree with `Tally_spec(_, _, true_privkey)` on the same ballot set (the privkey enters the Stage-1 decryption).
7. **`enclave_input_fidelity` (cross-layer link, added v0.3.1 T3; DISCHARGED v0.3.11 B6)**: the input the enclave actually decrypted equals `ballots@end_at` (the chain-side frozen ballot set, per B2 + §2.5 Stage-1 input discipline) plus the publicly-readable `candidates` and time fields. **Discharge (v0.3.11 B6)**: chose option (i) from the v0.3.9 deferred-decision menu — extend `canonical_serialization` with a `ballots_hash[32]` field. At PublishResult time the contract computes `chain_ballots_hash = compute_ballots_hash(election.candidates, BALLOTS.range(...))` walking candidates in declaration order; the runtime computes `runtime_ballots_hash = compute_ballots_hash(candidates, raw_ballots)` from its received input. Both feed `ReportData[0..32] = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ tally_body))`. A host that substitutes `raw_ballots` produces a `runtime_ballots_hash` different from the chain's `chain_ballots_hash`; the chain's `verify_publish_quote` then fails `AttestationCommitMismatch`. **Cross-test `compute_ballots_hash_contract_vs_runtime_byte_identical`** asserts byte-equality of the two implementations. Collision-resistance hypothesis: the same `Adv_commitTally_CR` (SHA-256) already in B9 — no new cryptographic assumption.
8. **Composition — B10**: from `B10_lean` (links 1+2+3) + image-identity-binding (link 4) + B8 (link 5) + `dstack_kms_trust` (link 6) + `enclave_input_fidelity` (link 7), with the existential quantifier in B10 instantiated by the dstack-KMS-derived `privkey` for the registered (MRTD, RTMR) tuple: the published `tally_result` equals `Tally_spec(ballots@end_at, candidates, privkey)`. This is the B10 statement from Section 3.2.

**Why B10 is `cross-layer`, not `temporal`**: B10's witness traverses five layers — Lean-internal theorem (`B10_lean`, off-chain), build/deployment ledger (image-identity-binding, off-chain), chain-side attestation predicate (B8), operational KMS-keying-discipline assumption (`dstack_kms_trust`, off-chain), and cross-layer input binding (`enclave_input_fidelity`, attestation-aided). A *temporal* invariant has a chain-trajectory-only witness; B10 cannot, by construction. A chain-side walkthrough cannot exhibit a B10 violation directly — any tally the chain accepts is one where Block 6's well-formedness Requires hold and the attestation verified; the *correctness* gap (well-formed-but-wrong, Section 4.4) is invisible at the contract layer. Tagging B10 as `cross-layer` makes the discharge structure honest about which layers carry what part of the proof. **Failure of any of the five links produces an undetectable B10 violation on-chain**: proof regression on `B10_lean`, build/registration mismatch (link 4), B8 violation under 4.3b, KMS-keying-discipline failure (link 6 ~ 4.2's integrity face), or host-substituted-input attack (link 7 not yet discharged).

Trust claim consumed: B10 (tally-correctness). Composition pattern (v0.3.1): `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack_kms_trust ∧ enclave_input_fidelity`. The split makes the methodology factor cleanly: chain-side invariants (B1–B9) discharge via Quint + chain trajectory checks; `B10_lean` discharges via Lean; image-identity-binding discharges via the build pipeline + ledger; B8 spans Quint (the temporal predicate) + Quartz inheritance (the cryptographic content); `dstack_kms_trust` is an operational assumption with the same trust surface as §6.3; `enclave_input_fidelity` discharges via either an extended attestation `report_data` schema or a chain-inclusion-proof obligation (Lean-phase decision). B10 itself is the cross-layer aggregate.

---

## Open Questions

`TBD:` markers — questions the user could not answer at intent-doc time but that downstream specs will need resolved.

*(None outstanding as of Section 2 close. Will accumulate as later sections drill in.)*

## Revision Log

- **2026-05-14** — initial elicitation pass complete. All eight sections drafted: System Identity, Behaviors (2.1 happy + 2.2 boundary + 2.3 edge + 2.4 failures + 2.5 structured blocks), Invariants (10 structural S1–S10 + 9 behavioral B1–B9 with state/temporal tags, including B9 cross-project security reduction), Failure Modes (4.1–4.8 system-level), Non-Goals (13), Trust Boundaries (6.1–6.6 with explicit Quartz-inheritance dependency table), Performance Bounds (1M gas per handler), Concrete Scenarios (8.1 clean / 8.2 abstention / 8.3 deadlock / 8.4 premature voting rejected / 8.5 double-publish rejected / 8.6 impersonation rejected).
- **2026-05-14 (revision pass against Round 3a first-pass adversary findings)** — addressed 13 critical + selected serious findings from `.colosseum/attacks/intent-first-draft-2026-05-14.md`. Net changes:
  - **Section 2.1** worked example recomputed (Attacks 1 + 42); A wins instead of C; batch-elim mechanic now exercised correctly.
  - **Section 2.4** `InvalidTally` and `AttestationFailed` recovery columns reframed as conditional with explicit sub-causes (Attacks 11 + 15 + 27).
  - **Section 2.5** added `TallyResult` schema explicitly (Attack 34); added `IRV_spec` algorithm definition + batch-elim policy (Attack 10); added enclave-side Block E1 covering off-chain attested tally computation (Attack 36); Block 6 well-formedness gains set-relation clauses and explicit error-mapping (Attacks 28 + 32 + 35); Block 6 trigger clarified (any chain address, attestation is the trust anchor — Attack 40); cross-block discipline corrected re: Block 6 not being a self-loop (Attack 38).
  - **Section 3.1** S5 and S10 marked as derived corollaries of B1 and B3 (Attack 26); S6–S9 annotated with their temporal write-discipline complements (Attack 7).
  - **Section 3.2** **B6 re-tagged from `state` to `temporal`** with corrected next-state-relation form (Attack 6 — the methodology's central test, caught); **B10 (tally-correctness) added** as the load-bearing methodology-target invariant (Attack 20); **B9 restructured from 5 to 4 summands** — dropped `Adv_KMS_leakage` (Attack 17, confidentiality summand in an integrity bound) and `Adv_image_registration` (Attack 18, operational fault not a cryptographic advantage); added `Adv_commitTally_CR` to cover B8 clause (c)'s SHA-256 hash collision-resistance; **B5 marked as derived corollary** of B1 (Attack 33); **B8 clause (c) names SHA-256 explicitly** with classical-Prop shadow note (Attacks 5 + 16); B9 now carries explicit `inheriting retracted Quartz substrate` status block (Decision 1 — option ii).
  - **Section 4.9 added** (deterministic-malformed-tally deadlock — Attack 11) as the liveness face of 4.4's integrity face.
  - **Section 5** non-goals expanded with explicit operational-meaningfulness clause (Attack 21), explicit liveness-non-invariant clause (Attack 23), key-management vs coercion split (Attack 25), upstream-Quartz-axioms vs verified-rcv-primitives split (Attack 30).
  - **Section 6.2** `DstackKeyManager` inheritance row withdrawn (Attack 2 fabrication — `grep` returns zero hits in Quartz Lean tree); inheritance table reduced from 5 rows to 4. **B8/B9 substrate status block added** flagging Quartz's same-day retraction (`CORRECTION 2026-05-14` in Quartz ledger); resolution path is option (ii) "inherit retracted scaffolding explicitly" per user decision.
  - **Section 7** 1M gas demoted from "spec violation" to "design target / non-functional requirement" (Attack 24).
  - Cross-section consistency check passes; all temporal invariants retain scenario-level witnesses; the new B10 is the only temporal invariant without a Section-8 *chain-side* witness scenario, by construction (B10 quantifies over enclave-image behavior, not chain state). See sanity-pass fix below.
- **2026-05-14 (sanity-pass fixes — Claude + Mistral interim sanity-check synthesis, ahead of multi-model fan-out)** — applied 4 genuine findings:
  - **Section 2.5 IRV / Tally split** — introduced `Tally_spec(raw_ballots, candidates, enclave_privkey) → TallyResult` as the full Block E1 pipeline; pulled `decrypt_and_validate` out of the IRV core; `IRV_spec(valid_ballots, candidates)` now returns only the IRV-specific four-tuple. Resolves the B10 arity / RHS mismatch (mistral A2: `IRV_spec` was 2-arg in §2.5 but called with the decryption output in B10; also the `TallyResult` bookkeeping fields don't come out of the IRV recursion, they come out of Stage 1).
  - **Section 3.2 B10 restated** — uses `Tally_spec(ballots@end_at, candidates, enclave_privkey)`, frozen-at-`end_at` reading anchored by B2; explicit pointer to §8.7 for the off-chain witness story.
  - **Section 2.5 Block E1** — added explicit "enclave produces well-formed-but-wrong tally" failure-mode bullet cross-referencing Section 4.4 (mistral A4 — the load-bearing failure mode wasn't enumerated in the failure list); restated the B10 discharge obligation in input-output-relation form.
  - **Section 6.1** — added the **Enclave-image registration integrity** trust bullet that was promised in B9's note when `Adv_image_registration` was dropped per Round 3a first-pass adversary Attack 18 (Claude SANITY-PASS follow-up 1).
  - **Section 8.7 added** — B10 off-chain-witness scenario; describes the Stage-1 / Stage-2 / composition / image-identity-binding structure of the Lean discharge. Resolves the "B10 has no §8 witness" gap by explicitly classifying it as off-chain-by-construction with a separate witness structure.
  - **Not applied (design disagreement, not a fix)**: mistral's REGRESSION finding on B8/B9 retaining a retracted-substrate dependency. This is the explicit option-(ii) choice from the revision pass — the cross-project retraction is *visible and named* in §6.2, not silenced; surfacing it through the compose ledger is the v0.3 methodology demonstration.
- **2026-05-15 (revision pass against Round 3a re-adversarial fan-out — 7-voice ensemble verdict BREAKS-AGAIN; synthesis at `.colosseum/attacks/intent-revised-2026-05-14T200029Z/synthesis.md`)** — applied 6 multi-voice findings (themes 1+2+3+4+5+6 in synthesis nomenclature):
  - **Section 2.5 canonical_serialization** pinned to **Borsh** (themes 6+13, 3+ voices). Defines `canonical_serialization(contract_addr ‖ tally_body)` concretely as `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)` with declaration-order field emission; same Borsh discipline applies to Block E1 Stage 1's decrypted-plaintext parsing. Removes the under-specification that let two compliant-looking enclave implementations disagree on B8(c)'s hash.
  - **Section 6.2 substrate status block** updated to **`[2026-05-15 — de-retracted]`** (theme 5, plus user-relayed Quartz-agent feedback): the Quartz Lean tree implemented def-tying refactor across all 8 protocol-layer `_negl` lifts (cycles 6.4 through 6.11), resolving the 2026-05-14 retraction. PR shape now "v0.2 — back-port 7 asks + Round A adversarial review" (4 original + asks 5/6/7 added during implementation). Verified-rcv's B9 inheritance is now from a content-bearing substrate. Asks 6 (per-conjunct failure-mode analysis as bundle-count source) and 7 (degenerate-zero-advantage cycle intent declaration) recorded as Quartz-side methodology obligations that propagate to verified-rcv's compose ledger. **Section 6.2 `commitHashE` row** annotated **NOT CONSUMED by verified-rcv** with Note A (themes 8, claude S5 + gemma C2): verified-rcv uses `Adv_commitTally_CR` (SHA-256, verified-rcv-internal), not Quartz's pigeonhole-impossible `commitHashE` bundle; the row stays for traceability but downstream consumer count is zero.
  - **Section 3.1 S5 restated** as a *set-once write-discipline* property of the handler set (theme 4 — claude C1, gpt-oss S2, kimi A1, mistral c7). The prior `if tally_result.is_some() at σ, then any successor σ′ has...` formulation violated Section 3.1's preamble "No quantification over operations or time"; the new form (only Block 6 writes `tally_result`, gated on `tally_result.is_none()`) is pointwise-evaluable on the handler index. B1 continues to carry the trajectory claim.
  - **Section 3.2 B6 restated** to `∀-per-key + firing-transaction binding` (theme 3 — claude C3, gpt-oss C1+S7, kimi A2, qwen S5, glm S1, mistral C1). Old form `∃ tx, tx.msg.sender ∈ next.ballots ∧ ...` was satisfied by any historical SubmitBallot and didn't bind `tx` to the actual firing transition; failed under last-write-wins overwrites. New form `∀ k, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ ...` forces per-key attribution. **Section 2.5 transaction trace model paragraph added** as the formal substrate `fires_at_transition(σ → σ′)` lives in; downstream Quint encodes as action labels, downstream Lean as a relation parameter.
  - **Section 3.2 B9 conditioned** on `image_registration_honest` precondition (theme 7 / claude S4 / qwen S6 / kimi A3) — the LHS is now scoped to the cryptographic-failure subset of B8 violations, with the operational vkey-substitution disjunct (failure mode 4.3b) lifted out as a Section 6.1 trust-boundary precondition. Bound is no longer unconditionally false. Substrate provenance now points at the de-retracted Quartz lifts.
  - **Section 3.2 B10 restated** with `cross-layer` tag and existential quantifier `∃ privkey, dstack_kms_derived(privkey, contract_addr) ∧ ...` (theme 2 — claude S9, qwen C2, glm C3, kimi A6, mistral S9, gpt-oss S10). The prior chain-side temporal form referenced `enclave_privkey` as a free variable, ill-typed (chain has only `enclave_pubkey`). The new form lifts B10 out of chain-evaluable invariant class and ties the existential to Section 6.3's dstack-KMS trust boundary. **`B10_lean` added as a separate row** in the same table (theme 10 / claude S6) for the Lean-internal image-IO obligation, distinguishing it from B10 itself.
  - **Section 8.7 restructured** around the explicit composition `B10 ← B10_lean ∧ image-identity-binding ∧ B8` (theme 10, plus a v0.3 ask on Quartz's `Ecies.roundtrip` insufficiency for adversarial ciphertexts per theme 9 / claude S7). The Stage-1 / Stage-2 / `B10_lean` / image-identity-binding / B8 / B10-composition decomposition is now 6 numbered steps with the cross-layer composition explicit and `B10_lean` distinguished from B10.
  - **Not applied this pass** (deferred, in synthesis punch list as items #7-#17 / editorial): single-voice findings (Section 4.9 determinism requirement, Block 6 winners=last-round well-formedness, B8(c) 32-byte-digest-in-64-byte-slot, Stage-1 AEAD strengthening, IRV dead-code edge case, Section 1 framing softening, notation cleanup). All concrete but lower-impact; held for a follow-up sanity-pass round or absorbed into Quint translation if they surface there.
  - **False positives identified in synthesis and explicitly not addressed**: qwen's Section-2.1 arithmetic claim (kimi + gemma confirm the spec is correct), mistral's B10 arity claim (mis-read of which side of the Stage-1 decryption boundary `ballots@end_at` sits on), gpt-oss's Block 6 self-contradiction claim (conflated Block 5's idempotent self-loop with Block 6's transition rejection).
- **2026-05-16 (revision pass against Round 3a 3rd adversarial pass — subagent dispatch + claude subagent, 5-voice ensemble verdict BREAKS-AGAIN; synthesis at `.colosseum/attacks/pattern-b-v0.3.0-2026-05-16T125913Z/synthesis.md`)** — applied the 23-item v0.3.1 punch list (4 critical + 12 serious + 5 cosmetic + 2 false-positive-acknowledgments). Net changes (this revision pass is **MINOR** under the v0.3.1 SemVer rubric: 4 new failure modes added, 3 new scenarios added, several new defined symbols/predicates; no invariant weakened):
  - **§2.5 canonical serialization** — added explicit Borsh leaf-encoding pins for `Addr` (Bech32 string ⇒ Borsh u32-length-prefixed UTF-8 bytes) and `Nat` (Borsh `u64` little-endian, with the implicit invariant `0 ≤ tally_count < 2^64`); pinned candidate-declaration-order iteration discipline for Stage 1 (per T5 / 4-voice critical: `Map<Addr, _>` lexicographic-by-key iteration ≠ candidate-declaration order, making `Tally_spec` non-deterministic in v0.3.0).
  - **§2.5 transaction trace model** — added absent-key semantics for `state.ballots[k]` (lifted to `Option<Vec<u8>>`); replaced "tx" with "contract message"; defined `fires_at_transition(σ → σ′)` over CosmWasm message-set semantics rather than chain transactions (per T6 / 4-voice).
  - **§2.5 `EnclaveImage` typing** — added explicit 3-ary type signature `RawBallots × CandidateSet × PrivKey → TallyResult` with reserved-symbol discipline (the Lean-extracted model only; not an arbitrary `Bytes → Bytes` function) per T1 / 5-voice critical.
  - **§2.5 `ballots@end_at`** — defined notation explicitly (per F1 — single-voice but cited in B10).
  - **§3.1 preamble** — amended to admit handler-set inspection as a non-trajectory form of static quantification (per T4 / 5-voice serious).
  - **§3.1 S5** — restated as a handler-set property (not state-shape), with the trajectory form B1 preserved as the temporal complement (T4).
  - **§3.2 preamble** — tag taxonomy enumerated explicitly: `state`, `temporal`, `cross-layer`, `off-chain`, `meta-security` (per T17 / 2-voice cosmetic).
  - **§3.2 B6** — restated to `∀-per-key + ∃!-unique-msg-in-firing-transition` form (per T6 / 4-voice serious): the prior `∃` form admitted non-causal attribution.
  - **§3.2 B8** — clause (c) restated as lower-32-byte equality with domain-separation tag in the upper 32 bytes (per F7 / single-voice critical, kimi); clause (d) restated as on-chain registry tuple component-wise equality (per T10 / 3-voice serious).
  - **§3.2 B9** — retagged from `temporal` to `meta-security`; explicit `∀ PPT 𝒜, ∀ security parameter n` quantifier structure; `Adv_circuit_eq` promoted from probability summand to named correctness precondition `circuit_equivalence_honest`; summand count changed 4 → 3 in v0.3.1 not by dropping content but by promoting (per T9 / 3-voice).
  - **§3.2 B10** — added `enclave_input_fidelity(σ, σ′)` as a new conjunct in the existential body; B10's discharge structure updated to 5-link composition (per T3 / 4-voice critical).
  - **§4 preamble** — severity-tag taxonomy enumerated (base C/I/L + named refinements `integrity-per-voter`, `canonicality`, `recoverable`) per T16 / 3-voice cosmetic.
  - **§4 preamble recoverability** — amended to name §4.6 as the only auto-recoverable failure mode (per F5 / single-voice critical).
  - **§4.8** — content moved to §5 "No coordination / collusion resistance" bullet; slot marked RESERVED (per kimi failure-modes attack 4).
  - **§4.9** — added enclave retry policy specification (exponential backoff, initial 30s, max 1h) so the on-chain detection signature is well-defined (per T15 / 2-voice serious).
  - **§4.10, §4.11, §4.12, §4.13 added** — KMS unavailability at instantiation, enclave resource exhaustion during tabulation, ZK module algorithmic compromise, block-time non-monotonicity (per T11 / 3-voice serious).
  - **§6.1** — registry slot typed as `(vkey: Bytes32, mrtd: Bytes32, rtmr: Bytes32)`; formal predicate `image_registration_honest(σ)` defined as component-wise equality with `canonical_verified_rcv_*` values (per T2 / 5-voice critical; folds T9 quantifier fix).
  - **§6.2 status block** — downgraded banner from `[2026-05-15 — de-retracted]` to `[2026-05-16 — partially de-retracted]`; disclosed Quartz Ask-6 bundle-status open issue; restated `commitHashE` non-consumption with three independent reasons (pigeonhole-impossible sub-tag, Fintype carrier mismatch, structural UserData-slot-interface-only consumption) per T8 / 3-voice serious.
  - **§6.4** — caller-contract phrasing refined: "trusted to declare candidates honestly" replaced with "no on-chain check; off-chain responsibility, off-chain inspectability via public participation pattern" (per F4 / disputed-severity).
  - **§6.5** — added AEAD / authenticated-ciphertext precondition note: ECIES-as-typically-specified provides passive-eavesdropper confidentiality + honest-roundtrip correctness but not encryptor-identity binding; the Stage-1 soundness obligation is qualified accordingly (per T14 / 2-voice serious).
  - **§6.6** — output-contract guarantees conditioned on `image_registration_honest(σ)`; added explicit "well-formedness ≠ semantic correctness" gap; added bullet for "cannot infer semantic correctness from chain state alone — requires off-chain B10 discharge trust" (per T13 / 2-voice serious).
  - **§8.1** — `5-summand` reference corrected to `3-summand` after the v0.3.1 B9 retag (per F6 / single-voice critical).
  - **§8.4** — extended with post-`end_at` rejection scenario to witness B2 + B3 (not just B4); renamed accordingly (per T12 / 2-voice serious).
  - **§8.6** — restructured to provide both positive-existence and negative-rejection witnesses for the revised B6; renamed accordingly (per T12).
  - **§8.7** — composition pattern updated to 5-link factorization: `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack_kms_trust ∧ enclave_input_fidelity`; `EnclaveImage` typing reference added; explicit deferred decision between (i) extended attestation `report_data` schema vs (ii) chain-inclusion proof obligation for the input-fidelity discharge path (per T3 / 4-voice critical + T1 / 5-voice critical).
  - **§1** — system-identity wording refined to call out three-component scope explicitly rather than just "smart contract" (per F3 / single-voice cosmetic).
  - **SemVer rubric (header §)** — MAJOR rubric extended to admit "rewriting a previously-vacuous invariant" so the v0.3.0 → v0.3.1 classification can be MINOR even though the v0.2.0 → v0.3.0 step rewrote vacuous B9 (per F2 / single-voice cosmetic-from-claude).
  - **Not applied this pass** — none; all 23 punch-list items addressed (the only "deferred" item is the input-fidelity discharge path choice in §8.7 step 7, which is explicitly tracked as a Lean-phase decision rather than an intent-doc commitment).
  - **False positives explicitly noted in synthesis Section B**: F1 (`ballots@end_at` undefined — addressed by adding the definition, not by accepting the attack), F2 (SemVer mis-classification — addressed by extending the rubric), F3 (identity-wording — addressed via single-voice cosmetic), F4 (caller-contract phrasing — addressed despite disputed severity), F5 (§4 preamble vs §4.6 — addressed despite single-voice), F6 (5-summand → 3-summand stale reference — addressed despite single-voice), F7 (B8(c) byte-equality typing — addressed despite single-voice). The disposition rule: single-voice findings that pass the ground-truth-check were addressed; only findings that the synthesis ground-truth-refuted (none in this pass) would have been dropped.
  - **Methodology disagreements surfaced in synthesis Section C** (not action items, but recorded for v0.4 methodology back-port): D1 severity calibration on S5 (mistral critical vs consensus serious), D2 caller-contract phrasing severity (kimi cosmetic vs gemma serious), D3 scope of "structured behavior block" attacks vs "type signatures" attacks, D4 gemma flagged S10 invariant under timestamp re-orgs separately.
