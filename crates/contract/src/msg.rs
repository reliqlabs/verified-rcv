//! External message types.
//!
//! Schemas refine the Quint protocol model's action arguments. Phase is not
//! transported (it is derived per intent v0.3.2 A2); `CreateElection` is the
//! Block 1 alternate path; `CloseAndTally` is intent §2.5 Block 5.
//!
//! Audit-finding remediations (2026-05-26):
//! - C1: `AttestationEnvelope::Mock` is now compile-time-excluded from the
//!   production build. The variant only exists when the `mock-attestation`
//!   feature is enabled (tests / dev / Kani-harness builds opt in). The
//!   default build's wasm has no Mock arm; the contract's match is
//!   exhaustive over Dstack alone.
//! - M3: `UpdateRegistry` execute message added so the operator can rotate
//!   `(mrtd, rtmr, vkey)` between elections (gated by `exec_update_registry`
//!   to phases where no election is active).

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, HexBinary, Timestamp};

use verified_rcv_enclave_core::TallyResult;

// Bring the storage-side types into the QueryResponses derive's scope.
#[allow(unused_imports)]
use crate::state::{Config, Election, Phase};
use crate::state::EnclaveImageRegistry;

#[cw_serde]
pub struct InstantiateMsg {
    /// Admin address; defaults to `msg.sender` when `None`.
    pub admin: Option<Addr>,
    /// Image-identity-binding registry per intent §6.1. Set at instantiate;
    /// can be rotated later via `UpdateRegistry` when no election is active
    /// (M3 audit remediation). Shape-validated at instantiate time.
    pub registry: EnclaveImageRegistry,
    /// Block 1 parameter — voting window duration recorded on Config.
    /// Per-election `start_at` and `end_at` are supplied via
    /// `CreateElection`; this field is retained for future-default
    /// behaviour parity.
    pub voting_duration_seconds: u64,
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Block 1 (alternate path): create a new election (admin-only).
    /// Audit remediation M1: only allowed when no election is mid-flight
    /// (i.e., the phase is initial-Created OR Resolved). Voting/Tallying
    /// phases reject this call to avoid B1 violations across re-creations.
    ///
    /// `enclave_pubkey` is the dstack-KMS-derived ECIES public key for this
    /// election. Admin obtains it from dstack before calling this handler;
    /// the contract length-validates (audit C3) but does not cryptographically
    /// verify provenance — the admin trust boundary covers this (intent §6.3
    /// `dstack_kms_trust`). Future cycles: require a dstack-KMS-signed
    /// provenance proof at this handler.
    CreateElection {
        title: String,
        candidates: Vec<Addr>,
        start_at: Timestamp,
        end_at: Timestamp,
        enclave_pubkey: HexBinary,
    },

    /// Block 3: voter submits a ballot ciphertext. `msg.sender` must be a
    /// candidate (intent S4 + B6 contract-layer projection).
    SubmitBallot { ciphertext: HexBinary },

    /// Block 5 (idempotent wakeup, permissionless): cross `end_at`,
    /// transition to Tallying. No-op storage-wise; emits an event that the
    /// off-chain enclave watcher can subscribe to.
    CloseAndTally {},

    /// Block 6: enclave publishes the attested tally. `tally_result` is set
    /// once per election (B1); subsequent calls hit `AlreadyResolved`.
    ///
    /// Trigger semantics per intent §2.5 Block 6: **any chain address may
    /// submit**; the enclave identity is verified via the carried
    /// `attestation`, not via `msg.sender`. A replay of the enclave's
    /// `(tally, attestation)` from a different sender finalises the same
    /// result, so it is a no-op.
    PublishResult {
        tally: TallyResult,
        attestation: AttestationEnvelope,
    },

    /// M3 audit remediation: admin-only registry rotation. Gated on no
    /// active election (initial-Created or Resolved phase only). Used to
    /// upgrade the enclave image between elections without re-instantiating
    /// the contract.
    UpdateRegistry { registry: EnclaveImageRegistry },
}

/// Attestation envelope per intent §6.1. Concretely, this is the chain-side
/// shape that B8 clauses (a)-(d) verify.
///
/// **Audit remediation C1 (2026-05-26)**: `Mock` is compile-time-gated
/// behind the `mock-attestation` cargo feature. Production builds
/// (`default-features = []` or empty feature set) do NOT include the
/// `Mock` variant — the enum has only `Dstack` and any incoming
/// `{"mock": ...}` payload deserializes to an error.
///
/// **Audit remediation C2 (2026-05-26)**: the `Dstack` variant now has
/// real verification on-chain — domain-tag check + commit-hash equality
/// against `SHA-256(canonical_serialization(contract_addr ‖ election_id ‖
/// tally_body))` (intent §2.5 v0.3.8 form). Groth16 zkdcap verification
/// + MRTD/RTMR-vs-registry binding are queued for a follow-on cycle that
/// integrates Xion's `ProofVerifyGnark` module.
#[cw_serde]
pub enum AttestationEnvelope {
    /// Mock attestation — accepts any tally. **Dev/test ONLY**, gated
    /// behind the `mock-attestation` Cargo feature. NEVER in production.
    #[cfg(feature = "mock-attestation")]
    Mock,
    /// Real dstack attestation. Currently verifies the user_data binding
    /// (domain tag + commit hash); full TDX-quote + zkdcap-proof
    /// verification follows.
    Dstack {
        /// TDX quote bytes.
        quote: HexBinary,
        /// zkdcap Groth16 proof bytes.
        zk_proof: HexBinary,
        /// 64 bytes: upper 32 = domain-separation tag
        /// `DST_VERIFIED_RCV_TALLY_V1` (zero-padded); lower 32 =
        /// SHA-256 over `canonical_serialization(contract_addr ‖
        /// election_id ‖ tally_body)` per intent §2.5 v0.3.8.
        user_data: HexBinary,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},

    #[returns(Election)]
    Election {},

    /// Derived phase per intent v0.3.2 A2 — computed at query time.
    #[returns(Phase)]
    Phase {},

    /// Returns the full `(voter, ciphertext)` map for the enclave to
    /// consume at tally time.
    #[returns(BallotsResponse)]
    Ballots {},

    /// `None` until `PublishResult` succeeds; `Some(TallyResult)` thereafter.
    #[returns(ResultResponse)]
    Result {},

    #[returns(EnclaveImageRegistry)]
    Registry {},
}

#[cw_serde]
pub struct BallotsResponse {
    pub ballots: Vec<(Addr, HexBinary)>,
}

#[cw_serde]
pub struct ResultResponse {
    pub result: Option<TallyResult>,
}
