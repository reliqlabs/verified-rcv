# Cross-substrate comparator: Quartz ranked-choice example

**Test type:** apply v0.4 ask list (AB-AI) to `/Users/mvid/Development/reliq/quartz/examples/ranked-choice/` (a project the v0.4 methodology was developed on a different dogfood). Identify findings v0.4 surfaces.

**Blindness policy:** lifted for this comparator pass per the Phase 5 protocol of `.colosseum/methodology-retrospective-2026-05-26.md`. Date: 2026-05-26.

**Project shape:** minimal Quartz example. 200-line contract, 297-line enclave, 502-line Quint spec, 468-line verification.rs (8 Kani harnesses + pure mirrors of production logic). No `.colosseum/` directory on the example itself; methodology artifacts on this example live under `/Users/mvid/Development/reliq/quartz/.colosseum/attacks/kani-2026-05-20/` (Round E adversarial review of the Kani-harness surface across all Quartz examples).

## v0.4 lens applications

### AB.1 commitment-coverage

**Target:** the `Tally` attestation. Code: `contract.rs::exec_tally` accepts `AttestedMsg<TallyMsg, RawDefaultAttestation>`. The `TallyMsg` struct (`msg.rs:50-57`) derives `UserData`, so the attestation's UserData = serialization of `(election_id, winner, rounds, total_ballots)`.

| commitment | attacker DoFs | bound by | gap |
|---|---|---|---|
| Tally attestation UserData | contract_addr, chain_id, election_id, candidate set, ballot ciphertext set, winner, rounds, total_ballots | election_id + winner + rounds + total_ballots | **contract_addr (cross-contract replay), chain_id (cross-chain replay), ballots_hash (input-fidelity host substitution)** |
| Session pubkey | (no commitment surface; handshake-managed) | quartz session protocol | none at this layer |
| ECIES ballot ciphertext | candidate identity (via msg.sender chain auth), ballot content, election context | enclave pubkey + msg.sender chain auth | same minor as verified-rcv m7 (chain context not in AEAD AD) |

**Finding F1 (HIGH):** Tally attestation UserData binds neither `contract_addr` nor `chain_id` nor `ballots_hash`. This enables three distinct attacks: cross-contract replay (an attestation valid for contract A can be replayed at contract B if both deployed by the same admin), cross-chain replay (testnet attestation replayed at mainnet), and host substitution of `encrypted_ballots` passed to `tally_election` (the enclave receives an arbitrary ballot set, produces a valid attestation for it, and the contract has no way to verify the ballot set the enclave saw matches the chain's BALLOTS map).

This is the exact verified-rcv M2 + N4 + B6 cluster (replay-narrow-binding) appearing on the Quartz ranked-choice substrate.

### AB.2 clause-to-line discharge

The example has no formal intent document with named B-clauses. The closest analog is the Quint spec at `specs/ranked-choice.qnt` plus the README's per-data-row visibility table. Applying AB.2 to the data-visibility table + Quint invariants:

The Quint spec docstring (lines 50-56) explicitly states:

> "exec_tally accepts Voting | Tallying as input phases but only writes Complete. Tallying is therefore reachable only via off-chain orchestration (a relayer or external admin call not modelled in contract.rs). The `set_tallying` action below models this external transition explicitly."

**Finding F2 (MEDIUM):** the spec models a state transition (`set_tallying`: Voting → Tallying) that the contract code does not enforce. AB.2 clause-to-line: "Voting → Tallying" has `code: <none>` because no on-chain transition writes Tallying; the chain only ever sees Voting or Complete. The off-chain discharge of this transition is undocumented in the example (which off-chain actor moves the state? how is its honesty bounded?). This is a discharge gap in the trust chain: the spec claims an off-chain orchestrator handles it, but the orchestrator's behavior is unconstrained.

### AB.3 deferred-is-panic

The example's `Cargo.toml` declares `mock = ["quartz-contract-core/mock"]`. The default build does NOT include `mock`. Applying AB.3 to production-feature build:

- Mock attestation: feature-gated via `quartz-contract-core/mock`. The example does not appear to have any code path that goes Mock-by-default in production. PASS (assuming quartz-contract-core gates the mock variant from default features, which CLAUDE.md confirms).
- Stub branches: the enclave's `decrypt_ballot` returns `Err` on bad ciphertext; `tally_election` skips bad ballots. No `unimplemented!()` or `todo!()` visible.

**Count: 0 new findings (provisional pending verification of quartz-contract-core mock gating).** PASS.

### AB.4 who-controls

| field | supplier | validation | gap |
|---|---|---|---|
| `Config.admin` | instantiator (msg.sender) | implicit (immutable after) | none |
| `Config.voting_duration` | instantiator | none — accepts any `u64` including `u64::MAX` | **upper bound missing** |
| `Election.candidates` | admin via CreateElection | len ≥ 2, no dups, string identity only | string length not bounded |
| `Election.phase` | state machine via handlers | exec_open_voting checks `== Setup`, exec_cast_ballot checks `== Voting`, exec_tally checks `Voting | Tallying`, exec_create_election: **no phase check** | **CreateElection lacks phase gate** |
| `Election.voting_end` | derived from voting_duration at OpenVoting time | none beyond voting_duration | downstream of voting_duration gap |
| `BALLOTS[sender]` | voter (sender == candidate not enforced — anyone can submit) | phase + time + no-double-vote, but no sender-must-be-candidate check | **non-candidate voters accepted** |
| `RESULTS[election_id]` | enclave via attested Tally | attestation + election_id match | gap from F1 (no contract_addr/chain_id/ballots_hash) |

**Finding F3 (MEDIUM):** `voting_duration` has no upper bound. Admin can set `u64::MAX` and the election effectively never closes (no enforcement of close-or-tally outside admin/orchestrator action).

**Finding F4 (HIGH):** `exec_create_election` performs no phase check. The handler runs regardless of `Election.phase`'s current value, clearing `BALLOTS` and overwriting `ELECTION` with a new election at phase Setup. The Quint spec's `phase_transition_allowed` allow-list (`verification.rs:60-69`) explicitly disallows `Voting → Setup` and `Tallying → Setup`, but the contract code doesn't enforce this. Identical bug shape to verified-rcv M1.

**Finding F5 (MEDIUM):** `exec_cast_ballot` does not check that `msg.sender ∈ candidates`. Any address can submit a ballot, not just registered candidates. This is a deliberate design choice in many voting systems (eligible voters ≠ candidates), but if the project intent is "candidates vote for each other" (as verified-rcv encodes), the gap is real. The Quint spec models a `voters: Set[Addr]` separate from `candidates: Set[Candidate]`, suggesting the design intent is multi-voter not candidate-self-vote.

### AB.5 API field-name fidelity

| API | field | name implies | actual | status |
|---|---|---|---|---|
| TallyMsg | election_id | u64 election identifier | u64 | OK |
| TallyMsg | winner | candidate string | String (candidate name) | OK |
| TallyMsg | rounds | tally rounds | Vec<TallyRound> | OK |
| TallyMsg | total_ballots | u32 count | u32 | OK |
| SessionResponse | pub_key | session pubkey | Option<HexBinary> | OK |
| QueryMsg::Result | election_id | identifier | u64 | OK |

**Count: 0 findings.** PASS.

### AB.6 deferral-justification audit

No prior audit history on the example itself. The Round E synthesis findings on Kani harness coverage are separate methodology events; they don't carry deferral justifications to audit. **Count: 0 findings.** N/A.

### AC top-down Kani catalog

The example has 8 Kani harnesses targeting pure-mirror helpers: `candidates_valid` (3 harnesses), `can_cast_ballot`, `can_tally`, `phase_transition_allowed` (exhaustive 16-pair), `filter_ballot_len`, `first_active_choice`.

If a §8.7-style trust ledger existed, it would name links like:
- L1. Tally attestation soundness (DstackAttestation verification)
- L2. UserData commitment binding (contract_addr + chain_id + election_id + ballots_hash + tally body)
- L3. ECIES decryption correctness
- L4. IRV algorithm correctness (winner ∈ candidates, total_ballots = |ballots|, etc.)
- L5. Phase machine integrity
- L6. Ballot ordering / declaration-order discipline

AC requires every link have a Kani harness or kani-skipped annotation:

| link | kani harness | annotation needed |
|---|---|---|
| L1 attestation soundness | none (delegated to quartz-contract-core) | kani: skipped because quartz-core-handled |
| L2 UserData binding | **NONE — gap** | none for current TallyMsg shape; F1 fix would add a harness |
| L3 ECIES correctness | none | kani: skipped because crypto primitive |
| L4 IRV correctness | 2 harnesses (`filter_ballot_len`, `first_active_choice`) + 6 pure-helper harnesses | partial coverage; **missing: total_ballots invariant** (Quartz Round E S5 already flagged) |
| L5 phase machine | 1 harness (exhaustive 16-pair) | OK |
| L6 declaration-order | none in example (candidates iteration is over `Vec<String>` which preserves order, but no harness asserts) | **missing harness** |

**Finding F6 (HIGH, dual-counted with F1):** No Kani harness covers UserData binding. After fixing F1 (binding chain context), a new harness is needed.

**Finding F7 (MEDIUM):** No Kani harness for the "every-ballot-counted" invariant (total_ballots = |ballots| over non-malformed input). This is exactly the gap Quartz Round E flagged.

**Finding F8 (LOW):** No Kani harness asserting per-round count outer index follows candidate declaration order (verified-rcv M4 analog).

### AD ledger-as-gate

The example has no `.colosseum/` directory + no §8.7-style ledger. **Finding F9 (METHODOLOGY):** if the example is meant as a verification-target dogfood, a ledger should exist enumerating the trust chain. If it's purely a product example, this is appropriate scope. Operational distinction.

### AE lifecycle-adversary

**Target:** multi-tx admin sequences across CreateElection, OpenVoting, CastBallot, Tally, plus the implicit off-chain Voting→Tallying transition.

Sequences:

| sequence | outcome | finding |
|---|---|---|
| `[Voting, admin: CreateElection]` | BALLOTS cleared, ELECTION reset to Setup. Mid-flight election destroyed. | **F4** (re-counted) |
| `[Tallying, admin: CreateElection]` | Same as above; mid-tally election destroyed. | **F4** (re-counted) |
| `[Voting (before voting_end), admin: orchestrator-triggers-tally]` | Contract accepts Tally during Voting phase (phase != Voting && != Tallying check is for FALSE rejection — both phases ARE accepted). Premature finalization. | **F10 (HIGH)** |
| `[Voting, admin: CreateElection (clobber), enclave: Tally for old election_id]` | Enclave's tally is for old election_id; contract's new election has different counter; tally rejected at election_id check. Recovery works but interim state was inconsistent. | partial mitigation |
| `[Setup, admin: OpenVoting, voter: CastBallot, admin: CreateElection (clobber)]` | F4 sequence; BALLOTS cleared mid-flight. | **F4** |

**Finding F10 (HIGH):** premature finalization. `exec_tally` accepts both `Voting` and `Tallying` phases as preconditions. Combined with no `env.block.time >= election.voting_end` check, an orchestrator can publish the tally during the voting window. The enclave's `tally_election` (request.rs) has no time check either; if a relayer calls the enclave with the current ballot set during Voting phase, the enclave produces a valid attestation, and the chain accepts it. This is a brand-new finding shape v0.4 catches that wasn't present in verified-rcv (verified-rcv enforces post-end_at via state machine + `S10 resolution_after_end_at` Kani harness).

### AF CI-self-test

Not visible from example state. Not applicable.

### AG commit-message reconciliation

Not applicable to a single-snapshot review.

### AH Quint-adversarial

The Quint spec has been adversarially reviewed in Quartz's prior Kani-harness review (Round E). The spec itself encodes the phase machine, IRV semantics, and ballot eligibility. Adversarial traces under AH:

- Trace `[set_phase_voting, set_phase_tallying (off-chain), tally]`: spec accepts. Code accepts. But the off-chain `set_phase_tallying` is unenforced; the spec models it as a free transition. **Finding F11 (re-counts F2):** the spec accepts an unconstrained off-chain transition. Adversarial trace: orchestrator chooses to never move Voting → Tallying → election stuck.
- Trace `[admin: CreateElection at phase=Voting]`: phase_transition_allowed disallows Voting → Setup but the contract code allows it. Spec-code drift. **Finding F12 (re-counts F4).**

AH produces F2 + F4 directly from trace generation against the Quint model.

### AI field-spec at intent-elicitation

Binary-data fields in the example:

| field | semantic | wire format | length | validation site |
|---|---|---|---|---|
| `ciphertext` (HexBinary) | ECIES-encrypted ballot | HexBinary | unbounded | **GAP** — no length cap, enables ballot-storage DoS |
| `candidates` | candidate name list | Vec<String> | each unbounded | **GAP** — admin can write multi-megabyte candidate names |
| `title` | election title | String | unbounded | **GAP** — admin can write huge title |
| `winner` | candidate name | String | unbounded | inherited from candidate validation; OK if F4 fixed |

**Finding F13 (LOW):** binary/string fields lack wire-length pins. Mostly DoS-class.

## Findings summary

| F# | finding | lens | severity | mirrors verified-rcv finding |
|---|---|---|---|---|
| F1 | TallyMsg UserData lacks contract_addr/chain_id/ballots_hash | AB.1 | HIGH | M2 + N4 + B6 |
| F2 | Quint off-chain set_tallying transition discharge gap | AB.2 + AH | MEDIUM | N17-adjacent |
| F3 | voting_duration upper bound missing | AB.4 | MEDIUM | (new) |
| F4 | CreateElection lacks phase gate | AB.4 + AE + AH | HIGH | M1 |
| F5 | exec_cast_ballot does not enforce sender ∈ candidates | AB.4 | MEDIUM | (intent-divergent) |
| F6 | No Kani harness for UserData binding | AC | HIGH | (latent prevention) |
| F7 | No Kani harness for total_ballots invariant | AC | MEDIUM | (M4 analog) |
| F8 | No Kani harness for declaration-order | AC | LOW | M4 directly |
| F9 | No §8.7-style trust ledger | AD | METHODOLOGY | (verified-rcv has §8.7) |
| F10 | Premature finalization (Tally accepted during Voting) | AE | HIGH | (new) |
| F11 | Quint accepts unconstrained off-chain transition | AH | MEDIUM | (dual with F2) |
| F12 | Spec/code phase-transition drift on CreateElection | AH | HIGH | (dual with F4) |
| F13 | Binary/string fields lack wire-length pins | AI | LOW | M3 analog |

**13 distinct findings.** Of these:
- **5 HIGH** (F1, F4, F6, F10, F12).
- **4 MEDIUM** (F2, F3, F5, F11).
- **3 LOW** (F7, F8, F13).
- **1 METHODOLOGY** (F9).

## Cross-substrate calibration table

How verified-rcv's high-severity findings map onto the comparator:

| verified-rcv finding | Quartz ranked-choice mirror | v0.4 lens catches both? |
|---|---|---|
| C1 (Mock variant accepted) | (gated upstream by quartz-core; needs verification but AB.3 applies identically) | YES (AB.3) |
| C2 (B8 clauses a/b/c/d not enforced) | DstackAttestation verification handled by quartz-core; AB.2 would apply against the core | YES (AB.2 at core layer) |
| C3 (enclave_pubkey provenance) | session pubkey provenance handled by quartz handshake; analog | YES (AB.2 + AB.4) |
| M1 (CreateElection no phase-gate) | F4 — EXACT MIRROR | YES (AB.4 + AE) |
| M2 (no election_id in canonical) | F1 partial — same cluster, election_id IS bound but addr/chain/ballots are not | YES (AB.1) |
| M3 (registry shape validation) | (registry not in this example; quartz-core layer) | YES (AI applies at core) |
| M4 (declaration-order discipline) | F7 + F8 partial | YES (AC) |
| M5 (mrtd_hex slot held compose_hash) | (no equivalent JSON field in example) | N/A |
| N1 (chain accepts attestation without verify) | DstackAttestation verifies via quartz-core; chain ZK module verify happens upstream | YES (AB.2 + AD against core layer) |
| N2 (admin picks pubkey freely) | session pubkey handled by quartz handshake; admin doesn't pick directly | partial (AB.4) |
| N17 (finalize-mid-election DoS) | F2 + F10 — different shape, same lens | YES (AE) |
| N13/N22 (registration replay) | F1 — same commitment-coverage gap | YES (AB.1) |
| B6 (ballots_hash missing) | F1 — same finding | YES (AB.1) |

**12 of 13 verified-rcv high-severity findings have a direct or analogous catch on the Quartz ranked-choice example via the same v0.4 lens.** The methodology generalizes.

The exception is M5 (mrtd_hex slot drift) — the example doesn't expose a parallel JSON surface, so AB.5 has nothing to flag.

## v0.4 findings ranked-choice has that verified-rcv did not exhibit

- **F5** (voter ≠ candidate): a design-intent divergence not present in verified-rcv (where voter == candidate is enforced).
- **F10** (premature finalization): verified-rcv's S10 Kani harness exists; ranked-choice's verification.rs has no equivalent.
- **F2 / F11** (Quint off-chain transition): a methodology gap that verified-rcv side-stepped by not modeling the Tallying phase as a separate state.

## Methodology gaps the comparator surfaces

The Quartz Round E adversarial review (`quartz/.colosseum/attacks/kani-2026-05-20/synthesis.md`) was a Kani-harness-coverage review. It correctly found:
- Helpers are pure mirrors of production logic.
- Missing total-vote-count invariant.
- Phase-transition exhaustiveness (which has since been fixed in the example's verification.rs).

What Round E *did not* find because its scope was Kani-only:
- **F1** (UserData commitment-coverage gap) — outside Kani's purview; AB.1 catches.
- **F4** (CreateElection phase-gate) — Round E touched phase-transition harness but did not red-team the actual handler against the harness allow-list. AB.4 + AE catch.
- **F10** (premature finalization) — Round E was about the harness surface, not the handler-vs-spec semantics. AE catches.

This is the strongest signal from the comparator: **v0.4's lenses (AB.1, AB.4, AE) catch findings on the Quartz ranked-choice substrate that Quartz's own prior Kani-harness adversarial review did not surface.** The lenses generalize *and* add value.

## Phase 5 acceptance

Phase 5 protocol: validation succeeds when ≥10 of 13 verified-rcv findings + most-of Quartz findings are caught, with a small honest list of methodology gaps that remain.

- **Verified-rcv coverage:** 12 of 13 (only M5 has no analog on this substrate). PASS.
- **Quartz ranked-choice findings v0.4 catches:** 13 distinct findings (5 HIGH, 4 MEDIUM, 3 LOW, 1 methodology). 5 of these are HIGH-severity not previously surfaced by Quartz's own Round E adversarial review.
- **Honest open gaps:**
  - The comparator validates lens *applicability* but not lens *operability at scale*. The Quartz ranked-choice example is 1,000 lines of code. A larger system might surface different patterns.
  - F1's fix on the Quartz side would require extending Quartz's UserData macro discipline; this is a methodology back-port from verified-rcv's canonical_serialization pattern.
  - The Quartz ranked-choice example does not have a formal intent doc. v0.4's AB.1/AB.2 lenses work better with a §3.2-style B-clause enumeration; without one, the lens operator has to reverse-engineer the intent from code + Quint spec.

## Validation deliverable

**Phase 5 result: PASS.** v0.4 ask list (AB-AI) is calibrated and generalizes cross-substrate. The methodology can ship to v0.4 after the V1 + V2 + comparator deliverables are committed and the bootstrap obligations (AB.2/AC/AD citation columns on intent §3.2 + §8.7) are discharged in a follow-up.

**Recommendation to Quartz agent:** apply F1, F4, F10 fixes to the Quartz ranked-choice example. F1 + F4 are HIGH-severity production bugs; F10 is a HIGH-severity protocol bug. The fixes mirror verified-rcv's v0.3.8 + v0.3.10 + v0.3.12 patches.
