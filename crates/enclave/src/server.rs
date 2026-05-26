//! gRPC `TallyService` impl (Phase 1 per docs/runtime-integration.md).
//!
//! v0.3.9 N1: the response now carries `(proof, public_inputs)` instead
//! of a `DstackAttestation` envelope JSON. The orchestrator forwards
//! these directly to the contract's `PublishResult` execute message.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use verified_rcv_enclave_core::{Addr, RawBallots, RawEntry};

use crate::attestation::{compute_ballots_hash, produce_publish_artifacts, EnclaveIdentity};
use crate::dstack::DstackClient;
use crate::tally_spec;

pub mod proto {
    tonic::include_proto!("verified_rcv");
}

use proto::tally_service_server::{TallyService, TallyServiceServer};
use proto::{HealthRequest, HealthResponse, TallyRequest, TallyResponse};

pub struct TallyServiceImpl {
    dstack: Arc<dyn DstackClient>,
    /// The enclave identity tuple to attest over. In dev / simulator
    /// mode this is whatever the operator configures; in real-TDX mode
    /// this comes from a TDX-quote parser at boot. The mock-attestation
    /// path on the chain doesn't verify the gnark proof, so the
    /// concrete values matter only for the chain's measurement equality
    /// check against its registry.
    identity: EnclaveIdentity,
}

impl TallyServiceImpl {
    pub fn new(dstack: Arc<dyn DstackClient>, identity: EnclaveIdentity) -> Self {
        Self { dstack, identity }
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

        // 4. v0.3.9 N1 attestation artifacts. Default build: synthetic
        //    `(proof, public_inputs)` matching §2.5 byte layout; real
        //    build (`--features real-zkdcap`): drive the zkdcap gnark
        //    prover via unix socket.
        let (proof, public_inputs) = produce_publish_artifacts(
            &self.identity,
            &contract_addr,
            &chain_id,
            election_id,
            &ballots_hash,
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
        // Audit remediation M5 (2026-05-26):
        //
        // The honest answer for `mrtd_hex` / `rtmr_hex` requires parsing
        // a real TDX quote, which we don't have yet. The previous
        // implementation returned `compose_hash` (a dstack image
        // identifier) in the `mrtd_hex` slot, which would lead an operator
        // following docs/deploy.md to populate the on-chain
        // `EnclaveImageRegistry { mrtd, rtmr* }` with the wrong value —
        // making every subsequent attestation fail verification (or, worse,
        // pass against the wrong baseline).
        //
        // We now return `ready = false` and an EMPTY `mrtd_hex` /
        // `rtmr_hex` until a TDX-quote parser is wired.
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
        let resp = client
            .tally(Request::new(TallyRequest {
                contract_addr: "xion1contract".to_string(),
                election_id: 42,
                candidates: cands.clone(),
                raw_ballots,
                chain_id: "xion-test-1".to_string(),
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

        // v0.3.9 N1: the response carries (proof, public_inputs) — assert
        // shapes match the chain's expectations.
        assert!(!resp.proof.is_empty(), "proof bytes present");
        assert_eq!(
            resp.public_inputs.len(),
            crate::attestation::GNARK_PUBLIC_INPUTS_LEN,
            "public_inputs length matches §2.5 layout"
        );

        // ReportData[0..32] in the public_inputs should equal the commit
        // hash for this (contract_addr, election_id, tally). Extract the
        // first 32 ReportData bytes (elements 240..272) and compare.
        let pi = &resp.public_inputs;
        let mut rd_low = [0u8; 32];
        for i in 0..32 {
            rd_low[i] = pi[(240 + i) * 32 + 31];
        }
        // B6 (v0.3.11): server.rs hashed raw_ballots in candidate-declaration
        // order; replicate that here for the expected ReportData.
        let cands_str: Vec<String> =
            cands.iter().map(|c| c.to_string()).collect();
        let ballots_for_hash: Vec<(String, Vec<u8>)> = vec![
            ("A".to_string(), ct_a),
            ("B".to_string(), ct_b),
        ];
        let bh = crate::attestation::compute_ballots_hash(&cands_str, &ballots_for_hash);
        let expected_rd = crate::attestation::build_publish_report_data(
            "xion1contract",
            "xion-test-1",
            42,
            &bh,
            &tally,
        );
        assert_eq!(rd_low, expected_rd[..32], "ReportData[0..32] = commit_hash");

        server_task.abort();
    }
}
