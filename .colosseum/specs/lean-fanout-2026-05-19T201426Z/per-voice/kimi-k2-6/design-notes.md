# design-notes.md — kimi-k2-6 voice

## Stdlib vs Mathlib
Stdlib only. The spec needs no finset arithmetic, no ordering relations, and no
finite-type machinery; pulling in Mathlib would add 30+ seconds to every
typecheck round for no gain.

## Type choices

- `Addr := String`, `Bytes := List UInt8` — same as the canonical; these are
type synonyms and carry no invariant.
- `Ballot` is a structure wrapping `ranking : List Addr`. `Inhabited` is derived
so that `opaque` return types are satisfied.
- `RawBallots := List (Addr × Bytes)` — an association list rather than the
canonical's `List RawEntry`. This makes the "voter address" component directly
accessible as the first projection of the pair, which matches the intent's
`Map<Addr, Vec<u8>>` reading more closely.
- `RoundCounts := List (Addr × Nat)` — association list for per-round counts.
The sum theorem (S8) folds over `p.snd`, which is cleaner than a record
projection.
- `IrvOutput` — a separate structure for the Stage-2 result that omits the
Stage-1 bookkeeping fields (`ballots_dropped`, `dropped_voters`, `non_voters`).
This makes the separation of concerns explicit: `IRV_spec` is the combinatorial
core, `Tally_spec` is the assembly.
- `DecryptedSet` includes `non_voters : List Addr`, which the canonical omits.
The intent §2.5 says Stage 1 produces `non_voters`, and the composition section
says `Tally_spec` threads all three Stage-1 outputs into the final result.
Including `non_voters` in `DecryptedSet` makes this threading transparent.

## Theorem encoding

- **S6** (`s6_winner_subset`): winners are a non-empty subset of candidates,
bounded above by `cs.length`. Uses `∀ w ∈ t.winners, w ∈ cs` for the subset
claim.
- **S7** (`s7_voter_partition`): conservation equation
`ballots_tallied + ballots_dropped + non_voters.length = cs.length`. The intent
states this as a pointwise invariant on `tally_result.is_some()`; at the math
layer we state it directly as a theorem about `Tally_spec`.
- **S8** (`s8_round_counts_sum`): each round's counts sum to `ballots_tallied`.
Uses `foldl` over the association list.
- **S9** (`s9_no_reappearance`): eliminated candidates do not reappear in later
rounds. Uses list indexing with `!` (same idiom as the canonical). The
quantifier structure is `∀ i j, i < j → ... ∀ c ∈ eliminated[i]!, ∀ p ∈ counts[j]!, p.fst ≠ c`.
- **B10_lean** (`B10_lean`): `EnclaveImage raw cs pk = Tally_spec raw cs pk`.
`EnclaveImage` is an `axiom`, not a `def`, so `rfl` discharge is impossible.

## Omissions

- No `step` relation or temporal logic — this is the math layer, not the
protocol model (that's Quint).
- No explicit `non_voters = candidates \ raw_ballots.keys` lemma — S7 captures
the conservation property the intent cares about.
- No tie-break determinism theorem — the intent's batch-eliminate-all-tied
policy makes ties multi-winner, so deterministic tie-breaking is not required.
- No `Inhabited` derivation on `Ballot` would break `opaque`; `deriving Inhabited`
is included.
