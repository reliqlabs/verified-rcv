# Design notes

This Quint model encodes the contract state machine from intent §2.5 with a small bounded instance in `main.qnt` (`A`, `B`, `C`, `START_AT = 1`, `END_AT = 3`). `rcv.qnt` is parameterized over the candidate set and voting window.

Block 1 maps to `init`: candidates and timestamps are assumed valid by the instantiated constants, storage starts with empty `ballots`, empty `ballots_at_end`, and absent `tally_result`. Blocks 2 and 4 map to `tick`; the `tick` action advances `now`, and the crossing into `now >= END_AT` snapshots `ballots_at_end`. Block 3 maps to `submit_ballot(sender, encrypted_preferences)`, guarded by the voting window, candidate membership, and non-empty ciphertext, with last-write-wins via `Map.put`. Block 5 maps to `close_and_tally`, an idempotent no-storage-change self-loop in Tallying. Block 6 maps to `publish_result`, guarded by Tallying and unresolved state, which writes `tally_result` once. The terminal Resolved sink is `resolved_stutter`.

S1 through S4 are direct state predicates in `structural_invariants`: non-empty candidates, ordered timestamps, and ballot keys restricted to candidates. S5 is encoded by the action set: only `publish_result` writes a present tally, and it requires `not(tally_result.present)`. S6 through S10 are encoded by `tally_well_formed` and the resolved-time check.

B1, B5, and B7 are encoded by disabling all non-stutter actions after resolution. B2 is represented by `ballots_at_end`; after `now >= END_AT`, `ballots == ballots_at_end` must hold, and `submit_ballot` is disabled. B3 and B4 are action guards: `publish_result` requires `now >= END_AT`, while `submit_ballot` requires `START_AT <= now < END_AT`. B6 is approximated by `submit_ballot` taking an explicit `sender` and writing only `ballots[sender]`.

B8, B9, B10, and B10_lean cannot be fully discharged in this protocol-layer Quint model. I encoded a classical shadow: successful publish writes `valid_tally_for(ballots_at_end)`, where `valid_tally_for` is a deterministic stand-in for `Tally_spec`. Cryptographic proof validity, dstack KMS derivation, enclave image identity, input-fidelity attestations, and B9's negligibility bound are omitted and should be covered by the cross-layer ledger plus Lean or cryptographic proof artifacts.

The witness invariants are negations: reaching any submitted ballot, end boundary, or resolved state violates its witness.
