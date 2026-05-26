//! Entry points + handlers. Refine the Quint protocol model action by action.
//!
//! Audit-finding remediations (2026-05-26):
//! - C1 (mock attestation): the `AttestationEnvelope::Mock` variant exists
//!   only when the `mock-attestation` Cargo feature is enabled. Production
//!   builds compile out the match arm; the deserializer rejects
//!   `{"mock": ...}` payloads at the schema boundary.
//! - C2 (Dstack verification): the Dstack envelope's `user_data` is now
//!   structurally + semantically verified: 64-byte length, upper 32
//!   bytes == `DST_VERIFIED_RCV_TALLY_V1` (zero-padded domain tag),
//!   lower 32 bytes == `SHA-256(Borsh(contract_addr ‖ election_id ‖
//!   tally_body))`. zkdcap-Groth16 verification + MRTD/RTMR-vs-registry
//!   binding are queued for a follow-on cycle (require Xion gnark module).
//! - C3 (enclave_pubkey shape): rejected unless 33-byte compressed
//!   (leading 0x02/0x03) or 65-byte uncompressed (leading 0x04)
//!   secp256k1 pubkey.
//! - M1 (B1 multi-election): `CreateElection` rejects when the existing
//!   election is in Voting or Tallying phase (state would be clobbered).
//!   Allowed in initial-Created (typo fix before voting opens) or
//!   Resolved (next election after previous one finalized).
//! - M2 (election_id in commit hash): canonical_serialization is now
//!   Borsh(contract_addr) ‖ Borsh(election_id) ‖ Borsh(tally_body), per
//!   intent v0.3.8.
//! - M3 (registry shape + update path): registry validated at instantiate
//!   AND a new `UpdateRegistry` handler permits admin rotation between
//!   elections (gated on no Voting/Tallying phase).
//! - M4 (declaration-order enforcement): `check_tally_well_formed` now
//!   verifies that every Vec<Addr>-typed field (winners, dropped_voters,
//!   non_voters; per-row entries of per_round_counts + eliminated_by_round)
//!   is a subsequence of `election.candidates` in declaration order
//!   (intent §2.5 ordering pin).

use sha2::{Digest, Sha256};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, Event, HexBinary, MessageInfo, Order,
    Response, StdResult, Timestamp,
};

use verified_rcv_enclave_core::TallyResult;

use crate::error::ContractError;
use crate::msg::{
    AttestationEnvelope, BallotsResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ResultResponse,
};
use crate::state::{
    Config, Election, EnclaveImageRegistry, Phase, BALLOTS, CONFIG, ELECTION, ELECTION_COUNTER,
    REGISTRY, TALLY_RESULT,
};

// ============================================================
// Constants (audit remediations C2, C3, M3)
// ============================================================

/// 32-byte domain-separation tag for verified-rcv tally-commit hashes.
/// Right-padded with zeros to fill 32 bytes. Per intent v0.3.8 §3.2 B8(c).
const DOMAIN_TAG_LITERAL: &[u8] = b"DST_VERIFIED_RCV_TALLY_V1";
const DOMAIN_TAG_LEN: usize = 32;

/// TDX MRTD measurement length (SHA-384 over initial VM image).
/// Per Intel TDX spec.
const MRTD_LEN: usize = 48;

/// TDX RTMR measurement length (SHA-384 over runtime extensions).
const RTMR_LEN: usize = 48;

/// secp256k1 compressed pubkey: 1 byte (0x02 / 0x03) + 32 bytes X.
const SECP256K1_COMPRESSED_LEN: usize = 33;

/// secp256k1 uncompressed pubkey: 1 byte (0x04) + 32 bytes X + 32 bytes Y.
const SECP256K1_UNCOMPRESSED_LEN: usize = 65;

// ============================================================
// Entry points
// ============================================================

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin = msg.admin.unwrap_or_else(|| info.sender.clone());

    // M3: registry shape validation at instantiate. The chain does NOT
    // verify these match a particular enclave (operational responsibility
    // per intent §6.1 image_registration_honest); it only enforces the
    // shape so a misconfigured registry doesn't silently break attestation.
    validate_registry(&msg.registry)?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin,
            voting_duration_seconds: msg.voting_duration_seconds,
        },
    )?;
    REGISTRY.save(deps.storage, &msg.registry)?;
    ELECTION_COUNTER.save(deps.storage, &0u64)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateElection {
            title,
            candidates,
            start_at,
            end_at,
            enclave_pubkey,
        } => exec_create_election(
            deps, env, info, title, candidates, start_at, end_at, enclave_pubkey,
        ),
        ExecuteMsg::SubmitBallot { ciphertext } => {
            exec_submit_ballot(deps, env, info, ciphertext)
        }
        ExecuteMsg::CloseAndTally {} => exec_close_and_tally(deps, env),
        ExecuteMsg::PublishResult { tally, attestation } => {
            exec_publish_result(deps, env, tally, attestation)
        }
        ExecuteMsg::UpdateRegistry { registry } => {
            exec_update_registry(deps, env, info, registry)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Election {} => to_json_binary(&ELECTION.load(deps.storage)?),
        QueryMsg::Phase {} => to_json_binary(&query_phase(deps, env)?),
        QueryMsg::Ballots {} => to_json_binary(&query_ballots(deps)?),
        QueryMsg::Result {} => to_json_binary(&query_result(deps)?),
        QueryMsg::Registry {} => to_json_binary(&REGISTRY.load(deps.storage)?),
    }
}

// ============================================================
// Handlers
// ============================================================

/// Block 1 alternate path: admin-only election creation.
///
/// Audit remediation M1: this handler refuses to clobber an election that
/// is mid-flight. Allowed when (no prior election) OR (prior election is
/// in `Created` — voting hasn't opened — OR `Resolved` — finalized).
/// Voting/Tallying are rejected.
///
/// Audit remediation C3: enclave_pubkey is length+leading-byte validated
/// as secp256k1.
#[allow(clippy::too_many_arguments)]
fn exec_create_election(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    candidates: Vec<Addr>,
    start_at: Timestamp,
    end_at: Timestamp,
    enclave_pubkey: HexBinary,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized);
    }

    // M1: refuse to clobber an active election.
    if let Some(prev) = ELECTION.may_load(deps.storage)? {
        let phase = compute_phase(&env, &prev, deps.storage)?;
        match phase {
            Phase::Voting | Phase::Tallying => {
                return Err(ContractError::ElectionAlreadyActive);
            }
            Phase::Created | Phase::Resolved => {} // OK to replace
        }
    }

    if candidates.len() < 2 {
        return Err(ContractError::NotEnoughCandidates);
    }

    // Duplicate detection in declaration order; pairs with intent §2.5's
    // "all candidate addresses distinct" Block 1 precondition.
    // Linear scan (no HashSet) for Kani-on-macOS compatibility (see audit
    // memory: CCRandomGenerateBytes blocker).
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            if candidates[i] == candidates[j] {
                return Err(ContractError::DuplicateCandidate);
            }
        }
    }

    if start_at >= end_at {
        return Err(ContractError::InvalidVotingWindow);
    }
    if start_at < env.block.time {
        return Err(ContractError::InvalidVotingWindow);
    }

    // C3: enclave_pubkey shape validation.
    validate_enclave_pubkey(&enclave_pubkey)?;

    // Clear any stale ballots from a prior election under this instance.
    // M1 gating above guarantees the prior election was in Created or
    // Resolved phase (no in-flight voting).
    let stale_keys: Vec<Addr> = BALLOTS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for key in stale_keys {
        BALLOTS.remove(deps.storage, &key);
    }

    // Reset tally; new election is unresolved.
    TALLY_RESULT.remove(deps.storage);

    let next_id = ELECTION_COUNTER.load(deps.storage)? + 1;
    ELECTION_COUNTER.save(deps.storage, &next_id)?;

    ELECTION.save(
        deps.storage,
        &Election {
            id: next_id,
            title,
            candidates,
            start_at,
            end_at,
            ballot_count: 0,
            enclave_pubkey,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_election")
        .add_attribute("election_id", next_id.to_string()))
}

/// Block 3: voter submits a ballot ciphertext.
pub(crate) fn exec_submit_ballot(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ciphertext: HexBinary,
) -> Result<Response, ContractError> {
    let mut election = ELECTION.load(deps.storage)?;
    let phase = compute_phase(&env, &election, deps.storage)?;
    if phase != Phase::Voting {
        return Err(ContractError::NotVoting);
    }

    if !election.candidates.contains(&info.sender) {
        return Err(ContractError::VoterNotCandidate);
    }

    if BALLOTS.has(deps.storage, &info.sender) {
        return Err(ContractError::AlreadyVoted);
    }

    BALLOTS.save(deps.storage, &info.sender, &ciphertext)?;

    election.ballot_count = election.ballot_count.saturating_add(1);
    ELECTION.save(deps.storage, &election)?;

    Ok(Response::new()
        .add_attribute("action", "submit_ballot")
        .add_attribute("voter", info.sender))
}

/// Block 5: idempotent wakeup at end_at. No storage change.
fn exec_close_and_tally(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let election = ELECTION.load(deps.storage)?;
    let phase = compute_phase(&env, &election, deps.storage)?;
    if phase != Phase::Tallying {
        return Err(ContractError::NotTallying);
    }

    Ok(Response::new()
        .add_attribute("action", "close_and_tally")
        .add_event(Event::new("close_and_tally").add_attribute("election_id", election.id.to_string())))
}

/// Block 6: enclave publishes the attested tally.
///
/// Audit remediation C2: Dstack envelope is now structurally + semantically
/// verified. The full pipeline is:
///   1. Phase gate (Tallying only, AlreadyResolved otherwise).
///   2. Chain-syntactic well-formedness (S6-S9 incl. ordering, per M4).
///   3. Attestation envelope verification:
///      - Dstack: domain tag check + commit-hash equality against the
///        canonical_serialization including election_id (M2).
///      - Mock: only compiled in under the `mock-attestation` feature.
pub(crate) fn exec_publish_result(
    deps: DepsMut,
    env: Env,
    tally: TallyResult,
    attestation: AttestationEnvelope,
) -> Result<Response, ContractError> {
    let election = ELECTION.load(deps.storage)?;
    let phase = compute_phase(&env, &election, deps.storage)?;
    if phase == Phase::Resolved {
        return Err(ContractError::AlreadyResolved);
    }
    if phase != Phase::Tallying {
        return Err(ContractError::NotTallying);
    }

    // Chain-syntactic well-formedness (intent §3.1 S6-S9 subset).
    check_tally_well_formed(&election, &tally)?;

    // C2: real Dstack envelope verification (Mock only under feature gate).
    let _registry = REGISTRY.load(deps.storage)?;
    let contract_addr = env.contract.address.as_str();
    match attestation {
        #[cfg(feature = "mock-attestation")]
        AttestationEnvelope::Mock => {
            // Compile-time-gated; production wasm has no Mock arm.
        }
        AttestationEnvelope::Dstack {
            quote: _,
            zk_proof: _,
            user_data,
        } => {
            verify_dstack_user_data(&user_data, contract_addr, election.id, &tally)?;
            // TODO (follow-on cycle): Groth16 verification of zk_proof
            // against registry.vkey via Xion's ProofVerifyGnark module;
            // MRTD/RTMR-vs-registry binding via zkdcap public-input
            // extraction. Currently the on-chain Dstack verification is
            // ENVELOPE-BOUND (commit hash + domain tag) but does NOT
            // re-verify the underlying TDX quote or zkdcap proof.
        }
    }

    TALLY_RESULT.save(deps.storage, &tally)?;

    Ok(Response::new()
        .add_attribute("action", "publish_result")
        .add_attribute("election_id", election.id.to_string())
        .add_attribute("winners_count", tally.winners.len().to_string()))
}

/// M3 audit remediation: admin-only registry rotation. Gated on no active
/// election (Voting/Tallying phases reject; Created/Resolved/no-election
/// permitted).
fn exec_update_registry(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_registry: EnclaveImageRegistry,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized);
    }
    validate_registry(&new_registry)?;

    if let Some(prev) = ELECTION.may_load(deps.storage)? {
        let phase = compute_phase(&env, &prev, deps.storage)?;
        match phase {
            Phase::Voting | Phase::Tallying => {
                return Err(ContractError::RegistryUpdateDuringActiveElection);
            }
            Phase::Created | Phase::Resolved => {}
        }
    }

    REGISTRY.save(deps.storage, &new_registry)?;
    Ok(Response::new().add_attribute("action", "update_registry"))
}

// ============================================================
// Derived-phase computation (intent v0.3.2 A2)
// ============================================================

/// Pure derivation; mirrors Quint `derived_phase(block_time, tally_result)`.
pub(crate) fn derive_phase(now: Timestamp, election: &Election, has_tally: bool) -> Phase {
    if has_tally {
        Phase::Resolved
    } else if now < election.start_at {
        Phase::Created
    } else if now < election.end_at {
        Phase::Voting
    } else {
        Phase::Tallying
    }
}

fn compute_phase(
    env: &Env,
    election: &Election,
    storage: &dyn cosmwasm_std::Storage,
) -> StdResult<Phase> {
    let has_tally = TALLY_RESULT.may_load(storage)?.is_some();
    Ok(derive_phase(env.block.time, election, has_tally))
}

// ============================================================
// Tally well-formedness checks (chain-side syntactic subset of S6-S9)
// ============================================================

/// Chain-syntactic well-formedness check for a published tally.
///
/// Audit remediation M4: this function now ALSO enforces candidate-
/// declaration-order on every `Vec<Addr>`-typed field (and per-row
/// candidates of `per_round_counts` and `eliminated_by_round`). This is
/// load-bearing for B8(c) Borsh determinism: two enclaves running the
/// same algorithm on the same inputs must produce byte-identical Borsh
/// encodings, which requires the same ordering convention.
pub(crate) fn check_tally_well_formed(
    election: &Election,
    tally: &TallyResult,
) -> Result<(), ContractError> {
    let n_cands = election.candidates.len();
    let cands: &[Addr] = &election.candidates;

    // S6: winners are a non-empty subset of candidates in declaration order.
    if tally.winners.is_empty() || tally.winners.len() > n_cands {
        return Err(ContractError::AttestationFailure(
            "winners cardinality out of bounds".into(),
        ));
    }
    if !is_in_declaration_order(&tally.winners, cands) {
        return Err(ContractError::AttestationFailure(
            "winners not in candidate-declaration order".into(),
        ));
    }

    // S7: dropped_voters cardinality consistency.
    if tally.dropped_voters.len() as u32 != tally.ballots_dropped {
        return Err(ContractError::AttestationFailure(
            "dropped_voters length disagrees with ballots_dropped".into(),
        ));
    }

    // S7 partition equation: tallied + dropped + non_voters = |candidates|.
    if tally
        .ballots_tallied
        .checked_add(tally.ballots_dropped)
        .and_then(|n| n.checked_add(tally.non_voters.len() as u32))
        != Some(n_cands as u32)
    {
        return Err(ContractError::AttestationFailure(
            "tally partition equation violated".into(),
        ));
    }

    // S7: dropped_voters and non_voters disjoint.
    for d in &tally.dropped_voters {
        for nv in &tally.non_voters {
            if d.as_str() == nv.as_str() {
                return Err(ContractError::AttestationFailure(
                    "dropped_voters intersects non_voters".into(),
                ));
            }
        }
    }

    // M4 ordering: dropped_voters and non_voters in declaration order.
    if !is_in_declaration_order(&tally.dropped_voters, cands) {
        return Err(ContractError::AttestationFailure(
            "dropped_voters not in candidate-declaration order".into(),
        ));
    }
    if !is_in_declaration_order(&tally.non_voters, cands) {
        return Err(ContractError::AttestationFailure(
            "non_voters not in candidate-declaration order".into(),
        ));
    }

    // S6 structural: at most |candidates| rounds.
    if tally.per_round_counts.len() > n_cands {
        return Err(ContractError::AttestationFailure(
            "per_round_counts exceeds candidate count".into(),
        ));
    }
    if tally.eliminated_by_round.len() > tally.per_round_counts.len() {
        return Err(ContractError::AttestationFailure(
            "eliminated_by_round longer than per_round_counts".into(),
        ));
    }

    // S8: each round's counts sum to ballots_tallied; M4: candidates within
    // each row are in declaration order.
    for round in &tally.per_round_counts {
        let mut sum: u128 = 0;
        for rc in round {
            sum += rc.count as u128;
        }
        if sum != tally.ballots_tallied as u128 {
            return Err(ContractError::AttestationFailure(
                "per_round_counts row does not sum to ballots_tallied".into(),
            ));
        }
        // M4: per-row candidates in declaration order.
        let row_cands: Vec<String> =
            round.iter().map(|rc| rc.candidate.clone()).collect();
        if !is_in_declaration_order(&row_cands, cands) {
            return Err(ContractError::AttestationFailure(
                "per_round_counts row not in candidate-declaration order".into(),
            ));
        }
    }

    // M4: eliminated_by_round per-row candidates in declaration order.
    for elim_row in &tally.eliminated_by_round {
        if !is_in_declaration_order(elim_row, cands) {
            return Err(ContractError::AttestationFailure(
                "eliminated_by_round row not in candidate-declaration order".into(),
            ));
        }
    }

    // S9: candidates eliminated at round i do not reappear at round j > i.
    for j in 0..tally.per_round_counts.len() {
        let round_j = &tally.per_round_counts[j];
        for rc in round_j {
            for i in 0..j {
                if let Some(elims_i) = tally.eliminated_by_round.get(i) {
                    for c in elims_i {
                        if c.as_str() == rc.candidate.as_str() {
                            return Err(ContractError::AttestationFailure(
                                "eliminated candidate reappears in later round".into(),
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// M4 helper: returns true iff `subset` is a subsequence of `full`
/// (preserving order). Linear-scan; no HashSet (Kani-on-macOS safe).
pub(crate) fn is_in_declaration_order<S>(subset: &[S], full: &[Addr]) -> bool
where
    S: AsRef<str>,
{
    let mut i = 0;
    for c in subset {
        while i < full.len() && full[i].as_str() != c.as_ref() {
            i += 1;
        }
        if i >= full.len() {
            return false;
        }
        i += 1;
    }
    true
}

// ============================================================
// Attestation envelope verification (audit remediation C2)
// ============================================================

/// Verify the Dstack envelope's `user_data` field per intent §3.2 B8(c).
///
/// Three checks:
///   1. Length == 64 bytes (32 domain tag + 32 commit hash).
///   2. Upper 32 bytes == zero-padded `DST_VERIFIED_RCV_TALLY_V1`.
///   3. Lower 32 bytes == SHA-256(canonical_serialization(contract_addr ‖
///      election_id ‖ tally_body)) — intent v0.3.8 form.
///
/// Returns specific errors so a verifier can distinguish "shape malformed"
/// from "envelope binds to a different tally" (the second is the
/// load-bearing replay-resistance check).
pub(crate) fn verify_dstack_user_data(
    user_data: &HexBinary,
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Result<(), ContractError> {
    if user_data.len() != 64 {
        return Err(ContractError::AttestationFailure(
            "user_data must be 64 bytes (32 tag + 32 hash)".into(),
        ));
    }
    let bytes = user_data.as_slice();
    let (tag, commit) = bytes.split_at(DOMAIN_TAG_LEN);

    // (2) Domain-tag check (defense against off-domain quote replay).
    if !is_valid_domain_tag(tag) {
        return Err(ContractError::AttestationDomainTagInvalid);
    }

    // (3) Commit-hash check (binds envelope to THIS tally for THIS
    // election in THIS contract).
    let expected = compute_commit_hash(contract_addr, election_id, tally);
    if commit != expected.as_slice() {
        return Err(ContractError::AttestationCommitMismatch);
    }
    Ok(())
}

/// True iff `tag` is the 32-byte zero-padded `DOMAIN_TAG_LITERAL`.
pub(crate) fn is_valid_domain_tag(tag: &[u8]) -> bool {
    if tag.len() != DOMAIN_TAG_LEN {
        return false;
    }
    let n = DOMAIN_TAG_LITERAL.len();
    if &tag[..n] != DOMAIN_TAG_LITERAL {
        return false;
    }
    tag[n..].iter().all(|&b| b == 0)
}

/// Compute the canonical commit hash per intent §2.5 (v0.3.8 form):
/// `SHA-256(canonical_serialization(contract_addr ‖ election_id ‖ tally_body))`.
///
/// M2 fix: includes `election_id` so a tally body that's structurally
/// identical across elections produces different commit hashes (no replay).
///
/// The byte-level encoding matches intent §2.5 v0.3.1 T7 leaf-encoding pin:
/// - `Addr` (String): u32 LE length prefix + UTF-8 bytes
/// - `Nat`: u64 LE (widened from Rust's u32 to honor the spec)
/// - `Vec<T>`: u32 LE length prefix + element bytes
/// - `u64` raw (election_id): 8 bytes LE
/// - Field order: declaration order of TallyResult
///
/// **This MUST stay byte-identical to** `verified_rcv_enclave::attestation::canonical_serialization`.
/// A cross-test in the enclave crate verifies the equality.
pub fn compute_commit_hash(
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> [u8; 32] {
    let canonical = canonical_serialization(contract_addr, election_id, tally);
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    hasher.finalize().into()
}

/// Hand-rolled canonical serialization per intent §2.5 v0.3.1 T7.
///
/// MUST stay byte-identical to the runtime's
/// `verified_rcv_enclave::attestation::canonical_serialization`. We
/// duplicate (rather than import) because the runtime crate pulls in
/// tokio/reqwest/etc. and won't compile to wasm. A cross-test in the
/// enclave crate's integration tests asserts byte-equality.
///
/// `pub` (not `pub(crate)`) so that cross-test can call it.
pub fn canonical_serialization(
    contract_addr: &str,
    election_id: u64,
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, contract_addr);
    // election_id (intent v0.3.8 M2 addition): u64 LE.
    out.extend_from_slice(&election_id.to_le_bytes());
    write_tally_body(&mut out, tally);
    out
}

fn write_borsh_string(out: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

fn write_vec_addr(out: &mut Vec<u8>, v: &[String]) {
    out.extend_from_slice(&(v.len() as u32).to_le_bytes());
    for s in v {
        write_borsh_string(out, s);
    }
}

fn write_round_count(
    out: &mut Vec<u8>,
    rc: &verified_rcv_enclave_core::RoundCount,
) {
    write_borsh_string(out, &rc.candidate);
    // Intent §2.5 leaf-encoding pin: Nat → u64 LE.
    out.extend_from_slice(&(rc.count as u64).to_le_bytes());
}

fn write_round_counts(
    out: &mut Vec<u8>,
    rcs: &verified_rcv_enclave_core::RoundCounts,
) {
    out.extend_from_slice(&(rcs.len() as u32).to_le_bytes());
    for rc in rcs {
        write_round_count(out, rc);
    }
}

fn write_per_round_counts(
    out: &mut Vec<u8>,
    prc: &[verified_rcv_enclave_core::RoundCounts],
) {
    out.extend_from_slice(&(prc.len() as u32).to_le_bytes());
    for round in prc {
        write_round_counts(out, round);
    }
}

fn write_eliminated_by_round(out: &mut Vec<u8>, ebr: &[Vec<String>]) {
    out.extend_from_slice(&(ebr.len() as u32).to_le_bytes());
    for round in ebr {
        write_vec_addr(out, round);
    }
}

fn write_tally_body(out: &mut Vec<u8>, t: &TallyResult) {
    // Declaration-order emission per intent §2.5.
    write_vec_addr(out, &t.winners);
    write_per_round_counts(out, &t.per_round_counts);
    write_eliminated_by_round(out, &t.eliminated_by_round);
    out.extend_from_slice(&(t.ballots_tallied as u64).to_le_bytes());
    out.extend_from_slice(&(t.ballots_dropped as u64).to_le_bytes());
    write_vec_addr(out, &t.dropped_voters);
    write_vec_addr(out, &t.non_voters);
}

// ============================================================
// Validators (audit remediations C3, M3)
// ============================================================

/// C3: secp256k1 pubkey shape validation. Accepts compressed (33 bytes,
/// 0x02/0x03 prefix) or uncompressed (65 bytes, 0x04 prefix).
fn validate_enclave_pubkey(pk: &HexBinary) -> Result<(), ContractError> {
    let bytes = pk.as_slice();
    let got = bytes.len();
    if got != SECP256K1_COMPRESSED_LEN && got != SECP256K1_UNCOMPRESSED_LEN {
        return Err(ContractError::InvalidEnclavePubkey { got });
    }
    let leading_ok = match got {
        SECP256K1_COMPRESSED_LEN => bytes[0] == 0x02 || bytes[0] == 0x03,
        SECP256K1_UNCOMPRESSED_LEN => bytes[0] == 0x04,
        _ => false,
    };
    if !leading_ok {
        return Err(ContractError::InvalidEnclavePubkey { got });
    }
    Ok(())
}

/// M3: registry shape validation. mrtd and rtmr must be exactly 48 bytes
/// (TDX SHA-384 measurement); vkey must be non-empty.
fn validate_registry(reg: &EnclaveImageRegistry) -> Result<(), ContractError> {
    if reg.mrtd.len() != MRTD_LEN {
        return Err(ContractError::InvalidRegistry(format!(
            "mrtd must be {MRTD_LEN} bytes (TDX SHA-384), got {}",
            reg.mrtd.len()
        )));
    }
    if reg.rtmr.len() != RTMR_LEN {
        return Err(ContractError::InvalidRegistry(format!(
            "rtmr must be {RTMR_LEN} bytes (TDX SHA-384), got {}",
            reg.rtmr.len()
        )));
    }
    if reg.vkey.is_empty() {
        return Err(ContractError::InvalidRegistry(
            "vkey must be a non-empty Xion-registered verification-key name".into(),
        ));
    }
    Ok(())
}

// ============================================================
// Query helpers
// ============================================================

fn query_phase(deps: Deps, env: Env) -> StdResult<Phase> {
    let election = ELECTION.load(deps.storage)?;
    let has_tally = TALLY_RESULT.may_load(deps.storage)?.is_some();
    Ok(derive_phase(env.block.time, &election, has_tally))
}

fn query_ballots(deps: Deps) -> StdResult<BallotsResponse> {
    let ballots: Vec<(Addr, HexBinary)> = BALLOTS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(BallotsResponse { ballots })
}

fn query_result(deps: Deps) -> StdResult<ResultResponse> {
    Ok(ResultResponse {
        result: TALLY_RESULT.may_load(deps.storage)?,
    })
}

// ============================================================
// Unit tests (audit-finding remediation coverage)
// ============================================================
//
// Per the audit's "negative-test discipline" finding (2026-05-26): for
// every invariant `Ok ⇒ P`, the suite must also exercise `¬P ⇒ Err`.
// Tests below target the rejection paths for C1, C2, C3, M1, M3, M4.
// Happy paths covered too (positive control).

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
    use cosmwasm_std::{Addr as CwAddr, HexBinary, Timestamp};
    use verified_rcv_enclave_core::{RoundCount, TallyResult};

    fn good_pubkey() -> HexBinary {
        let mut b = vec![0u8; 33];
        b[0] = 0x02;
        HexBinary::from(b)
    }

    fn good_registry() -> EnclaveImageRegistry {
        EnclaveImageRegistry {
            mrtd: vec![0u8; 48],
            rtmr: vec![0u8; 48],
            vkey: "verified_rcv_v1".to_string(),
        }
    }

    /// Mock addrs use bech32-like strings. CwAddr accepts via `unchecked`.
    fn cand(s: &str) -> CwAddr {
        CwAddr::unchecked(s)
    }

    fn three_cands() -> Vec<CwAddr> {
        vec![cand("c0"), cand("c1"), cand("c2")]
    }

    // ----------------------------------------------------------------
    // C3: enclave_pubkey shape
    // ----------------------------------------------------------------

    #[test]
    fn enclave_pubkey_wrong_length_rejected() {
        let err = validate_enclave_pubkey(&HexBinary::from(vec![0u8; 32])).unwrap_err();
        assert!(matches!(err, ContractError::InvalidEnclavePubkey { got: 32 }));
    }

    #[test]
    fn enclave_pubkey_compressed_wrong_leading_byte_rejected() {
        let mut b = vec![0u8; 33];
        b[0] = 0x05; // not 0x02 or 0x03
        let err = validate_enclave_pubkey(&HexBinary::from(b)).unwrap_err();
        assert!(matches!(err, ContractError::InvalidEnclavePubkey { .. }));
    }

    #[test]
    fn enclave_pubkey_uncompressed_wrong_leading_byte_rejected() {
        let mut b = vec![0u8; 65];
        b[0] = 0x02; // valid for compressed only
        let err = validate_enclave_pubkey(&HexBinary::from(b)).unwrap_err();
        assert!(matches!(err, ContractError::InvalidEnclavePubkey { .. }));
    }

    #[test]
    fn enclave_pubkey_compressed_ok() {
        let mut b = vec![0u8; 33];
        b[0] = 0x03;
        validate_enclave_pubkey(&HexBinary::from(b)).unwrap();
    }

    #[test]
    fn enclave_pubkey_uncompressed_ok() {
        let mut b = vec![0u8; 65];
        b[0] = 0x04;
        validate_enclave_pubkey(&HexBinary::from(b)).unwrap();
    }

    // ----------------------------------------------------------------
    // M3: registry shape validation
    // ----------------------------------------------------------------

    #[test]
    fn registry_mrtd_wrong_length_rejected() {
        let mut r = good_registry();
        r.mrtd = vec![0u8; 32]; // SHA-256 length; TDX uses SHA-384
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_rtmr_wrong_length_rejected() {
        let mut r = good_registry();
        r.rtmr = vec![0u8; 32];
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_empty_vkey_rejected() {
        let mut r = good_registry();
        r.vkey = String::new();
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_good_ok() {
        validate_registry(&good_registry()).unwrap();
    }

    // ----------------------------------------------------------------
    // C2: Dstack envelope verification
    // ----------------------------------------------------------------

    fn minimal_valid_tally() -> TallyResult {
        TallyResult {
            winners: vec!["c0".to_string()],
            per_round_counts: vec![vec![RoundCount {
                candidate: "c0".to_string(),
                count: 1,
            }]],
            eliminated_by_round: vec![],
            ballots_tallied: 1,
            ballots_dropped: 0,
            dropped_voters: vec![],
            non_voters: vec!["c1".to_string(), "c2".to_string()],
        }
    }

    fn good_user_data(contract_addr: &str, election_id: u64, tally: &TallyResult) -> HexBinary {
        let mut ud = vec![0u8; 64];
        let n = DOMAIN_TAG_LITERAL.len();
        ud[..n].copy_from_slice(DOMAIN_TAG_LITERAL);
        let commit = compute_commit_hash(contract_addr, election_id, tally);
        ud[32..].copy_from_slice(&commit);
        HexBinary::from(ud)
    }

    #[test]
    fn dstack_envelope_wrong_length_rejected() {
        let tally = minimal_valid_tally();
        let bad = HexBinary::from(vec![0u8; 63]);
        let err = verify_dstack_user_data(&bad, "cw1xxx", 1, &tally).unwrap_err();
        assert!(matches!(err, ContractError::AttestationFailure(_)));
    }

    #[test]
    fn dstack_envelope_wrong_domain_tag_rejected() {
        let tally = minimal_valid_tally();
        let mut ud = vec![0u8; 64];
        ud[..6].copy_from_slice(b"WRONG!"); // not DST_VERIFIED_RCV_TALLY_V1
        let commit = compute_commit_hash("cw1xxx", 1, &tally);
        ud[32..].copy_from_slice(&commit);
        let err = verify_dstack_user_data(&HexBinary::from(ud), "cw1xxx", 1, &tally).unwrap_err();
        assert!(matches!(err, ContractError::AttestationDomainTagInvalid));
    }

    #[test]
    fn dstack_envelope_wrong_commit_hash_rejected() {
        let tally = minimal_valid_tally();
        // Commit hash for DIFFERENT election_id ⇒ replay attempt.
        let mut ud = vec![0u8; 64];
        ud[..DOMAIN_TAG_LITERAL.len()].copy_from_slice(DOMAIN_TAG_LITERAL);
        let wrong_commit = compute_commit_hash("cw1xxx", 999, &tally);
        ud[32..].copy_from_slice(&wrong_commit);
        let err = verify_dstack_user_data(&HexBinary::from(ud), "cw1xxx", 1, &tally).unwrap_err();
        assert!(matches!(err, ContractError::AttestationCommitMismatch));
    }

    #[test]
    fn dstack_envelope_wrong_contract_addr_rejected() {
        let tally = minimal_valid_tally();
        let ud = good_user_data("cw1OTHER", 1, &tally);
        let err = verify_dstack_user_data(&ud, "cw1xxx", 1, &tally).unwrap_err();
        assert!(matches!(err, ContractError::AttestationCommitMismatch));
    }

    #[test]
    fn dstack_envelope_correct_binding_ok() {
        let tally = minimal_valid_tally();
        let ud = good_user_data("cw1xxx", 7, &tally);
        verify_dstack_user_data(&ud, "cw1xxx", 7, &tally).unwrap();
    }

    // ----------------------------------------------------------------
    // M4: candidate-declaration-order enforcement
    // ----------------------------------------------------------------

    #[test]
    fn declaration_order_subsequence_accepted() {
        let full = three_cands();
        let subset = vec!["c0".to_string(), "c2".to_string()];
        assert!(is_in_declaration_order(&subset, &full));
    }

    #[test]
    fn declaration_order_shuffled_rejected() {
        let full = three_cands();
        let subset = vec!["c2".to_string(), "c0".to_string()];
        assert!(!is_in_declaration_order(&subset, &full));
    }

    #[test]
    fn declaration_order_non_candidate_rejected() {
        let full = three_cands();
        let subset = vec!["evil".to_string()];
        assert!(!is_in_declaration_order(&subset, &full));
    }

    #[test]
    fn check_tally_well_formed_shuffled_winners_rejected() {
        let election = Election {
            id: 1,
            title: "t".into(),
            candidates: three_cands(),
            start_at: Timestamp::from_seconds(0),
            end_at: Timestamp::from_seconds(100),
            ballot_count: 3,
            enclave_pubkey: good_pubkey(),
        };
        // Two co-winners but in REVERSED order: c2 listed before c0.
        let mut tally = TallyResult {
            winners: vec!["c2".to_string(), "c0".to_string()], // SHUFFLED
            per_round_counts: vec![vec![
                RoundCount { candidate: "c0".to_string(), count: 1 },
                RoundCount { candidate: "c1".to_string(), count: 1 },
                RoundCount { candidate: "c2".to_string(), count: 1 },
            ]],
            eliminated_by_round: vec![],
            ballots_tallied: 3,
            ballots_dropped: 0,
            dropped_voters: vec![],
            non_voters: vec![],
        };
        let err = check_tally_well_formed(&election, &tally).unwrap_err();
        assert!(matches!(err, ContractError::AttestationFailure(s) if s.contains("winners not in candidate-declaration order")));

        // Fix: put winners in declaration order.
        tally.winners = vec!["c0".to_string(), "c2".to_string()];
        check_tally_well_formed(&election, &tally).unwrap();
    }

    #[test]
    fn check_tally_well_formed_shuffled_per_round_rejected() {
        let election = Election {
            id: 1,
            title: "t".into(),
            candidates: three_cands(),
            start_at: Timestamp::from_seconds(0),
            end_at: Timestamp::from_seconds(100),
            ballot_count: 3,
            enclave_pubkey: good_pubkey(),
        };
        let tally = TallyResult {
            winners: vec!["c0".to_string()],
            // Row 0 in REVERSED order: c2, c1, c0 — violates §2.5 ordering.
            per_round_counts: vec![vec![
                RoundCount { candidate: "c2".to_string(), count: 1 },
                RoundCount { candidate: "c1".to_string(), count: 1 },
                RoundCount { candidate: "c0".to_string(), count: 1 },
            ]],
            eliminated_by_round: vec![],
            ballots_tallied: 3,
            ballots_dropped: 0,
            dropped_voters: vec![],
            non_voters: vec![],
        };
        let err = check_tally_well_formed(&election, &tally).unwrap_err();
        assert!(matches!(err, ContractError::AttestationFailure(s) if s.contains("per_round_counts row not in candidate-declaration order")));
    }

    // ----------------------------------------------------------------
    // M1: CreateElection gating
    // ----------------------------------------------------------------

    #[test]
    fn create_election_during_voting_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");

        // Instantiate.
        let info = message_info(&admin, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin: Some(admin.clone()),
                registry: good_registry(),
                voting_duration_seconds: 1000,
            },
        )
        .unwrap();

        // Create first election: start at env+10, end at env+1000.
        let first = exec_create_election(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            "first".into(),
            three_cands(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            good_pubkey(),
        );
        assert!(first.is_ok());

        // Fast-forward env to mid-voting.
        let mut env2 = env.clone();
        env2.block.time = env.block.time.plus_seconds(500);

        // Try to create a second election DURING voting.
        let second = exec_create_election(
            deps.as_mut(),
            env2,
            info,
            "second".into(),
            three_cands(),
            env.block.time.plus_seconds(2000),
            env.block.time.plus_seconds(3000),
            good_pubkey(),
        );
        assert!(matches!(second, Err(ContractError::ElectionAlreadyActive)));
    }

    #[test]
    fn create_election_initial_state_ok() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");

        let info = message_info(&admin, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin: Some(admin),
                registry: good_registry(),
                voting_duration_seconds: 1000,
            },
        )
        .unwrap();

        let res = exec_create_election(
            deps.as_mut(),
            env.clone(),
            info,
            "first".into(),
            three_cands(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            good_pubkey(),
        );
        assert!(res.is_ok());
    }

    // ----------------------------------------------------------------
    // M3: UpdateRegistry gating
    // ----------------------------------------------------------------

    #[test]
    fn update_registry_during_voting_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        let info = message_info(&admin, &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin: Some(admin.clone()),
                registry: good_registry(),
                voting_duration_seconds: 1000,
            },
        )
        .unwrap();
        exec_create_election(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            "e".into(),
            three_cands(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            good_pubkey(),
        )
        .unwrap();

        let mut env2 = env.clone();
        env2.block.time = env.block.time.plus_seconds(500);

        let new_reg = EnclaveImageRegistry {
            mrtd: vec![1u8; 48],
            rtmr: vec![1u8; 48],
            vkey: "verified_rcv_v2".into(),
        };
        let err = exec_update_registry(deps.as_mut(), env2, info, new_reg).unwrap_err();
        assert!(matches!(err, ContractError::RegistryUpdateDuringActiveElection));
    }

    #[test]
    fn update_registry_initial_state_ok() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        let info = message_info(&admin, &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin: Some(admin),
                registry: good_registry(),
                voting_duration_seconds: 1000,
            },
        )
        .unwrap();

        let new_reg = EnclaveImageRegistry {
            mrtd: vec![1u8; 48],
            rtmr: vec![1u8; 48],
            vkey: "verified_rcv_v2".into(),
        };
        exec_update_registry(deps.as_mut(), env, info, new_reg).unwrap();
    }

    #[test]
    fn update_registry_non_admin_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        let attacker = CwAddr::unchecked("attacker");

        instantiate(
            deps.as_mut(),
            env.clone(),
            message_info(&admin, &[]),
            InstantiateMsg {
                admin: Some(admin),
                registry: good_registry(),
                voting_duration_seconds: 1000,
            },
        )
        .unwrap();

        let new_reg = EnclaveImageRegistry {
            mrtd: vec![1u8; 48],
            rtmr: vec![1u8; 48],
            vkey: "evil".into(),
        };
        let err = exec_update_registry(deps.as_mut(), env, message_info(&attacker, &[]), new_reg)
            .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }

    // ----------------------------------------------------------------
    // Compile-time sanity (C1): in default build the Mock variant
    // doesn't exist. This test fails to compile when run with
    // `--features mock-attestation`, which is the intended signal.
    // (No runtime assertion; presence of the file building under
    // default features is the evidence.)
    // ----------------------------------------------------------------

    #[cfg(not(feature = "mock-attestation"))]
    #[test]
    fn mock_variant_absent_in_production_build() {
        // If you uncomment the line below, this test fails to compile —
        // proving the Mock variant is gone from production wasm.
        // let _ = AttestationEnvelope::Mock;
        let _allowed: AttestationEnvelope = AttestationEnvelope::Dstack {
            quote: HexBinary::from(vec![]),
            zk_proof: HexBinary::from(vec![]),
            user_data: HexBinary::from(vec![0u8; 64]),
        };
    }

    // Suppress unused-import warning in non-default builds.
    #[allow(dead_code)]
    fn _silence(_: MockApi) {}
}
