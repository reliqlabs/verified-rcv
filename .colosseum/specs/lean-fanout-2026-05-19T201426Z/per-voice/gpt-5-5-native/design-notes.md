# gpt-5-5-native Lean design notes

This voice stays stdlib-only. I used reducible `abbrev`s for `Addr = String`, `Bytes = List UInt8`, `PrivKey = Bytes`, `CandidateSet = List Addr`, and `RawBallots = List RawBallot`. I represented maps as deterministic ordered lists rather than importing Mathlib maps or finsets. This matches the intent's candidate-declaration-order serialization rule and keeps the file lightweight.

`RawBallot` stores `(voter, ciphertext)`. `Ballot` stores the decoded preference list. `RoundCounts` is `List (Addr × Nat)`, interpreted as an ordered map over surviving candidates. `DecryptedSet` includes `valid`, `dropped_voters`, and `non_voters`, so Stage 1 owns all voter-accounting fields. `IRVResult` contains only the Stage 2 recursion output: winners, round counts, eliminations, and `ballots_tallied`.

`decrypt_and_validate` and `IRV_spec` are `opaque`. `Tally_spec` is a transparent `def`: it calls Stage 1, feeds `valid` into Stage 2, then constructs `TallyResult` by combining IRV fields with dropped/non-voter bookkeeping. `ballots_dropped` is defined as `dropped_voters.length`.

The theorem statements encode the required obligations with `sorry` bodies:

- `s6_winner_subset`: winners are nonempty, bounded by candidate count, and each winner is in `cs`.
- `s7_voter_partition`: `ballots_tallied + ballots_dropped + len(non_voters) = len(candidates)`.
- `s8_round_counts_sum`: each round's count total equals `ballots_tallied`.
- `s9_no_reappearance`: if `c` is in `eliminated_by_round[i]`, then no later round `j > i` mentions `c` in its count map.
- `B10_lean`: universal equality between the reserved extracted-model symbol and `Tally_spec`.

`EnclaveImage` is declared as an `axiom` with type `RawBallots → CandidateSet → PrivKey → TallyResult`. It is not definitionally tied to `Tally_spec`, preserving the anti-tautology discipline in §2.5 and §3.2.

Omissions: this file does not implement IRV recursion, permutation validation, Borsh parsing, ECIES behavior, or input-fidelity/image-binding. Those are represented by opaque functions or cross-layer obligations outside this Lean math-layer statement file.
