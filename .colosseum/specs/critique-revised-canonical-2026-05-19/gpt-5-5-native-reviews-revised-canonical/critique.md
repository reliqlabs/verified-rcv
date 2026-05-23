# Re-critique of revised canonical: gpt-5-5-native

## Q1. Is the S8/S9 fix structurally sound?

Yes, for the stated S8/S9 obligations.

`s8_round_counts_sum` folds over `t.per_round_counts` and requires `sum_round(m) == t.ballots_tallied` for every round map `m`. That matches intent §3.1 S8: each `per_round_counts[i]` sums to `ballots_tallied`.

`s9_no_reappearance` is also structurally correct for intent §3.1 S9. It walks `per_round_counts` by index, checks the current round keys against candidates eliminated in earlier rounds, then adds the candidates eliminated at the current round. This permits a candidate to appear in `per_round_counts[i]` when they are eliminated in `eliminated_by_round[i]`, and forbids them from appearing in any later `per_round_counts[j]`, `j > i`.

The predicate does not enforce full IRV structure, such as candidate-set membership for every eliminated address, uniqueness of eliminations, or exact length relation between `per_round_counts` and `eliminated_by_round`. Those are outside the narrow S9 statement, but should be covered elsewhere if the Quint layer is meant to validate more than the monotone-no-reappearance shape.

## Q2. Did the revision introduce any new defects?

No new structural defect in the S8/S9 repair itself.

The new `t_one_round` branch is reachable in principle: after crossing `END_AT` with no submitted ballots, `is_tally_spec_output` accepts the shape with `ballots_tallied = 0`, two zero-sum rounds, and one eliminated candidate. That means S9 is not purely syntactic dead code; it can check a later round whose key set omits a previously eliminated candidate.

Coverage caveat: the existing reachability witnesses do not specifically require an elimination-round tally to be reached. `quint run` is sampling-based here, and a successful `all_invariants` run can publish only `t_zero_round`. Add a witness such as “published tally has nonempty `eliminated_by_round`” if the convergence experiment wants positive coverage for the new S9 path.

## Q3. Remaining concerns

The most material remaining concern is B10 under-encoding in `is_tally_spec_output`. The predicate is named and commented as a Tally-spec output check, but it only checks bookkeeping fields derivable from the sealed input set plus `well_formed_tally`. It does not enforce the IRV result relation.

This is observable in simulation: `quint run --invariant=all_invariants` produced a resolved state with three submitted ballots, `per_round_counts = [{A: 3, B: 0, C: 0, D: 0, E: 0}]`, and `winners = {A, B, C, D, E}`. That is syntactically well-formed under the current predicates but not a valid IRV outcome, since A has a strict majority. The comments say the IRV-specific proof lives in `B10_lean`, so this may be an intentional protocol-layer over-approximation. If so, the spec should avoid calling this `is_tally_spec_output` without qualification and should require a composition artifact tying this Quint predicate to the Lean B10 discharge. If not intentional, B10 remains materially under-specified in the canonical Quint layer.

## Optional notes

The revised canonical is stronger than my v11 on B2 because it keeps a ghost snapshot and checks post-end equality as a state invariant. It is also stronger on S7 bookkeeping after adding `dropped_voters.size() == ballots_dropped`.

STATUS: ok
