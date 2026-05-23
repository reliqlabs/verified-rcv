# gemma-4-26b-a4b — slice scope — System scope, identity, and external contracts

## Cross-section reads
- Section 1/5: Notes that legitimacy of election instances is off-chain, contradicting the "trust" requirement in Section 6.4.
- Section 1/5: Notes the lack of on-chain candidate-consent gates, which impacts the "honesty" requirement in Section 6.4.
- Section 1/5: Notes the lack of formal verification for upstream Quartz axioms, which impacts the reliability claim in Section 6.6.
- Section 1/6: Notes chain trust for ZK modules and registration, which impacts the reliability claim in Section 6.6.

## Attacks on this slice
### 1. Section 6.4 Over-specification of Caller Trust [serious]
- **Category**: over-specification
- **Affected**: Section 6.4 / line 524
- **What's wrong**: Section 6.4 states the instantiator "is trusted to declare candidates honestly" (524). However, Section 1/5 explicitly states that the legitimacy of any election instance is off-chain (23) and that there is "no on-chain candidate-consent gate" (452). Since the contract does not verify whether candidates have consented to run, the instantiator is not "trusted" to do something the contract doesn't care about; they are simply permitted to provide any valid chain addresses, and the contract will proceed regardless of their "honesty" regarding candidate legitimacy.
- **Cite**: > Is trusted to declare candidates honestly (verified-rcv does not gate this; legitimacy is off-chain per Section 5).
- **Fix recommendation**: Remove the "is trusted to" requirement for honesty regarding candidate legitimacy, as the contract explicitly disclaims gating on consent. Frame it instead as an operational assumption for the instantiator.

### 2. Section 6.6 Over-specification of Output Reliability [serious]
- **Category**: over-specification
- **Affected**: Section 6.6 / line 540
- **What's wrong**: Section 6.6 claims the Output Contract provides reliability on attestation (540). However, Section 1/5/6.1 admit the underlying attestation correctness (Quartz/zkdcap) and chain-side registration are outside the `verified-rcv` scope/verification (21, 465, 478, 480). A consumer relying on Section 6.6 for security is assuming something the spec explicitly disclaims (Section 5, 465). If the chain/Quartz are compromised, the Output Contract claim (540) is violated on-chain, yet the `verified-rcv` spec *claims* to provide this reliability as part of its defined output contract.
- **Cite**: > Any party reading `tally_result` from chain state can rely on: ... `tally_result.is_some()` ⇒ a valid `DstackAttestation` was supplied ... (540)
- **Fix recommendation**: Temper the claim in Section 6.6 to state that the observer can rely on the *on-chain evidence* of attestation, rather than the underlying cryptographic/operational validity of the attestation itself, which is explicitly delegated to upstream components.

## Slice-local summary
- Critical: 0
- Serious: 2
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-SLICE