//! Contract storage schema (verified-rcv intent §2.5; v0.3.9 N1 update for
//! gnark `ProofVerifyGnark` integration — `EnclaveImageRegistry` adopts the
//! split RTMR + optional-slot + accepted-TCB shape).
//!
//! The derived `Phase` enum is **not stored** — it is computed at query time
//! from `(block.time, start_at, end_at, tally_result.is_some())` per the
//! v0.3.2 encoding-discipline A2 note in the Quint protocol model. The
//! contract similarly derives it on demand (see `contract::derived_phase`).

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, Timestamp};
use cw_storage_plus::{Item, Map};

use verified_rcv_enclave_core::TallyResult;

/// Contract-wide configuration. Set at instantiate; admin may rotate registry
/// later via the timelocked `ProposeRegistryUpdate` + `FinalizeRegistryUpdate`
/// flow (v0.3.10 N2). Config itself is immutable post-instantiate.
#[cw_serde]
pub struct Config {
    pub admin: Addr,
    /// Block 1 parameter — currently unused at the handler layer (election
    /// supplies its own start_at / end_at) but retained on Config for
    /// future-default behaviour parity with the intent schema.
    pub voting_duration_seconds: u64,
    /// v0.3.10 N2: number of seconds between `ProposeRegistryUpdate` and
    /// the earliest `FinalizeRegistryUpdate`. Production deployments
    /// should set this to a meaningful value (e.g., 86400 = 1 day) so
    /// voters can detect a registry rotation before it lands. 0 means
    /// immediate finalize allowed (same-block propose → finalize).
    pub registry_update_delay_seconds: u64,
}

/// v0.3.10 N2: a registry update that has been proposed but not yet
/// finalized. `apply_after` is the earliest timestamp at which
/// `FinalizeRegistryUpdate` may apply the new registry.
#[cw_serde]
pub struct PendingRegistry {
    pub registry: EnclaveImageRegistry,
    pub apply_after: Timestamp,
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

/// Election storage. The contract holds at most one active election at a
/// time; `CreateElection` is admin-only (M1) and refuses to clobber an
/// election in Voting or Tallying phase. `enclave_pubkey` is bound to a
/// registration TDX quote at `CreateElection` time per B8(e) (v0.3.9) —
/// the admin no longer picks the pubkey freely.
#[cw_serde]
pub struct Election {
    pub id: u64,
    pub title: String,
    /// Candidate set; immutable for the election's lifetime. The candidate
    /// declaration order pinned here is load-bearing per intent v0.3.1 T5
    /// (Stage 1 iteration discipline) and v0.3.4 A6 (round-count Vec
    /// ordering).
    pub candidates: Vec<Addr>,
    /// v0.3.14: human-readable display names parallel-indexed to
    /// `candidates`. Length equals `candidates.len()`. Each name is
    /// validated at CreateElection time: byte-length 1..=64, no NUL byte,
    /// byte-distinct from every other name in the same election. Names
    /// are off-chain trust in the §6.4 sense (admin-supplied labels, not
    /// a cryptographic binding to voter intent); the chain enforces
    /// structural well-formedness + chain-side immutability (B11). The
    /// `names_hash` field in `canonical_serialization` binds the value
    /// the runtime sees at publish-time to the value stored here.
    pub candidate_names: Vec<String>,
    pub start_at: Timestamp,
    pub end_at: Timestamp,
    pub ballot_count: u32,
    /// dstack-derived ECIES public key for this election. v0.3.9 N1: bound
    /// to a TDX quote via the registration `(proof, public_inputs)` carried
    /// by `CreateElection`. The chain verifies that the quote's
    /// `ReportData[0..32] = SHA-256(enclave_pubkey)` and that the quote
    /// originates from an enclave whose measurements match the registry.
    pub enclave_pubkey: HexBinary,
}

/// Image-identity-binding registry per intent §6.1 (v0.3.9 N1 schema).
///
/// The chain-side verification surface for B8 clauses (a) — (d):
/// - `vkey_name` resolves an entry in Xion's on-chain `xion.zk` VKey store.
///   verified-rcv's contract calls `/xion.zk.v1.Query/ProofVerifyGnark`
///   referencing this name; the actual vkey bytes live in the zk module.
/// - `mrtd`, `rtmr1`, `rtmr2` are 48-byte TDX SHA-384 measurements that
///   the chain bytewise compares to the corresponding fields extracted
///   from the gnark proof's `public_inputs`.
/// - `rtmr0`, `rtmr3` are optional. `None` means "do not enforce" — the
///   gnark circuit still binds them in the proof, but the chain skips the
///   equality check. Mirrors zkdcap-verifier's `check_rtmr0` convention.
/// - `accepted_tcb_statuses` lists the TCB severity values (0..=6) the
///   chain accepts. Default in `validate_registry` is `{0,1,2,3}` — the
///   "configuration / SW hardening" classes. Severity 6 (Revoked) is
///   additionally hard-rejected by the gnark circuit itself.
///
/// `image_registration_honest(σ)` in the intent is the predicate that
/// (i) verified-rcv's registry has the canonical values AND (ii) the
/// xion.zk store binds the canonical vkey bytes under `vkey_name`. Both
/// surfaces are admin/governance-controlled; the contract enforces shape
/// only.
#[cw_serde]
pub struct EnclaveImageRegistry {
    /// Name of the gnark vkey registered in Xion's on-chain `xion.zk`
    /// VKey store. Used as `vkey_name` in `QueryVerifyGnarkRequest`.
    pub vkey_name: String,
    /// TDX MRTD (SHA-384, 48 bytes): the enclave image identity component.
    pub mrtd: Vec<u8>,
    /// TDX RTMR1 (SHA-384, 48 bytes): mandatory runtime measurement.
    pub rtmr1: Vec<u8>,
    /// TDX RTMR2 (SHA-384, 48 bytes): mandatory runtime measurement.
    pub rtmr2: Vec<u8>,
    /// TDX RTMR0 (SHA-384, 48 bytes): optional. `None` = chain does not
    /// enforce equality (firmware measurement; operator-controlled,
    /// frequently left unbound).
    pub rtmr0: Option<Vec<u8>>,
    /// TDX RTMR3 (SHA-384, 48 bytes): optional. `None` = chain does not
    /// enforce equality (workload-extended; sometimes unbound for stable
    /// deployments).
    pub rtmr3: Option<Vec<u8>>,
    /// TCB severity values the chain accepts (0=UpToDate .. 6=Revoked).
    /// Empty `Vec` is rejected at validation (operator must opt in to at
    /// least one severity); 6 is rejected at validation (Revoked is
    /// circuit-hard-rejected anyway).
    pub accepted_tcb_statuses: Vec<u8>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ELECTION: Item<Election> = Item::new("election");
pub const ELECTION_COUNTER: Item<u64> = Item::new("election_counter");
pub const REGISTRY: Item<EnclaveImageRegistry> = Item::new("registry");
/// v0.3.10 N2: pending registry update awaiting timelock expiry.
pub const PENDING_REGISTRY: Item<PendingRegistry> = Item::new("pending_registry");

/// Tally result. Set once at `PublishResult`; immutable thereafter (B1).
/// Stored as `Item<TallyResult>` and queried as `Option<TallyResult>`
/// (the storage absence is the `None` case).
///
/// v0.3.10 N3: when `CreateElection` succeeds while the prior election
/// is in `Resolved` phase, the prior tally is moved into
/// `HISTORICAL_TALLIES` (keyed by its election_id) before being cleared.
/// Consumers that pinned by `election_id` can still resolve the result.
pub const TALLY_RESULT: Item<TallyResult> = Item::new("tally_result");

/// Encrypted ballots: voter address → ciphertext. Keys are constrained to
/// the candidate set by the `SubmitBallot` handler (intent S4 + B6).
pub const BALLOTS: Map<&Addr, HexBinary> = Map::new("ballots");

/// Historical tallies indexed by election_id. v0.3.10 N3: on
/// `CreateElection` from `Resolved` phase, the just-resolved tally is
/// archived here so consumers can pin by election_id without depending
/// on `TALLY_RESULT` (which always reflects the *current* election).
pub const HISTORICAL_TALLIES: Map<u64, TallyResult> = Map::new("historical_tallies");

/// Historical Election metadata indexed by election_id. v0.3.12 N21:
/// the v0.3.10 N3 archival fix preserved `TallyResult` but lost the
/// `Election` (title, start_at, end_at, enclave_pubkey, candidates).
/// Consumers querying `HistoricalTally` for old elections now also get
/// the full Election context via `HistoricalElection`.
pub const HISTORICAL_ELECTIONS: Map<u64, Election> = Map::new("historical_elections");
