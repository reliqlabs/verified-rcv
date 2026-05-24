use cosmwasm_std::StdError;
use thiserror::Error;

/// Contract error variants. Each rejects the transaction atomically; no state
/// mutates on failure. Names mirror intent §2.4's `ContractError` enum but
/// collapse a few intent variants whose handler-level distinction is moot
/// (e.g., the contract never rejects an `EmptyBallot` shape directly because
/// the ciphertext is opaque to the chain).
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("standard cosmwasm error: {0}")]
    Std(#[from] StdError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("not enough candidates: need at least 2")]
    NotEnoughCandidates,

    #[error("duplicate candidate")]
    DuplicateCandidate,

    #[error("invalid voting window: start_at must be < end_at")]
    InvalidVotingWindow,

    #[error("not in voting phase")]
    NotVoting,

    #[error("not in tallying phase")]
    NotTallying,

    #[error("already voted")]
    AlreadyVoted,

    #[error("voter not in candidate set")]
    VoterNotCandidate,

    #[error("election already resolved")]
    AlreadyResolved,

    #[error("attestation failure: {0}")]
    AttestationFailure(String),

    #[error("election not found")]
    NoElection,
}
