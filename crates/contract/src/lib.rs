//! verified-rcv CosmWasm contract.
//!
//! Implements the chain-side state machine specified in
//! `specs/rcv.qnt` (the Quint protocol model). The actual IRV tally runs
//! off-chain in the dstack TDX enclave (`crates/enclave/`); this contract
//! owns the ballot storage, the image-identity registry, and the
//! single-write `tally_result` slot.

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;
