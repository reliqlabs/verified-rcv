# gpt-oss-120b — slice behaviors-types

## Cross-section reads
- None (all needed material appears within the slice).

## Attacks on this slice
### 1. Missing secret‑key input — temporal‑state mismatch [critical]
- **Category**: temporal‑state mismatch
- **Affected**: `Tally_spec` type signature (lines 153‑155)
- **What's wrong**: `Tally_spec` is declared as  
  > `Tally_spec(raw_ballots, candidates, enclave_privkey) → TallyResult` (lines 153‑155)  
  yet the contract state only stores `enclave_pubkey` (line 116) and never exposes the private key. The spec never explains how the off‑chain enclave obtains `enclave_privkey`, nor does it model the secret‑key as a state variable or a trusted input. Consequently the functional contract‑level specification is unsound: it assumes a secret that the on‑chain model cannot provide, breaking the correspondence between the on‑chain state transition and the off‑chain computation required by B10.
- **Cite**: > `Tally_spec(raw_ballots, candidates, enclave_privkey) → TallyResult` (lines 153‑155)  
  > `enclave_pubkey: PubKey` — generated at instantiation (line 116)
- **Fix recommendation**: Introduce an explicit trusted input abstraction for the enclave’s private key (e.g., a *trusted enclave interface* that supplies `enclave_privkey` after attestation) and add a corresponding state‑or‑environment clause to the spec, or redesign `Tally_spec` to take only `enclave_pubkey` and treat decryption as an abstract oracle whose correctness is captured by a separate trust assumption.

### 2. Unspecified iteration order of `Map` entries — under‑specification [serious]
- **Category**: under‑specification
- **Affected**: deterministic ordering claim for `Map<Addr, Nat>` (lines 134‑135, 136‑139)
- **What's wrong**: The spec asserts that “entries within each map … are **deterministic‑by‑candidate‑declaration‑order**” (lines 134‑135) and that this ordering is used for Borsh serialization (lines 136‑139). However, `Map` in CosmWasm/Rust has no defined iteration order; without an explicit ordering discipline (e.g., sorted by address), two compliant implementations could produce different byte streams, breaking B8(c) and any downstream Lean/Quint proofs that rely on a unique canonical representation.
- **Cite**: > `Ordering of entries within each map and within Vec<Addr> fields is **deterministic‑by‑candidate‑declaration‑order**` (lines 134‑135)  
  > `canonical_serialization(contract_addr ‖ tally_body)` … `Map<Addr, Nat>` entries are emitted in **candidate‑declaration order** (lines 136‑139)
- **Fix recommendation**: Define `Map` as an ordered associative array (e.g., a vector of `(Addr, Nat)` sorted by the index of `Addr` in `candidates`) or replace `Map` with a fixed‑size array indexed by candidate position. Explicitly state the ordering algorithm in the spec.

### 3. Ambiguous handling of non‑candidate ballot keys — under‑specification [serious]
- **Category**: under‑specification
- **Affected**: Stage 1 description of `decrypt_and_validate` (lines 155‑159) and the `ballots` storage type (line 117)
- **What's wrong**: `ballots` is a `Map<Addr, Vec<u8>>` (line 117) with no restriction that keys must be candidates. Stage 1 processes every `(addr, ciphertext)` in `raw_ballots` (lines 155‑158) but only validates the *decrypted* preference list against `candidates`. A malicious sender could insert a ballot under a non‑candidate address; the decryption would succeed, the permutation check would fail, and the address would be added to `dropped_voters`. The spec never forbids such entries, allowing an attacker to inflate `ballots_dropped` arbitrarily or to cause denial‑of‑service by flooding with invalid keys. Moreover, the invariant `non_voters = candidates \ ballots.keys` (line 266) assumes `ballots.keys ⊆ candidates`, which is not guaranteed.
- **Cite**: > `ballots: Map<Addr, Vec<u8>>` — candidate → encrypted_preferences; absent = no vote (line 117)  
  > Stage 1 … `For each (addr, ciphertext) ∈ raw_ballots` … `validate as a permutation of candidates` (lines 155‑158)  
  > `non_voters = candidates \ ballots.keys` (line 266)
- **Fix recommendation**: Add a precondition to `submit_ballot` (Block 3) that rejects any `msg.sender` not in `candidates` (already present) **and** enforce that `ballots` never contains keys outside `candidates` (e.g., by pruning or rejecting such entries in Stage 1). Update the well‑formedness clause to require `ballots.keys ⊆ candidates`.

### 4. Incomplete definition of transaction trace ordering — ambiguity [serious]
- **Category**: ambiguity
- **Affected**: Transaction trace model (lines 149‑150)
- **What's wrong**: The model states that a transition is parameterized by the “multiset of transactions `fires_at_transition(σ → σ′)` … ordered by CosmWasm's deterministic execution order.” A multiset is unordered, yet the phrase “ordered by … execution order” is contradictory and leaves unspecified whether the order matters for invariants (e.g., B6). Without a precise ordering rule (e.g., lexicographic by sender address, then by message type), proofs that rely on per‑key attribution could be unsound.
- **Cite**: > `fires_at_transition(σ → σ′)` … `ordered by CosmWasm's deterministic execution order` (lines 149‑150)
- **Fix recommendation**: Replace “multiset” with an ordered list and explicitly define the ordering algorithm (e.g., the order in which the Cosmos SDK processes messages in a block). Alternatively, state that invariants only depend on the set of transactions, not their order, and adjust B6 accordingly.

### 5. Batch‑elimination rule can violate per‑round count conservation — coverage gap [serious]
- **Category**: coverage gap
- **Affected**: Batch‑elimination safety note (lines 183‑184) and per‑round count conservation invariant (line 265)
- **What's wrong**: The rule “batch‑eliminate all candidates tied at the lowest count, even if this would produce an ‘early co‑winner’ outcome” (lines 183‑184) can lead to a round where the sum of eliminated candidates equals the total remaining candidates, causing the algorithm to terminate with multiple co‑winners *before* any round’s `per_round_counts` is recorded for the eliminated set. The spec’s well‑formedness clause requires “each `per_round_counts[i]` has all entries summing to `ballots_tallied`” (line 265) but does not specify what `per_round_counts` should contain for the final co‑winner round when elimination empties the candidate set. This leaves an implementation freedom that could break the invariant.
- **Cite**: > `batch‑eliminate all candidates tied at the lowest count … early co‑winner` (lines 183‑184)  
  > `per‑round counts internally consistent … each per_round_counts[i] has all entries summing to ballots_tallied` (line 265)
- **Fix recommendation**: Define explicitly that in the “early co‑winner” case the final `per_round_counts` entry must list *all* remaining candidates with their current firstplace tallies (which will all be equal) before declaring co‑winners, ensuring the conservation invariant holds.

### 6. Over‑specification of canonical serialization discipline — over‑specification [cosmetic]
- **Category**: over‑specification
- **Affected**: Canonical serialization paragraph (lines 136‑139)
- **What's wrong**: The spec mandates Borsh serialization with deterministic ordering of maps and vectors (lines 136‑139). While this is useful for reproducibility, the on‑chain contract does not expose the raw serialized bytes; only the hash is used in attestation. Requiring a specific ordering of map entries adds unnecessary constraints on implementations that could otherwise use any order‑preserving serialization as long as the hash matches. This over‑constrains the design without providing additional security.
- **Cite**: > `canonical_serialization … Borsh … entries are emitted in candidate‑declaration order` (lines 136‑139)
- **Fix recommendation**: Relax the requirement to “any deterministic serialization that yields the same hash for equivalent logical values,” or move the ordering requirement to the off‑chain enclave implementation rather than the on‑chain spec.

## Slice‑local summary
- Critical: 1
- Serious: 5
- Cosmetic: 1

## VERDICT (slice-local): BREAKS-AT-SLICE
The slice contains multiple critical and serious flaws that undermine the functional correspondence between on‑chain state and the off‑chain tally computation, leave undefined or ambiguous behaviours, and impose unnecessary constraints. These issues must be resolved before the specification can be considered sound.