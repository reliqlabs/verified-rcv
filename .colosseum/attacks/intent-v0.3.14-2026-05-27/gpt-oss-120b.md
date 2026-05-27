# gpt-oss-120b — v0.3.14 review

## Attacks

### 1. under-spec — Names-hash preimage collisions [critical]
- **Category**: under-spec
- **Affected**: §2 (canonical_serialization)
- **What's wrong**: The spec defines the names-hash preimage as `u32_LE(N) ‖ Borsh(names[0]) ‖ … ‖ Borsh(names[N-1])` but never specifies handling of overflow or endianness for length fields inside each `Borsh(String)`. An attacker could craft two distinct name-sets whose concatenated length fields overflow 32-bit unsigned integers, yielding identical byte streams and thus identical `names_hash`.
- **Cite**: "`names_hash = SHA-256(u32_LE(N) ‖ Borsh(names[0]) ‖ … ‖ Borsh(names[N-1]))`"
- **Severity**: critical (per voice; **synthesis dispute**: u32 is exactly 32 bits with max 2^32 ≈ 4B values; with 64-byte name cap + chain gas limits, neither N nor len_i can approach 2^32. The injectivity-by-prefix-coding property of Borsh strings (Borsh's well-defined canonical encoding) makes the preimage injective given valid inputs. Likely false positive; worth one-line explicit bound statement to close the lens.)
- **Fix**: state explicitly that N ≤ contract candidate cap (e.g., ≤ 1000) and each name has len ≤ 64; cite Borsh's canonical-encoding injectivity property to make the lens-output mechanical.

### 2. coverage-gap — Registration does not bind `names_hash` [critical]
- **Category**: coverage-gap
- **Affected**: Registration ReportData
- **What's wrong**: ReportData for `CreateElection` binds only `(enclave_pubkey, contract_addr, election_id)`. Does NOT bind `names_hash`. A malicious host could alter `candidate_names` after registration without any on-chain evidence at registration-time.
- **Severity**: critical (per voice; **synthesis dispute**: at PublishResult the chain reads stored `candidate_names` from ELECTION storage and computes names_hash from there, then compares to runtime-supplied names_hash via the attestation. B11 makes stored names immutable. So host tampering between CreateElection and PublishResult is caught by the publish-side binding. The asymmetry is real but defense-in-depth, NOT critical: registration-time evidence would be a stronger property — catch tampering earlier — but the publish-time check is sufficient for the on-chain integrity property. Cross-voice convergence: Claude crypto-voice + nemotron + gpt-oss all surfaced this same shape. MEDIUM-MAJOR by panel consensus.)
- **Fix**: extend registration ReportData binding from `(enclave_pubkey, contract_addr, election_id)` to `(enclave_pubkey, contract_addr, election_id, names_hash)`. Defense-in-depth; preserves the audit-clear discipline of v0.3.12 N22 (which made registration triple-bind to prevent leaked-pubkey replay).

### 3. composition-failure — B11 immutability enforced only by missing handler [serious]
- **Category**: composition-failure
- **Affected**: §3.2 B11 description
- **What's wrong**: Immutability is guaranteed solely because the contract has no `UpdateNames` handler (structural). Fragile to future migration / contract upgrade.
- **Cite**: "B11 immutability: structural (no UpdateNames handler)"
- **Severity**: serious
- **Fix**: encode an explicit `names_locked: bool` flag set true at CreateElection and required true by all post-create paths. Any future migration must preserve this flag. Alternatively, document the migration discipline as a §6.x trust-boundary clause.

### 4. denial-of-service — Host can permanently block publication by lying about names [serious]
- **Category**: denial-of-service / edge-case
- **Affected**: liveness of publication
- **What's wrong**: Host supplies candidate names to enclave. If host lies, `names_hash` mismatches stored value, publish rejected. Indefinite stall.
- **Severity**: serious (per voice; **synthesis note**: this is the same DoS surface as a host that simply doesn't run the enclave at all — covered by failure mode 4.9 (deterministic-malformed-tally deadlock) in intent §4. Not a new attack class. Mitigation is operational: replace host operator.)
- **Fix**: panel recommends documenting under failure mode 4.9 as a sub-case rather than inventing a new mitigation.

## Summary
- Critical (per voice): 2 (1 disputed false positive, 1 cross-voice-confirmed but downgraded to MAJOR by synthesis panel)
- Serious: 2 (1 new finding — B11 structural fragility, 1 already-covered DoS variant)
