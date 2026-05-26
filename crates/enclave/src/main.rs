//! verified-rcv enclave runtime CLI.
//!
//! Reads (candidates, raw_ballots, privkey) from stdin as JSON, runs
//! Stage 1 + Stage 2 (Tally_spec per intent §2.5), prints the resulting
//! TallyResult as JSON.
//!
//! This is the OFF-TDX path — useful for local end-to-end testing of
//! the chain → enclave → published-tally flow. The TDX-attested
//! production binary (with dstack KMS + Quartz attestation envelope
//! construction) is a future runtime mode; the math is identical.
//!
//! ## Input JSON shape
//!
//! ```json
//! {
//!   "candidates": ["addr_a", "addr_b", "addr_c"],
//!   "raw_ballots": [
//!     {"voter": "addr_a", "ciphertext": "00abcdef..."},
//!     {"voter": "addr_b", "ciphertext": "01abcdef..."}
//!   ],
//!   "privkey_hex": "deadbeef..."
//! }
//! ```
//!
//! `ciphertext` is a hex string of the ECIES wire bytes; `privkey_hex`
//! is the 64-character hex of the secp256k1 secret key.

use std::io::Read;

use serde::Deserialize;
use verified_rcv_enclave::tally_spec;
use verified_rcv_enclave_core::{Addr, RawBallots, RawEntry};

#[derive(Deserialize)]
struct Input {
    candidates: Vec<Addr>,
    raw_ballots: Vec<InputRawEntry>,
    privkey_hex: String,
}

#[derive(Deserialize)]
struct InputRawEntry {
    voter: Addr,
    /// Hex-encoded ECIES wire bytes.
    ciphertext: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;

    let input: Input = serde_json::from_str(&buf)?;

    let privkey = hex::decode(&input.privkey_hex)?;
    if privkey.len() != 32 {
        return Err(format!("privkey_hex must decode to 32 bytes, got {}", privkey.len()).into());
    }

    let raw_ballots: RawBallots = input
        .raw_ballots
        .into_iter()
        .map(|e| {
            Ok::<RawEntry, hex::FromHexError>(RawEntry {
                voter: e.voter,
                ciphertext: hex::decode(&e.ciphertext)?,
            })
        })
        .collect::<Result<_, _>>()?;

    let result = tally_spec(&raw_ballots, &input.candidates, &privkey);

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}
