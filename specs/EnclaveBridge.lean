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

## Discharge plan (Round 3e-bridge, future-session work)

The math layer is now COMPLETE: all four Stage-2 obligations
(`irv_ballots_tallied`, `irv_winners_shape`, `irv_round_counts_sum`,
`irv_no_reappearance`) are proven theorems against the concrete math
`IRV_spec` in `RcvSpec.lean` (Round 3e closed 2026-05-26). What remains
is the BRIDGE: showing the Aeneas-extracted code's behavior matches.

The bridge is a multi-day Aeneas refinement proof. Architectural plan:

### Phase 1: Per-helper bridge lemmas (`@[step]` style)

Each extracted helper has a math counterpart. Bridge lemma asserts they
agree under the lifts:

- `addr_in_bridge`: `addr_in xs a = .ok b` iff `b = (a ∈ xs.v)`. (Math
  uses `x ∈ to_remove`; no separate `addr_in` function.)
- `position_of_bridge`: `position_of needle haystack = .ok r` iff
  `r.map .val = position_of_math needle haystack.v` (bridging the
  Std.Usize → Nat conversion).
- `first_active_index_bridge`: similar, but over the ranking × remaining
  shape.
- `count_at_index_bridge`: bridging valid : Slice ↔ List.
- `tally_round_bridge`: produces matching RoundCounts.
- `min_count_bridge`, `total_count_bridge`: U32 → Nat conversions.
- `candidates_with_count_bridge`: Vec ↔ List of String.
- `first_majority_index_bridge` ↔ `first_majority_candidate`:
  NOTE: extracted returns the INDEX; math returns the CANDIDATE.
  Bridge involves extracting `rc.candidate` from the indexed entry.
- `remove_from_bridge`: Vec ↔ List, identical algorithm.

Each ~30-50 lines of Lean proof using Aeneas's `step`/`progress`
tactic and induction over the `loop` combinator.

### Phase 2: irv_loop0 bridge

The main loop. Match `verified_rcv_enclave_core.irv_spec_loop0` (using
Aeneas's `loop` + `ControlFlow`) to math's `irv_loop` (fuel-bounded
recursion). Branch-by-branch correspondence:
- `round >= max_rounds`: matches math's `fuel = 0` (defensive).
- `remaining.length = 0/1`: terminal branches with single rc.
- `total = 0`: all-abstain. **Tricky**: extracted uses `Vec.push` of
  tally_round result then `index_mut_back` to retroactively REPLACE the
  last entry with zero_round. Math directly returns
  `[zero_round_over_cs]`. Semantically equivalent; the bridge must
  show `(push rc; index_mut_back 0 zero_round).v = [zero_round]`.
- `first_majority = some idx`: extracted reads `rc[idx].candidate` and
  builds singleton vec. Math returns `[w]` where `w` is the candidate.
- terminal tie: both return remaining as winners.
- Recursive: extracted continues; math recurses on smaller fuel.

### Phase 3: irv_spec top-level

Compose Phase 1+2 with the top-level structure (empty-candidates
short-circuit + the loop invocation).

### Risks

- **Vec.push + index_mut_back semantics**: needs an Aeneas-specific
  lemma about retroactive list-update.
- **Std.Usize.ofNat bounds**: position indices need a bound proof.
- **Loop combinator induction**: Aeneas's `loop` desugars to a Lean
  function; induction on the "step count" requires a measure argument.
- **Vec.clone calls**: extracted has many `clone` operations whose
  semantics are `Vec.v` preserves. Need to either bridge or pre-prove
  Vec.clone refines identity at the lift level.

### Why deferred from this session

The math-layer discharge (~24 new Lean lemmas, all 4 Stage-2 obligations
proven, A7+A8 methodology findings patched) is a single coherent piece
of verification work. The bridge proof is a SEPARATE multi-day project
requiring Aeneas-framework expertise. Mixing the two risks subtle
errors in the trust chain. Per the methodology's "conservative/safe"
guidance, the bridge is queued for its own focused session.

The current `sorry` here is well-defined and structurally sound: the
lift functions are concrete, the math `IRV_spec` is concrete, and the
goal is `lake env lean`-inspectable. -/
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
