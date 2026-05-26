# verified-rcv enclave runtime integration brief

Brief for the agent / session working on `crates/enclave/`: gRPC tally service, dstack TDX boot integration, and **direct chain-side gnark verification** (replaces the old envelope wrapper as of v0.3.9). This is integration / glue work; it does not touch the math layer (already proven) or the IRV core (already extracted via Aeneas).

**Brief version: aligned with intent v0.3.10 / commit `4cc50b0` (2026-05-26).**

## What changed in v0.3.9 + v0.3.10

The audit re-review (N1) eliminated the `DstackEnvelope` wrapper. The chain now calls `xion.zk.v1.Query/ProofVerifyGnark` directly via `QueryRequest::Grpc` and extracts measurements + ReportData from the gnark public_inputs blob bytewise. The runtime no longer builds an envelope — it produces `(proof, public_inputs)` directly.

Concrete API changes affecting this crate:

| Surface | v0.3.8 form | v0.3.10 form |
|---|---|---|
| `attestation.rs` | `build_envelope(...)` returns `AttestationEnvelopeJson::Dstack { quote, zk_proof, user_data }` | `produce_publish_artifacts(identity, contract_addr, chain_id, election_id, tally)` and `produce_registration_artifacts(identity, enclave_pubkey)` each return `(proof, public_inputs)` |
| `server.rs` `TallyResponse` | `{ tally_json, attestation_json }` | `{ tally_json, proof, public_inputs }` |
| `server.rs` `TallyRequest` | `{ contract_addr, election_id, candidates, raw_ballots }` | `{ contract_addr, election_id, candidates, raw_ballots, chain_id }` (chain_id added v0.3.10 N4) |
| `server.rs` `TallyServiceImpl::new(dstack, EnvelopeConfig)` | takes prover URL | takes `EnclaveIdentity` (mrtd/rtmr*/tcb_status/timestamp) instead |
| `attestation.rs` `canonical_serialization(addr, election_id, tally)` | 3-arg | 4-arg: `(addr, chain_id, election_id, tally)` |
| `attestation.rs` `build_user_data` | existed | **removed** — `build_publish_report_data` produces the 64-byte ReportData directly |
| Real prover gate | `ZKDCAP_PROVER_URL` env var (HTTP) | `real-zkdcap` cargo feature; default emits synthetic stub |

## Where this fits (v0.3.10)

```
chain (verified-rcv contract on Xion)
   │
   │  Ballots query (every ciphertext for the election)
   ▼
off-chain orchestrator
   │
   │  gRPC: tally(contract_addr, chain_id, election_id, candidates, raw_ballots)
   ▼
enclave (this code) — inside a dstack-TDX VM
   │
   │  1. dstack KMS: derive privkey for (contract_addr, election_id)
   │  2. Stage 1 decrypt_and_validate (DONE)
   │  3. Stage 2 irv_spec via enclave-core (DONE)
   │  4. Construct TallyResult (DONE)
   │  5. Request TDX quote with report_data = build_publish_report_data(...)
   │  6. Drive zkdcap gnark prover → (proof, public_inputs)   [STUB / real-zkdcap]
   │
   │  gRPC response: (tally_json, proof, public_inputs)
   ▼
off-chain orchestrator
   │
   │  ExecuteMsg::PublishResult { tally, proof, public_inputs } → contract
   ▼
chain
   │
   │  Contract issues QueryRequest::Grpc to /xion.zk.v1.Query/ProofVerifyGnark
   │  Extracts measurements + ReportData from public_inputs bytewise
   │  Checks: gnark verifies, measurements == registry, ReportData[0..32] ==
   │          SHA-256(canonical_serialization(...)), ReportData[32..64] == DST
   │
   ▼
chain accepts or rejects
```

The work in this brief: **step 1 (KMS) + step 5 (TDX quote) + step 6 (gnark prover) + the gRPC layer around all of it.** Both registration (CreateElection) and publish (PublishResult) flows produce the same `(proof, public_inputs)` shape, differing only in the ReportData preimage:

- Publish: `ReportData[0..32] = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ tally_body))`, `ReportData[32..64] = DST_VERIFIED_RCV_TALLY_V1` (zero-padded to 32).
- Registration: `ReportData[0..32] = SHA-256(enclave_pubkey)`, `ReportData[32..64] = DST_VERIFIED_RCV_PUBKEY_V1` (zero-padded to 32).

Domain separation prevents cross-purpose replay between registration and publish quotes even though both use the same circuit + same vkey.

## Trust model — what the runtime MUST NOT do

- Re-implement IRV. Use `verified_rcv_enclave::tally_spec`.
- Cache or reorder `raw_ballots` other than the candidate-declaration-order projection (intent §2.5 Stage 1 iteration discipline).
- Skip the dstack KMS handshake. Privkey MUST come from dstack KMS for `(contract_addr, election_id)`.
- Embed any privkey, MRTD/RTMR, or attestation in the binary.
- **Construct or display `AttestationEnvelope` JSON.** That type is removed at v0.3.9; downstream consumers (UI, contract) work with the raw `(proof, public_inputs)` pair.

## What exists today (do not duplicate)

- **`crates/enclave/src/lib.rs::decrypt_and_validate`** — Real Stage 1 (ECIES + Borsh + permutation validation). 7 tests pass.
- **`crates/enclave/src/lib.rs::tally_spec`** — Stage 1 + Stage 2 composition.
- **`crates/enclave/src/main.rs`** — Off-TDX CLI (stdin/stdout). Debug mode.
- **`crates/enclave/src/bin/roundtrip.rs`** — ECIES/Borsh roundtrip CLI consumed by `ui/test/encryption.test.ts`.
- **`crates/enclave/src/bin/server.rs`** — gRPC server entry point. Reads identity from env vars (`MRTD`, `RTMR0..3`, `TCB_STATUS`, `ATTESTATION_TIMESTAMP`).
- **`crates/enclave/src/bin/smoke.rs`** — e2e smoke test that asserts the v0.3.9 public_inputs shape.
- **`crates/enclave/src/dstack.rs`** — `DstackClient` trait + `SimulatorDstackClient` impl. The real-dstack impl is a TODO; the simulator covers dev.
- **`crates/enclave/src/attestation.rs`** — `canonical_serialization`, `build_publish_report_data`, `build_registration_report_data`, `build_public_inputs`, `EnclaveIdentity`, `produce_publish_artifacts`, `produce_registration_artifacts`.
- **`crates/enclave/src/server.rs`** — `TallyServiceImpl` gRPC handler.
- **`crates/enclave/README.md`** — Pins the ECIES + Borsh wire format.

## Current proto (`crates/enclave/proto/tally.proto`)

```proto
syntax = "proto3";
package verified_rcv;

service TallyService {
  rpc Tally(TallyRequest) returns (TallyResponse);
  rpc Health(HealthRequest) returns (HealthResponse);
}

message TallyRequest {
  string contract_addr = 1;
  uint64 election_id = 2;
  repeated string candidates = 3;
  repeated RawBallot raw_ballots = 4;
  string chain_id = 5;                          // v0.3.10 N4
}

message RawBallot {
  string voter = 1;
  bytes ciphertext = 2;
}

message TallyResponse {
  string tally_json = 1;
  bytes proof = 2;                              // v0.3.9 N1
  bytes public_inputs = 3;                      // v0.3.9 N1; 9_792 bytes
}

message HealthRequest {}

message HealthResponse {
  bool ready = 1;
  string mrtd_hex = 2;                          // empty until TDX-quote parser lands
  string rtmr_hex = 3;
}
```

## Phase 1: gRPC tally service (DONE)

Already implemented in `src/server.rs`. The handler:

1. Calls `dstack.derive_privkey(contract_addr, election_id)` (Phase 2).
2. Walks `raw_ballots` and runs `tally_spec`.
3. Calls `produce_publish_artifacts(&self.identity, contract_addr, chain_id, election_id, tally)` (Phase 3).
4. Returns `TallyResponse { tally_json, proof, public_inputs }`.

Server bin reads identity from env at startup:

```bash
MRTD=<96-char hex>        # 48-byte TDX MRTD; required for production
RTMR0=<96-char hex>       # optional; chain skips if registry.rtmr0 = None
RTMR1=<96-char hex>       # required
RTMR2=<96-char hex>       # required
RTMR3=<96-char hex>       # optional
TCB_STATUS=<u8>           # 0..=5 (6=Revoked is circuit-rejected); default 0
ATTESTATION_TIMESTAMP=<u64>  # unix seconds; defaults to now
```

Unset values default to all-zeros / `0` / now — fine for `mock-attestation` builds where the chain skips the gnark verify; will fail measurement equality against a real registry.

## Phase 2: dstack TDX boot + KMS integration (PARTIAL)

### What exists

- `DstackClient` trait + `SimulatorDstackClient` (in-memory privkey from `DSTACK_DEV_PRIVKEY` env). 2 roundtrip tests pass.
- `build_client_from_env()` picks the simulator when `DSTACK_SIMULATOR=1` is set, otherwise expects a real socket path / vsock endpoint (NOT YET IMPLEMENTED).
- Health probe at server boot logs the dstack image identity.

### What's missing

- Real `DstackClient` implementation against a TDX-VM dstack socket. Reference: Quartz's `quartz-tee` crate at `/Users/mvid/Development/reliq/quartz/`. Mirror its transport.
- **TDX-quote parser** for `HealthResponse.mrtd_hex` / `rtmr_hex` (audit M5 partial fix returns empty strings rather than misleading compose_hash). Use `dcap-qvl` or a similar TDX quote decoder to extract MRTD + RTMRs from the boot quote.

## Phase 3: Attestation artifacts (DONE — synthetic; real prover gated)

### ReportData construction

```rust
// Publish quote
pub fn build_publish_report_data(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    tally: &TallyResult,
) -> [u8; 64] {
    let canonical = canonical_serialization(contract_addr, chain_id, election_id, tally);
    let commit = Sha256::digest(&canonical);
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(&commit);
    rd[32..32 + DST_TALLY.len()].copy_from_slice(b"DST_VERIFIED_RCV_TALLY_V1");
    rd
}

// Registration quote
pub fn build_registration_report_data(enclave_pubkey: &[u8]) -> [u8; 64] {
    let h = Sha256::digest(enclave_pubkey);
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(&h);
    rd[32..32 + DST_PUBKEY.len()].copy_from_slice(b"DST_VERIFIED_RCV_PUBKEY_V1");
    rd
}
```

The chain's contract reconstructs the same value byte-for-byte and compares.

### canonical_serialization (v0.3.10 layout)

```rust
pub fn canonical_serialization(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, contract_addr);  // u32_LE len + UTF-8 bytes
    write_borsh_string(&mut out, chain_id);       // v0.3.10 N4: cross-chain replay defense
    out.extend_from_slice(&election_id.to_le_bytes());  // u64 LE
    write_tally_body(&mut out, tally);            // declaration-order fields
    out
}
```

Cross-tested against the contract's identical implementation in `crates/enclave/tests/cross_canonical.rs` (8 tests).

### public_inputs construction (gnark layout)

```rust
pub fn build_public_inputs(
    mrtd: &[u8; 48],
    rtmr0: &[u8; 48], rtmr1: &[u8; 48], rtmr2: &[u8; 48], rtmr3: &[u8; 48],
    report_data: &[u8; 64],
    tcb_status: u8,
    timestamp: u64,
) -> Vec<u8> {
    // 306 BE fr-elements × 32 bytes = 9_792 bytes
    // Each uints.U8 byte sits at offset i*32 + 31 (high 31 bytes = 0)
    // TcbStatus + Timestamp: u64 BE at offset elem*32 + 24..32
    ...
}
```

Layout pinned at intent §2.5 gnark public_inputs byte layout (table normative). 306 fr-elements:
- `[0..48]` MrTd (48 elements)
- `[48..96]` Rtmr0
- `[96..144]` Rtmr1
- `[144..192]` Rtmr2
- `[192..240]` Rtmr3
- `[240..304]` ReportData (64 elements)
- `[304]` TcbStatus
- `[305]` Timestamp

Contract reads back via `extract_u8_from_fr(pi, elem_idx)` / `extract_u64_from_fr(pi, elem_idx)` with the `uints.U8` high-byte invariant enforced — wrong layout fails fast.

### Real gnark prover (`real-zkdcap` feature, NOT YET WIRED)

Default build (`default = []`): `produce_artifacts_inner` returns a 192-byte sentinel proof + synthetic public_inputs assembled from the configured `EnclaveIdentity`. The chain's `mock-attestation` feature skips the gnark verify and accepts. **Production builds (no mock-attestation) reject the sentinel proof** — that's the intended signal to compile in `--features real-zkdcap`.

`real-zkdcap` build (TODO): connect to the zkdcap Go prove server via unix socket per `/Users/mvid/Development/reliq/zkdcap/host/src/gnark.rs`. Steps:

1. `request_body = { quote_hex, pre_verified_json, timestamp }`
2. POST to `/prove` over the unix socket.
3. Parse returned proof JSON.
4. Extract `proof` (gnark-native Groth16 BN254 serialization) and `public_inputs` (concatenated 32-byte BE fr-elements).
5. Sanity-check `public_inputs.len() == 9_792` before returning.

The zkdcap host crate exposes `prove_quote(raw_tdx_quote)` for the SP1 path; the gnark path is currently exposed through the `dcap-gnark` Go binary as a separate prove server (oauth3 deploys this in a CVM at `/Users/mvid/Development/reliq/oauth3/contracts/...`). For verified-rcv, deploy the same gnark prove server in the same TDX VM (or alongside as a sibling service) and connect via unix socket.

## Open: `RegisterPubkey` gRPC RPC (NOT YET IMPLEMENTED)

CreateElection requires a *registration* `(proof, public_inputs)` pair, not a publish one. The current server.rs only exposes `Tally` (which produces publish artifacts). A `RegisterPubkey` RPC is needed so the admin can obtain the registration quote without side-channel coordination with the operator.

Shape:

```proto
message RegisterPubkeyRequest {
  string contract_addr = 1;
  uint64 election_id = 2;             // next-election id from contract counter
  // Optional: forces fresh privkey derivation if true
  bool fresh_key = 3;
}

message RegisterPubkeyResponse {
  bytes enclave_pubkey = 1;           // 33-byte compressed secp256k1
  bytes proof = 2;
  bytes public_inputs = 3;
}
```

Handler skeleton:
1. `privkey = dstack.derive_privkey(contract_addr, election_id)`.
2. `enclave_pubkey = secp256k1_pubkey_from_privkey(privkey)` (uncompressed → compressed serialization).
3. `produce_registration_artifacts(&self.identity, &enclave_pubkey).await`.
4. Return.

Until this RPC lands, admin flow requires manual coordination: operator runs the enclave with the desired identity, exports `(enclave_pubkey, proof, public_inputs)` once at deployment, admin pastes into a CreateElection tx. Document the manual path in `docs/deploy.md`.

## Cargo features

```toml
[features]
default = []
real-zkdcap = []                      # connects to zkdcap Go prove server
```

The chain crate (`verified-rcv-contract`) has a separate `mock-attestation` feature that gates the chain-side gnark verify. The combinations:

| Runtime feature | Contract feature | Behavior |
|---|---|---|
| default (no real-zkdcap) | default (no mock-attestation) | Runtime emits stub proof; chain rejects. Use only with `cfg(test)` chains. |
| default | mock-attestation | Runtime stub; chain skips gnark verify. Test / Kani / Verus / mock_dependencies. |
| real-zkdcap | default | Runtime real prover; chain real verify. **Production.** |
| real-zkdcap | mock-attestation | Real proof, chain doesn't verify it. Dev-on-real-chain. |

`cfg(test)` automatically enables the contract's mock path so unit tests don't need to mock the gRPC querier.

## Tests in this crate

1. **`server_smoke`** (server.rs) — gRPC server with `SimulatorDstackClient`, asserts response is well-shaped: tally winners + `public_inputs.len() == 9_792` + ReportData[0..32] equals expected commit hash.
2. **`canonical_serialization_byte_for_byte`** (attestation.rs) — hand-derived byte sequence for the minimal tally; pins layout.
3. **`canonical_serialization_chain_id_affects_bytes`** (cross_canonical.rs) — v0.3.10 N4 binding.
4. **`publish_report_data_layout`** + **`registration_report_data_layout`** — DST tag + commit/SHA-256 placement.
5. **`public_inputs_total_length`** + **`public_inputs_u8_invariant`** — gnark layout constraints.
6. **`canonical_serialization_contract_vs_runtime_byte_identical`** (cross_canonical.rs) — load-bearing equivalence test; the runtime's and contract's `canonical_serialization` MUST produce identical bytes.
7. **`commit_hash_matches_runtime_publish_report_data`** (cross_canonical.rs) — chain's `compute_commit_hash` lower 32 == runtime's `build_publish_report_data` lower 32.
8. **`synthetic_public_inputs_round_trip_contract_extraction`** (cross_canonical.rs) — runtime builds public_inputs; contract extracts measurements + ReportData; bytes match.

## Pre-implementation checklist for remaining work

Before wiring `real-zkdcap` or `RegisterPubkey`:

1. **Confirm zkdcap gnark prove server transport.** Read `/Users/mvid/Development/reliq/zkdcap/host/src/gnark.rs` (unix socket, HTTP-over-socket). Mirror its request shape.
2. **Confirm xion.zk vkey registration**. The gnark vkey for the verified-rcv circuit must be registered on Xion's `xion.zk` module under some `vkey_name`. Without it, `ProofVerifyGnark` returns "vkey not found." Coordinate with chain governance; record the chosen `vkey_name` in `docs/deploy.md`.
3. **TDX runtime version**. Confirm with the Quartz repo (`/Users/mvid/Development/reliq/quartz`) which dstack version is in use, what socket path / vsock channel it uses. Use the same.
4. **Borsh field ordering for `TallyResult`**. Confirmed in cross-test; if you ever touch `write_tally_body`, the cross-test will catch divergence from the chain side.

## What this work does NOT touch

- `crates/enclave-core/` — Aeneas-extractable IRV core, formally verified math. Do not modify.
- `specs/RcvSpec.lean` — math spec. 8 theorems proven; 1 sorry on B10_lean (separate multi-day Aeneas refinement).
- `crates/contract/src/contract.rs` — chain-side handlers. The v0.3.9+v0.3.10 changes already landed.

## Open question: Aeneas extractability of Stage 1

`crates/enclave/src/lib.rs::decrypt_and_validate` is still not Aeneas-extracted. The `ecies` and `borsh` crates probably do NOT extract cleanly via charon. Two paths:

- **(a) Hand-translate `decrypt_and_validate`'s body to Lean**, treating ECIES and Borsh decode as opaque axioms.
- **(b) Refactor to separate pure-Rust permutation logic from system-crypto** so the pure part extracts.

Path (b) is cleaner. Unblocks `B10_lean_decrypt`'s eventual discharge.

## Honest disclosure to your operator

v0.3.10 (commit `4cc50b0`) state:

**Deployable today (testnet, with caveats):**
- `crates/enclave/src/bin/server.rs` — gRPC `TallyService` (`verified-rcv-enclave-server`)
- `crates/enclave/src/bin/smoke.rs` — e2e smoke
- `ops/Dockerfile.enclave` + `ops/docker-compose.phala.yml` — Phala CVM compose template
- `ops/zkdcap-prover-shim/` — standalone HTTP shim around `zkdcap_host::prove_quote` (SP1 path; gnark path is separate)
- `ops/smoke-phala.sh` — three-mode smoke script

**Remaining operator-policy obligations (not chain-enforced):**

1. **`(mrtd, rtmr1, rtmr2, [rtmr0], [rtmr3])` registry honesty** (intent §6.1). Operator must register canonical values. The Health RPC returns empty `mrtd_hex` / `rtmr_hex` until a TDX-quote parser lands; until then, operators must extract MRTD/RTMR from dstack's image manifest or a separate side-channel TDX-quote inspection.
2. **`vkey_name` registration** in Xion's `xion.zk` VKey store. Operator must register the gnark verifier vkey via chain governance before any PublishResult will succeed.
3. **zkdcap prover identity**. The runtime currently emits a synthetic stub proof; switching to `--features real-zkdcap` requires standing up the gnark prove server (operator runs it; ideally in a CVM with its own attested identity).
4. **TDX MRTD/RTMR injection at boot**. Operator sets env vars (`MRTD`, `RTMR1`, `RTMR2`, optionally `RTMR0`/`RTMR3`). For real-TDX deployment, the operator script reads these from the boot record.
5. **`registry_update_delay_seconds` on instantiate** (v0.3.10 N2). Operator sets at deploy. Production should be ≥ 86400 (1 day) so voters can react to a proposed registry change.
6. **Aeneas-extractability of Stage 1** (open question above) — orthogonal to attestation; verification-layer item.
7. **`enclave_input_fidelity`** (intent §8.7 link 7) — still open. The chain binds the *output* (tally body) but not the *input* (ballots). A malicious host can substitute ballots; the chain would accept the resulting attested tally. Cheapest closure path documented in intent: extend `canonical_serialization` with `ballots_hash`.

## Pre-mainnet checklist

The contract code is ready (45 unit + 8 cross + 19 runtime + 11 math + 2 dstack tests; wasm builds; clippy clean). Blockers to mainnet:

1. **B1**: real gnark prover wired (`real-zkdcap` feature).
2. **B2**: vkey registered in xion.zk via Xion governance.
3. **B3**: TDX-quote parser populating `HealthResponse.mrtd_hex` / `rtmr_hex`.
4. **B4**: `B10_lean` discharge (multi-day Aeneas refinement).
5. **B5**: reproducible build (image-identity-binding §8.7 link 4).
6. **B6**: `enclave_input_fidelity` (§8.7 link 7).
7. **B7**: end-to-end Xion testnet smoke.
