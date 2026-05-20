# verified-rcv

Greenfield methodology-disciplined IRV (instant-runoff voting) implementation. Round 3a dogfood of Colosseum.

## Current priority

Round 3a — validate Colosseum's greenfield intent → spec → adversarial → verify → compose pipeline on IRV. Compare against Quartz's existing `examples/ranked-choice/` at the end.

- **Blindness policy**: do NOT read `/Users/mvid/Development/reliq/quartz/examples/ranked-choice/**`, `/Users/mvid/Development/reliq/quartz/examples/ranked-choice/specs/rcv.qnt` (removed), or any commit diff mentioning ranked-choice, until task #20 (the comparator pass). Comparator must be honest.
- **Scope this cycle**: spec-layer only. Intent doc + Quint spec + optional Lean math spec + integration ledger. No contract, no frontend, no enclave.
- **Intent source**: elicited via `colosseum-intent` skill with the user as the human anchor — NOT pre-written.

## Methodology context

This repo's `.colosseum/` directory is the deliverable — it holds the intent, the ledger, the adversarial reports, and the change records that constitute Round 3a's output.

The methodology under validation lives at `/Users/mvid/Development/reliq/colosseum/` (v0.2 as of this session). Any gaps Round 3a surfaces become v0.3 asks, back-ported to the methodology repo.

## Related repos

- `/Users/mvid/Development/reliq/colosseum` — methodology
- `/Users/mvid/Development/reliq/quartz` — comparator (off-limits until task #20)
