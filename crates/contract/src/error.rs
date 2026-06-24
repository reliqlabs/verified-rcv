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

    // N1 audit re-review remediations (v0.3.9 — direct xion.zk proof verify).

    /// N1: `public_inputs` blob does not have the expected length per the
    /// dcap-noir packed UltraHonk layout (17 BN254 fields × 32 BE bytes =
    /// 544 bytes for the verified-rcv DCAP circuit).
    #[error("public_inputs length: expected {expected}, got {got}")]
    PublicInputsLength { got: usize, expected: usize },

    /// N1: a packed limb in `public_inputs` violates the pack_be
    /// injectivity invariant — bytes above the limb's K low bytes are
    /// non-zero. A canonical dcap-noir output never sets them; a violation
    /// is malformed or adversarial input.
    #[error("public_inputs field {field} is a non-canonical packed limb (non-zero high bytes)")]
    PublicInputsMalformed { field: usize },

    /// N1: `xion.zk.v1.Query/ProofVerifyUltraHonk` returned `verified=false`.
    /// The UltraHonk proof does not verify under the registered vkey.
    #[error("UltraHonk proof verification failed via xion.zk module")]
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
    /// circuit-hard-rejected by the dcap-noir prover.
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

    // N17 audit re-review remediation (v0.3.12 — finalize-mid-election DoS).

    /// N17: `CreateElection` while a registry update is pending. If
    /// allowed, an attacker would race to finalize after voting opens
    /// and cause every PublishResult to fail with
    /// `AttestationMeasurementMismatch`. Admin must finalize or cancel
    /// the pending update before starting an election.
    #[error("a registry update is pending; finalize or cancel before creating an election")]
    PendingRegistryUpdateBlocksCreateElection,

    /// N17 defense-in-depth: `FinalizeRegistryUpdate` while an election
    /// is in `Voting` or `Tallying` phase. The propose-gate plus the
    /// create-gate make this state unreachable in normal flow, but the
    /// finalize-gate catches any future code path that could create an
    /// election with a pending registry slipping through.
    #[error("registry update cannot finalize while an election is in Voting or Tallying phase")]
    FinalizeDuringActiveElection,

    // N22 audit re-review remediation (v0.3.12 — registration quote replay
    // across elections).

    /// N22: `CreateElection`'s registration quote was bound to a different
    /// `(contract_addr, election_id)` tuple than the one being assigned.
    /// This blocks privkey-replay-across-elections: admin who reuses an
    /// old quote for a new election fails this check because the
    /// quote's ReportData hashes a different election_id.
    #[error("registration quote bound to wrong election_id (replay attempt)")]
    RegistrationQuoteWrongElection,

    // v0.3.14 candidate_names validation.

    /// v0.3.14: `candidate_names.len()` does not equal `candidates.len()`.
    #[error("candidate_names length {actual} != candidates length {expected}")]
    CandidateNamesLengthMismatch { expected: usize, actual: usize },

    /// v0.3.14: a candidate name at the given index is empty (byte-length
    /// zero is rejected — every candidate must carry a non-empty label).
    #[error("candidate_names[{index}] is empty (byte-length must be >= 1)")]
    CandidateNameEmpty { index: usize },

    /// v0.3.14: a candidate name exceeds the 64-byte UTF-8 cap.
    #[error("candidate_names[{index}] byte-length {len} exceeds cap of {max}")]
    CandidateNameTooLong { index: usize, len: usize, max: usize },

    /// v0.3.14: a candidate name failed UTF-8 validation. `Vec<String>` is
    /// type-system-guaranteed valid UTF-8 in the cw_serde JSON path, so
    /// this variant is unreachable today; reserved for a future Borsh
    /// bytes path that could carry pre-validated raw bytes.
    #[allow(dead_code)]
    #[error("candidate_names[{index}] is not valid UTF-8")]
    CandidateNameInvalidUtf8 { index: usize },

    /// v0.3.14: a candidate name contains an embedded NUL (0x00) byte.
    /// NUL is rejected to keep names safe for C-string-style consumers
    /// (UIs, logs) without per-consumer escaping.
    #[error("candidate_names[{index}] contains an embedded NUL byte")]
    CandidateNameContainsNul { index: usize },

    /// v0.3.14: two candidate names are byte-equal. The chain enforces
    /// byte-distinctness (case-sensitive); visual-confusability is
    /// out-of-scope per intent §6.4 off-chain responsibility note.
    #[error("candidate_names[{index}] is byte-equal to candidate_names[{duplicate_of}]")]
    DuplicateCandidateName { index: usize, duplicate_of: usize },
}
