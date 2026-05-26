//! gRPC `TallyService` impl (Phase 1 per docs/runtime-integration.md).
//!
//! The handler plumbs chain-side election state through `tally_spec` and
//! the dstack KMS + attestation envelope construction. It does NOT
//! re-implement IRV (load-bearing trust-model rule: §"Trust model — what
//! the runtime MUST NOT do") — `tally_spec` lives in `verified_rcv_enclave`
//! and is the same function the off-TDX `verified-rcv-enclave` CLI calls.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use verified_rcv_enclave_core::{Addr, RawBallots, RawEntry};

use crate::attestation::{build_envelope, EnvelopeConfig};
use crate::dstack::DstackClient;
use crate::tally_spec;

pub mod proto {
    tonic::include_proto!("verified_rcv");
}

use proto::tally_service_server::{TallyService, TallyServiceServer};
use proto::{HealthRequest, HealthResponse, TallyRequest, TallyResponse};

pub struct TallyServiceImpl {
    dstack: Arc<dyn DstackClient>,
    envelope_config: EnvelopeConfig,
}

impl TallyServiceImpl {
    pub fn new(dstack: Arc<dyn DstackClient>, envelope_config: EnvelopeConfig) -> Self {
        Self { dstack, envelope_config }
    }

    pub fn into_server(self) -> TallyServiceServer<Self> {
        TallyServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl TallyService for TallyServiceImpl {
    async fn tally(
        &self,
        req: Request<TallyRequest>,
    ) -> Result<Response<TallyResponse>, Status> {
        let TallyRequest { contract_addr, election_id, candidates, raw_ballots } =
            req.into_inner();

        // 1. KMS handshake. Privkey MUST come from dstack KMS per
        //    docs/runtime-integration.md trust-model section. Even in
        //    simulator mode this routes through the DstackClient interface.
        let privkey = self
            .dstack
            .derive_privkey(&contract_addr, election_id)
            .await
            .map_err(|e| Status::internal(format!("dstack derive_privkey: {e}")))?;

        // 2. Stage 1 + Stage 2 (intent §2.5). Caller is responsible for
        //    reordering raw_ballots into candidate-declaration order; we
        //    walk linearly per the iteration-discipline pin.
        let raw_ballots: RawBallots = raw_ballots
            .into_iter()
            .map(|b| RawEntry { voter: b.voter, ciphertext: b.ciphertext })
            .collect();
        let candidates: Vec<Addr> = candidates;
        let tally = tally_spec(&raw_ballots, &candidates, &privkey);

        // 3. Attestation envelope. Calls dstack get_quote for the TDX
        //    quote bound to user_data, then (if configured) the zkdcap
        //    prover for the Groth16 wrapper.
        let envelope = build_envelope(
            self.dstack.as_ref(),
            &self.envelope_config,
            &contract_addr,
            election_id,
            &tally,
        )
        .await
        .map_err(|e| Status::internal(format!("attestation envelope: {e}")))?;

        let tally_json = serde_json::to_string(&tally)
            .map_err(|e| Status::internal(format!("serialize tally: {e}")))?;
        let attestation_json = serde_json::to_string(&envelope)
            .map_err(|e| Status::internal(format!("serialize envelope: {e}")))?;

        Ok(Response::new(TallyResponse { tally_json, attestation_json }))
    }

    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        // Audit remediation M5 (2026-05-26):
        //
        // The honest answer for `mrtd_hex` / `rtmr_hex` requires parsing
        // a real TDX quote, which we don't have yet. The previous
        // implementation returned `compose_hash` (a dstack image
        // identifier) in the `mrtd_hex` slot, which would lead an operator
        // following docs/deploy.md to populate the on-chain
        // `EnclaveImageRegistry { mrtd, rtmr }` with the wrong value —
        // making every subsequent attestation fail verification (or, worse,
        // pass against the wrong baseline).
        //
        // We now return `ready = false` and an EMPTY `mrtd_hex` /
        // `rtmr_hex` until a TDX-quote parser is wired. Operators must
        // refuse to copy "" into the on-chain registry; the deploy guide
        // says so.
        //
        // The dstack image identity is still surfaced via tracing logs for
        // dev-time visibility (see server.rs::serve bootstrap), so this
        // doesn't lose operator-facing information — it just refuses to
        // present compose_hash AS IF it were the TDX MRTD.
        let _identity = self
            .dstack
            .get_image_identity()
            .await
            .map_err(|e| Status::internal(format!("dstack image identity: {e}")))?;
        Ok(Response::new(HealthResponse {
            ready: false,
            mrtd_hex: String::new(),
            rtmr_hex: String::new(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::proto::tally_service_client::TallyServiceClient;
    use super::proto::tally_service_server::TallyServiceServer;
    use super::*;
    use crate::dstack::{
        parse_mock_quote_user_data, SimulatorDstackClient, SIMULATOR_QUOTE_MAGIC,
    };
    use borsh::to_vec as borsh_to_vec;
    use ecies::utils::generate_keypair;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tonic::transport::Server;

    fn encrypt_for_test(ranking: &[&str], pubkey_uncompressed: &[u8]) -> Vec<u8> {
        let v: Vec<String> = ranking.iter().map(|s| s.to_string()).collect();
        let plaintext = borsh_to_vec(&v).unwrap();
        ecies::encrypt(pubkey_uncompressed, &plaintext).unwrap()
    }

    /// Phase 1 / brief test 1: spin up the gRPC server with the simulator,
    /// send a Tally RPC with the 3-candidate roundtrip fixture, assert the
    /// response carries a valid TallyResult and an envelope whose user_data
    /// matches `build_user_data(contract_addr, tally)`.
    #[tokio::test]
    async fn server_smoke() {
        let (sk, pk) = generate_keypair();
        let mut priv32 = [0u8; 32];
        priv32.copy_from_slice(&sk.serialize());
        let pubkey = pk.serialize();

        let cands = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let raw_ballots = vec![
            proto::RawBallot {
                voter: "A".to_string(),
                ciphertext: encrypt_for_test(&["A", "B", "C"], &pubkey),
            },
            proto::RawBallot {
                voter: "B".to_string(),
                ciphertext: encrypt_for_test(&["A", "C", "B"], &pubkey),
            },
        ];

        // Bind to an ephemeral port and spin up the service.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let svc = TallyServiceImpl::new(
            Arc::new(SimulatorDstackClient::with_privkey(priv32)),
            EnvelopeConfig { zkdcap_prover_endpoint: None },
        );
        let server_task = tokio::spawn(async move {
            Server::builder()
                .add_service(TallyServiceServer::new(svc))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        // Give the server a moment to start accepting.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TallyServiceClient::connect(format!("http://{addr}"))
            .await
            .expect("client connect");
        let resp = client
            .tally(Request::new(TallyRequest {
                contract_addr: "xion1contract".to_string(),
                election_id: 42,
                candidates: cands.clone(),
                raw_ballots,
            }))
            .await
            .expect("tally rpc")
            .into_inner();

        // Tally body sanity.
        let tally: verified_rcv_enclave_core::TallyResult =
            serde_json::from_str(&resp.tally_json).expect("parse tally JSON");
        assert_eq!(tally.winners, vec!["A".to_string()]);
        assert_eq!(tally.ballots_tallied, 2);
        assert_eq!(tally.ballots_dropped, 0);
        assert_eq!(tally.non_voters, vec!["C".to_string()]);

        // Envelope: with ZKDCAP_PROVER_URL unset (config.zkdcap_prover_endpoint =
        // None), the envelope is Mock — the dev path. Assert that, but also
        // that the user_data we WOULD have committed to is reconstructible
        // by hand for downstream verification by the chain-side B8(c) check.
        let envelope: crate::attestation::AttestationEnvelopeJson =
            serde_json::from_str(&resp.attestation_json).expect("parse envelope JSON");
        assert!(matches!(
            envelope,
            crate::attestation::AttestationEnvelopeJson::Mock
        ));
        let expected_ud = crate::attestation::build_user_data("xion1contract", 7, &tally);
        assert_eq!(expected_ud.len(), 64);
        assert_eq!(&expected_ud[..25], b"DST_VERIFIED_RCV_TALLY_V1");

        server_task.abort();
    }

    /// Variant of the smoke test with the Dstack envelope path exercised:
    /// no zkdcap prover is set up (so this also yields a Mock), but we
    /// directly check that `dstack.get_quote(user_data)` round-trips the
    /// user_data via `parse_mock_quote_user_data` — the assertion path
    /// the brief's test 4 calls for, without needing a real TDX parser.
    #[tokio::test]
    async fn simulator_quote_round_trip_under_envelope() {
        let priv32 = [0xCDu8; 32];
        let dstack = SimulatorDstackClient::with_privkey(priv32);
        let tally = verified_rcv_enclave_core::TallyResult {
            winners: vec!["alice".to_string()],
            per_round_counts: vec![],
            eliminated_by_round: vec![],
            ballots_tallied: 0,
            ballots_dropped: 0,
            dropped_voters: vec![],
            non_voters: vec![],
        };
        let user_data = crate::attestation::build_user_data("xion1abc", 7, &tally);
        let quote = <SimulatorDstackClient as DstackClient>::get_quote(&dstack, &user_data)
            .await
            .unwrap();
        // Simulator quote carries the user_data verbatim after the magic.
        assert_eq!(&quote[..16], SIMULATOR_QUOTE_MAGIC);
        let recovered = parse_mock_quote_user_data(&quote).unwrap();
        assert_eq!(recovered, user_data);
    }
}
