# verified-rcv implementation roadmap

End goal: a CosmWasm contract + dstack TDX enclave for ranked-choice voting whose correctness is **mechanically verified end-to-end**. Intent → spec → code → proofs, with the integration ledger tracking what's proven, what's axiomatized, and what stands on faith.

## State at Round 3a close (2026-05-20 — 2026-05-24)

- ✅ Intent v0.3.4 — versioned source of truth (8 revisions, multi-voice adversarial trail)
- ✅ Quint protocol spec — typechecks; `all_invariants` model-checked on sampling (100 traces × 30 steps)
- ✅ Lean math spec — typechecks; S6/S7/S8/S9 PROVEN against 4 Stage-1/Stage-2 axioms + irv_ballots_tallied; B10_lean still `sorry`
- ✅ Integration ledger — 9 axioms inventoried (5 bucket-b demotable, 4 bucket-c carrier/named-constant)
- ❌ Rust contract
- ❌ Rust enclave
- ❌ Aeneas extraction
- ❌ B10_lean discharge
- ❌ Contract refinement proofs
- ❌ Image-identity-binding artifact

## Round-by-round plan

### Round 3c — Rust contract crate

**Deliverable**: `verified-rcv/crates/contract/` CosmWasm crate that implements the Quint protocol model.

Crate structure:
```
crates/contract/
  Cargo.toml
  src/
    lib.rs        # entry points
    contract.rs   # execute/instantiate/query handlers
    state.rs      # CONFIG, ELECTION, BALLOTS storage items
    msg.rs        # InstantiateMsg, ExecuteMsg, QueryMsg
    error.rs      # ContractError enum
    verification.rs  # Kani harnesses + Verus annotations
```

Derived from `specs/rcv.qnt`:
- `instantiate(InstantiateMsg)` → Block 1 (creates Election; admin = msg.sender)
- `create_election(title, candidates)` → admin-only; resets to Setup phase
- `open_voting()` → admin-only; Setup → Voting; sets voting_end
- `submit_ballot(ciphertext)` → permissioned (sender ∈ candidates); ballot store frozen at end_at (B2)
- `close_and_tally()` → Voting → Tallying; idempotent; permissionless wakeup
- `publish_result(AttestedTallyMsg)` → enclave-attested; Tallying → Resolved; tally_result set once (B1)

Reuses Quartz attestation crate `quartz-contract-core` for the attestation envelope verification.

Integrates `image_registration_honest` per intent §6.1 — registry holds (mrtd, rtmr, vkey) tuple, checked at publish_result.

**Refinement obligations**:
- Kani harness for B1 (tally_result monotone-once-set)
- Kani harness for S4 (ballot keys ⊆ candidates)
- Kani harness for S10 (resolution-after-end_at)
- Verus annotations for B2 (no-late-ballots — snapshot invariant)
- Verus annotations for B6 (∀-per-key + ∃!-unique-msg)

### Round 3d — Rust enclave crate + Aeneas extraction

**Deliverable**: `verified-rcv/crates/enclave/` Rust crate implementing `Tally_spec`.

Crate structure:
```
crates/enclave/
  Cargo.toml
  src/
    lib.rs
    decrypt_validate.rs   # Stage 1: decrypt ECIES, validate permutation
    irv.rs                # Stage 2: Australian Federal IRV (batch elimination on tied lowest)
    tally.rs              # Composition: decrypt_and_validate + IRV_spec → TallyResult
    attest.rs             # Quartz attestation envelope + dstack TDX integration
```

Algorithm per intent §2.5:
- Stage 1 iterates candidate-declaration order (NOT lexicographic) per v0.3.1 T5
- Stage 1 validates ballots as Vec<Addr> Borsh, permutation-of-candidates
- Stage 2 IRV: batch eliminate tied-lowest; ties → multi-winner; full preferential
- ballots_tallied = |valid_ballots| (per A5)
- non_voters = candidates \ raw_ballots.keys (per A4)

**Aeneas extraction**:
- Use `mcp__aeneas__extract_rust_to_lean` on `crates/enclave/`
- Output Lean term replaces `axiom EnclaveImage` in `specs/RcvSpec.lean`
- The 5 (b)-bucket axioms become **provable theorems** about the extracted term:
  - `irv_ballots_tallied` ← derivable from concrete `IRV_spec` definition
  - `decrypt_partition_length` ← derivable from concrete `decrypt_and_validate`
  - `irv_winners_shape` ← derivable from IRV termination + invariant
  - `irv_round_counts_sum` ← derivable from IRV conservation
  - `irv_no_reappearance` ← derivable from IRV elimination structure

### Round 3e — B10_lean discharge

**Deliverable**: `theorem B10_lean ... := by <real proof>` in `specs/RcvSpec.lean`.

The proof:
- Statement: `∀ raw cs pk, EnclaveImage raw cs pk = Tally_spec raw cs pk`
- After Aeneas extraction, `EnclaveImage` is the concrete extracted term
- `Tally_spec` is the existing composition def
- Goal reduces to showing the extracted Rust IRV equals the mathematical IRV spec

Strategy:
1. Use Claude (Agent tool) as primary discharge voice per Ask W
2. Iterate with `lake env lean` for verification (NOT `lean-lsp_lean_run_code` per Ask V)
3. Likely requires intermediate lemmas about ballot ordering, candidate iteration, recursion termination
4. Leanstral as second-opinion / single-tactic-step proposals

Once discharged: the 5 (b)-bucket axioms are also derived, leaving 4 axioms in the ledger (EnclaveImage as opaque/extracted is reduced to its extracted body; the 3 intent-level operational assumptions remain).

### Round 3f — Composition assembly

**Deliverable**: updated `.colosseum/ledger.md` declaring full B10 chain proven.

B10 = B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack_kms_trust ∧ enclave_input_fidelity

Link status after Round 3c-3e:
- B10_lean: PROVEN (3e)
- image-identity-binding: OPERATIONAL (build-pipeline ledger entry binds (mrtd, rtmr) to enclave crate hash; not a Lean proof)
- B8: clauses (a)+(b) inherited from Quartz substrate `cross_component_session_bind_negl` (def-tied per Quartz cycle-6.4-through-6.11); clause (c) is `Adv_commitTally_CR` axiom
- dstack_kms_trust: AXIOMATIC OPERATIONAL (intent §6.3)
- enclave_input_fidelity: attestation-envelope-witnessed (input-hash field per intent §8.7 step 5b)

End-state ledger trust density:
- (a) demotable-to-def-or-dead: 0
- (b) demotable-to-derived-theorem: 0 (all demoted in Round 3d/3e)
- (c) honest-computational-assumption: 4 (Adv_commitTally_CR + 3 intent-level operational)
- (d) impossibility-or-over-strength: 0

That's the "formally verified" shape — irreducible cryptographic + operational trust only.

## Methodology rounds in flight or queued

| Round | Status | Notes |
|---|---|---|
| 3a (spec layer) | DONE 2026-05-20 | Intent v0.3.4, Quint, Lean, ledger |
| 3b (Bidboard) | DEFERRED in this push | Multi-component dogfood; methodology pattern validation. The user gave free reign on verified-rcv specifically, so 3b can wait. |
| 3c (contract) | IN PROGRESS 2026-05-24 | This roadmap initiated the push |
| 3d (enclave + extraction) | IN PROGRESS 2026-05-24 | Parallel with 3c |
| 3e (B10_lean) | BLOCKED on 3d completion | Needs Aeneas extraction first |
| 3f (composition) | BLOCKED on 3c, 3d, 3e | Final assembly |

## Operating principles for this push

1. **Spec is source of truth**. The Rust code is derived from Quint protocol model + intent §2.5 Tally_spec definition. NOT derived from the Quartz examples/ranked-choice/ implementation (which is a different IRV variant per the self-comparator finding).

2. **Aeneas-extractability is a design constraint**. The enclave Rust must be written in a style charon + aeneas can extract. Standard discipline: pure functions, no impl blocks with complex trait bounds, explicit ownership, avoid certain stdlib idioms.

3. **Verus + Kani complement each other**. Kani for bounded model-checking of state-only properties (fast, cheap). Verus for invariants requiring history quantification (B2, B6 — the temporal ones).

4. **One commit per coherent piece**. Easier to bisect if something breaks; ledger entry per axiom demotion makes the trust surface change inspectable per commit.

5. **No mocking shortcuts in the verification layer**. If dstack-rs isn't available, write a `MockDstack` trait that satisfies the same interface, but ensure the verification harnesses cover both real and mock paths.

6. **Honest blockers**. If something is genuinely undecidable in scope (e.g., Aeneas can't extract a particular pattern), document the blocker in the ledger and propose a workaround. Don't pretend the gap closes when it doesn't.
