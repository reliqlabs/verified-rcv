# verified-rcv

Methodology-disciplined implementation of instant-runoff voting (IRV) ranked-choice tabulation, targeting full formal verification end-to-end: intent → spec → code → proofs.

This repo started as the **Round 3a dogfood** of the [Colosseum methodology](https://github.com/reliqlabs/colosseum) (spec layer only). It is now through Round 3a+3c+3d+3e (math layer COMPLETE 2026-05-26): all four Stage-2 obligation theorems are proven against the concrete math `IRV_spec`, all five Kani pure-logic harnesses verified, the enclave runtime's Stage 1 `decrypt_and_validate` has a real ECIES body, and the application is runnable end-to-end (off-TDX). Remaining: the Aeneas-bridge proof (`B10_lean_irv`), the storage-going Kani harnesses, full dstack-TDX runtime integration, image-identity-binding, and Round 3f composition assembly.

## State at HEAD

### Specification layer (Round 3a — done)

- **Intent v0.3.7** at `.colosseum/intent.md`. 11 revision cycles. 8 encoding-discipline notes (A2/A3/A4/A5/A6/A7/A8) propagated from cross-critique + concretization + Quartz-audit findings back into the intent doc. A7 (cs.length ≥ 1) and A8 (ballot-cover hypothesis) added during the Round 3e concreteness pass — both were soundness gaps the `opaque` abstraction had hidden. v0.3.7 (Quartz audit) clarifies the groth16 doubled-negligibility framing + adds caveats (IsPPT partial-closure, Ecies.roundtrip spec-layer abstraction).
- **Quint protocol spec** at `specs/rcv.qnt` (537 lines). 6 named state invariants + composite + 5 reachability witnesses. `quint run` 0 violations across sampled traces. Apalache exhaustive verification hits a known limitation on the dynamic `0.to(CANDIDATES.length())` range.
- **Lean math spec** at `specs/RcvSpec.lean`. Tally_spec composition, **concrete `IRV_spec`** (Round 3e), 4 structural theorems (S6-S9) + 4 Stage-2 obligation theorems + ~22 auxiliary lemmas + 2 axioms + 1 opaque function. **8 of 9 dischargeable theorems PROVEN** (S6, S7, S8, S9 + all four Stage-2 obligations: `irv_ballots_tallied` by `rfl`, `irv_winners_shape`/`irv_round_counts_sum`/`irv_no_reappearance` via structural induction on `irv_loop` fuel + ~24 lemmas). Only B10_lean (image-IO equality bridging to the Aeneas extraction) remains `sorry`. All discharged theorems axiom-clean (propext + Quot.sound; one uses Classical.choice via byContradiction).
- **Integration ledger** at `.colosseum/ledger.md`. 2 composition theorems (B10 cross-layer 5-link, B9 negligibility 3-summand), 9 trust-surface items. Round 3e moved 4 of 5 (b)-bucket items from `axiom` to fully-DISCHARGED Lean theorems. One axiom remains in the (b) bucket (`decrypt_partition_length`), demotable once the runtime crate is Aeneas-extracted.
- **Adversarial trail** at `.colosseum/specs/*`. Multi-voice fan-out + cross-critique + defense + re-cross-critique cycles on both Quint and Lean layers.

### Implementation layer (Round 3c+3d — in progress)

- **Rust workspace** at `Cargo.toml` with three crates:
  - `crates/contract/` — CosmWasm contract refining the Quint protocol. Handlers for `CreateElection`, `SubmitBallot`, `CloseAndTally`, `PublishResult`. Compiles to wasm. Chain-syntactic S6–S9 well-formedness checks at PublishResult. Quartz attestation envelope stubbed.
  - `crates/enclave-core/` — Aeneas-extractable Tally_spec core. Australian Federal IRV (full preferential, batch elimination on lowest tied, ties → multi-winner). 11/11 unit tests pass. Pure-function shape with Aeneas-compatibility constraints observed throughout.
  - `crates/enclave/` — Runtime wrapper. **Stage 1 `decrypt_and_validate` IMPLEMENTED 2026-05-26** with real ECIES + Borsh + permutation validation (7 unit tests pass). `tally_spec` composition (Stage 1 + Stage 2 → TallyResult) implemented (1 end-to-end test passes). `src/main.rs` is a usable CLI (stdin JSON → tally JSON). Still queued: gRPC service, dstack TDX boot, Quartz attestation envelope construction.
- **Aeneas extraction** at `lean-extraction/Enclave-core.lean` and `specs/EnclaveExtracted.lean` (1440 lines). Full IRV core extracted to Lean as concrete `verified_rcv_enclave_core.irv_spec` (in `Aeneas.Std.Result` monad). Stage 1 `decrypt_and_validate` extracts to `fail panic` because the Rust body is `unimplemented!()` in this crate (real decryption lives in the runtime crate, queued for future extraction).
- **Lean bridge** at `specs/EnclaveBridge.lean`. Real lift functions implemented (`Slice.v` + `Vec.v` + `UScalar.val` conversions). Theorem statement for `B10_lean_irv` is concrete and `lake env lean`-inspectable; proof body is `sorry` (Round 3e obligation).
- **Lean toolchain** upgraded to Lean 4.30.0-rc2 + Mathlib4 v4.30.0-rc2 + Aeneas backend (main, 4.30.0-rc2 pinned).
- **Kani harnesses** at `crates/contract/src/verification.rs` (~700 lines, behind `--features verification`). **5 pure-logic harnesses verified successful**: `derive_phase` totality + partition; S6/S7/S8/S9 implication semantics on `check_tally_well_formed` (Ok ⇒ each clause; clause violation ⇒ Err). Three workarounds applied: expose pure helpers `pub(crate)` to bypass CosmWasm storage; refactor `check_tally_well_formed` off HashSet (macOS Security.framework path Kani 0.67 doesn't model); narrow per-invariant harness shape (concrete tally + one symbolic perturbation each, not a unified symbolic-input builder). 5 storage-going harnesses (B1, S4, S10, AlreadyVoted, AlreadyResolved) remain blocked — those need the Verus alternative or a separate refactor.

## Outstanding work (in dependency order)

1. **B10_lean_irv discharge** (Round 3e bridge). The central refinement obligation: `extracted irv_spec refines math IRV_spec`. Statement concrete in `specs/EnclaveBridge.lean` with a detailed architectural plan inline. Multi-day Aeneas refinement work: ~10 `@[step]` per-helper bridge lemmas + a main `irv_loop0` bridge + the top-level compose. Math layer is fully proven now, so the bridge has a concrete target.
2. **B10_lean_decrypt** discharge. Needs Aeneas extraction of the runtime crate (which now has the real ECIES `decrypt_and_validate` body). Currently a `True` placeholder axiom. Aeneas-extractability of `ecies`/`k256` is a known-hard piece — may require modeling those crates as opaque axioms in the extracted Lean.
3. **Demote `decrypt_partition_length`** (Round 3e cleanup). The last (b)-bucket axiom; demotes to derivable theorem once the runtime crate is Aeneas-extracted.
4. **Storage-going contract harnesses** (Round 3c task #54 follow-on). Pure-helper subset is DONE (5 Kani harnesses verified). Storage-going subset (B1 write-once, S4 ballot-key subset, S10 resolution-after-end-at, AlreadyVoted, AlreadyResolved) remains blocked on CosmWasm + serde_json + macOS Security.framework path. Path forward: Verus annotations (Verus is installed at v0.2026.05.05; toolchain 1.95.0).
5. **Full enclave runtime** (Round 3d second half). Stage 1 + tally_spec done. Still needed: gRPC service, dstack TDX boot integration, Quartz attestation envelope construction.
6. **Image-identity-binding artifact** (Round 3f). Build-pipeline ledger binding `(mrtd, rtmr)` to the enclave crate's reproducible build hash.
7. **Composition assembly** (Round 3f). Stitch B10 = B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack_kms_trust ∧ enclave_input_fidelity in the integration ledger.

## What "fully verified" means here

In dependency order, the trust chain that has to close:

1. The Quint protocol model is model-checked sound (✓ on sampling; exhaustive blocked on Apalache compat).
2. The Rust contract refines the Quint protocol (✗ — Kani+CosmWasm blocker, Verus path queued).
3. The Rust enclave implements Tally_spec correctly (✓ — extracted; equality with math spec is B10_lean_irv, ✗ — proof pending).
4. The on-chain registered (mrtd, rtmr) match the enclave crate's reproducible build hash (✗ — image-identity-binding artifact pending).
5. The attestation predicate B8 holds for any tally posted on-chain (Quint shadow on sampling; Lean side inherits from Quartz substrate).
6. B9 negligibility budget bounds the probabilistic failure of B8 (inherited from Quartz, def-tied per cycle-6.4-through-6.11).
7. `dstack_kms_trust` axiom: the per-election privkey came from dstack KMS for the registered image (operational, axiomatic).

End-state trust density (target):
- (a) demotable-to-def-or-dead: 0
- (b) demotable-to-derived-theorem: 0
- (c) honest-computational-assumption: 4 (Adv_commitTally_CR + 3 intent-level operational)
- (d) impossibility-or-over-strength: 0

Current trust density: 0 / 5 / 4 / 0 — but the (b) bucket has shifted dramatically after Round 3e: **4 of 5 items now fully-DISCHARGED Lean theorems** (irv_ballots_tallied + irv_winners_shape + irv_round_counts_sum + irv_no_reappearance). Only `decrypt_partition_length` remains as an axiom, pending runtime crate extraction.

## Layout

```
verified-rcv/
  Cargo.toml                                   # workspace root
  Cargo.lock
  .colosseum/
    intent.md                                  # source of truth (v0.3.7)
    ledger.md                                  # integration ledger
    roadmap.md                                 # six-round implementation plan
    self-comparator-pre-round-3a-2026-05-20.md
    attacks/                                   # intent adversarial reports
    specs/                                     # multi-voice fan-out + cross-critique outputs
    scripts/                                   # dispatch scripts (fan-out, cross-critique, re-cross-critique)
  specs/
    lakefile.lean                              # Lake project (Mathlib + Aeneas)
    lean-toolchain                             # leanprover/lean4:v4.30.0-rc2
    lake-manifest.json
    rcv.qnt                                    # Quint canonical (protocol layer)
    main.qnt                                   # Quint entry point
    RcvSpec.lean                               # Lean math canonical
    EnclaveExtracted.lean                      # Aeneas extraction (copy of lean-extraction/)
    EnclaveBridge.lean                         # Refinement bridge for B10_lean_irv
  crates/
    contract/                                  # CosmWasm contract (compiles to wasm)
    enclave-core/                              # Aeneas-extractable Tally_spec core (11 tests pass)
    enclave/                                   # Runtime wrapper — Stage 1 + tally_spec + CLI (8 tests pass)
  lean-extraction/
    Enclave-core.lean                          # Aeneas extraction output
    enclave-core.llbc                          # charon LLBC IR
```

## Tooling stack

| Layer | Tool | Status |
|---|---|---|
| Intent | Colosseum methodology | ✓ at v0.3.7 |
| Protocol spec | Quint (sampling) | ✓ — 6 invariants + composite + 5 witnesses |
| Protocol spec (exhaustive) | Quint via Apalache | ✗ blocked on dynamic-range encoding |
| Math spec | Lean 4.30.0-rc2 + Mathlib | ✓ — **8 theorems proven** (S6-S9 + all 4 Stage-2 obligations) |
| Math spec (Stage 2 obligations) | Lean concrete IRV_spec | ✓ ALL 4 obligations discharged |
| Rust extraction | charon + aeneas | ✓ — extraction succeeds |
| Refinement bridge | Lean | ✓ statement + lifts + architectural plan; ✗ B10_lean_irv proof body |
| Rust contract | CosmWasm | ✓ compiles to wasm |
| Rust enclave core | Aeneas-extractable | ✓ 11/11 tests |
| Rust enclave runtime (Stage 1) | ECIES + Borsh + validation | ✓ 8/8 tests, real implementation |
| Rust enclave runtime (Stage 2 compose) | tally_spec end-to-end | ✓ implemented + tested |
| Rust enclave runtime (TDX + gRPC + envelope) | dstack TDX integration | ✗ stub |
| Contract refinement (pure helpers) | Kani | ✓ 5 harnesses verified (derive_phase + S6/S7/S8/S9 implication) |
| Contract refinement (storage handlers) | Kani | ✗ blocked on CosmWasm + serde_json |
| Contract refinement (storage handlers, alt) | Verus | not yet attempted (Verus 0.2026.05.05 installed) |
| Cryptography | VCV-io | not yet wired (no v4.30 release) |
| SNARK / IOR | ArkLib | not yet relevant |

## Related repos

- `/Users/mvid/Development/reliq/colosseum` — the methodology under validation
- `/Users/mvid/Development/reliq/quartz` — the Quartz substrate (attestation framework + dstack TDX + zkdcap)
