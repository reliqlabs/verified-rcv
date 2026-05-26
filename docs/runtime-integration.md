# verified-rcv enclave runtime integration brief

Brief for an agent / session focused on completing the **non-verification** parts of `crates/enclave/`: gRPC tally service, dstack TDX boot integration, and Quartz attestation envelope construction. This is integration / glue work; it does **not** touch the math layer (already proven) or the IRV core (already extracted via Aeneas).

Spin up this work in a focused session that has access to dstack TDX runtime (or a TDX simulator for dev). This document is the handoff.

## Where this fits

```
chain (verified-rcv contract on Xion)
   │
   │  Ballots query (every ciphertext for the election)
   ▼
off-chain orchestrator
   │
   │  gRPC: tally(contract_addr, candidates, raw_ballots, end_at)
   ▼
enclave (this code) — inside a dstack-TDX VM
   │
   │  1. dstack KMS: derive privkey for (contract_addr, election_id)
   │  2. Stage 1 decrypt_and_validate (DONE)
   │  3. Stage 2 irv_spec via enclave-core (DONE)
   │  4. Construct TallyResult (DONE)
   │  5. Build AttestationEnvelope::Dstack (TODO)
   │
   │  gRPC response: (tally, attestation)
   ▼
off-chain orchestrator
   │
   │  PublishResult(tally, attestation) → contract
   ▼
chain
```

The work in this brief: **steps 1 (KMS request) + step 5 (envelope construction) + the gRPC layer around all of it**.

## Trust model — what the runtime MUST NOT do

The enclave's job is to **mirror the math** (which is formally verified) and **bind the result to the attested identity** (which is the dstack TDX + Quartz substrate's job). It must NOT:

- Re-implement IRV or do any tally logic locally. Use `verified_rcv_enclave::tally_spec`. The runtime's job is to plumb inputs/outputs around it, not to "improve" the math.
- Cache or transform `raw_ballots` other than the candidate-declaration-order projection. Any reordering or filtering breaks intent §2.5 Stage 1's iteration discipline (load-bearing).
- Skip the dstack KMS handshake. The privkey MUST come from dstack KMS for `(contract_addr, election_id)`, not from any other source. Even on dev/test paths, route through the KMS interface (which can have a simulator backend).
- Embed any privkey, MRTD/RTMR, or attestation in the binary. Everything per-deployment comes from the dstack runtime.

## What exists today (do not duplicate)

- **`crates/enclave/src/lib.rs::decrypt_and_validate`** — Real Stage 1 with ECIES + Borsh + permutation validation. 7 tests pass.
- **`crates/enclave/src/lib.rs::tally_spec`** — Stage 1 + Stage 2 composition. 1 end-to-end test passes.
- **`crates/enclave/src/main.rs`** — CLI that reads `{candidates, raw_ballots, privkey_hex}` from stdin, calls `tally_spec`, prints `TallyResult` JSON to stdout. This is the **OFF-TDX path**; keep it functional as a debug/test mode. The new gRPC service should be an additional binary/mode, not a replacement.
- **`crates/enclave/src/bin/roundtrip.rs`** — ECIES/Borsh roundtrip CLI used by `ui/test/encryption.test.ts`.
- **`crates/enclave/README.md`** — Pins the ECIES + Borsh wire format. The new code must honor this; it's the contract between the UI encoder and the enclave decoder.
- **`crates/contract/src/msg.rs::AttestationEnvelope`** — The on-chain envelope shape:
  ```rust
  pub enum AttestationEnvelope {
      Mock,
      Dstack {
          quote: HexBinary,    // TDX quote bytes
          zk_proof: HexBinary, // zkdcap Groth16 proof
          user_data: HexBinary // 64 bytes: 32 domain-tag + 32 commit-hash
      },
  }
  ```
  The runtime constructs the `Dstack` variant.

## Phase 1: gRPC tally service

### Proto file

Add `crates/enclave/proto/tally.proto`:

```proto
syntax = "proto3";

package verified_rcv;

// The orchestrator calls Tally with the chain-side election state; the
// enclave responds with the published-result tuple.
service TallyService {
  rpc Tally(TallyRequest) returns (TallyResponse);
  rpc Health(HealthRequest) returns (HealthResponse);
}

message TallyRequest {
  // The CosmWasm contract address (bech32). Bound into the attestation
  // user_data per intent §2.5 canonical_serialization pin.
  string contract_addr = 1;

  // Election ID. Used in the dstack KMS key-derivation path.
  uint64 election_id = 2;

  // Candidate addresses in declaration order. Order is load-bearing
  // (intent §2.5 Stage 1 iteration discipline).
  repeated string candidates = 3;

  // (voter, ciphertext) pairs as they appear in the chain's Ballots
  // query, REORDERED into candidate-declaration order by the caller
  // before this RPC. The runtime walks them linearly without further
  // reordering.
  repeated RawBallot raw_ballots = 4;
}

message RawBallot {
  string voter = 1;
  bytes ciphertext = 2; // ECIES wire format per crates/enclave/README.md
}

message TallyResponse {
  // The TallyResult per intent §2.5; serialized as JSON (the same shape
  // the contract's PublishResult message takes). Borsh-encoding is done
  // by the orchestrator when packaging the PublishResult message.
  string tally_json = 1;

  // The Quartz attestation envelope, serialized as JSON.
  // Shape: {"Dstack":{"quote":"hex","zk_proof":"hex","user_data":"hex"}}
  string attestation_json = 2;
}

message HealthRequest {}

message HealthResponse {
  // Returns whether the dstack KMS is reachable and the TDX boot has
  // produced a usable quote at startup.
  bool ready = 1;
  // The enclave's published MRTD (hex). Operators verify this matches
  // the on-chain registry.
  string mrtd_hex = 2;
  // The enclave's published RTMR (hex). Same purpose.
  string rtmr_hex = 3;
}
```

### Stack pin

- **`tonic`** 0.12 for gRPC server.
- **`prost`** 0.13 (comes with tonic) for proto compilation.
- **`tokio`** 1.40+ with `rt-multi-thread, macros, signal`.
- **`tracing`** + **`tracing-subscriber`** for structured logging.

Add a `bin/server.rs` that runs the gRPC server. The existing `main.rs` (stdin/stdout CLI) stays as a debug entry point.

### Implementation

```rust
// crates/enclave/src/server.rs (new module)
use tonic::{Request, Response, Status};
// ... generated module from tally.proto

pub struct TallyServiceImpl {
    dstack: DstackClient, // see Phase 2
}

#[tonic::async_trait]
impl tally::tally_service_server::TallyService for TallyServiceImpl {
    async fn tally(
        &self,
        req: Request<TallyRequest>,
    ) -> Result<Response<TallyResponse>, Status> {
        let req = req.into_inner();

        // 1. Derive privkey from dstack KMS for (contract_addr, election_id).
        let privkey = self.dstack
            .derive_privkey(&req.contract_addr, req.election_id)
            .await
            .map_err(|e| Status::internal(format!("kms derive: {e}")))?;

        // 2. Walk raw_ballots and compute Tally_spec.
        let raw = req.raw_ballots.into_iter()
            .map(|b| RawEntry { voter: b.voter, ciphertext: b.ciphertext })
            .collect();
        let candidates = req.candidates;
        let tally = verified_rcv_enclave::tally_spec(&raw, &candidates, &privkey);

        // 3. Build the attestation envelope per Phase 3.
        let envelope = self.build_envelope(&req.contract_addr, &tally).await?;

        Ok(Response::new(TallyResponse {
            tally_json: serde_json::to_string(&tally).unwrap(),
            attestation_json: serde_json::to_string(&envelope).unwrap(),
        }))
    }
    // ... health
}
```

## Phase 2: dstack TDX boot + KMS integration

### What dstack provides

- A TDX-attested VM environment with measured boot.
- A KMS that:
  - Knows the canonical verified-rcv image identity (MRTD/RTMR registered out-of-band).
  - Derives per-election privkeys deterministically from `(contract_addr, election_id)` plus the image identity. Privkeys ONLY release to enclaves matching that image.
  - Exposes the derivation via a unix socket / vsock / HTTP-over-VSOCK transport.
- A quote-signing endpoint that the enclave calls with `report_data` (≤ 64 bytes) to get a TDX quote signed by the platform.

### Reference implementation pattern

Look at Quartz's `quartz-tee` crate (in the parent repo at `/Users/mvid/Development/reliq/quartz/`) for the canonical dstack integration. Mirror the same transport + handshake.

```rust
// crates/enclave/src/dstack.rs
pub struct DstackClient {
    socket_path: PathBuf, // /run/dstack.sock (default)
}

impl DstackClient {
    pub async fn derive_privkey(
        &self,
        contract_addr: &str,
        election_id: u64,
    ) -> Result<[u8; 32], DstackError> {
        // Derivation context: include the canonical_serialization of
        // (contract_addr, election_id) per intent §2.5.
        let context = format!("verified-rcv-v1:{contract_addr}:{election_id}");
        // dstack request: derive_key(context, alg=secp256k1)
        // returns the 32-byte secret key (in a sealed transport).
        ...
    }

    pub async fn get_quote(&self, report_data: &[u8; 64]) -> Result<Vec<u8>, DstackError> {
        // dstack request: get_quote(report_data)
        // returns the TDX quote bytes (~4-5 KB).
        ...
    }

    pub async fn get_mrtd_rtmr(&self) -> Result<(Vec<u8>, Vec<u8>), DstackError> {
        // Read from the boot record; cache after first read.
        ...
    }
}
```

### Dev mode

When `DSTACK_SIMULATOR=1` is set, swap `DstackClient` for a `MockDstackClient` that:
- Reads a fixed privkey from `DSTACK_DEV_PRIVKEY` env var.
- Returns a hand-crafted "fake" quote with the expected shape (the on-chain `AttestationEnvelope::Mock` variant is the production path for dev). This is fine because the contract's `Mock` variant bypasses TDX verification.

Add `DSTACK_SIMULATOR=1` to the dev recipe in `docs/deploy.md`.

## Phase 3: Quartz attestation envelope construction

### Envelope construction

Per intent §3.2 B8(c), the `user_data` field of the TDX quote binds the enclave's output. Shape:

```
user_data (64 bytes) = upper_32_bytes (domain tag) ‖ lower_32_bytes (commit hash)
```

Where:
- `upper_32_bytes` = `b"DST_VERIFIED_RCV_TALLY_V1"` zero-padded to 32 bytes.
- `lower_32_bytes` = `SHA-256(canonical_serialization(contract_addr ‖ tally_body))`.
- `canonical_serialization` is Borsh per intent §2.5 — the same encoding the JS UI uses for ballot plaintexts. Use the `borsh` crate's `to_vec` for both `contract_addr` (Borsh String) and `tally_body` (the `TallyResult` struct).

```rust
fn build_user_data(contract_addr: &str, tally: &TallyResult) -> [u8; 64] {
    use sha2::{Digest, Sha256};
    use borsh::to_vec;

    const DST: &[u8] = b"DST_VERIFIED_RCV_TALLY_V1";
    let mut domain_tag = [0u8; 32];
    domain_tag[..DST.len()].copy_from_slice(DST);

    let mut canonical = Vec::new();
    canonical.extend_from_slice(&to_vec(&contract_addr.to_string()).unwrap());
    canonical.extend_from_slice(&to_vec(tally).unwrap());

    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let commit_hash = hasher.finalize();

    let mut user_data = [0u8; 64];
    user_data[..32].copy_from_slice(&domain_tag);
    user_data[32..].copy_from_slice(&commit_hash);
    user_data
}
```

### zkdcap proof generation

The TDX quote produced by dstack is the **inner** attestation. Quartz wraps it in a zkdcap Groth16 proof for the chain to verify (because direct TDX-quote verification on-chain is too expensive).

zkdcap proof generation: call out to a separate `zkdcap-prover` service that takes the TDX quote bytes and returns the Groth16 proof bytes. This service is part of the Quartz substrate (not built in this crate). Reference: `/Users/mvid/Development/reliq/quartz/circuits/dcap-gnark/`.

```rust
async fn generate_zkdcap_proof(
    quote_bytes: &[u8],
    prover_endpoint: &str, // e.g., http://localhost:9001
) -> Result<Vec<u8>, AttestationError> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{prover_endpoint}/prove"))
        .body(quote_bytes.to_vec())
        .send()
        .await?;
    Ok(response.bytes().await?.to_vec())
}
```

### Final envelope

```rust
async fn build_envelope(
    &self,
    contract_addr: &str,
    tally: &TallyResult,
) -> Result<AttestationEnvelope, AttestationError> {
    let user_data = build_user_data(contract_addr, tally);
    let quote = self.dstack.get_quote(&user_data).await?;
    let zk_proof = generate_zkdcap_proof(&quote, &self.prover_endpoint).await?;

    Ok(AttestationEnvelope::Dstack {
        quote: HexBinary::from(quote),
        zk_proof: HexBinary::from(zk_proof),
        user_data: HexBinary::from(user_data.to_vec()),
    })
}
```

## Cargo.toml additions

```toml
[dependencies]
# existing deps already present...

tonic = "0.12"
prost = "0.13"
tokio = { version = "1.40", features = ["rt-multi-thread", "macros", "signal", "net"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
sha2 = "0.10"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }

[build-dependencies]
tonic-build = "0.12"

[[bin]]
name = "verified-rcv-enclave-server"
path = "src/bin/server.rs"
```

Add `build.rs`:
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/tally.proto")?;
    Ok(())
}
```

## Tests to write

1. **`server_smoke`** — start the gRPC server with `DSTACK_SIMULATOR=1`, send a `Tally` RPC with the 3-candidate roundtrip-test fixture, assert the response contains a valid `TallyResult` JSON and an envelope with the expected `user_data` shape (64 bytes = 32 tag + 32 hash).

2. **`user_data_binding_matches_intent`** — for a known `(contract_addr, tally)`, compute `build_user_data` and assert:
   - Bytes 0..25 equal `b"DST_VERIFIED_RCV_TALLY_V1"`.
   - Bytes 25..32 are zero (padding).
   - Bytes 32..64 are the SHA-256 of the Borsh-encoded canonical serialization.

3. **`mock_dstack_derive_privkey_deterministic`** — same `(contract_addr, election_id)` produces the same privkey across calls (idempotent derivation).

4. **`tdx_quote_user_data_matches`** — round-trip the quote through `dstack.get_quote(ud)` and verify the returned quote's `report_data` field decodes back to `ud`. (Quote-parsing utility from Quartz can be reused.)

## Pre-implementation checklist

Before writing any code, lock down:

1. **dstack runtime version and transport**. Confirm with the Quartz repo (`/Users/mvid/Development/reliq/quartz`) which dstack version is in use, what socket path / vsock channel it uses, and what the request/response protocol looks like. Use the same.
2. **zkdcap prover endpoint**. Is there a centrally-operated one, or does each operator run their own? Pin to the Quartz reference.
3. **Borsh field ordering for `TallyResult`**. Confirm by computing `to_vec(&TallyResult { ... })` and comparing against intent §2.5's canonical-serialization pin (Vec-typed fields, declaration-order, leaf encodings per v0.3.1 T7).
4. **Domain-separation tag value**. `b"DST_VERIFIED_RCV_TALLY_V1"` is what intent §3.2 B8(c) names. Cross-check the exact byte literal once with intent §3.2.

## What this work does NOT touch

- `crates/enclave-core/` — Aeneas-extractable IRV core, formally verified math. Do not modify.
- `specs/RcvSpec.lean` — math spec. 8 theorems proven.
- `crates/contract/src/contract.rs` — the on-chain handlers. The chain-side `PublishResult` already accepts `AttestationEnvelope::Dstack` with the shape constructed here.

## Open question: Aeneas extractability of Stage 1

Once this Phase 1+2+3 work lands, a future verification pass will want to Aeneas-extract `crates/enclave/src/lib.rs::decrypt_and_validate` so `B10_lean_decrypt` can be proven (it's currently a `True` placeholder axiom in `specs/EnclaveBridge.lean`). The `ecies` and `borsh` crates probably do NOT extract cleanly via charon — they use system crypto and macro-derived implementations. Two paths:

- **(a) Hand-translate `decrypt_and_validate`'s body to Lean**, treating ECIES and Borsh decode as opaque axioms with appropriate spec lemmas.
- **(b) Refactor `decrypt_and_validate` to separate the pure-Rust logic (permutation validation) from the system-crypto calls (ECIES decrypt, Borsh decode)**. The pure-Rust portion can be Aeneas-extracted; the crypto stays opaque.

Path (b) is cleaner. If you have spare capacity after Phase 1+2+3, that refactor unblocks `B10_lean_decrypt`'s eventual discharge.

## Honest disclosure to your operator

Until Phase 3 lands, the contract-side `AttestationEnvelope::Dstack` path is not exercisable end-to-end and the trust chain reduces to `AttestationEnvelope::Mock`. The Mock path is fine for development and CI; production deployments MUST gate on Phase 1+2+3 being completed and the resulting (mrtd, rtmr) being cross-verified by independent operators (per `docs/image-identity-binding.md`).
