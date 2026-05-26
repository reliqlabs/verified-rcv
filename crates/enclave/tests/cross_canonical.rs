//! Cross-test: the runtime's `canonical_serialization` and the contract's
//! `canonical_serialization` MUST produce byte-identical output for the
//! same `(contract_addr, election_id, tally_body)`. They live in separate
//! crates (the contract is wasm; the runtime pulls tokio/tonic/etc.), so
//! we duplicate the implementation rather than share it via a third crate.
//! This test is the canary that catches divergence.
//!
//! Per the audit-finding remediation (2026-05-26 M2): canonical_serialization
//! now includes `election_id` between `contract_addr` and `tally_body`.

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
    // Minimal tally; election_id is the only difference. Verifies the
    // u64 LE election_id is in the right structural position (between
    // contract_addr and tally_body).
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

    // The election_id is 8 bytes after the contract_addr String (4-byte
    // length + 1 byte "a" = 5 bytes). So position 5..13 of the output is
    // election_id LE.
    assert_eq!(bytes_e1[0..5], bytes_e2[0..5]); // contract_addr identical
    assert_eq!(bytes_e1[5..13], 1u64.to_le_bytes());
    assert_eq!(bytes_e2[5..13], 2u64.to_le_bytes());
    assert_eq!(bytes_e1[13..], bytes_e2[13..]); // tally_body identical
}

#[test]
fn build_user_data_commit_hash_matches_contract_compute() {
    // Cross-test: the runtime's build_user_data lower 32 bytes equal the
    // contract's compute_commit_hash. This is the load-bearing fact for
    // C2 envelope binding.
    use sha2::{Digest, Sha256};

    let tally = sample_tally();
    let ud = runtime_impl::build_user_data("xion1c", 7, &tally);
    let commit_lower = &ud[32..];

    let contract_commit = contract_impl::compute_commit_hash("xion1c", 7, &tally);
    assert_eq!(commit_lower, &contract_commit[..]);

    // Also check independent SHA-256 path.
    let canonical = runtime_impl::canonical_serialization("xion1c", 7, &tally);
    let expected = Sha256::digest(&canonical);
    assert_eq!(commit_lower, &expected[..]);
}
