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

/-! ## Phase 1 bridge-lemma scaffolding

Methodology-driven enumeration (v0.4 ask AB.6 deferral-justification audit
applied to the prior monolithic `B10_lean_irv` sorry): each Phase 1 helper
gets a dedicated bridge lemma statement with its own `sorry`. The
discharge surface is now ten small inspectable per-helper goals plus the
loop-induction terminal goal, rather than one opaque theorem-blocking
sorry. Each lemma is independently provable by `progress` + induction over
the loop combinator; closing them in any order is sound.

Status: all sorry; closures queued for the focused Aeneas-bridge session.
The terminal `B10_lean_irv` theorem uses these lemmas as named hypotheses
so the loop-level proof can be sketched even while the helper proofs
remain open. -/

/-- Bridge for `addr_in`: extracted returns `Ok` iff the math containment
holds. -/
theorem addr_in_bridge
    (xs : Aeneas.Std.alloc.vec.Vec String) (a : String) :
    ∀ b,
      verified_rcv_enclave_core.addr_in xs a = .ok b →
      b = (a ∈ xs.v)
    := by
  sorry

/-- Bridge for `position_of`: extracted index equals the math
`position_of` result under `Usize.val`. -/
theorem position_of_bridge
    (a : String) (xs : Aeneas.Std.alloc.vec.Vec String) :
    ∀ r,
      verified_rcv_enclave_core.position_of a xs = .ok r →
      r.map (·.val) = position_of a xs.v
    := by
  sorry

/-- Bridge for `first_active_index`: returns the math first-active-index
over lifted lists. -/
theorem first_active_index_bridge
    (ranking : Aeneas.Std.alloc.vec.Vec String)
    (remaining : Aeneas.Std.alloc.vec.Vec String) :
    ∀ r,
      verified_rcv_enclave_core.first_active_index ranking remaining = .ok r →
      r.map (·.val) = first_active_index ranking.v remaining.v
    := by
  sorry

/-- Bridge for `count_at_index`: extracted count matches the math
`count_at_index` under `U32 → Nat`. -/
theorem count_at_index_bridge
    (valid_slice : Aeneas.Std.Slice (String × verified_rcv_enclave_core.Ballot))
    (remaining : Aeneas.Std.alloc.vec.Vec String)
    (target_idx : Aeneas.Std.Usize) :
    ∀ c,
      verified_rcv_enclave_core.count_at_index valid_slice remaining target_idx = .ok c →
      c.val = count_at_index remaining.v target_idx.val
        (lift_valid_slice valid_slice) := by
  sorry

/-- Bridge for `tally_round`: produces math-equivalent RoundCounts. -/
theorem tally_round_bridge
    (valid_slice : Aeneas.Std.Slice (String × verified_rcv_enclave_core.Ballot))
    (remaining : Aeneas.Std.alloc.vec.Vec String) :
    ∀ rcs,
      verified_rcv_enclave_core.tally_round valid_slice remaining = .ok rcs →
      rcs.v.map liftRoundCount = tally_round (lift_valid_slice valid_slice) remaining.v
    := by
  sorry

/-- Bridge for `min_count`: matches under `U32.val`. -/
theorem min_count_bridge
    (rcs : Aeneas.Std.alloc.vec.Vec verified_rcv_enclave_core.RoundCount) :
    ∀ m,
      verified_rcv_enclave_core.min_count rcs = .ok m →
      m.val = min_count (rcs.v.map liftRoundCount)
    := by
  sorry

/-- Bridge for `total_count`: matches under `U32.val`. -/
theorem total_count_bridge
    (rcs : Aeneas.Std.alloc.vec.Vec verified_rcv_enclave_core.RoundCount) :
    ∀ t,
      verified_rcv_enclave_core.total_count rcs = .ok t →
      t.val = total_count (rcs.v.map liftRoundCount)
    := by
  sorry

/-- Bridge for `candidates_with_count`: produces math-equivalent candidate
list. Argument order matches the extracted signature `(rc, m)`. -/
theorem candidates_with_count_bridge
    (rcs : Aeneas.Std.alloc.vec.Vec verified_rcv_enclave_core.RoundCount)
    (m : Aeneas.Std.U32) :
    ∀ cs,
      verified_rcv_enclave_core.candidates_with_count rcs m = .ok cs →
      cs.v = candidates_with_count m.val (rcs.v.map liftRoundCount)
    := by
  sorry

/-- Bridge for `first_majority_index`: extracted returns the index, math
returns the candidate. The bridge states presence-equivalence plus the
in-bounds property on any resolved index. The candidate-identity step
("`rc[idx].candidate = math-result`") is left to the loop-level proof,
which has the local context to discharge it under `idx.val < length`.
Argument order matches the extracted signature `(rc, threshold)`. -/
theorem first_majority_index_bridge
    (rcs : Aeneas.Std.alloc.vec.Vec verified_rcv_enclave_core.RoundCount)
    (threshold : Aeneas.Std.U32) :
    ∀ r,
      verified_rcv_enclave_core.first_majority_index rcs threshold = .ok r →
      r.isSome = (first_majority_candidate threshold.val
                    (rcs.v.map liftRoundCount)).isSome ∧
      ∀ idx, r = some idx → idx.val < rcs.v.length
    := by
  sorry

/-- Bridge for `remove_from`: vec-level identical algorithm to math. -/
theorem remove_from_bridge
    (to_remove : Aeneas.Std.alloc.vec.Vec String)
    (xs : Aeneas.Std.alloc.vec.Vec String) :
    ∀ r,
      verified_rcv_enclave_core.remove_from to_remove xs = .ok r →
      r.v = remove_from to_remove.v xs.v
    := by
  sorry

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

/-! ## Round 3f composition: B10 cross-layer assembly

Assembles the 5-link discharge (§8.7) for B10 into a single named theorem.
The shape makes each link's trust dependency explicit:

| link | what | discharge |
|---|---|---|
| 1 | `B10_lean_decrypt` (Stage 1) | axiom (line 306); gated on runtime crate Aeneas extraction |
| 2 | `B10_lean_irv` (Stage 2) | theorem with sorry (line 287); decomposes into 10 bridge sorries |
| 4 | `image_identity_binding` | axiom (below); operational, gated on reproducible-build pipeline |
| 5 | `B8` chain-witness | discharged on-chain via `verify_publish_quote` (contract.rs:893) |
| 6 | `dstack_kms_trust` | operational assumption (intent §6.3) |
| 7 | `enclave_input_fidelity` | discharged on-chain via `ballots_hash` (contract.rs:437) |

Links 5, 6, 7 are not Lean-discharged; they are chain-side or operational.
Links 1, 2, 4 are the Lean-discharged ingredients of `B10_lean`. -/

/-- **Link 4: image-identity-binding** (operational, off-chain).

States that the `EnclaveImage` symbol (`RcvSpec.lean:1485`) equals
`Tally_spec` on its math inputs. The binding is discharged operationally
by the reproducible-build pipeline: the wasm or TDX image's MRTD+RTMR
registered on-chain matches the hash of the binary that Aeneas extracted
from. Without a reproducible-build pipeline, this axiom is the load-bearing
trust gap.

v0.3.13 packaging: the axiom replaces v0.3.0's implicit "EnclaveImage
corresponds to extracted model" assumption with an explicit, refutable
claim. AB.6 deferral-justification audit: the deferral is justified by
named infrastructure (reproducible build) whose discharge plan is
recorded in intent §8.7 link 4. -/
axiom image_identity_binding
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    EnclaveImage raw cs pk = Tally_spec raw cs pk

/-- **B10_composition** (Round 3f).

Assembles links 1+2+4 into the math statement `B10_lean`. Currently the
composition collapses to a single application of `image_identity_binding`
because `B10_lean_decrypt` and `B10_lean_irv` operate on Aeneas types and
do not directly compose with the math `Tally_spec` statement without
intermediate lifts. The structured form makes future strengthening direct:

1. When `B10_lean_decrypt` graduates from `axiom (… → True)` to a real
   refinement claim (link 1 discharged via runtime crate Aeneas
   extraction), it weakens the trust load on `image_identity_binding`.
2. When `B10_lean_irv`'s 10 bridge sorries close (link 2 discharged), it
   weakens the trust load further.
3. When the reproducible-build pipeline lands (link 4 discharged), the
   only remaining trust dependencies are links 5/6/7 — and 5/7 are
   chain-witnessed, 6 is the `dstack_kms_trust` operational assumption.

In the current v0.3.13 state, `B10_composition` and `B10_lean` are
"true modulo `image_identity_binding`" — the entire Lean discharge
collapses to one named operational axiom. The audit reading is: B10
holds for the verified-rcv deployment when the chain's registered
MRTD/RTMR identifies a binary extracted by Aeneas from the documented
source tree. -/
theorem B10_composition
    (raw : RawBallots) (cs : CandidateSet) (pk : PrivKey) :
    EnclaveImage raw cs pk = Tally_spec raw cs pk := by
  exact image_identity_binding raw cs pk

end VerifiedRcv
