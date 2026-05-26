# Methodology retrospective: why Colosseum missed the audit findings

**Project:** verified-rcv (Round 3a dogfood)
**Audit window:** 2026-05-26 (4 audit rounds, intent v0.3.7 → v0.3.12, 7 commits)
**Retrospective scope:** evaluate why the Colosseum process (intent → spec → adversarial → verify → compose) did not surface the 22 findings closed across the 4 audit rounds. Drive concrete methodology asks for v0.4.

This document is **Phase 1** of the 5-phase retrospective plan (see top-level conversation). It produces the normalized finding inventory that Phase 2 (methodology-stage mapping) and Phase 3 (root-cause clustering) operate on.

## Inventory protocol

Each audit round produced findings labeled `C{n}` (Critical), `M{n}` (Major), `m{n}` (Minor), `i{n}` (Informational), and `N{n}` (re-audit findings, mixed severities). Below: every finding with severity Critical, Major, or Major-shadow (residual or escalated) is fully enumerated. Minor + Informational are summarized in aggregate tables further down.

**Audit rounds:**
- **R1** — `2026-05-26-18-36-audit-claude.md` against intent v0.3.7 / commit `3869b1e`. 3 Critical + 5 Major + 8 Minor + 10 Informational.
- **R2** — `2026-05-26-19-59-reaudit-claude.md` against intent v0.3.8 / commit `36fd01b` (after C1–C3 + M1–M5 fixed). 2 Critical-residual + 4 Minor + 1 Informational.
- **R3** — `2026-05-26-21-21-reaudit2-claude.md` against intent v0.3.9 / commit `db2021c` (after N1 fixed). 10 findings, all Minor / Informational / Pre-emptive.
- **R4** — `2026-05-26-21-35-reaudit3-claude.md` against intent v0.3.11 / commit `2ad894d` (after N2–N6 + B6 fixed). 1 Major + 3 Minor + 2 Informational.

**Categorization columns:**
- `id` — finding identifier
- `severity` — Critical / Major / Minor / Informational; (R) denotes residual (same root cause as a prior C-finding that the fix didn't fully close); (E) denotes escalated (Minor that the auditor flagged as escalating to Major under named conditions)
- `R#` — audit round
- `defect_type` — see legend below
- `surface` — where the bug lived (contract / runtime / cross-layer / spec / build-pipeline / CI / doc)
- `intent_stated_invariant` — did intent v0.3.x state the invariant the bug violated? (Y/N/partial)
- `quint_modeled` — did `specs/rcv.qnt` model the property? (Y/N/N-out-of-scope)
- `kani_covered` — did a Kani harness exercise the rejection path? (Y/N/N-no-harness)
- `closed_in_version` — intent version that closed it

### Defect-type legend

| code | description | example |
|---|---|---|
| `stub-as-Ok` | code accepts a placeholder instead of verifying; "deferred is panic" violated | C1 (Mock variant), N11 (real-zkdcap returns Err) |
| `missing-soundness-check` | shape checked, cryptographic verification skipped | C2 (envelope length only), N1 (B8 a/b/d) |
| `missing-input-validation` | admin-supplied data not validated | C3 (enclave_pubkey shape), M3 (registry shape) |
| `replay-narrow-binding` | hash commits to too few dimensions | M2 (no election_id), N4 (no chain_id), N22 (registration no addr/id), B6 (no ballots_hash) |
| `partial-state-mutation` | clears state without archiving / records | M1 (CreateElection clobbers), N3 (tally lost), N21 (Election lost) |
| `API-presentation-drift` | runtime surface field mismatched intent semantic | M5 (compose_hash in mrtd_hex slot) |
| `declaration-order-not-enforced` | ordering pin from intent not checked in code | M4 (per_round_counts not declaration-order) |
| `pending-state-race` | multi-tx admin sequence reaches unsafe state | N17 (finalize-mid-election DoS) |
| `chain-soundness-deferred` | chain accepts attestation without cryptographic verification of upstream layers | N1 (gnark verification deferred) |
| `pubkey-substitution` | trust boundary admits adversarial input that downstream layers can't catch | N2 (admin picks pubkey freely) |
| `CI-broken` | CI gate added but fails on clean build | N5/N18/N19/N20 |
| `doc-stale` | doc-comments don't reflect code reality | N6 |
| `defense-in-depth-only` | wouldn't break under normal use but degrades adversarial resistance | N12, N14, N15, N16 |

## Critical + Major + Major-shadow findings (the 11 high-severity entries)

| id | severity | R# | defect_type | surface | intent_stated_invariant | quint_modeled | kani_covered | closed_in |
|---|---|---|---|---|---|---|---|---|
| **C1** | Critical | R1 | `stub-as-Ok` | contract | Y (§3.2 B8: attestation MUST verify) | partial (Quint axiomatizes attested-eq-image) | N (no Kani harness for Mock-rejection) | v0.3.8 |
| **C2** | Critical | R1 | `missing-soundness-check` | contract | Y (§3.2 B8 clauses a/b/c/d) | partial | N | v0.3.8 (partial) → v0.3.9 (full) |
| **C3** | Critical | R1 | `missing-input-validation` + `pubkey-substitution` | contract | partial (§6.3 dstack_kms_trust assumes pubkey provenance, §6.1 silent on shape) | N | N | v0.3.8 (shape) → v0.3.9 (provenance B8(e)) → v0.3.12 (replay binding) |
| **M1** | Major | R1 | `partial-state-mutation` + `pending-state-race` | contract | Y (§3.2 B1: tally write-once per-election) | Y (Quint B1 invariant) | N (no harness asserts phase gate on CreateElection) | v0.3.8 |
| **M2** | Major | R1 | `replay-narrow-binding` | cross-layer | Y (§2.5 canonical_serialization specifies fields) | N (Quint doesn't model byte layout) | N | v0.3.8 |
| **M3** | Major | R1 | `missing-input-validation` | contract | partial (§6.1 names mrtd/rtmr as TDX measurements but no length pin) | N | N | v0.3.8 |
| **M4** | Major | R1 | `declaration-order-not-enforced` | contract | Y (§2.5 declaration-order discipline, T5) | Y (Quint encodes ordering) | N (no harness for per-round ordering) | v0.3.8 |
| **M5** | Major | R1 | `API-presentation-drift` | runtime | Y (§6.1 mrtd is TDX measurement) | N/A (runtime layer not in Quint) | N/A | v0.3.8 (returns empty) → v0.3.12 (real parser via dcap-qvl) |
| **N1** | Critical (R) | R2 | `chain-soundness-deferred` | contract | Y (§3.2 B8 a/b/d) | partial (B9 axiomatizes Adv summands) | N | v0.3.9 |
| **N2** | Critical (R) | R2 | `pubkey-substitution` | contract | partial (§6.3 mentions dstack_kms_trust, §8.7 link 6) | N | N | v0.3.9 (B8(e) gnark binding) → v0.3.10 (N2 timelock) → v0.3.12 (registration nonce via election_id) |
| **N17** | Major | R4 | `pending-state-race` | contract | N (intent did not enumerate the propose→create→finalize sequence) | N (Quint did not model multi-step admin lifecycle around registry rotation) | N | v0.3.12 |
| **N13/N22** | Minor (E to Major under leaked-privkey) | R3+R4 | `replay-narrow-binding` | contract + runtime | partial (§3.2 B8(e) named the binding but not its preimage shape) | N | N | v0.3.12 |
| **B6** | Major (implicit, §8.7 link 7 open) | self-identified | `replay-narrow-binding` | cross-layer | Y (§8.7 link 7 named as open) | N (intent flagged it as deferred) | N | v0.3.11 |

**Observation:** 13 entries qualify as Critical, Major, or Major-shadow. Of these:
- **8 of 13 were fully in the intent** (intent_stated_invariant = Y); **4 partial**; **1 unmodeled** (N17). The intent said the right thing in most cases; the code didn't honor it.
- **2 of 13 were modeled in Quint** (Y); 3 partial; 8 unmodeled. The protocol model focused on protocol-level invariants and didn't extend to byte-layout / multi-step admin lifecycle / cryptographic-soundness layers.
- **0 of 13 were Kani-covered** before the audit. Kani harnesses existed for some structural invariants (S6, S7, S8, S9, B1 state-shape) but not for any of the trust-boundary surfaces the auditor attacked.

## Minor + Informational findings (aggregated)

### R1 Minor (m1–m8)

| id | type | surface | summary | closed_in |
|---|---|---|---|---|
| m1 | missing-input-validation | contract | empty ciphertext accepted | not yet (acceptable for v0.3.x; tracked) |
| m2 | API-drift | contract | first-write-wins vs intent's last-write-wins | not yet (intent §2.3 vs code) |
| m3 | type-mismatch | contract | start_at >= block.time accepted vs intent > | not yet |
| m4 | cross-version-API | UI ↔ runtime | borsh-js ^2.0.0 vs borsh-rust 1.5.x | not yet (single behavioral test |
| m5 | CI-broken | build-pipeline | cosmwasm-check rejects raw cargo build | v0.3.12 (cosmwasm/optimizer docker) |
| m6 | missing-input-validation | contract | Addr not run through deps.api.addr_validate | not yet |
| m7 | replay-narrow-binding | contract | ECIES ciphertext lacks chain context | not yet (would need ballot-layer redesign) |
| m8 | panic-on-malformed | runtime | dstack::99 panic at startup | not yet (operational only) |

### R1 Informational (i1–i10)

Code-hygiene + spec-doc-comment items: u32 saturating_add lag, HashSet usage in candidate dedup, ECIES comment misnames AEAD as ChaCha20-Poly1305 (actual: AES-256-GCM), HKDF info empty, TallyResult Borsh derive absent, dropped_voters bucket conflates failure modes, `#![forbid(unsafe_code)]` absent, enclave-core `unimplemented!()` reachable, vkey field type drift, permissionless Ballots query (ECDLP-bound confidentiality).

### R2 (N3–N6)

| id | severity | type | summary | closed_in |
|---|---|---|---|---|
| N3 | Minor | partial-state-mutation | CreateElection clobbers TALLY_RESULT | v0.3.10 (archive map) |
| N4 | Minor | replay-narrow-binding | chain_id not in canonical_serialization | v0.3.10 |
| N5 | Minor | CI-broken | CI lacks clippy / cargo-audit / cosmwasm-check / mock-feature gate | v0.3.10 |
| N6 | Informational | doc-stale | state.rs doc-comments stale post-M1/M3 | v0.3.10 |

### R3 (N7–N16, 10 findings)

The R3 audit returned a long tail of mostly defense-in-depth items, with two notable categories:
- **CI broken** (N8 clippy fail; N9 clippy::needless_range_loop in extraction code).
- **Process drift** (N7: code added N4 changes while commit message said N4 was deferred — a *commit-message vs code* mismatch).
- **No real production path** (N11: real-zkdcap returns Err stub).
- **Registration replay** (N13: re-flagged as N22 in R4 after the R3 deferral justification was found incomplete).
- **Defense-in-depth** (N12, N14, N15, N16).

### R4 (N17–N22, 6 findings)

Already enumerated above. N17 Major. N18/N19/N20 CI broken. N21/N22 informational (N22 = N13 escalated).

## Summary statistics

| metric | value |
|---|---|
| Total findings closed | 22 (across 4 audit rounds) |
| Critical + Major + Major-shadow | 13 |
| Findings the intent already specified | 9/13 (69%) |
| Findings the Quint spec modeled | 2/13 (15%) |
| Findings any Kani harness exercised | 0/13 (0%) |
| Critical findings that took 2+ audit rounds to fully close | C2 (R1+R2), C3 (R1+R2+R4) |
| Findings re-flagged after deferral justification was found wrong | 1 (N13 → N22) |
| CI gates that broke when added | 3 of 4 (N18, N19, N20) |

## Patterns visible in Phase 1

Even before Phase 2's stage mapping, several patterns are visible in the inventory:

1. **The intent said the right thing.** 9 of 13 high-severity findings violated invariants the intent v0.3.x text explicitly stated. The Colosseum elicitation + adversarial review of the *intent* worked — the gap was downstream.

2. **Quint modeled the protocol, not the trust boundary.** Quint encoded B1 (write-once), declaration order, and the IRV state machine — but not byte-layout commitments, multi-step admin lifecycles, or cryptographic-soundness layers. The properties Quint *did* model were the ones the code mostly got right (M1 was the partial exception, and M1's Quint-modeled B1 invariant was correctly stated; the code drift was the bug, not the model).

3. **Kani harnesses had a coverage gap.** No high-severity finding was Kani-covered before audit. The harnesses targeted *structural state invariants* of the contract (S6–S9 well-formedness, B1 post-publish presence), not *the trust-boundary verification logic* (attestation checks, registry shape, pubkey validation). This is the single most concentrated diagnostic in the inventory.

4. **Replay-narrow-binding is the most-recurring pattern.** 4 distinct findings (M2 election_id, N4 chain_id, N13/N22 registration pubkey replay, B6 ballots_hash) all share the shape "this hash binds X but not Y, attacker varies Y." Plus m7 (ECIES chain context) at the Minor tier. A single "commitment-coverage" review lens would have caught all four at once — across two audit rounds, in two cases (M2 + N4) appearing in the *same* `canonical_serialization` function over a 2-week interval.

5. **Multi-round cascade.** C2 took R1+R2 to close. C3 took R1+R2+R4 (three rounds). The methodology has no concept of "the intent's claim about the cryptographic chain is a multi-clause obligation, and partial fixes leave attack surface." The auditor caught this; the methodology didn't structure for it.

6. **Lifecycle (multi-tx admin sequences) is unmodeled.** N17 (finalize-mid-election DoS) lived in the *gap between* the Quint protocol model (which has CreateElection, SubmitBallot, PublishResult transitions) and the v0.3.10 N2 timelock additions (which the Quint model was not updated to reflect). No methodology stage asked "what are the worst admin sequences across multiple blocks?"

7. **CI gates added at v0.3.10 broke on their own clean build (3 of 4).** The CI gates were aspirational — written without running them locally first. This is its own methodology gap: "added CI checks must be green at the commit that adds them."

8. **Commit-message and code can drift** (N7). The methodology has no enforced step "diff actual code change against commit-message claim."

## Phase 2 readiness

With Phase 1 complete, Phase 2 (methodology-stage mapping) operates on this inventory by adding two columns per finding:

- `stage_that_should_have_caught_it` (one of: intent-elicitation, intent-adversarial, Quint-spec, Quint-adversarial, Lean-spec, Aeneas-extraction, code-implementation, **code-adversarial** [the missing one], Kani-harness, Verus-harness, cross-layer-ledger, **lifecycle-adversary** [proposed], **CI-self-test** [proposed])
- `why_stage_missed_it` (diagnostic free-text or category from a closed list)

Phase 2's output is a (finding × stage) matrix from which Phase 3's root-cause clusters fall out mechanically.

## Open notes for Phase 2

A few entries in the inventory above need a Phase-2 judgment call I don't want to pre-decide:

- **C1 Mock variant.** Was this catchable at intent-adversarial? Probably yes — "what if a stub variant ships?" is a generic concern. But the more concentrated catch is at code-implementation review with the "deferred is panic" lens. Phase 2 should attribute it to the *cheapest* methodology stage that would have surfaced it.
- **M2 election_id.** Catchable at intent-adversarial under a "commitment-coverage" lens? Or only catchable at cross-layer-ledger review (because it requires understanding both contract and runtime hash computations simultaneously)?
- **N13 → N22 (registration replay).** R3 deferred this with incomplete justification. The deferral itself was a methodology event — Phase 2 should track "deferral justifications that turned out to be wrong" as a separate cell.

---

**End of Phase 1 deliverable.**

# Phase 2: methodology-stage mapping

For each of the 13 high-severity findings, two new columns: the *cheapest* Colosseum stage that would have surfaced the bug had it been present and correctly applied, and the diagnostic for why the stage as run did not catch it.

## Stage taxonomy

The Colosseum v0.2 pipeline as run on verified-rcv had the following stages (in pipeline order):

| stage | what it does | what it produced on this project |
|---|---|---|
| intent-elicitation | author writes intent.md, codifies invariants + trust assumptions | intent v0.1 → v0.3.7 (pre-audit), 9 trust-chain links in §8.7 |
| intent-adversarial | red-team reads intent, attacks under "what's the worst input that satisfies this?" | `.colosseum/specs/lean-cross-critique-2026-05-20`, `cross-critique-2026-05-19`, `synthesis-2026-05-19` |
| Quint-spec | encode protocol-level invariants as a TLA-style transition system | `.colosseum/specs/quint-fanout-2026-05-16T183215Z` → `specs/rcv.qnt` with B1, S6, S7, S8, S9, S10, IRV ordering |
| Quint-adversarial | red-team the Quint model under "what trace violates the invariant?" | (no separate Quint-adversarial artifact distinct from intent-adversarial cross-critique) |
| Lean-spec | reify selected invariants as Mathlib theorems with proofs | `IRV_spec.lean`, Stage-1 theorems closed, 3 Stage-2 sorries |
| Aeneas-extraction | extract Rust to Lean, prove code↔spec | `enclave-core` Aeneas-extractable; no code↔spec discharge on contract |
| code-implementation | author writes Rust, follows intent | `crates/contract` + `crates/enclave` + `crates/enclave-core` |
| code-adversarial **[absent]** | red-team reads the *implementation*, attacks under "does this code honor the intent?" | **never performed** as a Colosseum-disciplined stage |
| Kani-harness | bounded model-checking on the executable code | 10 harnesses, all on IRV result structure (B1, S4, S6–S10, voted/resolved enforcement, derive_phase) |
| Verus-harness | proof-carrying verification, currently blocked | not in v0.2 substrate |
| cross-layer-ledger | track invariants that span trust boundaries in `.colosseum/ledger.md` | 9 links in §8.7; ledger ≈ map of trust-edge obligations |
| **lifecycle-adversary [proposed]** | red-team multi-tx admin sequences | **proposed in this retrospective** |
| **CI-self-test [proposed]** | added CI gates must be green at the commit that adds them | **proposed in this retrospective** |

## Finding × stage matrix

For each high-severity finding, the cheapest catching stage + diagnostic.

| id | stage_that_should_have_caught_it | why_stage_missed_it |
|---|---|---|
| **C1** (Mock-attestation accepted) | code-adversarial [absent] | Intent §3.2 B8 stated "attestation MUST verify." The Mock variant was a documented placeholder in code. Intent-adversarial did not red-team the code; intent-adversarial only read the intent. No methodology stage took the (intent invariant, code path) pair and asked "does this path enforce that invariant?" The pattern is **"deferred is panic"**: any deferred-implementation branch should panic, not return Ok. Catchable cheapest at code-adversarial with a generic "deferred is panic" review lens. A weaker but workable catch sits at intent-adversarial: "what if a stub variant ships with default features?" — but that lens was not part of v0.2's intent-adversarial. |
| **C2** (envelope length-check only) | code-adversarial [absent] | Intent §3.2 B8 enumerated clauses a/b/c/d (proof verifies, measurement matches, ReportData matches, DST present). The code checked envelope shape, not the clauses. Catchable cheapest at code-adversarial with a checklist-driven review: "for each B8 clause, point at the line of code that enforces it." This is the **"clause-to-line mapping"** discipline. Cross-layer-ledger had the clauses; nothing forced a code↔ledger reconciliation. |
| **C3** (enclave_pubkey provenance) | intent-adversarial (partial) + code-adversarial [absent] | Intent §6.3 named `dstack_kms_trust` as an assumption but did not insist the chain layer verify it. Intent-adversarial *did* surface a related concern (see `.colosseum/specs/cross-critique-2026-05-19`) but accepted the trust assumption rather than escalating to "the chain must close this." The fix took 3 audit rounds: shape (v0.3.8), provenance (v0.3.9 B8(e)), replay binding (v0.3.12). The methodology missed the **"multi-clause trust-assumption decomposition"** — when an assumption has multiple necessary conditions, each must be discharged or named as still-open. |
| **M1** (CreateElection clobbers in-flight) | Quint-adversarial (partial) + code-adversarial [absent] | Quint modeled B1 (write-once per election). The code drift was: the CreateElection handler did not gate on the current phase. A *trace-level* Quint-adversarial would have produced a trace `[CreateElection, SubmitBallot, CreateElection]` — second CreateElection should be rejected per B1. Quint-spec did model B1; Quint-adversarial as run did not generate adversarial-trace exhaustively against the admin transitions. Cheapest catch: Quint-adversarial under the "what if admin acts at the wrong phase?" lens. |
| **M2** (canonical_serialization missing election_id) | intent-adversarial + cross-layer-ledger | Intent §2.5 specified the field set of `canonical_serialization` and gave each field a purpose. Intent-adversarial under a **"commitment-coverage"** lens — "for each external attacker degree of freedom, is it bound by the hash?" — would have caught: attacker controls election_id, but the hash doesn't bind election_id. Cross-layer-ledger could also catch it (the same hash appears in §8.7 link 5 from runtime side and §3.2 B8(c) from contract side, and the two sides must agree on coverage). Cheapest is intent-adversarial because it requires no cross-layer reasoning. |
| **M3** (registry-shape validation) | intent-elicitation + intent-adversarial | Intent §6.1 named mrtd/rtmr as TDX measurements without pinning the wire length. Intent-elicitation should have produced a length-typed entry; intent-adversarial under "what if a malformed registry is stored?" should have escalated. Cheapest is intent-elicitation: the field-spec discipline should require wire-length pins for every binary-data field. |
| **M4** (per_round_counts declaration-order) | Kani-harness | Intent §2.5 + T5 specified declaration-order discipline. Quint modeled the ordering. The bug was the contract handler used HashMap iteration order instead of declaration order at one site. **A Kani harness would catch this in seconds** by asserting that for a given Candidate vector, the per_round_counts output is canonicalized. The stage that should have caught it was Kani, and Kani had every prerequisite (Quint property, intent text, executable code) — but the harness was not written. Catalog gap: **the Kani harness catalog was built from "structural correctness of the result" not "ordering discipline."** |
| **M5** (mrtd_hex slot holds compose_hash) | code-adversarial [absent] | The runtime had `mrtd_hex` as a JSON field name but the value placed there was a Quartz compose_hash. Intent §6.1 + §8.7 link 5 named mrtd as a TDX register. Cheapest catch: code-adversarial reading the runtime against §6.1, asking "does the value at JSON field X carry the semantic the field name implies?" This is the **"API field-name fidelity"** lens. |
| **N1** (chain accepts attestation without gnark verify) | cross-layer-ledger + code-adversarial [absent] | Intent §3.2 B8(a)(b)(d) was clear. The contract checked the envelope shape and accepted. The ledger §8.7 link 5 said "chain verifies proof" — but no stage cross-checked the ledger claim against the actual contract code. The fix wired direct GrpcQuery to `xion.zk.v1.Query/ProofVerifyGnark`. Cheapest catch: cross-layer-ledger with a **"ledger-to-line mapping"** discipline (every link claim points to executable code that discharges it). |
| **N2** (admin picks enclave_pubkey freely) | intent-adversarial + Quint-adversarial | A trace `[admin_sets_registry(pubkey=adversary), submit_ballot, publish(tally=fake, attestation=valid_for_adversary_pubkey)]` is admissible if the registry's pubkey field admits any value. Intent-adversarial under the **"who controls each field of state?"** lens — every field in stored state needs an attacker-degree-of-freedom column — would catch this. The fix combined v0.3.9 B8(e) (gnark binds pubkey to attested measurement) + v0.3.10 timelock + v0.3.12 registration nonce. Cheapest catch: intent-adversarial with the "who controls" lens. |
| **N17** (finalize-mid-election DoS) | lifecycle-adversary [proposed] | The Quint model encoded protocol transitions (CreateElection, SubmitBallot, PublishResult) but was not extended to model v0.3.10's registry-rotation lifecycle (ProposeRegistryUpdate, FinalizeRegistryUpdate, CancelRegistryUpdate, with timelock). No methodology stage red-teamed multi-tx admin sequences across the registry-rotation feature. **lifecycle-adversary** is the proposed missing stage: when admin-controlled features ship, the Quint model and an adversarial review of multi-step admin sequences must follow within the same intent revision. |
| **N13/N22** (registration ReportData replay) | code-adversarial [absent] + **"deferral justification audit"** | R3 surfaced N13. The R3 deferral justification — "N2 timelock makes this acceptable" — was wrong: N2 timelock prevents *registry rotation* but does not prevent *registration-quote replay* (an attacker with a leaked pubkey can re-register the same key against any election). The methodology had no audit of deferral justifications; the same auditor caught the error in R4 by re-reading the same surface. Cheapest catch: code-adversarial would have caught the bug; **"deferral justification audit"** would have caught the wrong-deferral *event*. |
| **B6** (ballots_hash missing from canonical_serialization) | intent-adversarial (commitment-coverage) | §8.7 link 7 already flagged this as open. The intent named the gap; no methodology stage forced a "close all §8.7 open items before audit" gate. Catch is identical to M2: the **"commitment-coverage"** lens at intent-adversarial. |

## Stage-level diagnostic paragraphs

### intent-elicitation
**Applied to:** the full intent.md (v0.1 → v0.3.7 pre-audit). **Missed:** M3 (length-pin discipline) and contributed-partially to C3 (multi-clause assumption decomposition). **Why it missed them:** intent-elicitation as practiced on verified-rcv produced English prose with carefully named obligations but no *field-spec discipline*. Every binary-data field needs a wire-length pin in the intent; every named assumption needs a decomposition into necessary conditions, each marked discharged or open. The intent had a §8.7 trust-chain ledger which was the closest analog, but it was not enforced as a gating artifact.

### intent-adversarial
**Applied to:** intent v0.3.0–v0.3.7 via `lean-cross-critique-2026-05-20`, `cross-critique-2026-05-19`, `synthesis-2026-05-19`. **Missed:** C3 (partial), M2, M3, N2, B6. **Why it missed them:** the cross-critique fan-out attacked the *theorem statements* and the *cryptographic chain narrative*, but did not run a structured red-team under three named lenses that would have caught a majority of the high-severity findings:
- **Commitment-coverage** ("for each attacker degree of freedom, is it bound by every commitment that should bind it?") — would catch M2, B6, N4, m7.
- **Who-controls** ("for each field of stored state, who supplies it and what stops adversarial values?") — would catch N2, C3, M3.
- **Multi-clause-decomposition** ("for each named trust assumption, decompose into necessary conditions and discharge each") — would catch C3, partial C2.

These three lenses are mechanical and could ship as a v0.4 ask (intent-adversarial checklist).

### Quint-spec
**Applied to:** `specs/rcv.qnt` modeling B1, S6, S7, S8, S9, S10, IRV ordering. **Missed:** M1 (partial), M4 (partial). **Why it missed them:** the Quint model was correct on the protocol surface it modeled. The drift was in *which* properties were modeled — not byte-layout commitments (M2/N4/B6), not multi-step admin lifecycles (N17), not cryptographic-soundness layers (C1/C2/N1). Quint-spec was applied to the IRV state machine; it was not applied to the trust boundary.

### Quint-adversarial
**Applied to:** essentially nothing distinct from intent-adversarial. **Missed:** M1 (write-once-with-trace-of-admin-misuse), N2 (admin-controls-registry-trace). **Why it missed them:** Quint-adversarial was not run as a separate stage. The Quint model encoded the right invariants; no one generated counter-examples or adversarial traces against the admin transitions. A v0.4 ask: every Quint property gets an adversarial trace generation pass.

### Lean-spec
**Applied to:** `IRV_spec.lean`. **Missed:** none of the 13 directly (the high-severity findings were not at the Lean-spec layer). **Why nothing surfaces here:** Lean-spec on verified-rcv targets the IRV mathematical correctness, not the trust boundary. This is by design and correct. The gap is downstream (code↔spec discharge via Aeneas).

### Aeneas-extraction
**Applied to:** enclave-core (Aeneas-extractable; structural extraction done; B10_lean discharge pending). **Missed:** C1, C2, M1, M4 — all contract-side bugs that Aeneas-on-the-contract would have flagged. **Why it missed them:** Aeneas was applied to the IRV core, not the contract handlers. Extracting the contract handlers would require declaring all of cosmwasm-std's storage API as axioms in Lean, which is currently impractical. The methodology gap is **"the cheapest verification substrate for trust-boundary code is Kani, not Aeneas"** — but the Kani harness catalog (next stage) did not target trust-boundary code either.

### code-implementation
**Applied to:** the full Rust workspace. **Missed:** all 13 (since they were bugs *in* the code-implementation output). **Why it missed them:** code-implementation is the stage that *introduces* the bugs; what's interesting is the *absence* of a downstream code-adversarial stage to catch them before audit. Diagnostic deferred to code-adversarial below.

### code-adversarial **[absent]**
**Applied to:** never. This stage did not exist in Colosseum v0.2. **What it should have caught:** C1, C2, C3, M5, N1, N13/N22 — 6 of 13 high-severity findings (46%). **What it is:** a red-team that reads the *implementation* (not the intent) and, for each named intent invariant, points at the line of code that enforces it (or notes the gap). The audit rounds were a *de-facto* code-adversarial; the methodology gap is that code-adversarial was external (auditor) rather than internal (Colosseum stage). **v0.4 ask AB:** code-adversarial as a first-class Colosseum stage with three named lenses (clause-to-line mapping, deferred-is-panic, API field-name fidelity).

### Kani-harness
**Applied to:** 10 harnesses, all on IRV result structure: B1 (post-publish), S4 (ballot-key subset), S6 (non-candidate winner rejected), S7 (partition equation), S8 (round sum), S9 (reappearance), S10 (resolution after end_at), already-voted, already-resolved, derive_phase. **Missed:** M4 (would have caught in seconds), partially M1 (phase-gate on CreateElection). **Why it missed them:** the Kani harness catalog was constructed bottom-up from the IRV correctness theorems, not top-down from the audit-surface invariants. The catalog has zero harnesses targeting: attestation verification paths, registry shape, gnark public_inputs construction, canonical_serialization byte layout, ECIES decoder rejection paths. **v0.4 ask AC:** Kani harness catalog must be derived from §8.7-style ledger links, not from theorem inventory.

### Verus-harness
**Applied to:** nothing (blocked harnesses are queued but Verus is not yet in the substrate). **Missed:** in principle the same surfaces Kani missed. **Why:** infrastructure gap; not a methodology gap per se.

### cross-layer-ledger
**Applied to:** `.colosseum/ledger.md` + intent §8.7 (9 trust-chain links). **Missed:** N1 (chain accepts attestation without verify), M2 (cross-layer hash coverage), partial C2 (clause-to-line). **Why it missed them:** the ledger was a *map* of obligations but not a *gate*. The ledger named the right links; no stage forced a reconciliation between ledger claims and executable code. **v0.4 ask AD:** every ledger link must point to a code-line citation (`file:line`) discharging the claim; CI gates on missing citations.

### lifecycle-adversary **[proposed]**
**Applied to:** never. **What it should have caught:** N17 entirely. Partial coverage of N2 (registry rotation lifecycle). **What it is:** when a multi-tx admin feature ships, an adversarial review of all multi-block sequences against active-phase invariants. **v0.4 ask AE:** lifecycle-adversary is a mandatory follow-on stage when the contract gains a feature with >1 admin transitions or any timelock.

### CI-self-test **[proposed]**
**Applied to:** never; the v0.3.10 CI additions broke 3 of 4 on their own clean build. **What it should have caught:** N18 (cargo-audit RUSTSEC ignore), N19 (mock_attestation regex false-positives on user-facing "verification" strings), N20 (cosmwasm-check rejects raw cargo build). **What it is:** before merging a CI gate, the gate must be green at the commit that adds it. **v0.4 ask AF:** added-CI-gates-must-pass-locally as an enforced pre-merge step.

## The four core methodology gaps

Phase 2 surfaces four sharp gaps in Colosseum v0.2:

1. **code-adversarial is absent.** 6 of 13 high-severity findings would have been caught by an internal code-adversarial stage with three named lenses. The audit rounds were the de-facto code-adversarial.
2. **intent-adversarial lacked named lenses.** Three lenses (commitment-coverage, who-controls, multi-clause-decomposition) would have caught a further 5 findings at the cheapest possible stage.
3. **Kani catalog was bottom-up.** The harness catalog was derived from IRV correctness theorems, not from trust-boundary invariants. Top-down catalog construction from §8.7 ledger links would catch M4 (and prevent the M2/B6/N4/N22 pattern from going un-harnessed).
4. **No lifecycle-adversary.** Multi-tx admin sequences across the registry-rotation feature were un-modeled in Quint and un-red-teamed in any stage.

These four gaps map to four v0.4 asks (AB, AC, AD, AE) detailed in Phase 4 below.

---

# Phase 3: root-cause clustering

Phase 2 produced a (finding × stage) matrix. Phase 3 collapses the 13 high-severity findings into root-cause clusters: groups of findings that share a single methodology defect such that one v0.4 ask would prevent all members of the cluster.

## Cluster R1: commitment-coverage (4 findings, 1 minor)

**Members:** M2 (election_id), N4 (chain_id), N22 (registration ReportData), B6 (ballots_hash). Plus minor m7 (ECIES chain context).

**Shared root cause:** the methodology does not enforce a commitment-coverage review pass. Every hash, every signature payload, every ReportData has a *coverage set* of attacker-controllable degrees of freedom. The review must enumerate the attacker DoFs and confirm each is bound. None of the four were bound; each was caught one-at-a-time across 4 audit rounds.

**Cost of repetition:** M2 + N4 + N22 + B6 closed across v0.3.8, v0.3.10, v0.3.12, v0.3.11 — four separate audit rounds + one self-identification. A single "for each external hash, enumerate attacker DoFs and confirm binding" review would have closed all four at v0.3.7.

**v0.4 ask:** **AB.1 commitment-coverage lens** — added to intent-adversarial checklist.

## Cluster R2: trust-assumption-decomposition (3 findings)

**Members:** C2 (B8 clauses a/b/c/d only checked envelope), C3 (enclave_pubkey provenance), N1 (gnark verification deferred).

**Shared root cause:** the methodology does not decompose multi-clause trust assumptions. Intent §3.2 B8 had four clauses (a/b/c/d) plus an unstated (e); §6.3 dstack_kms_trust had multiple necessary conditions. When code implements only a subset and the methodology lacks a "clause-to-line" discharge protocol, the gap is invisible until audit.

**Cost of repetition:** C2 took R1+R2 to close. C3 took R1+R2+R4 (three rounds!). N1 reopened C2.

**v0.4 ask:** **AB.2 clause-to-line discharge** — code-adversarial enforces a checklist mapping every named intent clause to a code-line citation.

## Cluster R3: deferred-is-panic (2 findings, 1 minor)

**Members:** C1 (Mock variant), N11 (real-zkdcap returns Err stub).

**Shared root cause:** stub-as-Ok pattern. The methodology had no enforced lens "any deferred-implementation branch must panic, not return Ok or empty."

**Cost of repetition:** C1 + N11 + (informational i8 unimplemented!()-reachable, same shape).

**v0.4 ask:** **AB.3 deferred-is-panic lens** — code-adversarial enforces that deferred branches panic.

## Cluster R4: who-controls + admin lifecycle (3 findings)

**Members:** N2 (admin picks pubkey), N17 (finalize-mid-election DoS), partial M1 (CreateElection phase-gate).

**Shared root cause:** the methodology does not enumerate, for each field of stored state, *who supplies it* and what stops adversarial values, and does not red-team multi-tx admin sequences. N2 + N17 are the deepest forms of this; M1's phase-gate-on-CreateElection is the simplest form.

**Cost of repetition:** Three findings across two audit rounds. N17 was a *brand new* attack class introduced by the v0.3.10 timelock feature whose Quint model wasn't extended.

**v0.4 ask:** **AE lifecycle-adversary stage** + **AB.4 who-controls lens**.

## Cluster R5: API field-name fidelity (1 finding, multiple informational)

**Members:** M5 (mrtd_hex slot held compose_hash). Plus informational i3 (ECIES misnames AEAD), i5 (vkey field type drift).

**Shared root cause:** runtime API surface drifted from intent semantic. The methodology had no lens "does the value placed at JSON field X carry the semantic the field name implies?"

**Cost of repetition:** mostly minor; M5 is the high-severity instance.

**v0.4 ask:** **AB.5 API field-name fidelity lens** in code-adversarial.

## Cluster R6: Kani catalog bottom-up (1 finding, latent multiple)

**Members:** M4 directly. Latent: every B8 clause, every canonical_serialization invariant, every registry-shape invariant — *zero* of which had Kani harnesses.

**Shared root cause:** Kani catalog derived from theorem inventory, not from §8.7 ledger. The harnesses Kani did have were *good*; the gap was in catalog construction.

**v0.4 ask:** **AC top-down Kani catalog** — every §8.7 ledger link must have either a Kani harness or an explicit "no-harness because…" annotation.

## Cluster R7: ledger as map vs ledger as gate (1 finding, contributes to several)

**Members:** N1 directly. Contributes to C2 (no clause-to-line gate), M2 (no cross-layer hash reconciliation).

**Shared root cause:** the §8.7 ledger named the right links but was not a gating artifact. No CI step required every link to point to executable code discharging it.

**v0.4 ask:** **AD ledger-as-gate** — every §8.7 link gets a `code: file:line` annotation; CI fails if any link lacks one.

## Cluster R8: deferral justifications + CI-self-test (transversal)

**Members:** N13→N22 (deferral justification was wrong, caught one audit round later). N18+N19+N20 (CI gates broke on add).

**Shared root cause:** the methodology has no protocol for auditing its own deferrals or self-testing its own CI additions.

**v0.4 asks:**
- **AB.6 deferral-justification audit** in code-adversarial: every "this is deferred because X" gets X as a refutable claim, audited at the next revision.
- **AF CI-self-test** stage: added CI gates must be green at the commit that adds them.

## Cluster summary

| cluster | members | v0.4 ask | findings closed at cheapest stage |
|---|---|---|---|
| R1 commitment-coverage | M2, N4, N22, B6, m7 | AB.1 | 4 high-sev + 1 minor |
| R2 clause-to-line | C2, C3, N1 | AB.2 | 3 high-sev |
| R3 deferred-is-panic | C1, N11 | AB.3 | 2 high-sev |
| R4 who-controls + lifecycle | N2, N17, M1-partial | AB.4 + AE | 3 high-sev |
| R5 field-name fidelity | M5 | AB.5 | 1 high-sev |
| R6 Kani top-down | M4 + latent | AC | 1 high-sev + latent prevention |
| R7 ledger-as-gate | N1, contributes to C2/M2 | AD | reinforces R2 |
| R8 deferral + CI-self-test | N13→N22, N18/N19/N20 | AB.6 + AF | 1 high-sev + 3 CI |

Eight clusters; the same five-or-six methodology asks would have caught every high-severity finding at the cheapest possible Colosseum stage.

---

# Phase 4: v0.4 methodology asks

Each ask is a concrete, mechanical addition to Colosseum v0.4. Phrased as a delta from v0.2 as run on verified-rcv.

## Ask AB: code-adversarial stage with six named lenses

**Status:** new stage.

**Trigger:** after code-implementation produces a commit, before audit.

**Operator:** an internal red-team agent (separate from the code-implementation author). Reads the *implementation*, references *intent + ledger*, applies the lenses below.

**Lenses:**

| lens | discipline |
|---|---|
| **AB.1 commitment-coverage** | For every hash / signature payload / ReportData / serialized commitment in the contract or runtime, enumerate the attacker-controllable degrees of freedom. Confirm each DoF is bound (named in the coverage set). Output: a table `(commitment, attacker DoFs, bound by, gap)`. |
| **AB.2 clause-to-line discharge** | For every named intent clause (B-clauses in §3.2, ledger links in §8.7, trust assumptions in §6.x), point at a code-line citation (`file:line`) that discharges it. Output: a table `(clause, expected discharge, actual code line, status)`. Status ∈ {discharged, partial, gap}. |
| **AB.3 deferred-is-panic** | For every branch labeled "deferred", "stub", "mock", "TODO" in the production build (i.e., default features), confirm the branch panics. Output: a list of all deferred branches with their failure modes. Any non-panic deferred-branch is a finding. |
| **AB.4 who-controls** | For every field of stored state, name *who supplies the value* and *what stops adversarial values*. Output: a table `(field, supplier, validation, gap)`. |
| **AB.5 API field-name fidelity** | For every JSON / proto / borsh field in a public API surface, confirm the value placed at the field carries the semantic the field name implies. Output: a table `(API surface, field, name implies, actual semantic, status)`. |
| **AB.6 deferral-justification audit** | For every deferred finding from a prior audit round, decompose the deferral justification into refutable claims and audit each. Output: `(prior finding, deferral claim, audit result)`. |

**Deliverable:** `.colosseum/code-adversarial/<date>.md` with the six tables above and a list of findings.

**Estimated coverage on verified-rcv:** 8 of 13 high-severity findings (C1, C2, C3, M2, M5, N1, N2, N13/N22) would have been caught at this stage.

## Ask AC: Kani catalog top-down from §8.7 ledger

**Status:** new discipline on existing Kani-harness stage.

**Trigger:** every revision of intent §8.7 (the trust-chain ledger).

**Operator:** the Kani-harness author.

**Discipline:** every link in §8.7 must have either (a) a Kani harness named `<link_id>_<assertion>` that exercises the link's claim, or (b) an explicit annotation `kani: skipped because <reason>` (e.g., "off-chain", "covered by cross-layer-ledger byte-equality test", "Verus-only"). The CI's verification feature must run all harnesses; a missing harness without skipped-annotation fails CI.

**Estimated coverage on verified-rcv:** M4 directly, plus latent coverage for every B8 clause and every commitment hash. Wouldn't catch every cluster-R1 finding immediately (Kani isn't the cheapest stage for commitment-coverage; that's intent-adversarial), but it would *prevent* the regression after the intent fix lands.

## Ask AD: ledger-as-gate (code-line citations on §8.7 links)

**Status:** new discipline on existing cross-layer-ledger stage.

**Trigger:** every revision of intent §8.7.

**Operator:** intent author + code-implementation author jointly.

**Discipline:** every link in §8.7 carries a `code: file:line` annotation pointing to the line that discharges the claim. (For chain-side claims, the contract handler; for runtime-side, the enclave path; for off-chain, the testing harness or "axiom" annotation.) CI runs a check: every §8.7 link has a citation, and every citation resolves to a non-empty line.

**Estimated coverage on verified-rcv:** N1 directly. Reinforces AB.2.

## Ask AE: lifecycle-adversary stage

**Status:** new stage.

**Trigger:** any contract revision that adds (a) a new admin transition, (b) a timelock, (c) a multi-tx feature (Propose/Finalize/Cancel patterns), or (d) a state archival.

**Operator:** an internal adversary agent.

**Discipline:** enumerate all multi-block sequences combining the new transitions with existing transitions. For each sequence, name the worst outcome for an honest user. The Quint model gets extended to encode the new transitions; Quint-adversarial generates counterexample traces against the active-phase invariants.

**Deliverable:** `.colosseum/lifecycle-adversary/<feature>.md`.

**Estimated coverage on verified-rcv:** N17 entirely. Partial coverage of N2.

## Ask AF: CI-self-test gate

**Status:** new pre-merge check.

**Trigger:** any PR that adds a CI step.

**Discipline:** the new CI step must be green at the commit that adds it (i.e., the CI must pass before merge, with the new step active). This is just enforcing the CI's own purpose, but the methodology gap on verified-rcv was that v0.3.10 added 4 CI gates of which 3 broke on their own clean build.

**Estimated coverage on verified-rcv:** N18, N19, N20.

## Ask AG (minor): commit-message vs code reconciliation

**Status:** new lightweight check.

**Trigger:** every commit.

**Discipline:** for every commit that claims to defer or implement a named finding, the diff is mechanically checked against the claim. The auditor flagged N7 (commit said "N4 deferred" while code changed N4); this is catchable by a simple grep + diff cross-check.

**Estimated coverage on verified-rcv:** N7.

## Asks summary

| ask | type | stage | high-sev findings prevented |
|---|---|---|---|
| AB.1–AB.6 | new lenses | code-adversarial [new] | 8 of 13 |
| AC | discipline | Kani-harness [existing] | M4 + latent regression prevention |
| AD | discipline | cross-layer-ledger [existing] | N1 + reinforces R2 |
| AE | new stage | lifecycle-adversary [new] | N17 + partial N2 |
| AF | new check | CI-self-test [new pre-merge] | N18 + N19 + N20 |
| AG | new check | commit-message reconciliation [new pre-merge] | N7 |

Combined: 11 of 13 high-severity findings + all CI breakages + N7 would have been caught at a cheaper stage than external audit.

The 2 high-severity findings *not* fully covered by these asks (M1 partial, M3) are partially covered:
- **M1** (phase-gate on CreateElection) — partially caught by AB.4 (who-controls: phase-supplied-by-protocol but CreateElection doesn't check), partially by AE (lifecycle-adversary on registry rotation could lift the discipline to other transitions). A full catch would require **Quint-adversarial as a separately-run stage with adversarial trace generation** — call this **Ask AH (extend Quint with adversarial-trace pass)**.
- **M3** (registry length-pin) — partially caught by AB.4 (who-controls names admin-supplied, validation gap). A full catch would require **intent-elicitation discipline: every binary-data field gets a wire-length pin** — call this **Ask AI (field-spec discipline at intent-elicitation)**.

With AH + AI added, the methodology covers all 13 high-severity findings.

---

# Phase 5: validation protocol against next project

The Colosseum v0.4 ask list (AB–AI) needs a validation pass before being trusted. The validation protocol:

## Comparator: Quartz ranked-choice re-implementation

**Status:** not yet started. Blindness policy remains in effect — `/Users/mvid/Development/reliq/quartz/examples/ranked-choice/**` must not be read until the comparator pass.

**Validation plan:**

1. **Snapshot the methodology.** At a known commit of `/Users/mvid/Development/reliq/colosseum/`, freeze v0.4 with asks AB–AI integrated. Tag the methodology commit.

2. **Run v0.4 against verified-rcv retrospectively.** Apply each new ask (AB–AI) to intent v0.3.7 (the pre-audit intent). Measure how many of the 13 high-severity findings the methodology *would* have surfaced before audit. Target: ≥11 of 13 (the count predicted in Phase 4).

3. **Run v0.4 against Quartz ranked-choice.** Lift the blindness policy. For each of the Quartz ranked-choice findings (these will surface in the comparator pass), measure how many v0.4 would have caught.

4. **Calibrate.** Differences between predicted-coverage and observed-coverage on Quartz are the validation signal. If v0.4 catches what verified-rcv-retrospective predicts but misses Quartz findings, the methodology has gaps the verified-rcv dogfood didn't expose. If v0.4 catches both, the methodology generalizes.

5. **Cross-tool comparator.** verified-rcv used Aeneas + Lean + Quint + Kani as the formal substrate. Quartz uses a different substrate (presumed: dstack + Quartz envelope). Ask asymmetry: does code-adversarial (AB) work on a different substrate? Does the §8.7 ledger discipline (AD) translate? The validation pass should produce a cross-substrate translation table.

## Self-validation tests

Before lifting the blindness policy on Quartz, two self-validation tests:

**Test V1 (false-negative test on verified-rcv).** Take the v0.4 methodology, run it against intent v0.3.6 (pre-audit). For each high-severity finding that v0.3.6 had: did the methodology surface it? Predicted: ≥11/13. Acceptance: ≥10/13 (allowing one miss per cluster as the methodology is not omniscient).

**Test V2 (false-positive test on intent v0.3.12).** Take the v0.4 methodology, run it against intent v0.3.12 (the current, all-findings-closed state). For each lens, count the "findings" the lens raises. Target: low single digits (the lenses should not produce massive false-positive lists; if they do, the lenses are too liberal). Acceptance: <5 lens-raised findings per lens that aren't already in the audit history.

## Validation deliverable

The validation pass produces `.colosseum/v0_4_validation/<date>.md` containing:

- The retrospective-coverage table (which of the 13 verified-rcv findings v0.4 catches).
- The Quartz-comparator coverage table (lifted after blindness policy is released).
- Test V1 + V2 results.
- A "this is what didn't generalize" section: any methodology gap that the verified-rcv dogfood couldn't surface.

Round 3a's methodology claim depends on the validation pass closing without "everything works perfectly" — that would be a calibration failure, not a success. The pass succeeds when ≥10/13 verified-rcv findings + most-of Quartz findings are caught, with a small honest list of methodology gaps that remain.

---

# Closing notes

## What the four audit rounds proved

- Colosseum v0.2's intent-elicitation worked. 9 of 13 high-severity invariants were named in the intent.
- Colosseum v0.2's intent-adversarial was thin. It attacked theorem statements but lacked named lenses for commitment-coverage / who-controls / multi-clause-decomposition.
- Colosseum v0.2 had no code-adversarial stage. The audit rounds *were* the code-adversarial, but external.
- Colosseum v0.2's Kani catalog was bottom-up. Good for what it covered; blind to trust-boundary surfaces.
- Colosseum v0.2's ledger was a map, not a gate. The §8.7 trust-chain links named the right structure but were not enforced against code.
- Colosseum v0.2 had no lifecycle-adversary stage. N17 was a brand-new attack class introduced by a feature whose Quint model wasn't extended.

## What the v0.4 ask list achieves

If applied to intent v0.3.7 + the implementation commits at the pre-audit state:
- **11 of 13 high-severity findings** caught at a stage cheaper than external audit.
- **3 of 4 CI breakages** caught at pre-merge.
- **All 4 instances of the replay-narrow-binding cluster** caught at a single intent-adversarial pass.
- **All 3 instances of the trust-assumption-decomposition cluster** caught at code-adversarial.

With AH + AI added, coverage extends to 13 of 13 high-severity findings.

## What remains uncertain

- Whether the lenses generalize off the verified-rcv dogfood. The cross-substrate validation against Quartz is the calibration signal.
- Whether code-adversarial as an internal stage drifts toward author-self-review (a known anti-pattern). The stage operator must be a different agent than the code-implementation author.
- Whether the AC top-down Kani catalog scales — verified-rcv has 9 ledger links + a small contract; a larger system may have hundreds of links.

These are honest open questions for v0.4 → v0.5.

---

**End of retrospective.** Phases 1–5 complete; v0.4 ask list ready for back-port to `/Users/mvid/Development/reliq/colosseum/`.
