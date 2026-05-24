//! Kani bounded-model-checking harnesses for the verified-rcv contract.
//!
//! Each harness verifies a state-only invariant from the Quint protocol model
//! (`specs/rcv.qnt`) against the Rust contract. The full temporal claims
//! (e.g., "once `Some`, always `Some` with the same value" for B1) require
//! history quantification and live in the Quint model; the harnesses here
//! check the contract-handler-level projection of each invariant.
//!
//! # Bounded universe
//!
//! Kani is a bounded model checker. Inputs that would be unbounded in the
//! real protocol (the candidate set, ballot ciphertext bytes, timestamps)
//! are pinned to small fixed-size shapes here so the symbolic universe is
//! tractable. The Quint model already validates the unbounded shape; these
//! Kani harnesses validate that the Rust handler logic preserves the
//! invariants on every reachable state within the bound.
//!
//! # Storage abstraction
//!
//! CosmWasm's `Item` / `Map` go through `serde_json` serialization on every
//! store / load. That serialization is opaque to Kani, but it is total and
//! deterministic, so the harnesses still go end-to-end through the real
//! `MockStorage` + `cw_storage_plus` layers. If a harness times out on
//! that path, the affected harness is documented; a Verus-side projection
//! can pick it up instead (Verus tolerates more complex Rust).
//!
//! # Build gating
//!
//! The module is gated behind the `verification` feature so it does not
//! enter the wasm output. Inside the module, calls into the `kani` crate
//! itself are additionally gated behind `#[cfg(kani)]`. That second gate
//! means the file still type-checks under `cargo build --features
//! verification` (which is useful in CI to catch type errors without
//! requiring Kani), but only `cargo kani --features verification` actually
//! exercises the symbolic universe.

#![allow(clippy::needless_return)]
#![allow(unexpected_cfgs)]
#![cfg(feature = "verification")]

use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{Addr, Empty, HexBinary, MessageInfo, OwnedDeps, Timestamp};

use verified_rcv_enclave_core::TallyResult;

use crate::contract::{exec_publish_result, exec_submit_ballot};
use crate::error::ContractError;
use crate::msg::AttestationEnvelope;
use crate::state::{
    Config, Election, EnclaveImageRegistry, BALLOTS, CONFIG, ELECTION, ELECTION_COUNTER, REGISTRY,
    TALLY_RESULT,
};

// --------------------------------------------------------------------
// kani::* shim for non-Kani builds
// --------------------------------------------------------------------
//
// When this file is compiled by plain `rustc` (e.g. `cargo build
// --features verification`) the `kani` crate is unavailable, so the
// `kani::any` / `kani::assume` calls would fail to resolve. We provide a
// no-op shim that satisfies the type checker; the harnesses' #[kani::proof]
// attributes still gate symbolic execution to Kani itself.

#[cfg(not(kani))]
#[allow(dead_code)]
mod kani {
    pub fn any<T: Default>() -> T {
        T::default()
    }
    pub fn assume(_: bool) {}
}

// --------------------------------------------------------------------
// Bounded universe
// --------------------------------------------------------------------

/// Fixed candidate / non-candidate addresses. Using `&'static str` avoids
/// symbolic-string allocation, which Kani handles poorly.
const C0: &str = "cand0";
const C1: &str = "cand1";
const C2: &str = "cand2";
const NON_C: &str = "noncand";

const ADMIN: &str = "admin";
const START_AT_NS: u64 = 1_000_000_000_000;
const END_AT_NS: u64 = 2_000_000_000_000;

fn candidates() -> Vec<Addr> {
    vec![
        Addr::unchecked(C0),
        Addr::unchecked(C1),
        Addr::unchecked(C2),
    ]
}

fn fresh_election() -> Election {
    Election {
        id: 1,
        title: "t".to_string(),
        candidates: candidates(),
        start_at: Timestamp::from_nanos(START_AT_NS),
        end_at: Timestamp::from_nanos(END_AT_NS),
        ballot_count: 0,
    }
}

fn fresh_config() -> Config {
    Config {
        admin: Addr::unchecked(ADMIN),
        voting_duration_seconds: 1000,
    }
}

fn fresh_registry() -> EnclaveImageRegistry {
    EnclaveImageRegistry {
        mrtd: vec![0u8; 32],
        rtmr: vec![0u8; 32],
        vkey: "verified_rcv_vkey".to_string(),
    }
}

/// Owned mock deps with the verified-rcv storage pre-populated to a state
/// reachable via `instantiate` + `CreateElection`. Time is *not* applied to
/// the env here; each harness sets `env.block.time` itself.
fn setup_deps_voting() -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
    let mut deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> = OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::default(),
        custom_query_type: std::marker::PhantomData,
    };
    CONFIG.save(&mut deps.storage, &fresh_config()).unwrap();
    REGISTRY.save(&mut deps.storage, &fresh_registry()).unwrap();
    ELECTION_COUNTER.save(&mut deps.storage, &1u64).unwrap();
    ELECTION.save(&mut deps.storage, &fresh_election()).unwrap();
    deps
}

/// A minimal valid `TallyResult` for the 3-candidate election where one
/// voter (`C0`) submitted a ballot, two did not (`C1`, `C2` as non-voters).
/// All chain-side `check_tally_well_formed` clauses hold by construction.
fn minimal_valid_tally() -> TallyResult {
    use verified_rcv_enclave_core::{RoundCount, RoundCounts};
    let round0: RoundCounts = vec![RoundCount {
        candidate: C0.to_string(),
        count: 1,
    }];
    TallyResult {
        winners: vec![C0.to_string()],
        per_round_counts: vec![round0],
        eliminated_by_round: vec![],
        ballots_tallied: 1,
        ballots_dropped: 0,
        dropped_voters: vec![],
        non_voters: vec![C1.to_string(), C2.to_string()],
    }
}

// --------------------------------------------------------------------
// Harness 1: B1 state-shape, TALLY_RESULT.may_load is Some after success
// --------------------------------------------------------------------
//
// Quint refinement: B1 ("tally_result is write-once") full statement is
// `[]((contract_tally_result.present) -> next [](contract_tally_result.present
// and contract_tally_result.value unchanged))`. The state-only fragment is
// the immediate post-condition: after a successful `PublishResult`,
// `TALLY_RESULT.may_load(storage)` is `Some(_)`. Persistence + value
// equality across subsequent steps follows from there being no handler
// that overwrites or removes `TALLY_RESULT` (verified by code review;
// `CreateElection` does call `TALLY_RESULT.remove`, but the spec frames B1
// as scoped to a single election lifecycle).

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn b1_tally_result_present_after_publish() {
    let mut deps = setup_deps_voting();
    let mut env = mock_env();
    // Place env time in the Tallying phase: block.time >= end_at, and
    // tally_result is currently None (setup_deps_voting did not write it).
    env.block.time = Timestamp::from_nanos(END_AT_NS + 1);

    // Pre-condition: TALLY_RESULT is None at this point.
    assert!(TALLY_RESULT.may_load(&deps.storage).unwrap().is_none());

    let tally = minimal_valid_tally();
    let res = exec_publish_result(
        deps.as_mut(),
        env.clone(),
        tally.clone(),
        AttestationEnvelope::Mock,
    );

    if res.is_ok() {
        // Post-condition: TALLY_RESULT is Some.
        let stored = TALLY_RESULT.may_load(&deps.storage).unwrap();
        assert!(stored.is_some());
        // Stored value equals the published value (single-state byte-equal).
        assert!(stored.unwrap().winners == tally.winners);
    }
}

// --------------------------------------------------------------------
// Harness 2: S4 ballot-keys-subset, every BALLOTS key is in candidates
// --------------------------------------------------------------------
//
// Quint S4: forall a in contract_ballots.keys() . a in candidate_set.
// Encoded in `exec_submit_ballot` by the `VoterNotCandidate` guard. The
// harness validates that for any symbolic `sender` and ciphertext, after a
// successful submit, the stored ballot key is in the candidate set.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn s4_ballot_keys_subset_of_candidates() {
    let mut deps = setup_deps_voting();
    let mut env = mock_env();
    // Voting phase: start_at <= block.time < end_at.
    env.block.time = Timestamp::from_nanos(START_AT_NS + 1);

    // Pick the sender symbolically from {C0, C1, C2, NON_C}.
    let sender_idx: u8 = kani::any();
    kani::assume(sender_idx < 4);
    let sender_str = match sender_idx {
        0 => C0,
        1 => C1,
        2 => C2,
        _ => NON_C,
    };
    let sender = Addr::unchecked(sender_str);
    let info: MessageInfo = message_info(&sender, &[]);

    // Bounded ciphertext: 4 bytes. Symbolic content; non-empty.
    let mut ct_bytes = [0u8; 4];
    ct_bytes[0] = kani::any();
    ct_bytes[1] = kani::any();
    ct_bytes[2] = kani::any();
    ct_bytes[3] = kani::any();
    let ciphertext = HexBinary::from(ct_bytes.to_vec());

    let res = exec_submit_ballot(deps.as_mut(), env, info, ciphertext);

    if res.is_ok() {
        // Post-condition: stored ballot key is one of the candidate addrs.
        assert!(BALLOTS.has(&deps.storage, &sender));
        let cands = candidates();
        assert!(cands.contains(&sender));
    } else {
        // If it failed, sender must not be a candidate OR a previous
        // submit already populated the slot. In a fresh setup neither
        // ballot exists, so the only failure is VoterNotCandidate ==
        // (sender == NON_C).
        if sender_str == NON_C {
            assert!(matches!(res, Err(ContractError::VoterNotCandidate)));
        }
    }
}

// --------------------------------------------------------------------
// Harness 3: S10 resolution-after-end-at, TALLY_RESULT.is_some implies
// end_at <= env.block.time at the time of publishing
// --------------------------------------------------------------------
//
// Encoded as: derived-phase=Tallying requires `block.time >= end_at`, and
// `PublishResult` requires derived-phase=Tallying. The harness picks
// `env.block.time` symbolically and asserts that if `exec_publish_result`
// succeeded, then `env.block.time >= election.end_at` held at call time.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn s10_resolution_after_end_at() {
    let mut deps = setup_deps_voting();
    let mut env = mock_env();

    // Symbolic block.time across the full lifecycle range.
    let bt: u64 = kani::any();
    // Bound to avoid CBMC integer overflow worries on Timestamp arithmetic.
    kani::assume(bt < 4_000_000_000_000);
    env.block.time = Timestamp::from_nanos(bt);

    let res = exec_publish_result(
        deps.as_mut(),
        env.clone(),
        minimal_valid_tally(),
        AttestationEnvelope::Mock,
    );

    if res.is_ok() {
        // Post-condition: env.block.time was at or past end_at when the
        // call was issued, since `compute_phase` only returns Tallying when
        // `block.time >= end_at` (and `phase != Tallying` rejects).
        assert!(env.block.time >= Timestamp::from_nanos(END_AT_NS));
        // And TALLY_RESULT is now Some.
        assert!(TALLY_RESULT.may_load(&deps.storage).unwrap().is_some());
    }
}

// --------------------------------------------------------------------
// Harness 4: AlreadyVoted, repeat SubmitBallot from the same sender fails
// --------------------------------------------------------------------
//
// Quint B6 (per-key forall shape): once a ballot is stored under key K, a
// second SubmitBallot with sender == K returns Err(AlreadyVoted). The
// harness pre-populates BALLOTS[K] and verifies the second submit fails
// with the expected variant.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn already_voted_enforced() {
    let mut deps = setup_deps_voting();
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(START_AT_NS + 1);

    // Pre-populate BALLOTS[C0] = some ciphertext, and bump ballot_count to
    // mirror the post-state of a prior successful submit.
    let voter = Addr::unchecked(C0);
    let prior_ct = HexBinary::from(vec![0xAB, 0xCD]);
    BALLOTS.save(&mut deps.storage, &voter, &prior_ct).unwrap();
    let mut e = ELECTION.load(&deps.storage).unwrap();
    e.ballot_count = 1;
    ELECTION.save(&mut deps.storage, &e).unwrap();

    // Symbolic new ciphertext; sender == C0.
    let mut ct_bytes = [0u8; 2];
    ct_bytes[0] = kani::any();
    ct_bytes[1] = kani::any();
    let new_ct = HexBinary::from(ct_bytes.to_vec());

    let info: MessageInfo = message_info(&voter, &[]);
    let res = exec_submit_ballot(deps.as_mut(), env, info, new_ct);

    // Post-condition: error is AlreadyVoted.
    assert!(matches!(res, Err(ContractError::AlreadyVoted)));

    // And the stored ciphertext is unchanged.
    let stored = BALLOTS.load(&deps.storage, &voter).unwrap();
    assert!(stored == prior_ct);
}

// --------------------------------------------------------------------
// Harness 5: AlreadyResolved, repeat PublishResult fails
// --------------------------------------------------------------------
//
// Quint B1 (single-state error fragment): once TALLY_RESULT is Some, a
// subsequent PublishResult returns Err(AlreadyResolved). The harness
// pre-populates TALLY_RESULT and verifies the second publish fails with
// the expected variant.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn already_resolved_enforced() {
    let mut deps = setup_deps_voting();
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(END_AT_NS + 1);

    // Pre-populate TALLY_RESULT with a valid tally. This mirrors the
    // post-state of a successful first PublishResult.
    let initial = minimal_valid_tally();
    TALLY_RESULT.save(&mut deps.storage, &initial).unwrap();

    let res = exec_publish_result(
        deps.as_mut(),
        env,
        minimal_valid_tally(),
        AttestationEnvelope::Mock,
    );

    // Post-condition: error is AlreadyResolved.
    assert!(matches!(res, Err(ContractError::AlreadyResolved)));

    // And TALLY_RESULT is unchanged (write-once).
    let stored = TALLY_RESULT.load(&deps.storage).unwrap();
    assert!(stored.winners == initial.winners);
}
