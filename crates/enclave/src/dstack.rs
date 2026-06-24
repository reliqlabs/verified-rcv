//! dstack guest-agent client.
//!
//! Per docs/runtime-integration.md Phase 2: the enclave never embeds a
//! privkey or quote-signing key; the dstack KMS derives per-election
//! secp256k1 keys deterministically from `(contract_addr, election_id)`,
//! and the dstack quote-signing endpoint produces TDX quotes bound to
//! caller-supplied `report_data` (64 bytes, intent §3.2 B8(c) user_data).
//!
//! ## Transport selection (matches oauth3 `src/attestation/mod.rs` pattern)
//!
//! - `DSTACK_SOCKET=/var/run/dstack.sock` → async reqwest with
//!   `.unix_socket(path)`; base URL `http://dstack`. Production Phala CVM
//!   path — the dstack guest agent listens on this socket inside the CVM.
//! - `DSTACK_SIMULATOR=1` → in-process simulator (no HTTP). Reads
//!   `DSTACK_DEV_PRIVKEY` (hex32) for the privkey returned by every
//!   `derive_privkey` call. Matches the brief's literal "fixed privkey"
//!   simulator semantics. The mock quote carries the caller's
//!   report_data verbatim after a magic prefix so the user-data binding
//!   test can round-trip it.
//! - Otherwise → HTTP transport using `DSTACK_ENDPOINT` (default
//!   `http://localhost:8090`) and the Tappd prpc URL shape — the
//!   external dstack-simulator binary path.
//!
//! ## Honest gap
//!
//! `get_image_identity` reads dstack's `/Info` `compose-hash` field and
//! returns it in the `compose_hash` slot of `ImageIdentity` (consumed by
//! the gRPC HealthResponse `mrtd_hex` field). `rtmr` is empty until a
//! TDX quote parser lands. Operators verifying the on-chain
//! `EnclaveImageRegistry` should cross-check `compose-hash` as the
//! canonical image identity for now; per intent §6.1 the registry stores
//! `(mrtd, rtmr, vkey)` as opaque bytes and the chain doesn't enforce
//! the binding, so the operator-side policy can adapt.

use std::env;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DstackError {
    #[error("dstack request failed: {0}")]
    Request(String),
    #[error("dstack returned malformed payload: {0}")]
    Malformed(String),
    #[error("simulator misconfigured: {0}")]
    SimulatorConfig(String),
}

#[async_trait]
pub trait DstackClient: Send + Sync {
    /// Derive the secp256k1 secret key for `(contract_addr, election_id)`.
    /// Derivation context (intent §6.3 dstack_kms_trust): the literal byte
    /// string `verified-rcv-v1:{contract_addr}:{election_id}` is the path
    /// passed to dstack's `DeriveKey`.
    async fn derive_privkey(
        &self,
        contract_addr: &str,
        election_id: u64,
    ) -> Result<[u8; 32], DstackError>;

    /// Sign a TDX quote over the caller-supplied 64-byte `report_data`.
    /// Per intent §3.2 B8(c), `report_data` is laid out as 32-byte
    /// domain-separation tag || 32-byte SHA-256 commit hash; this method
    /// is layout-agnostic and just passes the bytes through.
    async fn get_quote(&self, report_data: &[u8; 64]) -> Result<Vec<u8>, DstackError>;

    /// Image identity (compose-hash + reserved rtmr slot).
    async fn get_image_identity(&self) -> Result<ImageIdentity, DstackError>;
}

#[derive(Debug, Clone, Default)]
pub struct ImageIdentity {
    /// dstack's `compose-hash` (32 bytes hex-decoded). Used by Health as
    /// the `mrtd_hex` field; see module comment for the honest-gap note.
    pub compose_hash: Vec<u8>,
    /// RTMR (empty until TDX quote parsing lands).
    pub rtmr: Vec<u8>,
}

/// Build the canonical client for this process based on env-var selection.
pub fn build_client_from_env() -> Box<dyn DstackClient> {
    if env::var("DSTACK_SIMULATOR").map(|v| v == "1").unwrap_or(false) {
        Box::new(
            SimulatorDstackClient::from_env()
                .expect("DSTACK_SIMULATOR=1 needs DSTACK_DEV_PRIVKEY"),
        )
    } else {
        Box::new(HttpDstackClient::from_env())
    }
}

// ---------------------------------------------------------------------------
// HTTP / Unix-socket client (async reqwest, mirrors oauth3 transport)
// ---------------------------------------------------------------------------

pub struct HttpDstackClient {
    socket_path: Option<String>,
    base_url: String,
}

impl HttpDstackClient {
    pub fn from_env() -> Self {
        let socket_path = env::var("DSTACK_SOCKET").ok();
        let base_url = if socket_path.is_some() {
            "http://dstack".to_string()
        } else {
            env::var("DSTACK_ENDPOINT").unwrap_or_else(|_| "http://localhost:8090".to_string())
        };
        Self { socket_path, base_url }
    }

    fn build_client(&self) -> Result<reqwest::Client, DstackError> {
        let mut builder = reqwest::Client::builder();
        if let Some(path) = &self.socket_path {
            builder = builder.unix_socket(std::path::Path::new(path));
        }
        builder
            .build()
            .map_err(|e| DstackError::Request(format!("client: {e}")))
    }

    fn is_unix(&self) -> bool {
        self.socket_path.is_some()
    }

    /// Phala dstack path: `POST {base}/{Endpoint}` JSON in / JSON out.
    /// Simulator HTTP path: `GET {base}/prpc/Tappd.{Endpoint}?json={url-encoded JSON}`.
    async fn call<Req: Serialize + Sync, Resp: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &Req,
    ) -> Result<Resp, DstackError> {
        let client = self.build_client()?;
        let response = if self.is_unix() {
            let url = format!("{}/{endpoint}", self.base_url);
            client
                .post(&url)
                .json(body)
                .send()
                .await
                .map_err(|e| DstackError::Request(format!("post {url}: {e}")))?
        } else {
            let json_param = serde_json::to_string(body)
                .map_err(|e| DstackError::Malformed(format!("serialize: {e}")))?;
            let url = format!(
                "{}/prpc/Tappd.{endpoint}?json={}",
                self.base_url,
                urlencoding::encode(&json_param),
            );
            client
                .get(&url)
                .send()
                .await
                .map_err(|e| DstackError::Request(format!("get {url}: {e}")))?
        };

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(DstackError::Request(format!(
                "{endpoint} failed: status={status} body={text}"
            )));
        }
        response
            .json::<Resp>()
            .await
            .map_err(|e| DstackError::Malformed(format!("{endpoint} json: {e}")))
    }

    async fn fetch_info(&self) -> Result<DstackInfoResponse, DstackError> {
        let client = self.build_client()?;
        let url = format!("{}/Info", self.base_url);
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| DstackError::Request(format!("get {url}: {e}")))?;
        response
            .json::<DstackInfoResponse>()
            .await
            .map_err(|e| DstackError::Malformed(format!("info json: {e}")))
    }
}

/// dstack 0.5 GetKey request. Was DeriveKey {path, type} in tappd 0.3.x;
/// renamed and gained a `purpose` field that lets the host scope keys by
/// intent. We pass an empty purpose since verified-rcv doesn't multiplex.
#[derive(Debug, Serialize)]
struct GetKeyRequest<'a> {
    path: &'a str,
    purpose: &'a str,
    algorithm: &'a str,
}

#[derive(Debug, Deserialize)]
struct GetKeyResponse {
    key: String,
    #[serde(default)]
    _signature_chain: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GetQuoteRequest {
    report_data: String,
}

/// dstack 0.5 GetQuote response. Field names are snake_case (not camelCase
/// as tappd 0.3.x used). Keep the legacy aliases so the simulator path
/// continues to work.
#[derive(Debug, Deserialize)]
struct GetQuoteResponse {
    quote: String,
    #[serde(default, alias = "eventLog")]
    _event_log: Option<String>,
    #[serde(default, alias = "vmConfig")]
    _vm_config: Option<String>,
    #[serde(default)]
    _report_data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DstackInfoResponse {
    #[serde(rename = "compose-hash", default)]
    compose_hash: String,
}

#[async_trait]
impl DstackClient for HttpDstackClient {
    async fn derive_privkey(
        &self,
        contract_addr: &str,
        election_id: u64,
    ) -> Result<[u8; 32], DstackError> {
        let context = format!("verified-rcv-v1:{contract_addr}:{election_id}");
        // dstack 0.5: POST /GetKey { path, purpose, algorithm }. Was
        // POST /DeriveKey { path, type } in tappd 0.3.x — renamed at the
        // 0.4 → 0.5 cut, deployed on Phala as dstack-dev-0.5.9.
        let resp: GetKeyResponse = self
            .call(
                "GetKey",
                &GetKeyRequest {
                    path: &context,
                    purpose: "",
                    algorithm: "secp256k1",
                },
            )
            .await?;
        let bytes = hex::decode(&resp.key)
            .map_err(|e| DstackError::Malformed(format!("privkey hex: {e}")))?;
        if bytes.len() != 32 {
            return Err(DstackError::Malformed(format!(
                "privkey length {} != 32",
                bytes.len()
            )));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(out)
    }

    async fn get_quote(&self, report_data: &[u8; 64]) -> Result<Vec<u8>, DstackError> {
        let rd_hex = hex::encode(report_data);
        // dstack 0.5: POST /GetQuote. Was /TdxQuote in tappd 0.3.x.
        let resp: GetQuoteResponse = self
            .call("GetQuote", &GetQuoteRequest { report_data: rd_hex })
            .await?;
        hex::decode(&resp.quote).map_err(|e| DstackError::Malformed(format!("quote hex: {e}")))
    }

    async fn get_image_identity(&self) -> Result<ImageIdentity, DstackError> {
        let info = self.fetch_info().await?;
        let compose_hash = if info.compose_hash.is_empty() {
            Vec::new()
        } else {
            hex::decode(&info.compose_hash)
                .map_err(|e| DstackError::Malformed(format!("compose-hash hex: {e}")))?
        };
        Ok(ImageIdentity { compose_hash, rtmr: Vec::new() })
    }
}

// ---------------------------------------------------------------------------
// In-process simulator (DSTACK_SIMULATOR=1)
// ---------------------------------------------------------------------------

pub struct SimulatorDstackClient {
    fixed_privkey: [u8; 32],
}

impl SimulatorDstackClient {
    pub fn from_env() -> Result<Self, DstackError> {
        let hex_priv = env::var("DSTACK_DEV_PRIVKEY")
            .map_err(|_| DstackError::SimulatorConfig("DSTACK_DEV_PRIVKEY env not set".into()))?;
        let bytes = hex::decode(&hex_priv)
            .map_err(|e| DstackError::SimulatorConfig(format!("DSTACK_DEV_PRIVKEY hex: {e}")))?;
        if bytes.len() != 32 {
            return Err(DstackError::SimulatorConfig(format!(
                "DSTACK_DEV_PRIVKEY length {} != 32",
                bytes.len()
            )));
        }
        let mut fixed_privkey = [0u8; 32];
        fixed_privkey.copy_from_slice(&bytes);
        Ok(Self { fixed_privkey })
    }

    pub fn with_privkey(privkey: [u8; 32]) -> Self {
        Self { fixed_privkey: privkey }
    }
}

/// Magic header that lets the smoke test recognise a simulator quote.
pub const SIMULATOR_QUOTE_MAGIC: &[u8; 16] = b"DSTACK_MOCK_QV01";

pub fn mock_quote(report_data: &[u8; 64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + 64 + 32);
    out.extend_from_slice(SIMULATOR_QUOTE_MAGIC);
    out.extend_from_slice(report_data);
    let mut hasher = Sha256::new();
    hasher.update(report_data);
    out.extend_from_slice(&hasher.finalize());
    out
}

pub fn parse_mock_quote_user_data(quote: &[u8]) -> Option<[u8; 64]> {
    if quote.len() < 16 + 64 {
        return None;
    }
    if &quote[..16] != SIMULATOR_QUOTE_MAGIC {
        return None;
    }
    let mut out = [0u8; 64];
    out.copy_from_slice(&quote[16..16 + 64]);
    Some(out)
}

#[async_trait]
impl DstackClient for SimulatorDstackClient {
    async fn derive_privkey(
        &self,
        _contract_addr: &str,
        _election_id: u64,
    ) -> Result<[u8; 32], DstackError> {
        Ok(self.fixed_privkey)
    }

    async fn get_quote(&self, report_data: &[u8; 64]) -> Result<Vec<u8>, DstackError> {
        Ok(mock_quote(report_data))
    }

    async fn get_image_identity(&self) -> Result<ImageIdentity, DstackError> {
        Ok(ImageIdentity::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simulator_derive_privkey_is_deterministic() {
        let key = [7u8; 32];
        let client = SimulatorDstackClient::with_privkey(key);
        let a = client.derive_privkey("xion1abc", 7).await.unwrap();
        let b = client.derive_privkey("xion1abc", 7).await.unwrap();
        let c = client.derive_privkey("xion1xyz", 99).await.unwrap();
        assert_eq!(a, key);
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[tokio::test]
    async fn simulator_quote_round_trips_user_data() {
        let client = SimulatorDstackClient::with_privkey([1u8; 32]);
        let report = [0xABu8; 64];
        let quote = client.get_quote(&report).await.unwrap();
        let recovered = parse_mock_quote_user_data(&quote).expect("simulator-magic quote");
        assert_eq!(recovered, report);
    }

    #[tokio::test]
    async fn simulator_image_identity_empty() {
        let client = SimulatorDstackClient::with_privkey([1u8; 32]);
        let id = client.get_image_identity().await.unwrap();
        assert!(id.compose_hash.is_empty());
        assert!(id.rtmr.is_empty());
    }
}
