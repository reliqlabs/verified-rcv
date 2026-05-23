# Round 3a comparator pass — verified-rcv vs Quartz `examples/ranked-choice/`

- Date: 2026-05-20
- verified-rcv state: intent v0.3.3, Quint canonical at `specs/rcv.qnt`, Lean canonical at `specs/RcvSpec.lean`, ledger at `.colosseum/ledger.md`
- Quartz state read for the first time today (blindness policy per `CLAUDE.md` released for this task)
- Quartz reference: `/Users/mvid/Development/reliq/quartz/examples/ranked-choice/`

This is the Round 3a payoff comparator. Both pipelines targeted "private ranked-choice voting via Quartz/dstack TDX + zkdcap." verified-rcv came at it via the Colosseum methodology (intent → spec → adversarial → spec → ledger, no implementation). Quartz built the engineering surface (Quint spec → Apalache verification → working contract + enclave + frontend). The honest comparison is shape-versus-shape, not "which is right."

## Inventory diff

| Artifact | Quartz `examples/ranked-choice/` | verified-rcv |
|---|---|---|
| User-facing README | 82 lines, how-to / privacy table | none |
| Intent document | none (intent is implicit in spec + README + code comments) | ~3000-line versioned intent doc with revision log, failure modes, scenarios, blocks, invariants, trust boundaries |
| Quint spec | `specs/ranked-choice.qnt` 502 lines, 10 invariants, Apalache exhaustive on a bounded universe (3 candidates, 2 voters, 9 representative ballots) | `specs/rcv.qnt` 537 lines, 6 named state invariants + 1 composite + 5 witness reachability invariants, `quint run` sampling (max-samples=100 × max-steps=30) |
| Lean math spec | none for ranked-choice (Lean work at `proofs/lean/` is on the Quartz substrate, not this example) | `specs/RcvSpec.lean` ~210 lines: `Tally_spec` composition + Stage 1 / Stage 2 opaque + 5 theorem statements with `sorry` + 2 axioms (`EnclaveImage`, `irv_ballots_tallied`) |
| Apalache run output | yes — `_apalache-out/server/2026-05-14T19-27-29_*` | no |
| CosmWasm contract | yes — `contracts/src/` (lib + state + msg + contract + verification + error) = 714 lines | none (out of scope this cycle per CLAUDE.md) |
| Enclave Rust crate | yes — `enclave/src/` (cli + main + request) = 378 lines, 7 unit tests | none (out of scope this cycle) |
| Frontend | yes — Next.js + abstraxion | none (out of scope this cycle) |
| Adversarial review trail | `.colosseum/attacks/quint-recently-revised-2026-05-14/` (Round B) found one tie-break docstring inversion | 8 cycles: v0.1.0 → v0.3.3, including multi-voice fan-out, cross-critique, defense, re-cross-critique, encoding-discipline propagation |
| Integration ledger | none for the example (the parent Quartz `.colosseum/ledger.md` is 26-axiom substrate inventory, not ranked-choice-specific) | yes — `.colosseum/ledger.md` records 2 composition theorems (B10, B9), 5 axioms, dead-axiom scan, per-tool coverage, trust density |

**Inventory finding: the two artifact sets barely overlap.** verified-rcv is a verification-surface artifact set; Quartz is an engineering-surface artifact set. The single common artifact is the Quint spec, and even there the targets diverge (see below).

## Intent / spec coverage diff — what each side names

### IRV variant divergence (most consequential)

**The two specs encode different IRV variants.**

Quartz's IRV variant (per `enclave/src/request.rs` lines 30-153 and `specs/ranked-choice.qnt` lines 9-15):

- Candidates are `String` names (e.g., "Alice", "Bob", "Carol"). Voters are separate `Addr` values.
- Ballots are `Vec<String>` ranked-choice lists, possibly truncated (subset of candidates, not necessarily a full permutation).
- Majority threshold: `total_active_votes / 2 + 1` over the *current round's non-exhausted* ballots (shrinking denominator).
- Tie-break on elimination: lex-LARGEST among candidates tied for fewest votes. (The Quint spec calls this out as a Round B 2026-05-14 finding: the prior docstring said lex-smallest but the code actually picks lex-largest because of `sort (b.1 desc, a.0 asc)` then `iter().rev().find(min)`.)
- Termination: majority found OR one candidate left.
- Result schema: `TallyMsg { election_id, winner: String, rounds: Vec<TallyRound>, total_ballots }`. Single winner (not a set). `TallyRound` contains `counts: Vec<(String, u32)>` and `eliminated: Option<String>`.

verified-rcv's IRV variant (per intent §2.5 and `specs/RcvSpec.lean`):

- Voters and candidates are the SAME set — `candidates: Vec<Addr>` is both. Each candidate has permission to submit a ranked ballot of all candidates.
- Ballots MUST be full permutations of `candidates`. Non-permutation ballots are dropped at Stage 1 validation. No truncated ballots; no shrinking denominator.
- Tie-break: ALL candidates tied for lowest are eliminated in one batch (no individual tie-break needed).
- Edge case: when batch elimination would empty `remaining`, all current remaining candidates are declared co-winners.
- Termination: majority found OR all remaining tied (multi-winner).
- Result schema: `TallyResult { winners: Set, per_round_counts, eliminated_by_round, ballots_tallied, ballots_dropped, dropped_voters, non_voters }`. Set-valued winners; full round-by-round structure.

These are different elections. Quartz models Optional Preferential IRV (typical for civic elections with large electorates where voters rank some-not-all). verified-rcv models Full Preferential Council Vote (the candidates vote among themselves; each must rank all; ties → co-winners).

**Neither is wrong; they're solving different problems.** But that means the verification artifacts don't transfer between them — verified-rcv's `s9_no_reappearance` theorem statement and Quartz's `inv_deterministic_tiebreak` invariant are about different protocols.

### Phase model divergence

Quartz: explicit phase enum `{Setup, Voting, Tallying, Complete}` stored in `Election.phase`. The Quint spec notes that no `execute` branch in `contract.rs` writes `Tallying` — that transition is modeled as an off-chain `set_tallying` action (admin or relayer-flipped). `exec_tally` accepts `Voting | Tallying` as input phases.

verified-rcv: phase is `derived_phase` — a pure function of `block_time` relative to `start_at`/`end_at` and `contract_tally_result.present`. No `Tallying` state-as-data; the spec derives it from the time having crossed `end_at` without resolution.

Quartz's phase model is engineering-driven (explicit state for the orchestration tooling); verified-rcv's is methodology-driven (state should be derivable from observable inputs to avoid phase-desynchronization bugs).

### Quartz invariants that verified-rcv intent does not name

- **`inv_deterministic_tiebreak`**: the candidate that disappears between rounds is exactly `find_loser` of the previous round's counts (lex-LARGEST among min-vote candidates). verified-rcv doesn't need this because it eliminates all tied candidates as a batch (no per-round tie-break).
- **`inv_monotone_progress`**: each round's active candidate set is a strict subset of the previous. verified-rcv has this implicitly via S9 (no reappearance) + S8 (sum invariant), but doesn't name it as a separate invariant.
- **`inv_rounds_bounded`**: `round_count ≤ candidate_count`. verified-rcv implies it via S8 + S9 but doesn't name a closed-form bound.

### verified-rcv invariants that Quartz does not name

- **B1 (tally_result monotone-once-set)**: temporal claim that once `tally_result.is_some()`, it stays the same value forever. Quartz's contract enforces this operationally via `exec_tally`'s rejection of non-`Voting|Tallying` phases, but doesn't state the temporal claim as a spec invariant.
- **B2 (no late ballots)**: temporal claim that `ballots = ballots@end_at` for all time after `end_at`. Quartz models this via the phase guard; verified-rcv encodes it as a checkable state invariant over a ghost snapshot (encoding-discipline note A3).
- **B6 (ballot writer is the ballot voter)**: `∀-per-key + ∃!-unique-msg` form forcing per-key attribution of writes to a unique firing `SubmitBallot` message. Quartz's contract enforces this via `BALLOTS.save(deps.storage, &info.sender, ...)`, but doesn't state the per-key attribution as a spec invariant.
- **B8 (attestation-binds-tally)**: 5-clause attestation predicate covering TDX quote validation, zkdcap proof verification, SHA-256 commit on `(contract_addr, tally_body)` with domain-separation tag, and (mrtd, rtmr) match against on-chain registry. Quartz has the attestation handshake in `quartz-contract-core` but doesn't state this 5-clause predicate at the example level.
- **B9 (B8 negligibility-budget decomposition)**: 3-summand probabilistic bound conditioned on `image_registration_honest ∧ circuit_equivalence_honest`. Quartz's parent `.colosseum/ledger.md` covers the substrate negligibility lifts; the example doesn't have its own B9 statement.
- **B10 (tally-correctness)**: cross-layer 5-link composition `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack_kms_trust ∧ enclave_input_fidelity`. Quartz's `inv_winner_satisfies_irv` is the rough Quint shadow but doesn't decompose into cross-layer conjuncts.

### Failure modes that verified-rcv intent names and Quartz README doesn't

13 failure modes spanning enclave hardware compromise, KMS key compromise, attestation vkey substitution, enclave software bug, chain consensus halt, enclave network partition, voter address compromise, deterministic-malformed-tally deadlock, KMS unavailability at instantiation, enclave resource exhaustion, ZK module algorithmic compromise, block-time non-monotonicity. Each with severity, scope, detection, mitigation.

Quartz's README mentions privacy properties but doesn't enumerate failure modes. (Some live in Quartz's parent `.colosseum/` adversarial trail, not in the example.)

## Implementation correctness against spec

Reading the Quartz Quint spec against the Rust implementation:

- `tally_election` filters ballots to registered candidates (line 41-47 of `request.rs`) — matches the Quint spec's ballot-to-active-candidate filtering.
- Majority threshold `active_count / 2 + 1` (line 89) — matches `has_majority` in the Quint spec at line 169.
- Sort order `b.1.cmp(&a.1).then(a.0.cmp(&b.0))` (line 93) — votes DESC, names ASC, matching the spec's `find_loser` reasoning.
- Reverse-iter to find loser (lines 137-143) — confirmed lex-LARGEST tied-loser semantics, matching the Round B revision of the Quint spec.
- Duplicate-candidate rejection (`contract.rs` lines 71-76) — operationally enforces S2 distinctness. verified-rcv's intent v0.3.3 A4 note covers the Lean-side encoding; Quartz handles it at the contract Requires-clause level.

**Quartz's spec ↔ implementation alignment is high.** The Round B adversarial review caught a documentation/spec inversion (lex-smallest vs lex-largest) and the revised spec correctly names lex-LARGEST. The 7 unit tests in `request.rs` cover the IRV branches the spec enumerates.

## Methodology assessment — what each side caught and missed

### What verified-rcv's methodology caught that Quartz's did not

- **Stage 1 / Stage 2 type separation** (cross-critique 2026-05-20): verified-rcv's canonical was found to be conflating Stage 2's IRV-output type with the full Tally_spec result, missing the `non_voters` thread from Stage 1. The cross-critique caught it; the re-cross-critique verified the fix. Quartz's `tally_election` is a single function with no Stage 1 / Stage 2 boundary at the type level — the composition is monolithic. Whether this matters depends on the proof goal: for "the enclave computes the IRV algorithm correctly" Quartz's monolith is fine; for "the proof decomposes into a Stage-1 decryption-correctness lemma plus a Stage-2 IRV-correctness theorem" verified-rcv's split is load-bearing.
- **B10 5-link cross-layer composition** documented and inspectable. Quartz has the substrate negligibility lifts at the parent level but doesn't surface the example-level cross-layer composition theorem.
- **Encoding-discipline notes (A2/A3/A4/A5)** propagated as convergence-lever findings from voice divergence to intent revisions. Quartz's Round B review was a single defect-and-fix cycle; verified-rcv ran 8 revision cycles with multi-voice fan-out.
- **Tautological-shadow defect class**: verified-rcv's adversarial review explicitly looks for predicates that typecheck but claim nothing (`val foo_shadow = true`, theorems discharged by `rfl`). Quartz's example didn't surface this defect class.
- **Integration ledger**: verified-rcv ships a per-tool coverage snapshot + axiom inventory + trust-density taxonomy. Quartz's example doesn't have one at the example level (the parent Quartz `.colosseum/ledger.md` is substrate-scope).

### What Quartz's methodology caught that verified-rcv did not

- **Spec/implementation alignment loop**: Quartz's spec explicitly references implementation files (`contracts/src/contract.rs:160`, `enclave/src/request.rs:92`) and uses the same encoding choices the Rust code does (sort order, reverse-iter, lex-rank table). verified-rcv has no implementation to align against, so this discipline is currently dormant.
- **Apalache exhaustive verification on a bounded universe**: Quartz's Quint spec ran `_apalache-out/server/2026-05-14T19-27-29_*` for exhaustive bounded model-checking. verified-rcv uses `quint run` sampling at 100 traces, which is cheaper but not exhaustive. The Apalache discipline forced Quartz's spec author to make explicit state-space reductions (`ballot_history` dropped, `last_action` dropped, `EnclaveState` wrapper flattened, `round_active` retained for SMT efficiency) — those reductions are spec-craft choices that verified-rcv hasn't been pressured into making.
- **User-facing README with privacy table**: Quartz ships a 28-line privacy property table (what's public, what's enclave-only, what's never revealed). verified-rcv's intent doc has the equivalent content scattered across §1, §2.6, §6.3, but doesn't surface it as a single reader-facing table.
- **Operational tie-break documentation**: Quartz's spec docstring explicitly walks through why lex-LARGEST is the correct reading of the `sort_by` + `iter().rev()` chain. This is rare in spec authoring — most specs would either model lex-smallest (matching naive intuition) and miss the back-iteration semantics, or model lex-largest without explaining why. verified-rcv's intent doesn't need this discipline (batch elimination), but the discipline itself is methodologically inspectable.

## Where the two pipelines could cross-pollinate

- **verified-rcv → Quartz**: ship a Round-3a-style intent doc at `examples/ranked-choice/.colosseum/intent.md` to make the Quartz example self-documenting at the verification surface. Currently the intent is implicit in spec + README + code; promoting it explicit would let Quartz run the Colosseum cross-critique + re-cross-critique cycle the next time the IRV variant or tally schema changes.
- **verified-rcv → Quartz**: add a Lean math spec for the IRV core. Quartz's `tally_election` function is well-tested but unverified at the math layer. A `proofs/lean/Examples/RankedChoice.lean` with the 5-theorem skeleton (parametric over Quartz's IRV variant, not verified-rcv's) would give the example a Lean discharge target.
- **Quartz → verified-rcv**: run Apalache exhaustive on `specs/rcv.qnt` with the same bounded-universe discipline. The sampling-based `quint run` doesn't catch worst-case state-space holes; verified-rcv's Round 3 caught a `bt_tallied = 4` coverage hole that exhaustive verification would have flagged structurally.
- **Quartz → verified-rcv**: ship a CosmWasm contract that implements the verified-rcv IRV variant. This is the next round's work (post-spec-layer); the comparator confirms there's a real implementation surface ready to receive the spec.
- **Both → both**: explicitly name the IRV variant in each project's README/intent. Right now a reader who knows "IRV" but not "Optional Preferential vs Full Preferential" can't tell which protocol either spec is claiming about.

## Variant compatibility — can verified-rcv adopt Quartz's protocol or vice versa

The two specs solve different problems and are NOT drop-in compatible:

- verified-rcv's `s7_voter_partition` (`ballots_tallied + ballots_dropped + non_voters.length = cs.length`) requires voter ⊆ candidate identification. Quartz's separate-voter-and-candidate model breaks this equation.
- Quartz's `inv_deterministic_tiebreak` requires individual tie-break (lex-LARGEST elimination). verified-rcv's batch elimination has no per-round tie-break to verify.
- verified-rcv's `Tally_spec` produces co-winners on ties. Quartz's `tally_election` produces a single `winner: String` (empty on the no-ballots edge case).

If verified-rcv ever needs to ship as a Quartz example with truncated-ballot semantics, the intent would need a §2.6 amendment (variant: Full Preferential → Optional Preferential), and B10 / B10_lean would change shape. The cross-critique methodology would then re-run.

## Methodology readout

Both pipelines produced correct, internally consistent verification artifacts for their stated targets. The artifacts don't transfer (different IRV variants), but the methodology shapes are comparable.

**verified-rcv's Colosseum-disciplined methodology produced a richer verification surface**:
- Source-of-truth intent doc
- Lean math spec
- 2 composition theorems with cross-layer decomposition
- Trust-density taxonomy
- Multi-cycle adversarial trail (cross-critique → defense → re-cross-critique → encoding-discipline back-propagation)

**Quartz's engineering-driven methodology produced a richer engineering surface**:
- Working contract + enclave + frontend
- Apalache exhaustive verification on a bounded universe
- Spec ↔ implementation alignment loop with line-number cross-references
- Round B adversarial finding (lex-largest tie-break) caught and patched

The Colosseum methodology pays off on the verification surface; the engineering pipeline pays off on the working-software surface. They're complementary tracks for the same protocol family. The honest finding is that for the Round 3a question — "does Colosseum produce verification artifacts that compete with or exceed engineering-as-usual" — verified-rcv's methodology trail is materially more thorough than Quartz's adversarial review of `ranked-choice.qnt`, but Quartz's combined Quint+Apalache+implementation surface is closer to a shippable product.

The next-round move that would honestly close the comparator: pick one of the two IRV variants as the target, build the verified-rcv intent → Quint → Lean stack for it, then re-implement the Quartz contract + enclave against that intent, and run the colosseum-change skill on the spec → code transitions. That would test whether Colosseum's intent doc holds up as a source-of-truth as code is written against it.

## Outstanding follow-ups from this comparator

1. **Intent variant ambiguity**: verified-rcv's intent currently says "the canonical IRV variant used by Australian federal parliament (full-preferential, batch-elimination by lowest tied set)". The Quartz comparator surfaces that this variant choice is load-bearing — a reader who assumes Optional Preferential will get a different protocol. Candidate for an intent v0.3.4 PATCH: explicit "IRV variant" subsection in §1 System Identity, naming Full Preferential / Council Vote / Co-winners-on-ties / Voters-are-candidates.
2. **Spec-code alignment hooks**: verified-rcv's intent has `EnclaveImage` as the extracted-model symbol but no concrete Rust crate to extract from yet. When a Rust enclave is built (out of scope this cycle), add line-number cross-references in the Quint and Lean specs the way Quartz's spec does. The colosseum-change skill is the canonical entry point for that transition.
3. **Apalache exhaustive run**: try `quint verify` on `specs/main.qnt` with a bounded-universe variant of the Quint canonical. May surface state-space holes that sampling missed.
4. **Cross-pollination**: file a methodology note in `/Users/mvid/Development/reliq/colosseum/methodology-v0.4-candidates.md` recording that the Apalache-discipline (state-space reductions documented in the spec) is a Quartz-side practice worth back-porting to the Colosseum SKILLs as an optional ask. Candidate for Ask U or V.
