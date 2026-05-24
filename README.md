# verified-rcv

Methodology-disciplined implementation of instant-runoff voting (IRV) ranked-choice tabulation, targeting full formal verification end-to-end: intent → spec → code → proofs.

This repo started as the **Round 3a dogfood** of the [Colosseum methodology](https://github.com/reliqlabs/colosseum) (spec layer only). It is now in Round 3c+3d (implementation + Aeneas extraction) with Round 3e (B10_lean discharge) in flight.

## State at HEAD

### Specification layer (Round 3a — done)

- **Intent v0.3.4** at `.colosseum/intent.md`. 8 revision cycles. 5 encoding-discipline notes (A2/A3/A4/A5/A6) propagated from cross-critique findings back into the intent doc.
- **Quint protocol spec** at `specs/rcv.qnt` (537 lines). 6 named state invariants + composite + 5 reachability witnesses. `quint run` 0 violations across sampled traces. Apalache exhaustive verification hits a known limitation on the dynamic `0.to(CANDIDATES.length())` range.
- **Lean math spec** at `specs/RcvSpec.lean`. Tally_spec composition, 5 theorem statements + 6 axioms + 2 opaque functions. **4 of 5 theorems PROVEN** (S6, S7, S8, S9 against the 4 Stage-1/Stage-2 axioms + irv_ballots_tallied). B10_lean (image-IO equality) still `sorry`.
- **Integration ledger** at `.colosseum/ledger.md`. 2 composition theorems (B10 cross-layer 5-link, B9 negligibility 3-summand), 9 axioms inventoried with 4-bucket trust-density taxonomy, dead-axiom scan 0 hits.
- **Adversarial trail** at `.colosseum/specs/*`. Multi-voice fan-out + cross-critique + defense + re-cross-critique cycles on both Quint and Lean layers.

### Implementation layer (Round 3c+3d — in progress)

- **Rust workspace** at `Cargo.toml` with three crates:
  - `crates/contract/` — CosmWasm contract refining the Quint protocol. Handlers for `CreateElection`, `SubmitBallot`, `CloseAndTally`, `PublishResult`. Compiles to wasm. Chain-syntactic S6–S9 well-formedness checks at PublishResult. Quartz attestation envelope stubbed.
  - `crates/enclave-core/` — Aeneas-extractable Tally_spec core. Australian Federal IRV (full preferential, batch elimination on lowest tied, ties → multi-winner). 11/11 unit tests pass. Pure-function shape with Aeneas-compatibility constraints observed throughout.
  - `crates/enclave/` — Runtime wrapper stub (gRPC + dstack TDX + ECIES integration queued).
- **Aeneas extraction** at `lean-extraction/Enclave-core.lean` and `specs/EnclaveExtracted.lean` (1440 lines). Full IRV core extracted to Lean as concrete `verified_rcv_enclave_core.irv_spec` (in `Aeneas.Std.Result` monad). Stage 1 `decrypt_and_validate` extracts to `fail panic` because the Rust body is `unimplemented!()` in this crate (real decryption lives in the runtime crate, queued for future extraction).
- **Lean bridge** at `specs/EnclaveBridge.lean`. Real lift functions implemented (`Slice.v` + `Vec.v` + `UScalar.val` conversions). Theorem statement for `B10_lean_irv` is concrete and `lake env lean`-inspectable; proof body is `sorry` (Round 3e obligation).
- **Lean toolchain** upgraded to Lean 4.30.0-rc2 + Mathlib4 v4.30.0-rc2 + Aeneas backend (main, 4.30.0-rc2 pinned).
- **Kani harnesses** scaffolding at `crates/contract/src/verification.rs` (372 lines, behind `--features verification`). Type-checks but **blocked**: `cargo kani` stalls on CosmWasm storage's `serde_json` layer (too heavy for Kani symbolic execution). Workarounds queued: pure-guard-logic refactor, Verus annotations, proptest.

## Outstanding work (in dependency order)

1. **B10_lean_irv discharge** (Round 3e). The central refinement obligation: `extracted irv_spec refines math IRV_spec`. Statement concrete in `specs/EnclaveBridge.lean`. Proof multi-week even with good tools; two subagent attempts in this session stalled on long iteration cycles. Two paths: (a) make math `IRV_spec` concrete and prove definitional equality, (b) refinement axioms + extracted satisfies same shape.
2. **B10_lean_decrypt** (Round 3d follow-on). Needs extraction of the runtime crate (which has ECIES decryption logic) via Aeneas. Currently a `True` placeholder axiom.
3. **Demote 5 (b)-bucket axioms** (Round 3e cleanup). `irv_ballots_tallied`, `irv_winners_shape`, `decrypt_partition_length`, `irv_round_counts_sum`, `irv_no_reappearance` all become derivable theorems once `B10_lean_irv` proves (or once the math `IRV_spec` is made concrete).
4. **Contract refinement harnesses** (Round 3c task #54). Blocked on Kani+CosmWasm storage issue. Path forward: Verus annotations (Verus handles complex Rust patterns better than Kani).
5. **Enclave runtime crate** (Round 3d second half). Real gRPC server, dstack TDX integration, ECIES decryption, Quartz attestation envelope construction.
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

Current trust density: 0 / 5 / 4 / 0 — the 5 (b)-bucket axioms are pending demotion when B10_lean_irv proves.

## Layout

```
verified-rcv/
  Cargo.toml                                   # workspace root
  Cargo.lock
  .colosseum/
    intent.md                                  # source of truth (v0.3.4)
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
    enclave/                                   # Runtime wrapper stub
  lean-extraction/
    Enclave-core.lean                          # Aeneas extraction output
    enclave-core.llbc                          # charon LLBC IR
```

## Tooling stack

| Layer | Tool | Status |
|---|---|---|
| Intent | Colosseum methodology | ✓ at v0.3.4 |
| Protocol spec | Quint (sampling) | ✓ — 6 invariants + composite + 5 witnesses |
| Protocol spec (exhaustive) | Quint via Apalache | ✗ blocked on dynamic-range encoding |
| Math spec | Lean 4.30.0-rc2 + Mathlib | ✓ — 4 theorems proven |
| Rust extraction | charon + aeneas | ✓ — extraction succeeds |
| Refinement bridge | Lean | ✓ statement + lifts; ✗ proof |
| Rust contract | CosmWasm | ✓ compiles to wasm |
| Rust enclave core | Aeneas-extractable | ✓ 11/11 tests |
| Rust enclave runtime | dstack TDX + ECIES + gRPC | ✗ stub |
| Contract refinement | Kani | ✗ blocked on CosmWasm + serde_json |
| Contract refinement (alt) | Verus | not yet attempted |
| Cryptography | VCV-io | not yet wired (no v4.30 release) |
| SNARK / IOR | ArkLib | not yet relevant |

## Related repos

- `/Users/mvid/Development/reliq/colosseum` — the methodology under validation
- `/Users/mvid/Development/reliq/quartz` — the Quartz substrate (attestation framework + dstack TDX + zkdcap)
