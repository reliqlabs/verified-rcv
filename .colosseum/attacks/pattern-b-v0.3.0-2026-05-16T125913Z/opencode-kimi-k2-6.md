# kimi-k2-6 — verified-rcv intent v0.3.0 adversarial pass (Pattern B, per-section dispatch)

- **Pass**: Pattern B calibration (4th attack against v0.3.0 — same intent, OpenCode + file-access subagent dispatch)
- **Slices dispatched**: 9

---

## Slice: scope

*Elapsed: 143.1s, 9,119 chars.*

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
---

## Slice: behaviors-core

*Elapsed: 273.6s, 8,160 chars.*

# kimi-k2-6 — slice behaviors-core

## Cross-section reads
- Section 2.5 Block 6 (`publish_result` error mapping) — to verify the ground-truth error codes for `derived state ≠ Tallying` and well-formedness checks.
- Section 2.5 `TallyResult` schema comment — to verify the "parallel to per_round_counts" definition of `eliminated_by_round`.
- Section 6.1 trust boundary — to verify the block-time monotonicity assumption (non-decreasing, not strictly increasing).

## Attacks on this slice

### 1. `publish_result` error mapping contradicts Block 6 [critical]
- **Category**: contradiction
- **Affected**: 2.4 Explicit failures table, rows `VotingStillOpen`, `AlreadyResolved`, `NotInTallyingState`
- **What's wrong**: The table maps `publish_result` before `end_at` to `VotingStillOpen` and `publish_result` after `tally_result.is_some()` to `AlreadyResolved`. However, Block 6's ground-truth error mapping assigns `derived state ≠ Tallying → NotInTallyingState` for any `publish_result` call where the state is not Tallying, which includes both Voting (before `end_at`) and Resolved (after `tally_result.is_some()`). The table therefore contradicts the structured behavior block and is internally inconsistent: the `NotInTallyingState` row says "see `AlreadyResolved`" for Resolved, but the `AlreadyResolved` row's cause description doesn't align with Block 6's mapping.
- **Cite**: > `| VotingStillOpen | close_and_tally or publish_result before end_at | Yes — wait for end_at |` and `| AlreadyResolved | Any state-mutating handler after tally_result.is_some() | No — election is terminal |` and `| NotInTallyingState | publish_result called when derived state is not Tallying | Yes if state was Created/Voting (wait); No if state was Resolved (terminal — see AlreadyResolved) |`
- **Fix recommendation**: Align the Explicit failures table with Block 6: `publish_result` in any non-Tallying state returns `NotInTallyingState`. Remove `publish_result` from the `VotingStillOpen` cause and remove the "see `AlreadyResolved`" note from the `NotInTallyingState` row. If `AlreadyResolved` is desired for Resolved state, update Block 6 to check `tally_result.is_some()` first.

### 2. Happy path `eliminated_by_round` omits final-round empty list [serious]
- **Category**: contradiction
- **Affected**: 2.1 Happy path, output block
- **What's wrong**: The happy path output shows `per_round_counts` with 3 entries (rounds 1–3) but `eliminated_by_round` with only 2 entries. Section 2.5 defines `eliminated_by_round` as "parallel to per_round_counts", implying identical length and indexing. Round 3 eliminated no one, so the parallel array should include `[]` as its third entry. The omission creates ambiguity about whether implementers should omit empty elimination rounds or maintain strict parallelism.
- **Cite**: > `per_round_counts    = [ {A: 2, B: 0, C: 1, D: 1, E: 1}, {A: 2, C: 1, D: 1, E: 1}, {A: 5} ]` and `eliminated_by_round = [[B], [C, D, E]]`
- **Fix recommendation**: Append `[]` to `eliminated_by_round` in the happy path output, or amend the 2.5 schema comment to clarify that "parallel" permits omitting rounds with no eliminations.

### 3. `ballots_dropped = N` uses undefined symbol [serious]
- **Category**: ambiguity
- **Affected**: 2.2 Boundary cases, "All ballots dropped as malformed"
- **What's wrong**: The text states `ballots_dropped = N` without defining `N`. An implementer cannot determine whether `N` equals `len(candidates)`, the number of submitted ballots, or some other value. Since the case asserts `non_voters = ∅`, all candidates must have submitted a ballot, so `N` should equal `len(candidates)`, but this is never made explicit.
- **Cite**: > `All ballots dropped as malformed (ballots_tallied = 0, ballots_dropped > 0) — same shape as all-abstain, but non_voters = ∅ and ballots_dropped = N.`
- **Fix recommendation**: Replace `N` with `len(candidates)` and explicitly state that this case assumes every candidate submitted exactly one malformed ballot.

### 4. Missing mixed abstain + malformed boundary case [serious]
- **Category**: coverage gap
- **Affected**: 2.2 Boundary cases
- **What's wrong**: The boundary cases cover all-abstain, all-dropped, and some-abstain-with-valid-ballots, but omit the reachable case where some candidates abstain, some submit malformed ballots, and some submit valid ballots. This mixed case exercises the conservation invariant `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)` in a non-trivial way and is the natural composition of the existing partial cases. Its absence leaves implementers without a worked example for the most general non-homogeneous input.
- **Cite**: > Boundary cases list: "Single candidate", "Two candidates, both vote, terminal 1-1 tie", "All candidates abstain", "Some candidates abstain", "All ballots dropped as malformed".
- **Fix recommendation**: Add a worked example with 4 candidates: 1 abstains, 1 submits malformed, 2 submit valid ballots, showing expected `ballots_tallied`, `ballots_dropped`, `non_voters`, and the IRV trace.

### 5. `InvalidTally` claims batch-elim check that Block 6 omits [serious]
- **Category**: contradiction
- **Affected**: 2.4 Explicit failures, `InvalidTally` row
- **What's wrong**: The `InvalidTally` cause description includes "batch-elim policy violation" as a checked condition. However, Block 6's well-formedness predicate — the actual on-chain validation gate — does not include any clause verifying that eliminations followed the batch-elimination rule. It only checks monotonicity, count conservation, set relations, and winner well-formedness. A tally that eliminates only one of two tied-lowest candidates would pass Block 6's checks but violates the stated batch-elim policy, meaning the chain would accept an invalid tally.
- **Cite**: > `InvalidTally { reason } | Result fails well-formedness (winner ∉ candidates, counts don't conserve, elimination sequence inconsistent, set-relation violation, batch-elim policy violation) | ...`
- **Fix recommendation**: Either add an explicit batch-elimination validation clause to Block 6 (e.g., "for each round i, eliminated_by_round[i] equals the set of candidates with minimum count in per_round_counts[i]") or remove "batch-elim policy violation" from the `InvalidTally` cause description.

### 6. Round 1 redistribution description is misleading [cosmetic]
- **Category**: ambiguity
- **Affected**: 2.1 Happy path, Round 1 row
- **What's wrong**: The text says "Redistribute B's ballot ... which already counts B's first-place vote — no movement." This is misleading because B had zero first-place votes in Round 1; there was no B-first-place vote to redistribute. The ballot was already counted for A. The phrasing implies a redistribution operation was performed and found no movement, when in fact no redistribution was needed.
- **Cite**: > `B has 0 first-place votes (unique lowest). Eliminate B. Redistribute B's ballot: B's first surviving choice is A (since A > B > … after dropping B is A > C > D > E), which already counts B's first-place vote — no movement.`
- **Fix recommendation**: Clarify that B had zero first-place votes, so no redistribution occurs; the ballot was already counted for A.

### 7. `start_at` edge case assumes strict per-block time advance [cosmetic]
- **Category**: edge case
- **Affected**: 2.3 Edge cases, "Instantiation with start_at close to env.block.time"
- **What's wrong**: The text claims the contract "enters Voting state on the very next block after start_at". This assumes block time strictly increases every block. The trust boundary in Section 6.1 only assumes non-decreasing monotonicity. If block time stalls across multiple blocks, the contract may remain in Created longer than stated.
- **Cite**: > `Instantiation with start_at close to env.block.time — admissible as long as start_at > env.block.time; the contract enters Voting state on the very next block after start_at.`
- **Fix recommendation**: Replace with "once a block with env.block.time ≥ start_at is produced".

## Slice-local summary
- Critical: 1
- Serious: 4
- Cosmetic: 2

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: behaviors-types

*Elapsed: 219.3s, 10,227 chars.*

# kimi-k2-6 — slice behaviors-types

## Cross-section reads
- §3.2 B6 (line 344) — verified that B6's ∀-per-key form quantifies over `fires_at_transition(σ → σ′)` and binds `tx.msg.sender = k`; checked whether the trace model definition in §2.5 actually supports this.
- §3.2 B10_lean (line 349) — confirmed `EnclaveImage(raw_ballots, candidates, privkey)` is used there with arity 3, while the CONTEXT_APPENDIX claims `EnclaveImage : Bytes → Bytes` and says "full def in §2.5".
- §2.3 edge cases (lines 81-92) — checked whether ballot exhaustion is addressed elsewhere; it is not.

## Attacks on this slice

### 1. Map iteration discipline is undefined, making Tally_spec non-deterministic [critical]
- **Category**: under-specification
- **Affected**: Stage 1 `decrypt_and_validate`, lines 155–159
- **What's wrong**: The spec states that `dropped_voters` is "ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline)." It never defines what that discipline is. In CosmWasm, `Map` iteration is lexicographic by the key's byte representation, but the spec doesn't say this. If the Lean model assumes candidate-declaration order (the ordering used for `non_voters` on the same line) while the Rust implementation uses lexicographic address order, the two will produce different `dropped_voters` vectors for the same logical input. Because `dropped_voters` is a field of `TallyResult`, its Borsh serialization (B8 clause (c)) will differ, causing attestation hash mismatches between an honest Lean model and an honest Rust enclave. The spec claims `Tally_spec` is canonical but leaves the ordering axiom unstated.
- **Cite**: > "For each `(addr, ciphertext) ∈ raw_ballots`: ... `dropped_voters` ordered by iteration order over `raw_ballots` (deterministic per Map iteration discipline)."
- **Fix recommendation**: Explicitly state the iteration order for `raw_ballots` (e.g., lexicographic ascending by `Addr` string, or candidate-declaration order if the Map is maintained that way). Make the same choice for `dropped_voters` and `non_voters` to avoid ordering mismatches.

### 2. Borsh serialization of logical `Map<Addr, Nat>` is type-ambiguous [critical]
- **Category**: ambiguity
- **Affected**: `canonical_serialization` paragraph, lines 136–140
- **What's wrong**: The spec says "`Vec<Addr>` and `Map<Addr, Nat>` entries are emitted in **candidate-declaration order**, making `borsh_bytes(tally_body)` a deterministic function of the logical value." Borsh does not have a native `Map` type; the concrete Rust type determines the serialized bytes. If the implementation uses `BTreeMap<Addr, Nat>`, Borsh will serialize entries in key-sorted order (by `Addr`), not candidate-declaration order. If it uses `HashMap`, order is arbitrary. If it uses `Vec<(Addr, Nat)>`, order is preserved. The spec pins the logical schema but not the concrete type, so two honest implementations can produce different byte representations for the same logical `TallyResult`, breaking B8(c) attestation binding.
- **Cite**: > "`Vec<Addr>` and `Map<Addr, Nat>` entries are emitted in **candidate-declaration order** (matching the ordering discipline above), making `borsh_bytes(tally_body)` a deterministic function of the logical value."
- **Fix recommendation**: Pin the concrete Rust type to `Vec<(Addr, Nat)>` (or an equivalent ordered sequence type) and mandate that it is populated in candidate-declaration order. Explicitly forbid `BTreeMap` or `HashMap` for these fields.

### 3. Transaction trace model's "slot" is undefined and conflates chain txs with contract messages [serious]
- **Category**: ambiguity
- **Affected**: Transaction trace model paragraph, line 149
- **What's wrong**: The spec defines `fires_at_transition(σ → σ′)` as "the multiset of transactions that fired in the block taking `σ` to `σ′` — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order." The term "slot" is never defined. In CosmWasm, a single chain transaction can contain multiple `MsgExecuteContract` messages, each with the same `msg.sender`. If `fires_at_transition` contains chain transactions, then a single `tx` can carry two `SubmitBallot` messages from the same sender, making `tx.kind` ill-defined and breaking B6's `tx.kind = SubmitBallot ∧ tx.msg.sender = k` binding. If `fires_at_transition` contains contract messages, the spec should say so explicitly. The "at most one per slot per sender" claim is also factually false for CosmWasm blocks, where a sender can appear in multiple transactions.
- **Cite**: > "A state transition `σ → σ′` is parameterized by the multiset of transactions `fires_at_transition(σ → σ′)` that fired in the block taking `σ` to `σ′` — at most one per slot per sender per block, ordered by CosmWasm's deterministic execution order."
- **Fix recommendation**: Replace "transactions" with "contract messages" (or "handler invocations") and drop the "at most one per slot" clause. Define `fires_at_transition` as the ordered list of `ExecuteMsg` payloads processed by the contract in the block, each with its own `msg.sender`.

### 4. IRV_spec carries voter identities that are semantically irrelevant to the combinatorial core [serious]
- **Category**: refinement mismatch
- **Affected**: `IRV_spec` signature and algorithm, lines 161 and 177
- **What's wrong**: The spec says `IRV_spec(valid_ballots, candidates)` where `valid_ballots: Map<Addr, Vec<Addr>>`. The IRV combinatorial core operates on anonymous ballots (preference lists), not on voter addresses. The keys are never used by the algorithm: `ballots_tallied := |valid_ballots|` only needs a count, and the redistribution step iterates over ballots, not voters. By forcing `IRV_spec` to take a `Map`, the spec creates a refinement mismatch: the Lean proof must reason about a Map structure whose keys are semantically inert, or an implicit "forget keys" step must exist that is not written down. An honest semantic split would have `IRV_spec` take `Vec<Vec<Addr>>` (a multiset of ballots), making the Lean proof obligation purely combinatorial.
- **Cite**: > "**Stage 2 — `IRV_spec(valid_ballots, candidates) → (winners, per_round_counts, eliminated_by_round, ballots_tallied)`**" and "`ballots_tallied := |valid_ballots|` (the count of ballots fed into the recursion)."
- **Fix recommendation**: Redefine `IRV_spec` to take `ballots: Vec<Vec<Addr>>` (or `Multiset<Ballot>`). Have `Tally_spec` extract the values from `valid_ballots` before calling `IRV_spec`, so the Lean theorem is about anonymous ballots and the chain-side proof links the Map to the Vec.

### 5. EnclaveImage is absent from §2.5 despite appendix claim and downstream dependency [serious]
- **Category**: coverage gap
- **Affected**: Entire slice (missing definition)
- **What's wrong**: The CONTEXT_APPENDIX states "Type signatures (full def in §2.5): EnclaveImage : Bytes → Bytes". However, §2.5 (this slice) contains no definition of `EnclaveImage`. The symbol is used in §3.2 B10_lean but is never introduced in the behavior blocks. The slice does mention "attested enclave identity (MRTD / RTMR) matches the expected `verified-rcv` enclave image" (Block 6, line 260), but it never defines what an "enclave image" is — a binary blob? a hash? a measurement? This leaves `EnclaveImage` as a free variable in the intent document and makes B10_lean ungrounded.
- **Cite**: > "attested enclave identity (MRTD / RTMR) matches the expected `verified-rcv` enclave image" (Block 6, line 260); absence of any `EnclaveImage` type signature or definition in §2.5.
- **Fix recommendation**: Add an `EnclaveImage` type signature to §2.5, e.g., `EnclaveImage : Bytes → Bytes` where the input is the enclave binary and the output is its MRTD/RTMR measurement, and clarify how it relates to the "expected verified-rcv enclave image" in Block 6.

### 6. IRV_spec algorithm omits output ordering, creating refinement gap with canonical_serialization [serious]
- **Category**: refinement mismatch
- **Affected**: `IRV_spec` recursive definition (lines 161–168) and `TallyResult` schema ordering discipline (line 134)
- **What's wrong**: The `IRV_spec` algorithm (lines 161–168) defines the combinatorial result but never specifies the ordering of entries within `per_round_counts[i]` or `eliminated_by_round[i]`. The `TallyResult` schema (line 134) separately mandates that "Ordering of entries within each map and within `Vec<Addr>` fields is deterministic-by-candidate-declaration-order." This creates a refinement mismatch: the Lean proof of `IRV_spec` will prove properties about unordered collections, but the chain-side Borsh serialization and attestation require a specific order. `Tally_spec` is supposed to compose Stage 1 and Stage 2, yet it doesn't include a canonicalization step that orders the outputs. Two honest implementations of `IRV_spec` could produce the same logical result but different serialized bytes.
- **Cite**: > "Ordering of entries within each map and within `Vec<Addr>` fields is deterministic-by-candidate-declaration-order (the order in `candidates` at instantiation); this matters for downstream Lean / Quint encoding to produce reproducible serialization." (line 134); and the `IRV_spec` algorithm (lines 161–168) which never mentions ordering.
- **Fix recommendation**: Add an explicit ordering step to `Tally_spec` (or `IRV_spec`) that states `per_round_counts[i]` and `eliminated_by_round[i]` are emitted in candidate-declaration order, with eliminated candidates omitted from `per_round_counts[i]`.

## Slice-local summary
- Critical: 2
- Serious: 4
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE

The slice has two critical determinism gaps (Map iteration order and Borsh Map type ambiguity) that directly threaten the attestation binding (B8). It also has four serious issues: the trace model is too vague to support B6's ∀-per-key form, the IRV_spec split is semantically dishonest, EnclaveImage is missing despite being load-bearing downstream, and the output ordering requirement is disconnected from the algorithm definition. These are not cosmetic or speculative; they are concrete under-specifications that will cause Lean↔Rust mismatches or vacuous proof obligations.
---

## Slice: state-invariants

*Elapsed: 371.1s, 6,770 chars.*

# kimi-k2-6 — slice state-invariants

## Cross-section reads
- Section 2.5 Block 6 Requires (lines 261–266) — to ground coverage-gap claim that four set-relation well-formedness clauses are missing from the S-series table
- Section 2.5 `TallyResult` schema (lines 123–131) — to verify that `winners`, `dropped_voters`, `non_voters` are `Vec<Addr>` (ordered sequences), not sets, making set-notation in S6 and Block 6 ambiguous
- Section 2.5 transaction-trace model paragraph (line 149) — to confirm "handler index" is not a state variable
- Section 3.2 B3 (line 341) — to evaluate S10's claimed derivation
- Section 6.1 trust boundary (line 475) — to verify block-time monotonicity is an operational assumption, not an invariant

## Attacks on this slice

### 1. S5 is a code property, not a pointwise state invariant [serious]
- **Category**: temporal-state mismatch
- **Affected**: S5 (line 326)
- **What's wrong**: The Section 3.1 preamble requires properties "evaluable on contract state at any reachable moment" with "No quantification over operations or time." S5 claims to satisfy this by being "Evaluable on a single state by inspecting the handler index." But the contract state (Section 2.5) contains only `candidates`, `start_at`, `end_at`, `enclave_pubkey`, `ballots`, and `tally_result` — there is no "handler index" in the state. The property "no handler writes to `tally_result` after it has been set to `Some(_)`" is a statement about the contract's code (which handlers exist and what they do), not about any single state snapshot. It quantifies over operations (handlers), violating the preamble. The restatement does not successfully purge successor quantification; it merely hides it behind an undefined "handler index" pseudo-state.
- **Cite**: > "Evaluable on a single state by inspecting the handler index, not by quantifying over successor states."
- **Fix recommendation**: Remove S5 from Section 3.1. The write-discipline is a static code property that belongs in the handler specification (Block 6). If a state-shape shadow is truly needed, replace S5 with the trivial type-level observation "`tally_result` is either `None` or `Some(_)`", and let B1 carry the temporal load.

### 2. Block 6 set-relation well-formedness clauses missing from S1..S10 [serious]
- **Category**: coverage gap
- **Affected**: S1..S10 table (lines 320–331) vs. Block 6 Requires (lines 261–266)
- **What's wrong**: Block 6's well-formedness predicate includes four pointwise-evaluable set-relation checks: `non_voters = candidates \ ballots.keys`, `dropped_voters ⊆ ballots.keys`, `dropped_voters ∩ non_voters = ∅`, and `len(dropped_voters) = ballots_dropped`. These are pure state-shape properties — they require only reading `ballots`, `candidates`, and the published `tally_result` at a single moment. Yet none appear in the structural invariant table. A reader or downstream spec author treating Section 3.1 as the exhaustive catalog of state-shape invariants will omit them.
- **Cite**: > "set relations (Round 3a first-pass adversary Attack 28 + 35): `non_voters = candidates \ ballots.keys`, `dropped_voters ⊆ ballots.keys`, `dropped_voters ∩ non_voters = ∅`. `len(dropped_voters) = ballots_dropped`."
- **Fix recommendation**: Add explicit structural invariants (e.g., S11–S14) capturing these four set relations in Section 3.1.

### 3. S6 allows duplicate winners and winners eliminated in earlier rounds [serious]
- **Category**: under-specification
- **Affected**: S6 (line 328)
- **What's wrong**: S6 requires `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`. Because `winners` is a `Vec<Addr>` (not a set) and the invariant uses set-subset notation, a malformed result with `winners = [A, A]` satisfies S6 (`{A} ⊆ candidates`, length 2 ≤ `len(candidates)`). More severely, S6 does not require winners to appear in the final round of `per_round_counts`. A tally with `winners = [Z]`, `eliminated_by_round = [[Z]]`, and `per_round_counts = [{A: 5}]` satisfies S6 and S9 (Z does not reappear in later rounds) while being nonsensical — the declared winner was eliminated in round 0. The worked examples (Sections 2.1, 2.2) always show winners as the surviving candidates of the last round, but no structural invariant enforces this.
- **Cite**: > `winners ⊆ candidates ∧ 1 ≤ len(winners) ≤ len(candidates)`
- **Fix recommendation**: Strengthen S6 to: (a) `winners` contains no duplicate addresses, (b) `winners = per_round_counts[last].keys()` (as sets), and mirror this in Block 6's well-formedness predicate.

### 4. S9 permits duplicate eliminations and unrecorded disappearances [serious]
- **Category**: under-specification
- **Affected**: S9 (line 330)
- **What's wrong**: S9 only states that eliminated candidates do not reappear in later rounds' counts. It imposes no constraint on `eliminated_by_round` itself. A buggy enclave could produce `eliminated_by_round = [[B], [B, C]]` (B eliminated twice) and satisfy S9 because S9 never says each candidate is eliminated at most once. Conversely, a candidate could vanish from `per_round_counts[i]` to `per_round_counts[i+1]` without ever appearing in any `eliminated_by_round` entry, and S9 would still hold. The worked examples consistently show `len(per_round_counts) = len(eliminated_by_round) + 1` and pairwise-disjoint elimination rounds, but no invariant codifies either property.
- **Cite**: > "if a candidate appears in `eliminated_by_round[i]`, they appear in no `per_round_counts[j]` for `j > i`"
- **Fix recommendation**: Add structural invariants: (a) the vectors inside `eliminated_by_round` are pairwise disjoint; (b) `len(per_round_counts) = len(eliminated_by_round) + 1` when `tally_result.is_some()`; (c) any candidate present in `per_round_counts[i]` but absent from `per_round_counts[i+1]` must appear in `eliminated_by_round[i]`.

### 5. S10 derivation omits block-time monotonicity premise [cosmetic]
- **Category**: refinement mismatch
- **Affected**: S10 (line 331)
- **What's wrong**: S10 is labeled "Derived from B3." B3 guarantees `env.block.time ≥ end_at` at the exact transition where `tally_result` becomes `Some`. To conclude that `env.block.time ≥ end_at` holds at *all* later reachable states where `tally_result.is_some()`, one needs the additional premise that `env.block.time` is non-decreasing across blocks. This premise lives in Section 6.1 as a trust-boundary assumption, not as an invariant. The one-line "Derived from B3" note is therefore an incomplete proof sketch.
- **Cite**: > "Derived from B3 (temporal causal version)"
- **Fix recommendation**: Amend the derivation note to "Derived from B3 plus block-time monotonicity (Section 6.1)."

## Slice-local summary
- Critical: 0
- Serious: 4
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: temporal-invariants

*Elapsed: 249.2s, 13,789 chars.*

# kimi-k2-6 — slice temporal-invariants

## Cross-section reads
- §2.5 transaction trace model (lines 149-150) — to verify `fires_at_transition(σ → σ′)` definition and binding discipline for B6.
- §2.5 Block 6 / Block E1 (lines 253-310) — to verify B8(c) user_data construction and B10's `Tally_spec` reference.
- §6.1 trust boundary (lines 471-480) — to confirm `image_registration_honest` is operational prose, not a formal predicate.
- §6.2 Quartz inheritance table (lines 486-510) — to verify `Adv_circuit_eq` provenance and `groth16Verifier` bundle tagging.
- §8.7 B10 witness decomposition (lines 684-701) — to verify the factorization `B10 ← B10_lean ∧ image-identity-binding ∧ B8`.

## Attacks on this slice

### 1. B8 clause (c) asserts equality between 64-byte attested user_data and 32-byte SHA-256 digest [critical]
- **Category**: contradiction
- **Affected**: B8 clause (c), line 346
- **What's wrong**: B8(c) states `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))`. The same line's implementation note states the SHA-256 digest is "embedded in the lower half of Quartz's 64-byte `UserData` slot; upper half reserved for domain-separation tag." A 64-byte value cannot equal a 32-byte value. The "implementation detail" parenthetical does not resolve the contradiction—it confirms it. Downstream encoders will either enforce the equality (and reject valid attestations) or enforce the 64-byte construction (and violate B8(c)).
- **Cite**: > `attested user_data = SHA-256(canonical_serialization(contract_addr ‖ tally_body))` … `32-byte digest embedded in the lower half of Quartz's 64-byte UserData slot; upper half reserved for domain-separation tag (implementation detail).`
- **Fix recommendation**: Restate B8(c) as a sub-field equality: `the lower 32 bytes of attested user_data equal SHA-256(...)`, and make the domain-separation tag part of the canonical serialization or an explicit separate clause.

### 2. B9's `image_registration_honest` precondition is a free predicate with no formal definition [critical]
- **Category**: under-specification
- **Affected**: B9, line 347
- **What's wrong**: B9 wraps `image_registration_honest` in an `always` modality and treats it as an antecedent of a formal probability bound. The slice describes it only as an operational assumption: "the chain-side registered verified-rcv vkey + MRTD have not been substituted post-registration; Section 6.1 trust boundary." Cross-reading Section 6.1 confirms it is "an operational assumption on the deployment / governance surface … not a cryptographic property reducible to a security parameter." Because no state predicate, transition relation, or formal typing is supplied, B9 reduces to "if [undefined], then bound holds." It is not evaluable by downstream tools and provides no falsifiable constraint.
- **Cite**: > `always (image_registration_honest → Pr[B8 violated at the next state by a polynomial-time adversary] ≤ …)` — "conditioned on the operational precondition `image_registration_honest` (the chain-side registered verified-rcv vkey + MRTD have not been substituted post-registration; Section 6.1 trust boundary)."
- **Fix recommendation**: Define `image_registration_honest(σ)` as a concrete state predicate (e.g., `chain_registered_vkey(σ) = expected_vkey ∧ chain_registered_mrtd(σ) = expected_mrtd`) and remove it from the `always` modality, restating B9 as a conditional meta-theorem under a static assumption.

### 3. B10 cross-layer factorization omits input-integrity and KMS-derivation conjuncts [critical]
- **Category**: composition failure
- **Affected**: B10 (line 348) and Section 8.7 factorization (line 701)
- **What's wrong**: The slice claims `B10 ← B10_lean ∧ image-identity-binding ∧ B8`. B10_lean proves the enclave image computes `Tally_spec` correctly for *any* inputs. Image-identity-binding proves the on-chain registered image equals the Lean-proven binary. B8 proves the attestation binds the published tally to the registered image. None of the three conjuncts witness that the enclave actually received `ballots@end_at` and `candidates` as inputs, nor that the `privkey` satisfying `dstack_kms_derived(privkey, contract_addr)` was used. A malicious host could feed manipulated ballots to a legitimate enclave image; the enclave would correctly compute `Tally_spec(fake_ballots, candidates, privkey)`, B8 would verify, and B10 would be violated despite all three conjuncts holding. The factorization is therefore insufficient for the claimed conclusion.
- **Cite**: > `B10 ← B10_lean ∧ image-identity-binding ∧ B8` … `next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))` … `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr)`
- **Fix recommendation**: Add an input-integrity conjunct (e.g., `enclave_input = ballots@end_at ‖ candidates`) and a KMS-derivation conjunct to the factorization, or weaken B10 to quantify over the enclave's actual inputs.

### 4. B10_lean's `EnclaveImage` symbol is undefined and arity-mismatched with the type appendix [critical]
- **Category**: under-specification
- **Affected**: B10_lean, line 349
- **What's wrong**: B10_lean introduces `EnclaveImage(raw_ballots, candidates, privkey)` as a 3-ary function symbol representing the enclave's input-output relation. This symbol is not defined anywhere in the intent document prior to this line. The CONTEXT_APPENDIX types `EnclaveImage : Bytes → Bytes` (a 1-ary function on bytes). There is no bridging definition reconciling these arities or types. B10_lean is therefore a free-symbol assertion with no grounding in the document's type system.
- **Cite**: > `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)`
- **Fix recommendation**: Define `EnclaveImage` in §2.5 with its full 3-ary type signature, or replace the symbol with an explicit lambda over the enclave's observable behavior.

### 5. B9 is tagged `temporal` but its body is a probabilistic meta-security statement, not a temporal property [serious]
- **Category**: temporal-state mismatch
- **Affected**: B9, line 347
- **What's wrong**: The slice preamble states that "temporal properties require explicit history quantification downstream" and warns that "the wrong tag silently mis-encodes intent." B9's body is `always (image_registration_honest → Pr[B8 violated … by a polynomial-time adversary] ≤ Σ Adv_i(n))`. The inner `Pr[…]` ranges over adversaries and random coins; it is a cryptographic security statement, not a property of system states or execution trajectories. Tagging it as `temporal` mis-encodes intent: downstream tools will treat B9 as an LTL-style execution invariant when it is actually a meta-theoretic negligibility bound.
- **Cite**: > `B9 | B8 negligibility-budget decomposition | **temporal** | always (image_registration_honest → Pr[B8 violated …] ≤ …)`
- **Fix recommendation**: Retag B9 as `meta-security` or `probabilistic`, or move it out of the behavioral-invariant table entirely into a separate security-budget section.

### 6. B6's ∀-per-key formula mixes LTL modalities with free transition parameters, weakening causal attribution [serious]
- **Category**: ambiguity
- **Affected**: B6, line 344
- **What's wrong**: B6 states `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), …)`. The `always` modality quantifies over states, but `fires_at_transition(σ → σ′)` references explicit transition parameters `σ` and `σ′` that are not bound by the modality. The formula is therefore not well-formed in standard LTL and creates an ambiguity in downstream encoding (Quint action labels vs Lean step relations). Moreover, the existential only requires correlation, not causation: if some unauthorized tx also writes to `ballots[k]` and the final value happens to equal a co-occurring `SubmitBallot` from `k`, B6 is satisfied even though the authorized tx did not cause the write.
- **Cite**: > `always (∀ k ∈ Addr, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences)`
- **Fix recommendation**: Bind the transition explicitly using an action modality (e.g., `[_]_{fires_at_transition}`) or restate B6 as a pure LTL formula over `step` relations. Strengthen the existential to assert that the identified tx is the *unique* writer to `k` in that transition.

### 7. B9's 4-summand decomposition conflates a correctness theorem with a security advantage [serious]
- **Category**: disjunction-vs-decomposition collapse
- **Affected**: B9, line 347; Section 6.2 inheritance table, line 489
- **What's wrong**: B9 decomposes its bound into four summands, including `Adv_circuit_eq(n)`. Section 6.2 states the discharge path for `groth16Verifier` includes a "reference DCAP circuit-equivalence theorem." Circuit equivalence is a binary correctness property (the circuit either correctly encodes the TDX quote validity relation or it does not); it is not a probabilistic security advantage that decreases with a security parameter `n`. By including it in a sum of negligible advantages, B9 collapses a correctness assumption into a security bound, misrepresenting the logical structure. The honest form would separate correctness assumptions (circuit equivalence, image-identity-binding) from cryptographic advantage bounds.
- **Cite**: > `Adv_tdxVerifier_sound(n) + Adv_groth16_KS(n) + Adv_circuit_eq(n) + Adv_commitTally_CR(n)` … `reference DCAP circuit-equivalence theorem`
- **Fix recommendation**: Remove `Adv_circuit_eq` from the negligibility sum and state it as a standalone correctness hypothesis. Restate B9 as a conditional bound under the conjunction of correctness assumptions.

### 8. B8 clause (d) "matches the registered verified-rcv enclave image" is ambiguous [serious]
- **Category**: ambiguity
- **Affected**: B8 clause (d), line 346
- **What's wrong**: B8(d) requires that "attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image." The slice never defines what "matches" means (exact equality? prefix? hash? composite?). Nor does it define the type of the "registered verified-rcv enclave image" — is it an MRTD value, an RTMR value, a pair, a hash, or a build artifact? Section 6.1 calls it "enclave-image identity (MRTD / RTMR)" with a slash, suggesting disjunction or pairing, but B8(d) treats it as a single registered value. This ambiguity makes the clause unenforceable on-chain.
- **Cite**: > `(d) attested enclave identity (MRTD/RTMR) matches the registered verified-rcv enclave image`
- **Fix recommendation**: Define the registered value as a concrete on-chain storage slot (e.g., `chain_registered_mrtd: Bytes32`) and replace "matches" with exact equality: `attested_mrtd = chain_registered_mrtd`.

### 9. Cross-project dependency note promises future ledger recording but supplies no formal propagation mechanism [serious]
- **Category**: coverage gap
- **Affected**: Cross-project dependency note, lines 351-358
- **What's wrong**: The note states that B8 depends on Quartz axioms and that "Any drift in either of those bundles' cardinality at the Quartz level … should propagate as a downstream alert to verified-rcv's ledger." It then says "The verified-rcv compose ledger (forthcoming) will record this dependency explicitly." There is no formal mechanism, invariant, or procedural rule in the slice that ensures this propagation actually happens. The dependency is a hand-waved future promise, not a specified behavior.
- **Cite**: > `The verified-rcv compose ledger (forthcoming) will record this dependency explicitly.`
- **Fix recommendation**: Replace the forward-looking promise with a concrete specification of how bundle-cardinality drift is detected and propagated (e.g., a CI gate, a ledger schema, or a periodic diff check against Quartz's bundle registry).

### 10. Tag preamble does not define `cross-layer` or `off-chain`, silently expanding the tag taxonomy [cosmetic]
- **Category**: ambiguity
- **Affected**: B10 (line 348), B10_lean (line 349), and the tag preamble (line 335)
- **What's wrong**: The slice preamble explicitly limits the tag taxonomy to `state` and `temporal`, stating the distinction is "load-bearing per Colosseum methodology." B10 is tagged `cross-layer` and B10_lean is tagged `off-chain` — neither of which is defined in the preamble. This silently expands the taxonomy without updating the methodological contract, creating ambiguity about discharge obligations for these invariants.
- **Cite**: > `The **state**/**temporal** tag is load-bearing per Colosseum methodology: state invariants discharge at every state independently; temporal properties require explicit history quantification downstream.`
- **Fix recommendation**: Update the preamble to enumerate all valid tags (`state`, `temporal`, `cross-layer`, `off-chain`, `probabilistic`) and their discharge disciplines.

## Slice-local summary
- Critical: 4
- Serious: 5
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE

The slice contains four critical flaws: a type contradiction in B8(c) (64-byte vs 32-byte equality), an undefined precondition in B9 that makes the bound epistemically vacuous, a compositionally insufficient factorization for B10 that omits input-integrity and KMS-derivation witnesses, and a free function symbol in B10_lean with no grounding in the document's type system. These are compounded by five serious issues including a temporal-state mismatch on B9, ambiguous causal attribution in B6, a category error in B9's decomposition, ambiguous matching semantics in B8(d), and an unfulfilled coverage gap in the cross-project dependency note. The slice does not survive adversarial scrutiny.
---

## Slice: failure-modes

*Elapsed: 141.5s, 10,742 chars.*

# kimi-k2-6 — slice failure-modes

## Cross-section reads
- Block 1 (instantiate) — to verify DstackKeyManager failure at instantiation is not covered in §4.
- Block E1 (enclave tally computation) — to check retry semantics and determinism claims in §4.9.
- §2.4 (Explicit failures) — to confirm `InvalidInstantiation` KMS case is a per-handler error, not a system-level failure mode.

## Attacks on this slice

### 1. §4 intro and §4.6 directly contradict on recoverability [critical]
- **Category**: contradiction
- **Affected**: §4 preamble + §4.6
- **What's wrong**: The preamble states as a universal rule: "Per the no-admin-recovery design decision, **no failure here is auto-recoverable from within the election instance** — recovery always means deploying a fresh election with corrected configuration." Yet §4.6's Recovery clause says: "Enclave re-publishes once reconnected. `publish_result` is idempotent by construction ... so retries are safe." This is auto-recovery within the same election instance without any fresh deployment. A universal quantifier in the preamble is falsified by a concrete counter-example three subsections later.
- **Cite**: > "Per the no-admin-recovery design decision, **no failure here is auto-recoverable from within the election instance** — recovery always means deploying a fresh election with corrected configuration."  
  > "Recovery: Enclave re-publishes once reconnected. `publish_result` is idempotent by construction (B1 plus Block 6's `AlreadyResolved` rejection), so retries are safe."
- **Fix recommendation**: Either weaken the preamble to "no failure here is auto-recoverable except §4.6 (network partition)", or reclassify §4.6 recovery as requiring a fresh election instance (which would be false to the actual design). The honest fix is to scope the preamble: "no failure here is auto-recoverable from within the election instance **by administrative action**" — but then §4.6 is also auto-recoverable without admin action, so the exception must be named explicitly.

### 2. §4.3a detection claim contradicts its own cause clause [serious]
- **Category**: contradiction
- **Affected**: §4.3a
- **What's wrong**: The Cause includes "or was unregistered between instantiation and publish_result" — a runtime event. The Detection claim says "Detectable pre-deployment". A runtime unregistration that happens after deployment cannot be detected pre-deployment. The detection clause is therefore too strong and contradicts the temporal scope of the cause.
- **Cite**: > "Cause: The verified-rcv `zkdcap_vkey` was never registered on the chain's ZK module under the expected slot, or was unregistered between instantiation and `publish_result`."  
  > "Detection: Verifiable on-chain — any caller can query the registered vkey and compare to the expected verified-rcv image. Detectable pre-deployment."
- **Fix recommendation**: Split into two sub-modes: (a) never registered — detectable pre-deployment; (b) unregistered at runtime — detectable only by on-chain monitoring after instantiation. Or weaken Detection to "Verifiable on-chain; the never-registered case is detectable pre-deployment."

### 3. §4.7 and §4.5 introduce undefined severity classes [serious]
- **Category**: ambiguity
- **Affected**: §4.7 ("integrity-per-voter"), §4.5 ("canonicality")
- **What's wrong**: The document never defines a severity taxonomy beyond confidentiality / integrity / liveness. §4.7 tags "integrity-per-voter" and §4.5 tags "canonicality" without explaining how these relate to the three base classes. "Integrity-per-voter" appears to be a subset of integrity (one voter's ballot is corrupted); "canonicality" appears to be a liveness/integrity hybrid (fork ambiguity). Because the classes are undefined, a downstream spec writer cannot tell whether a mitigation for "integrity" also covers "integrity-per-voter", or whether "canonicality" needs separate test harnesses.
- **Cite**: > "Voter address compromise — *integrity-per-voter*"  
  > "Chain consensus halt or fork — *liveness or canonicality*"
- **Fix recommendation**: Map every tag to the base C/I/L taxonomy. "integrity-per-voter" → "integrity (per-voter scope)". "canonicality" → either "liveness" (if the issue is lack of a single agreed state) or "integrity" (if different forks admit different winners), or split the failure mode into two sub-modes with distinct tags.

### 4. §4.8 is not a failure mode but consumes a numbered slot [serious]
- **Category**: coverage gap
- **Affected**: §4.8
- **What's wrong**: The section title is "Failure Modes". §4.8 is explicitly labeled "not a failure". Including it in the enumerated failure modes list breaks the boundary of the section and makes automated extraction unreliable. A methodology that requires completeness checking of failure modes cannot distinguish §4.8 from actual failures without parsing the free-text subtitle. The honest place for this content is §5 (Non-Goals), where "No coordination / collusion resistance" is already listed.
- **Cite**: > "Candidate collusion / coordinated voting — *not a failure*"
- **Fix recommendation**: Remove §4.8 from the Failure Modes section and consolidate its content into §5 (Non-Goals), which already contains the matching bullet.

### 5. Missing failure mode: DstackKeyManager unavailability at instantiation [serious]
- **Category**: coverage gap
- **Affected**: §4 (absent)
- **What's wrong**: Block 1 (instantiate) Requires "DstackKeyManager-issued keypair derivable for this contract instance". If the KMS is unavailable at instantiation time, the contract creation fails with `InvalidInstantiation`. This is a system-level liveness failure (election never starts) distinct from §4.2 (KMS compromise, which assumes the KMS was available and later leaked) and §4.6 (network partition of the enclave post-instantiation). The methodology should require that any Block Requires clause with an external dependency has a corresponding failure mode.
- **Cite**: > (Block 1) "Requires: ... DstackKeyManager-issued keypair derivable for this contract instance"
- **Fix recommendation**: Add §4.x "KMS unavailability at instantiation — *liveness*" with cause "DstackKeyManager unreachable during `instantiate`", effect "contract creation fails; election never starts", detection "failed instantiation transaction", mitigation "operational KMS health checks".

### 6. Missing failure mode: enclave resource exhaustion during tabulation [serious]
- **Category**: coverage gap
- **Affected**: §4 (absent)
- **What's wrong**: Block E1 runs ECIES decryption plus IRV recursion inside a TDX enclave. For large candidate sets the working set could exceed enclave memory or CPU time limits. The enclave would abort without producing an attestation, causing a liveness failure that is distinct from §4.6 (network partition — the enclave is running but cannot reach the chain). The intent does not bound `len(candidates)`, so this failure is admissible. The methodology should require acknowledging resource-bound failures for unbounded inputs.
- **Cite**: > (Block E1) "Stage 1 — `decrypt_and_validate`: for each `(addr, ciphertext) ∈ ballots` ... Stage 2 — `IRV_spec`: run the IRV recursion on `(valid_ballots, candidates)`"
- **Fix recommendation**: Add §4.x "Enclave resource exhaustion — *liveness*" with cause "candidate set or ballot count exceeds enclave memory/CPU limits", effect "enclave aborts without attestation; election deadlocks", mitigation "pre-deployment bench-marking + candidate-set size cap".

### 7. §4.9 deadlock soundness relies on unstated enclave retry semantics [serious]
- **Category**: under-specification
- **What's wrong**: §4.9 claims "re-running the enclave produces the same malformed tally and `publish_result` rejects again. Infinite rejection loop." and "Detection: On-chain — repeated `InvalidTally` rejection events". However, Block E1 does not specify that the enclave retries at all. If the enclave is a one-shot process (run once, submit, exit on any error), there is exactly one `InvalidTally` event and then silence. The "infinite loop" and "repeated events" claims are only true under an unstated retry policy. Without that policy, the deadlock is silent and the detection mechanism fails.
- **Cite**: > "The enclave is deterministic over the ballot set; the ballot set is frozen at `end_at` (B2). Therefore re-running the enclave produces the *same* malformed tally and `publish_result` rejects again. Infinite rejection loop."  
  > "Detection: On-chain — repeated `InvalidTally` rejection events with identical `reason` field."
- **Fix recommendation**: Add an explicit requirement in Block E1 that the enclave must retry `publish_result` indefinitely (or with a documented backoff) until success or operator intervention. Without this, §4.9's detection claim is ungrounded.

### 8. §4.1 "potentially integrity" is ambiguous under the stated threat model [serious]
- **Category**: ambiguity
- **Affected**: §4.1
- **What's wrong**: §4.1 tags the failure as "confidentiality + potentially integrity". The text says integrity is lost only "if the vulnerability also permits forging attestations". But the Mitigation says this is "Inherited from Quartz `tdxVerifier` soundness assumption" — meaning that within the threat model, forging attestations is bounded by a negligible advantage. Therefore integrity loss is either out-of-scope (contingent on an upstream assumption) or impossible under the threat model. The "potentially" tag is misleading because it suggests integrity loss is an in-scope risk comparable to confidentiality loss.
- **Cite**: > "Effect: Adversary decrypts all on-chain ballots. If the vulnerability also permits forging attestations, integrity is also lost."  
  > "Mitigation: Inherited from Quartz `tdxVerifier` soundness assumption. Out of scope at the verified-rcv layer."
- **Fix recommendation**: Split into two sub-modes or re-tag: "confidentiality (in-scope); integrity (out-of-scope — contingent on `tdxVerifier` soundness failure, see Quartz spec)". Alternatively, if integrity is considered in-scope, the mitigation must address it at the verified-rcv layer, which the text says is out of scope.

## Slice-local summary
- Critical: 1
- Serious: 7
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE
The slice contains a critical self-contradiction between the universal preamble and §4.6's recoverability, multiple undefined severity tags, a non-failure occupying a failure-mode slot, and two significant coverage gaps (instantiation-time KMS unavailability, enclave resource exhaustion). Additionally, §4.9's deadlock detection claim is under-specified because it assumes unstated enclave retry behavior. The slice does not survive adversarial scrutiny.
---

## Slice: trust-quartz

*Elapsed: 432.4s, 14,326 chars.*

# kimi-k2-6 — slice trust-quartz

## Cross-section reads
- §3.2 B9 (lines 347) — to verify `image_registration_honest` is used as a formal antecedent in an `always`-modality probability bound with 4 summands.
- §4.1 (lines 364-370) — to confirm TDX compromise effect includes "forging attestations" (integrity loss).
- §4.3a/4.3b (lines 380-394) — to confirm failure modes cover vkey registration issues, not ZK verification-algorithm bugs.
- Block 1 (§2.5, lines 185-200) — to confirm `instantiate` stores only `candidates`, `start_at`, `end_at`, `enclave_pubkey`; no vkey or MRTD/RTMR is stored.

## Attacks on this slice

### 1. De-retracted status block over-claims substrate readiness [critical]
- **Category**: composition failure
- **Affected**: §6.2 B8/B9 substrate status block `[2026-05-15 — de-retracted]`
- **What's wrong**: The banner proclaims the retraction "resolved upstream on 2026-05-14/15" and asserts "verified-rcv inherits B8/B9 from a now-contented substrate." But the caveats immediately below admit that Quartz asks 6 and 7 "have documented form in the Quartz PR; enforced form is gated on the executable-layer decision." This means the substrate is only partially ready — critical methodology obligations (per-conjunct failure-mode analysis and degenerate-zero-advantage intent declaration) are documented but not enforced. Furthermore, the document cites commit paths in the Quartz repo but provides no evidence that verified-rcv independently audited those commits. The "now-contented" claim is a statement about Quartz's state, not verified-rcv's verification of it. By presenting a clean resolution while hiding unenforced gaps in caveats, the spec misleads readers about the solidity of its cryptographic foundation.
- **Cite**: > "The retraction has been resolved upstream on 2026-05-14/15." ... "These asks have documented form in the Quartz PR; enforced form is gated on the executable-layer decision."
- **Fix recommendation**: Restate the status banner as `[2026-05-15 — partially de-retracted]` and add an explicit caveat that verified-rcv has not independently re-verified the Quartz refactor; the inheritance claim is provisional pending enforcement of asks 6+7 and a verified-rcv-side audit.

### 2. `image_registration_honest` is undefined operational prose wrapped in B9's formal modality [critical]
- **Category**: ambiguity
- **Affected**: §6.1 Enclave-image registration integrity bullet (line 480)
- **What's wrong**: The bullet describes the assumption in operational prose: "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an operational assumption on the deployment / governance surface... not a cryptographic property reducible to a security parameter." But B9 (§3.2) uses `image_registration_honest` as a formal antecedent in `always (image_registration_honest → Pr[B8 violated ...] ≤ ...)`. The slice never defines `image_registration_honest` as a state predicate, transition relation, or formal typing — it is governance prose masquerading as a logical symbol. Because it is undefined, B9 reduces to "if [unspecified operational condition], then bound holds" — epistemically equivalent to having no bound at all for the operational-failure case. The trust boundary section fails to supply the formal definition that B9 requires.
- **Cite**: > "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification. This is an **operational** assumption on the deployment / governance surface of the chain's vkey-registration mechanism, not a cryptographic property reducible to a security parameter."
- **Fix recommendation**: Define `image_registration_honest` as a concrete evaluable predicate, e.g., `chain_registered_vkey(σ) = expected_vkey ∧ chain_registered_mrtd(σ) = expected_mrtd`, with explicit evaluation semantics (evaluated at the `publish_result` transition, not under an `always` modality). Alternatively, restate B9 as a conditional meta-theorem under a static assumption rather than a temporal invariant.

### 3. ZK module "iff" assumption lacks failure mode coverage [serious]
- **Category**: coverage gap
- **Affected**: §6.1 ZK module endpoint semantics bullet (line 478)
- **What's wrong**: The bullet states: "`/xion.zk.v1.Query/ProofVerifyGnark` returns true iff the supplied proof verifies against the registered vkey under gnark's verification algorithm." The "iff" is a strong functional-correctness claim — no false positives, no false negatives. But the document lists no failure mode for "ZK module verification algorithm is buggy" or "gnark implementation diverges from spec." Failure modes 4.3a and 4.3b cover vkey registration issues, not the verification algorithm itself. If the ZK module has a bug that accepts invalid proofs, B8 clause (b) is violated with high probability, yet this is not captured by B9's negligibility budget or any failure mode. The Non-Goals section (§5) excludes "formal verification of upstream-Quartz axioms," but the ZK module is an Xion chain component, not a Quartz artifact — so the exclusion does not apply. This is an unacknowledged, load-bearing assumption.
- **Cite**: > "ZK module endpoint semantics — `/xion.zk.v1.Query/ProofVerifyGnark` returns true iff the supplied proof verifies against the registered vkey under gnark's verification algorithm. Trusted as Xion-spec'd; not re-verified at this layer."
- **Fix recommendation**: Add a failure mode for ZK module algorithmic compromise (e.g., "ZK verification bug — accepts invalid proofs"), or weaken the "iff" to "is intended to return true iff" with an explicit caveat that module correctness is assumed and not verified at this layer.

### 4. Groth16 inheritance row conflates two distinct B9 summands [serious]
- **Category**: disjunction-vs-decomposition collapse
- **Affected**: §6.2 Quartz inheritance table, row 2 (`verifyGroth16`, line 489)
- **What's wrong**: The table lists a single row for `verifyGroth16` with discharge path "ArkLib Groth16 KS reduction + reference DCAP circuit-equivalence theorem." But B9's negligibility bound contains TWO separate Groth16-related summands: `Adv_groth16_KS(n)` and `Adv_circuit_eq(n)`. The status block confirms this decomposition: "the `tdxVerifier`-tagged + two `groth16Verifier`-tagged summands of that lift." A single table row with a combined discharge path obscures the mapping. A reader cannot tell whether `verifyGroth16` is one claim with two reduction steps, or whether two separate Quartz claims are collapsed into one row. The honest form would decompose this into named summands matching B9's budget.
- **Cite**: > "`verifyGroth16` accepts a proof iff it is a valid Groth16 proof of the encoded TDX-quote-validity statement under the registered vkey" ... "ArkLib Groth16 KS reduction + reference DCAP circuit-equivalence theorem"
- **Fix recommendation**: Split the `verifyGroth16` row into two rows — one for `Adv_groth16_KS` (Groth16 knowledge-soundness) and one for `Adv_circuit_eq` (DCAP circuit-equivalence) — each with its own discharge path and explicit mapping to the B9 summand it supports.

### 5. TDX integrity assumption is not connected to B9's precondition structure [serious]
- **Category**: composition failure
- **Affected**: §6.3 TDX integrity bullet (line 515)
- **What's wrong**: §6.3 states "TDX integrity — the TDX platform isolates the enclave's memory from the untrusted host... Failure mode 4.1." Cross-reading §4.1 confirms that a TDX vulnerability "also permits forging attestations" — i.e., B8 can be violated. But B9 (§3.2) has no precondition for TDX integrity and no summand for TDX compromise. If TDX is compromised, a polynomial-time adversary can forge attestations with probability ≈ 1, which is not bounded by the 4 negligible summands. §6.3 does not flag that B9's bound implicitly depends on this assumption. The trust boundary presents TDX integrity as a standalone item without explaining its load-bearing role for B9, creating a dangerous gap: a reader might believe B9 holds regardless of TDX state.
- **Cite**: > "TDX integrity — the TDX platform isolates the enclave's memory from the untrusted host; the enclave's runtime state is not observable to the host operator. Failure mode 4.1."
- **Fix recommendation**: Add an explicit note to §6.3: "B9's negligibility bound is only valid under this assumption; TDX compromise is outside the cryptographic model and would falsify the bound." Alternatively, add a `tdx_integrity_honest` precondition to B9.

### 6. commitHashE NOT CONSUMED reason omits finite-type mismatch [serious]
- **Category**: under-specification
- **Affected**: §6.2 Quartz inheritance table row 4 (`commitHashE`, line 491) and Note A (line 493)
- **What's wrong**: Note A explains that `commitHashE` is NOT CONSUMED because "that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous." But the table's discharge path says "VCVio `randomOracle` + `[Fintype UserData]` carrier refinement." The `Fintype UserData` assumption means the user data space is finite. Verified-rcv's `Adv_commitTally_CR` bounds SHA-256 collision resistance on `Borsh(contract_addr ‖ tally_body)`, where the input space is NOT finite (contract addresses and tally bodies vary in size). The document never mentions this finite-type mismatch as a reason for non-consumption. The stated reason is a methodological classification issue; the unstated reason is a technical incompatibility. By omitting the type mismatch, the document prevents readers from assessing whether `commitHashE` could be adapted for verified-rcv or whether the two hash assumptions are fundamentally incomparable.
- **Cite**: > "VCVio `randomOracle` + `[Fintype UserData]` carrier refinement; **NOT CONSUMED by verified-rcv — see Note A below**" ... "that bundle's `(d) pigeonhole-impossible` sub-tag would make any inheritance vacuous."
- **Fix recommendation**: Add to Note A: "Additionally, `commitHashE`'s `[Fintype UserData]` carrier does not apply to verified-rcv's variable-length `Borsh(contract_addr ‖ tally_body)` inputs, making the bundle technically incompatible even if the sub-tag were content-bearing."

### 7. Block time monotonicity lacks failure mode [serious]
- **Category**: coverage gap
- **Affected**: §6.1 Block time monotonicity bullet (line 475)
- **What's wrong**: The bullet assumes "`env.block.time` is non-decreasing across consecutive blocks" and notes this is "Used by every state-derived predicate (Created / Voting / Tallying)." But there is no failure mode for non-monotonic block time. In Cosmos SDK, block timestamps are proposer-determined and can go backwards (malicious or buggy proposer). If `env.block.time` decreases, a contract in Tallying or Resolved could revert to Voting or Created, breaking temporal invariants B3 (no premature tally) and B4 (no premature voting). The document acknowledges fork resolution as a trust boundary but not block-time regression as a failure scenario. This is a coverage gap — the assumption is load-bearing but unguarded.
- **Cite**: > "Block time monotonicity — `env.block.time` is non-decreasing across consecutive blocks. Used by every state-derived predicate (Created / Voting / Tallying)."
- **Fix recommendation**: Add a failure mode for non-monotonic block time (e.g., "Block time regression — proposer produces block with earlier timestamp, causing derived-state oscillation"), or explicitly state that this assumption is unguarded and chain-dependent.

### 8. Enclave-image registration integrity conflates vkey and MRTD/RTMR [serious]
- **Category**: disjunction-vs-decomposition collapse
- **Affected**: §6.1 Enclave-image registration integrity bullet (line 480)
- **What's wrong**: The bullet says "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification." This treats vkey and MRTD/RTMR as a single "registration." But they are used in different B8 clauses (vkey in clause (b) for zkdcap verification; MRTD/RTMR in clause (d) for enclave identity matching) and likely stored in different chain mechanisms. They can be compromised independently: governance could substitute the vkey while leaving the MRTD intact, or vice versa. Lumping them together means a partial compromise is not covered. Also, "at instantiation" is misleading — Block 1 stores only `candidates`, `start_at`, `end_at`, `enclave_pubkey`; the vkey and MRTD are assumed to exist on-chain, not stored by the contract.
- **Cite**: > "the chain stores the registered verified-rcv vkey + enclave-image identity (MRTD / RTMR) at instantiation, and the registration is not subject to undetected post-registration modification."
- **Fix recommendation**: Decompose into two separate bullets — "Vkey registration integrity" and "Enclave-image identity (MRTD/RTMR) registration integrity" — each with its own failure mode reference. Clarify that "at instantiation" means "assumed correct on-chain at the time of contract instantiation," not "stored by the contract during instantiation."

## Slice-local summary
- Critical: 2
- Serious: 6
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE

The trust-quartz slice contains two critical issues and six serious issues. The de-retracted status block over-claims the readiness of the Quartz substrate while hiding unenforced methodology gaps (Attack 1). The `image_registration_honest` predicate is undefined operational prose that B9 treats as a formal antecedent, making the bound unverifiable (Attack 2). Additionally, the ZK module assumption lacks a failure mode (Attack 3), the Groth16 inheritance row collapses two summands into one (Attack 4), TDX integrity is disconnected from B9's precondition structure (Attack 5), the commitHashE non-consumption reason is incomplete (Attack 6), block time monotonicity has no failure mode (Attack 7), and the registration integrity bullet conflates two independently compromisable values (Attack 8). These flaws materially weaken the spec's honesty about its trust boundaries and cross-project composition.
---

## Slice: scenarios

*Elapsed: 176.9s, 9,964 chars.*

# kimi-k2-6 — slice scenarios

## Cross-section reads
- Section 3.2 (B9 definition) — to verify the 4-summand bound vs 8.1's "5-summand" claim.
- Section 6.2 (B9 decomposition lineage) — to confirm the 4-summand count and the removal of the 5th summand in prior revision.

## Attacks on this slice

### 1. Stale B9 summand count in 8.1 contradicts Section 3.2 [critical]
- **Category**: contradiction
- **Affected**: §8.1, Trust claim consumed line
- **What's wrong**: §8.1 states the trustworthiness of the published result "reduces to the 5-summand negligibility budget defined in Section 3." Section 3.2's B9, however, is explicitly a "4-summand bound, conditioned on the operational precondition `image_registration_honest`." The 5th summand (`Adv_image_registration`) was removed per Round 3a first-pass adversary Attack 18 and the decomposition lineage in §6.2 confirms "4 after Round 3a first-pass adversary (dropped `Adv_KMS_leakage`... and `Adv_image_registration`)". The scenario text is stale relative to the invariant definition, creating a direct numerical contradiction that misleads downstream spec writers about the security budget cardinality.
- **Cite**: > "Trust claim consumed: B8 + B9 — the published result is bound to the verified-rcv enclave's attested computation, and its trustworthiness reduces to the 5-summand negligibility budget defined in Section 3."
- **Fix recommendation**: Change "5-summand" to "4-summand" in §8.1 and §8.2, and add a parenthetical note naming the dropped summand so readers understand the revision history.

### 2. B9 is a probabilistic bound, not witnessable by a narrative trace [serious]
- **Category**: under-specification
- **Affected**: §8.1 and §8.2, Trust claim consumed lines
- **What's wrong**: §8.1 and §8.2 claim to "consume" B9 as a trust claim witnessed by the scenario. B9 is a negligibility-budget inequality: `Pr[B8 violated by a polynomial-time adversary] ≤ sum of negligible functions`. A single happy-path narrative trace cannot witness, test, or demonstrate a probability bound over all polynomial-time adversaries. Claiming B9 is "consumed" by a functional scenario conflates a cryptographic security statement with a functional test case, misleading downstream verification about what kind of artifact actually discharges B9 (it requires a proof, not a scenario).
- **Cite**: > "Trust claim consumed: B8 + B9 — the published result is bound to the verified-rcv enclave's attested computation, and its trustworthiness reduces to the 5-summand negligibility budget defined in Section 3."
- **Fix recommendation**: Remove B9 from the "Trust claim consumed" lines in §8.1 and §8.2. Replace with a note that B9 is a proof obligation (discharged by the Lean/Quartz composition), not a scenario-level witness. B8 alone is the correct trace-level claim for these scenarios.

### 3. B1, B2, B3, and B7 lack scenario witnesses in §8.1–§8.6 [serious]
- **Category**: coverage gap
- **Affected**: §8.1–§8.6 overall; specifically B1, B2, B3, B7 from §3.2
- **What's wrong**: The slice is characterized as "witnesses for B1..B6, B10," yet no scenario in §8.1–§8.6 explicitly witnesses B1 (tally_result monotone-once-set), B2 (no late ballots — ballot store frozen at end_at), B3 (no premature tally — causal gate that publish_result requires env.block.time ≥ end_at), or B7 (terminal-state immutability — Resolved is a sink). B1 is mentioned only as a supporting assertion in §8.5's B5 witness; B2 is conflated with B4 in §8.1's passing mention of late-ballot rejection; B3 and B7 are entirely absent. A scenario section that omits nearly half of the B-series invariants it is supposed to witness leaves significant gaps in downstream test and spec coverage.
- **Cite**: > (slice identifier) "scenarios — Concrete scenarios §8.1..§8.6 (witnesses for B1..B6, B10)"
- **Fix recommendation**: Add explicit scenario witnesses: (a) a "Result immutability" scenario showing a post-Resolution mutation attempt rejected (B1 + B7); (b) a "Ballot store frozen after end_at" scenario showing that ballots do not change even if a late submit_ballot is attempted (B2); (c) a "Premature tally rejected" scenario showing publish_result before end_at rejected (B3).

### 4. §8.6 does not actually witness B6 — no ballot write occurs [serious]
- **Category**: under-specification
- **Affected**: §8.6, title and Trust claim consumed line
- **What's wrong**: §8.6 is titled "Impersonation attempt rejected (witness for B6)" and claims to consume B6. B6 states: `∀ k, next.ballots[k] ≠ ballots[k] → ∃ tx ∈ fires_at_transition(σ → σ′), tx.kind = SubmitBallot ∧ tx.msg.sender = k ∧ next.ballots[k] = tx.encrypted_preferences`. The scenario shows a non-candidate (`Eve`) attempting to submit a ballot and being rejected. No ballot is written; `next.ballots = ballots`. Therefore the antecedent of B6 is false, and the implication is vacuously satisfied — the scenario does not exercise the actual load-bearing content of B6 (attribution of a ballot write to its submitting transaction). The positive case (a candidate successfully submits, and the written ballot is attributable to their transaction) is absent. The "stolen-key variant" paragraph even admits the contract cannot verify the voter's intent, further undermining the claim that this scenario witnesses B6's writer-voter binding.
- **Cite**: > "Given a active election in Voting state, when a non-candidate chain address attempts to submit a ballot, the contract rejects with NotACandidate; the on-chain msg.sender check makes voter-identity forgery impossible at the contract layer." ... "Transaction reverts atomically with ContractError::NotACandidate. ballots is unchanged; no event is emitted." ... "Trust claim consumed: B6 (ballot writer is the ballot voter)"
- **Fix recommendation**: Retitle §8.6 as "Non-candidate ballot rejection (Block 3 precondition witness)" and add a new §8.X scenario showing a candidate successfully submitting a ballot and the ballot being attributable to that transaction (B6 positive witness). Alternatively, reframe the existing §8.6 as witnessing Block 3's Requires clause, not B6.

### 5. §8.4 incomplete B4 witness — only lower time bound is shown [serious]
- **Category**: under-specification
- **Affected**: §8.4, title and step-by-step
- **What's wrong**: §8.4 is titled "Premature voting attempt rejected (witness for B4)". B4 is `always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)` — a two-sided time bound. The scenario only demonstrates the lower bound: a candidate attempts to vote before `start_at` and is rejected. It does not demonstrate the upper bound: a candidate attempting to vote at or after `end_at` and being rejected. While §8.1 mentions late ballots are rejected, that mention is not framed as a B4 witness. A scenario claiming to witness a two-sided temporal bound should cover both sides, or explicitly disclaim the partial coverage.
- **Cite**: > "Given a freshly instantiated contract in Created state, when a candidate attempts to submit a ballot before start_at, the contract rejects the transaction with NotInVotingWindow; no state mutates." ... B4: "always (next.ballots ≠ ballots → start_at ≤ env.block.time < end_at)"
- **Fix recommendation**: Expand §8.4 to include a step showing a late-ballot rejection after `end_at`, or split into two sub-scenarios (pre-start_at and post-end_at) each explicitly tied to the corresponding conjunct of B4.

### 6. B10 witness is outside the slice in §8.7, contradicting slice characterization [serious]
- **Category**: coverage gap
- **Affected**: §8.1–§8.6 scope vs §8.7
- **What's wrong**: The slice identifier characterizes §8.1–§8.6 as "witnesses for B1..B6, B10." However, B10's witness scenario is explicitly placed in §8.7 ("B10 witness — cross-layer discharge"), which is outside the slice. B10 is not mentioned in §8.1–§8.6 at all. This means the slice fails to witness B10 despite the characterization claiming it does. The separation is intentional per the text ("B10 is the only temporal invariant without a Section-8 chain-side witness scenario, by construction"), but the slice identifier ignores this architectural fact.
- **Cite**: > (slice identifier) "scenarios — Concrete scenarios §8.1..§8.6 (witnesses for B1..B6, B10)" ... §8.7: "B10 witness — cross-layer discharge (Lean + image-identity binding + B8)"
- **Fix recommendation**: Update the slice identifier to accurately reflect that B10 is witnessed in §8.7, not §8.1–§8.6. If §8.1–§8.6 must witness B10, add a cross-reference in §8.1 pointing to §8.7 as the B10 witness location.

### 7. §8.3 witnesses no B-series invariant [serious]
- **Category**: coverage gap
- **Affected**: §8.3 overall
- **What's wrong**: §8.3 ("Deadlock via missing enclave publish") is inside a slice characterized as "witnesses for B1..B6, B10," yet it consumes no B-series invariant. Its trust claim is "the explicit failure mode 4.6 + Section 5 non-goal 'no admin / creator recovery role'." While deadlock-as-spec'd is a valid design decision, including a scenario that witnesses zero B-series invariants inside a slice whose purpose is B-series witness coverage dilutes the traceability matrix. Downstream test derivation from this scenario will not produce any B-series invariant test cases.
- **Cite**: > "Trust claim consumed: the explicit failure mode 4.6 + Section 5 non-goal 'no admin / creator recovery role'. The deadlock is part of the spec, not a bug."
- **Fix recommendation**: Either (a) move §8.3 outside the B-series witness slice and characterize it as a "failure mode scenario" rather than an invariant witness, or (b) add an explicit B-series linkage — e.g., note that §8.3 is the negative witness for the absence of a liveness invariant (Section 5: "No liveness invariant binds publish_result"), which is a meta-witness for the B-series design boundary.

## Slice-local summary
- Critical: 1
- Serious: 6
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE
---

## Slice: off-chain-witness

*Elapsed: 288.5s, 10,678 chars.*

# kimi-k2-6 — slice off-chain-witness — §8.7 — B10 cross-layer discharge (Lean + image-identity binding + B8) — restructured 6-step decomposition

## Cross-section reads
- §2.5 Block E1 (lines 284–310) — to verify whether input acquisition is covered by the 6-step chain
- §3.2 B8/B9/B10/B10_lean table (lines 346–349) — to ground the exact B10 statement and EnclaveImage usage
- §2.5 Type signatures / CONTEXT_APPENDIX — to verify EnclaveImage arity

## Attacks on this slice

### 1. EnclaveImage symbol has contradictory typing: appendix says `Bytes → Bytes`, §8.7 uses it as 3-ary [critical]
- **Category**: contradiction
- **Affected**: §8.7 step 3, §3.2 B10_lean, CONTEXT_APPENDIX type signatures
- **What's wrong**: The CONTEXT_APPENDIX lists `EnclaveImage : Bytes → Bytes` (annotated `image_extract`). §3.2 B10_lean states `∀ raw_ballots, candidates, privkey. EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(...)`. §8.7 step 3 repeats the 3-ary application. Nowhere in §8.7, §2.5, or §3.2 is the symbol redefined from unary `Bytes → Bytes` to ternary `(RawBallots, CandidateSet, PrivKey) → TallyResult`. The two arities are incompatible. A reader or downstream spec author cannot tell whether `EnclaveImage` is an extraction function (unary), a model semantics (ternary), or two different symbols with the same name. The decomposition builds on a symbol whose type is unsettled.
- **Cite**: > `EnclaveImage : Bytes → Bytes` (CONTEXT_APPENDIX type signatures); > `EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)` (§3.2 B10_lean, line 349); > `the enclave image's externally observable input-output relation satisfies EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)` (§8.7 step 3, line 694)
- **Fix recommendation**: Either rename the unary extraction function (e.g., `extract_enclave_model : Bytes → EnclaveModel`) and introduce `EnclaveModel.semantics : RawBallots → CandidateSet → PrivKey → TallyResult`, or explicitly overload `EnclaveImage` with its two signatures and a coercion rule. The intent doc must resolve the arity clash before downstream Lean or Quint can use the symbol.

### 2. Step 6 smuggles dstack-KMS correctness as a fourth conjunct outside the named decomposition [serious]
- **Category**: composition failure
- **Affected**: §8.7 step 6, §3.2 B10, §8.7 closing paragraph (line 701)
- **What's wrong**: The slice advertises the composition pattern `B10 ← B10_lean ∧ image-identity-binding ∧ B8` (line 701). But the actual B10 statement in §3.2 contains `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(...))`. Step 6 says the existential is "instantiated by the dstack-KMS-derived privkey (Section 6.3)". None of the three named conjuncts — B10_lean, image-identity-binding, or B8 — mention `dstack_kms_derived` or key provenance. B10_lean is universally quantified over `privkey` and silent on KMS. Image-identity-binding is about binary identity. B8 is about attestation content and image identity. The 6-step chain therefore lacks a load-bearing link: it does not establish that the `privkey` used by the enclave is the one dstack-KMS derived for this contract. The decomposition pattern is materially incomplete; honest restatement requires `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack-kms-correctness`.
- **Cite**: > `Composition pattern: B10 ← B10_lean ∧ image-identity-binding ∧ B8` (§8.7 line 701); > `∃ privkey: PrivKey, dstack_kms_derived(privkey, contract_addr) ∧ next.tally_result = Some(Tally_spec(ballots@end_at, candidates, privkey))` (§3.2 B10, line 348); > `with the existential quantifier in B10 instantiated by the dstack-KMS-derived privkey (Section 6.3)` (§8.7 step 6, line 697)
- **Fix recommendation**: Add a fourth explicit conjunct to the decomposition pattern and a corresponding step (e.g., step 4.5 or step 7) that discharges `dstack_kms_derived(privkey, contract_addr)`. If this is intended to be an axiom rather than a proved lemma, label it as such in the pattern: `B10 ← B10_lean ∧ image-identity-binding ∧ B8 ∧ dstack-kms-axiom`.

### 3. Step 1's lemma statement is stronger than its own caveat admits [serious]
- **Category**: preconditional over-strength
- **Affected**: §8.7 step 1, §2.5 Stage 1 spec
- **What's wrong**: Step 1 states: "the implementation of Stage 1 returns the partition `(valid_ballots, dropped_voters, non_voters)` matching the spec in Section 2.5." But the caveat immediately below admits that `Ecies.roundtrip` "proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts" and that "the soundness claim 'a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate' requires an additional authenticated-decryption (AEAD) property." Section 2.5's Stage-1 spec defines the decoder as "ECIES-decrypt ... parse ... validate as a permutation" — it does not authenticate the sender. Without AEAD, a malicious voter can craft a ciphertext that decrypts to a valid permutation under the enclave's key but was not their honest ballot. Step 1's lemma, as stated, implies the partition faithfully represents the actual voter intent, which is false without the AEAD precondition. The lemma is therefore unsound as stated; it needs a precondition that ciphertexts were honestly generated.
- **Cite**: > `the implementation of Stage 1 returns the partition (valid_ballots, dropped_voters, non_voters) matching the spec in Section 2.5` (§8.7 step 1, line 692); > `Ecies.roundtrip proves honestly-encrypted plaintexts roundtrip; it does not bound adversarial / malformed ciphertexts` (§8.7 step 1 caveat, line 692); > `the soundness claim "a ciphertext that decrypts to a valid-looking permutation was actually encrypted by the named candidate" requires an additional authenticated-decryption (AEAD) property` (§8.7 step 1 caveat, line 692)
- **Fix recommendation**: Restate the Stage-1 lemma with an explicit honesty precondition: "For all `(raw_ballots, candidates, privkey)` where every ciphertext in `raw_ballots` was honestly encrypted by its associated address, the implementation returns the partition matching Section 2.5." Without this precondition, step 3's composition and step 6's B10 conclusion are unsound — B10 requires the tally to reflect the actual on-chain ballots, not adversarially injected forgeries.

### 4. "Lean-extracted model" in step 3 is undefined — step 4's binding claim is ungrounded [serious]
- **Category**: under-specification
- **Affected**: §8.7 steps 3–4
- **What's wrong**: Step 3 says B10_lean is "a Lean theorem about the Lean-extracted model of the enclave binary". Step 4 says "the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that B10_lean is over." But §8.7 never defines what "Lean-extracted model" means, what extraction function produces it, what language the model is expressed in, or what equivalence relation connects the model to the binary. The claim that the binding "cannot be a Lean theorem because the Lean spec models the binary; the binding claim is about the modelling discipline" is metatheoretical prose, not a definition. Without a criterion for "matches", step 4 is unverifiable. A build pipeline could produce any binary, register its MRTD, and assert it "matches" the undefined model.
- **Cite**: > `a Lean theorem about the Lean-extracted model of the enclave binary` (§8.7 step 3, line 694); > `the binary whose MRTD/RTMR is registered on-chain matches the Lean-extracted model that B10_lean is over` (§8.7 step 4, line 695); > `extraction-soundness + build-reproducibility obligation` (§8.7 step 4, line 695)
- **Fix recommendation**: Define the extraction function (e.g., `extract : Binary → LeanTerm`), the target formalism (e.g., a shallowly-embedded state monad or a deeply-embedded ISA semantics), and the matching relation (e.g., binary MRTD equals reproducible-build hash of the source whose extraction yields the model). Without these definitions, step 4 is a placeholder, not an obligation.

### 5. The 6-step chain has an input-fidelity gap: B10_lean is pure-functional, B10 requires specific inputs [serious]
- **Category**: composition failure
- **Affected**: §8.7 step 6, §3.2 B10, §2.5 Block E1
- **What's wrong**: B10_lean (step 3) is a universal pure-functional equality: `∀ raw_ballots, candidates, privkey. EnclaveImage(...) = Tally_spec(...)`. B8 (step 5) witnesses that the registered image produced an attestation over a specific `tally_body`. Step 6 concludes that `tally_result = Tally_spec(ballots@end_at, candidates, privkey)`. But nowhere in steps 1–5 is it established that the enclave's *actual input* was `ballots@end_at` (the frozen on-chain ballot set) and the contract's `candidates`, as opposed to some other ballot set or candidate list fed to it by a compromised host. Block E1 (§2.5) specifies that the enclave reads from chain, but Block E1 is not one of the proof steps in §8.7's decomposition. B10_lean holds for *any* inputs; B8 binds the *output* to an image identity; neither binds the execution to the specific on-chain inputs. A host could feed the enclave empty ballots, the enclave (running the correct image) would correctly compute `Tally_spec(∅, candidates, privkey)`, and B8 would attest to that output — yet B10 would be violated because the result does not equal `Tally_spec(ballots@end_at, candidates, privkey)`.
- **Cite**: > `EnclaveImage(raw_ballots, candidates, privkey) = Tally_spec(raw_ballots, candidates, privkey)` (§8.7 step 3, line 694); > `the attestation chain witnesses that the firing transition's tally was produced by an enclave image whose attested identity equals the registered identity` (§8.7 step 5, line 696); > `the published tally_result equals Tally_spec(ballots@end_at, candidates, privkey)` (§8.7 step 6, line 697); > `ballots: Map<Addr, Vec<u8>> — read from contract storage at the first chain state where env.block.time ≥ end_at` (§2.5 Block E1, line 291)
- **Fix recommendation**: Add an explicit input-fidelity step to the decomposition (e.g., between steps 5 and 6) that witnesses the enclave read `ballots@end_at` and `candidates` from the chain. This could take the form of a TDX-reported input hash, a chain-inclusion proof in the attestation, or an additional temporal invariant that the enclave's observed inputs equal the on-chain state. Without this, the gap between "the image is correct" and "this execution used the correct inputs" remains unbridged.

## Slice-local summary
- Critical: 1
- Serious: 4
- Cosmetic: 0

## VERDICT (slice-local): BREAKS-AT-SLICE
