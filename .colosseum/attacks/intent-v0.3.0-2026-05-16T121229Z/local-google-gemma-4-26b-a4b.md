# google/gemma-4-26b-a4b (local) — verified-rcv revised intent adversarial pass

- **Elapsed**: 280.7s
- **Finish reason**: stop
- **Usage**: {"prompt_tokens": 27449, "completion_tokens": 13984, "total_tokens": 41433, "completion_tokens_details": {"reasoning_tokens": 12518}}

---

VERDICT: BREAKS

### 1. SemVer Contradiction
**Category**: Contradiction  
**Severity**: cosmetic  
**Affected location**: Revision history (v0.3.0) vs. SemVer header  
**What's wrong**: The document classifies the v0.3.0 bump as a **MAJOR** revision. However, according to the document's own SemVer rules in the header, a MAJOR bump is reserved for when an "invariant [is] removed / weakened / trust boundary widened / behavior previously specified becomes unspecified." The changes in v0.3.0 (B6 re-formulation, B10 addition, B9 conditioning) are all **strengthenings** or **clarifications** of existing invariants. Under the provided rules, these should be classified as **MINOR** (new invariant / trust boundary narrowed).  
**Cite of intent-doc text**: 
- Header: `MAJOR (0.x.0): invariant removed / weakened / trust boundary widened...`
- Revision history: `v0.3.0 — 2026-05-15 MAJOR: revision against Round 3a re-adversarial fan-out...`

### 2. Borsh Map Serialization Ambiguity
**Category**: Under-specification  
**Severity**: serious  
**Affected location**: Section 2.5 (Canonical serialization)  
**What's wrong**: The spec relies on `canonical_serialization(x)` being deterministic to satisfy B8(c). For the `TallyResult` struct, it specifies that `Map<Addr, Nat>` entries are emitted in "candidate-declaration order." However, Borsh does not have a native `Map` type; it serializes collections as `Vec<(K, V)>`. The spec fails to define the transformation required to turn a logical `Map` into a Borsh-compatible `Vec`. If two implementations use different internal sorting or conversion logic (e.g., one converts the Map to a Vec and then sorts it, while another relies on an implicit iteration order), they will produce different byte streams for the same logical `TallyResult`, causing B8(c) to fail and breaking the attestation binding.  
**Cite of intent-doc text**: 
- Section 2.5: "`Map<Addr, Nat>` entries are emitted in **candidate-declaration order** (matching the ordering discipline above), making `borsh_bytes(tally_body)` a deterministic function of the logical value."

### 3. SHA-256 Concatenation Ambiguity
**Category**: Under-specification  
**Severity**: serious  
**Affected location**: Section 3.2 (B8 clause c) and Section 2.5 (Canonical serialization)  
**What's wrong**: B8(c) defines the binding as `SHA-256(canonical_serialization(contract_addr ‖ tally_body))`. The spec defines `‖` as byte concatenation of Borsh-encoded components. However, it does not specify the type of `contract_addr`. If `contract_addr` is treated as a raw byte array (e.g., `[u8; 32]`) without a Borsh-encoded length prefix, the concatenation is vulnerable to collision attacks where bytes can be shifted from the end of the address into the start of the `tally_body`. While Borsh-encoding a `Vec<u8>` would include a length prefix, the spec is silent on whether `contract_addr` is serialized as a fixed-size array or a variable-length vector.  
**Cite of intent-doc text**: 
- Section 3.2 (B8): `(c) attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`
- Section 2.5: "`canonical_serialization(contract_addr ‖ tally_body)` denotes `borsh_bytes(contract_addr) ‖ borsh_bytes(tally_body)`"

### 4. Quartz Inheritance Noise (Composition Failure)
**Category**: Composition failure  
**Severity**: cosmetic  
**Affected location**: Section 6.2 (Quartz inheritance table)  
**What's wrong**: The inheritance table includes a row for `commitHashE`, but explicitly notes it is "NOT CONSUMED by verified-rcv." This creates a "ghost dependency" in the composition model. An auditor or automated tool reviewing the inheritance tree would see a dependency on Quartz's `commitHashE` (which has a specific pigeonhole-impossible property), whereas the system actually relies on a local SHA-256 implementation (`Adv_commitTally_CR`). Including unused primitives in a formal inheritance table is a failure of the composition discipline and introduces noise that can lead to incorrect security reasoning.  
**Cite of intent-doc text**: 
- Section 6.2: `commitHashE ... NOT CONSUMED by verified-rcv — see Note A below`

### 5. Image-Identity Binding Gap
**Category**: Refinement mismatch  
**Severity**: serious  
**Affected location**: Section 8.7 (B10 witness) and Section 3.2 (B10)  
**What's wrong**: The spec claims B10 is satisfied by the composition `B10_lean ∧ image-identity-binding ∧ B8`. However, `image-identity-binding` is defined as an "obligation" that lives in the "build pipeline + deployment ledger layer," not a formal invariant within the specification. There is no spec-level mechanism (invariant or property) that captures the integrity of the link between the Lean-proven model and the actual binary deployed on-chain. This creates a refinement gap: you can prove `B10_lean` in Lean, but the "correctness" of B10 as a system property is entirely dependent on an unmodeled, non-formalized DevOps process.  
**Cite of intent-doc text**: 
- Section 8.7: "This is an **extraction-soundness + build-reproducibility** obligation that lives at the **build pipeline + deployment ledger layer**, not the Lean layer."

***

**META**
- **Categories attacked**: Contradiction, Under-specification, Composition failure, Refinement mismatch.
- **Categories not attacked**: Triviality (found B10 to be a valid, if non-verifiable, obligation), Temporal-state mismatch (B6 and S5 were correctly tagged), Impossibility-hypothesis-vacuity (B9 conditioning was found to be a valid conditional).
- **Artifacts wanted**: The specific Borsh implementation/library intended for use (to check Map serialization), the type definition of `contract_addr` in the contract code.
- **Estimated confidence**: High