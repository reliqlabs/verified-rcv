# kimi-k2-6 — slice scope

## Cross-section reads
- Section 3.2 B9 — to verify `image_registration_honest` precondition exists and is load-bearing (grounds Attack 1).
- Section 2.5 (Block 6 well-formedness vs B10 correctness note) — to confirm well-formedness is explicitly syntactic, not semantic (grounds Attack 4).
- Section 4.3b and 4.4 — to confirm vkey-substitution and enclave-software-bug failure modes are in-scope and undetectable on-chain (grounds Attacks 1 and 4).
- Section 2.2 boundary case "All candidates abstain" — to confirm zero-ballot elections produce "practically meaningless" results that are still admitted (grounds Attack 3).

## Attacks on this slice

### 1. Output contract states categorical reliance when B9 requires `image_registration_honest` precondition [serious]
- **Category**: preconditional over-strength
- **Affected**: Section 6.6, first bullet ("`tally_result.is_some()` ⇒ a valid `DstackAttestation` was supplied... the attestation's enclave identity matched the registered verified-rcv image")
- **What's wrong**: Section 6.6 tells consumers they "can rely on" the attestation binding and enclave identity match as categorical facts. But B9 (Section 3.2) explicitly conditions the cryptographic bound on `image_registration_honest` — the operational assumption that the chain-registered vkey and MRTD/RTMR have not been substituted post-registration. If `image_registration_honest` is false (failure mode 4.3b), a malicious vkey is registered, and `publish_result` accepts an attestation from a non-verified-rcv enclave. In that case, `tally_result.is_some()` does NOT imply the attestation came from the legitimate verified-rcv image; it implies only that the attestation matched the *substituted* registration. Section 6.6 omits this precondition, overstating what output consumers can categorically rely on.
- **Cite**: > "Any party reading `tally_result` from chain state can rely on: `tally_result.is_some()` ⇒ a valid `DstackAttestation` was supplied at `publish_result` time (B8); the attestation's enclave identity matched the registered verified-rcv image"
- **Fix recommendation**: Restate the first bullet as conditional: "`tally_result.is_some()` ⇒ a valid `DstackAttestation` was supplied... **conditioned on `image_registration_honest`** (Section 6.1; if the registered vkey was maliciously substituted, this guarantee is void — see Section 4.3b)."

### 2. System identity understates off-chain scope [serious]
- **Category**: identity overreach (narrower-than-admitted)
- **Affected**: Section 1, first bullet ("Name: verified-rcv — instant-runoff voting smart contract for public CosmWasm chains")
- **What's wrong**: The name labels the system as a "smart contract," but the scope bullet immediately below admits two large off-chain components: "a dstack-TDX enclave performing tabulation" and "zkdcap attestation of the enclave's tally output." The correctness (B10) and security (B8/B9) claims are load-bearing on these off-chain components. Calling it a "smart contract" in the identity line understates the system's actual architecture and could mislead implementers, auditors, or downstream spec writers into treating the CosmWasm contract as the entire system. This is particularly dangerous because B10's discharge is explicitly cross-layer (Section 8.7) and the chain alone cannot verify tally correctness.
- **Cite**: > "Name: verified-rcv — instant-runoff voting smart contract for public CosmWasm chains" followed by "Scope: end-to-end system spanning (i) a CosmWasm contract... (ii) a dstack-TDX enclave... (iii) zkdcap attestation"
- **Fix recommendation**: Change the name bullet to "verified-rcv — instant-runoff voting system for public CosmWasm chains, comprising an on-chain election contract, a TDX enclave tabulator, and zkdcap attestation."

### 3. No operational-viability checks creates coverage gap against stated purpose [serious]
- **Category**: coverage gap
- **Affected**: Section 5, bullet "No on-chain operational-viability checks" and Section 1, purpose bullet
- **What's wrong**: Section 1 states the purpose is to "enable a public, verifiable IRV election." Section 5 explicitly excludes any check that instantiation parameters are operationally meaningful. Block 1's Requires are minimal (`len(candidates) ≥ 1`, `start_at > env.block.time`, `end_at > start_at`). This admits configurations like a 1-nanosecond voting window or a candidate set of 10,000 addresses that exceeds chain gas limits. The boundary case in Section 2.2 explicitly calls the all-abstain result "mathematically defined but practically meaningless." The non-goal therefore permits the system to "enable" elections that cannot function as elections, creating a gap between the stated purpose and the admitted behaviors.
- **Cite**: > "No on-chain operational-viability checks (e.g., minimum voting window). All instantiations satisfying Block 1's Requires are admitted; the instantiator alone is responsible for choosing operationally-meaningful parameter values" and Section 2.2 "Result is mathematically defined but practically meaningless"
- **Fix recommendation**: Either narrow the purpose statement to "provide an IRV election mechanism" (removing the implication that results are meaningful), or add a non-binding advisory note in Section 6.4 that operationally meaningless configurations are admitted and the instantiator bears full responsibility for viability.

### 4. Output contract omits the well-formedness-vs-correctness gap [serious]
- **Category**: under-specification
- **Affected**: Section 6.6, second bullet ("The tally body satisfies the well-formedness invariants S6 + S7 + S8 + S9")
- **What's wrong**: Section 6.6 tells output consumers they can rely on well-formedness invariants. But Section 2.5's "Tally-correctness obligation" paragraph explicitly states: "well-formedness above is *syntactic*... **Semantic correctness** — i.e., `tally = Tally_spec(...)` — is enforced *by the enclave software*, not by the chain." Failure mode 4.4 ("Enclave software bug") describes a well-formed but semantically wrong tally that the chain accepts. An output consumer reading only Section 6.6 would reasonably believe the published result is correct; the spec does not warn them that well-formedness does not imply correctness. This is a material omission in the output contract's trust claims.
- **Cite**: > "The tally body satisfies the well-formedness invariants S6 + S7 + S8 + S9" (Section 6.6) vs Section 2.5 "well-formedness above is syntactic — the published tally satisfies the structural invariants. Semantic correctness... is enforced by the enclave software, not by the chain"
- **Fix recommendation**: Add a bullet to Section 6.6's "What callers cannot infer" list: "That the tally is semantically correct (i.e., equals `Tally_spec` applied to the frozen ballots). Well-formedness is syntactic; a well-formed but incorrect tally is possible if the enclave software contains a bug (Section 4.4)."

### 5. Caller contract "trusted to declare candidates honestly" is toothless and misleading [cosmetic]
- **Category**: ambiguity
- **Affected**: Section 6.4, second bullet ("Is trusted to declare candidates honestly")
- **What's wrong**: Section 6.4 says the instantiator "is trusted to declare candidates honestly," but Section 1 says "Anyone may instantiate a contract" and Section 5 explicitly lists "No on-chain candidate-consent gate" as a non-goal. There is no enforcement mechanism for this "trust" — the instantiator can list arbitrary addresses without consent, create sybil candidate sets (same person controlling multiple addresses), or list inactive addresses. The phrase "is trusted to" implies the system assumes honesty, but the surrounding clauses make clear the system does not verify honesty and does not require consent. This creates ambiguity about whether the instantiator is in or out of the trust boundary.
- **Cite**: > "Is trusted to declare candidates honestly (verified-rcv does not gate this; legitimacy is off-chain per Section 5)" and Section 5 "No on-chain candidate-consent gate. Anyone may instantiate a contract listing any chain addresses as candidates."
- **Fix recommendation**: Rephrase as "Is **unverified** in candidate declaration — anyone may instantiate with any address list; legitimacy is established entirely off-chain." This removes the false impression that the system places the instantiator under a trust assumption it enforces.

## Slice-local summary
- Critical: 0
- Serious: 4
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE

The slice contains four serious issues: (1) the output contract overstates categorical guarantees by omitting the `image_registration_honest` precondition; (2) the system identity understates its off-chain scope; (3) the "no operational-viability checks" non-goal creates a coverage gap against the stated purpose; and (4) the output contract omits the critical well-formedness-vs-correctness distinction that Section 2.5 and failure mode 4.4 explicitly acknowledge. These are grounded in specific text and materially weaken the spec's honesty.