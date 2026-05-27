# nemotron-3-120b-a12b — v0.3.14 review

**NOTE**: response truncated at 3000 tokens (finish_reason=length). 3 attacks visible; #3 partial.

## Attacks

### 1. Commitment-coverage gap: `names_hash` does not bind candidate addresses
- **Category**: coverage gap
- **Affected**: §6.4 (trust model), §8.7 link 7 annotation, state field `candidate_names`
- **Severity**: serious (per voice; reviewer note: see synthesis — partially mitigated by ballots_hash + tally_body declaration-order binding)
- **What's wrong**: `names_hash` hashes only the name strings; the parallel-indexed `candidates: Vec<Addr>` is never included in any cryptographic commitment specifically alongside the names. Consequently the *pairing* (addr_i, name_i) is bound only indirectly (via ballots_hash declaration-order and tally_body address-keyed maps). The voice argues a "same set of names paired with two different address vectors" equivocation. Synthesis note: within one election (one CreateElection) the pairing is fixed in storage, so this is bounded by §6.4 off-chain-trust scope rather than a chain-level vulnerability. Still, an explicit `candidates_hash` (or fused `(addr, name)_hash`) would make the binding inspectable and not rely on the declaration-order discipline of separate commitments.
- **Cite**:
  - "New state field `candidate_names: Vec<String>` parallel-indexed to `candidates: Vec<Addr>`."
  - "§8.7 link 7 annotation extended: input-fidelity covers (ballots_hash, names_hash) tuple."
- **Fix recommendation**: introduce `candidates_hash = SHA-256(u32_LE(N) ‖ for i: Borsh(candidates[i]))` OR fuse names with addresses into one hash `pairs_hash = SHA-256(u32_LE(N) ‖ for i: Borsh(candidates[i]) ‖ Borsh(candidate_names[i]))`. Update canonical_serialization to slot it adjacent to names_hash or replace names_hash entirely.

### 2. Missing domain separation for `names_hash` enables cross-protocol replay
- **Category**: ambiguity
- **Affected**: §8.7 link 7 annotation, canonical serialization format
- **Severity**: serious
- **What's wrong**: canonical_serialization concatenates `ballots_hash[32] ‖ names_hash[32]` without any domain-separation tag distinguishing the two fields. Although the layout is fixed at the outer canonical_serialization level, both hashes have structurally identical preimage shape `SHA-256(u32_LE(N) ‖ Borsh(...))` with no internal DST. If either hash were exposed independently (cross-protocol use, partial dump, sub-protocol), an attacker could substitute one for the other in another context. Pure structural defense-in-depth.
- **Cite**:
  - "`names_hash[32]` slot in `canonical_serialization`, between `ballots_hash` and `tally_body` ..."
- **Fix recommendation**: prefix each internal hash preimage with a distinct DST byte/string (e.g., `0x01` or `"verified-rcv:ballots:v1"` for ballots_hash; `0x02` or `"verified-rcv:names:v1"` for names_hash). Removes future foot-gun.

### 3. Registration ReportData omits `names_hash`, creating a trust-anchor mismatch
- **Category**: precondition over-strength (partial, truncated)
- **Affected**: §6.4 (trust model), Registration ReportData description, B11 immutability clause
- **Severity**: serious
- **What's wrong**: registration binds `(enclave_pubkey, contract_addr, election_id)` but deliberately excludes `names_hash`. While B11 makes `candidate_names` immutable post-CreateElection, the registration quote does NOT attest to having seen the names. An attacker who obtains a valid registration quote can [TRUNCATED — voice ran out of token budget]
- **Cite**: [partial]
- **Fix recommendation**: [partial — likely "extend registration ReportData to include names_hash" — synthesis should treat as same shape as Claude voice's M1]

## Summary (partial)
- Serious: 3 (commitment-coverage on addresses, DST separation, registration-names asymmetry)
