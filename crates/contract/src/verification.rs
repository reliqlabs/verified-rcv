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
#![allow(dead_code)]
#![cfg(feature = "verification")]

use cosmwasm_std::testing::{
    message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{Addr, Empty, HexBinary, MessageInfo, OwnedDeps, Timestamp};

use verified_rcv_enclave_core::{RoundCount, RoundCounts, TallyResult};

use crate::contract::{
    build_publish_report_data, build_registration_report_data, build_synthetic_public_inputs,
    check_tally_well_formed, compute_ballots_hash, compute_commit_hash, derive_phase,
    exec_publish_result, exec_submit_ballot, verify_publish_quote, verify_registration_quote,
};
use crate::error::ContractError;
use crate::state::{
    Config, Election, EnclaveImageRegistry, Phase, BALLOTS, CONFIG, ELECTION, ELECTION_COUNTER,
    REGISTRY, TALLY_RESULT,
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
        enclave_pubkey: HexBinary::from({
            // Valid compressed secp256k1 shape: 0x02 prefix + 32-byte X.
            let mut b = vec![0u8; 33];
            b[0] = 0x02;
            b
        }),
    }
}

fn fresh_config() -> Config {
    Config {
        admin: Addr::unchecked(ADMIN),
        voting_duration_seconds: 1000,
        registry_update_delay_seconds: 0,
    }
}

fn fresh_registry() -> EnclaveImageRegistry {
    EnclaveImageRegistry {
        vkey_name: "verified_rcv_vkey".to_string(),
        mrtd: vec![0u8; 48], // TDX SHA-384 length (audit M3 / N1 schema)
        rtmr1: vec![0u8; 48],
        rtmr2: vec![0u8; 48],
        rtmr0: None,
        rtmr3: None,
        accepted_tcb_statuses: vec![0, 1, 2, 3],
    }
}

/// Synthesize a chain-acceptable publish-quote `(proof, public_inputs)`
/// pair for `tally` under the fresh registry. The `mock-attestation`
/// feature (transitively enabled by `verification`) makes
/// `verify_gnark_proof_via_xion` a no-op, so the proof bytes themselves
/// are irrelevant — only the layout + commit_hash + DST equality is checked.
fn fresh_publish_artifacts(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    tally: &TallyResult,
) -> (HexBinary, HexBinary) {
    let reg = fresh_registry();
    // Empty ballots view for the harness — harnesses pre-populate BALLOTS
    // selectively and the publish flow walks BALLOTS at PublishResult
    // time. For the harness's chain-side commit, hash the empty view.
    let bh = compute_ballots_hash(&[], &[]);
    let commit = compute_commit_hash(contract_addr, chain_id, election_id, &bh, tally);
    let rd = build_publish_report_data(&commit);
    let mrtd: [u8; 48] = reg.mrtd.clone().try_into().unwrap();
    let r1: [u8; 48] = reg.rtmr1.clone().try_into().unwrap();
    let r2: [u8; 48] = reg.rtmr2.clone().try_into().unwrap();
    let pi = build_synthetic_public_inputs(
        &mrtd, &[0; 48], &r1, &r2, &[0; 48], &rd, 0, 1_700_000_000,
    );
    (HexBinary::from(vec![0xABu8; 192]), HexBinary::from(pi))
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
    let contract_addr = env.contract.address.to_string();
    let election_id = ELECTION.load(&deps.storage).unwrap().id;
    let (proof, pi) = fresh_publish_artifacts(&contract_addr, &env.block.chain_id, election_id, &tally);
    let res = exec_publish_result(
        deps.as_mut(),
        env.clone(),
        tally.clone(),
        proof,
        pi,
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

    let tally = minimal_valid_tally();
    let contract_addr = env.contract.address.to_string();
    let election_id = ELECTION.load(&deps.storage).unwrap().id;
    let (proof, pi) = fresh_publish_artifacts(&contract_addr, &env.block.chain_id, election_id, &tally);
    let res = exec_publish_result(
        deps.as_mut(),
        env.clone(),
        tally,
        proof,
        pi,
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

    let tally = minimal_valid_tally();
    let contract_addr = env.contract.address.to_string();
    let election_id = ELECTION.load(&deps.storage).unwrap().id;
    let (proof, pi) = fresh_publish_artifacts(&contract_addr, &env.block.chain_id, election_id, &tally);
    let res = exec_publish_result(
        deps.as_mut(),
        env,
        tally,
        proof,
        pi,
    );

    // Post-condition: error is AlreadyResolved.
    assert!(matches!(res, Err(ContractError::AlreadyResolved)));

    // And TALLY_RESULT is unchanged (write-once).
    let stored = TALLY_RESULT.load(&deps.storage).unwrap();
    assert!(stored.winners == initial.winners);
}

// ====================================================================
// Pure-logic harnesses (no storage)
// ====================================================================
//
// The harnesses above go through CosmWasm storage's `serde_json` layer,
// which has been observed to stall Kani's symbolic execution at the
// 10-minute wall-time mark on this hardware. The harnesses below target
// the **pure helper functions** (`derive_phase`, `check_tally_well_formed`)
// directly — these are total deterministic functions of their inputs with
// no storage interaction, so Kani's symbolic universe stays tractable.
//
// What's verified:
//   - `derive_phase` is total over (Timestamp, Election, bool) and returns
//     exactly one of {Created, Voting, Tallying, Resolved}.
//   - `derive_phase` agrees with the Quint refinement: has_tally=true ⇒
//     Resolved; otherwise time-based partition.
//   - `check_tally_well_formed` returning Ok implies each chain-syntactic
//     S6-S9 clause holds (winner-membership, ballots_dropped consistency,
//     partition equation, dropped/non_voters disjoint, round-bound,
//     elim-bound, per-round-sum, S9 monotonicity at small bounds).
//
// These harnesses are the verification-critical subset of intent §3.1
// well-formedness; the temporal complement (B6 ∀-per-key, B1 write-once)
// is handled by the storage-going harnesses above (which still type-check
// but may not run under Kani — they ARE valid Quint refinements at the
// proptest layer if Kani stalls).

// --------------------------------------------------------------------
// Harness 6: derive_phase totality + phase-partition
// --------------------------------------------------------------------
//
// For any (now, election, has_tally), `derive_phase` returns a valid
// `Phase`. The phase is determined by has_tally and the relative
// position of `now` against start_at/end_at. Verifies the function is
// total (no panic) and produces a value matching the Quint partition.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(32))]
pub fn derive_phase_total_and_partitioned() {
    // Symbolic: now, start_at, end_at, has_tally.
    // Bounded: timestamps < 4e12 (well within u64).
    let now_ns: u64 = kani::any();
    let start_ns: u64 = kani::any();
    let end_ns: u64 = kani::any();
    let has_tally: bool = kani::any();
    kani::assume(now_ns < 4_000_000_000_000);
    kani::assume(start_ns < 4_000_000_000_000);
    kani::assume(end_ns < 4_000_000_000_000);

    let election = Election {
        id: 1,
        title: "t".to_string(),
        candidates: candidates(),
        start_at: Timestamp::from_nanos(start_ns),
        end_at: Timestamp::from_nanos(end_ns),
        ballot_count: 0,
        enclave_pubkey: HexBinary::from({
            // Valid compressed secp256k1 shape: 0x02 prefix + 32-byte X.
            let mut b = vec![0u8; 33];
            b[0] = 0x02;
            b
        }),
    };
    let now = Timestamp::from_nanos(now_ns);

    let phase = derive_phase(now, &election, has_tally);

    // Quint-aligned partition. Exactly one branch matches.
    if has_tally {
        assert!(phase == Phase::Resolved);
    } else if now < election.start_at {
        assert!(phase == Phase::Created);
    } else if now < election.end_at {
        assert!(phase == Phase::Voting);
    } else {
        assert!(phase == Phase::Tallying);
    }
}

// --------------------------------------------------------------------
// Narrow per-invariant harnesses
// --------------------------------------------------------------------
//
// Each harness builds a CONCRETE tally that satisfies all clauses, then
// perturbs exactly ONE thing symbolically. This keeps the symbolic state
// space small enough for cbmc to enumerate in seconds rather than hours.
//
// Earlier-session attempt with a unified `symbolic_tally_bounded` builder
// ran for 1+ hours without output: the cross-product of symbolic Vec
// lengths × symbolic Vec<String> contents × symbolic counts blew up
// combinatorially even with small per-dimension bounds.
//
// `picks_cand`: a tiny helper that maps 0/1/2 to the C0/C1/C2 static
// strings. The strings themselves are pre-allocated (returning `.to_string()`
// from a `&'static str` allocates a tiny known-shape heap region).
fn pick_cand(idx: u8) -> String {
    match idx % 3 {
        0 => C0.to_string(),
        1 => C1.to_string(),
        _ => C2.to_string(),
    }
}

// --------------------------------------------------------------------
// Harness 7: S6 winner-not-in-candidates ⇒ check returns Err
// --------------------------------------------------------------------
//
// Build a concrete tally where one winner address is symbolic. If the
// symbolic winner is NOT in {C0, C1, C2}, the check must reject.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(32))]
pub fn s6_non_candidate_winner_rejected() {
    let election = fresh_election();
    // Symbolic: is the winner C0 (in-set) or NON_C (out-of-set)?
    let in_set: bool = kani::any();
    let winner_str = if in_set { C0 } else { NON_C };

    let round0: RoundCounts = vec![
        RoundCount { candidate: winner_str.to_string(), count: 1 },
    ];
    let tally = TallyResult {
        winners: vec![winner_str.to_string()],
        per_round_counts: vec![round0],
        eliminated_by_round: vec![],
        ballots_tallied: 1,
        ballots_dropped: 0,
        dropped_voters: vec![],
        non_voters: vec![C1.to_string(), C2.to_string()],
    };

    let res = check_tally_well_formed(&election, &tally);

    // If winner is out-of-set, check must fail. If in-set, must succeed
    // (the concrete tally is well-formed by construction).
    if in_set {
        assert!(res.is_ok());
    } else {
        assert!(res.is_err());
    }
}

// --------------------------------------------------------------------
// Harness 8: S7 partition equation must balance for Ok
// --------------------------------------------------------------------
//
// Build a concrete tally with symbolic `ballots_tallied`. The partition
// equation requires `tallied + dropped + |non_voters| = |candidates|`.
// For our shape (3 candidates, 0 dropped, 2 non_voters), tallied MUST be 1.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(32))]
pub fn s7_partition_equation_required() {
    let election = fresh_election();
    let symbolic_tallied: u32 = kani::any();
    kani::assume(symbolic_tallied <= 3);

    let round0: RoundCounts = vec![
        RoundCount { candidate: C0.to_string(), count: symbolic_tallied },
    ];
    let tally = TallyResult {
        winners: vec![C0.to_string()],
        per_round_counts: vec![round0],
        eliminated_by_round: vec![],
        ballots_tallied: symbolic_tallied,
        ballots_dropped: 0,
        dropped_voters: vec![],
        non_voters: vec![C1.to_string(), C2.to_string()],
    };

    let res = check_tally_well_formed(&election, &tally);

    // 3 candidates - 0 dropped - 2 non_voters = 1 tallied
    if symbolic_tallied == 1 {
        assert!(res.is_ok());
    } else {
        assert!(res.is_err());
    }
}

// --------------------------------------------------------------------
// Harness 9: S8 per-round sum must equal ballots_tallied
// --------------------------------------------------------------------
//
// Build a concrete shape with two candidates surviving round 0 and
// symbolic counts for one of them. The round-sum check requires
// counts to total `ballots_tallied`.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(32))]
pub fn s8_round_sum_required() {
    let election = fresh_election();
    let c0_count: u32 = kani::any();
    kani::assume(c0_count <= 3);

    // ballots_tallied is fixed at 1; non_voters fills the rest of cs.
    // round0 = [{C0: c0_count}], sum must equal 1.
    let round0: RoundCounts = vec![
        RoundCount { candidate: C0.to_string(), count: c0_count },
    ];
    let tally = TallyResult {
        winners: vec![C0.to_string()],
        per_round_counts: vec![round0],
        eliminated_by_round: vec![],
        ballots_tallied: 1,
        ballots_dropped: 0,
        dropped_voters: vec![],
        non_voters: vec![C1.to_string(), C2.to_string()],
    };

    let res = check_tally_well_formed(&election, &tally);

    if c0_count == 1 {
        assert!(res.is_ok());
    } else {
        assert!(res.is_err());
    }
}

// --------------------------------------------------------------------
// Harness 10: S9 elimination monotonicity required for Ok
// --------------------------------------------------------------------
//
// Build a concrete 2-round tally. C2 is eliminated at round 0. Round 1
// has a symbolic-candidate entry. If the round-1 entry has candidate C2,
// it's a violation; otherwise (C0 or C1) the check must accept.
//
// Election shape: 3 candidates; 3 ballots tallied across two rounds.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(32))]
pub fn s9_reappearance_rejected() {
    let election = fresh_election();
    let symbolic_round1_cand: u8 = kani::any();
    kani::assume(symbolic_round1_cand < 3); // 0=C0, 1=C1, 2=C2 (eliminated)
    let round1_cand_str = match symbolic_round1_cand {
        0 => C0,
        1 => C1,
        _ => C2, // the eliminated candidate — should be rejected
    };

    // 3 ballots tallied: round 0 distributes 2 to C0, 1 to C1, 0 to C2.
    // C2 eliminated; round 1 shows C0=2, C1=1 (or violation if reappears).
    let round0: RoundCounts = vec![
        RoundCount { candidate: C0.to_string(), count: 2 },
        RoundCount { candidate: C1.to_string(), count: 1 },
        RoundCount { candidate: C2.to_string(), count: 0 },
    ];
    // round 1: two entries, one of them symbolic-named.
    let round1: RoundCounts = vec![
        RoundCount { candidate: C0.to_string(), count: 2 },
        RoundCount { candidate: round1_cand_str.to_string(), count: 1 },
    ];
    let tally = TallyResult {
        winners: vec![C0.to_string()],
        per_round_counts: vec![round0, round1],
        eliminated_by_round: vec![vec![C2.to_string()]],
        ballots_tallied: 3,
        ballots_dropped: 0,
        dropped_voters: vec![],
        non_voters: vec![],
    };

    let res = check_tally_well_formed(&election, &tally);

    // round1[1].candidate == C2 means an eliminated candidate reappeared.
    if round1_cand_str == C2 {
        assert!(res.is_err());
    } else {
        // C0 or C1 in slot 1 — no violation.
        assert!(res.is_ok());
    }
}

// ====================================================================
// B8 attestation-path harnesses (v0.4 backfill)
// ====================================================================
//
// v0.3.9 added code-level B8 checks (commit-hash binding, measurement
// equality, registration pubkey binding) with unit-test coverage; the
// Kani harness layer never picked them up. These three harnesses close
// that gap by symbolically perturbing exactly one byte of a known-good
// `(proof, public_inputs)` pair and asserting `verify_*_quote` rejects.
//
// The byte offsets below MUST stay in lockstep with `contract.rs`
// (FR_BYTES, ELEM_MRTD_START, ELEM_REPORTDATA_START). Kept private here
// rather than promoted to `pub` so the production surface stays minimal;
// any drift would be caught by the `quote_correct_binding_ok` unit tests.
const FR_BYTES_LOCAL: usize = 32;
const ELEM_MRTD_START_LOCAL: usize = 0;
const ELEM_REPORTDATA_START_LOCAL: usize = 240;

/// Synthesize a chain-acceptable publish-quote PI for the given tally,
/// without touching any storage. Mirrors the structure of
/// `fresh_publish_artifacts` but returns the raw bytes so a harness can
/// perturb individual offsets before calling `verify_publish_quote`.
fn known_good_publish_pi_bytes(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Vec<u8> {
    let reg = fresh_registry();
    let bh = compute_ballots_hash(&[], &[]);
    let commit = compute_commit_hash(contract_addr, chain_id, election_id, &bh, tally);
    let rd = build_publish_report_data(&commit);
    let mrtd: [u8; 48] = reg.mrtd.clone().try_into().unwrap();
    let r1: [u8; 48] = reg.rtmr1.clone().try_into().unwrap();
    let r2: [u8; 48] = reg.rtmr2.clone().try_into().unwrap();
    build_synthetic_public_inputs(&mrtd, &[0; 48], &r1, &r2, &[0; 48], &rd, 0, 1_700_000_000)
}

/// Synthesize a chain-acceptable registration-quote PI for the given
/// `(pubkey, contract_addr, election_id)` triple. Mirror of the helper
/// `synthetic_pi_for_pubkey_with_id` used in `contract.rs` unit tests.
fn known_good_registration_pi_bytes(
    enclave_pubkey: &[u8],
    contract_addr: &str,
    election_id: u64,
) -> Vec<u8> {
    let reg = fresh_registry();
    let rd = build_registration_report_data(enclave_pubkey, contract_addr, election_id);
    let mrtd: [u8; 48] = reg.mrtd.clone().try_into().unwrap();
    let r1: [u8; 48] = reg.rtmr1.clone().try_into().unwrap();
    let r2: [u8; 48] = reg.rtmr2.clone().try_into().unwrap();
    build_synthetic_public_inputs(&mrtd, &[0; 48], &r1, &r2, &[0; 48], &rd, 0, 1_700_000_000)
}

// --------------------------------------------------------------------
// Harness 11: B8(c) ReportData commit-hash binding enforced
// --------------------------------------------------------------------
//
// Build a known-good publish-quote PI committing to a concrete tally,
// then flip one symbolic byte of ReportData[0..32]. The verifier MUST
// reject with `AttestationCommitMismatch`.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn b8c_reportdata_commit_hash_matches() {
    let reg = fresh_registry();
    let tally = minimal_valid_tally();
    let contract_addr = "cw1xxx";
    let chain_id = "cosmos-testnet-14002";
    let election_id: u64 = 1;
    let mut pi_bytes =
        known_good_publish_pi_bytes(contract_addr, chain_id, election_id, &tally);

    // Symbolic byte index within ReportData[0..32] (the commit_hash slot).
    let k: u8 = kani::any();
    kani::assume(k < 32);
    // Each ReportData byte sits at PI offset (240 + k) * 32 + 31.
    let off = (ELEM_REPORTDATA_START_LOCAL + k as usize) * FR_BYTES_LOCAL + 31;
    // Flip the byte (XOR with 0xFF) so it is guaranteed different.
    pi_bytes[off] ^= 0xFF;

    let bh = compute_ballots_hash(&[], &[]);
    let expected_commit =
        compute_commit_hash(contract_addr, chain_id, election_id, &bh, &tally);
    let proof = HexBinary::from(vec![0xABu8; 192]);
    let pi = HexBinary::from(pi_bytes);
    let deps = mock_dependencies();
    let res = verify_publish_quote(deps.as_ref(), &reg, &expected_commit, &proof, &pi);

    // Post-condition: any single-byte flip in the commit_hash slot causes
    // `AttestationCommitMismatch`. (Measurement + TCB pass: those slots
    // were not touched.)
    assert!(matches!(res, Err(ContractError::AttestationCommitMismatch)));
}

// --------------------------------------------------------------------
// Harness 12: B8(d) MrTd measurement equality enforced
// --------------------------------------------------------------------
//
// Build a known-good publish-quote PI matching `fresh_registry`'s
// measurements, then flip one symbolic byte of the MrTd slot. The
// verifier MUST reject with `AttestationMeasurementMismatch { field:
// "mrtd" }` because `verify_measurements_match_registry` runs before
// the commit-hash equality check.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn b8d_measurement_mismatch_rejected() {
    let reg = fresh_registry();
    let tally = minimal_valid_tally();
    let contract_addr = "cw1xxx";
    let chain_id = "cosmos-testnet-14002";
    let election_id: u64 = 1;
    let mut pi_bytes =
        known_good_publish_pi_bytes(contract_addr, chain_id, election_id, &tally);

    // Symbolic byte index within MrTd[0..48].
    let k: u8 = kani::any();
    kani::assume(k < 48);
    // Each MrTd byte sits at PI offset (0 + k) * 32 + 31.
    let off = (ELEM_MRTD_START_LOCAL + k as usize) * FR_BYTES_LOCAL + 31;
    pi_bytes[off] ^= 0xFF;

    let bh = compute_ballots_hash(&[], &[]);
    let expected_commit =
        compute_commit_hash(contract_addr, chain_id, election_id, &bh, &tally);
    let proof = HexBinary::from(vec![0xABu8; 192]);
    let pi = HexBinary::from(pi_bytes);
    let deps = mock_dependencies();
    let res = verify_publish_quote(deps.as_ref(), &reg, &expected_commit, &proof, &pi);

    // Post-condition: MrTd byte flip causes the measurement check to fail
    // before any other check.
    assert!(matches!(
        res,
        Err(ContractError::AttestationMeasurementMismatch { field: "mrtd" })
    ));
}

// --------------------------------------------------------------------
// Harness 13: B8(e) registration-quote pubkey+addr+id binding enforced
// --------------------------------------------------------------------
//
// Build a known-good registration-quote PI binding pubkey+addr+id, then
// symbolically perturb either ReportData[0..32] (the binding hash) or
// ReportData[32..64] (the DST tag). The verifier MUST reject; the
// specific error variant differs by region:
//   - flip in [0..32]: `AttestationPubkeyBindingMismatch` (the perturbed
//     hash will not coincide with `SHA-256(pubkey_only)`, so the
//     `RegistrationQuoteWrongElection` branch does not fire).
//   - flip in [32..64]: `AttestationDomainTagInvalid`.

#[cfg_attr(kani, kani::proof)]
#[cfg_attr(kani, kani::unwind(8))]
pub fn b8e_registration_pubkey_binding() {
    let reg = fresh_registry();
    // Use the same compressed-secp256k1 shape as `fresh_election`.
    let mut pk_bytes = vec![0u8; 33];
    pk_bytes[0] = 0x02;
    let pk = HexBinary::from(pk_bytes);
    let contract_addr = "cw1xxx";
    let election_id: u64 = 1;
    let mut pi_bytes =
        known_good_registration_pi_bytes(pk.as_slice(), contract_addr, election_id);

    // Symbolic byte index across the full ReportData[0..64] range.
    let k: u8 = kani::any();
    kani::assume(k < 64);
    let off = (ELEM_REPORTDATA_START_LOCAL + k as usize) * FR_BYTES_LOCAL + 31;
    pi_bytes[off] ^= 0xFF;

    let proof = HexBinary::from(vec![0xABu8; 192]);
    let pi = HexBinary::from(pi_bytes);
    let deps = mock_dependencies();
    let res = verify_registration_quote(
        deps.as_ref(),
        &reg,
        &pk,
        contract_addr,
        election_id,
        &proof,
        &pi,
    );

    // Post-condition: any single-byte flip in ReportData causes a
    // rejection. The exact variant depends on which half was hit:
    //   - k < 32: PubkeyBindingMismatch (the v0.3.12 N22 binding), OR
    //             RegistrationQuoteWrongElection in the (concrete but
    //             pubkey-dependent) corner case where the flipped value
    //             happens to coincide with SHA-256(pubkey_only).
    //   - k >= 32: AttestationDomainTagInvalid (the DST half).
    // Both branches discharge B8(e); we assert rejection in either case.
    if k < 32 {
        assert!(matches!(
            res,
            Err(ContractError::AttestationPubkeyBindingMismatch)
                | Err(ContractError::RegistrationQuoteWrongElection)
        ));
    } else {
        assert!(matches!(res, Err(ContractError::AttestationDomainTagInvalid)));
    }
}
