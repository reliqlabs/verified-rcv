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
    /// length constraints; vkey_name must be non-empty; accepted_tcb_statuses
    /// must be non-empty and not contain 6 (Revoked).
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

    /// C2: attestation commit-hash mismatch. The publish-quote's
    /// `ReportData[0..32]` must equal
    /// `SHA-256(canonical_serialization(contract_addr ‖ election_id ‖
    /// tally_body))`.
    #[error("attestation commit hash mismatch: ReportData[0..32] binds to a different (contract_addr, election_id, tally_body)")]
    AttestationCommitMismatch,

    /// C2: attestation domain-separation-tag mismatch. The quote's
    /// `ReportData[32..64]` must equal the purpose-specific DST tag
    /// (DST_VERIFIED_RCV_TALLY_V1 for publish; DST_VERIFIED_RCV_PUBKEY_V1
    /// for registration).
    #[error("attestation domain tag mismatch")]
    AttestationDomainTagInvalid,

    // N1 audit re-review remediations (v0.3.9 — gnark ProofVerifyGnark).

    /// N1: `public_inputs` blob does not have the expected length per
    /// the §2.5 gnark public_inputs byte layout (306 fr-elements ×
    /// 32 BE bytes = 9_792 bytes for the verified-rcv DCAP circuit).
    #[error("gnark public_inputs length: expected {expected}, got {got}")]
    GnarkPublicInputsLength { got: usize, expected: usize },

    /// N1: a `uints.U8` field in the gnark `public_inputs` has a non-zero
    /// high byte (the U8 invariant requires the high 31 bytes of each
    /// 32-byte BE field element to be zero; only the last byte carries
    /// the U8 value).
    #[error("gnark public_inputs element {elem_idx} is not a valid uints.U8 (non-zero high byte)")]
    GnarkPublicInputNotU8 { elem_idx: usize },

    /// N1: a `frontend.Variable` field in the gnark `public_inputs`
    /// exceeds u64 range (high 24 bytes of the 32-byte BE field element
    /// must be zero for the values we expect: TcbStatus, Timestamp).
    #[error("gnark public_inputs element {elem_idx} exceeds u64 range")]
    GnarkPublicInputOutOfRange { elem_idx: usize },

    /// N1: `xion.zk.v1.Query/ProofVerifyGnark` returned `verified=false`.
    /// The Groth16 proof does not verify under the registered vkey.
    #[error("gnark proof verification failed via xion.zk module")]
    ProofVerificationFailed,

    /// N1: extracted measurement (MrTd / Rtmr0 / Rtmr1 / Rtmr2 / Rtmr3)
    /// differs from the registry-bound value.
    #[error("attestation measurement mismatch: field={field}")]
    AttestationMeasurementMismatch { field: &'static str },

    /// N1: `ReportData[0..32]` of a *registration* quote does not equal
    /// `SHA-256(enclave_pubkey)` — the pubkey is not bound to a TDX quote
    /// from the registered enclave image (B8(e) v0.3.9).
    #[error("attestation pubkey binding mismatch: ReportData[0..32] ≠ SHA-256(enclave_pubkey)")]
    AttestationPubkeyBindingMismatch,

    /// N1: extracted TcbStatus is not in the registry's
    /// `accepted_tcb_statuses` set. Severity 6 (Revoked) is also
    /// circuit-hard-rejected by the gnark prover.
    #[error("attestation TcbStatus={status} not in accepted set")]
    AttestationTcbStatusUnaccepted { status: u8 },

    // N2 audit re-review remediations (v0.3.10 — timelocked registry update).

    /// N2: `ProposeRegistryUpdate` while another update is already pending.
    /// The admin must `CancelRegistryUpdate` or `FinalizeRegistryUpdate`
    /// first (only one slot).
    #[error("a registry update is already pending; cancel or finalize before proposing another")]
    RegistryUpdateAlreadyPending,

    /// N2: `FinalizeRegistryUpdate` with no pending update.
    #[error("no pending registry update to finalize")]
    NoPendingRegistryUpdate,

    /// N2: `FinalizeRegistryUpdate` called before the timelock expired.
    #[error("registry update timelock not yet expired (apply_after unreached)")]
    RegistryUpdateTimelockNotExpired,
}
