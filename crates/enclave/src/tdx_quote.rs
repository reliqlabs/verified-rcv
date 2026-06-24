//! Minimal TDX quote v4 measurement extractor.
//!
//! Reads `MRTD` and `RTMR0..3` from a signed TDX quote at fixed byte
//! offsets. Does NOT cryptographically verify the quote (that's the
//! chain-side `xion.zk.v1.Query/ProofVerifyUltraHonk` path, plus the
//! operator's Intel-PCS cross-check via dstack's verifier). The values
//! returned here populate the `HealthResponse.mrtd_hex` / `rtmr*_hex`
//! fields so operators can read the deployed image's identity for the
//! on-chain registry (intent §6.1 image-identity-binding policy).
//!
//! Offsets pinned at the dcap-noir TD report layout
//! (TDX 1.0 / TD report v1.0 quote v4 layout). TDX 1.5 quote v5 has a
//! different layout; the version check rejects it so we don't surface
//! garbage measurements.
//!
//! Why inline parser instead of dcap-qvl: extracting raw measurement
//! bytes at known offsets has no cert chain or collateral dependency.
//! Pulling dcap-qvl in for this would add transitive deps the runtime
//! doesn't otherwise need (zkdcap-host uses dcap-qvl from the
//! `feat/cosmwasm` branch of a reliqlabs fork — a heavier path than is
//! warranted for "read 5 byte arrays at fixed offsets").

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("quote too short ({0} < 632 bytes)")]
    TooShort(usize),
    #[error("quote header version {0} is not 4 (TDX 1.0); refusing to parse")]
    UnsupportedVersion(u16),
    #[error("quote tee_type 0x{0:08x} is not TDX (0x00000081)")]
    NotTdx(u32),
}

/// Parsed measurements from a TDX 1.0 signed quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measurements {
    pub mrtd: [u8; 48],
    pub rtmr0: [u8; 48],
    pub rtmr1: [u8; 48],
    pub rtmr2: [u8; 48],
    pub rtmr3: [u8; 48],
}

/// Offsets within a signed TDX 1.0 quote (header[0..48] + TD report).
const SQ_MR_TD: usize = 184;
const SQ_RTMR0: usize = 376;
const SQ_RTMR1: usize = 424;
const SQ_RTMR2: usize = 472;
const SQ_RTMR3: usize = 520;
const SQ_MIN_LEN: usize = 632;

/// Extract `(MRTD, RTMR0..3)` from a signed TDX 1.0 quote. Returns
/// `Err` on malformed / unsupported quotes — the caller should treat
/// those as "no measurements available" (HealthResponse → ready=false).
pub fn parse_measurements(quote: &[u8]) -> Result<Measurements, ParseError> {
    if quote.len() < SQ_MIN_LEN {
        return Err(ParseError::TooShort(quote.len()));
    }

    let version = u16::from_le_bytes([quote[0], quote[1]]);
    if version != 4 {
        return Err(ParseError::UnsupportedVersion(version));
    }
    // tee_type at offset 4 (u32 LE) per Intel TDX Quote Format §3.2.
    // 0x00000081 = TDX (vs 0x00000000 = SGX).
    let tee_type = u32::from_le_bytes([quote[4], quote[5], quote[6], quote[7]]);
    if tee_type != 0x81 {
        return Err(ParseError::NotTdx(tee_type));
    }

    let mut mrtd = [0u8; 48];
    mrtd.copy_from_slice(&quote[SQ_MR_TD..SQ_MR_TD + 48]);
    let mut rtmr0 = [0u8; 48];
    rtmr0.copy_from_slice(&quote[SQ_RTMR0..SQ_RTMR0 + 48]);
    let mut rtmr1 = [0u8; 48];
    rtmr1.copy_from_slice(&quote[SQ_RTMR1..SQ_RTMR1 + 48]);
    let mut rtmr2 = [0u8; 48];
    rtmr2.copy_from_slice(&quote[SQ_RTMR2..SQ_RTMR2 + 48]);
    let mut rtmr3 = [0u8; 48];
    rtmr3.copy_from_slice(&quote[SQ_RTMR3..SQ_RTMR3 + 48]);

    Ok(Measurements { mrtd, rtmr0, rtmr1, rtmr2, rtmr3 })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid TDX 1.0 quote stub with the requested
    /// measurement bytes at the canonical offsets. Everything else is
    /// zero. Used by tests to exercise the parser without pulling in a
    /// real Phala-signed quote.
    fn stub_quote(mrtd: u8, rtmr0: u8) -> Vec<u8> {
        let mut q = vec![0u8; SQ_MIN_LEN];
        q[0] = 4; // version u16 LE
        q[1] = 0;
        q[4] = 0x81; // tee_type u32 LE
        for b in &mut q[SQ_MR_TD..SQ_MR_TD + 48] {
            *b = mrtd;
        }
        for b in &mut q[SQ_RTMR0..SQ_RTMR0 + 48] {
            *b = rtmr0;
        }
        q
    }

    #[test]
    fn parses_valid_stub() {
        let q = stub_quote(0xAB, 0xCD);
        let m = parse_measurements(&q).expect("valid stub parses");
        assert_eq!(m.mrtd, [0xAB; 48]);
        assert_eq!(m.rtmr0, [0xCD; 48]);
        assert_eq!(m.rtmr1, [0; 48]);
    }

    #[test]
    fn rejects_short_quote() {
        let q = vec![0u8; 100];
        match parse_measurements(&q) {
            Err(ParseError::TooShort(100)) => {}
            other => panic!("expected TooShort(100), got {other:?}"),
        }
    }

    #[test]
    fn rejects_wrong_version() {
        let mut q = stub_quote(0, 0);
        q[0] = 5;
        match parse_measurements(&q) {
            Err(ParseError::UnsupportedVersion(5)) => {}
            other => panic!("expected UnsupportedVersion(5), got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_tdx() {
        let mut q = stub_quote(0, 0);
        q[4] = 0; // sgx-style tee_type
        match parse_measurements(&q) {
            Err(ParseError::NotTdx(0)) => {}
            other => panic!("expected NotTdx(0), got {other:?}"),
        }
    }

    #[test]
    fn rejects_simulator_mock_quote() {
        // The DSTACK_SIMULATOR mock quote is 112 bytes (magic 16 +
        // report_data 64 + sha256 32). Parser must reject without panic
        // so the server falls back to ready=false.
        let mut q = Vec::from(b"DSTACK_MOCK_QV01" as &[u8]);
        q.extend_from_slice(&[0u8; 64]);
        q.extend_from_slice(&[0u8; 32]);
        assert_eq!(q.len(), 112);
        assert!(matches!(
            parse_measurements(&q),
            Err(ParseError::TooShort(112))
        ));
    }
}
