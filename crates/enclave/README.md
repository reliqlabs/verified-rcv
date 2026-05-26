# verified-rcv-enclave runtime crate

Runtime wrapper for verified-rcv. Hosts the Stage 1 `decrypt_and_validate`
implementation (the ECIES decryptor that feeds the Aeneas-extractable
`verified-rcv-enclave-core::irv_spec`), the dstack TDX integration, and the
Quartz attestation envelope construction.

`src/main.rs` is currently a stub. The Stage 1 decoder lives here because
it depends on `ecies` and `k256` (system crypto), which the
`verified-rcv-enclave-core` crate intentionally excludes so charon can walk
it.

## Ballot wire format (pre-implementation pin)

This is the contract between the on-chain UI encryptor and the enclave-side
decryptor. Anything that disagrees with this gets dropped at Stage 1.
Reference: `docs/ui-integration.md` pre-implementation checklist item #1.

### Layer 1: plaintext = Borsh-encoded `Vec<Addr>`

The ballot plaintext is a Borsh-serialized `Vec<String>`, where each
`String` is the candidate's bech32 address.

```
plaintext :=
  u32 LE  // n = ranking.len()
  repeat n times:
    u32 LE  // addr_len = utf8_bytes(addr).len()
    [u8; addr_len]  // utf8 bytes of bech32 addr
```

Borsh derives this for `Vec<String>` per the Borsh spec
(https://borsh.io/) and crate `borsh = "1.5"`. The decoder MUST use the
exact same crate version family or a wire-compatible one.

### Layer 2: ciphertext = ecies-rs 0.2.11 with `pure` feature

`crates/enclave/Cargo.toml` pins `ecies = { version = "0.2",
default-features = false, features = ["pure"] }` which resolves to v0.2.11.
The `pure` feature enables `aes-rust` (pure-Rust AES-256-GCM) and leaves
`aes-short-nonce` and `xchacha20` off. The global `ECIES_CONFIG` is at its
default (`is_ephemeral_key_compressed = false`,
`is_hkdf_key_compressed = false`) — the enclave runtime MUST NOT call
`ecies::config::update_config`.

The resulting wire format is the concatenation:

```
ciphertext :=
  ephemeral_pubkey  // 65 bytes, uncompressed SEC1 (0x04 || X || Y)
  nonce             // 16 bytes, random per encryption
  tag               // 16 bytes, AES-256-GCM authentication tag
  body              // N bytes, AES-256-GCM ciphertext of `plaintext`
```

Total length: `97 + N` bytes, where `N` is the Borsh plaintext length.

Symmetric-key derivation:

- ECDH: `shared_point = receiver_pubkey * ephemeral_secret` (scalar mul on
  secp256k1).
- KDF: `HKDF-SHA256(IKM, salt=None, info=empty, L=32)` where
  `IKM = uncompressed_ephemeral_pubkey (65 bytes) ||
          uncompressed_shared_point (65 bytes)` (130 bytes total).
- AEAD: `AES-256-GCM` with the 32-byte HKDF output as key, random 16-byte
  nonce, empty associated data, 16-byte tag. The order of fields in
  `ecies::symmetric::aead::encrypt` is `nonce || tag || ciphertext` — note
  the tag comes *before* the body, not after.

### Receiver public key provisioning

The receiver public key is the per-election enclave pubkey stored on the
contract's `Election` struct as the `enclave_pubkey: HexBinary` field
(landed 2026-05-26 per UI-integration feedback). The UI fetches it via the
existing `Election` query and ECIES-encrypts under it.

The admin obtains the pubkey from dstack KMS before calling `CreateElection`
— dstack-KMS-derived keys per intent §6.3, where the key derivation is
deterministic over `(contract_addr, election_id)` so the pubkey is known
before the enclave actually runs. The corresponding privkey is released by
dstack only after the enclave attests at tally time.

For development against the Mock attestation variant, the admin can pass
any keypair as long as the UI's encryption side and the roundtrip binary's
`--privkey-hex` side are a matching pair. The contract does not verify the
shape of `enclave_pubkey` — it's an opaque `HexBinary` until decryption.

## Roundtrip test binary

`cargo run --bin verified-rcv-roundtrip -- decrypt
  --privkey-hex <64 hex chars> --ciphertext-hex <hex>`

prints the decrypted Borsh `Vec<String>` ranking as a JSON array on stdout.
Used by `ui/test/encryption.test.ts` to validate the JS-side encryptor
matches this crate's decoder exactly.

`cargo run --bin verified-rcv-roundtrip -- keygen` prints a random
`(privkey_hex, pubkey_uncompressed_hex)` pair for fixture use. The output
pubkey is 130 hex characters (65 bytes uncompressed SEC1).

`cargo run --bin verified-rcv-roundtrip -- encrypt
  --pubkey-hex <130 hex chars> --ranking <addr1>,<addr2>,...`
performs Borsh-encode + ECIES-encrypt and prints the wire-format hex.

## Building

```
cargo build -p verified-rcv-enclave
cargo test -p verified-rcv-enclave
```

This crate is **not** part of the wasm contract build. The contract crate
(`crates/contract/`) compiles independently with its own (cosmwasm-std)
dependency set.
