//! Attestation construction — v0.3.9 N1 form.
//!
//! ## What this module produces
//!
//! Two artifacts the chain consumes via direct
//! `/xion.zk.v1.Query/ProofVerifyGnark` from `verified-rcv`'s contract:
//!
//! - `proof: Vec<u8>` — gnark-native Groth16 BN254 proof bytes.
//! - `public_inputs: Vec<u8>` — 9_792-byte blob per the §2.5 gnark
//!   public_inputs byte layout (306 BE fr-elements × 32 bytes).
//!
//! `public_inputs` carries `MrTd ‖ Rtmr0..3 ‖ ReportData ‖ TcbStatus ‖
//! Timestamp`. `ReportData` is 64 bytes, split into two purpose-tagged
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
//! public_inputs)` matching the §2.5 byte layout — the proof bytes are
//! a fixed sentinel and the `public_inputs` carries the correct
//! measurements + ReportData (driven from the caller-supplied identity
//! tuple). Under `--features real-zkdcap`, the runtime instead connects
//! to the zkdcap Go prover (see
//! `/Users/mvid/Development/reliq/zkdcap/host/src/gnark.rs`) over a unix
//! socket and embeds the returned proof + extracted public_inputs.

use sha2::{Digest, Sha256};
use thiserror::Error;

use verified_rcv_enclave_core::{RoundCount, RoundCounts, TallyResult};

use crate::dstack::DstackError;

/// 32-byte domain-separation tag for publish quotes (zero-padded). 25 ASCII bytes.
pub const DST_TALLY: &[u8] = b"DST_VERIFIED_RCV_TALLY_V1";
/// 32-byte domain-separation tag for registration quotes (zero-padded). 26 ASCII bytes.
pub const DST_PUBKEY: &[u8] = b"DST_VERIFIED_RCV_PUBKEY_V1";

/// Total `public_inputs` byte length (intent §2.5 gnark layout for the
/// zkdcap reference DCAP circuit).
pub const GNARK_PUBLIC_INPUTS_LEN: usize = 306 * 32;

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
/// election_id ‖ ballots_hash ‖ tally_body)`. The `chain_id` field was
/// added at v0.3.10 (N4) for cross-chain replay defense; the
/// `ballots_hash` field was added at v0.3.11 (B6) to close §8.7 link 7
/// (enclave_input_fidelity).
pub fn canonical_serialization(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, contract_addr);
    write_borsh_string(&mut out, chain_id);
    out.extend_from_slice(&election_id.to_le_bytes());
    out.extend_from_slice(ballots_hash);
    write_tally_body(&mut out, tally);
    out
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
    let mut canonical = Vec::new();
    canonical.extend_from_slice(&included.to_le_bytes());
    canonical.extend_from_slice(&body);
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
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
/// `chain_id`; v0.3.11 (B6) added `ballots_hash` for input-fidelity binding.
pub fn build_publish_report_data(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    tally: &TallyResult,
) -> [u8; 64] {
    let canonical =
        canonical_serialization(contract_addr, chain_id, election_id, ballots_hash, tally);
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
/// The election_id binding blocks an admin replaying an old registration
/// quote for a new election. Preimage layout must stay byte-identical to
/// the contract's `verified_rcv_contract::contract::build_registration_report_data`
/// — `tests/cross_canonical.rs::registration_report_data_dst_matches_contract_layout`
/// is the load-bearing equality cross-test.
pub fn build_registration_report_data(
    enclave_pubkey: &[u8],
    contract_addr: &str,
    election_id: u64,
) -> [u8; 64] {
    let mut preimage = Vec::with_capacity(enclave_pubkey.len() + contract_addr.len() + 16);
    preimage.extend_from_slice(enclave_pubkey);
    write_borsh_string(&mut preimage, contract_addr);
    preimage.extend_from_slice(&election_id.to_le_bytes());
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    let h = hasher.finalize();
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(&h);
    rd[32..32 + DST_PUBKEY.len()].copy_from_slice(DST_PUBKEY);
    rd
}

// ---------------------------------------------------------------------------
// public_inputs construction (intent §2.5 gnark byte layout, v0.3.9)
// ---------------------------------------------------------------------------

const FR_BYTES: usize = 32;
const ELEM_MRTD_START: usize = 0;
const ELEM_RTMR0_START: usize = 48;
const ELEM_RTMR1_START: usize = 96;
const ELEM_RTMR2_START: usize = 144;
const ELEM_RTMR3_START: usize = 192;
const ELEM_REPORTDATA_START: usize = 240;
const ELEM_TCBSTATUS: usize = 304;
const ELEM_TIMESTAMP: usize = 305;

/// Build a 9_792-byte `public_inputs` blob in the layout pinned at
/// intent §2.5 gnark public_inputs byte layout. Each `uints.U8` byte
/// sits at offset `i*32 + 31`; the preceding 31 bytes are zero.
/// `TcbStatus` and `Timestamp` are u64 BE in the last 8 bytes of their
/// respective 32-byte chunks (high 24 bytes zero).
///
/// Under `default` features this synthesizes a chain-acceptable payload
/// without invoking the gnark prover. Under `real-zkdcap`, this same
/// helper builds the chain-side companion; the proof itself comes from
/// the prover and the prover's witness builder consumes the same
/// `(mrtd, rtmr*, report_data, tcb, ts)` tuple.
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
    let mut out = vec![0u8; GNARK_PUBLIC_INPUTS_LEN];
    for (i, &b) in mrtd.iter().enumerate() {
        out[(ELEM_MRTD_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr0.iter().enumerate() {
        out[(ELEM_RTMR0_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr1.iter().enumerate() {
        out[(ELEM_RTMR1_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr2.iter().enumerate() {
        out[(ELEM_RTMR2_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr3.iter().enumerate() {
        out[(ELEM_RTMR3_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in report_data.iter().enumerate() {
        out[(ELEM_REPORTDATA_START + i) * FR_BYTES + 31] = b;
    }
    out[ELEM_TCBSTATUS * FR_BYTES + 31] = tcb_status;
    let ts = timestamp.to_be_bytes();
    out[ELEM_TIMESTAMP * FR_BYTES + 24..ELEM_TIMESTAMP * FR_BYTES + 32].copy_from_slice(&ts);
    out
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
/// verify). Real build (`--features real-zkdcap`): drives the zkdcap
/// gnark prover via unix socket and binds a dstack-signed TDX quote.
pub async fn produce_publish_artifacts(
    dstack: &dyn crate::dstack::DstackClient,
    identity: &EnclaveIdentity,
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    tally: &TallyResult,
) -> Result<(Vec<u8>, Vec<u8>), AttestationError> {
    let report_data = build_publish_report_data(
        contract_addr,
        chain_id,
        election_id,
        ballots_hash,
        tally,
    );
    produce_artifacts_inner(dstack, identity, &report_data).await
}

/// Synthesize a registration-quote `(proof, public_inputs)` pair.
/// v0.3.12 N22: binds `(contract_addr, election_id)` so an admin can't
/// replay an old registration quote for a new election.
pub async fn produce_registration_artifacts(
    dstack: &dyn crate::dstack::DstackClient,
    identity: &EnclaveIdentity,
    enclave_pubkey: &[u8],
    contract_addr: &str,
    election_id: u64,
) -> Result<(Vec<u8>, Vec<u8>), AttestationError> {
    let report_data = build_registration_report_data(enclave_pubkey, contract_addr, election_id);
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
    // ProofVerifyGnark gRPC call entirely. Production-without-mock will
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
    // 1. Real TDX quote bound to report_data via dstack guest agent.
    let quote = dstack.get_quote(report_data).await?;

    // 2. POST to the gnark prove server. Mirrors
    //    /Users/mvid/Development/reliq/zkdcap/host/src/gnark.rs (POST
    //    /prove over a unix socket, body
    //    `{quote_hex, pre_verified_json, timestamp}`). Response is the
    //    proof JSON pinned by
    //    /Users/mvid/Development/reliq/zkdcap/circuits/dcap-gnark/cmd/verify-remote/main.go::proofJSON
    //    (pi_a / pi_b / pi_c / commitments / commitment_pok /
    //    public_signals as decimal strings).
    let socket_path = std::env::var("ZKDCAP_PROVER_SOCKET")
        .unwrap_or_else(|_| "/tmp/gnark-prove-gpu.sock".to_string());
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AttestationError::ZkProver(format!("clock: {e}")))?
        .as_secs();
    let pre_verified_json = real_zkdcap::build_pre_verified_json(&quote, now_secs).await?;
    let request_body = serde_json::json!({
        "quote_hex": hex::encode(&quote),
        "pre_verified_json": pre_verified_json,
        "timestamp": now_secs,
    });
    let response_bytes =
        real_zkdcap::post_unix_socket(&socket_path, &request_body).await?;
    let proof_json: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|e| AttestationError::ZkProver(format!("parse prove response: {e}")))?;

    // 3. Public inputs: built locally from the (identity, report_data)
    //    tuple. The gnark server's `public_signals` SHOULD produce the
    //    same bytewise blob; we use the local build to avoid parser
    //    risk and keep the chain-side verifier's input deterministic
    //    relative to our state. A future hardening pass can extract
    //    `public_signals` and assert equality.
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

    // 4. Proof bytes for the chain's `xion.zk.v1.Query/ProofVerifyGnark`.
    //    The gnark server's JSON is the canonical proof object (per
    //    verify-remote/main.go reconstruction); the chain's verifier
    //    accepts either gnark-native binary or JSON depending on the
    //    xion zk module's decoder. We forward the raw response bytes so
    //    the verifier sees byte-identical input to what `verify-remote`
    //    accepts off-chain. If xion's decoder rejects JSON and requires
    //    gnark-native binary, the conversion belongs here (see TODO).
    //
    //    TODO(real-zkdcap N23): when xion's gnark verifier serialization
    //    is pinned, replace this passthrough with the canonical encoder.
    //    Reconstruct `*groth16_bn254.Proof` from `proof_json.{pi_a,
    //    pi_b, pi_c, commitments, commitment_pok}` (matches
    //    verify-remote/main.go::reconstructProof) and emit gnark-native
    //    bytes (320 + N*64 for N commitments).
    let proof = response_bytes;
    let _ = proof_json; // Parsed for future use; not directly consumed today.

    Ok((proof, public_inputs))
}

// ---------------------------------------------------------------------------
// real-zkdcap helpers (gated; not compiled in default builds)
// ---------------------------------------------------------------------------

#[cfg(feature = "real-zkdcap")]
mod real_zkdcap {
    use super::AttestationError;
    use anyhow::Context;
    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    /// Fetch PCS collateral + extract pre-verified inputs, then convert
    /// to the JSON shape the gnark server expects (camelCase per
    /// zkdcap/circuits/dcap-gnark/witness/types.go::PreVerifiedJSON).
    ///
    /// Implementation mirrors `zkdcap/host/src/gnark.rs::build_pre_verified_json`
    /// and `build_qe_identity_json` to keep the chain-side verifier
    /// happy when the deployed gnark prove server is the same binary
    /// oauth3 deploys.
    pub async fn build_pre_verified_json(
        quote: &[u8],
        now_secs: u64,
    ) -> Result<Value, AttestationError> {
        let collateral = dcap_qvl::collateral::get_collateral_from_pcs(quote)
            .await
            .map_err(|e| AttestationError::ZkProver(format!("PCS collateral fetch: {e}")))?;
        let pre = dcap_qvl::verify::rustcrypto::extract_pre_verified(quote, &collateral, now_secs)
            .map_err(|e| AttestationError::ZkProver(format!("extract pre-verified: {e}")))?;

        let tcb_info =
            serde_json::to_value(&pre.tcb_info).context("serialize tcb_info")
            .map_err(|e| AttestationError::ZkProver(format!("{e}")))?;
        let qe = &pre.qe_identity;
        let tcb_levels = serde_json::to_value(&qe.tcb_levels)
            .map_err(|e| AttestationError::ZkProver(format!("serialize qe tcb_levels: {e}")))?;
        let qe_identity = serde_json::json!({
            "id": qe.id,
            "version": qe.version,
            "issueDate": qe.issue_date,
            "nextUpdate": qe.next_update,
            "tcbEvaluationDataNumber": qe.tcb_evaluation_data_number,
            "miscselect": hex::encode(qe.miscselect),
            "miscselectMask": hex::encode(qe.miscselect_mask),
            "attributes": hex::encode(qe.attributes),
            "attributesMask": hex::encode(qe.attributes_mask),
            "mrsigner": hex::encode(qe.mrsigner),
            "isvprodid": qe.isvprodid,
            "tcbLevels": tcb_levels,
        });

        Ok(serde_json::json!({
            "tcb_info": tcb_info,
            "qe_identity": qe_identity,
            "pck_leaf_der": hex::encode(&pre.pck_leaf_der),
            "cpu_svn": hex::encode(pre.cpu_svn),
            "pce_svn": pre.pce_svn,
            "fmspc": hex::encode(pre.fmspc),
            "ppid": hex::encode(&pre.ppid),
        }))
    }

    /// POST `body` as JSON to `POST /prove HTTP/1.1` over a unix socket;
    /// return the response body bytes. Matches gnark.rs's raw HTTP wire
    /// format (gnark prove server speaks HTTP/1.1 over a unix socket
    /// with `Connection: close` semantics; we don't pull a full HTTP
    /// client in for this one POST).
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
                "gnark prove server returned non-200: {}",
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

    #[test]
    fn publish_report_data_layout() {
        let tally = sample_tally();
        let bh = empty_bh();
        let rd = build_publish_report_data("xion1contract", "xion-1", 7, &bh, &tally);
        let canonical = canonical_serialization("xion1contract", "xion-1", 7, &bh, &tally);
        let expect = Sha256::digest(&canonical);
        assert_eq!(&rd[..32], &expect[..]);
        assert_eq!(&rd[32..32 + DST_TALLY.len()], DST_TALLY);
        assert!(rd[32 + DST_TALLY.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn chain_id_affects_canonical_serialization_bytes() {
        let tally = sample_tally();
        let bh = empty_bh();
        let a = canonical_serialization("xion1c", "xion-1", 7, &bh, &tally);
        let b = canonical_serialization("xion1c", "xion-2", 7, &bh, &tally);
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
        let a = canonical_serialization("xion1c", "xion-1", 7, &bh_a, &tally);
        let b = canonical_serialization("xion1c", "xion-1", 7, &bh_b, &tally);
        assert_ne!(a, b, "ballots_hash must affect canonical_serialization bytes (B6)");
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
        let rd = build_registration_report_data(&pk, "xion1addr", 7);
        let mut preimage = Vec::new();
        preimage.extend_from_slice(&pk);
        write_borsh_string(&mut preimage, "xion1addr");
        preimage.extend_from_slice(&7u64.to_le_bytes());
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
        let a = build_registration_report_data(&pk, "xion1addr", 1);
        let b = build_registration_report_data(&pk, "xion1addr", 2);
        assert_ne!(a[..32], b[..32], "election_id must affect registration ReportData");
    }

    #[test]
    fn registration_report_data_contract_addr_binds_n22() {
        let pk = vec![0x03u8; 33];
        let a = build_registration_report_data(&pk, "xion1A", 7);
        let b = build_registration_report_data(&pk, "xion1B", 7);
        assert_ne!(a[..32], b[..32], "contract_addr must affect registration ReportData");
    }

    #[test]
    fn public_inputs_total_length() {
        let pi = build_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        assert_eq!(pi.len(), GNARK_PUBLIC_INPUTS_LEN);
        assert_eq!(pi.len(), 9_792);
    }

    #[test]
    fn public_inputs_u8_invariant() {
        // Every fr-element MUST have 31 leading zero bytes.
        let mut mrtd = [0u8; 48];
        mrtd[0] = 0xAB;
        mrtd[47] = 0xCD;
        let pi = build_public_inputs(
            &mrtd, &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        for elem in 0..306 {
            let chunk = &pi[elem * 32..elem * 32 + 32];
            for &b in &chunk[..31] {
                assert_eq!(b, 0, "non-zero high byte at element {elem}");
            }
        }
        // Byte values at the right offsets.
        assert_eq!(pi[31], 0xAB);
        assert_eq!(pi[47 * 32 + 31], 0xCD);
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
        let bytes = canonical_serialization("xc", "cid", 7, &bh, &tally);
        let mut expect = Vec::new();
        expect.extend_from_slice(&2u32.to_le_bytes());
        expect.extend_from_slice(b"xc");
        // N4 (v0.3.10): chain_id between contract_addr and election_id.
        expect.extend_from_slice(&3u32.to_le_bytes());
        expect.extend_from_slice(b"cid");
        expect.extend_from_slice(&7u64.to_le_bytes());
        // B6 (v0.3.11): ballots_hash 32 raw bytes between election_id and tally_body.
        expect.extend_from_slice(&bh);
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
