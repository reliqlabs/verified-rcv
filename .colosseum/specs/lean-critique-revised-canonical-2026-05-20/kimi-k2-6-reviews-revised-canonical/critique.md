# Re-critique of revised canonical: kimi-k2-6

## Q1. Is the fix structurally sound?

Yes. All four cross-critique findings are addressed correctly:

- `IRVResult` is a distinct structure containing only the four IRV-core fields (`winners`, `per_round_counts`, `eliminated_by_round`, `ballots_tallied`). It derives `Inhabited` and is returned by `IRV_spec`.
- `DecryptedSet` now carries `non_voters : List Addr`, sourced from Stage 1.
- `Tally_spec` threads correctly: Stage-2 fields (`winners`, `per_round_counts`, `eliminated_by_round`, `ballots_tallied`) come from `r : IRVResult`; Stage-1 fields (`ballots_dropped`, `dropped_voters`, `non_voters`) come from `d : DecryptedSet`; `ballots_dropped` is computed as `d.dropped.length`.
- `s9_no_reappearance` uses the `[i]? = some ...` Option pattern. No `[i]!` remains.

The composition is now transparent: `Tally_spec` is a definitional assembly of two opaque stages, which matches the intent's §2.5 composition description exactly.

## Q2. Did the revision introduce any new defects?

None.

The refactor is a pure structural improvement. Every field on `Tally_spec`'s output retains its prior source or gains a correct new source (`non_voters` from `d.non_voters`). Theorem statements reference fields that are now well-defined via the composition. The Option-pattern S9 statement retains `i < j` correctly and bounds indices implicitly through the `some` hypotheses. No `Inhabited` derivation is missing or spurious.

## Q3. Remaining concerns

The single most material remaining concern is **(a) CandidateSet distinctness not encoded at type level**.

The intent's S2 states `candidates` contains no duplicate addresses. This is a load-bearing structural invariant: `s7_voter_partition` claims `ballots_tallied + ballots_dropped + non_voters.length = cs.length`. If `cs` contains duplicates, `cs.length` overcounts, and the conservation equation fails even for a perfectly honest Stage 1 and Stage 2. The theorem as stated is therefore too strong (it claims the equation holds for all `cs`, including duplicates) and does not accurately encode the intent's qualified claim.

The fix is simple: add a `cs.Nodup` hypothesis to S6, S7, S8, and S9, or lift `CandidateSet` to a `List Addr` bundled with a distinctness proof. The revision should have addressed this because it is a prerequisite for the correctness of the very theorems the cross-critique was trying to save.

Concern (b) (`ballots_tallied = d.valid.length` not axiomatized) is real but less urgent: it is a constraint on the opaque `IRV_spec` that proof authors can add as a Stage-2 correctness axiom when they reach discharge. Concern (c) is largely handled by the `decrypt_and_validate` typing.

## Optional notes

The canonical's use of named record types (`RawEntry`, `RoundCount`) rather than raw pairs is an improvement over my fan-out spec. It makes field access explicit (`entry.candidate` vs `p.fst`) and reduces the risk of transposition errors in downstream proofs. This is a readability win on an axis the cross-critique did not cover.

A candidate for intent v0.3.3: explicitly state the expected length relation between `per_round_counts` and `eliminated_by_round` (the worked example shows `eliminated_by_round.length = per_round_counts.length - 1` in the typical case, but the all-abstain boundary case gives `0 = 1 - 1`). Encoding this as a well-formedness predicate would close a small gap in S9's preconditions.

STATUS: ok
