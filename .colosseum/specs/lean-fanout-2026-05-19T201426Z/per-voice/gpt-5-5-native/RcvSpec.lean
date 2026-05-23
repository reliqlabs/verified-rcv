/-
RcvSpec.lean, gpt-5-5-native voice.

Lean math-layer spec for verified-rcv intent v0.3.2. This file models the
off-chain tally function named by B10, not the chain protocol. Stage 1 and
Stage 2 are opaque; Tally_spec is transparent composition.
-/

namespace VerifiedRcv

abbrev Addr := String
abbrev Bytes := List UInt8
abbrev PrivKey := Bytes
abbrev CandidateSet := List Addr

/-- A validated full-preference ballot. The ranking is intended to be a
candidate-order permutation of the declared candidate set. -/
structure Ballot where
  preferences : List Addr
  deriving Repr, BEq, Inhabited

/-- Raw encrypted ballot entry. The list order represents candidate-declaration
order restricted to voters with a stored ciphertext, matching §2.5 Stage 1. -/
structure RawBallot where
  voter : Addr
  ciphertext : Bytes
  deriving Repr, BEq, Inhabited

abbrev RawBallots := List RawBallot
abbrev ValidBallots := List (Addr × Ballot)

/-- Ordered count map for one IRV round. The order is declaration order over
surviving candidates, not address sort order. -/
abbrev RoundCounts := List (Addr × Nat)

def roundTotal (rc : RoundCounts) : Nat :=
  rc.foldl (fun acc entry => acc + entry.snd) 0

def roundMentions (c : Addr) (rc : RoundCounts) : Prop :=
  ∃ n, (c, n) ∈ rc

/-- Stage 1 output: decrypted valid ballots plus voter-accounting fields. -/
structure DecryptedSet where
  valid : ValidBallots
  dropped_voters : List Addr
  non_voters : List Addr
  deriving Repr, Inhabited

/-- Stage 2 output contains only IRV recursion fields. Stage-1 bookkeeping is
threaded into the final TallyResult by Tally_spec. -/
structure IRVResult where
  winners : List Addr
  per_round_counts : List RoundCounts
  eliminated_by_round : List (List Addr)
  ballots_tallied : Nat
  deriving Repr, Inhabited

structure TallyResult where
  winners : List Addr
  per_round_counts : List RoundCounts
  eliminated_by_round : List (List Addr)
  ballots_tallied : Nat
  ballots_dropped : Nat
  dropped_voters : List Addr
  non_voters : List Addr
  deriving Repr, Inhabited

/-- Stage 1 (§2.5): decrypt, Borsh-parse as Vec<Addr>, validate as a
permutation of candidates, and emit dropped/non-voter bookkeeping in candidate
declaration order. Opaque here because this spec fixes the interface, not the
implementation. -/
opaque decrypt_and_validate : RawBallots → CandidateSet → PrivKey → DecryptedSet

/-- Stage 2 (§2.5): the deterministic IRV recursion with majority termination,
terminal ties as co-winners, and batch elimination of all lowest-count tied
candidates. Opaque here because theorem statements, not implementation proof,
are the fan-out artifact. -/
opaque IRV_spec : ValidBallots → CandidateSet → IRVResult

/-- Composition (§2.5): Stage 1 feeds Stage 2; Stage-1 dropped/non-voter data is
threaded into the final published TallyResult. This must remain transparent. -/
def Tally_spec (raw : RawBallots) (candidates : CandidateSet) (privkey : PrivKey) : TallyResult :=
  let dec := decrypt_and_validate raw candidates privkey
  let irv := IRV_spec dec.valid candidates
  { winners := irv.winners,
    per_round_counts := irv.per_round_counts,
    eliminated_by_round := irv.eliminated_by_round,
    ballots_tallied := irv.ballots_tallied,
    ballots_dropped := dec.dropped_voters.length,
    dropped_voters := dec.dropped_voters,
    non_voters := dec.non_voters }

/-- S6 (§3.1): a published winner list is nonempty and contains only declared
candidates. The upper bound records the intent's `len(winners) ≤ len(candidates)`
well-formedness clause. -/
theorem s6_winner_subset (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    let t := Tally_spec raw cs pk
    (∀ w, w ∈ t.winners → w ∈ cs) ∧ 1 ≤ t.winners.length ∧ t.winners.length ≤ cs.length := by
  sorry

/-- S7 (§3.1): candidate-voters are partitioned into tallied, dropped, and
non-voting accounts. This statement records the conservation equation required
by the intent. -/
theorem s7_voter_partition (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    let t := Tally_spec raw cs pk
    t.ballots_tallied + t.ballots_dropped + t.non_voters.length = cs.length := by
  sorry

/-- S8 (§3.1): every round count accounts for exactly the successfully tallied
ballots. -/
theorem s8_round_counts_sum (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    let t := Tally_spec raw cs pk
    ∀ rc, rc ∈ t.per_round_counts → roundTotal rc = t.ballots_tallied := by
  sorry

/-- S9 (§3.1): once eliminated, a candidate never appears in a later round's
surviving-candidate count map. -/
theorem s9_no_reappearance (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    let t := Tally_spec raw cs pk
    ∀ (i : Nat) (eliminated : List Addr) (j : Nat) (rc : RoundCounts) (c : Addr),
      t.eliminated_by_round[i]? = some eliminated →
      t.per_round_counts[j]? = some rc →
      i < j →
      c ∈ eliminated →
      ¬ roundMentions c rc := by
  sorry

/-
`EnclaveImage` is the reserved Lean-extracted model of the enclave binary with
the explicit §2.5 3-argument semantic signature. It is intentionally not a def
equal to Tally_spec.
-/
axiom EnclaveImage : RawBallots → CandidateSet → PrivKey → TallyResult

/-- B10_lean (§3.2): off-chain image-IO obligation. For every frozen raw ballot
set, candidate declaration list, and dstack-derived private key, the extracted
enclave model agrees with the mathematical tally specification. -/
theorem B10_lean :
    ∀ raw cs pk, EnclaveImage raw cs pk = Tally_spec raw cs pk := by
  sorry

end VerifiedRcv
