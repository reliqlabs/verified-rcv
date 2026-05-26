//! verified-rcv runtime: Stage 1 `decrypt_and_validate` implementation.
//!
//! This is the runtime wrapper that completes Tally_spec by providing the
//! Stage 1 body that `verified-rcv-enclave-core` leaves `unimplemented!()`.
//! The split is per intent §2.5: enclave-core is the Aeneas-extractable
//! IRV mathematics core; this runtime crate carries the ECIES decoder
//! (which uses `ecies` + `k256` system crypto crates that charon doesn't
//! walk cleanly).
//!
//! ## Stage 1 contract (intent §2.5 + `crates/enclave/README.md` wire-format)
//!
//! Inputs:
//! - `raw_ballots: Vec<(Addr, Vec<u8>)>` — candidate-declaration-order
//!   pairs of (voter, ciphertext). Caller MUST iterate `candidates` and
//!   for each addr in `raw_ballots.keys`, retain (addr, ciphertext) in
//!   candidate-declaration order. The on-chain contract's `Ballots` query
//!   returns the pairs in storage-key order; the enclave reorders by
//!   candidate-declaration order before invoking us.
//! - `candidates: &[Addr]` — the registered candidate set.
//! - `privkey: &[u8; 32]` — secp256k1 secret key derived by dstack KMS.
//!
//! Outputs (DecryptedSet from enclave-core):
//! - `valid: Vec<(Addr, Ballot)>` — successfully decrypted + permutation-
//!   validated; in candidate-declaration order over the subset.
//! - `dropped: Vec<Addr>` — voters whose ballot failed (any of: ECIES
//!   decrypt error, Borsh decode error, ranking not a permutation of
//!   candidates).
//! - `non_voters: Vec<Addr>` — `candidates \ raw_ballots.keys()`, in
//!   candidate-declaration order.
//!
//! ## Failure modes (each yields entry in `dropped`, NOT a Stage 1 panic)
//!
//! - ECIES decryption error (wrong privkey, malformed ciphertext, AEAD
//!   tag mismatch).
//! - Borsh deserialization error (truncated, type mismatch).
//! - Ranking is not a length-`candidates.len()` permutation of `candidates`
//!   (missing candidate, duplicate, extra address).
//!
//! Per intent §2.5 these are deterministic, decidable failures; Stage 1
//! produces a clean partition. The validation discipline matches the
//! `Validating ballots as permutations of candidates` requirement in §2.5
//! Stage 1.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use borsh::from_slice as borsh_from_slice;
use ecies::decrypt as ecies_decrypt;

use verified_rcv_enclave_core::{Addr, Ballot, DecryptedSet, RawBallots};

/// Stage 1: decrypt each ciphertext, validate as a permutation of
/// `candidates`, partition into (valid, dropped, non_voters).
///
/// This is the runtime-side body for the `decrypt_and_validate` declared
/// (as `unimplemented!()`) in `verified-rcv-enclave-core`. The enclave
/// boot wires this in before invoking `verified_rcv_enclave_core::tally_spec`.
///
/// Iteration is **candidate-declaration-ordered** (intent §2.5 Stage 1
/// iteration discipline). The caller passes `raw_ballots` already
/// projected from the contract's Ballots query into that order; we walk
/// it linearly so ordering is preserved through to the output.
pub fn decrypt_and_validate(
    raw_ballots: &RawBallots,
    candidates: &[Addr],
    privkey: &[u8],
) -> DecryptedSet {
    let mut valid: Vec<(Addr, Ballot)> = Vec::new();
    let mut dropped: Vec<Addr> = Vec::new();

    // Track which candidates voted (for non_voters computation).
    let mut voted: HashSet<&str> = HashSet::new();

    for entry in raw_ballots {
        voted.insert(entry.voter.as_str());

        match decode_one(&entry.ciphertext, candidates, privkey) {
            Ok(ballot) => valid.push((entry.voter.clone(), ballot)),
            Err(_reason) => dropped.push(entry.voter.clone()),
        }
    }

    // `non_voters = candidates \ raw_ballots.keys()`, emitted in
    // candidate-declaration order (per intent §2.5 Stage 1).
    let mut non_voters: Vec<Addr> = Vec::new();
    for c in candidates {
        if !voted.contains(c.as_str()) {
            non_voters.push(c.clone());
        }
    }

    DecryptedSet {
        valid,
        dropped,
        non_voters,
    }
}

/// Decode + validate a single ciphertext. Returns the Ballot on success,
/// or an error string describing which validation gate failed (for
/// debugging; the dispatch in `decrypt_and_validate` only cares about
/// ok-vs-err).
fn decode_one(
    ciphertext: &[u8],
    candidates: &[Addr],
    privkey: &[u8],
) -> Result<Ballot, &'static str> {
    // ECIES decrypt. `ecies` 0.2 uses secp256k1 + ChaCha20-Poly1305 by
    // default in the `pure+std` feature set. The wire format must match
    // what the UI's JS encoder produces (see crates/enclave/README.md).
    let plaintext = ecies_decrypt(privkey, ciphertext)
        .map_err(|_| "ecies decrypt failed")?;

    // Borsh decode as Vec<String> (the wire format for the ranking).
    let ranking: Vec<String> = borsh_from_slice(&plaintext)
        .map_err(|_| "borsh decode failed")?;

    // Validate as a permutation of candidates:
    //   - length matches |candidates|
    //   - every element of ranking is a candidate
    //   - no duplicates within ranking
    if ranking.len() != candidates.len() {
        return Err("ranking length != |candidates|");
    }

    // Build candidate set for membership check.
    let cand_set: HashSet<&str> = candidates.iter().map(|a| a.as_str()).collect();
    let mut seen: HashSet<&str> = HashSet::new();
    for r in &ranking {
        if !cand_set.contains(r.as_str()) {
            return Err("ranking has non-candidate address");
        }
        if !seen.insert(r.as_str()) {
            return Err("ranking has duplicate");
        }
    }

    Ok(Ballot { ranking })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use borsh::to_vec as borsh_to_vec;
    use ecies::utils::generate_keypair;
    use verified_rcv_enclave_core::RawEntry;

    fn addr(s: &str) -> Addr {
        s.to_string()
    }

    /// Encrypt a ranking under `pubkey` using the same wire format the UI
    /// produces. Helper for tests.
    fn encrypt_for_test(ranking: &[&str], pubkey_uncompressed: &[u8]) -> Vec<u8> {
        let v: Vec<String> = ranking.iter().map(|s| s.to_string()).collect();
        let plaintext = borsh_to_vec(&v).unwrap();
        ecies::encrypt(pubkey_uncompressed, &plaintext).unwrap()
    }

    #[test]
    fn happy_path_three_candidates() {
        let (sk, pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B"), addr("C")];

        let raw = vec![
            RawEntry {
                voter: addr("A"),
                ciphertext: encrypt_for_test(&["A", "B", "C"], &pk.serialize()),
            },
            RawEntry {
                voter: addr("B"),
                ciphertext: encrypt_for_test(&["B", "A", "C"], &pk.serialize()),
            },
        ];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 2);
        assert_eq!(result.dropped.len(), 0);
        assert_eq!(result.non_voters, vec![addr("C")]);

        // Verify rankings round-tripped correctly.
        assert_eq!(result.valid[0].0, addr("A"));
        assert_eq!(
            result.valid[0].1.ranking,
            vec![addr("A"), addr("B"), addr("C")]
        );
        assert_eq!(result.valid[1].0, addr("B"));
        assert_eq!(
            result.valid[1].1.ranking,
            vec![addr("B"), addr("A"), addr("C")]
        );
    }

    #[test]
    fn malformed_ciphertext_is_dropped() {
        let (sk, _pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B")];

        let raw = vec![RawEntry {
            voter: addr("A"),
            ciphertext: vec![0u8; 16], // not valid ECIES output
        }];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.dropped, vec![addr("A")]);
        assert_eq!(result.non_voters, vec![addr("B")]);
    }

    #[test]
    fn wrong_length_ranking_is_dropped() {
        let (sk, pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B"), addr("C")];

        let raw = vec![RawEntry {
            voter: addr("A"),
            ciphertext: encrypt_for_test(&["A", "B"], &pk.serialize()), // length 2, need 3
        }];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.dropped, vec![addr("A")]);
    }

    #[test]
    fn non_candidate_in_ranking_is_dropped() {
        let (sk, pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B"), addr("C")];

        let raw = vec![RawEntry {
            voter: addr("A"),
            ciphertext: encrypt_for_test(&["A", "B", "EVE"], &pk.serialize()), // EVE not in cands
        }];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.dropped, vec![addr("A")]);
    }

    #[test]
    fn duplicate_in_ranking_is_dropped() {
        let (sk, pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B"), addr("C")];

        let raw = vec![RawEntry {
            voter: addr("A"),
            ciphertext: encrypt_for_test(&["A", "B", "A"], &pk.serialize()), // A twice
        }];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.dropped, vec![addr("A")]);
    }

    #[test]
    fn all_abstain_no_ballots() {
        let (sk, _pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B"), addr("C")];

        let raw: RawBallots = vec![];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.dropped.len(), 0);
        assert_eq!(result.non_voters, cands);
    }

    #[test]
    fn non_voters_preserves_candidate_declaration_order() {
        let (sk, pk) = generate_keypair();
        let cands = vec![addr("A"), addr("B"), addr("C"), addr("D")];

        // Only B votes; non_voters = [A, C, D] in declaration order.
        let raw = vec![RawEntry {
            voter: addr("B"),
            ciphertext: encrypt_for_test(&["B", "A", "C", "D"], &pk.serialize()),
        }];

        let result = decrypt_and_validate(&raw, &cands, &sk.serialize());

        assert_eq!(result.valid.len(), 1);
        assert_eq!(result.non_voters, vec![addr("A"), addr("C"), addr("D")]);
    }
}
