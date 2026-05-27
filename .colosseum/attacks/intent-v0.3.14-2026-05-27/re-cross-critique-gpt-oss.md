# Re-cross-critique (Ask R) — gpt-oss-120b voice — 2026-05-27

Prompted with strengthened v0.3.14 (after F1+F2+F3+F4 incorporation). Asked to confirm convergence OR surface revision-induced regressions.

## Findings raised by voice

### 1. DST zero-padding in ReportData[32..64] (Low, per voice)
Voice argues zero-padding the DST creates an "absorption gap" — a future implementation might treat the 64-byte field as variable-length. **Synthesis adjudication: FALSE POSITIVE.** ReportData is a fixed 64-byte field per the TDX hardware specification + gnark public_inputs byte layout (§2.5). The chain-side verifier does bytewise-equality check on fixed offsets [0..32] and [32..64]; no parser does variable-length interpretation. The voice imagined a future code path that the methodology's byte-layout discipline already structurally precludes.

### 2. §6.7 migration discipline clashes with "§4.2 immutability" (Medium, per voice)
Voice claims §4.2 states election cryptographic parameters are "frozen". **Synthesis adjudication: FALSE POSITIVE (hallucinated clause).** The actual §4.2 in verified-rcv intent is "dstack KMS key compromise — confidentiality" (a failure-mode entry, not an immutability discipline). No such "§4.2 immutability clause" exists. The voice fabricated the citation. Immutability in verified-rcv is via B1 (tally), B7 (Resolved sink), and B11 (candidate_names) — §6.7 ENCODES these as migration constraints, doesn't contradict them.

### 3. DST length-prefix omission (Low, per voice)
Voice argues that prepending DSTs without internal length prefix creates a collision risk if future DST has same length as current+other-data. **Synthesis adjudication: INFORMATIONAL (hygiene, not attack).** Current spec defines 3 DSTs: `verified-rcv:ballots:v1` (23 bytes), `verified-rcv:names:v1` (22 bytes), `DST_VERIFIED_RCV_PUBKEY_V1` (different namespace + length). All textually distinct. A 4th colliding DST would require deliberate construction; the version suffix `:v1` plus prefix-disjointness rules it out for the current state. Future addition of new DSTs should follow the same convention; the voice's recommendation (length prefix) is preventive hygiene worth recording but not a live attack.

## Regressions from F1+F2+F3+F4 incorporation

**None.** The voice explicitly noted "no outright functional regressions". The 3 findings above are not regressions — they're either false positives or future-hygiene observations against an already-strengthened spec.

## Convergence verdict

**CONFIRMED** per synthesis adjudication.

- Finding 1: precluded by TDX hardware spec.
- Finding 2: hallucinated clause that doesn't exist in the intent.
- Finding 3: hygiene observation, not a live attack.

The strengthened v0.3.14 successfully absorbs the original 4 findings (F1+F2+F3+F4) without introducing regressions or new attack surface.

## Methodology note (v0.4 ask R validation)

Ask R says "re-cross-critique after canonical revision when revision touches load-bearing predicates. Catches revision-induced regressions (~5 min wall-clock per voice)."

This re-cross-critique pass found **zero true regressions** and surfaced **two false positives** (hallucinated clause + structurally-precluded scenario) plus **one informational hygiene point** (DST length prefix as future-proofing). The methodology empirically demonstrates that:

1. Ask R is fast (one voice, ~3 min wall-clock).
2. The false-positive rate is non-trivial — the voice did fabricate a clause. The synthesis adjudication is load-bearing.
3. Even false positives have signal: Finding 3's hygiene recommendation (length-prefix DSTs) is worth recording as a v0.4-or-later methodology ask. Add to candidates as informational.

The convergence-confirmed result locks v0.3.14 for commit.
