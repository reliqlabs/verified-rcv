//! Attestation construction — UltraHonk form.
//!
//! ## What this module produces
//!
//! Two artifacts the chain consumes via direct
//! `/xion.zk.v1.Query/ProofVerifyUltraHonk` from `verified-rcv`'s contract:
//!
//! - `proof: Vec<u8>` — UltraHonk (bb) proof bytes.
//! - `public_inputs: Vec<u8>` — 544-byte packed blob per the dcap-noir
//!   layout (17 BN254 fields × 32 bytes).
//!
//! `public_inputs` carries `MrTd ‖ Rtmr0..3 ‖ ReportData ‖ TcbStatus ‖
//! Timestamp ‖ cert_serial ‖ fmspc`, packed limb-wise (the circuit's
//! pack_be output). `ReportData` is 64 bytes, split into two purpose-tagged
//! halves per the §2.5 ReportData layout:
//!
//! - Publish quote: `SHA-256(canonical_serialization(contract_addr ‖
//!   election_id ‖ tally_body)) ‖ DST_VERIFIED_RCV_TALLY_V1` (zero-padded).
//! - Registration quote: `SHA-256(enclave_pubkey) ‖
//!   DST_VERIFIED_RCV_PUBKEY_V1` (zero-padded).
//!
//! ## `canonical_serialization`
//!
//! Intent §2.5 v0.3.1 T7 leaf-encoding pin. Hand-rolled so the encoding
//! is auditable side-by-side with the intent text and so we don't have
//! to touch the formally-verified enclave-core crate for a derive.
//!
//! ## Real-prover gate
//!
//! Under the default build, `produce_publish_artifacts` /
//! `produce_registration_artifacts` return synthetic `(proof,
//! public_inputs)` in the packed dcap-noir layout — the proof bytes are a
//! fixed sentinel and the `public_inputs` is built locally from the
//! caller-supplied identity tuple via [`build_public_inputs`]. Under
//! `--features real-zkdcap`, the runtime instead POSTs the TDX quote +
//! Intel PCS collateral to the noir/bb prove server (default socket
//! `/run/noir/prove.sock`, env `ZKDCAP_PROVER_SOCKET`), which builds the
//! Noir witness, runs `bb prove`, and returns the proof together with the
//! circuit's PACKED public_inputs. On that path the enclave does NOT build
//! public_inputs — bb emits them — but the produced blob is re-checked
//! against the bound ReportData + timestamp before use.

use sha2::{Digest, Sha256};
use thiserror::Error;

use verified_rcv_enclave_core::{RoundCount, RoundCounts, TallyResult};

use crate::dstack::DstackError;

/// 32-byte domain-separation tag for publish quotes (zero-padded). 25 ASCII bytes.
pub const DST_TALLY: &[u8] = b"DST_VERIFIED_RCV_TALLY_V1";
/// 32-byte domain-separation tag for registration quotes (zero-padded). 26 ASCII bytes.
pub const DST_PUBKEY: &[u8] = b"DST_VERIFIED_RCV_PUBKEY_V1";
/// v0.3.14 F2 DST prefix for `compute_ballots_hash` preimage. 23 ASCII
/// bytes, no length prefix. MUST stay byte-identical to the contract's
/// `contract::DST_BALLOTS`.
pub const DST_BALLOTS: &[u8] = b"verified-rcv:ballots:v1";
/// v0.3.14 F2 DST prefix for `compute_names_hash` preimage. 21 ASCII
/// bytes, no length prefix. MUST stay byte-identical to the contract's
/// `contract::DST_NAMES`.
pub const DST_NAMES: &[u8] = b"verified-rcv:names:v1";

/// Total `public_inputs` byte length (dcap-noir packed UltraHonk layout:
/// 17 BN254 fields × 32 bytes).
pub const ULTRAHONK_PUBLIC_INPUTS_LEN: usize = 17 * 32;

#[derive(Debug, Error)]
pub enum AttestationError {
    #[error("dstack: {0}")]
    Dstack(#[from] DstackError),
    #[error("zkdcap prover: {0}")]
    ZkProver(String),
}

// ---------------------------------------------------------------------------
// canonical_serialization (intent §2.5 v0.3.1 T7 leaf pin)
// ---------------------------------------------------------------------------

/// Borsh-style canonical serialization of `(contract_addr ‖ chain_id ‖
/// election_id ‖ ballots_hash ‖ names_hash ‖ tally_body)`. The `chain_id`
/// field was added at v0.3.10 (N4) for cross-chain replay defense; the
/// `ballots_hash` field was added at v0.3.11 (B6) to close §8.7 link 7
/// (enclave_input_fidelity); the `names_hash` field was added at v0.3.14
/// to bind admin-supplied candidate display names (B11).
pub fn canonical_serialization(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    names_hash: &[u8; 32],
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, contract_addr);
    write_borsh_string(&mut out, chain_id);
    out.extend_from_slice(&election_id.to_le_bytes());
    out.extend_from_slice(ballots_hash);
    // v0.3.14: names_hash sits between ballots_hash and tally_body.
    // 32 raw bytes (no length prefix, fixed size).
    out.extend_from_slice(names_hash);
    write_tally_body(&mut out, tally);
    out
}

/// v0.3.14: compute `SHA-256(u32_LE(names.len()) ‖ for name in names: Borsh(name))`
/// over the admin-supplied candidate display names in declaration order.
///
/// MUST stay byte-identical to the contract's
/// `verified_rcv_contract::contract::compute_names_hash` — cross-tested in
/// `crates/enclave/tests/cross_canonical.rs::compute_names_hash_contract_vs_runtime_byte_identical`.
pub fn compute_names_hash(candidate_names: &[String]) -> [u8; 32] {
    let mut preimage = Vec::new();
    // v0.3.14 F2: DST prefix domain-separates the names-hash preimage
    // from the ballots-hash preimage. 22 ASCII bytes, no length prefix.
    preimage.extend_from_slice(DST_NAMES);
    preimage.extend_from_slice(&(candidate_names.len() as u32).to_le_bytes());
    for name in candidate_names {
        write_borsh_string(&mut preimage, name);
    }
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    hasher.finalize().into()
}

/// v0.3.11 B6 — compute SHA-256 over the enclave-side view of the
/// (voter, ciphertext) pairs the runtime actually consumed.
///
/// `entries` is the raw_ballots vector the orchestrator passed in,
/// in candidate-declaration order. Encoding: u32 LE count + for each
/// included entry, Borsh-encoded `(voter: String, ciphertext: bytes)`.
///
/// MUST stay byte-identical to the chain's `compute_ballots_hash` so
/// the resulting commit_hash matches under chain-side verification.
pub fn compute_ballots_hash(
    candidates: &[String],
    entries: &[(String, Vec<u8>)],
) -> [u8; 32] {
    let mut body = Vec::new();
    let mut included: u32 = 0;
    for cand in candidates {
        for (voter, ct) in entries {
            if voter == cand {
                write_borsh_string(&mut body, voter);
                write_borsh_bytes(&mut body, ct);
                included = included.saturating_add(1);
                break;
            }
        }
    }
    // v0.3.14 F2: DST prefix on the ballots-hash preimage. 23 ASCII bytes.
    let mut preimage = Vec::with_capacity(DST_BALLOTS.len() + 4 + body.len());
    preimage.extend_from_slice(DST_BALLOTS);
    preimage.extend_from_slice(&included.to_le_bytes());
    preimage.extend_from_slice(&body);
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    hasher.finalize().into()
}

fn write_borsh_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn write_borsh_string(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn write_vec_addr(out: &mut Vec<u8>, v: &[String]) {
    out.extend_from_slice(&(v.len() as u32).to_le_bytes());
    for s in v {
        write_borsh_string(out, s);
    }
}

fn write_round_count(out: &mut Vec<u8>, rc: &RoundCount) {
    write_borsh_string(out, &rc.candidate);
    out.extend_from_slice(&(rc.count as u64).to_le_bytes());
}

fn write_round_counts(out: &mut Vec<u8>, rcs: &RoundCounts) {
    out.extend_from_slice(&(rcs.len() as u32).to_le_bytes());
    for rc in rcs {
        write_round_count(out, rc);
    }
}

fn write_per_round_counts(out: &mut Vec<u8>, prc: &[RoundCounts]) {
    out.extend_from_slice(&(prc.len() as u32).to_le_bytes());
    for round in prc {
        write_round_counts(out, round);
    }
}

fn write_eliminated_by_round(out: &mut Vec<u8>, ebr: &[Vec<String>]) {
    out.extend_from_slice(&(ebr.len() as u32).to_le_bytes());
    for round in ebr {
        write_vec_addr(out, round);
    }
}

fn write_tally_body(out: &mut Vec<u8>, t: &TallyResult) {
    write_vec_addr(out, &t.winners);
    write_per_round_counts(out, &t.per_round_counts);
    write_eliminated_by_round(out, &t.eliminated_by_round);
    out.extend_from_slice(&(t.ballots_tallied as u64).to_le_bytes());
    out.extend_from_slice(&(t.ballots_dropped as u64).to_le_bytes());
    write_vec_addr(out, &t.dropped_voters);
    write_vec_addr(out, &t.non_voters);
}

// ---------------------------------------------------------------------------
// ReportData construction (intent §2.5 ReportData layout, v0.3.9)
// ---------------------------------------------------------------------------

/// 64-byte ReportData for a publish quote: lower 32 = commit_hash, upper
/// 32 = DST_VERIFIED_RCV_TALLY_V1 zero-padded. v0.3.10 (N4) added
/// `chain_id`; v0.3.11 (B6) added `ballots_hash` for input-fidelity
/// binding; v0.3.14 added `names_hash` for candidate-display-name
/// binding (B11).
pub fn build_publish_report_data(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    names_hash: &[u8; 32],
    tally: &TallyResult,
) -> [u8; 64] {
    let canonical = canonical_serialization(
        contract_addr,
        chain_id,
        election_id,
        ballots_hash,
        names_hash,
        tally,
    );
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let commit = hasher.finalize();
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(&commit);
    rd[32..32 + DST_TALLY.len()].copy_from_slice(DST_TALLY);
    rd
}

/// 64-byte ReportData for a registration quote.
///
/// v0.3.11 form: `SHA-256(enclave_pubkey)` only.
/// v0.3.12 (N22) form: `SHA-256(enclave_pubkey ‖ borsh_string(contract_addr) ‖ u64_LE(election_id))`.
/// v0.3.14 (F1) form: `SHA-256(enclave_pubkey ‖ borsh_string(contract_addr) ‖ u64_LE(election_id) ‖ names_hash)`.
/// The election_id binding blocks an admin replaying an old registration
/// quote for a new election; the names_hash binding (v0.3.14) blocks an
/// admin from swapping candidate display names between CreateElection
/// and PublishResult. Preimage layout must stay byte-identical to the
/// contract's `verified_rcv_contract::contract::build_registration_report_data`
/// — `tests/cross_canonical.rs::registration_report_data_dst_matches_contract_layout`
/// is the load-bearing equality cross-test.
pub fn build_registration_report_data(
    enclave_pubkey: &[u8],
    contract_addr: &str,
    election_id: u64,
    names_hash: &[u8; 32],
) -> [u8; 64] {
    // v0.3.14 F1 form: ReportData[0..32] = SHA-256(enclave_pubkey ‖
    // borsh_string(contract_addr) ‖ u64_LE(election_id) ‖ names_hash).
    // Names binding extends N22's election binding so registration-time
    // name tampering is also detectable. The chain verifies against THIS
    // form; a pubkey-only-hash quote is detected and surfaced as
    // RegistrationQuoteWrongElection.
    let mut preimage =
        Vec::with_capacity(enclave_pubkey.len() + contract_addr.len() + 16 + 32);
    preimage.extend_from_slice(enclave_pubkey);
    write_borsh_string(&mut preimage, contract_addr);
    preimage.extend_from_slice(&election_id.to_le_bytes());
    preimage.extend_from_slice(names_hash);
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    let h = hasher.finalize();
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(&h);
    rd[32..32 + DST_PUBKEY.len()].copy_from_slice(DST_PUBKEY);
    rd
}

// ---------------------------------------------------------------------------
// public_inputs construction (dcap-noir packed UltraHonk layout)
// ---------------------------------------------------------------------------
//
// 17 BN254 fields x 32 BE bytes = 544 bytes. The Noir circuit packs K (<=31)
// bytes into one field via pack_be, so a K-byte limb occupies the LOW K bytes
// of its 32-byte field. Field order mirrors the contract's `contract.rs`:
//   0-1 mr_td; 2-3 rtmr0; 4-5 rtmr1; 6-7 rtmr2; 8-9 rtmr3 (each 31 + 17);
//   10-12 report_data (31 + 31 + 2); 13 tcb_status (low byte); 14 timestamp
//   (low 8 bytes u64 BE); 15 cert_serial (20B); 16 fmspc (6B).

const FR_BYTES: usize = 32;
const F_MRTD: usize = 0;
const F_RTMR0: usize = 2;
const F_RTMR1: usize = 4;
const F_RTMR2: usize = 6;
const F_RTMR3: usize = 8;
const F_REPORTDATA: usize = 10;
const F_TCBSTATUS: usize = 13;
const F_TIMESTAMP: usize = 14;

/// Write a big-endian limb into field `f` (low `bytes.len()` bytes; high
/// 32-len(bytes) stay zero). Mirror of the dcap-noir pack_be output.
fn put_limb(out: &mut [u8], f: usize, bytes: &[u8]) {
    let end = f * FR_BYTES + FR_BYTES;
    out[end - bytes.len()..end].copy_from_slice(bytes);
}

/// Low `k` bytes of field `f`, asserting the high 32-k bytes are zero (the
/// pack_be injectivity invariant). `None` on a short blob or non-canonical
/// limb.
fn read_limb(pi: &[u8], f: usize, k: usize) -> Option<&[u8]> {
    let fb = pi.get(f * FR_BYTES..f * FR_BYTES + FR_BYTES)?;
    if fb[..FR_BYTES - k].iter().any(|b| *b != 0) {
        return None;
    }
    Some(&fb[FR_BYTES - k..])
}

/// Build a 544-byte packed `public_inputs` blob in the dcap-noir layout.
/// Each measurement register packs into 2 limbs (31 + 17); report_data into
/// 3 limbs (31 + 31 + 2); tcb_status rides the low byte of field 13;
/// timestamp the low 8 bytes (u64 BE) of field 14. cert_serial + fmspc are
/// left zero (verified-rcv does not gate on them).
///
/// Under `default` features this synthesizes a chain-acceptable payload
/// without invoking the prover. Under `real-zkdcap`, `public_inputs` comes
/// straight from `bb` instead (the circuit emits the same packed layout);
/// this helper is the synthetic/default builder and the cross-test companion.
#[allow(clippy::too_many_arguments)]
pub fn build_public_inputs(
    mrtd: &[u8; 48],
    rtmr0: &[u8; 48],
    rtmr1: &[u8; 48],
    rtmr2: &[u8; 48],
    rtmr3: &[u8; 48],
    report_data: &[u8; 64],
    tcb_status: u8,
    timestamp: u64,
) -> Vec<u8> {
    let mut out = vec![0u8; ULTRAHONK_PUBLIC_INPUTS_LEN];
    for (reg, m) in [
        (F_MRTD, mrtd),
        (F_RTMR0, rtmr0),
        (F_RTMR1, rtmr1),
        (F_RTMR2, rtmr2),
        (F_RTMR3, rtmr3),
    ] {
        put_limb(&mut out, reg, &m[..31]);
        put_limb(&mut out, reg + 1, &m[31..48]);
    }
    put_limb(&mut out, F_REPORTDATA, &report_data[0..31]);
    put_limb(&mut out, F_REPORTDATA + 1, &report_data[31..62]);
    put_limb(&mut out, F_REPORTDATA + 2, &report_data[62..64]);
    out[F_TCBSTATUS * FR_BYTES + FR_BYTES - 1] = tcb_status;
    put_limb(&mut out, F_TIMESTAMP, &timestamp.to_be_bytes());
    out
}

/// Extract the 64-byte ReportData from a packed `public_inputs` (fields
/// 10..=12). `None` on a malformed blob. Used by the `real-zkdcap` path to
/// defensively confirm the prover's output carries the bound ReportData,
/// and by the server tests to read the ReportData back out.
pub fn extract_report_data(pi: &[u8]) -> Option<[u8; 64]> {
    if pi.len() != ULTRAHONK_PUBLIC_INPUTS_LEN {
        return None;
    }
    let mut rd = [0u8; 64];
    rd[0..31].copy_from_slice(read_limb(pi, F_REPORTDATA, 31)?);
    rd[31..62].copy_from_slice(read_limb(pi, F_REPORTDATA + 1, 31)?);
    rd[62..64].copy_from_slice(read_limb(pi, F_REPORTDATA + 2, 2)?);
    Some(rd)
}

/// Quote timestamp (u64 BE in the low 8 bytes of field 14). `None` on a
/// malformed blob.
pub fn extract_timestamp(pi: &[u8]) -> Option<u64> {
    if pi.len() != ULTRAHONK_PUBLIC_INPUTS_LEN {
        return None;
    }
    let limb = read_limb(pi, F_TIMESTAMP, 8)?;
    Some(u64::from_be_bytes(limb.try_into().ok()?))
}

// ---------------------------------------------------------------------------
// Artifact production: (proof, public_inputs) for the chain.
// ---------------------------------------------------------------------------

/// Identity tuple the enclave attests over. Matches the registry's chain
/// representation. Optional rtmr0/rtmr3 fall back to a zero buffer when
/// unbound — the chain's `verify_measurements_match_registry` skips
/// those fields when its registry's slot is `None`.
#[derive(Debug, Clone)]
pub struct EnclaveIdentity {
    pub mrtd: [u8; 48],
    pub rtmr0: [u8; 48],
    pub rtmr1: [u8; 48],
    pub rtmr2: [u8; 48],
    pub rtmr3: [u8; 48],
    pub tcb_status: u8,
    pub timestamp: u64,
}

/// Synthesize a publish-quote `(proof, public_inputs)` pair. Default
/// build: the proof is a 192-byte sentinel and the dstack client is
/// unused (the chain's `mock-attestation` build skips cryptographic
/// verify). Real build (`--features real-zkdcap`): drives the noir/bb
/// prove server via unix socket and binds a dstack-signed TDX quote.
#[allow(clippy::too_many_arguments)]
pub async fn produce_publish_artifacts(
    dstack: &dyn crate::dstack::DstackClient,
    identity: &EnclaveIdentity,
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    candidate_names: &[String],
    tally: &TallyResult,
) -> Result<(Vec<u8>, Vec<u8>), AttestationError> {
    // v0.3.14: compute names_hash from the runtime-received candidate_names
    // list. If the host substitutes names mid-flight, the resulting hash
    // diverges from the chain's stored Election.candidate_names and the
    // publish attestation rejects with AttestationCommitMismatch.
    let names_hash = compute_names_hash(candidate_names);
    let report_data = build_publish_report_data(
        contract_addr,
        chain_id,
        election_id,
        ballots_hash,
        &names_hash,
        tally,
    );
    produce_artifacts_inner(dstack, identity, &report_data).await
}

/// Synthesize a registration-quote `(proof, public_inputs)` pair.
/// v0.3.12 N22: binds `(contract_addr, election_id)` so an admin can't
/// replay an old registration quote for a new election.
/// v0.3.14 F1: also binds `names_hash` (computed internally from
/// `candidate_names`) so registration-time name tampering is detectable.
/// The runtime computes `names_hash` here so callers do not need to
/// import the helper themselves.
pub async fn produce_registration_artifacts(
    dstack: &dyn crate::dstack::DstackClient,
    identity: &EnclaveIdentity,
    enclave_pubkey: &[u8],
    contract_addr: &str,
    election_id: u64,
    candidate_names: &[String],
) -> Result<(Vec<u8>, Vec<u8>), AttestationError> {
    let names_hash = compute_names_hash(candidate_names);
    let report_data =
        build_registration_report_data(enclave_pubkey, contract_addr, election_id, &names_hash);
    produce_artifacts_inner(dstack, identity, &report_data).await
}

#[cfg(not(feature = "real-zkdcap"))]
async fn produce_artifacts_inner(
    _dstack: &dyn crate::dstack::DstackClient,
    identity: &EnclaveIdentity,
    report_data: &[u8; 64],
) -> Result<(Vec<u8>, Vec<u8>), AttestationError> {
    let public_inputs = build_public_inputs(
        &identity.mrtd,
        &identity.rtmr0,
        &identity.rtmr1,
        &identity.rtmr2,
        &identity.rtmr3,
        report_data,
        identity.tcb_status,
        identity.timestamp,
    );
    // Sentinel proof. The chain's `mock-attestation` build skips the
    // ProofVerifyUltraHonk gRPC call entirely. Production-without-mock will
    // reject this; that's the intended signal to compile in
    // `--features real-zkdcap`.
    let proof = vec![0xABu8; 192];
    Ok((proof, public_inputs))
}

#[cfg(feature = "real-zkdcap")]
async fn produce_artifacts_inner(
    dstack: &dyn crate::dstack::DstackClient,
    identity: &EnclaveIdentity,
    report_data: &[u8; 64],
) -> Result<(Vec<u8>, Vec<u8>), AttestationError> {
    use base64::Engine as _;

    // 1. Real TDX quote bound to report_data via dstack guest agent.
    let quote = dstack.get_quote(report_data).await?;

    // 2. Intel PCS collateral -> the dcap-qvl bundle the noir witness
    //    generator (genprover) consumes.
    let collateral_json = real_zkdcap::fetch_collateral_json(&quote).await?;

    // 3. POST to the noir/bb prove server (default /run/noir/prove.sock,
    //    env ZKDCAP_PROVER_SOCKET). `identity.timestamp` rides in the
    //    request and the proof's witness so the freshness windows agree.
    let socket_path = std::env::var("ZKDCAP_PROVER_SOCKET")
        .unwrap_or_else(|_| "/run/noir/prove.sock".to_string());
    let timestamp = identity.timestamp;
    let request_body = serde_json::json!({
        "quote_hex": hex::encode(&quote),
        "collateral_json": collateral_json,
        "timestamp": timestamp,
    });
    let response_bytes = real_zkdcap::post_unix_socket(&socket_path, &request_body).await?;

    // 4. UltraHonk proof + the circuit's packed public_inputs, both base64.
    //    On this path bb EMITS public_inputs; the enclave does not build them.
    let resp: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|e| AttestationError::ZkProver(format!("parse prove response: {e}")))?;
    let b64 = base64::engine::general_purpose::STANDARD;
    let field_b64 = |key: &str| -> Result<Vec<u8>, AttestationError> {
        let s = resp
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| AttestationError::ZkProver(format!("prove response missing `{key}`")))?;
        b64.decode(s)
            .map_err(|e| AttestationError::ZkProver(format!("decode {key}: {e}")))
    };
    let proof = field_b64("proof")?;
    let public_inputs = field_b64("public_inputs")?;

    // 5. Defensive: the prover's packed public_inputs MUST carry the
    //    ReportData we bound and the requested timestamp, or the chain-side
    //    recompute-and-compare would reject. Fail fast with a clear error
    //    instead of shipping a proof the contract is guaranteed to refuse.
    if extract_report_data(&public_inputs).as_ref() != Some(report_data) {
        return Err(AttestationError::ZkProver(
            "prover public_inputs report_data != bound report_data".into(),
        ));
    }
    if extract_timestamp(&public_inputs) != Some(timestamp) {
        return Err(AttestationError::ZkProver(
            "prover public_inputs timestamp != requested timestamp".into(),
        ));
    }

    Ok((proof, public_inputs))
}

// ---------------------------------------------------------------------------
// real-zkdcap helpers (gated; not compiled in default builds)
// ---------------------------------------------------------------------------

#[cfg(feature = "real-zkdcap")]
mod real_zkdcap {
    use super::AttestationError;
    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    /// Fetch PCS collateral (TCB info, QE identity, PCK CRL) and serialize
    /// the dcap-qvl bundle to JSON for the noir witness generator. genprover
    /// reads the same collateral.json shape the fixtures use; the prove
    /// server pairs it with the quote.
    pub async fn fetch_collateral_json(quote: &[u8]) -> Result<Value, AttestationError> {
        let collateral = dcap_qvl::collateral::get_collateral_from_pcs(quote)
            .await
            .map_err(|e| AttestationError::ZkProver(format!("PCS collateral fetch: {e}")))?;
        serde_json::to_value(&collateral)
            .map_err(|e| AttestationError::ZkProver(format!("serialize collateral: {e}")))
    }

    /// POST `body` as JSON to `POST /prove HTTP/1.1` over a unix socket;
    /// return the response body bytes. The noir/bb prove server speaks
    /// HTTP/1.1 over a unix socket with `Connection: close` semantics; we
    /// don't pull a full HTTP client in for this one POST.
    pub async fn post_unix_socket(
        socket_path: &str,
        body: &Value,
    ) -> Result<Vec<u8>, AttestationError> {
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| AttestationError::ZkProver(format!("serialize request: {e}")))?;

        let mut stream = UnixStream::connect(socket_path)
            .await
            .map_err(|e| AttestationError::ZkProver(format!("connect {socket_path}: {e}")))?;
        let request = format!(
            "POST /prove HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body_bytes.len()
        );
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| AttestationError::ZkProver(format!("write headers: {e}")))?;
        stream
            .write_all(&body_bytes)
            .await
            .map_err(|e| AttestationError::ZkProver(format!("write body: {e}")))?;
        stream
            .flush()
            .await
            .map_err(|e| AttestationError::ZkProver(format!("flush: {e}")))?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|e| AttestationError::ZkProver(format!("read response: {e}")))?;

        // Split off HTTP status + headers; require 200.
        let sep = b"\r\n\r\n";
        let body_start = response
            .windows(4)
            .position(|w| w == sep)
            .ok_or_else(|| {
                AttestationError::ZkProver("malformed HTTP response (no header separator)".into())
            })?
            + 4;
        let status_line: &[u8] = response
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|i| &response[..i])
            .unwrap_or(&response[..0]);
        if !status_line.windows(3).any(|w| w == b"200") {
            return Err(AttestationError::ZkProver(format!(
                "noir prove server returned non-200: {}",
                String::from_utf8_lossy(status_line)
            )));
        }
        Ok(response[body_start..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verified_rcv_enclave_core::{RoundCount, TallyResult};

    fn sample_tally() -> TallyResult {
        TallyResult {
            winners: vec!["alice".to_string()],
            per_round_counts: vec![vec![
                RoundCount { candidate: "alice".to_string(), count: 2 },
                RoundCount { candidate: "bob".to_string(), count: 1 },
            ]],
            eliminated_by_round: vec![],
            ballots_tallied: 3,
            ballots_dropped: 0,
            dropped_voters: vec![],
            non_voters: vec!["carol".to_string()],
        }
    }

    fn empty_bh() -> [u8; 32] {
        compute_ballots_hash(&[], &[])
    }

    /// v0.3.14: empty names_hash (zero-length names list).
    fn empty_nh() -> [u8; 32] {
        compute_names_hash(&[])
    }

    #[test]
    fn publish_report_data_layout() {
        let tally = sample_tally();
        let bh = empty_bh();
        let nh = empty_nh();
        let rd = build_publish_report_data("xion1contract", "xion-1", 7, &bh, &nh, &tally);
        let canonical = canonical_serialization("xion1contract", "xion-1", 7, &bh, &nh, &tally);
        let expect = Sha256::digest(&canonical);
        assert_eq!(&rd[..32], &expect[..]);
        assert_eq!(&rd[32..32 + DST_TALLY.len()], DST_TALLY);
        assert!(rd[32 + DST_TALLY.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn chain_id_affects_canonical_serialization_bytes() {
        let tally = sample_tally();
        let bh = empty_bh();
        let nh = empty_nh();
        let a = canonical_serialization("xion1c", "xion-1", 7, &bh, &nh, &tally);
        let b = canonical_serialization("xion1c", "xion-2", 7, &bh, &nh, &tally);
        assert_ne!(a, b, "chain_id must affect canonical_serialization bytes (N4)");
    }

    #[test]
    fn ballots_hash_affects_canonical_serialization_bytes() {
        // B6 (v0.3.11): different raw_ballots produce different commit hashes,
        // even with the same chain/contract/election/tally.
        let tally = sample_tally();
        let bh_a = compute_ballots_hash(
            &["alice".to_string()],
            &[("alice".to_string(), vec![0u8; 4])],
        );
        let bh_b = compute_ballots_hash(
            &["alice".to_string()],
            &[("alice".to_string(), vec![0xFFu8; 4])],
        );
        let nh = empty_nh();
        let a = canonical_serialization("xion1c", "xion-1", 7, &bh_a, &nh, &tally);
        let b = canonical_serialization("xion1c", "xion-1", 7, &bh_b, &nh, &tally);
        assert_ne!(a, b, "ballots_hash must affect canonical_serialization bytes (B6)");
    }

    #[test]
    fn names_hash_affects_canonical_serialization_bytes() {
        // v0.3.14: different candidate_names produce different commit hashes.
        let tally = sample_tally();
        let bh = empty_bh();
        let nh_a = compute_names_hash(&["Alice".to_string(), "Bob".to_string()]);
        let nh_b = compute_names_hash(&["Carol".to_string(), "Dan".to_string()]);
        let a = canonical_serialization("xion1c", "xion-1", 7, &bh, &nh_a, &tally);
        let b = canonical_serialization("xion1c", "xion-1", 7, &bh, &nh_b, &tally);
        assert_ne!(a, b, "names_hash must affect canonical_serialization bytes (v0.3.14)");
    }

    #[test]
    fn compute_names_hash_order_sensitive() {
        // v0.3.14: name declaration order is load-bearing — reordering
        // names produces a different hash even when the set of names is
        // identical. Mirrors compute_ballots_hash's declaration-order pin.
        let a = compute_names_hash(&["Alice".to_string(), "Bob".to_string()]);
        let b = compute_names_hash(&["Bob".to_string(), "Alice".to_string()]);
        assert_ne!(a, b, "name order must affect names_hash");
    }

    #[test]
    fn compute_names_hash_empty_stable() {
        // Empty names list -> well-defined hash (SHA-256 of just the
        // u32_LE(0) length prefix).
        let a = compute_names_hash(&[]);
        let b = compute_names_hash(&[]);
        assert_eq!(a, b);
    }

    #[test]
    fn ballots_hash_candidate_declaration_order_pinned() {
        // The orchestrator MUST pass raw_ballots in candidate-declaration
        // order. If it doesn't, ballots_hash diverges from the chain's
        // expected value and the chain rejects.
        let cands = vec!["alice".to_string(), "bob".to_string()];
        let in_order = compute_ballots_hash(
            &cands,
            &[
                ("alice".to_string(), vec![1, 2, 3]),
                ("bob".to_string(), vec![4, 5, 6]),
            ],
        );
        // Same ballots, but presented in a different order to the helper.
        // Since the helper walks `candidates` in declaration order and
        // filters by voter match, the order of `entries` shouldn't matter
        // -- it should produce the same hash because both contain (alice,
        // [1,2,3]) and (bob, [4,5,6]).
        let shuffled = compute_ballots_hash(
            &cands,
            &[
                ("bob".to_string(), vec![4, 5, 6]),
                ("alice".to_string(), vec![1, 2, 3]),
            ],
        );
        assert_eq!(in_order, shuffled);
    }

    #[test]
    fn registration_report_data_layout() {
        let pk = vec![0x02u8; 33];
        let nh = compute_names_hash(&["Alice".to_string(), "Bob".to_string()]);
        let rd = build_registration_report_data(&pk, "xion1addr", 7, &nh);
        let mut preimage = Vec::new();
        preimage.extend_from_slice(&pk);
        write_borsh_string(&mut preimage, "xion1addr");
        preimage.extend_from_slice(&7u64.to_le_bytes());
        preimage.extend_from_slice(&nh);
        let expect = Sha256::digest(&preimage);
        assert_eq!(&rd[..32], &expect[..]);
        assert_eq!(&rd[32..32 + DST_PUBKEY.len()], DST_PUBKEY);
        assert!(rd[32 + DST_PUBKEY.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn registration_report_data_election_id_binds_n22() {
        // N22 (v0.3.12): election_id MUST affect the registration ReportData
        // bytes so old quotes can't replay across elections.
        let pk = vec![0x03u8; 33];
        let nh = [0u8; 32];
        let a = build_registration_report_data(&pk, "xion1addr", 1, &nh);
        let b = build_registration_report_data(&pk, "xion1addr", 2, &nh);
        assert_ne!(a[..32], b[..32], "election_id must affect registration ReportData");
    }

    #[test]
    fn registration_report_data_contract_addr_binds_n22() {
        let pk = vec![0x03u8; 33];
        let nh = [0u8; 32];
        let a = build_registration_report_data(&pk, "xion1A", 7, &nh);
        let b = build_registration_report_data(&pk, "xion1B", 7, &nh);
        assert_ne!(a[..32], b[..32], "contract_addr must affect registration ReportData");
    }

    /// v0.3.14 F1: names_hash MUST affect the registration ReportData bytes
    /// so an admin can't swap candidate display names between CreateElection
    /// and PublishResult without re-running registration.
    #[test]
    fn registration_report_data_names_hash_binds_f1() {
        let pk = vec![0x03u8; 33];
        let nh_a = compute_names_hash(&["Alice".to_string()]);
        let nh_b = compute_names_hash(&["Bob".to_string()]);
        assert_ne!(nh_a, nh_b, "compute_names_hash distinguishes inputs");
        let a = build_registration_report_data(&pk, "xion1addr", 7, &nh_a);
        let b = build_registration_report_data(&pk, "xion1addr", 7, &nh_b);
        assert_ne!(a[..32], b[..32], "names_hash must affect registration ReportData");
    }

    #[test]
    fn public_inputs_total_length() {
        let pi = build_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        assert_eq!(pi.len(), ULTRAHONK_PUBLIC_INPUTS_LEN);
        assert_eq!(pi.len(), 544);
    }

    #[test]
    fn public_inputs_pack_be_invariant() {
        // Each packed limb fills only its LOW K bytes; the high 32-K bytes
        // of its 32-byte field stay zero (the pack_be injectivity invariant).
        let mut mrtd = [0u8; 48];
        mrtd[0] = 0xAB; // first byte of the low-31 limb (field 0)
        mrtd[47] = 0xCD; // last byte of the low-17 limb (field 1)
        let pi = build_public_inputs(
            &mrtd, &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        // Field 0 carries mrtd[0..31] in its low 31 bytes -> mrtd[0] lands at
        // byte offset (32 - 31) = 1.
        assert_eq!(pi[1], 0xAB);
        assert_eq!(pi[0], 0, "field 0 high byte zero");
        // Field 1 carries mrtd[31..48] in its low 17 bytes -> mrtd[47]
        // (limb byte 16) lands at byte offset 32 + (32 - 17) + 16 = 63.
        let field1 = 32usize;
        assert_eq!(pi[field1 + 31], 0xCD);
        for b in &pi[field1..field1 + (32 - 17)] {
            assert_eq!(*b, 0, "field 1 high bytes zero");
        }
    }

    #[test]
    fn public_inputs_extract_round_trip() {
        let mut mrtd = [0u8; 48];
        for (i, b) in mrtd.iter_mut().enumerate() {
            *b = (i as u8).wrapping_add(0x10);
        }
        let mut rd = [0u8; 64];
        for (i, b) in rd.iter_mut().enumerate() {
            *b = (i as u8).wrapping_add(0xA0);
        }
        let pi = build_public_inputs(
            &mrtd, &[0; 48], &[0; 48], &[0; 48], &[0; 48], &rd, 3, 1_700_000_000,
        );
        assert_eq!(extract_report_data(&pi), Some(rd));
        assert_eq!(extract_timestamp(&pi), Some(1_700_000_000));
    }

    #[test]
    fn canonical_serialization_byte_for_byte() {
        let tally = TallyResult {
            winners: vec!["a".to_string()],
            per_round_counts: vec![vec![RoundCount {
                candidate: "a".to_string(),
                count: 1,
            }]],
            eliminated_by_round: vec![],
            ballots_tallied: 1,
            ballots_dropped: 0,
            dropped_voters: vec![],
            non_voters: vec![],
        };
        // B6 (v0.3.11): empty ballots_hash used here (no entries for the
        // single-candidate `a` since the helper requires the candidate to
        // appear in raw_ballots; sample tally above is the math result, not
        // the input). We use a fixed all-zero buffer to keep the byte-pin
        // stable across tests.
        let bh = [0u8; 32];
        // v0.3.14: names_hash 32 raw bytes between ballots_hash and
        // tally_body. Use a distinct all-FF buffer so any byte-position
        // drift surfaces immediately.
        let nh = [0xFFu8; 32];
        let bytes = canonical_serialization("xc", "cid", 7, &bh, &nh, &tally);
        let mut expect = Vec::new();
        expect.extend_from_slice(&2u32.to_le_bytes());
        expect.extend_from_slice(b"xc");
        // N4 (v0.3.10): chain_id between contract_addr and election_id.
        expect.extend_from_slice(&3u32.to_le_bytes());
        expect.extend_from_slice(b"cid");
        expect.extend_from_slice(&7u64.to_le_bytes());
        // B6 (v0.3.11): ballots_hash 32 raw bytes between election_id and names_hash.
        expect.extend_from_slice(&bh);
        // v0.3.14: names_hash 32 raw bytes between ballots_hash and tally_body.
        expect.extend_from_slice(&nh);
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(b"a");
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(b"a");
        expect.extend_from_slice(&1u64.to_le_bytes());
        expect.extend_from_slice(&0u32.to_le_bytes());
        expect.extend_from_slice(&1u64.to_le_bytes());
        expect.extend_from_slice(&0u64.to_le_bytes());
        expect.extend_from_slice(&0u32.to_le_bytes());
        expect.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(bytes, expect);
    }

    #[test]
    fn dst_tags_match_intent_v0_3_9() {
        assert_eq!(DST_TALLY, b"DST_VERIFIED_RCV_TALLY_V1");
        assert_eq!(DST_PUBKEY, b"DST_VERIFIED_RCV_PUBKEY_V1");
    }
}
