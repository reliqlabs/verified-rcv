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

    // Audit-finding remediations (2026-05-26).

    /// C3: enclave_pubkey wrong shape (must be 33-byte compressed or
    /// 65-byte uncompressed secp256k1).
    #[error("invalid enclave_pubkey: expected 33 or 65 bytes, got {got}")]
    InvalidEnclavePubkey { got: usize },

    /// M3: registry shape rejected at instantiate. mrtd/rtmr have
    /// length constraints; vkey must be non-empty.
    #[error("invalid registry: {0}")]
    InvalidRegistry(String),

    /// M1: CreateElection while a previous election is mid-flight
    /// (Voting or Tallying). The contract forbids clobbering an
    /// active election. Wait for it to resolve.
    #[error("an election is already active in this contract; cannot create a new one until it resolves")]
    ElectionAlreadyActive,

    /// M3: UpdateRegistry attempted while an election is mid-flight.
    /// Registry can only be updated before any election or after
    /// the previous one is fully resolved.
    #[error("registry can only be updated when no election is active")]
    RegistryUpdateDuringActiveElection,

    /// C2: attestation commit-hash mismatch. The envelope's user_data
    /// bottom 32 bytes must equal SHA-256(canonical_serialization).
    #[error("attestation commit hash mismatch: envelope binds to a different (contract_addr, election_id, tally_body)")]
    AttestationCommitMismatch,

    /// C2: attestation domain-separation-tag mismatch. The envelope's
    /// user_data top 32 bytes must equal the verified-rcv domain tag.
    #[error("attestation domain tag mismatch")]
    AttestationDomainTagInvalid,
}
