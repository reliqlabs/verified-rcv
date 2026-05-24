# Colosseum integration ledger — verified-rcv

- Project: `/Users/mvid/Development/reliq/verified-rcv`
- Generated: 2026-05-24
- Intent version: v0.3.4 (`.colosseum/intent.md`)
- Compared against: ledger.md @ 2026-05-23 (prior commit)
- Scope this cycle: Round 3c+3d implementation push — Rust contract crate, Aeneas-extractable enclave-core, Aeneas extraction completed, Lean bridge module in place. Round 3e (B10_lean discharge) and Round 3c verification harnesses (Kani/Verus) in progress.

## Composition theorems

### 1. B10 — tally-correctness (cross-layer 5-link composition)

**Theorem:** B10 (intent §3.2, central methodology obligation)
**Located at:** `.colosseum/intent.md:400` (statement); `specs/rcv.qnt:507` (`inv_b10_chain_side_projection`, classical-Prop shadow); `specs/RcvSpec.lean:263` (`B10_lean`, off-chain half).
**Statement:** Whenever the chain transitions to a state where `tally_result.is_some()` and the prior state had `tally_result.is_none()`, there exists a `privkey` such that (i) `privkey` was dstack-KMS-derived for this contract, (ii) the enclave consumed `ballots@end_at` and `candidates` as its inputs (not host-substituted values), and (iii) `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))`. The tag is `cross-layer` — the existential ranges over off-chain values; the witness path crosses the Lean discharge layer.

**Depends on (5-link factorisation, intent §8.7):**

- `B10_lean` — Lean theorem `EnclaveImage(rb, cs, pk) = Tally_spec(rb, cs, pk)`. Located at `specs/RcvSpec.lean:263`. **[Lean / sorry]** — statement present, proof unproven. Discharge requires enclave Rust crate + Aeneas extraction.
- `image-identity-binding` — operational obligation: the on-chain registered `(mrtd, rtmr)` matches the build artifact of the Rust crate from which `EnclaveImage` was extracted. **[operational / axiomatic at this cycle]** — no Rust crate to extract from yet; the binding is intent-stated only.
- `B8` — attestation-binds-tally (intent §3.2). Quint classical-Prop shadow at `specs/rcv.qnt:496` (`inv_b8_publish_implies_registry_honest`); model-checked. Probabilistic content lives in B9.
- `dstack_kms_trust` — axiomatic operational assumption that the per-election privkey was released only to the matching attested enclave image. Intent §6.3 trust boundary.
- `enclave_input_fidelity` — operational claim that the input the enclave consumed equals `(ballots@end_at(σ), candidates(σ))`. Discharged operationally by an input-hash field in the attestation envelope (intent §8.7 step 5b).

**Per-conjunct failure-mode table:**

| Conjunct | Status | Source |
|----------|--------|--------|
| `B10_lean` | unconditional theorem (statement only; proof pending enclave extraction) | `specs/RcvSpec.lean:263` |
| `image-identity-binding` | operational-fault precondition (not probabilistic) | `.colosseum/intent.md` §8.7 step 4 |
| `B8` clauses (a)+(b) | probabilistic-failure mode (TDX-quote soundness + Groth16 KS) | Quartz-substrate inherited (`cross_component_session_bind_negl`) |
| `B8` clause (c) | probabilistic-failure mode (SHA-256 collision resistance) | `Adv_commitTally_CR` in B9 |
| `dstack_kms_trust` | operational-fault precondition (not probabilistic) | `.colosseum/intent.md` §6.3 |
| `enclave_input_fidelity` | operational claim, witnessed by attestation envelope | `.colosseum/intent.md` §8.7 step 5b |

**Trust boundary:** B10_lean (statement-only, proof pending); image-identity-binding (no enclave Rust yet); dstack_kms_trust (axiomatic); enclave_input_fidelity (attestation-envelope-witnessed, not formalized this cycle). B8 sub-clauses derive their negligibility from B9.

**Bundle cardinality:** 2 — `B8` clauses (a)+(b) bundled as the Quartz-substrate `cross_component_session_bind_negl` lift (single probabilistic-failure mode in the def-tied form); `B8` clause (c) bundled separately as the verified-rcv-internal `Adv_commitTally_CR` summand.
**Bundle cardinality (prior ledger):** 2 (unchanged from 2026-05-20 emission).

### 2. B9 — B8 negligibility-budget decomposition

**Theorem:** B9 (intent §3.2, meta-security tag)
**Located at:** `.colosseum/intent.md:399`.
**Statement:** Conditioned on `image_registration_honest(σ) ∧ circuit_equivalence_honest`, the probability that a PPT adversary 𝒜 induces a violation of B8 clauses (a)+(b)+(c) at the resolution transition is bounded by the 3-summand sum `Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_commitTally_CR(n)`.

**Depends on:**

- Quartz substrate `cross_component_session_bind_negl` (def-tied form per Quartz cycle-6.4-through-6.11). Two of the three summands (TDX-quote soundness, Groth16 KS) are inherited from this lift. **[Quartz lean spec / proven up to def-tying]**
- `Adv_commitTally_CR` — SHA-256 collision-resistance bound on `Borsh(contract_addr ‖ tally_body)`. **[axiomatic — honest computational assumption]**
- Precondition `image_registration_honest(σ)` — formally defined in intent §6.1. Required to scope out failure mode 4.3b (vkey substitution, adversarial success ≈ 1).
- Precondition `circuit_equivalence_honest` — the reference DCAP gnark circuit correctly encodes the TDX-quote-validity relation.

**Per-conjunct failure-mode table:**

| Conjunct | Status | Source |
|----------|--------|--------|
| `Adv_tdxVerifier_sound(n)` | probabilistic-failure mode | inherited from Quartz `tdxVerifier` lift |
| `Adv_groth16_KS(n)` | probabilistic-failure mode | inherited from Quartz `groth16Verifier` lift |
| `Adv_commitTally_CR(n)` | probabilistic-failure mode | SHA-256 collision-resistance, verified-rcv-internal |
| `image_registration_honest(σ)` | operational-fault precondition | intent §6.1 |
| `circuit_equivalence_honest` | correctness assumption (binary, not probabilistic) | intent §3.2 B9 antecedent |

**Trust boundary:** 3 probabilistic summands + 2 operational/correctness preconditions. The decomposition lineage (intent records this): 5 → 4 → 3 summands across iterations of methodology pressure; `Adv_circuit_eq` was correctly lifted out of the probabilistic sum as a correctness assumption in v0.3.1.

**Bundle cardinality:** 3 (each summand has its own probabilistic-failure mode and is not currently bundled).
**Bundle cardinality (prior ledger):** 3 (unchanged from 2026-05-20 emission).

## Trust-boundary axiom inventory

| # | Axiom | Kind | Bucket | Sub-tag | Used by | Justification |
|---|-------|------|--------|---------|---------|---------------|
| 1 | `EnclaveImage : RawBallots → CandidateSet → PrivKey → TallyResult` | Lean `axiom` | (c) honest-computational-assumption | `carrier` | B10_lean | Opaque type for an externally-supplied representation: the enclave binary's input-output relation modeled in Lean. Sourced via a documented extraction discipline (Aeneas Rust-to-Lean or hand-written). A proof author MAY NOT instantiate `EnclaveImage := Tally_spec` to discharge B10_lean by `rfl` — this is the tautological-shadow defect the canonical guards against. Located at `specs/RcvSpec.lean:261`. |
| 2 | `irv_ballots_tallied : ∀ valid cs, (IRV_spec valid cs).ballots_tallied = valid.length` | Lean `axiom` | (b) demotable-to-derived-theorem | `derived-from-spec-model` | S7, S8 | Restates intent §2.5's Stage 2 obligation `ballots_tallied := |valid_ballots|` that opaque `IRV_spec` cannot capture by itself. Demotable when `IRV_spec` is given a concrete definition (proof discharge or Aeneas extraction). Located at `specs/RcvSpec.lean:113`. |
| 3 | `decrypt_partition_length` | Lean `axiom` | (b) demotable-to-derived-theorem | `derived-from-spec-model` | S7 | Records the length-only consequence of Stage 1's partition `(valid, dropped, non_voters)` covering `cs` exactly once (assuming `cs.Nodup`). Restates intent §2.5's Stage 1 partition obligation. Demotable when `decrypt_and_validate` is given a concrete definition. Located at `specs/RcvSpec.lean:122`. Added 2026-05-23. |
| 4 | `irv_winners_shape` | Lean `axiom` | (b) demotable-to-derived-theorem | `derived-from-spec-model` | S6 | Records the three S6 claims about `IRV_spec`'s `winners`: subset of `cs`, non-empty, bounded by `|cs|` (under `cs.Nodup`). Restates intent §2.5/§3.1's Stage 2 winner-shape obligation. Located at `specs/RcvSpec.lean:136`. Added 2026-05-23. |
| 5 | `irv_round_counts_sum` | Lean `axiom` | (b) demotable-to-derived-theorem | `derived-from-spec-model` | S8 | Records that each round's per-candidate counts sum to `ballots_tallied` — the conservation property of IRV (eliminated candidates' ballots transfer rather than disappear). Restates intent §2.5/§3.1's Stage 2 round-conservation obligation. Located at `specs/RcvSpec.lean:148`. Added 2026-05-23. |
| 6 | `irv_no_reappearance` | Lean `axiom` | (b) demotable-to-derived-theorem | `derived-from-spec-model` | S9 | Records elimination monotonicity: a candidate eliminated at round `i` does not appear as a `RoundCount.candidate` entry in any later `per_round_counts[j]?` with `j > i`. Restates intent §2.5/§3.1's Stage 2 elimination-monotonicity obligation. Located at `specs/RcvSpec.lean:157`. Added 2026-05-23. |
| 7 | `dstack_kms_trust` (intent-level) | Intent operational axiom | (c) honest-computational-assumption | `named-constant` | B10 | Operational trust that the dstack KMS releases the per-election privkey only to the matching attested enclave image. Intent §6.3 trust boundary; not modeled as a Lean axiom yet. |
| 8 | `image_registration_honest(σ)` (intent-level) | Intent precondition | (c) honest-computational-assumption | `named-constant` | B9 antecedent | Operational claim that the on-chain registry's `(mrtd, rtmr, vkey)` tuple matches the canonical verified-rcv expected values. Intent §6.1 formal predicate. Not modeled as a Lean axiom yet. |
| 9 | `circuit_equivalence_honest` (intent-level) | Intent correctness assumption | (c) honest-computational-assumption | `circuit-equivalence` | B9 antecedent | The reference DCAP gnark circuit correctly encodes the TDX-quote-validity relation. Binary correctness property (not probabilistic). Intent §3.2 B9 antecedent. Discharge path: ArkLib-style circuit-equivalence reduction, or a hand-written Lean theorem about the circuit's encoding. |

### Dead-axiom scan

**0 hits**. Each of the 9 axioms above is referenced by at least one composition theorem or downstream Lean theorem. The 4 new Lean axioms (#3–#6) all participate in their corresponding S6–S9 discharge.

## Per-tool coverage snapshot

| Tool | Artifacts | Proven / Verified | Outstanding |
|------|-----------|-------------------|-------------|
| Lean math spec (`specs/RcvSpec.lean`, Lake project with Mathlib 4.30.0-rc2 + Aeneas) | 5 theorems + 6 axioms + 2 opaque functions | **4 theorems discharged** (S6, S7, S8, S9 — proven against the 4 Stage-1/Stage-2 axioms + irv_ballots_tallied) | 1 `sorry` (B10_lean — pending bridge proof to extracted enclave) |
| Lean extracted (`specs/EnclaveExtracted.lean` + `crates/enclave-core/`) | Aeneas extraction COMPLETE 2026-05-24 — 1440 lines extracted Lean (`tally_spec` + `irv_spec` + `decrypt_and_validate` stub + 8 helpers) | extracted module compiles under `lake build EnclaveExtracted` | extracted `decrypt_and_validate` is `fail panic` (Stage 1 runtime crate extraction queued for future round) |
| Lean bridge (`specs/EnclaveBridge.lean`) | B10_lean_irv refinement theorem statement + 3 lift function stubs | bridge module compiles; theorem statement is concrete | 1 `sorry` on B10_lean_irv proof + 3 lift stubs (in flight via subagent at the time of this commit) |
| Quint (`specs/rcv.qnt` + `specs/main.qnt`) | 6 named state invariants + 1 composite `all_invariants` + 5 witness invariants | 6 named invariants + composite hold across sampled traces; 5/5 witnesses violated (reachability witnesses) | model-checked at `--max-steps=30 --max-samples=100` only; not exhaustively proven (Apalache hit `NotInKeraError` on the dynamic `0.to(CANDIDATES.length())` range — re-encoding for Apalache compat is a tradeoff against parametric coverage) |
| Rust contract (`crates/contract/`) | `cargo check`, `cargo build --target wasm32-unknown-unknown --features library` clean. Handlers refine Quint actions: instantiate, CreateElection, SubmitBallot, CloseAndTally, PublishResult. Chain-syntactic S6-S9 well-formedness checks at PublishResult. | compiles cleanly to wasm | no Kani / Verus harnesses yet (Kani agent stalled on CosmWasm storage's serde_json layer; see Outstanding work) |
| Rust enclave-core (`crates/enclave-core/`) | IRV implementation per intent §2.5 (Australian Federal full preferential, batch elimination on lowest tied). 11 unit tests covering 6 scenarios. Aeneas-extractable shape (no nested-return-in-loops, no array-element-mutation, no closures-with-captures-in-loops). | 11/11 tests pass; Aeneas extraction succeeded after two refactor passes for Aeneas patterns. | none — clean state |
| Rust enclave runtime (`crates/enclave/`) | stub binary; gRPC + dstack-TDX + Quartz attestation integration queued for next round | compiles | full implementation pending |
| Kani | scaffolding at `crates/contract/src/verification.rs` (372 lines, gated behind `verification` feature, type-checks under `cargo build --features verification`) | 0 harnesses successfully verified | **BLOCKER**: subagent attempt 2026-05-24 stalled at 10min wall-time on CosmWasm storage's `serde_json` layer (too heavy for Kani's symbolic execution). Workarounds: (a) refactor handlers to expose pure guard logic separately and Kani those; (b) switch to Verus; (c) proptest the guard logic. |
| Verus | 0 | 0 | queued as alternative to Kani per (b) above |
| proptest | 0 | 0 | queued as alternative per (c) above |

## Trust density

| Bucket | Sub-tag distribution | Count | Delta vs prior |
|--------|----------------------|-------|----------------|
| (a) demotable-to-def-or-dead | — | 0 | unchanged |
| (b) demotable-to-derived-theorem | 5 `derived-from-spec-model` | 5 | **+4** (decrypt_partition_length, irv_winners_shape, irv_round_counts_sum, irv_no_reappearance) |
| (c) honest-computational-assumption | 1 `carrier`, 1 `circuit-equivalence`, 2 `named-constant` | 4 | unchanged |
| (d) impossibility-or-over-strength | — | 0 | unchanged |
| **Total** | | **9** | **+4** |

The (b) bucket grew by 4. All four new axioms are bucket (b) `derived-from-spec-model` — they restate intent §2.5's Stage 1 / Stage 2 obligations that opaque function declarations can't capture. **All four are demotable to derived theorems** once `decrypt_and_validate` and `IRV_spec` get concrete definitions via Aeneas extraction. The right shape after the methodology runs to completion is `(b) = 0` with the structural-invariant proofs deriving from the concrete IRV implementation.

The trust surface widened in number but stayed in the same risk class. There are no new bucket (c) `unforgeability` or `knowledge-soundness` axioms; nothing in bucket (d).

## Coverage delta vs. prior ledger (2026-05-23 emission)

**Lean spec layer** (`specs/`):
- Toolchain upgraded to Lean 4.30.0-rc2 + Mathlib4 v4.30.0-rc2 + Aeneas backend (main, 4.30.0-rc2 pinned). VCV-io temporarily dropped (no v4.30.0 release yet).
- The 4 discharged theorems (S6, S7, S8, S9) survived the toolchain bump unchanged.
- `specs/EnclaveExtracted.lean` added (1440 lines) — copy of the Aeneas-extracted IRV core, built by `lake build EnclaveExtracted`.
- `specs/EnclaveBridge.lean` added — refinement bridge module with `B10_lean_irv` theorem statement (`sorry`-d), three type-conversion lift stubs (`lift_valid_slice`, `lift_candidate_vec`, `lift_irv_result`).

**Rust workspace** (`crates/`):
- `Cargo.toml` workspace + 3 crates added: `contract/` (compiles to wasm), `enclave-core/` (11 unit tests pass, Aeneas-extractable), `enclave/` (stub).
- 4956 lines of Rust + extracted Lean across the implementation push.

**Aeneas extraction** (`lean-extraction/`):
- Successful extraction of `crates/enclave-core/` after two refactor passes:
  - `first_active_index` rewrite to avoid nested-return-in-loops (Aeneas limitation)
  - `tally_round` rewrite to avoid `arr[i] = arr[i] + 1` (Aeneas `expand_symbolic_value_no_branching` failure on indexed mutation)
- Output: `Enclave-core.lean` 1440 lines, builds cleanly.

**Trust surface**: no new axioms in this cycle (axiom count still 9). The 4 (b)-bucket axioms (`irv_winners_shape`, `decrypt_partition_length`, `irv_round_counts_sum`, `irv_no_reappearance`) are now **pending demotion**: the extracted IRV core provides the concrete `irv_spec` that these axioms describe, so they become derivable theorems once `B10_lean_irv` is discharged. Demotion will land in the next ledger when `B10_lean_irv` proves.

**Kani harness blocker**:
- `crates/contract/src/verification.rs` written (372 lines) with 5 Kani harnesses for state invariants (B1 state-shape, S4 ballot-keys, S10 resolution-after-end-at, AlreadyVoted, AlreadyResolved).
- Type-checks under `cargo build --features verification` but Kani symbolic execution stalls on CosmWasm's `serde_json` storage layer (10min wall-time, no progress).
- Three workaround paths queued: (a) pure-guard-logic refactor, (b) Verus annotations, (c) proptest.

**Methodology**: intent v0.3.4 (length-relation A6) reflected in spec references; no new intent revisions this cycle.

## Outstanding work

Ordered by criticality:

1. **B10_lean_irv discharge** (the Round 3e central obligation) — `specs/EnclaveBridge.lean` carries the statement with `sorry`. The proof is multi-week work even with good tools: requires lift functions from Aeneas's `Slice`/`Vec`/`Result` to math `List`/plain; induction matching the extracted recursion to the math algorithm; helper lemmas relating each extracted function to its math counterpart. A Claude subagent is attempting partial discharge at the time of this commit; full discharge likely needs a Lean-specialist proof model iterating.
2. **B10_lean (full, Stage 1 + Stage 2)** — composition of `B10_lean_irv` (above) + `B10_lean_decrypt`. The latter currently has only an axiom placeholder because the extracted Stage 1 returns `fail panic` (Rust `unimplemented!()` body in enclave-core; real decryption is in the runtime crate). Discharge requires extracting the runtime crate (which uses `ecies`, `k256`) — known-hard via Aeneas but tractable. Queued for a follow-on round.
3. **Demote 5 (b)-bucket axioms** — once `B10_lean_irv` proves, the 4 IRV-shape axioms (`irv_winners_shape`, `decrypt_partition_length`-but-wait-that's-decrypt, `irv_round_counts_sum`, `irv_no_reappearance`) + `irv_ballots_tallied` become derivable theorems about the concrete extracted `irv_spec`. Trust density (b) goes to 0 in the next ledger.
4. **Kani / Verus contract refinement harnesses (Round 3c task #54)** — BLOCKED on CosmWasm storage's `serde_json` layer being too heavy for Kani's symbolic execution. The verification.rs scaffolding (372 lines) is type-checked but no harness has been successfully Kani-verified. Path forward: refactor handlers to expose pure guard logic separately (Kani-testable), OR switch to Verus annotations (handles complex Rust patterns).
5. **Rust enclave runtime crate** — `crates/enclave/` is a stub. Real implementation needs gRPC server, dstack-TDX integration, Quartz attestation, ecies decryption. Round 3d second half.
6. **Image-identity-binding formalization** — currently an intent-level operational obligation. Becomes load-bearing once the enclave Rust crate has a stable build artifact whose `(mrtd, rtmr)` matches the on-chain registry.
7. **Quint model-checker coverage** — `quint run --max-samples=100` samples 100 traces. Exhaustive verification via `quint verify` (Apalache) hit `NotInKeraError` on the dynamic `0.to(CANDIDATES.length())` range from the bt_tallied fix. Re-encoding for Apalache compat is a tradeoff against parametric coverage; left as a methodology decision.
8. **Outstanding intent-level encoding-discipline candidate** (carried from prior ledger): the implicit length relation `per_round_counts.length = eliminated_by_round.length + 1` is documented in intent §2.5 (v0.3.4 A6 note) but not encoded as a separate well-formedness predicate. Optional — A6 is non-mandatory per the intent.

## Reviewer checklist

Before merging the current branch:

- [ ] Every new axiom has a justification line that survives independent review. **Currently: 9/9 axioms have justifications; all have discharge paths.**
- [ ] Every new `sorry` is accompanied by a follow-up issue, not silent. **Currently: 1 `sorry` marker (B10_lean), documented in Outstanding work item 1.**
- [ ] No composition theorem's dependency graph silently lost a node (compare to prior ledger). **Composition theorems B10 + B9 unchanged in structure; B10's `B10_lean` link is still pending discharge.**
- [ ] Coverage delta is in the expected direction (added coverage, not regressed). **4 theorem discharges added; 0 regressions.**

## Reading guide

This ledger says: **verified-rcv's verification layer at 2026-05-23 makes two composition claims (B10, B9), both well-stated and inspectable. The 4 structural well-formedness theorems (S6, S7, S8, S9) are now discharged against per-stage axioms that restate the intent's §2.5 obligations. B10's central Lean obligation (`B10_lean`) remains `sorry`; discharge is blocked on the enclave Rust crate (out of scope this cycle). B9 is fully composed against the Quartz substrate's def-tied form.**

A reviewer who reads this ledger should walk away knowing:

1. **What's proven** in Lean: 4 structural well-formedness theorems, each against an inspectable axiom.
2. **What's still axiomatic in Lean**: 6 axioms (5 demotable to derived theorems once `IRV_spec`/`decrypt_and_validate` get concrete bodies; 1 carrier axiom `EnclaveImage` that stays axiomatic until the Rust crate is extracted).
3. **What's still operational**: 3 intent-level assumptions about the deployment (dstack KMS trust, image-registration honesty, circuit-equivalence honesty).
4. **What's model-checked but not proven**: 6 Quint invariants + composite + 5 witnesses, sampled at 100 traces.
5. **What's pending**: B10_lean discharge (blocked on enclave Rust crate); the 5 (b)-bucket axioms demote once that crate exists; image-identity-binding formalization comes with the crate too.

The trust surface grew this cycle (5 → 9 axioms) but stayed in the same risk class — all 4 new axioms are bucket (b) and demotable. No new bucket (c) cryptographic assumptions; no bucket (d) over-strength claims.
