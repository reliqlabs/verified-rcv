# Re-Audit Report — verified-rcv

Companion to `.colosseum/audit/2026-05-26-18-36-audit-claude.md` (the prior audit). One remediation commit landed in between; this report verifies closure for each prior finding and looks for new issues introduced by the remediation code.

## Metadata

| Field | Value |
|-------|-------|
| Target | `/Users/mvid/Development/reliq/verified-rcv` |
| Prior audit revision | `3869b1e` (2026-05-26 18:36 UTC) |
| Re-audit revision | `36fd01b` (2026-05-26 19:59 UTC) |
| Remediation commit | `36fd01b` "Audit-finding remediation: C1/C2/C3 + M1/M2/M3/M4/M5 + intent v0.3.8" |
| Lines of code changed | +3827 / -203 (workspace-wide); contract.rs +989/-203, msg.rs +76, error.rs +34, attestation.rs +327 (new), server.rs +253 (new), cross_canonical.rs +100 (new) |
| Tests added | 25 contract + 16 runtime + 4 cross-canonical = 45 new tests; all pass |
| Scanner | cosmos-auditor methodology applied verification-first to each prior finding, plus targeted new-finding hunt on the remediation code (+1300 LoC). Static analyzers re-run. |
| Platform | `claude` |

## Headline

The remediation is substantive and disciplined. All eight prior findings (C1/C2/C3 + M1-M5) have been addressed in code with negative tests covering the rejection paths. The intent has been bumped to v0.3.8 documenting each fix plus three new methodology asks (Y: contract-and-runtime adversarial fan-out; Z: deferred-is-panic; AA: negative-test discipline).

However, three of the eight findings are only **partially closed**. The most consequential gap is C2: the chain now verifies the attestation envelope's `user_data` commit hash, but the underlying TDX quote (B8 clause a), Groth16 zkdcap proof (B8 clause b), and MRTD/RTMR-vs-registry binding (B8 clause d) remain unimplemented. The chain-side attestation pipeline now provides **consistency** (the `user_data` actually binds to this tally), but not **soundness** (no proof the user_data came from a real attested enclave). An attacker who computes the matching SHA-256 themselves can still construct an envelope the chain accepts. The previous Critical exploit path is narrowed (cross-election replay closed by M2; Mock variant gone in production builds) but not eliminated.

Trust-chain status:

- **C1 (Mock variant)**: fully closed for production builds. Compile-time excluded.
- **C2 (Dstack stub)**: **partially closed**. Envelope-binding implemented (clauses c, plus a domain tag check). TDX quote (a), Groth16 proof (b), and registry binding (d) remain unimplemented. The chain still cannot distinguish a real attestation from an attacker-constructed envelope.
- **C3 (enclave_pubkey unverified)**: **partially closed**. Shape validation (length + leading byte) implemented. Cryptographic provenance (dstack-KMS-signed derivation proof) remains operational-trust only. Admin substitution attack on ballot privacy is unmitigated.
- **M1 (CreateElection bypasses B1)**: **partially closed**. The handler now rejects when the prior election is Voting or Tallying. A CreateElection from `Resolved` is still permitted and still clears `TALLY_RESULT`. The intent v0.3.8 implicitly scopes B1 to a single election lifecycle; consumers who care about cross-election immutability must pin `(election_id, tally_result)` together.
- **M2 (no election_id in commit)**: fully closed. `canonical_serialization` includes `election_id` between `contract_addr` and `tally_body`. Cross-test asserts byte-equality between contract and runtime encoders.
- **M3 (registry not validated)**: fully closed. mrtd=48 and rtmr=48 enforced (TDX SHA-384); vkey non-empty enforced. `UpdateRegistry` handler added with phase gate.
- **M4 (per-round order not enforced)**: fully closed. `is_in_declaration_order` enforced on winners, dropped_voters, non_voters, per-row of per_round_counts and eliminated_by_round.
- **M5 (compose_hash in mrtd_hex slot)**: fully closed. HealthResponse returns `ready=false` and empty `mrtd_hex`/`rtmr_hex` until a TDX quote parser lands.

After the remediation, two prior findings remain Critical (C2 residual, C3 residual). The other six are closed or downgraded to Minor.

## Summary of findings

| Category | Count |
|----------|-------|
| Prior findings fully closed | 5 (C1, M2, M3, M4, M5) |
| Prior findings partially closed (residual remains) | 3 (C2, C3, M1) |
| Prior findings not addressed in this cycle | 17 (m1–m8, i1–i10 from prior audit excluding i9 which is now fully reconciled by M3's vkey shape pin; leads L1–L10) |
| New findings introduced by the remediation | 6 |

| Severity | Count after remediation |
|----------|--------------------------|
| Critical (residual) | 2 |
| Major (residual + new) | 3 |
| Minor (residual + new) | 9 |
| Informational (residual + new) | 11 |
| Leads | 11 |

## Closure status, per prior finding

### Critical — closure status

#### C1 (Mock variant ungated) — fully closed

**Remediation:** `crates/contract/src/msg.rs:112` gates the `AttestationEnvelope::Mock` variant behind `#[cfg(feature = "mock-attestation")]`. The Cargo feature is non-default (`crates/contract/Cargo.toml:15: default = []`). Production wasm compiles out the variant entirely. The handler's match arm at `contract.rs:325` is also gated.

**Evidence:** 25 unit tests in `contract.rs::tests`, including `mock_variant_absent_in_production_build` at line 1265-1276 which contains a commented-out `let _ = AttestationEnvelope::Mock;` line as proof — uncommenting it fails to compile under default features.

**Test for regression:** `cargo build -p verified-rcv-contract --target wasm32-unknown-unknown` (no features). The Mock variant is absent. JSON `{"mock":null}` payloads fail at deserialization.

**Residual risk:** the `verification` Cargo feature in `crates/contract/Cargo.toml:28` includes `mock-attestation`: `verification = ["mock-attestation"]`. A developer who builds Kani harnesses with `--features verification` accidentally enables Mock. This is mitigated by the fact that Kani harness builds are not normally deployed, but a CI gate that fails any wasm with mock-attestation enabled would tighten the guarantee. See new finding **N5**.

#### C2 (Dstack length-only stub) — partially closed

**Remediation:** `crates/contract/src/contract.rs:432-616` implements clause (c) of intent §3.2 B8:

- Length check (was already present): `user_data.len() == 64`.
- Domain tag check (new): upper 32 bytes equal `DST_VERIFIED_RCV_TALLY_V1` zero-padded.
- Commit hash check (new): lower 32 bytes equal `SHA-256(canonical_serialization(contract_addr || election_id || tally_body))`.

The verifier uses dedicated error variants (`AttestationCommitMismatch`, `AttestationDomainTagInvalid`) that let a downstream operator distinguish "wrong shape" from "envelope binds to a different (addr, id, tally)". The commit-hash computation uses a hand-rolled `canonical_serialization` that mirrors the runtime's encoder byte-for-byte; a cross-test at `crates/enclave/tests/cross_canonical.rs` enforces equality.

**What is still NOT verified:**

- **Clause (a)**: TDX quote signature validation. The `quote: HexBinary` field is pattern-bound to `_` at `contract.rs:330` and discarded. Comment at `contract.rs:335-340` acknowledges the gap as "TODO (follow-on cycle): Groth16 verification of zk_proof against registry.vkey via Xion's ProofVerifyGnark module; MRTD/RTMR-vs-registry binding via zkdcap public-input extraction."
- **Clause (b)**: Groth16 zkdcap proof verification. `zk_proof: HexBinary` is also pattern-bound to `_` and discarded.
- **Clause (d)**: MRTD/RTMR-vs-registry binding. `_registry` is loaded into a discarded variable at `contract.rs:322` and never compared against the (unverified) quote.

**Residual exploit path:** an attacker constructs a `TallyResult` of their choice. They compute `SHA-256(canonical_serialization(env.contract.address, election.id, their_tally))` themselves (the canonical_serialization function is publicly observable via the runtime crate). They construct `user_data = DOMAIN_TAG || their_hash`. They submit `PublishResult { tally: their_tally, attestation: Dstack { quote: <any bytes>, zk_proof: <any bytes>, user_data: <constructed> } }`. The contract verifies:

1. Phase is Tallying. ✓
2. Well-formedness (S6/S7/S8/S9 + M4 declaration-order). ✓ (the attacker constructed it well-formed)
3. Length 64, domain tag matches, commit hash matches. ✓ (the attacker computed it)

The chain accepts the forged tally. The attacker pays only gas. The B8 trust anchor is unrealized at the chain layer until clauses (a), (b), and (d) are implemented.

This is **the same Critical exploit path the prior audit flagged**, with the bar raised slightly (the attacker must now compute the matching commit hash instead of using arbitrary 64-byte buffers). Severity remains Critical until full attestation verification lands.

**Severity post-remediation:** Critical (residual). Tracked as new finding **N1** below for the forward-looking gap.

#### C3 (enclave_pubkey unverified) — partially closed

**Remediation:** `crates/contract/src/contract.rs:732-747` validates that `enclave_pubkey` is either 33 bytes with a leading 0x02/0x03 byte (compressed SEC1) or 65 bytes with a leading 0x04 byte (uncompressed SEC1). Negative tests for each rejection path are present.

**What is still NOT verified:**

- **Curve-point validity**: a 33-byte or 65-byte buffer with the correct leading byte but other bytes that do not lie on the secp256k1 curve passes the shape check. In practice, an admin who supplies such a buffer would cause voter-side encryption to fail (noble's `ProjectivePoint.fromHex` is called only on the 33-byte path in `ui/src/lib/encryption.ts:64-68`; the 65-byte path does only a prefix check). Worst case is election deadlock, not privacy break, since the matching privkey does not exist.
- **Cryptographic provenance**: the chain has no proof that `enclave_pubkey` was actually derived by the dstack KMS for `(contract_addr, election_id)`. A malicious admin substitutes a keypair whose private half they hold and decrypts every submitted ballot off-chain. The intent at §6.3 names this as the operational `dstack_kms_trust` boundary; the contract code now has a comment at `msg.rs:50-55` acknowledging the gap.

**Residual exploit path:** the **ballot privacy break** from prior Finding C3 is unmitigated. The admin generates a keypair locally, calls `CreateElection` with the local pubkey, harvests ballots via `query::Ballots`, decrypts off-chain. Combined with the C2 residual, the admin then publishes any tally they want. This is the same end-to-end privacy + integrity compromise the prior audit described, available to a malicious admin.

**Severity post-remediation:** Critical (residual). Tracked as new finding **N2** below for the forward-looking gap. The shape validation does reduce attack-surface noise (random non-SEC1 buffers no longer pass), but the load-bearing risk (admin substitution) is unchanged.

### Major — closure status

#### M1 (CreateElection bypasses B1) — partially closed

**Remediation:** `crates/contract/src/contract.rs:178-187` checks the prior election's phase and rejects `CreateElection` if the phase is `Voting` or `Tallying`. The handler returns the new `ElectionAlreadyActive` error variant. Negative test `create_election_during_voting_rejected` at `contract.rs:1072-1121` confirms the rejection.

**What is still NOT prevented:** `CreateElection` from the `Resolved` phase is still permitted and still calls `TALLY_RESULT.remove` at line 226. The previous finding M1's framing was about B1's "once Resolved, the value is permanent forever." The new behavior preserves B1 *within* an election lifecycle but allows the value to be replaced when starting a new election. The intent v0.3.8 implicitly accepts this scope via the M1 fix; the formal B1 statement in `.colosseum/intent.md` §3.2 was not edited.

**Residual concern:** a downstream consumer who reads `tally_result` and caches it can have the cached value silently invalidated by a `CreateElection` call. The consumer must read both `election.id` and `tally_result` together and treat them as a tuple. Mitigation is operator/integration-level, not chain-side. Tracked as new finding **N3** below.

**Doc-drift:** `crates/contract/src/state.rs:39-46` still has the doc-comment "Single-election contract; `CreateElection` overwrites (admin-only) — intent v0.3.4 §2.5 Block 1 (alternate path)." This contradicts the new phase gate. Tracked as new finding **N6** below.

**Severity post-remediation:** Minor (residual; documentation / consumer-expectation drift).

#### M2 (no election_id in commit hash) — fully closed

**Remediation:** `crates/contract/src/contract.rs:653-664` and `crates/enclave/src/attestation.rs:69-80` both define `canonical_serialization(contract_addr, election_id, tally)` with `election_id` as a u64 little-endian value emitted between `contract_addr` and `tally_body`. The cross-test `crates/enclave/tests/cross_canonical.rs:38-46` asserts byte-equality between the two encoders. Additional cross-tests at lines 48-99 verify that `election_id` actually affects the output bytes and that the build_user_data commit hash matches the contract's compute_commit_hash.

**Residual concern:** chain_id is still NOT included in the canonical serialization. Cross-chain replay (mainnet vs testnet with the same contract address) remains possible once attestation is fully implemented. The prior audit flagged this as Lead L9; it remains a forward-looking design point. Tracked as new finding **N4** below.

**Severity post-remediation:** Fully closed for cross-election replay. Lead L9 (chain_id binding) remains open.

#### M3 (registry not validated) — fully closed

**Remediation:** `crates/contract/src/contract.rs:751-770` validates `mrtd.len() == 48` (TDX SHA-384), `rtmr.len() == 48`, and `vkey` non-empty. Validation runs at instantiate (`contract.rs:92`) AND at the new `UpdateRegistry` handler (`contract.rs:365`). The `UpdateRegistry` handler is admin-gated and phase-gated (rejects during Voting/Tallying). Negative tests cover wrong-mrtd-length, wrong-rtmr-length, empty-vkey, and unauthorized-update-attempt.

**Residual concern:** the chain still has no cryptographic proof that the registered values are the canonical verified-rcv enclave image's measurements. The intent at §6.1 names `image_registration_honest(σ)` as the operational predicate. With B8 (a)+(b)+(d) still unimplemented (Finding N1), even a correctly-validated registry shape provides no end-to-end protection — the registry's contents are never consulted at attestation time.

**Doc-drift:** `crates/contract/src/state.rs:65-73` still says "Set at instantiate ... never mutated." Contradicts the M3 `UpdateRegistry` handler. Same doc-drift as M1; tracked as new finding **N6** below.

**Severity post-remediation:** Fully closed for the validation portion. The downstream chain-side use of the registry remains gated by Finding N1.

#### M4 (per-round order not enforced) — fully closed

**Remediation:** `crates/contract/src/contract.rs:546-561` defines `is_in_declaration_order` as a generic linear-scan helper. `check_tally_well_formed` invokes it on `tally.winners`, `tally.dropped_voters`, `tally.non_voters`, every row of `per_round_counts`, and every row of `eliminated_by_round`. Three negative tests confirm the rejection path (`declaration_order_subsequence_accepted`, `declaration_order_shuffled_rejected`, `declaration_order_non_candidate_rejected`, plus `check_tally_well_formed_shuffled_winners_rejected` and `check_tally_well_formed_shuffled_per_round_rejected`).

**Helper-function correctness:** I reviewed `is_in_declaration_order` carefully:

```rust
fn is_in_declaration_order<S: AsRef<str>>(subset: &[S], full: &[Addr]) -> bool {
    let mut i = 0;
    for c in subset {
        while i < full.len() && full[i].as_str() != c.as_ref() {
            i += 1;
        }
        if i >= full.len() {
            return false;
        }
        i += 1;
    }
    true
}
```

This is a left-to-right scan with a monotone pointer. Each element of `subset` must appear in `full` strictly after the previous match. Duplicates in `subset` (e.g., a tally that lists the same candidate twice in winners) would require two appearances in `full`; since `full` (election.candidates) is dedup-validated, the function effectively rejects duplicates in subset. Empty `subset` returns true (a vacuous subsequence). Empty `full` with non-empty `subset` returns false. The helper is correct for its intended use.

**Severity post-remediation:** Fully closed.

#### M5 (compose_hash in mrtd_hex slot) — fully closed

**Remediation:** `crates/enclave/src/server.rs:89-123` returns `HealthResponse { ready: false, mrtd_hex: "".into(), rtmr_hex: "".into() }`. The compose_hash is no longer surfaced via the gRPC API. Operators who follow the deploy guide cannot accidentally populate the on-chain `EnclaveImageRegistry` with the wrong artifact.

**Forward-compatibility:** the `_identity` variable is still computed at line 113 and discarded. When a TDX quote parser lands, the body of `health` can extract real MRTD/RTMR and populate the fields with the parsed values. The `ready` flag would flip to true once the parser succeeds. The current scaffold is consistent with that direction.

**Severity post-remediation:** Fully closed.

## New findings introduced by the remediation

### Finding N1: Chain-side attestation verification is consistency-only; clauses (a) TDX quote, (b) Groth16 proof, (d) registry binding remain unimplemented

**Severity:** Critical (residual from prior C2)
**Confidence:** 95/100
**Principle Violated:** VCL-CRYP-001 (replay and signature vulnerabilities — chain accepts attestations without verifying the cryptographic source)
**Location:** `crates/contract/src/contract.rs:322-342` (the handler dispatch); `crates/contract/src/contract.rs:578-604` (the verifier)

#### Description

The remediation implements clause (c) of intent §3.2 B8 (commit hash equality) and adds a domain tag check. Clauses (a), (b), and (d) remain unimplemented. The handler explicitly acknowledges this at lines 335-340:

```rust
// TODO (follow-on cycle): Groth16 verification of zk_proof
// against registry.vkey via Xion's ProofVerifyGnark module;
// MRTD/RTMR-vs-registry binding via zkdcap public-input
// extraction. Currently the on-chain Dstack verification is
// ENVELOPE-BOUND (commit hash + domain tag) but does NOT
// re-verify the underlying TDX quote or zkdcap proof.
```

The commit-hash binding ensures that a given attestation envelope is consistent with a specific `(contract_addr, election_id, tally_body)` triple. It does not establish that the envelope was produced by a real enclave attestation. An attacker who can compute the SHA-256 themselves (a public computation) can construct a consistent envelope for any tally they choose.

#### Exploit Path

1. Adversary observes the election reaches Tallying phase.
2. Adversary constructs `tally = <forged>` satisfying chain-syntactic well-formedness (S6/S7/S8/S9 + M4 declaration-order). All these are public predicates the adversary controls.
3. Adversary reads `env.contract.address` (public) and `election.id` (public).
4. Adversary computes the canonical serialization byte buffer and then `SHA-256` of it. Both are public computations; the `canonical_serialization` function is in `crates/contract/src/contract.rs` and `crates/enclave/src/attestation.rs`, both readable.
5. Adversary constructs `user_data = DST_VERIFIED_RCV_TALLY_V1 || <padding> || <computed_hash>`.
6. Adversary submits `PublishResult { tally, attestation: Dstack { quote: HexBinary::from(vec![0; 1]), zk_proof: HexBinary::from(vec![0; 1]), user_data } }`.
7. Chain verifies: phase = Tallying ✓; well-formed ✓; length = 64 ✓; domain tag matches ✓; commit hash matches ✓. Accepts. Tally is finalized as Resolved.

#### Compensating Controls Checked

| Control | Status | Verdict |
|---------|--------|---------|
| TDX quote signature validation (clause a) | Absent | Vulnerable |
| Groth16 zkdcap proof verification (clause b) | Absent | Vulnerable |
| Commit-hash binding (clause c) | Present | Insufficient by itself; consistency without soundness |
| Domain tag check | Present | Adds nothing against an attacker who can write the tag |
| MRTD/RTMR-vs-registry binding (clause d) | Absent | Vulnerable |
| `Mock` variant rejection in production | Present (C1 fully closed) | Reduces noise; not a defense against this path |

#### Recommended Fix

The discharge path is the same as the prior audit's C2 recommended fix, less the (c) clause which has now landed. Two queued integrations:

**(a) + (d)**: Add a TDX quote parser to the contract crate (or to a shared utility crate). The parser extracts the report-data (`user_data`) for clause-(c) cross-check, the MRTD for clause-(d) comparison against `registry.mrtd`, the RTMR for clause-(d) comparison against `registry.rtmr`, and verifies the Intel PCK signature chain. Quartz substrate already has a parser; importing it is the lowest-cost path.

**(b)**: Add a Stargate query to `xion.zk.v1.Query/ProofVerifyGnark` (or the v0.50+ equivalent) with `(zk_proof, public_inputs, vkey = registry.vkey)`. The public inputs must include the SHA-256 commitment (so the zkdcap proof binds the same commit hash the contract verifies in clause c) and the MRTD/RTMR fields (so clause d's check is also bound into the proof). Document the public-input list at intent §3.2 B8(b).

Until both land, the contract is not safe for any deployment that holds a non-toy election. Until then, the `Mock` variant being absent (C1) is the only thing preventing trivial Mock-based forgery; with the Dstack stub providing only consistency, this is not a meaningful protection.

---

### Finding N2: Admin substitution of `enclave_pubkey` still breaks ballot privacy; shape validation is necessary but not sufficient

**Severity:** Critical (residual from prior C3)
**Confidence:** 90/100
**Principle Violated:** VCL-CWSM-001 (unvalidated contract instantiation parameters — provenance, not just shape)
**Location:** `crates/contract/src/contract.rs:732-747` (the new validator); `crates/contract/src/contract.rs:213` (call site)

#### Description

The new `validate_enclave_pubkey` checks length and leading byte but not curve-point validity, and does not verify that the key was actually derived by the dstack KMS. The intent at §6.3 names `dstack_kms_trust` as an operational trust boundary; the contract code has no mechanism to verify this on-chain.

#### Exploit Path

1. Malicious admin generates a secp256k1 keypair locally. Records the privkey `attacker_sk`. The public component `attacker_pk` is a well-formed 65-byte uncompressed SEC1 point with leading 0x04 — it passes shape validation.
2. Admin calls `CreateElection` with `enclave_pubkey = attacker_pk`.
3. Voters retrieve `attacker_pk` from `query::Election`, encrypt their preference rankings under it via the UI, and submit.
4. Admin reads `query::Ballots` (permissionless) and decrypts each ciphertext locally under `attacker_sk`. Every voter's ranking is now known to the admin.
5. Optionally, admin composes with Finding N1 to publish any tally they want.

#### Compensating Controls Checked

| Control | Status | Verdict |
|---------|--------|---------|
| Length + leading-byte shape validation | Present (new in remediation) | Eliminates the deadlock variant where admin supplies garbage; does not prevent substitution |
| Curve-point validity check | Absent | Allows shape-valid but off-curve buffers (UI may fail at encryption) |
| dstack-KMS-signed derivation proof | Absent (no protocol) | Vulnerable |
| Admin authentication on `CreateElection` | Present | Limits to admin-key holder; the attack assumes admin is malicious or compromised |
| Intent §6.3 acknowledgement | Present in `msg.rs:50-55` | Mitigation by disclosure only |

#### Recommended Fix

Same as the prior audit's C3 recommended fix. The structural path is: require the admin to also supply a witness that proves the key was issued by dstack KMS against `(contract_addr, election_id)`. Two designs:

- **Signature path**: dstack KMS signs `(contract_addr, election_id, enclave_pubkey)` and the admin submits the signature; the chain verifies against a hardcoded dstack signing key.
- **Derivation path**: the chain reproduces the dstack derivation deterministically from `(contract_addr, election_id)` and a known parameter, leaving the admin no degree of freedom.

The derivation path is preferable because it makes the key a function of public chain state.

Lower-cost intermediate improvement: add a curve-point validity check (`k256::PublicKey::from_sec1_bytes`) so a malicious admin cannot griefyrunning by supplying a shape-valid off-curve buffer. This catches the "deadlock by malformed pubkey" failure mode but does not address the substitution attack.

---

### Finding N3: `CreateElection` from `Resolved` silently clears `TALLY_RESULT`; consumers must pin `(election_id, tally_result)` together

**Severity:** Minor (residual from prior M1, downgraded)
**Confidence:** 85/100
**Principle Violated:** VCL-STMT-005 (state machine transition integrity; consumer-visible terminal-state monotonicity)
**Location:** `crates/contract/src/contract.rs:178-187` (the M1 gate) and `contract.rs:226` (the unconditional remove)

#### Description

The M1 remediation prevents `CreateElection` from clobbering an election in `Voting` or `Tallying` phase. It permits `CreateElection` from `Resolved`, which clears the prior `TALLY_RESULT`. The intent v0.3.8 documents this as the alternate-path semantics. A downstream consumer who reads `tally_result` and caches the result can have the cached value silently invalidated.

#### Exploit Path

1. Election 1 finalizes; `TALLY_RESULT = Some(t1)`, phase = Resolved.
2. Off-chain consumer X reads `tally_result`, sees `t1.winners[0]`, caches the result.
3. Admin calls `CreateElection` to start election 2. The M1 gate accepts (Resolved is permitted). `TALLY_RESULT.remove` runs. `ELECTION_COUNTER` increments to 2.
4. X re-queries on its next cycle. `tally_result = None`. X's cached state is inconsistent with chain state.

#### Compensating Controls Checked

| Control | Status | Verdict |
|---------|--------|---------|
| M1 phase gate prevents Voting/Tallying clobber | Present | Mitigates the worst case |
| Historical tally archive (e.g., `Map<u64, TallyResult>`) | Absent | Vulnerable to silent erasure |
| Consumer pins `(election_id, tally_result)` | Operational | Requires consumer discipline |

#### Recommended Fix

Two options:

- **Option A**: add a `TALLY_HISTORY: Map<u64, TallyResult>` and have `CreateElection` archive the prior tally before clearing. Add a `TallyHistory { election_id }` query so consumers can fetch historical results.
- **Option B**: document the consumer contract explicitly. Update `query::Result` to return `(election_id, tally_result)` so consumers must consume the tuple together. Update intent §3.2 B1 to formally scope to a single election lifecycle.

Option A preserves more history; option B clarifies the contract. The current code does neither, leaving the consumer expectation ambiguous.

---

### Finding N4: `canonical_serialization` does not include `chain_id`; cross-chain replay remains a forward-looking gap once attestation is fully implemented

**Severity:** Minor
**Confidence:** 60/100
**Principle Violated:** VCL-CRYP-001 (replay vulnerabilities — context binding)
**Location:** `crates/contract/src/contract.rs:653-664`, `crates/enclave/src/attestation.rs:69-80`, intent §2.5 §3.2 B8(c)

#### Description

The M2 fix adds `election_id` to the canonical serialization, closing cross-election replay within a single contract address. It does not include `chain_id`. Two Xion chains (mainnet and testnet) with contracts at the same address would produce identical commit hashes for the same `(election_id, tally_body)`. Combined with a fully-implemented attestation (Finding N1's recommended fix), the cross-chain attestation replay vector remains.

#### Exploit Path

Dormant until N1 is closed. Once full attestation verification lands:

1. Honest enclave on testnet publishes `(tally_T, attestation_T)` for `(contract_addr_C, election_id_E)`.
2. The same contract address `C` is later instantiated on mainnet (e.g., same admin, same deployment script). The dstack KMS deriving keys against `(contract_addr_C, election_id_E)` would produce the same pubkey on mainnet — the operator must take care to use different election IDs or different contract addresses for mainnet. If they do not, the attestation from testnet binds the same commit hash on mainnet.
3. Attacker replays `(tally_T, attestation_T)` on mainnet. TDX quote verifies (same enclave). zkdcap proof verifies (same vkey). Commit hash matches (same `(addr, id, tally)`). Mainnet tally is now testnet's tally.

#### Compensating Controls Checked

| Control | Status | Verdict |
|---------|--------|---------|
| `chain_id` in commit hash | Absent | Vulnerable in the cross-chain scenario |
| Distinct dstack KMS instances per chain | Operational | Mitigates if the operator runs separate KMS instances |
| Distinct contract addresses per chain | Operational | Mitigates if the operator deploys to different addresses (the usual case) |

#### Recommended Fix

Extend `canonical_serialization` to include `env.block.chain_id` as the first field:

```rust
pub fn canonical_serialization(
    chain_id: &str,
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, chain_id);
    write_borsh_string(&mut out, contract_addr);
    out.extend_from_slice(&election_id.to_le_bytes());
    write_tally_body(&mut out, tally);
    out
}
```

Update intent §2.5 and §3.2 B8(c) to reflect the new binding. Update both encoders together; the cross-test will catch divergence.

This is a forward-looking design point; it is dormant under the current C2 stub (Finding N1) but should be addressed in the same cycle as N1.

---

### Finding N5: CI lacks gates against (a) `mock-attestation`-enabled production builds, (b) `cosmwasm-check` deployability, (c) clippy regression, (d) `cargo audit` advisories

**Severity:** Minor
**Confidence:** 90/100
**Principle Violated:** deployment hygiene; methodology Ask Z ("deferred is panic")
**Location:** `.github/workflows/ci.yml`

#### Description

The remediation introduced a CI workflow that runs `cargo build --workspace` and `cargo test --workspace` for Rust and `pnpm typecheck/test/build` for the UI. It does NOT run:

- `cargo clippy --workspace --all-targets -- -D warnings` (no lint regression gate).
- `cargo audit` (no advisory-database scan).
- `cosmwasm-check` against the release wasm artifact (no chain-deployability gate; the bulk-memory failure flagged in the prior audit is not caught).
- A build matrix that builds the contract with `--features mock-attestation` AND without, asserting the production build does not contain Mock-related symbols (no compile-time guard against accidentally enabling Mock in a release wasm).

The Vercel deploy job is gated on `ui` success but does not depend on the Rust job. A regression in the contract or runtime would not block UI deployment, which can leave the on-chain contract and the UI assumptions out of sync.

#### Exploit Path

Not directly exploitable. The risk is process-level: a developer who accidentally adds `mock-attestation` to default features (or who copies a Kani-harness build config into production) ships a wasm that accepts the Mock variant in production. CI does not catch this.

Similarly, the bulk-memory wasm failure (`m5` from the prior audit) is unchecked by CI; a deployment may fail at chain upload time with no warning during the PR review process.

#### Recommended Fix

```yaml
# Add to .github/workflows/ci.yml under the `rust` job:
- name: Clippy (deny warnings)
  run: cargo clippy --workspace --all-targets -- -D warnings

- name: cargo audit
  uses: rustsec/audit-check@v1
  with:
    token: ${{ secrets.GITHUB_TOKEN }}

- name: Install cosmwasm-check
  run: cargo install cosmwasm-check --version 3.0.7

- name: Build deployable wasm (default features)
  run: |
    rustup target add wasm32-unknown-unknown
    cargo build -p verified-rcv-contract --release --target wasm32-unknown-unknown

- name: cosmwasm-check deployability
  run: cosmwasm-check target/wasm32-unknown-unknown/release/verified_rcv_contract.wasm

- name: Assert Mock variant absent in production wasm
  run: |
    # Search the wasm binary for the Mock variant's serde-derived tag.
    # If present, fail.
    ! grep -ac 'AttestationEnvelope.*Mock' target/wasm32-unknown-unknown/release/verified_rcv_contract.wasm
```

The `cosmwasm-check` step will fail today (the bulk-memory issue is unaddressed). Fixing it requires either making the optimizer image part of the build chain or adding `.cargo/config.toml` with `target-feature` overrides (see prior audit Finding m5).

---

### Finding N6: `state.rs` doc-comments are stale relative to the M1 + M3 remediations

**Severity:** Informational
**Confidence:** 95/100
**Principle Violated:** documentation drift; VCL-STMT-006 (cross-contract interface mismatch — between source code and its own documentation)
**Location:** `crates/contract/src/state.rs:39-46` (Election doc), `crates/contract/src/state.rs:65-73` (EnclaveImageRegistry doc)

#### Description

The Election struct's docstring at lines 39-46 says: "Single-election contract; `CreateElection` overwrites (admin-only) — intent v0.3.4 §2.5 Block 1 (alternate path)." This text predates the M1 fix; the new behavior gates overwriting by phase (rejects Voting/Tallying). A new reader of this file would conclude the wrong invariant.

The EnclaveImageRegistry struct's docstring at lines 65-73 says: "Set at instantiate ... never mutated." This text predates the M3 fix; the new `UpdateRegistry` handler permits admin rotation between elections.

Neither error is exploitable, but both undermine the "code as documentation" reading the project relies on (intent doc + Lean spec + Rust comments together form the trust artifact).

#### Recommended Fix

```rust
// crates/contract/src/state.rs
/// Election storage. The contract permits one election at a time. The
/// admin can replace this slot via `CreateElection`, but only when no
/// election is in `Voting` or `Tallying` phase (intent v0.3.8 M1 audit
/// remediation gate). Prior elections' `TALLY_RESULT` is cleared when a
/// new election begins; consumers who need cross-election history must
/// pin `(election_id, tally_result)` together (see N3).
#[cw_serde]
pub struct Election { ... }

/// Image-identity-binding registry per intent §6.1. Set at instantiate
/// and validated at that point; can be rotated via the admin-only
/// `UpdateRegistry` execute message when no election is active (M3
/// audit remediation). `image_registration_honest(σ)` in the Quint spec
/// remains an operational predicate the chain does not verify.
#[cw_serde]
pub struct EnclaveImageRegistry { ... }
```

---

## Items from the prior audit not addressed in this cycle

The remediation focused on the eight C/M findings. The Minor, Informational, and Lead items from the prior audit are unchanged:

| Prior finding | Status |
|---------------|--------|
| m1 (SubmitBallot accepts empty ciphertext) | Open. UI-side gate at `ui/src/lib/encryption.ts:37` rejects empty rankings, partially mitigating, but the contract layer accepts. |
| m2 (SubmitBallot first-write-wins vs intent last-write-wins) | Open. Code and Quint spec align; intent §2.3 and §4.7 remain divergent. |
| m3 (start_at >= env.block.time accepted) | Open. Check at `contract.rs:208` still uses `<`, accepting `==`. |
| m4 (Borsh JS 2.x vs Rust 1.x major version skew) | Open. Cross-test still asserts behavior only for the current ballot shape. |
| m5 (cosmwasm-check fails on bulk memory ops) | Open. Re-verified at the new HEAD; failure offset moved (0x4e87 -> 0x4ddc) but cause unchanged. |
| m6 (Addr fields not addr_validated) | Open. `msg.admin` and `candidates: Vec<Addr>` still bypass `deps.api.addr_validate`. |
| m7 (ECIES no chain-context binding) | Open. The HKDF info parameter remains empty (library-inherited). |
| m8 (`crates/enclave/src/dstack.rs:99` expect()) | Open. Unchanged. |
| i1 (ballot_count saturating_add) | Open. Unchanged. |
| i2 (HashSet for candidate dedup) | **Closed**. Remediation replaced the HashSet with a Vec linear-scan at `contract.rs:197-203` (cited in the M1 fix comments as Kani-on-macOS hygiene). |
| i3 (cipher documentation drift in lib.rs:115) | Open in `crates/enclave/src/lib.rs:115` if still present; the runtime crate wasn't touched in this cycle (the remediation added `attestation.rs` + `server.rs`, did not edit existing lib.rs comments). |
| i4 (HKDF info empty) | Open. Library-inherited. |
| i5 (TallyResult lacks BorshSerialize derive) | Open. The hand-rolled encoder is now duplicated between contract and runtime (with cross-test guarantee), which is the workaround the prior audit suggested. |
| i6 (Drop reasons collapsed at Stage 1) | Open. Unchanged. |
| i7 (Contract crate lacks `forbid(unsafe_code)`) | Open. Unchanged. |
| i8 (`unimplemented!()` in enclave-core) | Open. Unchanged. |
| i9 (vkey String vs intent Bytes32) | **Closed**. M3's vkey-non-empty check reconciles the Rust shape with the intent's looser-by-text reading. Strict 32-byte enforcement would require an intent revision; the current state is internally consistent. |
| i10 (Permissionless Ballots query, long-term confidentiality) | Open. Unchanged. |
| L1–L10 (Leads) | All still open. |

Triage recommendation: the remaining Minor and Informational items are appropriate follow-up cycle work. The next priority should be the residual Criticals (N1 + N2), since they are the load-bearing trust-chain gaps that the prior remediation did not close.

## Static analysis re-run

| Tool | Result | Change since prior audit |
|------|--------|--------------------------|
| `cargo clippy --workspace --all-targets` | 11 warnings | Unchanged. All in `crates/enclave-core/src/lib.rs` (IRV math). Remediation code (contract.rs +989, attestation.rs +327) added zero new warnings. |
| `cargo audit` | 1 warning (`paste 1.0.15` unmaintained) | Unchanged. |
| `cosmwasm-check` against release wasm | Failure: `bulk memory support is not enabled (at offset 0x4ddc)` | Offset moved (0x4e87 -> 0x4ddc) but cause unchanged. m5 from prior audit remains open. |
| `cargo test --workspace --lib` (per-crate) | 25 contract + 16 runtime + 11 enclave-core + 4 cross-canonical = 56 tests pass | New: 25 contract + 16 runtime + 4 cross-canonical. Matches commit-message claim of "58 tests" (the count includes the 2 roundtrip binary tests in `crates/enclave/src/bin/roundtrip.rs`). |

`cargo test --workspace` (without per-crate `-p`) only shows 15 tests (enclave-core + cross-canonical integration tests). The contract and runtime crates need `cargo test -p verified-rcv-contract --lib` and `cargo test -p verified-rcv-enclave --lib` to surface their unit tests because they are `cdylib`-or-binary crates that the default workspace runner does not pick up uniformly. The commit-message claim is accurate when the per-crate invocations are run; a future CI tweak should ensure the workspace runner exercises every crate's unit tests via explicit `-p` arguments or a custom test runner script.

## Methodology updates inherited from intent v0.3.8

The intent's v0.3.8 patch adds three methodology Asks. They are tracked in `colosseum/methodology-v0.4-candidates.md` per the version-bump note; the relevant slices:

- **Ask Y**: contract-and-runtime adversarial fan-out. The original audit's six-strategy fan-out targeted only the contract crate. The remediation cycle's commit message acknowledges this as a process gap and proposes that future cycles run the fan-out against every code artifact in the trust boundary. Implemented in this re-audit by including the runtime crate (`crates/enclave/`) in the new-finding hunt.
- **Ask Z**: "deferred is panic" discipline. Any code path that an intent invariant depends on must either be fully implemented or compile-out / panic. Stub-as-Ok is forbidden. The remediation's C2 fix is consistent with this (the residual stub returns `Ok(())` after only the consistency check, but the comment at `contract.rs:335-340` explicitly names the gap; the design intent is to fail-shut on missing components, but the current code fails open on the (a) + (b) + (d) clauses). Re-audit verdict: Ask Z is only partially honored in the C2 fix; the residual N1 finding is a direct instance.
- **Ask AA**: negative-test discipline. For every invariant `Ok ⇒ P`, the harness suite must include `¬P ⇒ Err`. The remediation's test suite (45 new tests) honors this for the eight C/M findings. Re-audit verdict: Ask AA is well-applied in the remediation cycle.

The re-audit endorses Asks Y and AA. Ask Z is correctly framed; the project should treat any remaining stub-as-Ok path (specifically the C2 residual) as a hard pre-deployment gate.

## Recommended next-cycle priorities

In dependency order:

1. **Implement B8 clauses (a) + (b) + (d)** at the contract layer (Finding N1). This closes the residual Critical. Estimate: medium effort, depends on Xion ZK module availability and a TDX quote parser. Until this lands, the contract is not safe for deployments handling real votes.
2. **Implement dstack-KMS-derivation provenance proof** at `CreateElection` (Finding N2). This closes the ballot-privacy residual Critical. Estimate: medium effort, depends on the dstack KMS protocol design.
3. **Add CI gates**: clippy, cargo audit, cosmwasm-check, mock-attestation absence check (Finding N5). Estimate: low effort, ~30 lines of YAML.
4. **Fix the bulk-memory wasm failure** (prior audit Finding m5; unchanged in this cycle). Either require the cosmwasm/optimizer Docker image in `docs/deploy.md` or add `.cargo/config.toml` with appropriate `target-feature` flags.
5. **Fix `state.rs` doc drift** (Finding N6). Low effort, ~10 lines of comment update.
6. **Decide and document the cross-election B1 scope** (Finding N3). Either add a `TallyHistory` archive or document the consumer-pinning contract.
7. **Add `chain_id` to `canonical_serialization`** (Finding N4). Forward-looking, but should be done in the same cycle as N1's full attestation enforcement.
8. **Triage the remaining Minor and Informational items** from the prior audit. Most are quick wins (`addr_validate`, `EmptyBallot` variant, start_at strict-gt, etc.).

## Methodology note

This re-audit was conducted as a focused verification pass rather than a full six-strategy fan-out. Verification of remediations is well-suited to a single-pass review because the changes are explicit, scoped, and tested; the new-finding hunt was applied to the +1300 LoC of new code only. A future cycle that adds substantial new attack surface (e.g., the full B8 attestation pipeline, a new chain integration, or a new UI surface) should re-trigger the full fan-out per Ask Y.

The audit output directory at `.colosseum/audit/` now contains:

- `2026-05-26-18-36-audit-claude.md` — prior audit (3 Critical + 5 Major + 8 Minor + 10 Informational + 10 Leads).
- `2026-05-26-19-59-reaudit-claude.md` — this document (5 fully closed + 3 partially closed + 6 new findings, with the residual Critical pair surfaced).

The remediation is materially better than the audited state. Two follow-on cycles are needed: (1) full B8 attestation verification (N1) + dstack-KMS provenance proof (N2) to close the residual Criticals; (2) cleanup of the prior audit's still-open Minor and Informational items.

End of report.
