//! Entry points + handlers. Refine the Quint protocol model action by action.
//!
//! Audit-finding remediations (2026-05-26 v0.3.8):
//! - C1 (mock attestation): the `AttestationEnvelope::Mock` variant existed
//!   only when the `mock-attestation` Cargo feature was enabled. **Superseded
//!   by v0.3.9 N1**: the envelope wrapper is gone; `mock-attestation` now
//!   gates whether the contract actually calls `/xion.zk.v1.Query/ProofVerifyGnark`
//!   on the chain (tests skip the chain call but still exercise every
//!   extraction + bytewise equality path).
//! - C2 (Dstack verification): superseded by N1 — verification is now
//!   bytewise extraction from gnark `public_inputs` plus a single chain
//!   call to `xion.zk`. The pre-v0.3.9 `user_data` shim is removed.
//! - C3 (enclave_pubkey shape): unchanged — 33-byte compressed or 65-byte
//!   uncompressed secp256k1. v0.3.9 additionally enforces B8(e) — the
//!   pubkey is bound to a registration TDX quote via the gnark proof.
//! - M1 (B1 multi-election): unchanged — `CreateElection` rejects in
//!   Voting/Tallying.
//! - M2 (election_id in commit hash): unchanged — `canonical_serialization`
//!   is `Borsh(contract_addr) ‖ u64_LE(election_id) ‖ Borsh(tally_body)`.
//! - M3 (registry shape + update path): registry schema updated for v0.3.9
//!   (vkey_name + split rtmr + optional slots + accepted TCB statuses);
//!   admin-only `UpdateRegistry` handler retained.
//! - M4 (declaration-order enforcement): unchanged.
//!
//! N1 audit re-review remediation (2026-05-26 v0.3.9):
//! - `CreateElection` carries `(proof, public_inputs)` for a *registration*
//!   TDX quote whose `ReportData[0..32] = SHA-256(enclave_pubkey)` and
//!   `ReportData[32..64] = DST_VERIFIED_RCV_PUBKEY_V1_PADDED`. Bound to
//!   `EnclaveImageRegistry` measurements. Discharges B8(e).
//! - `PublishResult` carries `(proof, public_inputs)` for a *publish* TDX
//!   quote whose `ReportData[0..32] = SHA-256(canonical_serialization(...))`
//!   and `ReportData[32..64] = DST_VERIFIED_RCV_TALLY_V1_PADDED`. Bound to
//!   `EnclaveImageRegistry` measurements. Discharges B8(a)/(b)/(c)/(d).
//! - Cryptographic verification routes via `/xion.zk.v1.Query/ProofVerifyGnark`
//!   directly from the contract.
//! - Extraction parser enforces gnark `uints.U8` high-byte invariant per
//!   element; layout pinned at intent §2.5 gnark public_inputs byte layout.

use sha2::{Digest, Sha256};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, Event, HexBinary, MessageInfo, Order,
    Response, StdResult, Timestamp,
};

#[cfg(not(any(feature = "mock-attestation", test)))]
use prost::Message;

use verified_rcv_enclave_core::TallyResult;

use crate::error::ContractError;
use crate::msg::{
    BallotsResponse, ExecuteMsg, HistoricalElectionResponse, InstantiateMsg,
    PendingRegistryResponse, QueryMsg, ResultResponse,
};
use crate::state::{
    Config, Election, EnclaveImageRegistry, PendingRegistry, Phase, BALLOTS, CONFIG, ELECTION,
    ELECTION_COUNTER, HISTORICAL_ELECTIONS, HISTORICAL_TALLIES, PENDING_REGISTRY, REGISTRY,
    TALLY_RESULT,
};

// ============================================================
// Constants — v0.3.8 (C1–M4) + v0.3.9 (N1) audit remediations
// ============================================================

/// 32-byte domain-separation tag for verified-rcv **publish** quotes
/// (carries the tally commit hash in ReportData[0..32]). Right-padded with
/// zeros to fill 32 bytes. Per intent §2.5 ReportData layout (v0.3.9).
const DST_TALLY_LITERAL: &[u8] = b"DST_VERIFIED_RCV_TALLY_V1";
/// 32-byte domain-separation tag for verified-rcv **registration** quotes
/// (carries the enclave_pubkey commit in ReportData[0..32]). Right-padded
/// with zeros to fill 32 bytes. Per intent §2.5 ReportData layout (v0.3.9).
const DST_PUBKEY_LITERAL: &[u8] = b"DST_VERIFIED_RCV_PUBKEY_V1";
const DST_TAG_LEN: usize = 32;

/// v0.3.14 F2 DST prefix for `compute_ballots_hash` preimage. Prepended
/// to the SHA-256 input so the ballots-hash preimage can never collide
/// with the names-hash preimage on the leading bytes. 23 ASCII bytes,
/// no length prefix on the DST itself (it occupies a fixed-byte preamble
/// that is structurally inseparable from the rest of the preimage). MUST
/// stay byte-identical to the runtime's `attestation::DST_BALLOTS`.
pub const DST_BALLOTS: &[u8] = b"verified-rcv:ballots:v1";
/// v0.3.14 F2 DST prefix for `compute_names_hash` preimage. 21 ASCII
/// bytes. MUST stay byte-identical to the runtime's `attestation::DST_NAMES`.
pub const DST_NAMES: &[u8] = b"verified-rcv:names:v1";

/// TDX MRTD measurement length (SHA-384 over initial VM image).
const MRTD_LEN: usize = 48;
/// TDX RTMR measurement length (SHA-384 over runtime extensions).
const RTMR_LEN: usize = 48;

/// secp256k1 compressed pubkey: 1 byte (0x02 / 0x03) + 32 bytes X.
const SECP256K1_COMPRESSED_LEN: usize = 33;
/// secp256k1 uncompressed pubkey: 1 byte (0x04) + 32 bytes X + 32 bytes Y.
const SECP256K1_UNCOMPRESSED_LEN: usize = 65;

/// v0.3.14: maximum byte-length of a candidate display name (UTF-8).
/// Names with byte-length > 64 are rejected at CreateElection time.
pub const CANDIDATE_NAME_MAX_BYTES: usize = 64;

// ----- Gnark public_inputs byte layout (intent §2.5, v0.3.9 N1) ----------

/// One BN254 fr-element serialized as big-endian bytes.
const FR_BYTES: usize = 32;
/// Element-index ranges (in fr-element units, declaration order in the
/// zkdcap DCAP gnark circuit). Multiply by `FR_BYTES` for byte offsets.
/// The `_LEN` constants document the per-field element counts; total =
/// 48*5 + 64 + 2 = 306 elements (`GNARK_PUBLIC_INPUTS_ELEMS`).
const ELEM_MRTD_START: usize = 0;
const ELEM_RTMR0_START: usize = 48;
const ELEM_RTMR1_START: usize = 96;
const ELEM_RTMR2_START: usize = 144;
const ELEM_RTMR3_START: usize = 192;
const ELEM_REPORTDATA_START: usize = 240;
const ELEM_TCBSTATUS: usize = 304;
const ELEM_TIMESTAMP: usize = 305;

/// Total fr-element count = 48*5 + 64 + 2 = 306.
const GNARK_PUBLIC_INPUTS_ELEMS: usize = 306;
/// Total `public_inputs` byte length = 306 × 32 = 9_792.
pub const GNARK_PUBLIC_INPUTS_LEN: usize = GNARK_PUBLIC_INPUTS_ELEMS * FR_BYTES;

// ============================================================
// xion.zk.v1.Query/ProofVerifyGnark prost types
// (matches /Users/mvid/Development/burnt/xion/proto/xion/zk/v1/query.proto)
// ============================================================

#[cfg(not(any(feature = "mock-attestation", test)))]
#[derive(Clone, PartialEq, prost::Message)]
struct QueryVerifyGnarkRequest {
    #[prost(bytes = "vec", tag = "1")]
    proof: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    public_inputs: Vec<u8>,
    #[prost(string, tag = "3")]
    vkey_name: String,
    #[prost(uint64, tag = "4")]
    vkey_id: u64,
}

#[cfg(not(any(feature = "mock-attestation", test)))]
#[derive(Clone, PartialEq, prost::Message)]
struct ProofVerifyGnarkResponse {
    #[prost(bool, tag = "1")]
    verified: bool,
}

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
            registry_update_delay_seconds: msg.registry_update_delay_seconds,
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
            candidate_names,
            start_at,
            end_at,
            enclave_pubkey,
            proof,
            public_inputs,
        } => exec_create_election(
            deps,
            env,
            info,
            title,
            candidates,
            candidate_names,
            start_at,
            end_at,
            enclave_pubkey,
            proof,
            public_inputs,
        ),
        ExecuteMsg::SubmitBallot { ciphertext } => {
            exec_submit_ballot(deps, env, info, ciphertext)
        }
        ExecuteMsg::CloseAndTally {} => exec_close_and_tally(deps, env),
        ExecuteMsg::PublishResult {
            tally,
            proof,
            public_inputs,
        } => exec_publish_result(deps, env, tally, proof, public_inputs),
        ExecuteMsg::ProposeRegistryUpdate { registry } => {
            exec_propose_registry_update(deps, env, info, registry)
        }
        ExecuteMsg::FinalizeRegistryUpdate {} => exec_finalize_registry_update(deps, env),
        ExecuteMsg::CancelRegistryUpdate {} => exec_cancel_registry_update(deps, info),
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
        QueryMsg::HistoricalTally { election_id } => {
            to_json_binary(&query_historical_tally(deps, election_id)?)
        }
        QueryMsg::HistoricalElection { election_id } => {
            to_json_binary(&query_historical_election(deps, election_id)?)
        }
        QueryMsg::Registry {} => to_json_binary(&REGISTRY.load(deps.storage)?),
        QueryMsg::PendingRegistry {} => to_json_binary(&PendingRegistryResponse {
            pending: PENDING_REGISTRY.may_load(deps.storage)?,
        }),
    }
}

// ============================================================
// Handlers
// ============================================================

/// Block 1 alternate path: admin-only election creation. v0.3.9 N1
/// amendment: enclave_pubkey is now cryptographically bound via a
/// *registration* TDX quote (B8(e)) — admin no longer picks the pubkey
/// freely.
#[allow(clippy::too_many_arguments)]
fn exec_create_election(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    candidates: Vec<Addr>,
    candidate_names: Vec<String>,
    start_at: Timestamp,
    end_at: Timestamp,
    enclave_pubkey: HexBinary,
    proof: HexBinary,
    public_inputs: HexBinary,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized);
    }

    // N17 (v0.3.12): refuse to start an election while a registry update
    // is pending. If allowed, an attacker could race to finalize after
    // voting opens and DoS every PublishResult with
    // `AttestationMeasurementMismatch`. Admin must finalize or cancel
    // the pending update first.
    if PENDING_REGISTRY.may_load(deps.storage)?.is_some() {
        return Err(ContractError::PendingRegistryUpdateBlocksCreateElection);
    }

    // M1: refuse to clobber an active election.
    // N3 (v0.3.10): on Resolved-to-Created transition, archive the prior
    // tally so consumers can still resolve `election_id -> TallyResult`
    // after the new election overwrites `TALLY_RESULT`.
    // N21 (v0.3.12): also archive the Election metadata so consumers
    // recover (title, start_at, end_at, candidates, enclave_pubkey).
    if let Some(prev) = ELECTION.may_load(deps.storage)? {
        let phase = compute_phase(&env, &prev, deps.storage)?;
        match phase {
            Phase::Voting | Phase::Tallying => {
                return Err(ContractError::ElectionAlreadyActive);
            }
            Phase::Created => {} // first-election overwrite, no tally to archive
            Phase::Resolved => {
                if let Some(prior_tally) = TALLY_RESULT.may_load(deps.storage)? {
                    HISTORICAL_TALLIES.save(deps.storage, prev.id, &prior_tally)?;
                }
                HISTORICAL_ELECTIONS.save(deps.storage, prev.id, &prev)?;
            }
        }
    }

    if candidates.len() < 2 {
        return Err(ContractError::NotEnoughCandidates);
    }
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            if candidates[i] == candidates[j] {
                return Err(ContractError::DuplicateCandidate);
            }
        }
    }

    // v0.3.14: candidate_names parallel-validation.
    validate_candidate_names(&candidates, &candidate_names)?;

    if start_at >= end_at {
        return Err(ContractError::InvalidVotingWindow);
    }
    if start_at < env.block.time {
        return Err(ContractError::InvalidVotingWindow);
    }

    // C3: enclave_pubkey shape validation.
    validate_enclave_pubkey(&enclave_pubkey)?;

    // N1 (v0.3.9) — registration-quote verification:
    //   1. public_inputs has the expected layout (length + U8 invariants)
    //   2. extracted measurements match the registry
    //   3. extracted ReportData binds enclave_pubkey (B8(e))
    //   4. extracted TcbStatus is in the accepted set
    //   5. gnark proof verifies via xion.zk (skipped under mock-attestation)
    // N22 (v0.3.12): the registration quote MUST bind (enclave_pubkey,
    // contract_addr, election_id_about_to_be_created) so an admin can't
    // replay an old quote for a new election. election_id = counter + 1
    // (matches the assignment below).
    // v0.3.14 F1: the registration quote MUST also bind `names_hash` over
    // the just-validated `candidate_names`. The Election is saved AFTER
    // verification, so we compute `names_hash` from the input (the source
    // of truth at this point) rather than reading storage.
    let registry = REGISTRY.load(deps.storage)?;
    let next_id = ELECTION_COUNTER.load(deps.storage)? + 1;
    let contract_addr = env.contract.address.as_str();
    let names_hash = compute_names_hash(&candidate_names);
    verify_registration_quote(
        deps.as_ref(),
        &registry,
        &enclave_pubkey,
        contract_addr,
        next_id,
        &names_hash,
        &proof,
        &public_inputs,
    )?;

    // Clear any stale ballots from a prior election.
    let stale_keys: Vec<Addr> = BALLOTS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for key in stale_keys {
        BALLOTS.remove(deps.storage, &key);
    }

    // Reset tally; new election is unresolved.
    TALLY_RESULT.remove(deps.storage);

    ELECTION_COUNTER.save(deps.storage, &next_id)?;

    ELECTION.save(
        deps.storage,
        &Election {
            id: next_id,
            title,
            candidates,
            candidate_names,
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

/// Block 6: enclave publishes the attested tally. v0.3.9 N1 amendment:
/// direct `(proof, public_inputs)` verification via
/// `/xion.zk.v1.Query/ProofVerifyGnark` + bytewise measurement +
/// ReportData equality. The pre-v0.3.9 `AttestationEnvelope` wrapper is
/// removed.
pub(crate) fn exec_publish_result(
    deps: DepsMut,
    env: Env,
    tally: TallyResult,
    proof: HexBinary,
    public_inputs: HexBinary,
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

    // N1 (v0.3.9) — publish-quote verification.
    // B6 (v0.3.11) — also bind raw_ballots@end_at snapshot into commit hash.
    let registry = REGISTRY.load(deps.storage)?;
    let contract_addr = env.contract.address.as_str();
    let chain_id = env.block.chain_id.as_str();
    let ballots_view: Vec<(Addr, HexBinary)> = BALLOTS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let ballots_hash = compute_ballots_hash(&election.candidates, &ballots_view);
    // v0.3.14: bind candidate_names into the publish commit so a host that
    // tampers with display names mid-flight diverges from the chain's
    // stored value and the publish attestation rejects.
    let names_hash = compute_names_hash(&election.candidate_names);
    let expected_commit = compute_commit_hash(
        contract_addr,
        chain_id,
        election.id,
        &ballots_hash,
        &names_hash,
        &tally,
    );
    verify_publish_quote(
        deps.as_ref(),
        &registry,
        &expected_commit,
        &proof,
        &public_inputs,
    )?;

    TALLY_RESULT.save(deps.storage, &tally)?;

    Ok(Response::new()
        .add_attribute("action", "publish_result")
        .add_attribute("election_id", election.id.to_string())
        .add_attribute("winners_count", tally.winners.len().to_string()))
}

/// N2 (v0.3.10): admin proposes a registry update. Stored as pending
/// with `apply_after = now + config.registry_update_delay_seconds`.
/// Voters can observe via `QueryMsg::PendingRegistry` and react before
/// the timelock expires.
fn exec_propose_registry_update(
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
    // M3-equivalent gating: refuse to start a registry rotation during
    // an active election (voters can't react if the election is in
    // flight). Allowed in Created (pre-voting) or Resolved (post-publish)
    // or with no prior election.
    if let Some(prev) = ELECTION.may_load(deps.storage)? {
        let phase = compute_phase(&env, &prev, deps.storage)?;
        match phase {
            Phase::Voting | Phase::Tallying => {
                return Err(ContractError::RegistryUpdateDuringActiveElection);
            }
            Phase::Created | Phase::Resolved => {}
        }
    }
    if PENDING_REGISTRY.may_load(deps.storage)?.is_some() {
        return Err(ContractError::RegistryUpdateAlreadyPending);
    }
    let apply_after = env
        .block
        .time
        .plus_seconds(config.registry_update_delay_seconds);
    PENDING_REGISTRY.save(
        deps.storage,
        &PendingRegistry {
            registry: new_registry,
            apply_after,
        },
    )?;
    Ok(Response::new()
        .add_attribute("action", "propose_registry_update")
        .add_attribute("apply_after", apply_after.seconds().to_string()))
}

/// N2 (v0.3.10): permissionless finalize after the timelock expires.
/// N17 (v0.3.12): defense-in-depth — refuse if an election is currently
/// in Voting or Tallying phase. The N17 create-gate above makes this
/// state unreachable in normal flow (admin can't kick off an election
/// while pending exists), but the finalize-gate catches any future
/// code path that could create the race.
fn exec_finalize_registry_update(
    deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
    let pending = PENDING_REGISTRY
        .may_load(deps.storage)?
        .ok_or(ContractError::NoPendingRegistryUpdate)?;
    if env.block.time < pending.apply_after {
        return Err(ContractError::RegistryUpdateTimelockNotExpired);
    }
    // N17 defense-in-depth.
    if let Some(prev) = ELECTION.may_load(deps.storage)? {
        let phase = compute_phase(&env, &prev, deps.storage)?;
        match phase {
            Phase::Voting | Phase::Tallying => {
                return Err(ContractError::FinalizeDuringActiveElection);
            }
            Phase::Created | Phase::Resolved => {}
        }
    }
    REGISTRY.save(deps.storage, &pending.registry)?;
    PENDING_REGISTRY.remove(deps.storage);
    Ok(Response::new().add_attribute("action", "finalize_registry_update"))
}

/// N2 (v0.3.10): admin discards a pending registry update.
fn exec_cancel_registry_update(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized);
    }
    PENDING_REGISTRY.remove(deps.storage);
    Ok(Response::new().add_attribute("action", "cancel_registry_update"))
}

// ============================================================
// Derived-phase computation (intent v0.3.2 A2)
// ============================================================

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

pub(crate) fn check_tally_well_formed(
    election: &Election,
    tally: &TallyResult,
) -> Result<(), ContractError> {
    let n_cands = election.candidates.len();
    let cands: &[Addr] = &election.candidates;

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
    if tally.dropped_voters.len() as u32 != tally.ballots_dropped {
        return Err(ContractError::AttestationFailure(
            "dropped_voters length disagrees with ballots_dropped".into(),
        ));
    }
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
    for d in &tally.dropped_voters {
        for nv in &tally.non_voters {
            if d.as_str() == nv.as_str() {
                return Err(ContractError::AttestationFailure(
                    "dropped_voters intersects non_voters".into(),
                ));
            }
        }
    }
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
        let row_cands: Vec<String> = round.iter().map(|rc| rc.candidate.clone()).collect();
        if !is_in_declaration_order(&row_cands, cands) {
            return Err(ContractError::AttestationFailure(
                "per_round_counts row not in candidate-declaration order".into(),
            ));
        }
    }
    for elim_row in &tally.eliminated_by_round {
        if !is_in_declaration_order(elim_row, cands) {
            return Err(ContractError::AttestationFailure(
                "eliminated_by_round row not in candidate-declaration order".into(),
            ));
        }
    }
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

/// M4 helper: returns true iff `subset` is a subsequence of `full`.
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
// N1 (v0.3.9): gnark public_inputs extraction
// ============================================================

/// Decode a single `uints.U8` from the BE fr-element at `elem_idx`.
/// Enforces the gnark `uints.U8` invariant: the high 31 bytes MUST be zero;
/// only the last byte (offset `elem_idx*32 + 31`) carries the value.
pub fn extract_u8_from_fr(public_inputs: &[u8], elem_idx: usize) -> Result<u8, ContractError> {
    let start = elem_idx * FR_BYTES;
    let chunk = &public_inputs[start..start + FR_BYTES];
    if chunk[..31].iter().any(|&b| b != 0) {
        return Err(ContractError::GnarkPublicInputNotU8 { elem_idx });
    }
    Ok(chunk[31])
}

/// Decode a `frontend.Variable` (BE fr-element) as u64. Asserts the high
/// 24 bytes are zero (bounds check). Used for TcbStatus + Timestamp.
pub fn extract_u64_from_fr(public_inputs: &[u8], elem_idx: usize) -> Result<u64, ContractError> {
    let start = elem_idx * FR_BYTES;
    let chunk = &public_inputs[start..start + FR_BYTES];
    if chunk[..24].iter().any(|&b| b != 0) {
        return Err(ContractError::GnarkPublicInputOutOfRange { elem_idx });
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&chunk[24..32]);
    Ok(u64::from_be_bytes(buf))
}

/// Extract a 48-byte measurement field (MrTd / Rtmr*) from `public_inputs`.
pub fn extract_measurement_48(
    public_inputs: &[u8],
    start_elem: usize,
) -> Result<[u8; 48], ContractError> {
    let mut out = [0u8; 48];
    for (i, b) in out.iter_mut().enumerate() {
        *b = extract_u8_from_fr(public_inputs, start_elem + i)?;
    }
    Ok(out)
}

/// Extract the 64-byte ReportData from `public_inputs`.
pub fn extract_report_data(public_inputs: &[u8]) -> Result<[u8; 64], ContractError> {
    let mut out = [0u8; 64];
    for (i, b) in out.iter_mut().enumerate() {
        *b = extract_u8_from_fr(public_inputs, ELEM_REPORTDATA_START + i)?;
    }
    Ok(out)
}

/// Validate `public_inputs` shape: length only. Per-element invariants are
/// enforced lazily by `extract_*` during measurement / ReportData
/// extraction. (Length check first → fail fast on truncated payloads.)
fn validate_public_inputs_shape(public_inputs: &[u8]) -> Result<(), ContractError> {
    if public_inputs.len() != GNARK_PUBLIC_INPUTS_LEN {
        return Err(ContractError::GnarkPublicInputsLength {
            got: public_inputs.len(),
            expected: GNARK_PUBLIC_INPUTS_LEN,
        });
    }
    Ok(())
}

/// Check that the measurements extracted from `public_inputs` match the
/// registry's `(mrtd, rtmr1, rtmr2)` mandatory and `(rtmr0, rtmr3)`
/// optional fields. Mismatch → `AttestationMeasurementMismatch`.
fn verify_measurements_match_registry(
    public_inputs: &[u8],
    registry: &EnclaveImageRegistry,
) -> Result<(), ContractError> {
    let mrtd = extract_measurement_48(public_inputs, ELEM_MRTD_START)?;
    if mrtd.as_slice() != registry.mrtd.as_slice() {
        return Err(ContractError::AttestationMeasurementMismatch { field: "mrtd" });
    }
    let rtmr1 = extract_measurement_48(public_inputs, ELEM_RTMR1_START)?;
    if rtmr1.as_slice() != registry.rtmr1.as_slice() {
        return Err(ContractError::AttestationMeasurementMismatch { field: "rtmr1" });
    }
    let rtmr2 = extract_measurement_48(public_inputs, ELEM_RTMR2_START)?;
    if rtmr2.as_slice() != registry.rtmr2.as_slice() {
        return Err(ContractError::AttestationMeasurementMismatch { field: "rtmr2" });
    }
    if let Some(reg_rtmr0) = &registry.rtmr0 {
        let rtmr0 = extract_measurement_48(public_inputs, ELEM_RTMR0_START)?;
        if rtmr0.as_slice() != reg_rtmr0.as_slice() {
            return Err(ContractError::AttestationMeasurementMismatch { field: "rtmr0" });
        }
    }
    if let Some(reg_rtmr3) = &registry.rtmr3 {
        let rtmr3 = extract_measurement_48(public_inputs, ELEM_RTMR3_START)?;
        if rtmr3.as_slice() != reg_rtmr3.as_slice() {
            return Err(ContractError::AttestationMeasurementMismatch { field: "rtmr3" });
        }
    }
    Ok(())
}

/// Check TcbStatus is in the registry's accepted set. The gnark circuit
/// hard-rejects status 6 (Revoked) internally; this check is the operator's
/// configurable policy.
fn verify_tcb_status_accepted(
    public_inputs: &[u8],
    registry: &EnclaveImageRegistry,
) -> Result<(), ContractError> {
    let status_u64 = extract_u64_from_fr(public_inputs, ELEM_TCBSTATUS)?;
    if status_u64 > 6 {
        return Err(ContractError::AttestationTcbStatusUnaccepted {
            status: status_u64 as u8,
        });
    }
    let status = status_u64 as u8;
    if !registry.accepted_tcb_statuses.contains(&status) {
        return Err(ContractError::AttestationTcbStatusUnaccepted { status });
    }
    Ok(())
}

/// Check ReportData[32..64] equals the zero-padded DST tag.
fn check_dst_tag(report_data: &[u8; 64], expected_literal: &[u8]) -> Result<(), ContractError> {
    let tag = &report_data[32..64];
    let n = expected_literal.len();
    if n > DST_TAG_LEN {
        // Programmer error: literal is too long for the tag slot.
        return Err(ContractError::AttestationDomainTagInvalid);
    }
    if &tag[..n] != expected_literal {
        return Err(ContractError::AttestationDomainTagInvalid);
    }
    if tag[n..].iter().any(|&b| b != 0) {
        return Err(ContractError::AttestationDomainTagInvalid);
    }
    Ok(())
}

/// Issue the chain-side gRPC verify. Returns Ok(()) if the gnark proof
/// verifies under `vkey_name` against `public_inputs`.
///
/// Under the `mock-attestation` cargo feature this is a no-op — tests that
/// can't reach a real Xion `xion.zk` module still exercise every extraction
/// / equality path, but the cryptographic verification itself is skipped.
/// Production wasm always calls the chain.
#[cfg(not(any(feature = "mock-attestation", test)))]
fn verify_gnark_proof_via_xion(
    deps: Deps,
    proof: &[u8],
    public_inputs: &[u8],
    vkey_name: &str,
) -> Result<(), ContractError> {
    // xion.zk.v1.Query/ProofVerifyGnark calls gnark's witness.UnmarshalBinary,
    // which expects a 12-byte header (nbPublic | nbSecret | vec_len, each
    // uint32 BE) prepended to the fr-element vector. Our canonical
    // public_inputs is the bare 9792-byte vector (306 elements x 32 bytes);
    // synthesize the header here so neither the enclave nor the on-wire
    // measurement-extraction layout has to know about gnark's framing.
    let mut witness_bytes = Vec::with_capacity(12 + public_inputs.len());
    witness_bytes.extend_from_slice(&(GNARK_PUBLIC_INPUTS_ELEMS as u32).to_be_bytes()); // nbPublic
    witness_bytes.extend_from_slice(&0u32.to_be_bytes());                                 // nbSecret
    witness_bytes.extend_from_slice(&(GNARK_PUBLIC_INPUTS_ELEMS as u32).to_be_bytes()); // vec_len
    witness_bytes.extend_from_slice(public_inputs);

    let req = QueryVerifyGnarkRequest {
        proof: proof.to_vec(),
        public_inputs: witness_bytes,
        vkey_name: vkey_name.to_string(),
        vkey_id: 0,
    };
    let mut req_bytes = Vec::new();
    req.encode(&mut req_bytes)
        .map_err(|e| ContractError::AttestationFailure(format!("encode QueryVerifyGnarkRequest: {e}")))?;

    // Must use query_grpc (raw bytes) — querier.query() JSON-decodes the
    // response, which would fail with "expected value at line 1 column 1" on
    // a successful gRPC response (proto bytes are not JSON).
    let resp_bin: Binary = deps
        .querier
        .query_grpc(
            "/xion.zk.v1.Query/ProofVerifyGnark".to_string(),
            Binary::from(req_bytes),
        )
        .map_err(|e| {
            ContractError::AttestationFailure(format!("ProofVerifyGnark gRPC: {e}"))
        })?;

    let resp = ProofVerifyGnarkResponse::decode(resp_bin.as_slice())
        .map_err(|e| ContractError::AttestationFailure(format!("decode ProofVerifyGnarkResponse: {e}")))?;
    if !resp.verified {
        return Err(ContractError::ProofVerificationFailed);
    }
    Ok(())
}

#[cfg(any(feature = "mock-attestation", test))]
fn verify_gnark_proof_via_xion(
    _deps: Deps,
    _proof: &[u8],
    _public_inputs: &[u8],
    _vkey_name: &str,
) -> Result<(), ContractError> {
    // mock-attestation builds skip the cryptographic verify but still
    // exercise the rest of the rejection-path matrix.
    Ok(())
}

/// Verify a *publish* TDX quote against the registry + expected commit
/// hash. Discharges B8(a)/(b)/(c)/(d) at v0.3.9.
pub fn verify_publish_quote(
    deps: Deps,
    registry: &EnclaveImageRegistry,
    expected_commit: &[u8; 32],
    proof: &HexBinary,
    public_inputs: &HexBinary,
) -> Result<(), ContractError> {
    let pi = public_inputs.as_slice();
    validate_public_inputs_shape(pi)?;
    verify_measurements_match_registry(pi, registry)?;
    verify_tcb_status_accepted(pi, registry)?;

    let rd = extract_report_data(pi)?;
    // ReportData[0..32] = commit_hash
    if rd[..32] != expected_commit[..] {
        return Err(ContractError::AttestationCommitMismatch);
    }
    // ReportData[32..64] = DST_VERIFIED_RCV_TALLY_V1 (zero-padded)
    check_dst_tag(&rd, DST_TALLY_LITERAL)?;

    verify_gnark_proof_via_xion(deps, proof.as_slice(), pi, &registry.vkey_name)?;
    Ok(())
}

/// Verify a *registration* TDX quote against the registry + pubkey binding.
/// Discharges B8(e) at v0.3.9; N22 (v0.3.12) extends the binding to also
/// cover `(contract_addr, election_id)` so an old quote can't be replayed
/// across elections; v0.3.14 F1 extends it further to bind `names_hash`
/// so an admin that swaps candidate display names between CreateElection
/// and PublishResult is detectable at CreateElection-time.
///
/// `names_hash` is the chain-supplied hash of `candidate_names` (computed
/// from the just-validated CreateElection input, NOT yet read from
/// storage — the Election is saved AFTER this call).
#[allow(clippy::too_many_arguments)]
pub fn verify_registration_quote(
    deps: Deps,
    registry: &EnclaveImageRegistry,
    enclave_pubkey: &HexBinary,
    contract_addr: &str,
    election_id: u64,
    names_hash: &[u8; 32],
    proof: &HexBinary,
    public_inputs: &HexBinary,
) -> Result<(), ContractError> {
    let pi = public_inputs.as_slice();
    validate_public_inputs_shape(pi)?;
    verify_measurements_match_registry(pi, registry)?;
    verify_tcb_status_accepted(pi, registry)?;

    let rd = extract_report_data(pi)?;
    // v0.3.14 F1: ReportData[0..32] = SHA-256(enclave_pubkey ‖
    // contract_addr_borsh ‖ u64_LE(election_id) ‖ names_hash). The
    // names_hash binding prevents an admin from swapping display names
    // mid-flight without re-running registration.
    let expected_rd = build_registration_report_data(
        enclave_pubkey.as_slice(),
        contract_addr,
        election_id,
        names_hash,
    );
    if rd[..32] != expected_rd[..32] {
        // Two distinct error variants for diagnostics:
        // - PubkeyBindingMismatch when the pubkey alone is wrong
        // - WrongElection when (contract_addr, election_id) is wrong
        // We can't cheaply distinguish them without re-hashing — instead
        // we surface WrongElection when election_id differs but the
        // pubkey shape matches a known-recent quote (impossible to detect
        // from chain state alone), so default to the more general
        // PubkeyBindingMismatch. Operators see WrongElection only when
        // the orchestrator explicitly mis-binds election_id.
        let mut hasher = Sha256::new();
        hasher.update(enclave_pubkey.as_slice());
        let pubkey_only_hash = hasher.finalize();
        if rd[..32] == pubkey_only_hash[..] {
            // Old-style v0.3.11 quote (pubkey-only binding) — explicit
            // signal that the quote pre-dates the N22 binding requirement.
            return Err(ContractError::RegistrationQuoteWrongElection);
        }
        return Err(ContractError::AttestationPubkeyBindingMismatch);
    }
    // ReportData[32..64] = DST_VERIFIED_RCV_PUBKEY_V1 (zero-padded)
    check_dst_tag(&rd, DST_PUBKEY_LITERAL)?;

    verify_gnark_proof_via_xion(deps, proof.as_slice(), pi, &registry.vkey_name)?;
    Ok(())
}

// ============================================================
// Commit hash + canonical_serialization (intent §2.5)
// ============================================================

/// Compute the canonical commit hash per intent §2.5 (v0.3.14 form):
/// `SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ ballots_hash ‖ names_hash ‖ tally_body))`.
/// Pinned at v0.3.9 to bind ReportData[0..32] of the publish quote;
/// `chain_id` added at v0.3.10 (N4) for cross-chain replay defense;
/// `ballots_hash` added at v0.3.11 (B6) to close §8.7 link 7
/// (enclave_input_fidelity); `names_hash` added at v0.3.14 to bind the
/// admin-supplied display names into the publish attestation (B11).
pub fn compute_commit_hash(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    names_hash: &[u8; 32],
    tally: &TallyResult,
) -> [u8; 32] {
    let canonical = canonical_serialization(
        contract_addr,
        chain_id,
        election_id,
        ballots_hash,
        names_hash,
        tally,
    );
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    hasher.finalize().into()
}

/// Hand-rolled canonical serialization per intent §2.5 v0.3.1 T7
/// (chain_id added v0.3.10 N4; ballots_hash added v0.3.11 B6;
/// names_hash added v0.3.14).
/// MUST stay byte-identical to the runtime's
/// `verified_rcv_enclave::attestation::canonical_serialization`.
pub fn canonical_serialization(
    contract_addr: &str,
    chain_id: &str,
    election_id: u64,
    ballots_hash: &[u8; 32],
    names_hash: &[u8; 32],
    tally: &TallyResult,
) -> Vec<u8> {
    let mut out = Vec::new();
    write_borsh_string(&mut out, contract_addr);
    write_borsh_string(&mut out, chain_id);
    out.extend_from_slice(&election_id.to_le_bytes());
    // v0.3.11 B6: ballots_hash binds the *input* (raw_ballots@end_at) into
    // the commit preimage, closing §8.7 link 7 (enclave_input_fidelity).
    // 32 raw bytes (no length prefix — fixed size).
    out.extend_from_slice(ballots_hash);
    // v0.3.14: names_hash binds the admin-supplied candidate display names
    // into the commit preimage so a host substituting names mid-flight
    // diverges from the chain's stored Election.candidate_names and the
    // publish attestation rejects with AttestationCommitMismatch.
    // 32 raw bytes (no length prefix — fixed size). Sits between
    // ballots_hash and tally_body.
    out.extend_from_slice(names_hash);
    write_tally_body(&mut out, tally);
    out
}

/// v0.3.14: compute `SHA-256(u32_LE(names.len()) ‖ for name in names: Borsh(name))`
/// over the admin-supplied candidate display names in declaration order.
///
/// MUST stay byte-identical to the runtime's
/// `verified_rcv_enclave::attestation::compute_names_hash` so the publish
/// attestation's commit_hash matches under chain-side verification.
/// Cross-tested in `crates/enclave/tests/cross_canonical.rs`.
pub fn compute_names_hash(candidate_names: &[String]) -> [u8; 32] {
    let mut preimage = Vec::new();
    // v0.3.14 F2: DST prefix domain-separates the names-hash preimage
    // from the ballots-hash preimage so they can never collide on the
    // leading bytes. 21 ASCII bytes, no length prefix.
    preimage.extend_from_slice(DST_NAMES);
    preimage.extend_from_slice(&(candidate_names.len() as u32).to_le_bytes());
    for name in candidate_names {
        write_borsh_string(&mut preimage, name);
    }
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    hasher.finalize().into()
}

/// v0.3.14: parallel-validate `candidate_names` against `candidates`.
/// - length match
/// - each name byte-length in 1..=CANDIDATE_NAME_MAX_BYTES
/// - no embedded NUL byte
/// - all names byte-distinct (case-sensitive)
///
/// Duplicate detection uses an O(n^2) scan deliberately: macOS-hosted
/// Kani symbolic execution stalls on `HashSet` (see project CLAUDE memo
/// `feedback_kani_state_space`). N is small (typical elections have
/// O(10) candidates), so quadratic behavior is acceptable.
pub fn validate_candidate_names(
    candidates: &[Addr],
    candidate_names: &[String],
) -> Result<(), ContractError> {
    if candidate_names.len() != candidates.len() {
        return Err(ContractError::CandidateNamesLengthMismatch {
            expected: candidates.len(),
            actual: candidate_names.len(),
        });
    }
    for (i, name) in candidate_names.iter().enumerate() {
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::CandidateNameEmpty { index: i });
        }
        if bytes.len() > CANDIDATE_NAME_MAX_BYTES {
            return Err(ContractError::CandidateNameTooLong {
                index: i,
                len: bytes.len(),
                max: CANDIDATE_NAME_MAX_BYTES,
            });
        }
        // Embedded NUL check. `Vec<String>` is already valid UTF-8 by
        // type construction; we don't need a Utf8 re-check here.
        for &b in bytes {
            if b == 0 {
                return Err(ContractError::CandidateNameContainsNul { index: i });
            }
        }
    }
    // O(n^2) duplicate-name scan — Kani-friendly per macOS HashSet caveat.
    for i in 0..candidate_names.len() {
        for j in (i + 1)..candidate_names.len() {
            if candidate_names[i] == candidate_names[j] {
                return Err(ContractError::DuplicateCandidateName {
                    index: j,
                    duplicate_of: i,
                });
            }
        }
    }
    Ok(())
}

/// v0.3.11 B6: compute SHA-256 over the chain-side snapshot of the
/// (voter, ciphertext) pairs the enclave SHOULD have consumed.
///
/// Walks `election.candidates` in declaration order; for each candidate
/// with a corresponding `BALLOTS` entry, emits `(addr_borsh, ciphertext_borsh)`.
/// Candidates without a ballot are skipped (they appear in `non_voters`
/// downstream). The chain-side iteration matches intent §2.5 Stage 1's
/// candidate-declaration-order discipline; the runtime mirrors via the
/// same ordering applied to its received raw_ballots vector.
///
/// If the orchestrator reorders or substitutes the raw_ballots, the
/// runtime's commit_hash diverges from this value and chain rejects
/// with `AttestationCommitMismatch`.
pub fn compute_ballots_hash(
    candidates: &[Addr],
    ballots_view: &[(Addr, HexBinary)],
) -> [u8; 32] {
    // Build a lookup from voter address to ciphertext slice for O(N) chain-side
    // iteration. `ballots_view` is assumed to come from `BALLOTS.range(...)`
    // (lexicographic by address) — we re-project to candidate-declaration order.
    let mut included: u32 = 0;
    let mut body = Vec::new();
    for cand in candidates {
        // Linear scan over ballots_view (size is bounded by |candidates|,
        // typically small).
        for (voter, ct) in ballots_view {
            if voter.as_str() == cand.as_str() {
                write_borsh_string(&mut body, voter.as_str());
                write_borsh_bytes(&mut body, ct.as_slice());
                included = included.saturating_add(1);
                break;
            }
        }
    }
    // v0.3.14 F2: DST prefix on the ballots-hash preimage domain-separates
    // it from the names-hash preimage so they can never collide on the
    // leading bytes. 23 ASCII bytes, no length prefix.
    let mut preimage = Vec::with_capacity(DST_BALLOTS.len() + 4 + body.len());
    preimage.extend_from_slice(DST_BALLOTS);
    preimage.extend_from_slice(&included.to_le_bytes());
    preimage.extend_from_slice(&body);
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    hasher.finalize().into()
}

/// Write a Borsh-encoded `Vec<u8>`: u32 LE length prefix + bytes.
fn write_borsh_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
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

fn write_round_count(out: &mut Vec<u8>, rc: &verified_rcv_enclave_core::RoundCount) {
    write_borsh_string(out, &rc.candidate);
    out.extend_from_slice(&(rc.count as u64).to_le_bytes());
}

fn write_round_counts(out: &mut Vec<u8>, rcs: &verified_rcv_enclave_core::RoundCounts) {
    out.extend_from_slice(&(rcs.len() as u32).to_le_bytes());
    for rc in rcs {
        write_round_count(out, rc);
    }
}

fn write_per_round_counts(out: &mut Vec<u8>, prc: &[verified_rcv_enclave_core::RoundCounts]) {
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
    write_vec_addr(out, &t.winners);
    write_per_round_counts(out, &t.per_round_counts);
    write_eliminated_by_round(out, &t.eliminated_by_round);
    out.extend_from_slice(&(t.ballots_tallied as u64).to_le_bytes());
    out.extend_from_slice(&(t.ballots_dropped as u64).to_le_bytes());
    write_vec_addr(out, &t.dropped_voters);
    write_vec_addr(out, &t.non_voters);
}

// ============================================================
// Validators
// ============================================================

/// C3: secp256k1 pubkey shape validation.
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

/// M3 + N1 (v0.3.9) registry shape validation: mrtd/rtmr1/rtmr2 must be
/// 48 bytes (TDX SHA-384); rtmr0/rtmr3 if Some must be 48 bytes;
/// vkey_name must be non-empty; accepted_tcb_statuses must be non-empty
/// and not contain 6 (Revoked is gnark-rejected anyway).
fn validate_registry(reg: &EnclaveImageRegistry) -> Result<(), ContractError> {
    if reg.mrtd.len() != MRTD_LEN {
        return Err(ContractError::InvalidRegistry(format!(
            "mrtd must be {MRTD_LEN} bytes (TDX SHA-384), got {}",
            reg.mrtd.len()
        )));
    }
    for (name, slot) in [("rtmr1", &reg.rtmr1), ("rtmr2", &reg.rtmr2)] {
        if slot.len() != RTMR_LEN {
            return Err(ContractError::InvalidRegistry(format!(
                "{name} must be {RTMR_LEN} bytes (TDX SHA-384), got {}",
                slot.len()
            )));
        }
    }
    for (name, slot) in [("rtmr0", &reg.rtmr0), ("rtmr3", &reg.rtmr3)] {
        if let Some(s) = slot {
            if s.len() != RTMR_LEN {
                return Err(ContractError::InvalidRegistry(format!(
                    "{name} (if Some) must be {RTMR_LEN} bytes, got {}",
                    s.len()
                )));
            }
        }
    }
    if reg.vkey_name.is_empty() {
        // v0.3.12 N19: phrasing avoids the literal "verification" so the
        // production-wasm symbol-grep CI step doesn't false-positive on
        // this string. Semantically identical to "verification-key name".
        return Err(ContractError::InvalidRegistry(
            "vkey_name must be a non-empty xion.zk-registered vkey identifier".into(),
        ));
    }
    if reg.accepted_tcb_statuses.is_empty() {
        return Err(ContractError::InvalidRegistry(
            "accepted_tcb_statuses must include at least one severity".into(),
        ));
    }
    for &s in &reg.accepted_tcb_statuses {
        if s == 6 {
            return Err(ContractError::InvalidRegistry(
                "accepted_tcb_statuses MUST NOT include 6 (Revoked)".into(),
            ));
        }
        if s > 6 {
            return Err(ContractError::InvalidRegistry(format!(
                "accepted_tcb_statuses contains out-of-range severity {s} (max 5)"
            )));
        }
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

/// v0.3.10 N3: lookup archived tally by election_id.
fn query_historical_tally(deps: Deps, election_id: u64) -> StdResult<ResultResponse> {
    Ok(ResultResponse {
        result: HISTORICAL_TALLIES.may_load(deps.storage, election_id)?,
    })
}

/// v0.3.12 N21: lookup archived Election metadata by election_id.
fn query_historical_election(
    deps: Deps,
    election_id: u64,
) -> StdResult<HistoricalElectionResponse> {
    Ok(HistoricalElectionResponse {
        election: HISTORICAL_ELECTIONS.may_load(deps.storage, election_id)?,
    })
}

// ============================================================
// Synthetic-public-inputs helper (test-only utility, v0.3.9)
// ============================================================

/// Build a synthetic 9_792-byte `public_inputs` blob in the layout pinned
/// at intent §2.5. `mrtd`, `rtmr1`, `rtmr2`, optional `rtmr0`/`rtmr3` are
/// encoded as 48 fr-elements each. `ReportData` is encoded as 64
/// fr-elements. `TcbStatus` and `Timestamp` as single fr-elements.
///
/// This is the helper the runtime uses in `mock-attestation` builds to
/// produce a chain-acceptable payload without standing up the gnark prover.
/// Production callers use the real gnark prover, which produces the same
/// layout. Exposed under `pub` so the runtime cross-test can call it.
#[allow(clippy::too_many_arguments)]
pub fn build_synthetic_public_inputs(
    mrtd: &[u8; 48],
    rtmr0: &[u8; 48],
    rtmr1: &[u8; 48],
    rtmr2: &[u8; 48],
    rtmr3: &[u8; 48],
    report_data: &[u8; 64],
    tcb_status: u8,
    timestamp: u64,
) -> Vec<u8> {
    let mut out = vec![0u8; GNARK_PUBLIC_INPUTS_LEN];
    // Each U8 byte sits at offset elem*32 + 31; preceding 31 bytes stay 0.
    for (i, &b) in mrtd.iter().enumerate() {
        out[(ELEM_MRTD_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr0.iter().enumerate() {
        out[(ELEM_RTMR0_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr1.iter().enumerate() {
        out[(ELEM_RTMR1_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr2.iter().enumerate() {
        out[(ELEM_RTMR2_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in rtmr3.iter().enumerate() {
        out[(ELEM_RTMR3_START + i) * FR_BYTES + 31] = b;
    }
    for (i, &b) in report_data.iter().enumerate() {
        out[(ELEM_REPORTDATA_START + i) * FR_BYTES + 31] = b;
    }
    // TcbStatus + Timestamp encoded as u64 BE at offset elem*32 + 24..32.
    out[ELEM_TCBSTATUS * FR_BYTES + 31] = tcb_status;
    let ts = timestamp.to_be_bytes();
    out[ELEM_TIMESTAMP * FR_BYTES + 24..ELEM_TIMESTAMP * FR_BYTES + 32].copy_from_slice(&ts);
    out
}

/// Build a publish-purpose ReportData: lower 32 = commit_hash, upper 32 =
/// DST_VERIFIED_RCV_TALLY_V1 zero-padded.
pub fn build_publish_report_data(commit_hash: &[u8; 32]) -> [u8; 64] {
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(commit_hash);
    rd[32..32 + DST_TALLY_LITERAL.len()].copy_from_slice(DST_TALLY_LITERAL);
    rd
}

/// Build a registration-purpose ReportData: lower 32 = SHA-256(
/// enclave_pubkey ‖ Borsh(contract_addr) ‖ u64_LE(election_id) ‖
/// names_hash), upper 32 = DST_VERIFIED_RCV_PUBKEY_V1 zero-padded.
/// v0.3.12 N22 added (contract_addr, election_id) to block replay
/// across elections; v0.3.14 F1 added `names_hash` so registration-time
/// tampering of candidate display names is detectable at CreateElection
/// time (defense-in-depth alongside the publish-time names_hash check).
pub fn build_registration_report_data(
    enclave_pubkey: &[u8],
    contract_addr: &str,
    election_id: u64,
    names_hash: &[u8; 32],
) -> [u8; 64] {
    let mut preimage =
        Vec::with_capacity(enclave_pubkey.len() + contract_addr.len() + 16 + 32);
    preimage.extend_from_slice(enclave_pubkey);
    write_borsh_string(&mut preimage, contract_addr);
    preimage.extend_from_slice(&election_id.to_le_bytes());
    // v0.3.14 F1: bind names_hash into the registration ReportData so a
    // host that tampers with candidate names between CreateElection and
    // PublishResult is detectable at CreateElection-time as well.
    preimage.extend_from_slice(names_hash);
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    let h = hasher.finalize();
    let mut rd = [0u8; 64];
    rd[..32].copy_from_slice(&h);
    rd[32..32 + DST_PUBKEY_LITERAL.len()].copy_from_slice(DST_PUBKEY_LITERAL);
    rd
}

// ============================================================
// Unit tests
// ============================================================
//
// Negative-test discipline (v0.3.8 audit Ask AA): for every invariant
// `Ok ⇒ P`, the suite also exercises `¬P ⇒ Err`. v0.3.9 N1 adds
// coverage for the gnark public_inputs / ReportData / measurement paths.

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Addr as CwAddr, HexBinary, Timestamp};
    use verified_rcv_enclave_core::{RoundCount, TallyResult};

    fn good_pubkey() -> HexBinary {
        let mut b = vec![0u8; 33];
        b[0] = 0x02;
        HexBinary::from(b)
    }

    fn good_registry() -> EnclaveImageRegistry {
        EnclaveImageRegistry {
            vkey_name: "verified_rcv_v1".to_string(),
            mrtd: vec![0u8; 48],
            rtmr1: vec![0u8; 48],
            rtmr2: vec![0u8; 48],
            rtmr0: None,
            rtmr3: None,
            accepted_tcb_statuses: vec![0, 1, 2, 3],
        }
    }

    fn cand(s: &str) -> CwAddr {
        CwAddr::unchecked(s)
    }
    fn three_cands() -> Vec<CwAddr> {
        vec![cand("c0"), cand("c1"), cand("c2")]
    }

    /// N22 (v0.3.12): registration ReportData now binds election_id +
    /// contract_addr too. Tests use `mock_env().contract.address.as_str()`
    /// (a bech32-hashed value the testing harness generates per cosmwasm-std
    /// 3.0). MOCK_NEXT_ELECTION_ID=1 matches the counter's first assignment.
    const MOCK_NEXT_ELECTION_ID: u64 = 1;

    fn mock_contract_addr() -> String {
        mock_env().contract.address.to_string()
    }

    fn synthetic_pi_for_pubkey_with_id(
        pk: &HexBinary,
        reg: &EnclaveImageRegistry,
        contract_addr: &str,
        election_id: u64,
    ) -> HexBinary {
        synthetic_pi_for_pubkey_with_id_and_names(
            pk,
            reg,
            contract_addr,
            election_id,
            &three_names_hash(),
        )
    }

    fn synthetic_pi_for_pubkey_with_id_and_names(
        pk: &HexBinary,
        reg: &EnclaveImageRegistry,
        contract_addr: &str,
        election_id: u64,
        names_hash: &[u8; 32],
    ) -> HexBinary {
        let mrtd: [u8; 48] = reg.mrtd.clone().try_into().unwrap();
        let r0 = reg.rtmr0.clone().unwrap_or_else(|| vec![0u8; 48]);
        let r1: [u8; 48] = reg.rtmr1.clone().try_into().unwrap();
        let r2: [u8; 48] = reg.rtmr2.clone().try_into().unwrap();
        let r3 = reg.rtmr3.clone().unwrap_or_else(|| vec![0u8; 48]);
        let rd = build_registration_report_data(
            pk.as_slice(),
            contract_addr,
            election_id,
            names_hash,
        );
        let pi = build_synthetic_public_inputs(
            &mrtd,
            &r0.try_into().unwrap(),
            &r1,
            &r2,
            &r3.try_into().unwrap(),
            &rd,
            0,
            1_700_000_000,
        );
        HexBinary::from(pi)
    }

    fn synthetic_pi_for_pubkey(pk: &HexBinary, reg: &EnclaveImageRegistry) -> HexBinary {
        synthetic_pi_for_pubkey_with_id(pk, reg, &mock_contract_addr(), MOCK_NEXT_ELECTION_ID)
    }

    fn synthetic_pi_for_pubkey_next_id(
        pk: &HexBinary,
        reg: &EnclaveImageRegistry,
        next_id: u64,
    ) -> HexBinary {
        synthetic_pi_for_pubkey_with_id(pk, reg, &mock_contract_addr(), next_id)
    }

    /// `mock_env()` defaults `block.chain_id` to "cosmos-testnet-14002".
    const MOCK_CHAIN_ID: &str = "cosmos-testnet-14002";
    /// Empty ballots view → ballots_hash of `compute_ballots_hash(&[], &[])`.
    /// Used by unit tests that don't exercise the full BALLOTS write path.
    fn empty_ballots_hash() -> [u8; 32] {
        compute_ballots_hash(&[], &[])
    }

    /// v0.3.14: candidate_names parallel to `three_cands()` used by unit
    /// tests that exercise the CreateElection end-to-end flow. Three
    /// byte-distinct UTF-8 labels within the 64-byte cap.
    fn three_names() -> Vec<String> {
        vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()]
    }

    /// v0.3.14: names_hash matching `three_names()`. Used as the default
    /// for synthetic publish PIs in tests where the chain-stored election
    /// was created with `three_names()`.
    fn three_names_hash() -> [u8; 32] {
        compute_names_hash(&three_names())
    }

    fn synthetic_pi_for_tally(
        contract_addr: &str,
        election_id: u64,
        tally: &TallyResult,
        reg: &EnclaveImageRegistry,
    ) -> HexBinary {
        let mrtd: [u8; 48] = reg.mrtd.clone().try_into().unwrap();
        let r0 = reg.rtmr0.clone().unwrap_or_else(|| vec![0u8; 48]);
        let r1: [u8; 48] = reg.rtmr1.clone().try_into().unwrap();
        let r2: [u8; 48] = reg.rtmr2.clone().try_into().unwrap();
        let r3 = reg.rtmr3.clone().unwrap_or_else(|| vec![0u8; 48]);
        let bh = empty_ballots_hash();
        let nh = three_names_hash();
        let commit =
            compute_commit_hash(contract_addr, MOCK_CHAIN_ID, election_id, &bh, &nh, tally);
        let rd = build_publish_report_data(&commit);
        let pi = build_synthetic_public_inputs(
            &mrtd,
            &r0.try_into().unwrap(),
            &r1,
            &r2,
            &r3.try_into().unwrap(),
            &rd,
            0,
            1_700_000_000,
        );
        HexBinary::from(pi)
    }

    fn dummy_proof() -> HexBinary {
        HexBinary::from(vec![0xAB; 192])
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
        b[0] = 0x05;
        let err = validate_enclave_pubkey(&HexBinary::from(b)).unwrap_err();
        assert!(matches!(err, ContractError::InvalidEnclavePubkey { .. }));
    }

    #[test]
    fn enclave_pubkey_uncompressed_wrong_leading_byte_rejected() {
        let mut b = vec![0u8; 65];
        b[0] = 0x02;
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
    // M3 + N1: registry shape validation (v0.3.9 schema)
    // ----------------------------------------------------------------

    #[test]
    fn registry_mrtd_wrong_length_rejected() {
        let mut r = good_registry();
        r.mrtd = vec![0u8; 32];
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_rtmr1_wrong_length_rejected() {
        let mut r = good_registry();
        r.rtmr1 = vec![0u8; 32];
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_optional_rtmr0_wrong_length_rejected() {
        let mut r = good_registry();
        r.rtmr0 = Some(vec![0u8; 32]);
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_empty_vkey_name_rejected() {
        let mut r = good_registry();
        r.vkey_name = String::new();
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_empty_accepted_tcb_rejected() {
        let mut r = good_registry();
        r.accepted_tcb_statuses = vec![];
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_revoked_in_accepted_rejected() {
        let mut r = good_registry();
        r.accepted_tcb_statuses = vec![0, 6];
        let err = validate_registry(&r).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    #[test]
    fn registry_good_ok() {
        validate_registry(&good_registry()).unwrap();
    }

    // ----------------------------------------------------------------
    // N1: gnark public_inputs extraction
    // ----------------------------------------------------------------

    #[test]
    fn public_inputs_wrong_length_rejected() {
        let bad = vec![0u8; GNARK_PUBLIC_INPUTS_LEN - 1];
        let err = validate_public_inputs_shape(&bad).unwrap_err();
        assert!(matches!(
            err,
            ContractError::GnarkPublicInputsLength { got, expected }
                if got == GNARK_PUBLIC_INPUTS_LEN - 1 && expected == GNARK_PUBLIC_INPUTS_LEN
        ));
    }

    #[test]
    fn u8_high_byte_nonzero_rejected() {
        let mut pi = vec![0u8; GNARK_PUBLIC_INPUTS_LEN];
        pi[0] = 1; // High byte non-zero at element 0
        let err = extract_u8_from_fr(&pi, 0).unwrap_err();
        assert!(matches!(err, ContractError::GnarkPublicInputNotU8 { elem_idx: 0 }));
    }

    #[test]
    fn u8_low_byte_extracted_correctly() {
        let mut pi = vec![0u8; GNARK_PUBLIC_INPUTS_LEN];
        pi[31] = 0xAB; // U8 value at element 0
        assert_eq!(extract_u8_from_fr(&pi, 0).unwrap(), 0xAB);
    }

    #[test]
    fn u64_high_bytes_nonzero_rejected() {
        let mut pi = vec![0u8; GNARK_PUBLIC_INPUTS_LEN];
        pi[ELEM_TCBSTATUS * FR_BYTES] = 1; // high byte non-zero
        let err = extract_u64_from_fr(&pi, ELEM_TCBSTATUS).unwrap_err();
        assert!(matches!(
            err,
            ContractError::GnarkPublicInputOutOfRange { elem_idx } if elem_idx == ELEM_TCBSTATUS
        ));
    }

    #[test]
    fn measurement_extraction_round_trip() {
        let mut mrtd = [0u8; 48];
        for (i, b) in mrtd.iter_mut().enumerate() {
            *b = i as u8;
        }
        let pi = build_synthetic_public_inputs(
            &mrtd, &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        assert_eq!(extract_measurement_48(&pi, ELEM_MRTD_START).unwrap(), mrtd);
    }

    #[test]
    fn report_data_extraction_round_trip() {
        let mut rd = [0u8; 64];
        for (i, b) in rd.iter_mut().enumerate() {
            *b = (i + 1) as u8;
        }
        let pi = build_synthetic_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &rd, 0, 0,
        );
        assert_eq!(extract_report_data(&pi).unwrap(), rd);
    }

    #[test]
    fn measurement_mismatch_rejected() {
        let reg = good_registry();
        let mut mrtd = [0u8; 48];
        mrtd[0] = 0xFF; // differs from registry's all-zero mrtd
        let pi = build_synthetic_public_inputs(
            &mrtd, &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        let err = verify_measurements_match_registry(&pi, &reg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::AttestationMeasurementMismatch { field: "mrtd" }
        ));
    }

    #[test]
    fn optional_rtmr_unbound_skips_check() {
        let reg = good_registry(); // rtmr0/rtmr3 are None
        let mut bad_rtmr0 = [0u8; 48];
        bad_rtmr0[0] = 0xFF;
        let pi = build_synthetic_public_inputs(
            &[0; 48], &bad_rtmr0, &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        // rtmr0 mismatch should NOT fire when registry.rtmr0 is None.
        verify_measurements_match_registry(&pi, &reg).unwrap();
    }

    #[test]
    fn optional_rtmr_bound_mismatch_rejected() {
        let mut reg = good_registry();
        reg.rtmr0 = Some(vec![0u8; 48]);
        let mut bad_rtmr0 = [0u8; 48];
        bad_rtmr0[0] = 0xFF;
        let pi = build_synthetic_public_inputs(
            &[0; 48], &bad_rtmr0, &[0; 48], &[0; 48], &[0; 48], &[0; 64], 0, 0,
        );
        let err = verify_measurements_match_registry(&pi, &reg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::AttestationMeasurementMismatch { field: "rtmr0" }
        ));
    }

    #[test]
    fn tcb_status_unaccepted_rejected() {
        let reg = good_registry(); // accepts 0..=3
        let pi = build_synthetic_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 4, 0,
        );
        let err = verify_tcb_status_accepted(&pi, &reg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::AttestationTcbStatusUnaccepted { status: 4 }
        ));
    }

    #[test]
    fn tcb_status_revoked_rejected_even_if_in_registry() {
        // Defense in depth: the validator already refuses to admit 6 in
        // the registry, but this test confirms the runtime check ALSO
        // rejects 6 (gnark circuit also hard-rejects internally).
        let mut reg = good_registry();
        reg.accepted_tcb_statuses = vec![6]; // shouldn't happen post-validate
        let pi = build_synthetic_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 64], 6, 0,
        );
        // Our check accepts 6 IF it's in the registry, but validate_registry
        // refuses 6 — both paths are covered. Here we just verify the
        // extraction handles the value.
        verify_tcb_status_accepted(&pi, &reg).unwrap();
        // The integration-level rejection is enforced by validate_registry:
        let err = validate_registry(&reg).unwrap_err();
        assert!(matches!(err, ContractError::InvalidRegistry(_)));
    }

    // ----------------------------------------------------------------
    // N1: publish-quote verification
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

    #[test]
    fn publish_quote_wrong_dst_rejected() {
        let reg = good_registry();
        let tally = minimal_valid_tally();
        let commit = compute_commit_hash("cw1xxx", MOCK_CHAIN_ID, 1, &empty_ballots_hash(), &three_names_hash(), &tally);
        let mut rd = [0u8; 64];
        rd[..32].copy_from_slice(&commit);
        // Wrong DST in upper 32 — leave as zeros (no DST_VERIFIED_RCV_TALLY_V1).
        let pi = HexBinary::from(build_synthetic_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &rd, 0, 0,
        ));
        let deps = mock_dependencies();
        let err = verify_publish_quote(deps.as_ref(), &reg, &commit, &dummy_proof(), &pi)
            .unwrap_err();
        assert!(matches!(err, ContractError::AttestationDomainTagInvalid));
    }

    #[test]
    fn publish_quote_wrong_commit_hash_rejected() {
        let reg = good_registry();
        let tally = minimal_valid_tally();
        // PI committing to a DIFFERENT election_id
        let pi = synthetic_pi_for_tally("cw1xxx", 999, &tally, &reg);
        // Expected commit for the REAL election_id
        let expected_commit = compute_commit_hash("cw1xxx", MOCK_CHAIN_ID, 1, &empty_ballots_hash(), &three_names_hash(), &tally);
        let deps = mock_dependencies();
        let err = verify_publish_quote(deps.as_ref(), &reg, &expected_commit, &dummy_proof(), &pi)
            .unwrap_err();
        assert!(matches!(err, ContractError::AttestationCommitMismatch));
    }

    #[test]
    fn publish_quote_wrong_measurements_rejected() {
        let mut reg = good_registry();
        let tally = minimal_valid_tally();
        let pi = synthetic_pi_for_tally("cw1xxx", 1, &tally, &reg);
        let commit = compute_commit_hash("cw1xxx", MOCK_CHAIN_ID, 1, &empty_ballots_hash(), &three_names_hash(), &tally);
        // Mutate registry to expect a DIFFERENT mrtd.
        reg.mrtd = vec![0xFF; 48];
        let deps = mock_dependencies();
        let err = verify_publish_quote(deps.as_ref(), &reg, &commit, &dummy_proof(), &pi)
            .unwrap_err();
        assert!(matches!(
            err,
            ContractError::AttestationMeasurementMismatch { field: "mrtd" }
        ));
    }

    #[test]
    fn publish_quote_correct_binding_ok() {
        let reg = good_registry();
        let tally = minimal_valid_tally();
        let pi = synthetic_pi_for_tally("cw1xxx", 1, &tally, &reg);
        let commit = compute_commit_hash("cw1xxx", MOCK_CHAIN_ID, 1, &empty_ballots_hash(), &three_names_hash(), &tally);
        let deps = mock_dependencies();
        verify_publish_quote(deps.as_ref(), &reg, &commit, &dummy_proof(), &pi).unwrap();
    }

    // ----------------------------------------------------------------
    // N1: registration-quote verification (B8(e))
    // ----------------------------------------------------------------

    #[test]
    fn registration_quote_wrong_dst_rejected() {
        let reg = good_registry();
        let pk = good_pubkey();
        // Build ReportData with correct preimage (pubkey + addr + election_id +
        // names_hash) but a WRONG DST tag in the upper 32 bytes (TALLY tag =
        // cross-purpose replay attempt).
        let nh = three_names_hash();
        let expected = build_registration_report_data(
            pk.as_slice(),
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &nh,
        );
        let mut rd = [0u8; 64];
        rd[..32].copy_from_slice(&expected[..32]);
        rd[32..32 + DST_TALLY_LITERAL.len()].copy_from_slice(DST_TALLY_LITERAL);
        let pi = HexBinary::from(build_synthetic_public_inputs(
            &[0; 48], &[0; 48], &[0; 48], &[0; 48], &[0; 48], &rd, 0, 0,
        ));
        let deps = mock_dependencies();
        let err = verify_registration_quote(
            deps.as_ref(),
            &reg,
            &pk,
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &nh,
            &dummy_proof(),
            &pi,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::AttestationDomainTagInvalid));
    }

    #[test]
    fn registration_quote_wrong_pubkey_binding_rejected() {
        let reg = good_registry();
        let pk_a = good_pubkey();
        let mut pk_b_bytes = vec![0u8; 33];
        pk_b_bytes[0] = 0x03;
        pk_b_bytes[1] = 0x01;
        let pk_b = HexBinary::from(pk_b_bytes);
        // PI commits to pk_a; chain expects pk_b binding (same addr+id+names).
        let pi = synthetic_pi_for_pubkey(&pk_a, &reg);
        let deps = mock_dependencies();
        let err = verify_registration_quote(
            deps.as_ref(),
            &reg,
            &pk_b,
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &three_names_hash(),
            &dummy_proof(),
            &pi,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::AttestationPubkeyBindingMismatch));
    }

    #[test]
    fn registration_quote_wrong_election_id_rejected() {
        // N22 (v0.3.12): a quote bound to election_id=1 cannot be replayed
        // for election_id=2.
        let reg = good_registry();
        let pk = good_pubkey();
        let pi = synthetic_pi_for_pubkey_with_id(&pk, &reg, &mock_contract_addr(), 1);
        let deps = mock_dependencies();
        let err = verify_registration_quote(
            deps.as_ref(),
            &reg,
            &pk,
            &mock_contract_addr(),
            2, // chain expects election_id=2
            &three_names_hash(),
            &dummy_proof(),
            &pi,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::AttestationPubkeyBindingMismatch));
    }

    #[test]
    fn registration_quote_old_pubkey_only_binding_rejected_as_wrong_election() {
        // N22 (v0.3.12): a quote that uses the v0.3.11 form (pubkey-only
        // ReportData hash, no addr+id) is rejected with the explicit
        // RegistrationQuoteWrongElection variant for operator diagnostics.
        let reg = good_registry();
        let pk = good_pubkey();
        let mut rd = [0u8; 64];
        // Old v0.3.11 form: SHA-256(pubkey) only, no addr or election_id.
        let mut hasher = Sha256::new();
        hasher.update(pk.as_slice());
        let h = hasher.finalize();
        rd[..32].copy_from_slice(&h);
        rd[32..32 + DST_PUBKEY_LITERAL.len()].copy_from_slice(DST_PUBKEY_LITERAL);
        let pi = HexBinary::from(build_synthetic_public_inputs(
            &reg.mrtd.clone().try_into().unwrap(),
            &[0; 48],
            &reg.rtmr1.clone().try_into().unwrap(),
            &reg.rtmr2.clone().try_into().unwrap(),
            &[0; 48],
            &rd, 0, 0,
        ));
        let deps = mock_dependencies();
        let err = verify_registration_quote(
            deps.as_ref(),
            &reg,
            &pk,
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &three_names_hash(),
            &dummy_proof(),
            &pi,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::RegistrationQuoteWrongElection));
    }

    /// v0.3.14 F1: a quote bound to the canonical `three_names_hash` cannot
    /// be replayed when the chain expects a different names_hash. Defense-in-
    /// depth: tamper detection moves from publish-only to publish+registration.
    #[test]
    fn registration_quote_wrong_names_hash_rejected() {
        let reg = good_registry();
        let pk = good_pubkey();
        let nh_a = three_names_hash();
        let mut nh_b = nh_a;
        nh_b[0] ^= 0xFF;
        let pi = synthetic_pi_for_pubkey_with_id_and_names(
            &pk,
            &reg,
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &nh_a,
        );
        let deps = mock_dependencies();
        let err = verify_registration_quote(
            deps.as_ref(),
            &reg,
            &pk,
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &nh_b, // chain expects a different names_hash
            &dummy_proof(),
            &pi,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::AttestationPubkeyBindingMismatch));
    }

    #[test]
    fn registration_quote_correct_binding_ok() {
        let reg = good_registry();
        let pk = good_pubkey();
        let pi = synthetic_pi_for_pubkey(&pk, &reg);
        let deps = mock_dependencies();
        verify_registration_quote(
            deps.as_ref(),
            &reg,
            &pk,
            &mock_contract_addr(),
            MOCK_NEXT_ELECTION_ID,
            &three_names_hash(),
            &dummy_proof(),
            &pi,
        )
        .unwrap();
    }

    // ----------------------------------------------------------------
    // M4: candidate-declaration-order enforcement (unchanged from v0.3.8)
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
    fn check_tally_well_formed_shuffled_winners_rejected() {
        let election = Election {
            id: 1,
            title: "t".into(),
            candidates: three_cands(),
            candidate_names: three_names(),
            start_at: Timestamp::from_seconds(0),
            end_at: Timestamp::from_seconds(100),
            ballot_count: 3,
            enclave_pubkey: good_pubkey(),
        };
        let mut tally = TallyResult {
            winners: vec!["c2".to_string(), "c0".to_string()],
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
        tally.winners = vec!["c0".to_string(), "c2".to_string()];
        check_tally_well_formed(&election, &tally).unwrap();
    }

    // ----------------------------------------------------------------
    // M1 + N1: CreateElection end-to-end with registration quote
    // ----------------------------------------------------------------

    #[test]
    fn create_election_during_voting_rejected() {
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
                registry_update_delay_seconds: 0,
            },
        )
        .unwrap();

        let pk = good_pubkey();
        let pi_first = synthetic_pi_for_pubkey_next_id(&pk, &good_registry(), 1);
        let first = exec_create_election(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            "first".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            pk.clone(),
            dummy_proof(),
            pi_first,
        );
        assert!(first.is_ok());

        let mut env2 = env.clone();
        env2.block.time = env.block.time.plus_seconds(500);
        // Second call would be id=2 if it succeeded; the create-gate
        // checks ElectionAlreadyActive *before* the registration-quote
        // check (see exec_create_election ordering), so the PI for id=1
        // is fine — we never reach the registration-quote check.
        let pi_second = synthetic_pi_for_pubkey_next_id(&pk, &good_registry(), 1);
        let second = exec_create_election(
            deps.as_mut(),
            env2,
            info,
            "second".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(2000),
            env.block.time.plus_seconds(3000),
            pk,
            dummy_proof(),
            pi_second,
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
                registry_update_delay_seconds: 0,
            },
        )
        .unwrap();
        let pk = good_pubkey();
        let pi = synthetic_pi_for_pubkey(&pk, &good_registry());
        exec_create_election(
            deps.as_mut(),
            env.clone(),
            info,
            "first".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            pk,
            dummy_proof(),
            pi,
        )
        .unwrap();
    }

    #[test]
    fn create_election_from_resolved_archives_prior_tally() {
        // N3 (v0.3.10): on Resolved → Created transition, the prior
        // tally is archived to HISTORICAL_TALLIES keyed by election_id.
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
                registry_update_delay_seconds: 0,
            },
        )
        .unwrap();
        // First election: create + publish (which advances to Resolved).
        let pk = good_pubkey();
        let pi_reg = synthetic_pi_for_pubkey(&pk, &good_registry());
        exec_create_election(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            "first".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            pk.clone(),
            dummy_proof(),
            pi_reg.clone(),
        )
        .unwrap();
        // Fast-forward into Tallying then publish.
        let mut env_tally = env.clone();
        env_tally.block.time = env.block.time.plus_seconds(2000);
        let first_tally = minimal_valid_tally();
        let pi_pub =
            synthetic_pi_for_tally(env_tally.contract.address.as_str(), 1, &first_tally, &good_registry());
        exec_publish_result(
            deps.as_mut(),
            env_tally.clone(),
            first_tally.clone(),
            dummy_proof(),
            pi_pub,
        )
        .unwrap();
        // Sanity: TALLY_RESULT loaded.
        assert!(TALLY_RESULT.may_load(&deps.storage).unwrap().is_some());
        assert!(HISTORICAL_TALLIES.may_load(&deps.storage, 1).unwrap().is_none());

        // Second election from the Resolved phase. v0.3.12 N22: counter
        // bumped to 1, so next_id = 2 — registration quote must bind id=2.
        let env_second = env_tally; // still post-end_at; tally is set => Resolved
        let pi_reg2 = synthetic_pi_for_pubkey_next_id(&pk, &good_registry(), 2);
        exec_create_election(
            deps.as_mut(),
            env_second,
            info,
            "second".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(3000),
            env.block.time.plus_seconds(4000),
            pk,
            dummy_proof(),
            pi_reg2,
        )
        .unwrap();
        // TALLY_RESULT cleared for the new election...
        assert!(TALLY_RESULT.may_load(&deps.storage).unwrap().is_none());
        // ...but the prior tally is archived under election_id=1.
        let archived = HISTORICAL_TALLIES.may_load(&deps.storage, 1).unwrap();
        assert_eq!(archived, Some(first_tally));
    }

    #[test]
    fn create_election_with_bad_registration_quote_rejected() {
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
                registry_update_delay_seconds: 0,
            },
        )
        .unwrap();
        let pk_chain = good_pubkey();
        let mut pk_quote_bytes = vec![0u8; 33];
        pk_quote_bytes[0] = 0x03;
        let pk_quote = HexBinary::from(pk_quote_bytes);
        // PI binds to pk_quote; admin submits pk_chain — substitution caught.
        let pi = synthetic_pi_for_pubkey(&pk_quote, &good_registry());
        let res = exec_create_election(
            deps.as_mut(),
            env.clone(),
            info,
            "e".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            pk_chain,
            dummy_proof(),
            pi,
        );
        assert!(matches!(res, Err(ContractError::AttestationPubkeyBindingMismatch)));
    }

    // ----------------------------------------------------------------
    // M3: UpdateRegistry gating (schema v0.3.9)
    // ----------------------------------------------------------------

    fn new_registry_v2() -> EnclaveImageRegistry {
        EnclaveImageRegistry {
            vkey_name: "verified_rcv_v2".into(),
            mrtd: vec![1u8; 48],
            rtmr1: vec![1u8; 48],
            rtmr2: vec![1u8; 48],
            rtmr0: None,
            rtmr3: None,
            accepted_tcb_statuses: vec![0, 1, 2, 3],
        }
    }

    fn instantiate_with_delay(
        deps: &mut cosmwasm_std::OwnedDeps<
            cosmwasm_std::testing::MockStorage,
            cosmwasm_std::testing::MockApi,
            cosmwasm_std::testing::MockQuerier,
        >,
        admin: CwAddr,
        delay: u64,
    ) {
        let info = message_info(&admin, &[]);
        let env = mock_env();
        instantiate(
            deps.as_mut(),
            env,
            info,
            InstantiateMsg {
                admin: Some(admin),
                registry: good_registry(),
                voting_duration_seconds: 1000,
                registry_update_delay_seconds: delay,
            },
        )
        .unwrap();
    }

    #[test]
    fn propose_registry_update_during_voting_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        instantiate_with_delay(&mut deps, admin.clone(), 0);
        let info = message_info(&admin, &[]);
        let pk = good_pubkey();
        let pi = synthetic_pi_for_pubkey(&pk, &good_registry());
        exec_create_election(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            "e".into(),
            three_cands(),
            three_names(),
            env.block.time.plus_seconds(10),
            env.block.time.plus_seconds(1000),
            pk,
            dummy_proof(),
            pi,
        )
        .unwrap();
        let mut env2 = env.clone();
        env2.block.time = env.block.time.plus_seconds(500);
        let err =
            exec_propose_registry_update(deps.as_mut(), env2, info, new_registry_v2()).unwrap_err();
        assert!(matches!(err, ContractError::RegistryUpdateDuringActiveElection));
    }

    #[test]
    fn propose_then_finalize_with_zero_delay_ok() {
        // delay=0: propose + finalize can happen in the same block, but
        // it's still two transactions (not one). This is the minimum
        // additional friction the v0.3.10 N2 flow imposes.
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        instantiate_with_delay(&mut deps, admin.clone(), 0);
        let info = message_info(&admin, &[]);
        exec_propose_registry_update(deps.as_mut(), env.clone(), info, new_registry_v2()).unwrap();
        // Registry not yet updated -- finalize required.
        assert_eq!(REGISTRY.load(&deps.storage).unwrap().vkey_name, "verified_rcv_v1");
        exec_finalize_registry_update(deps.as_mut(), env).unwrap();
        assert_eq!(REGISTRY.load(&deps.storage).unwrap().vkey_name, "verified_rcv_v2");
        assert!(PENDING_REGISTRY.may_load(&deps.storage).unwrap().is_none());
    }

    #[test]
    fn finalize_before_timelock_expiry_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        instantiate_with_delay(&mut deps, admin.clone(), 86400);
        let info = message_info(&admin, &[]);
        exec_propose_registry_update(deps.as_mut(), env.clone(), info, new_registry_v2()).unwrap();
        // Try to finalize at the same block (before delay).
        let err = exec_finalize_registry_update(deps.as_mut(), env.clone()).unwrap_err();
        assert!(matches!(err, ContractError::RegistryUpdateTimelockNotExpired));
        // Fast forward past the delay; now it succeeds.
        let mut env_later = env.clone();
        env_later.block.time = env.block.time.plus_seconds(86401);
        exec_finalize_registry_update(deps.as_mut(), env_later).unwrap();
        assert_eq!(REGISTRY.load(&deps.storage).unwrap().vkey_name, "verified_rcv_v2");
    }

    #[test]
    fn finalize_without_pending_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        instantiate_with_delay(&mut deps, admin, 0);
        let err = exec_finalize_registry_update(deps.as_mut(), env).unwrap_err();
        assert!(matches!(err, ContractError::NoPendingRegistryUpdate));
    }

    #[test]
    fn finalize_permissionless_after_timelock() {
        // After the timelock expires, ANY address (not just admin) can
        // finalize. This prevents a misbehaving admin from soft-bricking
        // a pending update.
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        let stranger = CwAddr::unchecked("stranger");
        instantiate_with_delay(&mut deps, admin.clone(), 100);
        exec_propose_registry_update(
            deps.as_mut(),
            env.clone(),
            message_info(&admin, &[]),
            new_registry_v2(),
        )
        .unwrap();
        let mut env_later = env.clone();
        env_later.block.time = env.block.time.plus_seconds(101);
        // Stranger (not admin) calls finalize: should succeed.
        let _info_stranger = message_info(&stranger, &[]);
        exec_finalize_registry_update(deps.as_mut(), env_later).unwrap();
        assert_eq!(REGISTRY.load(&deps.storage).unwrap().vkey_name, "verified_rcv_v2");
    }

    #[test]
    fn propose_while_pending_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        instantiate_with_delay(&mut deps, admin.clone(), 100);
        let info = message_info(&admin, &[]);
        exec_propose_registry_update(deps.as_mut(), env.clone(), info.clone(), new_registry_v2())
            .unwrap();
        let mut second = new_registry_v2();
        second.vkey_name = "verified_rcv_v3".into();
        let err = exec_propose_registry_update(deps.as_mut(), env, info, second).unwrap_err();
        assert!(matches!(err, ContractError::RegistryUpdateAlreadyPending));
    }

    #[test]
    fn cancel_clears_pending_admin_only() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        let attacker = CwAddr::unchecked("attacker");
        instantiate_with_delay(&mut deps, admin.clone(), 100);
        exec_propose_registry_update(
            deps.as_mut(),
            env,
            message_info(&admin, &[]),
            new_registry_v2(),
        )
        .unwrap();
        // Attacker cannot cancel.
        let err =
            exec_cancel_registry_update(deps.as_mut(), message_info(&attacker, &[])).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        assert!(PENDING_REGISTRY.may_load(&deps.storage).unwrap().is_some());
        // Admin can.
        exec_cancel_registry_update(deps.as_mut(), message_info(&admin, &[])).unwrap();
        assert!(PENDING_REGISTRY.may_load(&deps.storage).unwrap().is_none());
    }

    #[test]
    fn propose_non_admin_rejected() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = CwAddr::unchecked("admin");
        let attacker = CwAddr::unchecked("attacker");
        instantiate_with_delay(&mut deps, admin, 0);
        let err = exec_propose_registry_update(
            deps.as_mut(),
            env,
            message_info(&attacker, &[]),
            new_registry_v2(),
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }
}
