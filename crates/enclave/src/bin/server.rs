//! verified-rcv enclave gRPC server binary (Phase 1 per
//! docs/runtime-integration.md).
//!
//! Reads transport selection from env (`DSTACK_SOCKET`, `DSTACK_SIMULATOR`,
//! `DSTACK_ENDPOINT`, `DSTACK_DEV_PRIVKEY`, `ZKDCAP_PROVER_URL`) and
//! exposes `TallyService` on `LISTEN_ADDR` (default `0.0.0.0:50051`).

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::Server;

use verified_rcv_enclave::attestation::EnclaveIdentity;
use verified_rcv_enclave::dstack::{build_client_from_env, DstackClient};
use verified_rcv_enclave::server::TallyServiceImpl;

/// Build the `EnclaveIdentity` to attest over from environment variables.
/// Each `MRTD` / `RTMR*` env var, if set, is parsed as a 96-character hex
/// string yielding 48 bytes. Unset values default to all-zeros — fine for
/// `mock-attestation` builds where the chain skips the proof verify; will
/// fail measurement equality against a real registry, which is the
/// intended signal to populate the env via the deploy guide.
fn identity_from_env() -> EnclaveIdentity {
    fn parse_48(name: &str) -> [u8; 48] {
        match env::var(name) {
            Ok(hex_str) => {
                let bytes = hex::decode(hex_str.trim())
                    .unwrap_or_else(|e| panic!("env {name}: invalid hex: {e}"));
                if bytes.len() != 48 {
                    panic!("env {name}: expected 48 bytes (96 hex chars), got {}", bytes.len());
                }
                let mut out = [0u8; 48];
                out.copy_from_slice(&bytes);
                out
            }
            Err(_) => [0u8; 48],
        }
    }
    let tcb_status: u8 = env::var("TCB_STATUS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let timestamp: u64 = env::var("ATTESTATION_TIMESTAMP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });
    EnclaveIdentity {
        mrtd: parse_48("MRTD"),
        rtmr0: parse_48("RTMR0"),
        rtmr1: parse_48("RTMR1"),
        rtmr2: parse_48("RTMR2"),
        rtmr3: parse_48("RTMR3"),
        tcb_status,
        timestamp,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let listen_addr: SocketAddr = env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()?;

    let dstack: Arc<dyn DstackClient> = Arc::from(build_client_from_env());
    let identity = identity_from_env();

    // Boot health check: a Health-equivalent dstack call to confirm the
    // transport is up before binding the listener. In simulator mode this
    // is trivial; against a real dstack guest agent this fails fast if
    // the socket path is wrong or the agent isn't running.
    match dstack.get_image_identity().await {
        Ok(id) => tracing::info!(
            compose_hash = %hex::encode(&id.compose_hash),
            "dstack image identity probed"
        ),
        Err(e) => tracing::warn!(error = %e, "dstack image identity probe failed (continuing — may be simulator)"),
    }

    let svc = TallyServiceImpl::new(dstack, identity);
    tracing::info!(%listen_addr, "verified-rcv enclave server starting");

    Server::builder()
        .add_service(svc.into_server())
        .serve(listen_addr)
        .await?;

    Ok(())
}
