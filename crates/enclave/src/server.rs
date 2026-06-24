//! gRPC `TallyService` impl (Phase 1 per docs/runtime-integration.md).
//!
//! v0.3.9 N1: the response now carries `(proof, public_inputs)` instead
//! of a `DstackAttestation` envelope JSON. The orchestrator forwards
//! these directly to the contract's `PublishResult` execute message.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use verified_rcv_enclave_core::{Addr, RawBallots, RawEntry};

use crate::attestation::{
    compute_ballots_hash, produce_publish_artifacts, produce_registration_artifacts,
    EnclaveIdentity,
};
use crate::dstack::DstackClient;
use crate::tally_spec;

pub mod proto {
    tonic::include_proto!("verified_rcv");
}

use proto::tally_service_server::{TallyService, TallyServiceServer};
use proto::{
    HealthRequest, HealthResponse, RegisterPubkeyRequest, RegisterPubkeyResponse, TallyRequest,
    TallyResponse,
};

pub struct TallyServiceImpl {
    dstack: Arc<dyn DstackClient>,
    /// The enclave identity tuple to attest over. In dev / simulator
    /// mode this is whatever the operator configures; in real-TDX mode
    /// this comes from a TDX-quote parser at boot. The mock-attestation
    /// path on the chain doesn't verify the UltraHonk proof, so the
    /// concrete values matter only for the chain's measurement equality
    /// check against its registry.
    identity: EnclaveIdentity,
    /// Track 2: measurements parsed from a real TDX quote at boot.
    /// `Some(_)` ⇒ Health returns ready=true + hex strings; `None` ⇒
    /// Health returns ready=false + empty (simulator path, or boot quote
    /// failed to parse). See `bin/server.rs::main` for the boot probe.
    measurements: Option<crate::tdx_quote::Measurements>,
}

impl TallyServiceImpl {
    pub fn new(dstack: Arc<dyn DstackClient>, identity: EnclaveIdentity) -> Self {
        Self { dstack, identity, measurements: None }
    }

    /// Same as `new`, but stamps in pre-parsed TDX measurements so Health
    /// can surface them. Production boot path uses this; simulator + tests
    /// use `new`.
    pub fn with_measurements(
        dstack: Arc<dyn DstackClient>,
        identity: EnclaveIdentity,
        measurements: crate::tdx_quote::Measurements,
    ) -> Self {
        Self {
            dstack,
            identity,
            measurements: Some(measurements),
        }
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
        let TallyRequest {
            contract_addr,
            election_id,
            candidates,
            raw_ballots,
            chain_id,
            candidate_names,
        } = req.into_inner();

        // 1. KMS handshake. Privkey MUST come from dstack KMS per
        //    docs/runtime-integration.md trust-model section.
        let privkey = self
            .dstack
            .derive_privkey(&contract_addr, election_id)
            .await
            .map_err(|e| Status::internal(format!("dstack derive_privkey: {e}")))?;

        // 2. Stage 1 + Stage 2 (intent §2.5).
        let raw_ballots: RawBallots = raw_ballots
            .into_iter()
            .map(|b| RawEntry { voter: b.voter, ciphertext: b.ciphertext })
            .collect();
        let candidates: Vec<Addr> = candidates;
        let tally = tally_spec(&raw_ballots, &candidates, &privkey);

        // 3. B6 (v0.3.11) — bind raw_ballots into the commit hash so a
        //    host-substituted input set produces a different commit and
        //    the chain rejects with AttestationCommitMismatch.
        let ballots_for_hash: Vec<(String, Vec<u8>)> = raw_ballots
            .iter()
            .map(|e| (e.voter.clone(), e.ciphertext.clone()))
            .collect();
        let ballots_hash = compute_ballots_hash(&candidates, &ballots_for_hash);

        // 4. Attestation artifacts. Default build: synthetic
        //    `(proof, public_inputs)` in the packed dcap-noir layout; real
        //    build (`--features real-zkdcap`): POST quote + collateral to
        //    the noir/bb prove server over a unix socket.
        let (proof, public_inputs) = produce_publish_artifacts(
            self.dstack.as_ref(),
            &self.identity,
            &contract_addr,
            &chain_id,
            election_id,
            &ballots_hash,
            &candidate_names,
            &tally,
        )
        .await
        .map_err(|e| Status::internal(format!("attestation artifacts: {e}")))?;

        let tally_json = serde_json::to_string(&tally)
            .map_err(|e| Status::internal(format!("serialize tally: {e}")))?;

        Ok(Response::new(TallyResponse {
            tally_json,
            proof,
            public_inputs,
        }))
    }

    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        // Track 2 (2026-05-26+): when the boot probe parsed real TDX
        // measurements from a dstack-signed quote, surface them. Otherwise
        // (simulator mode, or quote was malformed) keep the M5 honest-gap
        // fallback: ready=false + empty hex strings, so an operator who
        // populates the on-chain `EnclaveImageRegistry { mrtd, rtmr* }`
        // from this response will only do so against real values.
        //
        // The proto's `rtmr_hex` is conventionally RTMR0 (the build-time
        // measurement most operators register first). RTMR1..3 are not
        // surfaced today; if a deploy needs them, extend the proto.
        match &self.measurements {
            Some(m) => Ok(Response::new(HealthResponse {
                ready: true,
                mrtd_hex: hex::encode(m.mrtd),
                rtmr_hex: hex::encode(m.rtmr0),
            })),
            None => Ok(Response::new(HealthResponse {
                ready: false,
                mrtd_hex: String::new(),
                rtmr_hex: String::new(),
            })),
        }
    }

    async fn register_pubkey(
        &self,
        req: Request<RegisterPubkeyRequest>,
    ) -> Result<Response<RegisterPubkeyResponse>, Status> {
        let RegisterPubkeyRequest {
            contract_addr,
            election_id,
            candidate_names,
        } = req.into_inner();

        // 1. Same KMS derivation context the Tally path uses
        //    (verified-rcv-v1:{contract_addr}:{election_id}) — so the
        //    privkey released by dstack at tally time matches the pubkey
        //    we hand back here.
        let privkey_bytes = self
            .dstack
            .derive_privkey(&contract_addr, election_id)
            .await
            .map_err(|e| Status::internal(format!("dstack derive_privkey: {e}")))?;

        // 2. Derive the SEC1-compressed pubkey (33 bytes, leading 0x02|0x03).
        //    k256 is already a dep via the ECIES decoder; reuse it here
        //    so we don't introduce a second curve impl.
        let signing = k256::ecdsa::SigningKey::from_slice(&privkey_bytes)
            .map_err(|e| Status::internal(format!("k256 from_slice: {e}")))?;
        let pubkey_point = signing.verifying_key().to_encoded_point(true);
        let enclave_pubkey: Vec<u8> = pubkey_point.as_bytes().to_vec();

        // 3. Registration artifacts. v0.3.14 F1: ReportData[0..32] =
        //    SHA-256(enclave_pubkey ‖ borsh_string(contract_addr) ‖
        //    u64_LE(election_id) ‖ names_hash); ReportData[32..58] =
        //    DST_VERIFIED_RCV_PUBKEY_V1. The (contract_addr, election_id,
        //    names_hash) triple binding blocks an admin replaying an old
        //    registration quote across elections AND blocks an admin
        //    swapping candidate display names between CreateElection and
        //    PublishResult (defense-in-depth alongside the publish-time
        //    names_hash check).
        let (proof, public_inputs) = produce_registration_artifacts(
            self.dstack.as_ref(),
            &self.identity,
            &enclave_pubkey,
            &contract_addr,
            election_id,
            &candidate_names,
        )
        .await
        .map_err(|e| Status::internal(format!("registration artifacts: {e}")))?;

        Ok(Response::new(RegisterPubkeyResponse {
            enclave_pubkey,
            proof,
            public_inputs,
        }))
    }
}

// The simulator-backed server tests below exercise the synthetic
// `produce_artifacts_inner` stub under default features. Under
// `--features real-zkdcap`, `produce_artifacts_inner` calls
// `dcap_qvl::collateral::get_collateral_from_pcs` on the simulator's
// mock quote, which fails as "Unsupported quote version" (the mock
// isn't a real TDX quote). The integration test in
// `crates/enclave/tests/real_zkdcap.rs` covers the real path against a
// real dstack quote + a live prover socket.
#[cfg(all(test, not(feature = "real-zkdcap")))]
mod tests {
    use super::proto::tally_service_client::TallyServiceClient;
    use super::proto::tally_service_server::TallyServiceServer;
    use super::*;
    use crate::dstack::SimulatorDstackClient;
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

    fn dev_identity() -> EnclaveIdentity {
        EnclaveIdentity {
            mrtd: [0u8; 48],
            rtmr0: [0u8; 48],
            rtmr1: [0u8; 48],
            rtmr2: [0u8; 48],
            rtmr3: [0u8; 48],
            tcb_status: 0,
            timestamp: 1_700_000_000,
        }
    }

    /// Phase 1 / brief test 1: spin up the gRPC server with the simulator,
    /// send a Tally RPC with the 3-candidate roundtrip fixture, assert the
    /// response carries a valid TallyResult and a non-empty
    /// `(proof, public_inputs)` pair whose layout the chain expects.
    #[tokio::test]
    async fn server_smoke() {
        let (sk, pk) = generate_keypair();
        let mut priv32 = [0u8; 32];
        priv32.copy_from_slice(&sk.serialize());
        let pubkey = pk.serialize();

        let cands = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        // ECIES is nondeterministic (ephemeral key per encrypt). Capture
        // the ciphertexts ONCE and reuse for the RPC + the expected-hash
        // computation; otherwise the server's hash and the test's expected
        // hash will diverge by random bytes.
        let ct_a = encrypt_for_test(&["A", "B", "C"], &pubkey);
        let ct_b = encrypt_for_test(&["A", "C", "B"], &pubkey);
        let raw_ballots = vec![
            proto::RawBallot { voter: "A".to_string(), ciphertext: ct_a.clone() },
            proto::RawBallot { voter: "B".to_string(), ciphertext: ct_b.clone() },
        ];

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let svc = TallyServiceImpl::new(
            Arc::new(SimulatorDstackClient::with_privkey(priv32)),
            dev_identity(),
        );
        let server_task = tokio::spawn(async move {
            Server::builder()
                .add_service(TallyServiceServer::new(svc))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TallyServiceClient::connect(format!("http://{addr}"))
            .await
            .expect("client connect");
        let names = vec!["Ada".to_string(), "Bea".to_string(), "Cyd".to_string()];
        let resp = client
            .tally(Request::new(TallyRequest {
                contract_addr: "xion1contract".to_string(),
                election_id: 42,
                candidates: cands.clone(),
                raw_ballots,
                chain_id: "xion-test-1".to_string(),
                candidate_names: names.clone(),
            }))
            .await
            .expect("tally rpc")
            .into_inner();

        let tally: verified_rcv_enclave_core::TallyResult =
            serde_json::from_str(&resp.tally_json).expect("parse tally JSON");
        assert_eq!(tally.winners, vec!["A".to_string()]);
        assert_eq!(tally.ballots_tallied, 2);
        assert_eq!(tally.ballots_dropped, 0);
        assert_eq!(tally.non_voters, vec!["C".to_string()]);

        // The response carries (proof, public_inputs) — assert shapes
        // match the chain's expectations.
        assert!(!resp.proof.is_empty(), "proof bytes present");
        assert_eq!(
            resp.public_inputs.len(),
            crate::attestation::ULTRAHONK_PUBLIC_INPUTS_LEN,
            "public_inputs length matches the packed dcap-noir layout"
        );

        // ReportData[0..32] in the public_inputs should equal the commit
        // hash for this (contract_addr, election_id, tally). Unpack the
        // ReportData from the packed limbs and compare the low half.
        let rd = crate::attestation::extract_report_data(&resp.public_inputs)
            .expect("packed public_inputs decodes");
        let rd_low: [u8; 32] = rd[..32].try_into().unwrap();
        // B6 (v0.3.11): server.rs hashed raw_ballots in candidate-declaration
        // order; replicate that here for the expected ReportData.
        let cands_str: Vec<String> =
            cands.iter().map(|c| c.to_string()).collect();
        let ballots_for_hash: Vec<(String, Vec<u8>)> = vec![
            ("A".to_string(), ct_a),
            ("B".to_string(), ct_b),
        ];
        let bh = crate::attestation::compute_ballots_hash(&cands_str, &ballots_for_hash);
        let nh = crate::attestation::compute_names_hash(&names);
        let expected_rd = crate::attestation::build_publish_report_data(
            "xion1contract",
            "xion-test-1",
            42,
            &bh,
            &nh,
            &tally,
        );
        assert_eq!(rd_low, expected_rd[..32], "ReportData[0..32] = commit_hash");

        server_task.abort();
    }

    /// Track 3 smoke: derive enclave_pubkey + registration artifacts via
    /// RegisterPubkey RPC, assert pubkey shape + ReportData layout.
    #[tokio::test]
    async fn register_pubkey_smoke() {
        use sha2::{Digest, Sha256};

        // Deterministic fixed simulator key so the derivation is reproducible.
        let priv32 = [0x42u8; 32];

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let svc = TallyServiceImpl::new(
            Arc::new(SimulatorDstackClient::with_privkey(priv32)),
            dev_identity(),
        );
        let server_task = tokio::spawn(async move {
            Server::builder()
                .add_service(TallyServiceServer::new(svc))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TallyServiceClient::connect(format!("http://{addr}"))
            .await
            .expect("client connect");
        let reg_names = vec!["Ada".to_string(), "Bea".to_string(), "Cyd".to_string()];
        let resp = client
            .register_pubkey(Request::new(proto::RegisterPubkeyRequest {
                contract_addr: "xion1contract".to_string(),
                election_id: 7,
                candidate_names: reg_names.clone(),
            }))
            .await
            .expect("register_pubkey rpc")
            .into_inner();

        // (a) Pubkey shape: SEC1 compressed = 33 bytes, leading 0x02 or 0x03.
        assert_eq!(
            resp.enclave_pubkey.len(),
            33,
            "enclave_pubkey is SEC1-compressed (33 bytes)"
        );
        assert!(
            resp.enclave_pubkey[0] == 0x02 || resp.enclave_pubkey[0] == 0x03,
            "enclave_pubkey leading byte is 0x02 or 0x03 (got 0x{:02x})",
            resp.enclave_pubkey[0]
        );

        // (b) public_inputs length matches the packed dcap-noir layout.
        assert_eq!(
            resp.public_inputs.len(),
            crate::attestation::ULTRAHONK_PUBLIC_INPUTS_LEN
        );

        // (c) v0.3.12 N22 / v0.3.14 F1: ReportData[0..32] =
        //     SHA-256(enclave_pubkey ‖ borsh_string(contract_addr) ‖
        //     u64_LE(election_id) ‖ names_hash). Use the runtime's canonical
        //     builder to derive the expected hash bytewise, so this test
        //     fails loudly if either side drifts.
        let rd = crate::attestation::extract_report_data(&resp.public_inputs)
            .expect("packed public_inputs decodes");
        let rd_low: [u8; 32] = rd[..32].try_into().unwrap();
        let expected_nh = crate::attestation::compute_names_hash(&reg_names);
        let expected_rd = crate::attestation::build_registration_report_data(
            &resp.enclave_pubkey,
            "xion1contract",
            7,
            &expected_nh,
        );
        assert_eq!(rd_low, expected_rd[..32], "ReportData[0..32] = F1 preimage hash");
        let _ = Sha256::new(); // keep sha2 import warm for future tests

        // (d) ReportData[32..58] = DST_VERIFIED_RCV_PUBKEY_V1.
        let rd_high: [u8; 32] = rd[32..64].try_into().unwrap();
        assert_eq!(
            &rd_high[..crate::attestation::DST_PUBKEY.len()],
            crate::attestation::DST_PUBKEY,
            "ReportData[32..58] = DST_VERIFIED_RCV_PUBKEY_V1"
        );

        // (e) Proof bytes present (sentinel under default features).
        assert!(!resp.proof.is_empty(), "proof bytes present");

        server_task.abort();
    }

    /// Track 2 smoke: when `with_measurements` is used, Health returns the
    /// cached MRTD + RTMR0 hex strings and ready=true. Without it (the
    /// simulator boot path), Health returns ready=false + empty hex.
    #[tokio::test]
    async fn health_returns_cached_measurements() {
        let priv32 = [0x11u8; 32];

        // Path 1: no measurements cached.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let svc_no_m = TallyServiceImpl::new(
            Arc::new(SimulatorDstackClient::with_privkey(priv32)),
            dev_identity(),
        );
        let task_no_m = tokio::spawn(async move {
            Server::builder()
                .add_service(TallyServiceServer::new(svc_no_m))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut client = TallyServiceClient::connect(format!("http://{addr}"))
            .await
            .expect("client connect");
        let resp = client
            .health(Request::new(proto::HealthRequest {}))
            .await
            .unwrap()
            .into_inner();
        assert!(!resp.ready, "no measurements ⇒ ready=false");
        assert!(resp.mrtd_hex.is_empty());
        assert!(resp.rtmr_hex.is_empty());
        task_no_m.abort();

        // Path 2: with cached measurements.
        let m = crate::tdx_quote::Measurements {
            mrtd: [0xAA; 48],
            rtmr0: [0xBB; 48],
            rtmr1: [0xCC; 48],
            rtmr2: [0xDD; 48],
            rtmr3: [0xEE; 48],
        };
        let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = listener2.local_addr().unwrap();
        let svc_with_m = TallyServiceImpl::with_measurements(
            Arc::new(SimulatorDstackClient::with_privkey(priv32)),
            dev_identity(),
            m.clone(),
        );
        let task_with_m = tokio::spawn(async move {
            Server::builder()
                .add_service(TallyServiceServer::new(svc_with_m))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener2))
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut client2 = TallyServiceClient::connect(format!("http://{addr2}"))
            .await
            .expect("client2 connect");
        let resp2 = client2
            .health(Request::new(proto::HealthRequest {}))
            .await
            .unwrap()
            .into_inner();
        assert!(resp2.ready, "cached measurements ⇒ ready=true");
        assert_eq!(resp2.mrtd_hex, hex::encode(m.mrtd));
        assert_eq!(resp2.rtmr_hex, hex::encode(m.rtmr0));
        task_with_m.abort();
    }

    /// Determinism: same (contract_addr, election_id) → same pubkey, same proof.
    #[tokio::test]
    async fn register_pubkey_deterministic_for_same_input() {
        let priv32 = [0x33u8; 32];

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let svc = TallyServiceImpl::new(
            Arc::new(SimulatorDstackClient::with_privkey(priv32)),
            dev_identity(),
        );
        let server_task = tokio::spawn(async move {
            Server::builder()
                .add_service(TallyServiceServer::new(svc))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut client = TallyServiceClient::connect(format!("http://{addr}"))
            .await
            .expect("client connect");
        let mk = || proto::RegisterPubkeyRequest {
            contract_addr: "xion1abc".to_string(),
            election_id: 99,
            candidate_names: vec!["Ada".to_string(), "Bea".to_string()],
        };
        let a = client.register_pubkey(Request::new(mk())).await.unwrap().into_inner();
        let b = client.register_pubkey(Request::new(mk())).await.unwrap().into_inner();
        assert_eq!(a.enclave_pubkey, b.enclave_pubkey);
        assert_eq!(a.public_inputs, b.public_inputs);

        server_task.abort();
    }
}
