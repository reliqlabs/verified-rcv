//! Quartz attestation envelope construction (Phase 3 per docs/runtime-integration.md).
//!
//! ## Wire layout of `user_data` (intent §3.2 B8(c))
//!
//! ```text
//! user_data (64 bytes)
//!   = upper_32_bytes (domain-separation tag, ASCII, zero-padded)
//!   ‖ lower_32_bytes (SHA-256 commit hash)
//! ```
//!
//! - Upper 32: literal byte string `b"DST_VERIFIED_RCV_TALLY_V1"` (25 bytes)
//!   right-padded with zeros to 32 bytes. Constant per intent §3.2 B8(c).
//! - Lower 32: `SHA-256(canonical_serialization(contract_addr ‖ tally_body))`
//!   per intent §2.5's Borsh pin.
//!
//! ## canonical_serialization (intent §2.5 v0.3.1 T7 leaf pin)
//!
//! - `Addr` → Borsh `String` (u32-LE length prefix + UTF-8).
//! - `Nat` → `u64` little-endian. Note that `TallyResult`'s `ballots_*`
//!   fields are declared as `u32` in Rust; we widen to `u64` here to honor
//!   the intent's leaf-encoding pin. Future intent-vs-code reconciliation
//!   could either narrow the leaf pin or widen the Rust types; we honor
//!   the spec here because the spec is the trust anchor.
//! - `Vec<T>` → u32-LE length prefix + element bytes in order.
//! - Field-order: TallyResult declaration order (intent §2.5).
//!
//! We implement this as an explicit byte-builder rather than relying on a
//! `BorshSerialize` derive so the encoding is auditable side-by-side with
//! the intent text and so we don't have to touch the formally-verified
//! enclave-core crate for a derive.

use sha2::{Digest, Sha256};
use thiserror::Error;

use verified_rcv_enclave_core::{RoundCount, RoundCounts, TallyResult};

use crate::dstack::{DstackClient, DstackError};

/// Domain-separation tag per intent §3.2 B8(c). 25 ASCII bytes.
pub const DOMAIN_TAG: &[u8] = b"DST_VERIFIED_RCV_TALLY_V1";

#[derive(Debug, Error)]
pub enum AttestationError {
    #[error("dstack: {0}")]
    Dstack(#[from] DstackError),
    #[error("zkdcap prover: {0}")]
    ZkProver(String),
}

/// Construct the 64-byte `user_data` block bound into the TDX quote.
pub fn build_user_data(
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> [u8; 64] {
    let mut user_data = [0u8; 64];
    user_data[..DOMAIN_TAG.len()].copy_from_slice(DOMAIN_TAG);

    let canonical = canonical_serialization(contract_addr, election_id, tally);
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let commit = hasher.finalize();
    user_data[32..].copy_from_slice(&commit);
    user_data
}

/// Borsh-style canonical serialization of `(contract_addr ‖ tally_body)`
/// per intent §2.5 / v0.3.1 T7 leaf-encoding pin.
pub fn canonical_serialization(
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, contract_addr);
    // election_id (intent v0.3.8 M2 audit remediation): u64 LE.
    out.extend_from_slice(&election_id.to_le_bytes());
    write_tally_body(&mut out, tally);
    out
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
    // Intent §2.5 leaf-encoding pin: Nat → u64 LE.
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
    // Declaration-order emission per intent §2.5.
    write_vec_addr(out, &t.winners);
    write_per_round_counts(out, &t.per_round_counts);
    write_eliminated_by_round(out, &t.eliminated_by_round);
    out.extend_from_slice(&(t.ballots_tallied as u64).to_le_bytes());
    out.extend_from_slice(&(t.ballots_dropped as u64).to_le_bytes());
    write_vec_addr(out, &t.dropped_voters);
    write_vec_addr(out, &t.non_voters);
}

// ---------------------------------------------------------------------------
// Envelope assembly
// ---------------------------------------------------------------------------

/// JSON-serializable mirror of `verified_rcv_contract::msg::AttestationEnvelope`.
///
/// We re-declare the shape here so the runtime crate doesn't pull in the
/// `cosmwasm-std` dep just to hex-encode three byte buffers. The JSON the
/// orchestrator feeds back to the contract is structurally identical to
/// what `cw_serde` produces for the contract-side enum.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum AttestationEnvelopeJson {
    Mock,
    Dstack {
        quote: String,     // hex
        zk_proof: String,  // hex
        user_data: String, // hex
    },
}

/// Configuration for envelope construction.
#[derive(Debug, Clone)]
pub struct EnvelopeConfig {
    /// `None` ⇒ skip zkdcap proof generation; emit a `Mock` envelope. Used
    /// in dev (matches the contract-side `Mock` attestation variant).
    /// `Some(url)` ⇒ POST the quote bytes to `{url}/prove` and embed the
    /// returned Groth16 proof.
    pub zkdcap_prover_endpoint: Option<String>,
}

impl EnvelopeConfig {
    pub fn from_env() -> Self {
        let zkdcap_prover_endpoint = std::env::var("ZKDCAP_PROVER_URL").ok();
        Self { zkdcap_prover_endpoint }
    }
}

pub async fn build_envelope(
    client: &dyn DstackClient,
    config: &EnvelopeConfig,
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Result<AttestationEnvelopeJson, AttestationError> {
    let user_data = build_user_data(contract_addr, election_id, tally);
    let quote = client.get_quote(&user_data).await?;

    let prover_endpoint = match &config.zkdcap_prover_endpoint {
        None => {
            // Dev path. Matches the contract-side `Mock` envelope so the
            // chain still accepts the PublishResult. The honest disclosure
            // in docs/runtime-integration.md applies here.
            tracing::warn!(
                "ZKDCAP_PROVER_URL not set; emitting Mock attestation envelope (dev only)"
            );
            return Ok(AttestationEnvelopeJson::Mock);
        }
        Some(url) => url.clone(),
    };

    let zk_proof = generate_zkdcap_proof(&quote, &prover_endpoint).await?;

    Ok(AttestationEnvelopeJson::Dstack {
        quote: hex::encode(&quote),
        zk_proof: hex::encode(&zk_proof),
        user_data: hex::encode(user_data),
    })
}

async fn generate_zkdcap_proof(
    quote: &[u8],
    prover_endpoint: &str,
) -> Result<Vec<u8>, AttestationError> {
    let url = format!("{}/prove", prover_endpoint.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| AttestationError::ZkProver(format!("client: {e}")))?;
    let resp = client
        .post(&url)
        .body(quote.to_vec())
        .send()
        .await
        .map_err(|e| AttestationError::ZkProver(format!("post {url}: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AttestationError::ZkProver(format!(
            "zkdcap prover returned status {status}: {body}"
        )));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| AttestationError::ZkProver(format!("read body: {e}")))
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

    #[test]
    fn user_data_binding_matches_intent_layout() {
        let tally = sample_tally();
        let ud = build_user_data("xion1contract", 7, &tally);

        // Upper 32 bytes: DOMAIN_TAG followed by zeros.
        assert_eq!(&ud[..DOMAIN_TAG.len()], DOMAIN_TAG);
        assert!(ud[DOMAIN_TAG.len()..32].iter().all(|&b| b == 0));

        // Lower 32 bytes: SHA-256 of canonical_serialization.
        let canonical = canonical_serialization("xion1contract", 7, &tally);
        let expect = Sha256::digest(&canonical);
        assert_eq!(&ud[32..], &expect[..]);
    }

    #[test]
    fn canonical_serialization_byte_for_byte() {
        // Smallest non-trivial tally: 1 candidate winning, 0 rounds elims.
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
        let bytes = canonical_serialization("xc", 7, &tally);

        // Hand-derive the expected byte string per intent §2.5 v0.3.8 leaf
        // pin (M2 audit remediation: election_id added between
        // contract_addr and tally_body):
        //
        // contract_addr = "xc" → 02 00 00 00 'x' 'c'
        // election_id = 7 (u64 LE) → 07 00 00 00 00 00 00 00
        // winners (Vec<Addr>):
        //   01 00 00 00          (length 1)
        //   01 00 00 00 'a'       (Addr "a")
        // per_round_counts (Vec<Vec<RoundCount>>):
        //   01 00 00 00          (1 round)
        //     01 00 00 00         (1 entry)
        //       01 00 00 00 'a'    (candidate)
        //       01 00 00 00 00 00 00 00  (count u64 LE)
        // eliminated_by_round: 00 00 00 00
        // ballots_tallied: 01 00 00 00 00 00 00 00
        // ballots_dropped: 00 00 00 00 00 00 00 00
        // dropped_voters: 00 00 00 00
        // non_voters: 00 00 00 00
        let mut expect = Vec::new();
        expect.extend_from_slice(&2u32.to_le_bytes());
        expect.extend_from_slice(b"xc");
        expect.extend_from_slice(&7u64.to_le_bytes()); // election_id (M2)
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(b"a");
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(&1u32.to_le_bytes());
        expect.extend_from_slice(b"a");
        expect.extend_from_slice(&1u64.to_le_bytes());
        expect.extend_from_slice(&0u32.to_le_bytes()); // eliminated_by_round
        expect.extend_from_slice(&1u64.to_le_bytes()); // ballots_tallied
        expect.extend_from_slice(&0u64.to_le_bytes()); // ballots_dropped
        expect.extend_from_slice(&0u32.to_le_bytes()); // dropped_voters
        expect.extend_from_slice(&0u32.to_le_bytes()); // non_voters

        assert_eq!(bytes, expect, "canonical_serialization disagrees with intent §2.5");
    }

    #[test]
    fn domain_tag_bytes_match_intent_b8c() {
        // Intent §3.2 B8(c) literal: b"DST_VERIFIED_RCV_TALLY_V1".
        assert_eq!(DOMAIN_TAG.len(), 25);
        assert_eq!(DOMAIN_TAG, b"DST_VERIFIED_RCV_TALLY_V1");
    }
}
