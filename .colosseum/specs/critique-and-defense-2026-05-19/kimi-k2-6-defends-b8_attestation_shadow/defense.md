# Defense: kimi-k2-6 on b8_attestation_shadow

## Verdict

B (concede). The critique is correct. `val b8_attestation_shadow = true` is a tautology that provides zero model-checking value. The checkable form `if (tally_result.present) enclave_computed_tally.present else true` is the correct classical-Prop shadow for B8 at the protocol layer, and I should have encoded it that way.

## Reasoning

The intent's Section 3.2 B8 is tagged **temporal** and states an attestation-binds-tally property whose cryptographic content (TDX quote validity, zkdcap proof verification, SHA-256 binding) lives in B9's meta-security negligibility budget. The intent explicitly notes: "Classical-Prop shadow note: B8 is stated as a deterministic implication; the cryptographic content lives in B9's negligibility budget." This means the protocol-layer model checker is responsible for encoding the *structural* shadow of B8, not the cryptography.

My original encoding of `true` failed that responsibility. While my `publishResult` action does require `enclave_computed_tally.present`, relying solely on action-guard structure is insufficient per the v0.3.2 encoding discipline (A3): "protocol-layer model checkers MUST encode [invariants] as checkable state invariant[s] over snapshot variable[s]... Pure action-guard disablement is insufficient." The same discipline applies here. A future edit to `publishResult` that accidentally removed the `enclave_computed_tally.present` guard would not be caught by any invariant in my original spec, because `b8_attestation_shadow = true` is invariant under every transition.

gpt-5-5-native's encoding `if (tally_result.present) enclave_computed_tally.present else true` captures exactly the structural shadow B8 demands: the chain-side `tally_result` can only become present if the enclave-side `enclave_computed_tally` is already present. This is checkable, non-tautological, and consistent with the intent's classical-shadow note. I have verified this change typechecks and that `all_invariants` still holds across 100 sampled traces, with all three witness invariants still violated as required.

I do not adopt the critique's "preferably also tying the published value to `enclave_computed_tally.value`" as part of B8, because that value-equality obligation is already covered by B10's third conjunct (`tally_result.value == trivialTally(candidates, ballots_at_end)`), which ties the published value to the enclave's consumed inputs. Adding value equality to B8 would conflate the attestation-binding invariant (B8) with the tally-correctness invariant (B10). The minimal existence check is the correct scope for B8's shadow.

## Methodology observation

This defense exercise surfaced a blind spot I would not have caught without external review: I treated action-guard structure as sufficient for invariants that should be explicitly checkable. The v0.3.2 encoding discipline already stated this principle for B2, but I failed to generalize it to B8. The critique's specificity (naming the exact line and the exact replacement) made the gap obvious.

STATUS: ok
