# verified-rcv

Methodology-disciplined implementation of instant-runoff voting (IRV) ranked-choice tabulation.

This repo is the **Round 3a dogfood** of the [Colosseum methodology](https://github.com/reliqlabs/colosseum), exercising the full intent → spec → adversarial → compose pipeline on a greenfield target. Round 3a closed 2026-05-20; the verification surface artifacts are the deliverable.

## Outcome

Round 3a closed end-to-end on the spec layer:

- **Intent v0.3.3** at `.colosseum/intent.md`. 8 revision cycles. 4 encoding-discipline notes (A2/A3/A4/A5) propagated from cross-critique findings back into the intent doc.
- **Quint protocol spec** at `specs/rcv.qnt` (537 lines). 6 named state invariants + composite + 5 reachability witnesses. `quint typecheck` clean; `quint run --invariant=all_invariants` 0 violations across sampled traces; 3/3 reachability witnesses violated as expected.
- **Lean math spec** at `specs/RcvSpec.lean`. Tally_spec composition (Stage 1 `decrypt_and_validate` + Stage 2 `IRV_spec`), 5 well-formedness + image-IO theorem statements with `sorry`, 2 axioms (`EnclaveImage` carrier, `irv_ballots_tallied` Stage-2 obligation). Lean typecheck clean.
- **Integration ledger** at `.colosseum/ledger.md`. 2 composition theorems (B10 cross-layer 5-link, B9 negligibility 3-summand), 5 axioms inventoried with 4-bucket trust-density taxonomy, dead-axiom scan 0 hits.
- **Adversarial trail** at `.colosseum/specs/*`. Fan-out, cross-critique, defense, re-cross-critique cycles run on both Quint and Lean layers. Each cycle's findings traced into either intent revisions (when the fix is intent-level) or canonical patches (when spec-level).

The Round 3a outputs back-port to the methodology repo as Asks O–T (cross-critique, defense round, encoding-discipline intent-tightening, re-cross-critique, ghost-variable encoding, `--variant high` reasoning default). See `colosseum/methodology-v0.4-candidates.md`.

## Comparator status

The original plan was to compare against `quartz/examples/ranked-choice/` as an external baseline. That target turned out to be the user's own pre-Round-3a work in the local Quartz fork, not upstream — upstream `informalsystems/cycles-quartz` has only `pingpong` and `transfers` examples. A self-comparison artifact is preserved at `.colosseum/self-comparator-pre-round-3a-2026-05-20.md` for internal reference. A proper external comparator would require running `colosseum-reverse-intent` against a public IRV implementation (Vocdoni, Cosmos Hub governance, or a published academic spec). Deferred to a later round.

## Scope of this cycle

Spec-layer only: intent + Quint + Lean + ledger. No CosmWasm contract, no enclave Rust crate, no frontend. The 5 Lean `sorry` markers are pending Leanstral local install (proof model) and an enclave Rust crate to extract `EnclaveImage` from (both out of scope this cycle).

## Layout

```
verified-rcv/
  .colosseum/
    intent.md                                  # source of truth (v0.3.3)
    ledger.md                                  # integration ledger
    self-comparator-pre-round-3a-2026-05-20.md # self-comparison artifact
    attacks/                                   # intent adversarial reports
    specs/                                     # multi-voice fan-out + cross-critique outputs
    scripts/                                   # dispatch scripts (fan-out, cross-critique, re-cross-critique)
  specs/
    rcv.qnt                                    # Quint canonical (protocol layer)
    main.qnt                                   # Quint entry point
    RcvSpec.lean                               # Lean canonical (math layer)
```

## Discipline

This run was conducted full-blind against the would-be external comparator until task #20 (the comparator pass), at which point the blindness lifted and the self-comparison was produced. No code or spec from the comparator side fed into Round 3a's outputs.

## Related repos

- `/Users/mvid/Development/reliq/colosseum` — the methodology under validation
- `/Users/mvid/Development/reliq/quartz` — the Quartz substrate (verification + dstack TDX + zkdcap)
