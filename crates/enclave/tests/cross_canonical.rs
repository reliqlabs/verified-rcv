//! Cross-test: the runtime's `canonical_serialization` and the contract's
//! `canonical_serialization` MUST produce byte-identical output for the
//! same `(contract_addr, election_id, tally_body)`. They live in separate
//! crates (the contract is wasm; the runtime pulls tokio/tonic/etc.), so
//! we duplicate the implementation rather than share it via a third crate.
//! This test is the canary that catches divergence.
//!
//! v0.3.9 N1 update: also cross-checks the v0.3.9 ReportData /
//! public_inputs construction across the runtime and contract crates.

use verified_rcv_contract::contract as contract_impl;
use verified_rcv_enclave::attestation as runtime_impl;
use verified_rcv_enclave_core::{RoundCount, TallyResult};

fn sample_tally() -> TallyResult {
    TallyResult {
        winners: vec!["alice".to_string(), "carol".to_string()],
        per_round_counts: vec![
            vec![
                RoundCount { candidate: "alice".to_string(), count: 2 },
                RoundCount { candidate: "bob".to_string(), count: 1 },
                RoundCount { candidate: "carol".to_string(), count: 2 },
            ],
            vec![
                RoundCount { candidate: "alice".to_string(), count: 2 },
                RoundCount { candidate: "carol".to_string(), count: 3 },
            ],
        ],
        eliminated_by_round: vec![vec!["bob".to_string()]],
        ballots_tallied: 5,
        ballots_dropped: 1,
        dropped_voters: vec!["dave".to_string()],
        non_voters: vec!["eve".to_string()],
    }
}

#[test]
fn canonical_serialization_contract_vs_runtime_byte_identical() {
    let tally = sample_tally();
    let contract_bytes = contract_impl::canonical_serialization("xion1abcdef", 42, &tally);
    let runtime_bytes = runtime_impl::canonical_serialization("xion1abcdef", 42, &tally);
    assert_eq!(
        contract_bytes, runtime_bytes,
        "canonical_serialization divergence between contract and runtime"
    );
}

#[test]
fn canonical_serialization_election_id_affects_bytes() {
    let tally = sample_tally();
    let a = runtime_impl::canonical_serialization("xion1addr", 1, &tally);
    let b = runtime_impl::canonical_serialization("xion1addr", 2, &tally);
    assert_ne!(a, b, "election_id must affect canonical_serialization bytes (M2)");
}

#[test]
fn canonical_serialization_empty_tally_election_id_only() {
    let tally = TallyResult {
        winners: vec![],
        per_round_counts: vec![],
        eliminated_by_round: vec![],
        ballots_tallied: 0,
        ballots_dropped: 0,
        dropped_voters: vec![],
        non_voters: vec![],
    };
    let bytes_e1 = runtime_impl::canonical_serialization("a", 1, &tally);
    let bytes_e2 = runtime_impl::canonical_serialization("a", 2, &tally);
    assert_eq!(bytes_e1[0..5], bytes_e2[0..5]); // contract_addr identical
    assert_eq!(bytes_e1[5..13], 1u64.to_le_bytes());
    assert_eq!(bytes_e2[5..13], 2u64.to_le_bytes());
    assert_eq!(bytes_e1[13..], bytes_e2[13..]); // tally_body identical
}

#[test]
fn commit_hash_matches_runtime_publish_report_data() {
    // v0.3.9 N1: the contract's `compute_commit_hash` lower 32 must equal
    // the runtime's `build_publish_report_data` lower 32 (the SHA-256 over
    // canonical_serialization). This is the load-bearing equivalence for
    // B8(c) under the gnark path.
    use sha2::{Digest, Sha256};
    let tally = sample_tally();
    let contract_commit = contract_impl::compute_commit_hash("xion1c", 7, &tally);
    let runtime_rd = runtime_impl::build_publish_report_data("xion1c", 7, &tally);
    assert_eq!(&runtime_rd[..32], &contract_commit[..]);

    // Independent SHA-256 sanity.
    let canonical = runtime_impl::canonical_serialization("xion1c", 7, &tally);
    let expected = Sha256::digest(&canonical);
    assert_eq!(&runtime_rd[..32], &expected[..]);
}

#[test]
fn publish_report_data_dst_matches_contract_layout() {
    // ReportData[32..64] = DST_VERIFIED_RCV_TALLY_V1 zero-padded; same
    // literal used contract-side. The contract's `build_publish_report_data`
    // and runtime's `build_publish_report_data` should produce identical
    // 64-byte buffers.
    let tally = sample_tally();
    let commit = contract_impl::compute_commit_hash("xion1c", 7, &tally);
    let contract_rd = contract_impl::build_publish_report_data(&commit);
    let runtime_rd = runtime_impl::build_publish_report_data("xion1c", 7, &tally);
    assert_eq!(contract_rd, runtime_rd);
}

#[test]
fn registration_report_data_dst_matches_contract_layout() {
    let pk = vec![0x02u8; 33];
    let contract_rd = contract_impl::build_registration_report_data(&pk);
    let runtime_rd = runtime_impl::build_registration_report_data(&pk);
    assert_eq!(contract_rd, runtime_rd);
}

#[test]
fn synthetic_public_inputs_round_trip_contract_extraction() {
    // The runtime's `build_public_inputs` plus the contract's
    // `extract_measurement_48` / `extract_report_data` must round-trip
    // identical bytes — this is what the contract relies on at every
    // PublishResult.
    let mut mrtd = [0u8; 48];
    for i in 0..48 {
        mrtd[i] = (i as u8).wrapping_add(0x10);
    }
    let mut rtmr1 = [0u8; 48];
    for i in 0..48 {
        rtmr1[i] = (i as u8).wrapping_add(0x20);
    }
    let mut rd = [0u8; 64];
    for i in 0..64 {
        rd[i] = (i as u8).wrapping_add(0xA0);
    }
    let pi = runtime_impl::build_public_inputs(
        &mrtd, &[0; 48], &rtmr1, &[0; 48], &[0; 48], &rd, 3, 1_700_000_000,
    );
    // Element offsets — see contract.rs ELEM_* constants.
    let extracted_mrtd = contract_impl::extract_measurement_48(&pi, 0).unwrap();
    assert_eq!(extracted_mrtd, mrtd);
    let extracted_rtmr1 = contract_impl::extract_measurement_48(&pi, 96).unwrap();
    assert_eq!(extracted_rtmr1, rtmr1);
    let extracted_rd = contract_impl::extract_report_data(&pi).unwrap();
    assert_eq!(extracted_rd, rd);
}
