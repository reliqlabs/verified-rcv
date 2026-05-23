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