/-
EnclaveBridge.lean — refinement bridge between the Aeneas-extracted enclave
core (`EnclaveExtracted.lean`, produced by charon + aeneas from
`crates/enclave-core/`) and the math spec (`RcvSpec.lean`).

This file carries the lifts that translate Aeneas's representation
(`Slice` / `Vec` / `Result` monad) to the math representation (`List` /
plain) and states the central refinement theorem `B10_lean_irv` that
anchors B10_lean.

The lift functions use Aeneas's `Slice.v` and `Vec.v` abbreviations that
project to the underlying `List`. The per-element conversions for the
extracted `Ballot` and `RoundCount` structs are direct field rewrites
since the underlying types are isomorphic (both are records with
`Vec String` / `Nat`-counted fields).

`B10_lean_irv`'s proof is `sorry` — the discharge is multi-week work
even with good tools. The lifts being implemented means the theorem
statement is now concrete and the goal is inspectable in `lake env lean`.
-/

import RcvSpec
import EnclaveExtracted

namespace VerifiedRcv

open Aeneas Aeneas.Std

/-! ## Per-element type conversions

The extracted IRV core defines its own `Ballot` and `RoundCount` structs
(matching the Rust crate's types). These are isomorphic to the math
spec's types but use `Aeneas.Std.alloc.vec.Vec` instead of `List` for
inner sequences. The conversions below unwrap one layer at a time.
-/

/-- Convert an extracted `Ballot` (with `Vec String` ranking) to the math
`Ballot` (with `List Addr` ranking). `Addr := String` so the element type
matches; only the container wraps differently. -/
def liftBallot (b : verified_rcv_enclave_core.Ballot) : Ballot :=
  { ranking := b.ranking.v }

/-- Convert an extracted `RoundCount` to the math `RoundCount`. The
`count` field is `Std.U32` on the extracted side, `Nat` on the math
side; `U32.toNat` is the underlying conversion. -/
def liftRoundCount (rc : verified_rcv_enclave_core.RoundCount) : RoundCount :=
  { candidate := rc.candidate
    count     := rc.count.val }

/-! ## Container-level lifts -/

/-- Lift an Aeneas `Slice` of (Addr, Ballot) pairs to a math `List` of
the math types. Used for the `valid` ballots input to `irv_spec`. -/
def lift_valid_slice
    (s : Aeneas.Std.Slice (String × verified_rcv_enclave_core.Ballot)) :
    List (Addr × Ballot) :=
  s.v.map (fun p => (p.fst, liftBallot p.snd))

/-- Lift the extracted `Vec String` to a math `CandidateSet`. Since
`Addr := String` and `CandidateSet := List Addr`, this is just `.v`. -/
def lift_candidate_vec (v : Aeneas.Std.alloc.vec.Vec String) : CandidateSet :=
  v.v

/-- Lift the extracted `IRVResult` (with `Vec` fields and `U32` count) to
the math `IRVResult` (with `List` fields and `Nat` count). Per-field
projection with appropriate per-element lifts. -/
def lift_irv_result
    (r : verified_rcv_enclave_core.IRVResult) : IRVResult :=
  { winners             := r.winners.v
    per_round_counts    := r.per_round_counts.v.map (fun rc_vec =>
                             rc_vec.v.map liftRoundCount)
    eliminated_by_round := r.eliminated_by_round.v.map (fun v => v.v)
    ballots_tallied     := r.ballots_tallied.val }

/-! ## B10_lean_irv: extracted IRV matches math IRV

The central Stage-2 obligation: the extracted enclave's `irv_spec`
agrees with the math `IRV_spec` on lifted inputs/outputs.

Stated as a `Result`-monad refinement: if the extracted `irv_spec`
returns `.ok r` (i.e., doesn't panic), then the lifted result equals the
math `IRV_spec` applied to the lifted inputs.

The proof requires:
  1. Showing the extracted IRV core never panics for our inputs (Aeneas
     `progress` tactic + monotonicity on the bounded recursion).
  2. Induction matching the extracted loop structure (`irv_spec_loop0`,
     `tally_round_loop`, `first_majority_index_loop`, etc.) to the math
     `IRV_spec` definition — which is currently `opaque` in `RcvSpec.lean`.
     To close this gap, the math `IRV_spec` needs to be made concrete
     (define it explicitly as a Lean function mirroring intent §2.5's
     algorithm) OR a refinement axiom is added asserting the relation.

For the current commit, this is `sorry`. The discharge plan is in
`.colosseum/roadmap.md` (Round 3e). -/
theorem B10_lean_irv
    (valid_slice : Aeneas.Std.Slice (String × verified_rcv_enclave_core.Ballot))
    (candidates : Aeneas.Std.alloc.vec.Vec String) :
    ∀ r,
      verified_rcv_enclave_core.irv_spec valid_slice candidates = .ok r →
      lift_irv_result r =
        IRV_spec (lift_valid_slice valid_slice) (lift_candidate_vec candidates)
    := by
  sorry

/-! ## B10_lean_decrypt: Stage 1 placeholder

The extracted Stage 1 `decrypt_and_validate` returns `fail panic`
because the Rust crate's body is `unimplemented!()` (Stage 1 lives in
the runtime crate `verified-rcv-enclave` which has ECIES + dstack access;
Aeneas extraction of that crate is queued for a future round).

For this round the lemma is an axiom placeholder; the discharge path is
recorded in the ledger as a future obligation. -/
axiom B10_lean_decrypt
    (raw : Aeneas.Std.alloc.vec.Vec verified_rcv_enclave_core.RawEntry)
    (candidates : Aeneas.Std.alloc.vec.Vec String)
    (privkey : Aeneas.Std.alloc.vec.Vec Aeneas.Std.U8) :
    ∀ d,
      verified_rcv_enclave_core.decrypt_and_validate raw candidates privkey = .ok d →
      True

end VerifiedRcv
