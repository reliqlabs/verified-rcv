//! Track 1 integration test for `--features real-zkdcap`.
//!
//! Only runs end-to-end when:
//! 1. The crate is built with `--features real-zkdcap`; AND
//! 2. `ZKDCAP_PROVER_SOCKET` is set to a unix socket where the noir/bb
//!    prove server is listening (default /run/noir/prove.sock); AND
//! 3. `DSTACK_SOCKET` is set to a real dstack guest-agent socket (so
//!    `get_quote` returns a parseable TDX quote — the simulator's mock
//!    quote will fail `dcap-qvl`'s collateral fetch).
//!
//! Otherwise the test no-ops (prints "skipped" and returns Ok), so CI
//! without the prover deps still passes.

#[cfg(feature = "real-zkdcap")]
#[tokio::test]
async fn real_zkdcap_round_trip_or_skip() {
    use std::env;
    use verified_rcv_enclave::attestation::{produce_registration_artifacts, EnclaveIdentity};
    use verified_rcv_enclave::dstack::HttpDstackClient;

    let prover_sock = match env::var("ZKDCAP_PROVER_SOCKET") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("ZKDCAP_PROVER_SOCKET unset — skipping real-zkdcap round trip");
            return;
        }
    };
    if env::var("DSTACK_SOCKET").is_err() {
        eprintln!("DSTACK_SOCKET unset — skipping (need a real dstack quote)");
        return;
    }
    eprintln!("real-zkdcap round trip: prover_sock={prover_sock}");

    let dstack = HttpDstackClient::from_env();
    let identity = EnclaveIdentity {
        mrtd: [0u8; 48],
        rtmr0: [0u8; 48],
        rtmr1: [0u8; 48],
        rtmr2: [0u8; 48],
        rtmr3: [0u8; 48],
        tcb_status: 0,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    // Use the registration path — simpler preimage and doesn't require a
    // synthesised TallyResult fixture.
    let pk = vec![0x02u8; 33];
    let names = vec!["Ada".to_string(), "Bea".to_string()];
    let (proof, public_inputs) = produce_registration_artifacts(
        &dstack,
        &identity,
        &pk,
        "xion1real-zkdcap-smoke",
        7,
        &names,
    )
    .await
    .expect("produce_registration_artifacts with real-zkdcap");

    assert!(!proof.is_empty(), "real prover returned a non-empty proof");
    assert_eq!(
        public_inputs.len(),
        verified_rcv_enclave::attestation::ULTRAHONK_PUBLIC_INPUTS_LEN,
        "public_inputs length matches the packed dcap-noir layout"
    );
}

#[cfg(not(feature = "real-zkdcap"))]
#[test]
fn real_zkdcap_feature_required() {
    eprintln!("real-zkdcap feature not enabled — integration test skipped");
}
