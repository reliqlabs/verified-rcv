# qwen3.6-27b — slice scope — System scope, identity, and external contracts

## Cross-section reads
- 2.5: For context on B10's reliance on `Tally_spec`.
- 3.2: For context on B8/B9 security reduction structure.
- 4.3b, 4.4, 4.9: For context on integrity/liveness failure modes mentioned in scope.
- 6.1, 6.2: For context on registry/inheritance trust boundaries.

## Attacks on this slice

### 1. Identity Overreach in System Scope [serious]
- **Category**: `under-specification`
- **Affected**: ## 1. System Identity, line 21
- **What's wrong**: The scope claims to specify "protocol-level behavior across all three components" (contract, enclave, zkdcap). However, the actual specification for the "zkdcap attestation" component is entirely outsourced to Quartz axioms in Section 6.2, with no primitive behavior or interface for the zkdcap component itself defined in this document. A security property that relies on a third-party primitive (zkdcap) without defining the interface/contract between the enclave's output and the zkdcap provider constitutes an identity overreach: the system claims to encompass a component it does not actually specify.
- **Cite**: > - **Scope:** end-to-end system spanning (i) a CosmWasm contract..., (ii) a dstack-TDX enclave..., (iii) zkdcap attestation of the enclave's tally output. Specifies protocol-level behavior across all three components.
- **Fix recommendation**: Explicitly state that the zkdcap component's behavior is inherited from Quartz and that this spec only defines the *binding* requirements for its use, rather than claiming to specify its internal protocol-level behavior.

### 2. Vulnerable Instantiation: Zero-Candidate/Empty-Set Exploitation [critical]
- **Category**: `coverage gap`
- **Affected**: ## 1. System Identity, line 23; ## 5. Non-Goals, line 452
- **What's wrong**: Section 1 defines the electorate as "the candidate set itself" and states that anyone may instantiate a contract, with legitimacy being an off-chain concern. Section 5 explicitly states there is "No on-chain candidate-consent gate." Combined with Block 1's requirement in Section 2.5 that `len(candidates) ≥ 1`, an attacker can instantiate a contract with a single, attacker-controlled candidate address. Since "anyone may instantiate" and there is no quorum/consent gate, an attacker can effectively manufacture a "valid" election where they are the sole voter/winner, potentially for use in phishing or reputation-spoofing attacks on users who trust the "verified-rcv" brand, even if they don't trust a specific instance. The spec fails to address the security implications of an un-gated, single-candidate instantiation in its scope/identity definitions.
- **Cite**: > - **Electorate:** the candidate set itself — only declared candidates may submit ballots. Anyone may instantiate a contract; legitimacy of any given instance is off-chain.
- **Fix recommendation**: While maintaining the design, the scope/identity section should explicitly acknowledge that "contract legitimacy" includes a risk of "singleton elections" and define the security boundary for such instances (e.g., that they are mathematically valid but socially/economically untrusted).

### 3. Undefined "Legitimacy" Boundary for External Contracts [serious]
- **Category**: `ambiguity`
- **Affected**: ## 1. System Identity, line 23; ### 6.4 Caller contract, line 524
- **What's wrong**: The spec repeatedly uses the term "legitimacy" (e.g., "legitimacy of any given instance is off-chain") but fails to define what constitutes a "legitimate" instantiator versus an attacker in the context of system security. Section 6.4 states the instantiator "is trusted to declare candidates honestly," but in a decentralized/public chain context, "honesty" is not an assumption one can make for security properties. This creates a major ambiguity: does "legitimacy" imply that the system's security guarantees (like B8/B9) are only valid for "legitimate" elections, or does the system provide security even for a malicious instantiator? If it's the latter, "honesty" in 6.4 is a misleading/weak term; if it's the former, the security boundary for B8/B9 is ill-defined.
- **Cite**: > - **Electorate:** ...anyone may instantiate a contract; legitimacy of any given instance is off-chain.
- **Fix recommendation**: Replace "legitimacy" with specific security properties: e.g., "The system provides integrity/correctness guarantees for *any* instantiable election, regardless of the instantiator's intent, provided Block 1 requirements are met."

### 4. Semantic Gap in "Non-Goals" for Operational Viability [cosmetic]
- **Category**: `under-specification`
- **Affected**: ## 5. Non-Goals, line 463
- **What's wrong**: Section 5 admits "No on-chain operational-viability checks" and that instantiators are responsible for window length/candidate count. However, it does not specify the *upper bounds* that would make an election "operationally impossible" versus merely "sub-optimal." For example, a window of 1 block or $10^6$ candidates is admitted by Block 1, even though these would practically fail due to chain gas limits or time constraints. While Section 7 discusses gas, the "Non-Goals" section should more clearly distinguish between *contract security* (which is in scope) and *operational usefulness* (which is not).
- **Cite**: > - **No on-chain operational-viability checks** (e.g., minimum voting window). All instantiations satisfying Block 1's Requires are admitted...
- **Fix recommendation**: Clarify that "operational viability" refers to parameters that do not violate security invariants but may result in practical execution failure (e.g., gas exhaustion).

## Slice-local summary
- Critical: 1
- Serious: 2
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE