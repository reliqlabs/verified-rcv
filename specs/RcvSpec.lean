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
  - 2026-05-24: Round 3e concreteness pass. Replaced `opaque IRV_spec` with a
    concrete recursive definition mirroring intent §2.5's algorithm. The
    four interface axioms (irv_ballots_tallied, irv_winners_shape,
    irv_round_counts_sum, irv_no_reappearance) are now theorem statements
    against the concrete definition. `irv_ballots_tallied` is discharged
    by definitional equality (the trivial case); the other three are
    `sorry`-bodied theorems whose discharge requires induction on the
    `irv_loop` fuel parameter (multi-week proof work).
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

/-- `xs` with elements of `to_remove` filtered out, preserving order. -/
def remove_from (to_remove : List Addr) : List Addr → List Addr
  | []      => []
  | x :: xs =>
    if to_remove.contains x then remove_from to_remove xs
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

/-- Stage 2 winner-shape obligation (§2.5, §3.1 S6). Every winner is a
registered candidate; the recursion always terminates with at least one
winner (ties produce multiple winners, never zero); `|winners| ≤ |cs|`
when `cs.Nodup`.

Per intent v0.3.5 encoding-discipline note A7, the `1 ≤ cs.length`
hypothesis is required: for `cs = []`, the IRV core returns
`winners = []` (defensive zero-candidates path), violating
`1 ≤ winners.length`. Block 1 enforces `len(candidates) ≥ 1` so S6 only
applies post-instantiation; the hypothesis propagates that dependency.

Discharge plan: induction on `irv_loop` fuel. Each branch either records
winners directly (terminal cases — `[w]`, `remaining`, `candidates`) or
recurses on a strictly smaller problem. The winners set is always either
a subset of `remaining` (which is `⊆ candidates`) or `candidates` itself. -/
theorem irv_winners_shape :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
      cs.Nodup →
      1 ≤ cs.length →
        (∀ w ∈ (IRV_spec valid cs).winners, w ∈ cs) ∧
        1 ≤ (IRV_spec valid cs).winners.length ∧
        (IRV_spec valid cs).winners.length ≤ cs.length := by
  sorry

/-- Stage 2 round-conservation obligation (§2.5, §3.1 S8). Each round's
per-candidate counts sum to `ballots_tallied`. Eliminated candidates'
ballots transfer to the next-ranked surviving candidate rather than
disappearing, so total ballots is conserved across rounds.

Discharge plan: induction on `irv_loop` fuel. The base step is the
all-abstain zero-round case where every count is 0 (sum 0 = ballots_tallied
0 by S7's all-abstain corollary). The inductive step relies on
`tally_round` placing each valid ballot into exactly one bucket (its
first-active-among-remaining), and `count_at_index` summing across
buckets recovering `|valid|`. -/
theorem irv_round_counts_sum :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet),
      ∀ rc ∈ (IRV_spec valid cs).per_round_counts,
        (rc.foldl (fun acc r => acc + r.count) 0) =
          (IRV_spec valid cs).ballots_tallied := by
  sorry

/-- Stage 2 elimination-monotonicity obligation (§2.5, §3.1 S9). A
candidate eliminated at round `i` does not reappear as a `RoundCount`
entry in any later `per_round_counts[j]?` with `j > i`.

Discharge plan: induction on `irv_loop` fuel + the invariant that
`tally_round` is called with `remaining` shrinking monotonically across
recursive calls. `losers` is removed from `remaining` before the
recursive call, and `tally_round` enumerates `remaining` only — so any
candidate in `eliminated_by_round[i]` cannot appear as `rc.candidate` in
any subsequent `tally_round` output. -/
theorem irv_no_reappearance :
    ∀ (valid : List (Addr × Ballot)) (cs : CandidateSet)
      (i j : Nat) (eliminated : List Addr) (rc : RoundCounts) (c : Addr),
      (IRV_spec valid cs).eliminated_by_round[i]? = some eliminated →
      (IRV_spec valid cs).per_round_counts[j]? = some rc →
      i < j →
      c ∈ eliminated →
      ∀ entry ∈ rc, entry.candidate ≠ c := by
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
sum to `ballots_tallied`. -/
theorem s8_round_counts_sum
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) (_h_nodup : cs.Nodup) :
    let t := Tally_spec raw cs pk
    ∀ rc ∈ t.per_round_counts,
      (rc.foldl (fun acc r => acc + r.count) 0) = t.ballots_tallied := by
  simp only [Tally_spec]
  exact irv_round_counts_sum (decrypt_and_validate raw cs pk).valid cs

/-- S9 — no reappearance (§3.1). A candidate eliminated at round i does
not appear as a key in `per_round_counts[j]` for any j > i. -/
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
