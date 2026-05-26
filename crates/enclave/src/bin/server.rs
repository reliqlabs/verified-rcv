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
    let mut identity = identity_from_env();

    // Boot probe 1: dstack `/Info` to confirm transport. Cheap; just
    // logged (compose-hash is the dstack-side image identifier, not the
    // on-chain registry value).
    match dstack.get_image_identity().await {
        Ok(id) => tracing::info!(
            compose_hash = %hex::encode(&id.compose_hash),
            "dstack image identity probed"
        ),
        Err(e) => tracing::warn!(error = %e, "dstack image identity probe failed (continuing — may be simulator)"),
    }

    // Boot probe 2 (Track 2): request a TDX quote with all-zero report_data
    // and parse `MRTD` + `RTMR0..3` from it. On Phala this surfaces the
    // CVM's real image identity for the Health RPC. In simulator mode the
    // mock quote is too short / unparseable, so `measurements` stays None
    // and Health falls back to ready=false + empty hex.
    let measurements = match dstack.get_quote(&[0u8; 64]).await {
        Ok(quote) => match verified_rcv_enclave::tdx_quote::parse_measurements(&quote) {
            Ok(m) => {
                tracing::info!(
                    mrtd = %hex::encode(m.mrtd),
                    rtmr0 = %hex::encode(m.rtmr0),
                    "parsed TDX measurements from boot quote"
                );
                // The env-supplied `MRTD` / `RTMR*` defaults to all zeros
                // when unset. If the operator left them unset (production
                // path), populate from the boot-probed quote so the
                // chain's measurement equality check against the on-chain
                // registry uses the live values rather than the env stub.
                if identity.mrtd == [0u8; 48] {
                    identity.mrtd = m.mrtd;
                }
                if identity.rtmr0 == [0u8; 48] {
                    identity.rtmr0 = m.rtmr0;
                }
                if identity.rtmr1 == [0u8; 48] {
                    identity.rtmr1 = m.rtmr1;
                }
                if identity.rtmr2 == [0u8; 48] {
                    identity.rtmr2 = m.rtmr2;
                }
                if identity.rtmr3 == [0u8; 48] {
                    identity.rtmr3 = m.rtmr3;
                }
                Some(m)
            }
            Err(e) => {
                tracing::warn!(error = %e, "boot quote did not parse as TDX 1.0 (expected in simulator / non-Phala dev)");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "boot quote request failed (continuing without cached measurements)");
            None
        }
    };

    let svc = match measurements {
        Some(m) => TallyServiceImpl::with_measurements(dstack, identity, m),
        None => TallyServiceImpl::new(dstack, identity),
    };
    tracing::info!(%listen_addr, "verified-rcv enclave server starting");

    Server::builder()
        .add_service(svc.into_server())
        .serve(listen_addr)
        .await?;

    Ok(())
}
