/-
RcvSpec.lean — verified-rcv math spec (Claude-authored canonical baseline)

This is the Lean math-layer spec for the IRV tally pipeline named in
`.colosseum/intent.md` v0.3.4 §2.5 and §3.2. It is one voice in a multi-model
fan-out experiment (analogous to specs/rcv.qnt for the Quint protocol layer).

Scope:
  - Types (§2.5)
  - Stage 1 `decrypt_and_validate` (opaque — Stage 1 body lives in runtime crate)
  - Stage 2 `IRV_spec`         (CONCRETE — implements intent §2.5 algorithm)
  - Composition `Tally_spec`    (defined — composes Stage 1 + Stage 2)
  - Structural well-formedness theorems S6, S7, S8, S9 (§3.1)
  - B10_lean: the image-IO obligation (§3.2)

Imports Mathlib as of Round 3e — the structural proofs for the Stage-2
obligation theorems need `List.Nodup.length_le_of_subset` and standard
list lemmas. The spec definitions themselves remain stdlib-shaped (no
Mathlib types in the type signatures); Mathlib is used in proof bodies
only.

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
  - 2026-05-24: Round 3e concreteness pass. Replaced `opaque IRV_spec` with a
    concrete recursive definition mirroring intent §2.5's algorithm. The
    four interface axioms (irv_ballots_tallied, irv_winners_shape,
    irv_round_counts_sum, irv_no_reappearance) are now theorem statements
    against the concrete definition. `irv_ballots_tallied` is discharged
    by definitional equality (the trivial case); the other three are
    `sorry`-bodied theorems whose discharge requires induction on the
    `irv_loop` fuel parameter (multi-week proof work).
-/

import Mathlib.Data.List.Basic
import Mathlib.Data.List.Nodup
import Mathlib.Data.List.Perm.Subperm

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
each into the (valid, dropped, non_voters) partition. Body lives in the
runtime crate (`verified-rcv-enclave`) which has ECIES + dstack access;
that crate's Aeneas extraction is queued for a future round. -/
opaque decrypt_and_validate
  (raw : RawBallots) (candidates : CandidateSet) (privkey : PrivKey) : DecryptedSet

/-! ## Concrete IRV core (§2.5 algorithm)

Helper functions mirror the Rust `crates/enclave-core/` implementation
shape but use plain `List` rather than `Vec` / `Slice`. The recursion is
bounded by `candidates.length + 1` (each non-terminal round strictly
shrinks `remaining`, so fuel is a safe upper bound). -/

/-- Linear search: position of `a` in `xs`, mirroring Rust's `position_of`. -/
def position_of (a : Addr) : List Addr → Option Nat :=
  let rec aux (i : Nat) : List Addr → Option Nat
    | []      => none
    | x :: xs => if x = a then some i else aux (i + 1) xs
  aux 0

/-- First index in `remaining` that the ballot's `ranking` prefers, mirroring
Rust's `first_active_index`. Structural recursion on `ranking`. -/
def first_active_index : List Addr → List Addr → Option Nat
  | [],       _         => none
  | r :: rs,  remaining =>
    match position_of r remaining with
    | some idx => some idx
    | none     => first_active_index rs remaining

/-- Count ballots whose first-active choice maps to `remaining[target_idx]`.
Structural recursion on `valid` (the ballot list). -/
def count_at_index (remaining : List Addr) (target_idx : Nat) :
    List (Addr × Ballot) → Nat
  | []           => 0
  | (_, b) :: vs =>
    let suffix := count_at_index remaining target_idx vs
    match first_active_index b.ranking remaining with
    | some idx => if idx = target_idx then suffix + 1 else suffix
    | none     => suffix

/-- Tally first-preference counts per surviving candidate. Helper takes the
absolute index so `count_at_index` matches the candidate's position. -/
def tally_round_aux (valid : List (Addr × Ballot)) (remaining_full : List Addr) :
    Nat → List Addr → RoundCounts
  | _, []      => []
  | i, c :: cs =>
    { candidate := c
      count     := count_at_index remaining_full i valid }
      :: tally_round_aux valid remaining_full (i + 1) cs

def tally_round (valid : List (Addr × Ballot)) (remaining : List Addr) : RoundCounts :=
  tally_round_aux valid remaining 0 remaining

/-- Minimum count across a non-empty `RoundCounts`. Defensive zero for empty. -/
def min_count : RoundCounts → Nat
  | []      => 0
  | r :: rs => rs.foldl (fun m e => Nat.min m e.count) r.count

/-- Sum of counts in `rc`. -/
def total_count : RoundCounts → Nat
  | []      => 0
  | r :: rs => r.count + total_count rs

/-- All candidates whose count equals `m`, in `rc` order. -/
def candidates_with_count (m : Nat) : RoundCounts → List Addr
  | []      => []
  | r :: rs =>
    if r.count = m then r.candidate :: candidates_with_count m rs
    else candidates_with_count m rs

/-- First index in `rc` whose count strictly exceeds `threshold` (majority). -/
def first_majority_candidate (threshold : Nat) : RoundCounts → Option Addr
  | []      => none
  | r :: rs =>
    if r.count > threshold then some r.candidate
    else first_majority_candidate threshold rs

/-- `xs` with elements of `to_remove` filtered out, preserving order.
Uses `x ∈ to_remove` (Decidable Prop) rather than `to_remove.contains x`
(Bool) so the proof obligations work cleanly with Mathlib lemmas. -/
def remove_from (to_remove : List Addr) : List Addr → List Addr
  | []      => []
  | x :: xs =>
    if x ∈ to_remove then remove_from to_remove xs
    else x :: remove_from to_remove xs

/-- IRV recursion core. Returns `(winners, per_round_counts, eliminated_by_round)`.
Fuel parameter bounds the recursion; intent §2.5 guarantees `|candidates|`
rounds suffice (each non-terminal round strictly shrinks `remaining`). -/
def irv_loop
    (valid : List (Addr × Ballot)) (candidates : CandidateSet) :
    Nat → List Addr → (List Addr × List RoundCounts × List (List Addr))
  | 0,        _         => (candidates, [], [])    -- defensive fuel exhaustion
  | fuel + 1, remaining =>
    let rc := tally_round valid remaining
    if remaining.length = 0 then
      (candidates, [rc], [])
    else if remaining.length = 1 then
      (remaining, [rc], [])
    else
      let total := total_count rc
      if total = 0 then
        -- All-abstain: pin the recorded round to zero counts over the
        -- ORIGINAL candidate set per intent §2.5.
        let zero_round : RoundCounts :=
          candidates.map (fun c => { candidate := c, count := 0 })
        (candidates, [zero_round], [])
      else
        let threshold := total / 2
        match first_majority_candidate threshold rc with
        | some w => ([w], [rc], [])
        | none   =>
          let m      := min_count rc
          let losers := candidates_with_count m rc
          if losers.length = remaining.length then
            -- Terminal tie: all remaining co-win, no further elimination.
            (remaining, [rc], [])
          else
            let (w, rcs, elims) :=
              irv_loop valid candidates fuel (remove_from losers remaining)
            (w, rc :: rcs, losers :: elims)

/-- Stage 2 (§2.5). The combinatorial IRV core. Repeatedly counts
first-preference votes, batch-eliminates the lowest-tied set, terminates
on majority or all-remaining-tied. The Australian Federal IRV variant.

Concrete definition (Round 3e). Earlier revisions declared this `opaque`;
the four interface axioms below are now theorem statements against this
concrete definition. -/
def IRV_spec (valid : List (Addr × Ballot)) (candidates : CandidateSet) : IRVResult :=
  let triple :=
    if candidates.length = 0 then
      ([], [], [])
    else
      irv_loop valid candidates (candidates.length + 1) candidates
  { winners             := triple.1
    per_round_counts    := triple.2.1
    eliminated_by_round := triple.2.2
    ballots_tallied     := valid.length }

/-! ## Auxiliary lemmas for Stage-2 obligation proofs

These small lemmas factor out the structural facts needed to discharge
`irv_winners_shape` and friends. Each is independently provable from the
helper definitions above. -/

/-- `remove_from` produces a sublist of its input. -/
lemma remove_from_subset (to_remove xs : List Addr) :
    ∀ y ∈ remove_from to_remove xs, y ∈ xs := by
  induction xs with
  | nil => intro y h; simp [remove_from] at h
  | cons x rest ih =>
    intro y hy
    unfold remove_from at hy
    by_cases hx : x ∈ to_remove
    · rw [if_pos hx] at hy
      exact List.mem_cons_of_mem _ (ih y hy)
    · rw [if_neg hx] at hy
      cases hy with
      | head _ => exact List.mem_cons_self ..
      | tail _ hmem => exact List.mem_cons_of_mem _ (ih y hmem)

/-- `remove_from` preserves `Nodup` (no duplicates introduced; only removal). -/
lemma remove_from_nodup (to_remove : List Addr) {xs : List Addr} (h : xs.Nodup) :
    (remove_from to_remove xs).Nodup := by
  induction xs with
  | nil => simp [remove_from]
  | cons x rest ih =>
    unfold remove_from
    by_cases hx : x ∈ to_remove
    · rw [if_pos hx]
      exact ih (List.Nodup.of_cons h)
    · rw [if_neg hx]
      refine List.Nodup.cons ?_ (ih (List.Nodup.of_cons h))
      intro hmem
      exact (List.nodup_cons.mp h).1 (remove_from_subset _ _ x hmem)

/-- If some element of `xs` is not in `to_remove`, then `remove_from` has
positive length. Used in the recursive branch of `irv_loop` to show the
new `remaining` is non-empty. -/
lemma remove_from_length_pos
    {to_remove xs : List Addr} (h : ∃ x ∈ xs, x ∉ to_remove) :
    0 < (remove_from to_remove xs).length := by
  induction xs with
  | nil => obtain ⟨x, hmem, _⟩ := h; simp at hmem
  | cons x rest ih =>
    unfold remove_from
    by_cases hx : x ∈ to_remove
    · rw [if_pos hx]
      apply ih
      obtain ⟨y, hy, hyt⟩ := h
      cases hy with
      | head _ => exact absurd hx hyt
      | tail _ hyr => exact ⟨y, hyr, hyt⟩
    · rw [if_neg hx]
      simp

/-- Every `RoundCount.candidate` in `tally_round_aux`'s output is a member
of the `cs` argument (the suffix being walked). -/
lemma tally_round_aux_candidates_in_cs
    (valid : List (Addr × Ballot)) (remaining_full : List Addr) :
    ∀ (i : Nat) (cs : List Addr) (rc : RoundCount),
      rc ∈ tally_round_aux valid remaining_full i cs → rc.candidate ∈ cs := by
  intro i cs
  induction cs generalizing i with
  | nil => intro rc h; simp [tally_round_aux] at h
  | cons c rest ih =>
    intro rc h
    simp [tally_round_aux] at h
    cases h with
    | inl heq => exact heq ▸ List.mem_cons_self ..
    | inr h => exact List.mem_cons_of_mem _ (ih (i + 1) rc h)

/-- Specialisation of `tally_round_aux_candidates_in_cs`: every entry in
`tally_round valid remaining` has its candidate in `remaining`. -/
lemma tally_round_candidates_in_remaining
    (valid : List (Addr × Ballot)) (remaining : List Addr) :
    ∀ rc ∈ tally_round valid remaining, rc.candidate ∈ remaining := by
  intro rc h
  unfold tally_round at h
  exact tally_round_aux_candidates_in_cs valid remaining 0 remaining rc h

/-- If `first_majority_candidate threshold rc = some w`, then `w` is one of
the candidates listed in `rc`. -/
lemma first_majority_candidate_in_rc
    (threshold : Nat) :
    ∀ (rc : RoundCounts) (w : Addr),
      first_majority_candidate threshold rc = some w →
      ∃ entry ∈ rc, entry.candidate = w := by
  intro rc
  induction rc with
  | nil => intro w h; simp [first_majority_candidate] at h
  | cons r rs ih =>
    intro w h
    unfold first_majority_candidate at h
    by_cases hr : r.count > threshold
    · simp [hr] at h
      exact ⟨r, List.mem_cons_self .., h⟩
    · simp [hr] at h
      obtain ⟨entry, hmem, heq⟩ := ih w h
      exact ⟨entry, List.mem_cons_of_mem _ hmem, heq⟩

/-- `candidates_with_count` produces a sublist of `rc.map .candidate`. Each
member of the loser set was listed in `rc`. -/
lemma candidates_with_count_mem
    (m : Nat) :
    ∀ (rc : RoundCounts) (c : Addr),
      c ∈ candidates_with_count m rc →
      ∃ entry ∈ rc, entry.candidate = c := by
  intro rc
  induction rc with
  | nil => intro c h; simp [candidates_with_count] at h
  | cons r rs ih =>
    intro c h
    unfold candidates_with_count at h
    by_cases hr : r.count = m
    · simp [hr] at h
      cases h with
      | inl heq => exact ⟨r, List.mem_cons_self .., heq.symm⟩
      | inr h =>
        obtain ⟨entry, hmem, heq⟩ := ih c h
        exact ⟨entry, List.mem_cons_of_mem _ hmem, heq⟩
    · simp [hr] at h
      obtain ⟨entry, hmem, heq⟩ := ih c h
      exact ⟨entry, List.mem_cons_of_mem _ hmem, heq⟩

/-- `tally_round_aux`'s candidates are Nodup as long as the input suffix
`cs` is. The function emits each `c ∈ cs` exactly once in `cs` order. -/
lemma tally_round_aux_candidates_nodup
    (valid : List (Addr × Ballot)) (remaining_full : List Addr) :
    ∀ (i : Nat) (cs : List Addr),
      cs.Nodup →
      ((tally_round_aux valid remaining_full i cs).map (·.candidate)).Nodup := by
  intro i cs
  induction cs generalizing i with
  | nil => intro _; simp [tally_round_aux]
  | cons c rest ih =>
    intro h_nodup
    unfold tally_round_aux
    simp only [List.map_cons]
    rw [List.nodup_cons]
    refine ⟨?_, ih (i + 1) (List.Nodup.of_cons h_nodup)⟩
    intro hmem
    rw [List.mem_map] at hmem
    obtain ⟨rc, hrc, heq⟩ := hmem
    have h_in_rest : rc.candidate ∈ rest :=
      tally_round_aux_candidates_in_cs valid remaining_full (i + 1) rest rc hrc
    rw [heq] at h_in_rest
    exact (List.nodup_cons.mp h_nodup).1 h_in_rest

/-- `tally_round`'s candidates are Nodup whenever `remaining` is. -/
lemma tally_round_candidates_nodup
    (valid : List (Addr × Ballot)) (remaining : List Addr) (h_nodup : remaining.Nodup) :
    ((tally_round valid remaining).map (·.candidate)).Nodup := by
  unfold tally_round
  exact tally_round_aux_candidates_nodup valid remaining 0 remaining h_nodup

/-- `candidates_with_count` produces a Nodup list whenever its `rc` input
has Nodup candidates. Filtering preserves the Nodup property. -/
lemma candidates_with_count_nodup
    (m : Nat) (rc : RoundCounts) (h_nodup : (rc.map (·.candidate)).Nodup) :
    (candidates_with_count m rc).Nodup := by
  induction rc with
  | nil => simp [candidates_with_count]
  | cons r rs ih =>
    unfold candidates_with_count
    simp only [List.map_cons] at h_nodup
    rw [List.nodup_cons] at h_nodup
    obtain ⟨h_head, h_tail⟩ := h_nodup
    by_cases hr : r.count = m
    · rw [if_pos hr]
      rw [List.nodup_cons]
      refine ⟨?_, ih h_tail⟩
      intro hmem
      obtain ⟨entry, h_em, h_eq⟩ := candidates_with_count_mem _ _ r.candidate hmem
      apply h_head
      rw [List.mem_map]
      exact ⟨entry, h_em, h_eq⟩
    · rw [if_neg hr]
      exact ih h_tail

/-! ## Stage 2 obligation theorems (formerly axioms; intent v0.3.3 A5)

These were declared as axioms when `IRV_spec` was `opaque`. Now that
`IRV_spec` is concrete, each becomes a theorem against the concrete
definition. `irv_ballots_tallied` is discharged by definitional equality
(the field is built as `valid.length` regardless of which branch the body
takes). The other three are `sorry`-bodied; their discharge requires
induction on `irv_loop`'s fuel parameter and is the multi-week Round 3e
work documented in `.colosseum/roadmap.md`. -/

/-- Stage 2 obligation: `ballots_tallied` equals `|valid|`. By construction,
`IRV_spec` sets `ballots_tallied := valid.length` in every branch, so this
is `rfl`. Stated as a theorem (formerly an axiom) per the Round 3e
concreteness pass. -/
theorem irv_ballots_tallied :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
      (IRV_spec valid cs).ballots_tallied = valid.length := by
  intro valid cs
  rfl

/-- Stage 1 obligation (§2.5). The opaque `decrypt_and_validate` returns
three voter lists that partition `cs` (assuming `cs.Nodup`). Length-only
form; structural `List.Perm` form is possible but not needed for S7. -/
axiom decrypt_partition_length :
  ∀ (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey),
    cs.Nodup →
      (decrypt_and_validate raw cs pk).valid.length +
      (decrypt_and_validate raw cs pk).dropped.length +
      (decrypt_and_validate raw cs pk).non_voters.length = cs.length

/-- Generalised invariant for `irv_loop`'s winners (the `.1` of the result
triple). For any fuel and any `remaining ⊆ cs` satisfying Nodup and
non-empty, the winners are: a subset of `cs`, Nodup, non-empty, and
bounded by `cs.length`. The four sub-clauses are conjoined because they
mutually-imply across the inductive step (e.g. recursive `remaining'`
needs Nodup + non-empty established before the winners conclusion).

Discharge plan: structural induction on fuel. Each branch:
- fuel = 0: result = `(cs, [], [])` — clauses follow from `h_pos_cs`,
  `h_nodup_cs`, refl ⊆ and ≤.
- `remaining.length = 0`: contradicts `h_pos_rem`.
- `remaining.length = 1`: result winners = `remaining`. Subclauses follow
  from `h_sub`, `h_nodup_rem`, `h_pos_rem`.
- `total = 0`: result winners = `cs`. Same as fuel=0.
- `first_majority = some w`: result winners = `[w]`. `w ∈ rc` (by
  `first_majority_candidate_in_rc`), `rc.candidate ∈ remaining` (by
  `tally_round_candidates_in_remaining`), `remaining ⊆ cs` (hyp).
- Terminal tie: result winners = `remaining`. Same as length=1 branch.
- Recursive: result winners = recursive winners on `remove_from losers
  remaining`. Need `new_remaining ⊆ cs ∧ new_remaining.Nodup ∧
  1 ≤ new_remaining.length`. The first two from `remove_from_subset`
  composed with hyp + `remove_from_nodup`; the third from
  `remove_from_length_pos` once we observe `∃ x ∈ remaining, x ∉ losers`
  (which holds because we already excluded the case `losers.length =
  remaining.length`). -/
theorem irv_loop_winners_invariant
    (valid : List (Addr × Ballot)) (cs : CandidateSet)
    (h_nodup_cs : cs.Nodup) (h_pos_cs : 1 ≤ cs.length) :
    ∀ (fuel : Nat) (remaining : List Addr),
      (∀ x ∈ remaining, x ∈ cs) →
      remaining.Nodup →
      1 ≤ remaining.length →
      (∀ x ∈ (irv_loop valid cs fuel remaining).1, x ∈ cs) ∧
      (irv_loop valid cs fuel remaining).1.Nodup ∧
      1 ≤ (irv_loop valid cs fuel remaining).1.length ∧
      (irv_loop valid cs fuel remaining).1.length ≤ cs.length := by
  intro fuel
  induction fuel with
  | zero =>
    intro remaining _ _ _
    -- fuel = 0 branch: result is `(cs, [], [])`
    simp only [irv_loop]
    exact ⟨fun _ h => h, h_nodup_cs, h_pos_cs, Nat.le_refl _⟩
  | succ fuel' ih =>
    intro remaining h_sub h_nodup_rem h_pos_rem
    -- Unfold one step of the loop.
    simp only [irv_loop]
    -- Helper: |remaining| ≤ |cs| (used twice).
    have h_rem_le_cs : remaining.length ≤ cs.length :=
      (List.Nodup.subperm h_nodup_rem h_sub).length_le
    -- Branch on remaining.length = 0 (contradicts h_pos_rem).
    by_cases h_len0 : remaining.length = 0
    · exact absurd h_len0 (Nat.one_le_iff_ne_zero.mp h_pos_rem)
    rw [if_neg h_len0]
    -- Branch on remaining.length = 1.
    by_cases h_len1 : remaining.length = 1
    · rw [if_pos h_len1]
      exact ⟨h_sub, h_nodup_rem, h_pos_rem, h_rem_le_cs⟩
    rw [if_neg h_len1]
    -- Branch on total_count rc = 0.
    by_cases h_total0 : total_count (tally_round valid remaining) = 0
    · rw [if_pos h_total0]
      exact ⟨fun _ h => h, h_nodup_cs, h_pos_cs, Nat.le_refl _⟩
    rw [if_neg h_total0]
    -- Branch on first_majority_candidate.
    cases h_maj : first_majority_candidate
        (total_count (tally_round valid remaining) / 2)
        (tally_round valid remaining) with
    | some w =>
      obtain ⟨entry, h_entry_mem, h_entry_eq⟩ :=
        first_majority_candidate_in_rc _ _ _ h_maj
      have h_w_in_rem : w ∈ remaining := by
        rw [← h_entry_eq]
        exact tally_round_candidates_in_remaining valid remaining entry h_entry_mem
      have h_w_in_cs : w ∈ cs := h_sub w h_w_in_rem
      refine ⟨?_, List.nodup_singleton w, ?_, ?_⟩
      · intro x hx
        rw [List.mem_singleton] at hx
        exact hx ▸ h_w_in_cs
      · show 1 ≤ [w].length; simp
      · show [w].length ≤ cs.length
        simp
        exact h_pos_cs
    | none =>
      by_cases h_tie : (candidates_with_count
          (min_count (tally_round valid remaining))
          (tally_round valid remaining)).length = remaining.length
      · rw [if_pos h_tie]
        exact ⟨h_sub, h_nodup_rem, h_pos_rem, h_rem_le_cs⟩
      rw [if_neg h_tie]
      -- Recursive case: apply IH on (remove_from losers remaining).
      set losers := candidates_with_count
          (min_count (tally_round valid remaining))
          (tally_round valid remaining) with h_losers_def
      set rem' := remove_from losers remaining with h_rem'_def
      have h_sub' : ∀ x ∈ rem', x ∈ cs :=
        fun x h => h_sub x (remove_from_subset losers remaining x h)
      have h_nodup_rem' : rem'.Nodup := remove_from_nodup losers h_nodup_rem
      -- losers ⊆ remaining via tally_round_candidates_in_remaining + candidates_with_count_mem
      have h_losers_sub : ∀ x ∈ losers, x ∈ remaining := by
        intro x hx
        obtain ⟨entry, h_em, h_eq⟩ := candidates_with_count_mem _ _ x hx
        rw [← h_eq]
        exact tally_round_candidates_in_remaining valid remaining entry h_em
      -- |losers| ≤ |remaining|: losers is a List with possibly-duplicate entries,
      -- but each entry corresponds to a unique entry in `tally_round` which
      -- enumerates `remaining` (Nodup). Without proving losers.Nodup separately,
      -- we observe: at least one element of remaining is NOT in losers (else
      -- losers.length ≥ remaining.length contradicts h_tie via the IH).
      -- losers.Nodup via the chain: tally_round produces Nodup candidates
      -- (since remaining.Nodup) ⇒ candidates_with_count preserves Nodup.
      have h_losers_nodup : losers.Nodup :=
        candidates_with_count_nodup _ _
          (tally_round_candidates_nodup valid remaining h_nodup_rem)
      have h_losers_len_le : losers.length ≤ remaining.length :=
        (List.Nodup.subperm h_losers_nodup h_losers_sub).length_le
      have h_pos_rem' : 1 ≤ rem'.length := by
        have h_exists_not : ∃ x ∈ remaining, x ∉ losers := by
          by_contra h_all
          have h_all' : ∀ x ∈ remaining, x ∈ losers := by
            intro x hx
            by_contra hxn
            exact h_all ⟨x, hx, hxn⟩
          have h_ge : remaining.length ≤ losers.length :=
            (List.Nodup.subperm h_nodup_rem h_all').length_le
          -- Sandwich: losers.length ≤ remaining.length ≤ losers.length,
          -- so they're equal — contradicts h_tie.
          exact h_tie (Nat.le_antisymm h_losers_len_le h_ge)
        exact remove_from_length_pos h_exists_not
      have ih_app := ih rem' h_sub' h_nodup_rem' h_pos_rem'
      -- Result destructuring: `let (w, rcs, elims) := irv_loop ...; (w, ...)`
      -- The result triple's `.1` is precisely the recursive `.1`.
      simp only at ih_app ⊢
      exact ih_app

/-- Stage 2 winner-shape obligation (§2.5, §3.1 S6). Every winner is a
registered candidate; the recursion always terminates with at least one
winner (ties produce multiple winners, never zero); `|winners| ≤ |cs|`
when `cs.Nodup`.

Per intent v0.3.5 encoding-discipline note A7, the `1 ≤ cs.length`
hypothesis is required: for `cs = []`, the IRV core returns
`winners = []` (defensive zero-candidates path), violating
`1 ≤ winners.length`. Block 1 enforces `len(candidates) ≥ 1` so S6 only
applies post-instantiation; the hypothesis propagates that dependency.

Discharge: specialise `irv_loop_winners_invariant` with
`remaining := cs`. -/
theorem irv_winners_shape :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
      cs.Nodup →
      1 ≤ cs.length →
        (∀ w ∈ (IRV_spec valid cs).winners, w ∈ cs) ∧
        1 ≤ (IRV_spec valid cs).winners.length ∧
        (IRV_spec valid cs).winners.length ≤ cs.length := by
  intro valid cs h_nodup h_pos
  unfold IRV_spec
  have hne : cs.length ≠ 0 := Nat.one_le_iff_ne_zero.mp h_pos
  -- The `if cs.length = 0` branch is excluded; reduce to the else-branch.
  simp only [hne, if_false]
  -- Apply the loop invariant with `remaining := cs`.
  have inv := irv_loop_winners_invariant valid cs h_nodup h_pos
    (cs.length + 1) cs (fun _ h => h) h_nodup h_pos
  exact ⟨inv.1, inv.2.2.1, inv.2.2.2⟩

/-- Stage 2 round-conservation obligation (§2.5, §3.1 S8). Each round's
per-candidate counts sum to `ballots_tallied`.

Per intent v0.3.6 encoding-discipline note A8, the
`∀ (_, b) ∈ valid, ∀ c ∈ cs, c ∈ b.ranking` hypothesis is required:
without it, exhausted ballots trigger the all-abstain branch with
sum = 0 ≠ valid.length. Stage 1 validates ballots as permutations of
`cs`, so the hypothesis propagates that constraint to Stage 2.

Discharge plan: induction on `irv_loop` fuel. Under the hypothesis,
total > 0 at every recursion step (each ballot has a first-active in
`remaining ⊆ cs`), so the all-abstain branch is only reached when
`valid = []` (the base trivial case). Otherwise, `tally_round` places
each ballot into exactly one bucket, summing to `valid.length`. -/
theorem irv_round_counts_sum :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
      (∀ p ∈ valid, ∀ c ∈ cs, c ∈ p.snd.ranking) →
      ∀ rc ∈ (IRV_spec valid cs).per_round_counts,
        (rc.foldl (fun acc r => acc + r.count) 0) =
          (IRV_spec valid cs).ballots_tallied := by
  sorry

/-- Stage 2 elimination-monotonicity obligation (§2.5, §3.1 S9). A
candidate eliminated at round `i` does not reappear as a `RoundCount`
entry in any later `per_round_counts[j]?` with `j > i`.

Per intent v0.3.6 encoding-discipline note A8, the
`∀ (_, b) ∈ valid, ∀ c ∈ cs, c ∈ b.ranking` hypothesis is required:
without it, the all-abstain branch can trigger at deep recursion
(`total = 0` at step `k > 0`), writing a round with the FULL `cs` —
reintroducing every eliminated candidate. The hypothesis blocks that
branch for non-empty `valid`.

Discharge plan: induction on `irv_loop` fuel + the invariant that
`tally_round` is called with `remaining` shrinking monotonically across
recursive calls. `losers` is removed from `remaining` before the
recursive call, and `tally_round` enumerates `remaining` only — so any
candidate in `eliminated_by_round[i]` cannot appear as `rc.candidate` in
any subsequent `tally_round` output. -/
theorem irv_no_reappearance :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
      (∀ p ∈ valid, ∀ c ∈ cs, c ∈ p.snd.ranking) →
      ∀ (i j : Nat) (eliminated : List Addr) (rc : RoundCounts) (c : Addr),
      (IRV_spec valid cs).eliminated_by_round[i]? = some eliminated →
      (IRV_spec valid cs).per_round_counts[j]? = some rc →
      i < j →
      c ∈ eliminated →
      ∀ entry ∈ rc, entry.candidate ≠ c := by
  intro valid cs _h_cover i j eliminated rc c h_elim h_rc h_lt h_c_mem entry h_entry
  -- The proof splits on whether `valid = []` (vacuous via empty
  -- eliminated_by_round) vs `valid ≠ []` (the structural induction case).
  --
  -- Outline of the `valid ≠ []` case (the substantive work):
  --
  -- 1. Auxiliary lemma `irv_loop_step_total_pos`: under the cover
  --    hypothesis, if `valid ≠ []` and `remaining ⊆ cs` and
  --    `1 ≤ remaining.length`, then
  --    `total_count (tally_round valid remaining) > 0`. Proof: at least
  --    one ballot exists; pick its first-active index in `remaining`
  --    (defined because every ballot covers `cs ⊇ remaining`), and that
  --    bucket's count is ≥ 1.
  --
  -- 2. Corollary: under A8 + non-empty valid, the all-abstain branch
  --    inside `irv_loop` is unreachable. The all-abstain branch is the
  --    one that writes `per_round_counts[k] = cs.map(c => {c, 0})` —
  --    the only place where S9 would otherwise break (eliminated
  --    candidates can reappear there).
  --
  -- 3. Auxiliary lemma `irv_loop_per_round_in_remaining`: under A8 +
  --    non-empty valid, every entry in `(irv_loop ... remaining).2.1[k]`
  --    has its `.candidate` in `remaining_at_step_k`, where
  --    `remaining_at_step_(k+1) = remove_from elims[k] remaining_at_step_k`.
  --    Proof: induction on fuel, case-split irv_loop's branches; the
  --    all-abstain branch is excluded by (2).
  --
  -- 4. Joint invariant: c ∈ eliminated_by_round[i] ⇒ c ∈ remaining_at_step_i
  --    (since losers ⊆ rc.candidates ⊆ remaining); and c ∉
  --    remaining_at_step_(i+1) (by `remove_from`). By monotonicity of
  --    `remaining_at_step`, c ∉ remaining_at_step_j for all j ≥ i+1.
  --    By (3), c ∉ per_round_counts[j].candidates. Hence entry.candidate ≠ c.
  --
  -- This is multi-step Lean work analogous to `irv_winners_shape`'s
  -- discharge (~10 additional auxiliary lemmas + the joint invariant).
  -- Tracked as a follow-on for the same Round 3e pass.
  sorry

/-- Composition (§2.5). Threads Stage 1's voter bookkeeping (dropped,
non_voters) into Stage 2's IRV output. -/
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

Hypotheses (intent v0.3.3 A4 + v0.3.5 A7):
- `cs.Nodup`: `cs.length` overcounts for duplicate lists, making the upper
  bound unprovable without distinctness.
- `1 ≤ cs.length`: for `cs = []`, IRV returns `winners = []` (defensive
  zero-candidates path); Block 1 enforces this at instantiate-time. -/
theorem s6_winner_subset
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey)
    (h_nodup : cs.Nodup) (h_nonempty : 1 ≤ cs.length) :
    let t := Tally_spec raw cs pk
    (∀ w ∈ t.winners, w ∈ cs) ∧ 1 ≤ t.winners.length ∧ t.winners.length ≤ cs.length := by
  simp only [Tally_spec]
  exact irv_winners_shape (decrypt_and_validate raw cs pk).valid cs h_nodup h_nonempty

/-- S7 — voter partition (§3.1). The candidate set decomposes as
tallied ⊔ dropped ⊔ non_voters. -/
theorem s7_voter_partition
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (h_nodup : cs.Nodup) :
    let t := Tally_spec raw cs pk
    t.ballots_tallied + t.ballots_dropped + t.non_voters.length = cs.length := by
  simp only [Tally_spec, irv_ballots_tallied]
  exact decrypt_partition_length raw cs pk h_nodup

/-- S8 — round counts sum (§3.1). For each round, the per-candidate counts
sum to `ballots_tallied`.

Hypothesis (intent v0.3.6 A8): every Stage-1-valid ballot covers `cs`
(Stage 1 validates as permutation; this is the weakest sufficient form). -/
theorem s8_round_counts_sum
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (_h_nodup : cs.Nodup)
    (h_cover : ∀ p ∈ (decrypt_and_validate raw cs pk).valid, ∀ c ∈ cs, c ∈ p.snd.ranking) :
    let t := Tally_spec raw cs pk
    ∀ rc ∈ t.per_round_counts,
      (rc.foldl (fun acc r => acc + r.count) 0) = t.ballots_tallied := by
  simp only [Tally_spec]
  exact irv_round_counts_sum (decrypt_and_validate raw cs pk).valid cs h_cover

/-- S9 — no reappearance (§3.1). A candidate eliminated at round i does
not appear as a key in `per_round_counts[j]` for any j > i.

Hypothesis (intent v0.3.6 A8): same ballot-cover condition as S8. -/
theorem s9_no_reappearance
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (_h_nodup : cs.Nodup)
    (h_cover : ∀ p ∈ (decrypt_and_validate raw cs pk).valid, ∀ c ∈ cs, c ∈ p.snd.ranking) :
    let t := Tally_spec raw cs pk
    ∀ (i j : Nat) (eliminated : List Addr) (rc : RoundCounts) (c : Addr),
      t.eliminated_by_round[i]? = some eliminated →
      t.per_round_counts[j]? = some rc →
      i < j →
      c ∈ eliminated →
      ∀ entry ∈ rc, entry.candidate ≠ c := by
  simp only [Tally_spec]
  intro i j eliminated rc c h_elim h_rc h_lt h_mem
  exact irv_no_reappearance (decrypt_and_validate raw cs pk).valid cs h_cover
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
