# Crypto-voice adversarial review — intent v0.3.14

**Reviewer prior**: hostile spec reviewer, cryptographic-attacks lens.
**Scope**: v0.3.14 diff only — human-readable `candidate_names` + `names_hash`
addition. Reviewer has NOT seen v0.3.14 implementation or self-review.

## Summary

Five findings: **0 Critical, 0 Major, 2 Minor, 3 Informational.** The
v0.3.14 diff is, on the cryptographic surface, well-defended. Borsh's
length-prefix discipline gives unambiguous preimage decoding for
`names_hash`. Embedding `names_hash` inside `canonical_serialization`
inherits the publish-quote DST tag. B11's structural immutability
foreclosure plus the publish-time `names_hash` mismatch path closes
mid-flight tamper attacks. The confusable-name and IRL-identity
mislabelling vectors are pre-acknowledged in §6.4 as scope-acceptable.

The two Minor findings (M1, M2) identify defense-in-depth opportunities
that should be addressed before commitment. The three Informational
findings (I1, I2, I3) are best-practice notes that the methodology may
choose to address now or defer.

---

## M1 — `names_hash` not bound to registration quote (defense gap)

**Attack name**: registration-publish names binding gap.

**Trace**:
1. Admin calls CreateElection with `candidates = [A, B]`,
   `candidate_names = ["Alice", "Bob"]`. Registration quote at this
   moment binds `SHA-256(enclave_pubkey ‖ Borsh(contract_addr) ‖
   u64_LE(election_id))` per v0.3.12 N22. Names are NOT in the
   registration ReportData.
2. Election runs; ballots submitted.
3. At PublishResult time, the chain computes `compute_names_hash` from
   `election.candidate_names` (immutable per B11) and includes it in
   `commit_hash`. Runtime computes its own `names_hash` from whatever
   names it received; the publish quote attests it. Chain compares
   byte-for-byte; mismatch → `AttestationCommitMismatch`.

**Why this is not Critical**: B11 + the publish-time names_hash check
together catch any tamper. There is no exploitable window because names
are written exactly once at CreateElection and the publish-quote
commitment fully covers them.

**Why this is Minor not Informational**: the registration quote is
v0.3.12's tightening of the trust surface. Leaving names out of it
preserves an asymmetry: pubkey provenance is bound at registration, but
names provenance is bound only at publish. An enclave that comes up,
attests its pubkey, then later discovers it was operating on
host-substituted names has no early-failure signal — the mismatch fires
only after the full tally completes. Including `names_hash` in
registration ReportData would surface the gap at CreateElection time,
narrowing the operational fault-detection window.

**Severity**: Minor.

**Suggested fix**: extend the registration-quote ReportData preimage to
`SHA-256(enclave_pubkey ‖ Borsh(contract_addr) ‖ u64_LE(election_id)
‖ names_hash)`. This is a strict refinement of the v0.3.12 N22
three-axis binding (becomes four-axis). The enclave already has access
to `candidate_names` at CreateElection (must compute names_hash for the
publish quote anyway); threading it earlier costs nothing.

**Alternative**: declare explicitly in §3.2 B8(e) that names binding is
deliberately deferred to publish-time, with rationale (early-failure
signal not load-bearing because B11 forecloses mid-flight tamper anyway).
This would close the methodology gap without code changes.

---

## M2 — Internal hash domain separation absent on `names_hash` and `ballots_hash`

**Attack name**: cross-purpose internal-hash preimage collision (theoretical).

**Trace**:
1. `names_hash` preimage layout: `u32_LE(N) ‖ for i: u32_LE(len_i) ‖
   utf8_i`.
2. `ballots_hash` preimage layout: `u32_LE(N) ‖ for each: u32_LE(voter_len)
   ‖ voter_utf8 ‖ u32_LE(cipher_len) ‖ cipher_bytes`.
3. For N=1, a `names_hash` preimage with `name = voter_addr_str ‖
   u32_LE(cipher_len) ‖ cipher_bytes` (single contiguous byte sequence)
   has the byte layout `01 00 00 00 ‖ u32(name_len) ‖ voter_addr_str ‖
   u32(cipher_len) ‖ cipher_bytes`.
4. If `name_len == voter_addr_len`, the two preimages are byte-identical:
   the same SHA-256 input produces the same SHA-256 output.

**Why this is not Major**: the "name" required to mount the collision
would contain raw ciphertext bytes, which are vanishingly unlikely to be
valid UTF-8 (Block 1's `CandidateNameInvalidUtf8` check rejects it).
Even setting that aside, both hashes feed into the SAME outer
`canonical_serialization` blob and are SHA-256ed together into
`commit_hash`. The outer hash defeats any cross-purpose substitution
because the chain reconstructs both hashes from its own canonical
inputs and compares full bytes.

**Why this is Minor**: it is a violation of the cryptographic-engineering
discipline of "every distinct SHA-256 use site gets a distinct DST tag."
The publish-quote commit hash uses `DST_VERIFIED_RCV_TALLY_V1`; the
registration commit hash uses `DST_VERIFIED_RCV_ENCLAVE_PUBKEY_V1`.
`names_hash` and `ballots_hash` are SHA-256 invocations with no DST tag.
If a future revision exposes either hash independently (e.g., a query
that returns `names_hash` for off-chain verification), the lack of DST
becomes exploitable. Defense-in-depth via DST is essentially free.

**Severity**: Minor.

**Suggested fix**: prefix each internal-hash preimage with a 32-byte
DST tag. E.g., `names_hash = SHA-256("DST_VERIFIED_RCV_NAMES_V1" ‖
zero-pad-to-32 ‖ u32_LE(N) ‖ ...)`; analogously `ballots_hash` gets
`DST_VERIFIED_RCV_BALLOTS_V1`. This costs 32 bytes per hash invocation
and is implementable as a single-line preimage prefix at both chain and
runtime sites. The cross-byte-identity tests at
`crates/enclave/src/attestation.rs:718` would need a single-line update.

---

## I1 — UTF-8 NFC normalization not required

**Attack name**: NFC-vs-NFD byte-distinct visual-confusable (sub-case of
§6.4's confusable scope statement).

**Trace**:
1. Admin registers `candidates = [A, B]`, `candidate_names = ["café",
   "café"]` where the first is NFC (4 bytes: 63 61 66 C3 A9) and the
   second is NFD (5 bytes: 63 61 66 65 CC 81).
2. Both pass UTF-8 validation, length, no-NUL, byte-distinctness.
3. Voters see identical "café" twice; vote-splitting attack.

**Why this is Informational**: §6.4 already documents this category
("Cyrillic Аlice vs Latin Alice... homoglyph attacks with whitespace /
combining marks / RTL overrides"). The NFC/NFD case is a specific
instance of "combining marks" that §6.4's "MUST warn on confusable-
character detection (NFC normalization...)" already covers as a UI-layer
obligation.

**Severity**: Informational — scope-acceptable per intent §6.4.

**Suggested fix**: none required; §6.4 already pins this as UI-layer
responsibility. Optionally, §6.4 could call out NFC explicitly as the
single most important normalization to add to the UI warning list (the
intent currently mentions NFC parenthetically).

---

## I2 — Registration-time pubkey-provenance binding does not extend to names

(Companion observation to M1, framed differently.) The §3.2 B8(e) row's
trust-boundary narrowing is "the enclave_pubkey must come out of a TDX
enclave whose code matches the registered MRTD." With v0.3.14, names
also become part of the trust surface (off-chain candidate-legitimacy
attack surface per §6.4 v0.3.14 paragraph). But names provenance is not
bound to the enclave at registration: only the **pubkey** is. The
enclave does not attest at registration time that it has accepted any
particular candidate-name list.

This is the same observation as M1 from a different angle: the
methodology framing of "trust surface" in v0.3.14 puts names on equal
footing with addresses for off-chain legitimacy, but the cryptographic
binding chain reflects an inequality (pubkey: bound at registration AND
publish; names: bound at publish only).

**Severity**: Informational — methodology clarity, not soundness.

**Suggested fix**: see M1's "Alternative" — make the deferral explicit
in §3.2 B8(e) or §6.4 so the asymmetry is auditable.

---

## I3 — `compute_names_hash` chain-vs-runtime byte-identity cross-test missing from intent

The v0.3.11 entry introduces `compute_ballots_hash` with an explicit
cross-test `compute_ballots_hash_contract_vs_runtime_byte_identical`.
The v0.3.14 entry says `compute_names_hash` is "(contract + enclave,
cross-tested for byte-identity, parallel to compute_ballots_hash from
v0.3.11)" but does not name the cross-test or pin its assertion. This is
an inspectability gap relative to the v0.3.11 baseline.

**Severity**: Informational — discipline note.

**Suggested fix**: name the cross-test explicitly in the v0.3.14 revision-
log entry, paralleling v0.3.11's
`compute_ballots_hash_contract_vs_runtime_byte_identical` wording. E.g.,
"Cross-test `compute_names_hash_contract_vs_runtime_byte_identical` at
`crates/enclave/src/attestation.rs:<line>` asserts byte-equality."

---

## Attack lenses explicitly checked, no finding

1. **Borsh-prefix collision in names_hash preimage**: `u32_LE(N)` outer
   prefix plus per-name `u32_LE(len_i)` inner prefixes make the
   serialization injective. Two distinct `(N, names)` pairs cannot
   produce the same preimage byte string. SHA-256 collision-resistance
   is the only remaining hardness — accounted for in B9's
   `Adv_commitTally_CR` summand. **No collision attack found.**

2. **Empty / boundary preimage cases**: Block 1 enforces `len(candidates)
   ≥ 1` and each name byte-length ≥ 1, so N=0 and empty-name edge cases
   cannot arise in live elections. The `names_hash` function would still
   be well-defined for N=0 (preimage `00 00 00 00`, distinct from any
   N≥1 preimage), but this is moot. **No edge-case attack found.**

3. **Length-extension on names_hash**: `names_hash` is consumed only as
   input to the outer `commit_hash = SHA-256(canonical_serialization(...))`.
   Length-extension does not compose through a second hash. **No
   length-extension attack found.**

4. **Inverse confusable (impersonate-by-confusable, multiple addresses,
   one IRL identity)**: explicitly documented in §6.4 as scope-acceptable;
   the chain provides byte-distinctness, not real-world identity binding.
   The "Cyrillic Аlice vs Latin Alice" example in the prompt is the
   forward case (one IRL identity, multiple visually-identical names);
   the inverse case (one byte-distinct name, multiple IRL identities) is
   not coherently a cryptographic attack — it would require the chain to
   know IRL identity, which §6.4 explicitly disclaims. **Scope-acceptable
   per intent §6.4.**

5. **HISTORICAL_ELECTIONS replay**: archival preserves
   `candidate_names` byte-identically per v0.3.14. Replay across
   elections is foreclosed by election_id binding in commit_hash
   (v0.3.10 N4) and in registration ReportData (v0.3.12 N22). A
   historical election's `names_hash` is not exploitable in a new
   election because the new election has a different election_id, hence
   different commit_hash. **No replay attack found.**

6. **Canonical_serialization boundary parsing ambiguity**: the new
   `names_hash[32]` field is fixed 32 raw bytes (no length prefix),
   parallel to `ballots_hash[32]`. The chain dictates the encoding and
   compares full bytes against its own reconstruction — no
   attacker-controlled boundary ambiguity. **No boundary attack found.**

---

## Verdict

The v0.3.14 diff is cryptographically clean. The two Minor findings (M1,
M2) are defense-in-depth tightenings worth doing pre-commit; neither
gates the diff. The three Informational findings (I1, I2, I3) are
discipline notes that may be addressed in v0.3.14 or deferred to a
v0.3.15 patch entry.

**Recommendation**: incorporate M2's per-hash DST tags before v0.3.14
commits (cheap, structural, removes a future foot-gun). Decide M1 by
explicit choice: extend registration-quote ReportData to four-axis
binding (`pubkey ‖ contract_addr ‖ election_id ‖ names_hash`), OR
document the deferral rationale in B8(e). Either resolution is
audit-acceptable; silence on it is not.
