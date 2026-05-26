//! Contract storage schema (verified-rcv intent §2.5).
//!
//! The derived `Phase` enum is **not stored** — it is computed at query time
//! from `(block.time, start_at, end_at, tally_result.is_some())` per the
//! v0.3.2 encoding-discipline A2 note in the Quint protocol model. The
//! contract similarly derives it on demand (see `contract::derived_phase`).

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, Timestamp};
use cw_storage_plus::{Item, Map};

use verified_rcv_enclave_core::TallyResult;

/// Contract-wide configuration. Set at instantiate, never mutated.
#[cw_serde]
pub struct Config {
    pub admin: Addr,
    /// Block 1 parameter — currently unused at the handler layer (election
    /// supplies its own start_at / end_at) but retained on Config for
    /// future-default behaviour parity with the intent schema.
    pub voting_duration_seconds: u64,
}

/// The phase derived from `(block.time, election, tally_result)`. Computed
/// on demand by the `Phase` query and by every state-mutating handler that
/// guards on phase. Never stored.
#[cw_serde]
pub enum Phase {
    /// `block.time < start_at`
    Created,
    /// `start_at <= block.time < end_at`
    Voting,
    /// `block.time >= end_at AND tally_result.is_none()`
    Tallying,
    /// `tally_result.is_some()` — terminal
    Resolved,
}

/// Election storage. Single-election contract; `CreateElection` overwrites
/// (admin-only) — intent v0.3.4 §2.5 Block 1 (alternate path).
///
/// `enclave_pubkey` is the dstack-KMS-derived public key for this election
/// (intent §2.5 state variables; §6.3 trust boundary). Stored at
/// `CreateElection` time; the corresponding privkey is released by dstack
/// only after the enclave attests at tally time. Voters fetch this field
/// at `SubmitBallot` time and ECIES-encrypt their preference list under it.
#[cw_serde]
pub struct Election {
    pub id: u64,
    pub title: String,
    /// Candidate set; immutable for the election's lifetime. The candidate
    /// declaration order pinned here is load-bearing per intent v0.3.1 T5
    /// (Stage 1 iteration discipline) and v0.3.4 A6 (round-count Vec
    /// ordering).
    pub candidates: Vec<Addr>,
    pub start_at: Timestamp,
    pub end_at: Timestamp,
    pub ballot_count: u32,
    /// dstack-KMS-derived ECIES public key for this election. Set at
    /// `CreateElection` (admin obtains via dstack key-derivation against
    /// contract_addr + election_id, before the enclave is invoked).
    pub enclave_pubkey: HexBinary,
}

/// Image-identity-binding registry per intent §6.1. Set at instantiate
/// (verified-rcv chooses the simpler "register-at-instantiate" trust model
/// rather than the Quartz dynamic-handshake model); never mutated.
///
/// `image_registration_honest(σ)` in the Quint spec is the predicate that
/// these three fields equal the canonical values for the audited enclave
/// image. The contract does NOT enforce this on its own — the operator
/// is expected to register the canonical values at instantiate; if they
/// register adversarial values the chain has no recourse (per §4.3a/b).
#[cw_serde]
pub struct EnclaveImageRegistry {
    /// TDX measurement: the enclave-image identity component.
    pub mrtd: Vec<u8>,
    /// TDX runtime-measurement register: per-deployment image binding.
    pub rtmr: Vec<u8>,
    /// Verification key name as registered in Xion's ZK module (used by
    /// the gnark Groth16 verifier through the chain-side ProofVerifyGnark
    /// query).
    pub vkey: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ELECTION: Item<Election> = Item::new("election");
pub const ELECTION_COUNTER: Item<u64> = Item::new("election_counter");
pub const REGISTRY: Item<EnclaveImageRegistry> = Item::new("registry");

/// Tally result. Set once at `PublishResult`; immutable thereafter (B1).
/// Stored as `Item<TallyResult>` and queried as `Option<TallyResult>`
/// (the storage absence is the `None` case).
pub const TALLY_RESULT: Item<TallyResult> = Item::new("tally_result");

/// Encrypted ballots: voter address → ciphertext. Keys are constrained to
/// the candidate set by the `SubmitBallot` handler (intent S4 + B6).
pub const BALLOTS: Map<&Addr, HexBinary> = Map::new("ballots");
