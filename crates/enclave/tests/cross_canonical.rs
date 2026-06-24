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

/// Empty ballots_hash (no entries). v0.3.11 B6 cross-test fixture.
fn empty_bh() -> [u8; 32] {
    runtime_impl::compute_ballots_hash(&[], &[])
}

/// Empty names_hash (no candidate names). v0.3.14 cross-test fixture.
fn empty_nh() -> [u8; 32] {
    runtime_impl::compute_names_hash(&[])
}

#[test]
fn canonical_serialization_contract_vs_runtime_byte_identical() {
    let tally = sample_tally();
    let bh = empty_bh();
    let nh = empty_nh();
    let contract_bytes = contract_impl::canonical_serialization(
        "xion1abcdef", "xion-1", 42, &bh, &nh, &tally,
    );
    let runtime_bytes = runtime_impl::canonical_serialization(
        "xion1abcdef", "xion-1", 42, &bh, &nh, &tally,
    );
    assert_eq!(
        contract_bytes, runtime_bytes,
        "canonical_serialization divergence between contract and runtime"
    );
}

#[test]
fn canonical_serialization_election_id_affects_bytes() {
    let tally = sample_tally();
    let bh = empty_bh();
    let nh = empty_nh();
    let a = runtime_impl::canonical_serialization("xion1addr", "cid", 1, &bh, &nh, &tally);
    let b = runtime_impl::canonical_serialization("xion1addr", "cid", 2, &bh, &nh, &tally);
    assert_ne!(a, b, "election_id must affect canonical_serialization bytes (M2)");
}

#[test]
fn canonical_serialization_chain_id_affects_bytes() {
    let tally = sample_tally();
    let bh = empty_bh();
    let nh = empty_nh();
    let a = runtime_impl::canonical_serialization("xion1addr", "xion-1", 7, &bh, &nh, &tally);
    let b = runtime_impl::canonical_serialization("xion1addr", "xion-2", 7, &bh, &nh, &tally);
    assert_ne!(a, b, "chain_id must affect canonical_serialization bytes (N4)");
}

#[test]
fn canonical_serialization_ballots_hash_affects_bytes() {
    // B6 (v0.3.11): different ballots_hash MUST produce different commit
    // preimages -- closes §8.7 link 7 (enclave_input_fidelity).
    let tally = sample_tally();
    let bh_empty = empty_bh();
    let bh_one = runtime_impl::compute_ballots_hash(
        &["alice".to_string()],
        &[("alice".to_string(), vec![1, 2, 3])],
    );
    let nh = empty_nh();
    let a = runtime_impl::canonical_serialization("xion1c", "xion-1", 7, &bh_empty, &nh, &tally);
    let b = runtime_impl::canonical_serialization("xion1c", "xion-1", 7, &bh_one, &nh, &tally);
    assert_ne!(a, b, "ballots_hash must affect canonical_serialization bytes (B6)");
}

#[test]
fn canonical_serialization_names_hash_affects_bytes() {
    // v0.3.14: different names_hash MUST produce different commit
    // preimages -- closes the off-chain admin-name substitution gap (B11).
    let tally = sample_tally();
    let bh = empty_bh();
    let nh_a = runtime_impl::compute_names_hash(&["Alice".to_string(), "Bob".to_string()]);
    let nh_b = runtime_impl::compute_names_hash(&["Carol".to_string(), "Dan".to_string()]);
    let a = runtime_impl::canonical_serialization("xion1c", "xion-1", 7, &bh, &nh_a, &tally);
    let b = runtime_impl::canonical_serialization("xion1c", "xion-1", 7, &bh, &nh_b, &tally);
    assert_ne!(a, b, "names_hash must affect canonical_serialization bytes (v0.3.14)");
}

#[test]
fn canonical_serialization_empty_tally_position_check() {
    // Sanity layout: contract_addr="a" (5 bytes: 4-len + 1 char),
    // chain_id="b" (5 bytes), election_id (8 bytes), ballots_hash (32 bytes),
    // names_hash (32 bytes).
    let tally = TallyResult {
        winners: vec![], per_round_counts: vec![], eliminated_by_round: vec![],
        ballots_tallied: 0, ballots_dropped: 0,
        dropped_voters: vec![], non_voters: vec![],
    };
    let bh = empty_bh();
    let nh = empty_nh();
    let bytes_e1 = runtime_impl::canonical_serialization("a", "b", 1, &bh, &nh, &tally);
    let bytes_e2 = runtime_impl::canonical_serialization("a", "b", 2, &bh, &nh, &tally);
    assert_eq!(bytes_e1[0..5], bytes_e2[0..5]);   // contract_addr
    assert_eq!(bytes_e1[5..10], bytes_e2[5..10]); // chain_id
    assert_eq!(bytes_e1[10..18], 1u64.to_le_bytes());
    assert_eq!(bytes_e2[10..18], 2u64.to_le_bytes());
    assert_eq!(bytes_e1[18..50], bh);             // ballots_hash 32 bytes
    assert_eq!(bytes_e2[18..50], bh);
    assert_eq!(bytes_e1[50..82], nh);             // names_hash 32 bytes
    assert_eq!(bytes_e2[50..82], nh);
    assert_eq!(bytes_e1[82..], bytes_e2[82..]);
}

#[test]
fn commit_hash_matches_runtime_publish_report_data() {
    use sha2::{Digest, Sha256};
    let tally = sample_tally();
    let bh = empty_bh();
    let nh = empty_nh();
    let contract_commit =
        contract_impl::compute_commit_hash("xion1c", "xion-1", 7, &bh, &nh, &tally);
    let runtime_rd =
        runtime_impl::build_publish_report_data("xion1c", "xion-1", 7, &bh, &nh, &tally);
    assert_eq!(&runtime_rd[..32], &contract_commit[..]);
    let canonical =
        runtime_impl::canonical_serialization("xion1c", "xion-1", 7, &bh, &nh, &tally);
    let expected = Sha256::digest(&canonical);
    assert_eq!(&runtime_rd[..32], &expected[..]);
}

#[test]
fn publish_report_data_dst_matches_contract_layout() {
    let tally = sample_tally();
    let bh = empty_bh();
    let nh = empty_nh();
    let commit = contract_impl::compute_commit_hash("xion1c", "xion-1", 7, &bh, &nh, &tally);
    let contract_rd = contract_impl::build_publish_report_data(&commit);
    let runtime_rd =
        runtime_impl::build_publish_report_data("xion1c", "xion-1", 7, &bh, &nh, &tally);
    assert_eq!(contract_rd, runtime_rd);
}

#[test]
fn compute_names_hash_contract_vs_runtime_byte_identical() {
    // v0.3.14: the chain's compute_names_hash and the runtime's
    // compute_names_hash MUST produce byte-identical hashes for the same
    // candidate_names input. This is the load-bearing cross-test for the
    // names-binding leg of canonical_serialization (B11).
    let names = vec![
        "Alice".to_string(),
        "Bob".to_string(),
        "Carol Q. \u{00e9}".to_string(), // mix in a 2-byte UTF-8 codepoint
    ];
    let runtime_nh = runtime_impl::compute_names_hash(&names);
    let contract_nh = contract_impl::compute_names_hash(&names);
    assert_eq!(
        runtime_nh, contract_nh,
        "compute_names_hash divergence between contract and runtime"
    );

    // Empty input also byte-identical.
    let runtime_empty = runtime_impl::compute_names_hash(&[]);
    let contract_empty = contract_impl::compute_names_hash(&[]);
    assert_eq!(runtime_empty, contract_empty);
}

#[test]
fn compute_ballots_hash_contract_vs_runtime_byte_identical() {
    // B6 (v0.3.11): the chain's compute_ballots_hash and the runtime's
    // compute_ballots_hash MUST produce byte-identical hashes for the same
    // (candidates, ballots) input. This is the load-bearing cross-test
    // for §8.7 link 7 (enclave_input_fidelity).
    use cosmwasm_std::{Addr, HexBinary};
    let candidates_str = vec!["alice".to_string(), "bob".to_string(), "carol".to_string()];
    let candidates_addr: Vec<Addr> = candidates_str
        .iter()
        .map(|s| Addr::unchecked(s.clone()))
        .collect();
    let ballots_str: Vec<(String, Vec<u8>)> = vec![
        ("alice".to_string(), vec![0xAA; 8]),
        ("carol".to_string(), vec![0xCC; 12]),
        // bob didn't vote
    ];
    let ballots_addr: Vec<(Addr, HexBinary)> = ballots_str
        .iter()
        .map(|(v, c)| (Addr::unchecked(v.clone()), HexBinary::from(c.clone())))
        .collect();

    let runtime_bh = runtime_impl::compute_ballots_hash(&candidates_str, &ballots_str);
    let contract_bh = contract_impl::compute_ballots_hash(&candidates_addr, &ballots_addr);
    assert_eq!(
        runtime_bh, contract_bh,
        "compute_ballots_hash divergence between contract and runtime"
    );
}

#[test]
fn registration_report_data_dst_matches_contract_layout() {
    // v0.3.12 N22: ReportData[0..32] binds (enclave_pubkey,
    // contract_addr, election_id).
    // v0.3.14 F1: ReportData[0..32] also binds names_hash. Cross-test must
    // pass identical (contract_addr, election_id, names_hash) to both
    // implementations.
    let pk = vec![0x02u8; 33];
    let names = vec!["Alice".to_string(), "Bob".to_string()];
    let runtime_nh = runtime_impl::compute_names_hash(&names);
    let contract_nh = contract_impl::compute_names_hash(&names);
    assert_eq!(runtime_nh, contract_nh, "names_hash cross-byte-identity");
    let contract_rd =
        contract_impl::build_registration_report_data(&pk, "xion1addr", 7, &contract_nh);
    let runtime_rd =
        runtime_impl::build_registration_report_data(&pk, "xion1addr", 7, &runtime_nh);
    assert_eq!(contract_rd, runtime_rd);
}

/// v0.3.14 F1: the registration ReportData differs when names differ. This
/// is the load-bearing cross-test that names_hash actually feeds into the
/// registration binding (defense-in-depth alongside the publish-time check).
#[test]
fn registration_report_data_binds_names_hash() {
    let pk = vec![0x02u8; 33];
    let names_a = vec!["Alice".to_string(), "Bob".to_string()];
    let names_b = vec!["Carol".to_string(), "Dan".to_string()];
    let nh_a = runtime_impl::compute_names_hash(&names_a);
    let nh_b = runtime_impl::compute_names_hash(&names_b);
    let rd_a = runtime_impl::build_registration_report_data(&pk, "xion1addr", 7, &nh_a);
    let rd_b = runtime_impl::build_registration_report_data(&pk, "xion1addr", 7, &nh_b);
    assert_ne!(
        rd_a[..32],
        rd_b[..32],
        "names_hash must affect registration ReportData (F1)"
    );
    // Contract side matches.
    let crd_a = contract_impl::build_registration_report_data(&pk, "xion1addr", 7, &nh_a);
    let crd_b = contract_impl::build_registration_report_data(&pk, "xion1addr", 7, &nh_b);
    assert_eq!(rd_a, crd_a);
    assert_eq!(rd_b, crd_b);
}

#[test]
fn synthetic_public_inputs_round_trip_contract_extraction() {
    // The runtime's `build_public_inputs` plus the contract's
    // `extract_measurement_48` / `extract_report_data` must round-trip
    // identical bytes — this is what the contract relies on at every
    // PublishResult.
    let mut mrtd = [0u8; 48];
    for (i, b) in mrtd.iter_mut().enumerate() {
        *b = (i as u8).wrapping_add(0x10);
    }
    let mut rtmr1 = [0u8; 48];
    for (i, b) in rtmr1.iter_mut().enumerate() {
        *b = (i as u8).wrapping_add(0x20);
    }
    let mut rd = [0u8; 64];
    for (i, b) in rd.iter_mut().enumerate() {
        *b = (i as u8).wrapping_add(0xA0);
    }
    // _full builder sets the recency/validity scalars (fields 17-19) so the
    // cross-test pins them too. Field 14 timestamp = 1_700_000_000.
    let pi = runtime_impl::build_public_inputs_full(
        &mrtd, &[0; 48], &rtmr1, &[0; 48], &[0; 48], &rd, 3, 1_700_000_000, 42, 111, 222,
    );
    // Packed dcap-noir field indices — see contract.rs F_* constants.
    // mr_td starts at field 0; rtmr1 at field 4 (2 limbs per register).
    let extracted_mrtd = contract_impl::extract_measurement_48(&pi, 0).unwrap();
    assert_eq!(extracted_mrtd, mrtd);
    let extracted_rtmr1 = contract_impl::extract_measurement_48(&pi, 4).unwrap();
    assert_eq!(extracted_rtmr1, rtmr1);
    let extracted_rd = contract_impl::extract_report_data(&pi).unwrap();
    assert_eq!(extracted_rd, rd);
    // 20-field circuit: the new scalar fields (17-19) round-trip across crates.
    assert_eq!(contract_impl::extract_tcb_eval_num(&pi).unwrap(), 42);
    assert_eq!(contract_impl::extract_valid_from(&pi).unwrap(), 111);
    assert_eq!(contract_impl::extract_valid_until(&pi).unwrap(), 222);
}
