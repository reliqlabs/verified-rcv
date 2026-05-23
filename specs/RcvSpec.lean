/-
RcvSpec.lean — verified-rcv math spec (Claude-authored canonical baseline)

This is the Lean math-layer spec for the IRV tally pipeline named in
`.colosseum/intent.md` v0.3.4 §2.5 and §3.2. It is one voice in a multi-model
fan-out experiment (analogous to specs/rcv.qnt for the Quint protocol layer).

Scope:
  - Types (§2.5)
  - Stage 1 `decrypt_and_validate` (opaque — modeled, not implemented)
  - Stage 2 `IRV_spec`         (opaque — modeled, not implemented)
  - Composition `Tally_spec`    (defined — composes Stage 1 + Stage 2)
  - Structural well-formedness theorems S6, S7, S8, S9 (§3.1)
  - B10_lean: the image-IO obligation (§3.2)

Proof bodies are all `sorry`. The methodology question this layer answers is
"does the theorem statement encode the intent's claim?", not "is the proof
discharged?". Discharge is a downstream task for a Lean-specialist model.

Stdlib only — no Mathlib import. Voices may use Mathlib if they prefer; the
dispatch script accepts either.

Revisions:
  - 2026-05-20: cross-critique fix. Split Stage 2 output into a separate
    `IRVResult` type per gpt-5.5's voice (Stage 2 produces only IRV-specific
    fields; voter bookkeeping is Stage 1's). Added `non_voters` to
    `DecryptedSet` so `Tally_spec` threads it correctly. Adopted gpt-5.5's
    `[i]?`-Option-pattern for s9_no_reappearance. See
    `.colosseum/specs/lean-cross-critique-2026-05-20/meta-analysis.md`.
  - 2026-05-20: intent v0.3.3 propagation. Added `cs.Nodup` hypothesis to
    S6/S7/S8/S9 per encoding-discipline note A4 (S2 distinctness required
    for downstream Lean specs). Added `irv_ballots_tallied` axiom per
    encoding-discipline note A5 (Stage 2 obligation that opaque IRV_spec
    cannot capture). See `.colosseum/specs/lean-critique-revised-canonical-2026-05-20/meta-analysis.md`.
-/

namespace VerifiedRcv

abbrev Addr  := String
abbrev Bytes := List UInt8

structure Ballot where
  ranking : List Addr
  deriving Repr, BEq

structure RawEntry where
  voter      : Addr
  ciphertext : Bytes
  deriving Repr

abbrev RawBallots   := List RawEntry
abbrev CandidateSet := List Addr
abbrev PrivKey      := Bytes

structure RoundCount where
  candidate : Addr
  count     : Nat
  deriving Repr, BEq

abbrev RoundCounts := List RoundCount

/-- Stage 2 output. Per intent §2.5, the combinatorial IRV core produces
ONLY these fields. Stage 1 bookkeeping (`dropped_voters`, `non_voters`) is
threaded by `Tally_spec`. Splitting this from `TallyResult` is the
2026-05-20 cross-critique fix (gpt-5.5's encoding; canonical conceded). -/
structure IRVResult where
  winners             : List Addr
  per_round_counts    : List RoundCounts
  eliminated_by_round : List (List Addr)
  ballots_tallied     : Nat
  deriving Repr, Inhabited

structure TallyResult where
  winners             : List Addr
  per_round_counts    : List RoundCounts
  eliminated_by_round : List (List Addr)
  ballots_tallied     : Nat
  ballots_dropped     : Nat
  dropped_voters      : List Addr
  non_voters          : List Addr
  deriving Repr, Inhabited

/-- Stage 1 output. Per intent §2.5, Stage 1 partitions raw ballots into
`valid` (decrypted, well-formed, candidates-in-set) and `dropped`
(decryption failure or validation failure), and computes `non_voters` as
`candidates \ raw_ballots.keys`. The 2026-05-20 cross-critique fix added
`non_voters` to this struct so `Tally_spec` can thread it correctly. -/
structure DecryptedSet where
  valid      : List (Addr × Ballot)
  dropped    : List Addr
  non_voters : List Addr
  deriving Repr, Inhabited

/-- Stage 1 (§2.5). Opaque: decrypts ciphertexts under `privkey`, classifies
each into the (valid, dropped, non_voters) partition. Validity criteria
(well-formed ranking, no duplicates, all candidates in `candidates`) are
part of the extracted model and not re-stated here. -/
opaque decrypt_and_validate
  (raw : RawBallots) (candidates : CandidateSet) (privkey : PrivKey) : DecryptedSet

/-- Stage 2 (§2.5). Opaque: the combinatorial IRV core. Repeatedly counts
first-preference votes, eliminates the lowest, until a winner reaches a
majority or all remaining are tied. Returns IRV-specific fields only;
voter bookkeeping is Stage 1's job. -/
opaque IRV_spec
  (valid : List (Addr × Ballot)) (candidates : CandidateSet) : IRVResult

/-- Stage 2 obligation (§2.5, encoding-discipline note A5 in intent v0.3.3).
The opaque declaration of `IRV_spec` cannot capture the `ballots_tallied :=
|valid_ballots|` relation by itself. This axiom restores the constraint so
S7 and S8 reason about a `ballots_tallied` value that is tied to the actual
count of valid ballots fed into the recursion. -/
axiom irv_ballots_tallied :
  ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
    (IRV_spec valid cs).ballots_tallied = valid.length

/-- Stage 1 partition obligation (§2.5). The opaque `decrypt_and_validate`
returns three voter lists that together partition `cs` (assuming `cs.Nodup`).
This axiom records the length-only consequence of that partition, which is
what `s7_voter_partition` needs. A stronger structural form (`List.Perm`-based)
is possible but not needed for the structural well-formedness theorems. -/
axiom decrypt_partition_length :
  ∀ (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey),
    cs.Nodup →
      (decrypt_and_validate raw cs pk).valid.length +
      (decrypt_and_validate raw cs pk).dropped.length +
      (decrypt_and_validate raw cs pk).non_voters.length = cs.length

/-- Stage 2 winner-shape obligation (§2.5, §3.1 S6). The opaque `IRV_spec`
cannot expose its `winners` list structure by itself. This axiom records
the three facts S6 asserts: every winner is a registered candidate, the
recursion always terminates with at least one winner (ties produce
multiple winners, never zero), and `|winners| ≤ |cs|` when `cs` is
duplicate-free (winners are a Nodup subset of cs). Length bound is stated
directly to match the length-only style of `decrypt_partition_length`. -/
axiom irv_winners_shape :
  ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
    cs.Nodup →
      (∀ w ∈ (IRV_spec valid cs).winners, w ∈ cs) ∧
      1 ≤ (IRV_spec valid cs).winners.length ∧
      (IRV_spec valid cs).winners.length ≤ cs.length

/-- Stage 2 round-conservation obligation (§2.5, §3.1 S8). Each round's
per-candidate counts sum to `ballots_tallied`: the count of valid ballots
fed into the recursion is conserved across rounds (an eliminated
candidate's ballots transfer to the next-ranked surviving candidate
rather than disappearing). -/
axiom irv_round_counts_sum :
  ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
    ∀ rc ∈ (IRV_spec valid cs).per_round_counts,
      (rc.foldl (fun acc r => acc + r.count) 0) = (IRV_spec valid cs).ballots_tallied

/-- Stage 2 elimination-monotonicity obligation (§2.5, §3.1 S9). Once
`IRV_spec` eliminates a candidate at round `i`, that candidate does not
reappear as a `RoundCount.candidate` entry in any later
`per_round_counts[j]?` with `j > i`. -/
axiom irv_no_reappearance :
  ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet)
    (i j : Nat) (eliminated : List Addr) (rc : RoundCounts) (c : Addr),
    (IRV_spec valid cs).eliminated_by_round[i]? = some eliminated →
    (IRV_spec valid cs).per_round_counts[j]? = some rc →
    i < j →
    c ∈ eliminated →
    ∀ entry ∈ rc, entry.candidate ≠ c

/-- Composition (§2.5). Threads Stage 1's voter bookkeeping (dropped,
non_voters) into Stage 2's IRV output. This is the transparent assembly
layer described in §2.5. -/
def Tally_spec
    (raw : RawBallots) (candidates : CandidateSet) (privkey : PrivKey) : TallyResult :=
  let d := decrypt_and_validate raw candidates privkey
  let r := IRV_spec d.valid candidates
  { winners             := r.winners
    per_round_counts    := r.per_round_counts
    eliminated_by_round := r.eliminated_by_round
    ballots_tallied     := r.ballots_tallied
    ballots_dropped     := d.dropped.length
    dropped_voters      := d.dropped
    non_voters          := d.non_voters }

/-- S6 — winner well-formedness (§3.1). Winners are a non-empty subset of
candidates, bounded above by `|candidates|`.

A4 hypothesis (intent v0.3.3): `cs.Nodup` is required because `cs.length`
overcounts for duplicate-containing lists, making the upper bound
unprovable without distinctness. -/
theorem s6_winner_subset
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (h_nodup : cs.Nodup) :
    let t := Tally_spec raw cs pk
    (∀ w ∈ t.winners, w ∈ cs) ∧ 1 ≤ t.winners.length ∧ t.winners.length ≤ cs.length := by
  simp only [Tally_spec]
  exact irv_winners_shape (decrypt_and_validate raw cs pk).valid cs h_nodup

/-- S7 — voter partition (§3.1). The candidate set decomposes as
tallied ⊔ dropped ⊔ non_voters. Stated here as the conservation equation;
disjointness is a corollary at extraction time.

A4 hypothesis (intent v0.3.3): `cs.Nodup` makes `cs.length` equal the
cardinality of the candidate set, which is what the conservation equation
relates to. Without it, duplicate-containing `cs` inflates the RHS. -/
theorem s7_voter_partition
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (h_nodup : cs.Nodup) :
    let t := Tally_spec raw cs pk
    t.ballots_tallied + t.ballots_dropped + t.non_voters.length = cs.length := by
  simp only [Tally_spec, irv_ballots_tallied]
  exact decrypt_partition_length raw cs pk h_nodup

/-- S8 — round counts sum (§3.1). For each round, the per-candidate counts
sum to `ballots_tallied`.

A4 hypothesis (intent v0.3.3): `cs.Nodup` is included for consistency with
the rest of the structural-invariant block; S8 itself does not depend on
`cs.length` but is stated under the same hypothesis to keep the theorem
family closed under composition. -/
theorem s8_round_counts_sum
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (_h_nodup : cs.Nodup) :
    let t := Tally_spec raw cs pk
    ∀ rc ∈ t.per_round_counts,
      (rc.foldl (fun acc r => acc + r.count) 0) = t.ballots_tallied := by
  simp only [Tally_spec]
  exact irv_round_counts_sum (decrypt_and_validate raw cs pk).valid cs

/-- S9 — no reappearance (§3.1). A candidate eliminated at round i does
not appear as a key in `per_round_counts[j]` for any j > i.

Encoding fix (2026-05-20 cross-critique): uses `[i]?`-Option-pattern from
gpt-5-5's voice rather than `[i]!`-with-bounds-check. Convergent finding:
the Option encoding avoids reliance on the partial `!` indexing's default
value semantics. Three reviewers agreed.

A4 hypothesis (intent v0.3.3): included for consistency with the rest of
the structural-invariant block; S9's content is index-relational and does
not directly depend on `cs.length`. -/
theorem s9_no_reappearance
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (_h_nodup : cs.Nodup) :
    let t := Tally_spec raw cs pk
    ∀ (i j : Nat) (eliminated : List Addr) (rc : RoundCounts) (c : Addr),
      t.eliminated_by_round[i]? = some eliminated →
      t.per_round_counts[j]? = some rc →
      i < j →
      c ∈ eliminated →
      ∀ entry ∈ rc, entry.candidate ≠ c := by
  simp only [Tally_spec]
  intro i j eliminated rc c h_elim h_rc h_lt h_mem
  exact irv_no_reappearance (decrypt_and_validate raw cs pk).valid cs
    i j eliminated rc c h_elim h_rc h_lt h_mem

/-
B10_lean (§3.2) — image-IO obligation.

`EnclaveImage` is the **fixed extracted-model symbol** representing the
externally observable input-output relation of the enclave binary. A proof
author MAY NOT instantiate `EnclaveImage := Tally_spec` to discharge
`B10_lean` by `rfl`; the symbol is reserved for the model produced by the
documented extraction discipline (Aeneas Rust-to-Lean extraction or
hand-written translation with stated extraction-soundness obligations).
The tautological-shadow check at extraction time is: `EnclaveImage` is
declared via `axiom` or `opaque` and never refined to `Tally_spec`.
-/

axiom EnclaveImage : RawBallots → CandidateSet → PrivKey → TallyResult

theorem B10_lean
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    EnclaveImage raw cs pk = Tally_spec raw cs pk := by
  sorry

end VerifiedRcv
