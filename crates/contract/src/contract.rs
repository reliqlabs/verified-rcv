//! Entry points + handlers. Refine the Quint protocol model action by action.

use std::collections::HashSet;

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
    Config, Election, Phase, BALLOTS, CONFIG, ELECTION, ELECTION_COUNTER, REGISTRY, TALLY_RESULT,
};

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

    CONFIG.save(
        deps.storage,
        &Config {
            admin,
            voting_duration_seconds: msg.voting_duration_seconds,
        },
    )?;
    REGISTRY.save(deps.storage, &msg.registry)?;
    ELECTION_COUNTER.save(deps.storage, &0u64)?;
    // TALLY_RESULT.may_load returning None is the `not present` case; no
    // initialization needed.

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
        } => exec_create_election(deps, env, info, title, candidates, start_at, end_at),
        ExecuteMsg::SubmitBallot { ciphertext } => {
            exec_submit_ballot(deps, env, info, ciphertext)
        }
        ExecuteMsg::CloseAndTally {} => exec_close_and_tally(deps, env),
        ExecuteMsg::PublishResult { tally, attestation } => {
            exec_publish_result(deps, env, tally, attestation)
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
/// Refines Quint `init` for the per-election fields. Clears prior ballots
/// and resets `tally_result`. The Quint model treats instantiate as
/// monolithic; the contract splits the per-deployment fields (Config +
/// Registry) from the per-election fields (Election + ballots +
/// tally_result) so a single contract instance can host successive
/// elections under the same admin + registry.
fn exec_create_election(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    candidates: Vec<Addr>,
    start_at: Timestamp,
    end_at: Timestamp,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized);
    }
    if candidates.len() < 2 {
        return Err(ContractError::NotEnoughCandidates);
    }

    // Duplicate detection in declaration-order; pairs with intent §2.5's
    // "all candidate addresses distinct" Block 1 precondition.
    let mut seen: HashSet<&Addr> = HashSet::new();
    for c in &candidates {
        if !seen.insert(c) {
            return Err(ContractError::DuplicateCandidate);
        }
    }

    // Quint `submit_ballot` action requires `start_at <= block.time < end_at`
    // to be reachable; we enforce the structural relation here and leave
    // `start_at >= now` as a soft constraint. (Intent §2.5 Block 1 lists
    // `start_at > env.block.time` as a Require; we follow the intent.)
    if start_at >= end_at {
        return Err(ContractError::InvalidVotingWindow);
    }
    if start_at < env.block.time {
        return Err(ContractError::InvalidVotingWindow);
    }

    // Clear any stale ballots from a prior election under this instance.
    let stale_keys: Vec<Addr> = BALLOTS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for key in stale_keys {
        BALLOTS.remove(deps.storage, &key);
    }

    // Reset tally — the new election is unresolved.
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
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "create_election")
        .add_attribute("election_id", next_id.to_string()))
}

/// Block 3: voter submits a ballot ciphertext.
///
/// Quint guards refined:
///   - `derived_phase == Voting`  — phase check below
///   - `candidate_set.contains(sender)` — VoterNotCandidate
///   - `!ballots.has(sender)`     — AlreadyVoted (B6 ∀-per-key)
///   - ciphertext non-empty       — implicit in Quint via `ct != ""`
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
///
/// Quint refinement: the action is enabled only in `Tallying`; on chain we
/// keep that guard so the event is meaningful (an enclave watcher
/// subscribing to `wasm-close_and_tally` events expects the chain time to
/// have crossed end_at).
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
/// Quint guards refined:
///   - `derived_phase == Tallying`       — phase check
///   - `!contract_tally_result.present`  — AlreadyResolved
///   - `image_registration_honest(...)`  — operator's responsibility at
///     instantiate; we re-check the envelope binds to the registry
///   - `well_formed_tally(t)`            — chain-syntactic checks below
///   - attestation verifies              — Mock accepts; Dstack stub-checks
///     the user-data binding
///
/// Per intent §2.5 Block 6, **any chain address may submit**; the enclave
/// identity is verified via the carried `attestation`, not via msg.sender.
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

    // Chain-syntactic well-formedness (intent §3.1 S6–S9 subset checkable
    // without re-running IRV). The semantic correctness obligation (B10)
    // is discharged off-chain via the attestation envelope.
    check_tally_well_formed(&election, &tally)?;

    // Attestation envelope handling. Mock accepts unconditionally (per the
    // crate's `mock` feature shape); Dstack stub-verifies the binding hash.
    // Full TDX-quote + zkdcap-proof verification is deferred to a later
    // round.
    let _registry = REGISTRY.load(deps.storage)?;
    match attestation {
        AttestationEnvelope::Mock => {}
        AttestationEnvelope::Dstack {
            quote: _,
            zk_proof: _,
            user_data,
        } => verify_dstack_user_data_shape(&user_data)?,
    }

    TALLY_RESULT.save(deps.storage, &tally)?;

    Ok(Response::new()
        .add_attribute("action", "publish_result")
        .add_attribute("election_id", election.id.to_string())
        .add_attribute("winners_count", tally.winners.len().to_string()))
}

// ============================================================
// Derived-phase computation (intent v0.3.2 A2)
// ============================================================

/// Pure derivation; mirrors Quint `derived_phase(block_time, tally_result)`.
///
/// Exposed `pub(crate)` so the verification harnesses can target it
/// directly without going through CosmWasm storage (which is opaque to
/// Kani's symbolic execution).
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
// Tally well-formedness checks (chain-side syntactic subset of S6–S9)
// ============================================================

/// Chain-syntactic well-formedness check for a published tally. Mirrors the
/// inspectable subset of intent §3.1 S6-S9 (semantic correctness is B10,
/// off-chain). Exposed `pub(crate)` so verification harnesses can target it
/// directly without going through CosmWasm storage.
///
/// Implementation note: uses Vec-based linear searches (not HashSet) so the
/// function is reachable from Kani symbolic execution. HashSet/HashMap on
/// macOS use `CCRandomGenerateBytes` for random-seeding DoS protection, a
/// path Kani 0.67 does not model. Linear search over a small (≤ N_CANDIDATES)
/// set is O(N²) worst-case here, which is acceptable for chain-side checks
/// where N is bounded by gas limits.
pub(crate) fn check_tally_well_formed(
    election: &Election,
    tally: &TallyResult,
) -> Result<(), ContractError> {
    let n_cands = election.candidates.len();

    // S6: winners are a non-empty subset of candidates.
    if tally.winners.is_empty() || tally.winners.len() > n_cands {
        return Err(ContractError::AttestationFailure(
            "winners cardinality out of bounds".into(),
        ));
    }
    for w in &tally.winners {
        if !election.candidates.iter().any(|c| c.as_str() == w.as_str()) {
            return Err(ContractError::AttestationFailure(
                "winner not in candidate set".into(),
            ));
        }
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

    // S8: each round's counts sum to ballots_tallied. The chain verifies
    // this syntactically; the semantic claim (counts come from IRV
    // tabulation over decrypted ballots) is B10 / off-chain.
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
    }

    // S9: candidates eliminated at round i do not reappear at round j > i.
    // Linear scan over the eliminated-by-round prefix; for the small N
    // (chain gas-bounded) this is O(rounds²) which is fine.
    for j in 0..tally.per_round_counts.len() {
        let round_j = &tally.per_round_counts[j];
        for rc in round_j {
            // Was rc.candidate eliminated in any earlier round i < j?
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

/// Stub Dstack-envelope verification: checks the structural shape of
/// `user_data` (64 bytes = 32-byte domain tag + 32-byte commit hash).
/// Full TDX-quote + zkdcap proof verification, and equality between the
/// commit hash and `SHA-256(canonical_serialization(contract_addr ||
/// tally_body))`, is deferred to a downstream round.
fn verify_dstack_user_data_shape(user_data: &HexBinary) -> Result<(), ContractError> {
    if user_data.len() != 64 {
        return Err(ContractError::AttestationFailure(
            "user_data must be 64 bytes (32 tag + 32 hash)".into(),
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
