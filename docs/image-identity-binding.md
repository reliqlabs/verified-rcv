# Image-identity-binding artifact

This is the **build-pipeline contract** that ties the enclave crate's source code to the on-chain `EnclaveImageRegistry { mrtd, rtmr, vkey }` values. Without this binding, the entire B10 trust chain is decorative: an attestation could be cryptographically valid but bind to a different binary than the one whose Lean proofs we trust.

Status: **structural plan documented (this doc); reproducible build operational; (mrtd, rtmr) computation requires dstack-TDX environment (not yet wired)**.

## What the binding establishes

| Link | Claim | Established by |
|---|---|---|
| Source → binary | `crates/enclave/` source code at commit `HEAD` produces a specific binary `verified-rcv-enclave` with a deterministic SHA-256 hash. | Reproducible build (Dockerfile + cargo-lock-pinned + rustc-version-pinned). |
| Binary → measured identity | The TDX boot of that binary produces a specific `(mrtd, rtmr)` measurement tuple. | dstack TDX boot record; computed once at deployment and cross-verified by independent operators. |
| Measured identity → chain registry | The `EnclaveImageRegistry { mrtd, rtmr, vkey }` at the contract's `INSTANTIATE` matches the measured tuple from the prior step. | Operator's responsibility at instantiate-time. The chain has no on-chain verification path; integrity is operational (`image_registration_honest` per intent §6.1). |
| Chain registry → attested tally | Block 6's `PublishResult` checks the attestation envelope's MRTD/RTMR equal the registry. | On-chain check at `PublishResult`; B8 attestation chain witnesses. |

This document covers the **first two links**. The third is operational (intent §6.1 `image_registration_honest`). The fourth is implemented in `crates/contract/`.

## Reproducible build (Link 1)

The goal: anyone with the source tree at commit `HEAD` can reproduce a byte-identical `verified-rcv-enclave` binary.

### Toolchain pinning

- **Rust toolchain**: pin in `rust-toolchain.toml` at the repo root. Current pin: see file. Use a stable Rust version (the verus-toolchain `1.95.0` is fine for build; verus annotations are not used in production builds).
- **Cargo.lock**: pin at repo root, committed. `cargo build` MUST use `--locked` to refuse to update dependencies.
- **System dependencies**: avoid linking against system libraries other than `libc`. The `ecies` crate's `pure+std` feature set is pure-Rust; `k256` is pure-Rust. No OpenSSL, no Security.framework, no platform-specific crypto.

### Build environment (Dockerfile)

```dockerfile
# verified-rcv reproducible build environment.
# Use this to produce a deterministic binary that matches the on-chain
# (mrtd, rtmr) measurement.

FROM rust:1.95.0-bookworm AS builder

# Pin system dependencies. apt-get layer with --no-install-recommends
# and locked timestamps for full determinism.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      pkg-config \
      libssl-dev=3.0.* \
      ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

# Build with --locked to refuse dependency updates; --release for the
# canonical binary; reproducible-build flags to strip absolute paths and
# disable parallelism-induced reordering.
ENV CARGO_BUILD_JOBS=1
ENV RUSTFLAGS="--remap-path-prefix=/build=/ -C target-cpu=x86-64 -C debuginfo=0"
ENV SOURCE_DATE_EPOCH=1735689600

RUN cargo build --locked --release -p verified-rcv-enclave --bin verified-rcv-enclave

# Output the binary + its hash. Reproducible builds verify against this hash.
RUN sha256sum target/release/verified-rcv-enclave > target/release/verified-rcv-enclave.sha256

FROM scratch AS artifact
COPY --from=builder /build/target/release/verified-rcv-enclave /verified-rcv-enclave
COPY --from=builder /build/target/release/verified-rcv-enclave.sha256 /verified-rcv-enclave.sha256
```

### Verification recipe

```bash
# Build the binary twice in clean environments; verify hashes match.
docker build --no-cache -t verified-rcv-build .
docker run --rm verified-rcv-build sha256sum /verified-rcv-enclave > hash-1.txt

# In a different machine / different time / different cache state:
docker build --no-cache -t verified-rcv-build .
docker run --rm verified-rcv-build sha256sum /verified-rcv-enclave > hash-2.txt

# These two MUST match.
diff hash-1.txt hash-2.txt
```

If `diff` shows a difference, the build is not reproducible. Common causes:
- Absolute paths leaking into the binary (fix: `--remap-path-prefix`).
- Build timestamps (fix: `SOURCE_DATE_EPOCH`).
- Parallelism-introduced nondeterminism in linker output (fix: `CARGO_BUILD_JOBS=1`).
- System-dependency drift (fix: pin `libssl-dev` version explicitly).

## TDX measurement (Link 2)

The dstack TDX boot computes:
- **MRTD** (Measurement Register, Trust Domain): the SHA-384 of the TDX-VM's initial memory image. Fixed at boot.
- **RTMR** (Runtime Measurement Register): a chain of measurements of runtime events. Index 0 is the BIOS/firmware; the binary's specific measurement extends into a chosen RTMR.

The procedure:
1. Build the binary reproducibly (Link 1).
2. Wrap it in a TDX-compatible boot image (dstack provides this; in practice it's a dstack TDX VM template).
3. Boot the VM under dstack; capture the produced TDX quote.
4. Extract `(mrtd, rtmr)` from the quote.

**This requires dstack-TDX hardware or a verified TDX simulator.** Without that, this document describes the protocol but cannot exercise it.

### Computation script (sketch, requires dstack-TDX runtime)

```bash
# Inside the dstack-TDX VM:
dstack-tdx-tool quote --user-data $(echo -n "verified-rcv-image-attestation" | sha256sum | head -c 64) \
  > attestation-quote.bin

# Outside the VM (on the host that holds the quote):
dstack-tdx-tool decode-quote attestation-quote.bin --field mrtd > mrtd.hex
dstack-tdx-tool decode-quote attestation-quote.bin --field rtmr1 > rtmr.hex

cat mrtd.hex rtmr.hex
```

The output (a 32-byte hex MRTD + a 32-byte hex RTMR) goes into the `InstantiateMsg.registry` field per `docs/deploy.md`.

## On-chain registration (Link 3)

The values from Link 2 are passed to the contract's `instantiate` handler. The contract STORES them but does NOT verify them — verification is operational, per intent §6.1's `image_registration_honest` predicate.

To make the registration audit-able:
1. Publish the source commit hash that the binary was built from.
2. Publish the Dockerfile + `Cargo.lock` used.
3. Publish the SHA-256 of the binary (from Link 1's reproducible build).
4. Publish the captured TDX quote (Link 2 output).
5. Publish the `(mrtd, rtmr)` values that match the quote.

Any auditor can then:
- Verify the Dockerfile is the one in the published source tree.
- Reproduce the binary locally and check the SHA-256 matches.
- Verify the TDX quote is well-formed and binds to the published MRTD/RTMR.
- Verify the on-chain registry matches the published `(mrtd, rtmr)`.

A mismatch at any step breaks `image_registration_honest` and falls into failure mode 4.3b (vkey/identity substitution) per intent §4.

## What this binding does NOT establish

- It does NOT establish that the binary's source code is correct. That's what the formal verification (intent → spec → math `IRV_spec` → `irv_spec` extracted → bridge proof) is for.
- It does NOT establish that the chain registry is honest. That's the `image_registration_honest` operational assumption.
- It does NOT establish that the dstack KMS released the privkey only to the matching enclave. That's the `dstack_kms_trust` operational axiom (intent §6.3).

The image-identity binding closes the source → measured-identity gap. The other gaps live elsewhere in the trust chain.

## Round 3f deliverables

When this artifact lands as part of Round 3f composition:
1. Dockerfile committed to repo root (or `infra/`).
2. `rust-toolchain.toml` committed.
3. Reproducible build verification CI step (two clean-room builds + diff).
4. Reference TDX boot recipe (assumes dstack-TDX hardware).
5. Documented (mrtd, rtmr) for the audit-public verified-rcv-enclave-v1 build.
6. Update intent §6.1's `image_registration_honest` predicate to cite this doc's verification procedure.

## Honest disclosure

Until the Dockerfile and the actual TDX boot are exercised on real dstack hardware, **the binding is a documented protocol, not a verified one**. Any production deployment SHOULD have an independent operator group verify the procedure end-to-end before treating the on-chain registry as trustworthy.
