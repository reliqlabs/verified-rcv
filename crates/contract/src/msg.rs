//! External message types.
//!
//! Schemas refine the Quint protocol model's action arguments. Phase is not
//! transported (it is derived per intent v0.3.2 A2); `CreateElection` is the
//! Block 1 alternate path; `CloseAndTally` is intent §2.5 Block 5.
//!
//! Audit-finding remediations (2026-05-26 v0.3.8):
//! - C1: `AttestationEnvelope::Mock` was compile-time-excluded from the
//!   production build. **Superseded by N1 below** — the entire envelope
//!   wrapper is gone in v0.3.9; the `mock-attestation` cargo feature now
//!   gates the contract's `xion.zk` gRPC call instead.
//! - M3: `UpdateRegistry` execute message remains.
//!
//! N1 audit re-review remediation (2026-05-26 v0.3.9):
//! - `DstackEnvelope` / `AttestationEnvelope` removed.
//! - `PublishResult` and `CreateElection` now carry `proof: HexBinary` and
//!   `public_inputs: HexBinary` (the gnark Groth16 proof bytes and the
//!   9_792-byte `public_inputs` blob per intent §2.5 byte layout).
//! - Verification routes through `/xion.zk.v1.Query/ProofVerifyGnark`
//!   directly from the contract (see `contract::verify_gnark_proof`).

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
    /// Image-identity-binding registry per intent §6.1 (v0.3.9 schema:
    /// `vkey_name` + 48-byte mrtd/rtmr1/rtmr2 + optional rtmr0/rtmr3 +
    /// accepted_tcb_statuses). Set at instantiate; can be rotated later
    /// via `UpdateRegistry` when no election is active (M3 audit
    /// remediation). Shape-validated at instantiate time.
    pub registry: EnclaveImageRegistry,
    /// Block 1 parameter — voting window duration recorded on Config.
    /// Per-election `start_at` and `end_at` are supplied via
    /// `CreateElection`; this field is retained for future-default
    /// behaviour parity.
    pub voting_duration_seconds: u64,
    /// v0.3.10 N2: timelock delay between `ProposeRegistryUpdate` and
    /// the earliest `FinalizeRegistryUpdate`. Production deployments
    /// should set this to a non-zero value (e.g., 86400 = 1 day) so
    /// voters can detect a registry rotation before it takes effect.
    pub registry_update_delay_seconds: u64,
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg {
    /// Block 1 (alternate path): create a new election (admin-only).
    /// Audit remediation M1: only allowed when no election is mid-flight
    /// (i.e., the phase is initial-Created OR Resolved). Voting/Tallying
    /// phases reject this call to avoid B1 violations across re-creations.
    ///
    /// N1 v0.3.9 amendment: `enclave_pubkey` is now cryptographically
    /// bound to a TDX quote via B8(e) — the `proof` + `public_inputs`
    /// carry a registration quote whose `ReportData[0..32]` equals
    /// `SHA-256(enclave_pubkey)` and whose measurements match the
    /// `EnclaveImageRegistry`. Admin can no longer pick an arbitrary
    /// pubkey; the pubkey must come from a TDX enclave whose code
    /// measures to the registered MRTD/RTMR.
    CreateElection {
        title: String,
        candidates: Vec<Addr>,
        start_at: Timestamp,
        end_at: Timestamp,
        enclave_pubkey: HexBinary,
        /// Gnark Groth16 BN254 proof bytes (registration quote).
        proof: HexBinary,
        /// 9_792-byte `public_inputs` blob per intent §2.5 gnark byte
        /// layout. Carries MrTd ‖ Rtmr0..3 ‖ ReportData ‖ TcbStatus ‖
        /// Timestamp as 306 BE fr-elements.
        public_inputs: HexBinary,
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
    /// `(proof, public_inputs)`, not via `msg.sender`. A replay of the
    /// enclave's `(tally, proof, public_inputs)` from a different sender
    /// finalises the same result, so it is a no-op (replay-protection by
    /// B1 + `AlreadyResolved` rejection).
    ///
    /// N1 v0.3.9 amendment: the prior `attestation: DstackAttestation`
    /// wrapper is removed; the gnark proof + public_inputs are carried
    /// directly. Verification goes through
    /// `/xion.zk.v1.Query/ProofVerifyGnark` + measurement extraction +
    /// `ReportData[0..32] = commit_hash` equality + `ReportData[32..64] =
    /// DST_VERIFIED_RCV_TALLY_V1_PADDED` equality.
    PublishResult {
        tally: TallyResult,
        /// Gnark Groth16 BN254 proof bytes (publish quote).
        proof: HexBinary,
        /// 9_792-byte `public_inputs` blob per intent §2.5 gnark byte
        /// layout.
        public_inputs: HexBinary,
    },

    /// v0.3.10 N2 (replaces v0.3.8 M3 UpdateRegistry):
    /// admin-only proposal of a new registry. Stored as a pending update
    /// with `apply_after = env.block.time + config.registry_update_delay_seconds`.
    /// Voters can observe via `QueryMsg::PendingRegistry` and react
    /// before the timelock expires.
    ProposeRegistryUpdate { registry: EnclaveImageRegistry },

    /// v0.3.10 N2: permissionless finalize of a previously-proposed
    /// registry update. Requires `env.block.time >= pending.apply_after`.
    /// Anyone (not just admin) can call once the timelock expires, so a
    /// misbehaving admin who proposes-and-disappears doesn't soft-brick
    /// the contract.
    FinalizeRegistryUpdate {},

    /// v0.3.10 N2: admin-only cancellation of a pending registry update.
    /// Discards the pending slot; no-op if nothing is pending.
    CancelRegistryUpdate {},
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
    /// Reflects the CURRENT election only — for archived prior elections
    /// (v0.3.10 N3) use `HistoricalTally`.
    #[returns(ResultResponse)]
    Result {},

    /// v0.3.10 N3: query an archived tally by `election_id`. Returns
    /// `None` if no election with that id has resolved-then-been-superseded.
    #[returns(ResultResponse)]
    HistoricalTally { election_id: u64 },

    #[returns(EnclaveImageRegistry)]
    Registry {},

    /// v0.3.10 N2: query the currently-pending registry update (if any),
    /// for voters to observe before the timelock expires.
    #[returns(PendingRegistryResponse)]
    PendingRegistry {},
}

#[cw_serde]
pub struct BallotsResponse {
    pub ballots: Vec<(Addr, HexBinary)>,
}

#[cw_serde]
pub struct ResultResponse {
    pub result: Option<TallyResult>,
}

#[cw_serde]
pub struct PendingRegistryResponse {
    /// `None` if no update is currently pending.
    pub pending: Option<crate::state::PendingRegistry>,
}
