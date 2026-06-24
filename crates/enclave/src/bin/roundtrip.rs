//! Roundtrip test binary for the ECIES + Borsh ballot wire format.
//!
//! Used by `ui/test/encryption.test.ts` to validate that the JavaScript
//! encryptor produces output the Rust enclave-side decoder accepts.
//!
//! Three subcommands:
//!   - keygen    — emit a fresh secp256k1 keypair as hex
//!   - encrypt   — Borsh-encode a ranking, ECIES-encrypt under a pubkey
//!   - decrypt   — ECIES-decrypt, Borsh-decode, emit the ranking as JSON

use borsh::{from_slice as borsh_from_slice, BorshDeserialize, BorshSerialize};
use clap::{Parser, Subcommand};
use ecies::{decrypt, encrypt};
use k256::{
    elliptic_curve::{rand_core::OsRng, sec1::ToEncodedPoint},
    SecretKey,
};

#[derive(Parser)]
#[command(
    name = "verified-rcv-roundtrip",
    about = "ECIES + Borsh ballot wire-format roundtrip helper"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a fresh secp256k1 keypair as hex.
    Keygen,
    /// Borsh-encode the comma-separated ranking and ECIES-encrypt under
    /// `--pubkey-hex`. Prints the wire-format ciphertext as hex.
    Encrypt {
        /// 130-hex-character uncompressed SEC1 pubkey (0x04 || X || Y).
        #[arg(long)]
        pubkey_hex: String,
        /// Comma-separated bech32 addresses in preference order.
        #[arg(long)]
        ranking: String,
    },
    /// ECIES-decrypt `--ciphertext-hex` under `--privkey-hex`, Borsh-decode
    /// the plaintext as `Vec<String>`, print the ranking as a JSON array.
    Decrypt {
        /// 64-hex-character secp256k1 secret key.
        #[arg(long)]
        privkey_hex: String,
        /// Wire-format ciphertext as hex (`pk(65) || nonce(16) || tag(16) || body`).
        #[arg(long)]
        ciphertext_hex: String,
    },
}

#[derive(BorshSerialize, BorshDeserialize)]
struct BorshRanking(Vec<String>);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Keygen => keygen(),
        Command::Encrypt {
            pubkey_hex,
            ranking,
        } => encrypt_cmd(&pubkey_hex, &ranking),
        Command::Decrypt {
            privkey_hex,
            ciphertext_hex,
        } => decrypt_cmd(&privkey_hex, &ciphertext_hex),
    }
}

fn keygen() -> Result<(), Box<dyn std::error::Error>> {
    let sk = SecretKey::random(&mut OsRng);
    let pk = sk.public_key();
    let pk_bytes = pk.to_encoded_point(false);
    let sk_bytes: [u8; 32] = sk.to_bytes().into();

    println!("{{");
    println!("  \"privkey_hex\": \"{}\",", hex::encode(sk_bytes));
    println!(
        "  \"pubkey_hex\": \"{}\"",
        hex::encode(pk_bytes.as_bytes())
    );
    println!("}}");
    Ok(())
}

fn encrypt_cmd(pubkey_hex: &str, ranking_csv: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pubkey_bytes = hex::decode(pubkey_hex)?;
    let ranking: Vec<String> = ranking_csv
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();

    // Plaintext = Borsh(Vec<String>). BorshRanking is a transparent
    // newtype so the encoding is identical to a bare Vec<String>.
    let plaintext = borsh::to_vec(&BorshRanking(ranking))?;

    let ciphertext = encrypt(&pubkey_bytes, &plaintext).map_err(|e| format!("encrypt: {e:?}"))?;
    println!("{}", hex::encode(ciphertext));
    Ok(())
}

fn decrypt_cmd(privkey_hex: &str, ciphertext_hex: &str) -> Result<(), Box<dyn std::error::Error>> {
    let privkey_bytes = hex::decode(privkey_hex)?;
    let ciphertext_bytes = hex::decode(ciphertext_hex)?;

    let plaintext =
        decrypt(&privkey_bytes, &ciphertext_bytes).map_err(|e| format!("decrypt: {e:?}"))?;
    let BorshRanking(ranking) = borsh_from_slice(&plaintext)?;

    println!("{}", serde_json::to_string(&ranking)?);
    Ok(())
}

// Compile-time sanity: bare Vec<String> Borsh-encodes the same as our
// transparent newtype. Guarantees `decrypt` is interpreting the wire format
// the UI emits.
#[cfg(test)]
mod tests {
    use super::*;
    use borsh::to_vec;

    #[test]
    fn newtype_matches_vec_string_encoding() {
        let r = vec!["xion1aaa".to_owned(), "xion1bbb".to_owned()];
        let bytes_bare = to_vec(&r).unwrap();
        let bytes_newtype = to_vec(&BorshRanking(r)).unwrap();
        assert_eq!(bytes_bare, bytes_newtype);
    }

    #[test]
    fn ecies_roundtrip() {
        let sk = SecretKey::random(&mut OsRng);
        let pk_bytes = sk.public_key().to_encoded_point(false).as_bytes().to_vec();
        let sk_bytes: [u8; 32] = sk.to_bytes().into();

        let ranking = vec!["alice".to_owned(), "bob".to_owned(), "carol".to_owned()];
        let plaintext = to_vec(&BorshRanking(ranking.clone())).unwrap();
        let ciphertext = encrypt(&pk_bytes, &plaintext).unwrap();
        let recovered = decrypt(&sk_bytes, &ciphertext).unwrap();
        let BorshRanking(decoded) = borsh_from_slice(&recovered).unwrap();
        assert_eq!(decoded, ranking);
    }
}
