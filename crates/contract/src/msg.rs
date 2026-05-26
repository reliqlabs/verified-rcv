//! External message types.
//!
//! Schemas refine the Quint protocol model's action arguments. Phase is not
//! transported (it is derived per intent v0.3.2 A2); `CreateElection` is the
//! Block 1 alternate path; `CloseAndTally` is intent §2.5 Block 5.

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
    /// Image-identity-binding registry per intent §6.1. Set once at
    /// instantiate; never mutated.
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
    /// Block 1 (alternate path): create a new election (overwrites prior,
    /// admin-only). Clears all ballots and resets `tally_result`.
    ///
    /// `enclave_pubkey` is the dstack-KMS-derived ECIES public key for this
    /// election. Admin obtains it from dstack before calling this handler;
    /// the contract stores but does not verify it (dstack_kms_trust per
    /// intent §6.3). Voters fetch from `Election` query and encrypt under it.
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
    /// once (B1); subsequent calls hit `AlreadyResolved`.
    ///
    /// Trigger semantics per intent §2.5 Block 6: **any chain address may
    /// submit**; the enclave identity is verified via the carried
    /// `attestation`, not via `msg.sender`. This is intentional — a replay
    /// of the enclave's `(tally, attestation)` from a different sender
    /// finalises the same result, so it is a no-op.
    PublishResult {
        tally: TallyResult,
        attestation: AttestationEnvelope,
    },
}

/// Attestation envelope per intent §6.1. Concretely, this is the chain-side
/// shape that B8 clauses (a)–(d) verify. Round 3c lands the `Mock` variant
/// (accepts any tally; used for testing + mock builds) and a stub `Dstack`
/// variant; full TDX-quote + zkdcap-proof verification follows in a
/// downstream round.
#[cw_serde]
pub enum AttestationEnvelope {
    /// Mock attestation — accepts any tally. Used for testing + mock builds.
    Mock,
    /// Real dstack attestation. Verifies TDX quote + zkdcap proof + commit
    /// hash. Stubbed in Round 3c; full integration in a future round per
    /// the roadmap.
    Dstack {
        /// TDX quote bytes.
        quote: HexBinary,
        /// zkdcap Groth16 proof bytes.
        zk_proof: HexBinary,
        /// Upper 32 bytes = domain-separation tag `DST_VERIFIED_RCV_TALLY_V1`
        /// (zero-padded); lower 32 bytes = SHA-256 over
        /// `canonical_serialization(contract_addr || tally_body)` per B8(c).
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
