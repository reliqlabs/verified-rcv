// -*- mode: Bluespec; -*-
//
// verified-rcv — protocol-layer spec for the three-component IRV system
// (CosmWasm contract + dstack-TDX enclave + zkdcap attestation).
//
// Source of truth: ../.colosseum/intent.md v0.3.3 (A4/A5 added 2026-05-20;
// confirms the Quint spec needs no change — Set[Addr] is built-in deduplicated,
// and the nondet ballots_tallied search with well_formed_tally enforces the
// Stage-2 obligation that A5 axiomatizes for the Lean side).
//
// 2026-05-19 revision: TallyResult restored to intent §2.5 schema with
// per_round_counts + eliminated_by_round (replacing the synthetic
// rounds_played). S8 and S9 now encoded as real predicates rather than
// trivial bounds (kimi + gpt-5.5 cross-critique of canonical found the
// previous encoding insufficient). dropped_voters.size() == ballots_dropped
// added to well_formed_tally (kimi secondary finding).
// Idiomatic references: CometBFT reactor.qnt (Informal Systems),
// commonware/minimmit replica.qnt (BFT consensus).
//
// Encoding discipline (departing from the earlier LLM-anchored draft):
// - Temporal claims (B1..B7) are encoded by action-guard disablement
//   plus plain forall-style state invariants. No last_action tagging.
// - Ghost variables are used only where a temporal anchor cannot be
//   reconstructed from current state (B2 needs the snapshot at end_at).
//   Such vars are prefixed `ghost_`.
// - Negative scenarios (premature/late ballot, non-candidate, double
//   publish) are not separate actions — they are properties Quint
//   verifies by checking that no reachable trace violates the
//   corresponding invariant, since the action guards forbid the
//   bad transitions.
// - B8 / B9 are meta-security; Quint encodes only the conditional
//   "publish_result fired => registry was honest" preamble.
// - B10 is cross-layer; Quint encodes the chain-side projection
//   (published tally is a well-formed Tally_spec output over the
//   enclave's consumed ballot set, with consumed = ballots_at_end_at).

module rcv {

  // ============================================================
  // Parameters (bounded universe for finite model checking)
  // ============================================================

  // Candidate set + non-candidate witnesses, declared at the module
  // top with `const` so the spec can be instantiated for different
  // election sizes. A concrete instantiation lives in main.qnt.
  const CANDIDATES: List[str]
  const NON_CANDIDATES: Set[str]
  const START_AT: int
  const END_AT: int

  // Sample ciphertext universe and time-tick set, used by `step`.
  // Bounded so the model checker terminates.
  const CIPHERTEXTS: Set[str]
  const TICK_DELTAS: Set[int]

  // ============================================================
  // Types
  // ============================================================

  type Addr = str
  type Ciphertext = str
  type Time = int

  // Derived phase — not a stored variable; a pure function of
  // (block_time, start_at, end_at, tally_result).
  type Phase =
    | Created
    | Voting
    | Tallying
    | Resolved

  // Published tally schema (intent §2.5).
  // per_round_counts and eliminated_by_round are intent-mandated audit
  // fields. S8 requires each per_round_counts[i] to sum to ballots_tallied;
  // S9 requires no candidate eliminated at round i to appear in any
  // per_round_counts[j] for j > i.
  type TallyResult = {
    winners: Set[Addr],
    per_round_counts: List[Addr -> int],
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: Set[Addr],
    non_voters: Set[Addr],
  }

  type OptionalTally = {
    present: bool,
    value: TallyResult,
  }

  // On-chain registry tuple (intent §6.1 v0.3.1).
  type Registry = {
    vkey: str,
    mrtd: str,
    rtmr: str,
  }

  // ============================================================
  // Constants — canonical values for the predicate definitions
  // ============================================================

  pure val CANONICAL_REGISTRY: Registry = {
    vkey: "verified_rcv_vkey",
    mrtd: "verified_rcv_mrtd",
    rtmr: "verified_rcv_rtmr",
  }

  pure val EMPTY_TALLY: TallyResult = {
    winners: Set(),
    per_round_counts: List(),
    eliminated_by_round: List(),
    ballots_tallied: 0,
    ballots_dropped: 0,
    dropped_voters: Set(),
    non_voters: Set(),
  }

  pure val candidate_set: Set[Addr] = CANDIDATES.foldl(Set(), (acc, a) => acc.union(Set(a)))

  // ============================================================
  // State variables
  // ============================================================

  var contract_ballots: Addr -> Ciphertext
  var contract_tally_result: OptionalTally
  var registry: Registry
  var block_time: Time

  // Enclave-side state. `consumed_ballots` is what the enclave
  // actually decrypted — modeling `enclave_input_fidelity` from §8.7.
  // If a malicious host fed a different input, this would diverge
  // from `ghost_ballots_at_end_at`; the chain-side B10 projection
  // requires equality.
  var enclave_has_session_key: bool
  var enclave_consumed_ballots: Addr -> Ciphertext
  var enclave_computed_tally: OptionalTally

  // Ghost state — anchors B2 (`ballots@end_at` frozen).
  // Captured the first tick that crosses end_at; never modified after.
  var ghost_ballots_at_end_at: Addr -> Ciphertext
  var ghost_ballots_at_end_at_captured: bool

  // ============================================================
  // Pure predicates
  // ============================================================

  pure def derived_phase(bt: Time, tr: OptionalTally): Phase = {
    if (tr.present) Resolved
    else if (bt < START_AT) Created
    else if (bt < END_AT) Voting
    else Tallying
  }

  // intent §6.1: image_registration_honest(σ)
  pure def image_registration_honest(r: Registry): bool = {
    r.vkey == CANONICAL_REGISTRY.vkey
      and r.mrtd == CANONICAL_REGISTRY.mrtd
      and r.rtmr == CANONICAL_REGISTRY.rtmr
  }

  // Sum the values of a per-round count map.
  pure def sum_round(m: Addr -> int): int = {
    m.keys().fold(0, (acc, k) => acc + m.get(k))
  }

  // S8: each per_round_counts[i] map has values summing to ballots_tallied.
  pure def s8_round_counts_sum(t: TallyResult): bool = {
    t.per_round_counts.foldl(true, (acc, m) => acc and sum_round(m) == t.ballots_tallied)
  }

  // S9: no candidate eliminated at round i appears in per_round_counts[j]
  // for j > i. Encoded by walking rounds in order with an accumulator of
  // "already eliminated" addresses; at each round, the current
  // per_round_counts map's keys must be disjoint from the eliminated set,
  // then we add the round's freshly-eliminated candidates to the set.
  pure def s9_no_reappearance(t: TallyResult): bool = {
    val pairs_init: (bool, Set[Addr]) = (true, Set())
    val final = t.per_round_counts.indices().fold(pairs_init, (acc, i) => {
      val ok = acc._1
      val eliminated_so_far = acc._2
      val round_keys = t.per_round_counts[i].keys()
      val newly_eliminated =
        if (i < t.eliminated_by_round.length())
          t.eliminated_by_round[i].foldl(Set(), (s, c) => s.union(Set(c)))
        else Set()
      val ok_after =
        ok and round_keys.intersect(eliminated_so_far).size() == 0
      (ok_after, eliminated_so_far.union(newly_eliminated))
    })
    final._1
  }

  // intent §3.1 S6..S9 — well-formedness of a published tally
  pure def well_formed_tally(t: TallyResult): bool = {
    val cands = candidate_set
    and {
      // S6: winner well-formedness
      t.winners.subseteq(cands),
      t.winners.size() >= 1,
      t.winners.size() <= cands.size(),
      // S7: ballot accounting sets subset of candidates
      t.dropped_voters.subseteq(cands),
      t.non_voters.subseteq(cands),
      t.ballots_tallied >= 0,
      t.ballots_dropped >= 0,
      // S7: dropped_voters and non_voters are disjoint
      t.dropped_voters.intersect(t.non_voters) == Set(),
      // S7: partition equation — every candidate is in exactly one of
      // {tallied, dropped, non_voter}. Adopted from kimi-k2-6 and gpt-5.5
      // synthesis (2026-05-19 diff B1). Tighter than the prior bound.
      t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == cands.size(),
      // bookkeeping consistency: dropped_voters cardinality equals the
      // ballots_dropped count. Added 2026-05-19 per kimi cross-critique
      // of canonical (well_formed_tally previously allowed
      // ballots_dropped=1 with empty dropped_voters set).
      t.dropped_voters.size() == t.ballots_dropped,
      // S8: per-round count consistency.
      s8_round_counts_sum(t),
      // S9: elimination monotonicity.
      s9_no_reappearance(t),
      // round structural bound: an IRV election cannot exceed |candidates| rounds.
      t.per_round_counts.length() >= 0,
      t.per_round_counts.length() <= cands.size(),
      t.eliminated_by_round.length() <= t.per_round_counts.length(),
    }
  }

  // intent §2.5 Block E1 + §3.2 B10 chain-side projection.
  // A tally is consistent with the enclave's sealed input set if its
  // bookkeeping fields agree with what's derivable from the input set
  // alone. The IRV-specific correctness (winner from elimination) lives
  // in B10_lean, off-chain.
  //
  // SCOPE WARNING (gpt-5.5 re-critique 2026-05-19): the predicate name
  // is `is_tally_spec_output` but it ONLY checks the chain-observable
  // bookkeeping subset of Tally_spec. The IRV semantic correctness
  // (e.g., winners must be the set of candidates with strict-majority
  // first-place votes when one exists, not an arbitrary co-winner set)
  // is NOT checked here; it is the discharge obligation of B10_lean
  // (intent §3.2). A reachable state like `winners = candidate_set`
  // with `per_round_counts = [{A: 3, others: 0}]` passes this predicate
  // even though the IRV outcome should be `winners = {A}`. That is by
  // design: the chain cannot verify IRV semantics without running the
  // full algorithm, so the attestation envelope + B10_lean compose to
  // establish that.
  pure def is_tally_spec_output(t: TallyResult, sealed: Addr -> Ciphertext): bool = {
    val cands = candidate_set
    val voters_who_submitted = sealed.keys()
    and {
      well_formed_tally(t),
      t.dropped_voters.subseteq(voters_who_submitted),
      t.ballots_tallied <= voters_who_submitted.size(),
      t.non_voters == cands.exclude(voters_who_submitted),
    }
  }

  // ============================================================
  // Initial state — corresponds to intent §2.5 Block 1 (instantiate).
  // ============================================================

  action init = all {
    contract_ballots' = Map(),
    contract_tally_result' = { present: false, value: EMPTY_TALLY },
    registry' = CANONICAL_REGISTRY,
    block_time' = START_AT - 1,
    enclave_has_session_key' = true,
    enclave_consumed_ballots' = Map(),
    enclave_computed_tally' = { present: false, value: EMPTY_TALLY },
    ghost_ballots_at_end_at' = Map(),
    ghost_ballots_at_end_at_captured' = false,
  }

  // ============================================================
  // Actions
  // ============================================================

  // Time advance. Captures `ballots@end_at` the first tick that
  // crosses end_at — anchors B2.
  action tick(dt: int): bool = all {
    dt > 0,
    block_time' = block_time + dt,
    ghost_ballots_at_end_at' =
      if (ghost_ballots_at_end_at_captured) ghost_ballots_at_end_at
      else if (block_time < END_AT and (block_time + dt) >= END_AT) contract_ballots
      else ghost_ballots_at_end_at,
    ghost_ballots_at_end_at_captured' =
      if (ghost_ballots_at_end_at_captured) true
      else if (block_time < END_AT and (block_time + dt) >= END_AT) true
      else false,
    contract_ballots' = contract_ballots,
    contract_tally_result' = contract_tally_result,
    registry' = registry,
    enclave_has_session_key' = enclave_has_session_key,
    enclave_consumed_ballots' = enclave_consumed_ballots,
    enclave_computed_tally' = enclave_computed_tally,
  }

  // intent §2.5 Block 3 — submit_ballot. Requires the sender to be a
  // candidate and the phase to be Voting. The action's enablement
  // conditions encode B4 (no premature voting), B7 (voter is
  // candidate), and the contract-layer half of B6 (writer = voter,
  // since `sender` is bound to both the action parameter and the
  // ballots-map key).
  action submit_ballot(sender: Addr, ct: Ciphertext): bool = all {
    derived_phase(block_time, contract_tally_result) == Voting,
    candidate_set.contains(sender),
    ct != "",
    contract_ballots' = contract_ballots.put(sender, ct),
    contract_tally_result' = contract_tally_result,
    registry' = registry,
    block_time' = block_time,
    enclave_has_session_key' = enclave_has_session_key,
    enclave_consumed_ballots' = enclave_consumed_ballots,
    enclave_computed_tally' = enclave_computed_tally,
    ghost_ballots_at_end_at' = ghost_ballots_at_end_at,
    ghost_ballots_at_end_at_captured' = ghost_ballots_at_end_at_captured,
  }

  // intent §2.5 Block 5 — close_and_tally. Idempotent event-only;
  // no state mutation.
  action close_and_tally: bool = all {
    derived_phase(block_time, contract_tally_result) == Tallying,
    contract_ballots' = contract_ballots,
    contract_tally_result' = contract_tally_result,
    registry' = registry,
    block_time' = block_time,
    enclave_has_session_key' = enclave_has_session_key,
    enclave_consumed_ballots' = enclave_consumed_ballots,
    enclave_computed_tally' = enclave_computed_tally,
    ghost_ballots_at_end_at' = ghost_ballots_at_end_at,
    ghost_ballots_at_end_at_captured' = ghost_ballots_at_end_at_captured,
  }

  // intent §2.5 Block E1 — enclave computes Tally_spec over the
  // chain-frozen ballot set. The honest path binds
  // `consumed_ballots = ghost_ballots_at_end_at`
  // (enclave_input_fidelity from §8.7 link 7); a malicious-host
  // variant would relax this. We model the honest path only because
  // B10 is the discharge of the honest-case obligation.
  action enclave_tally(t: TallyResult): bool = all {
    derived_phase(block_time, contract_tally_result) == Tallying,
    enclave_has_session_key,
    ghost_ballots_at_end_at_captured,
    not(enclave_computed_tally.present),
    is_tally_spec_output(t, ghost_ballots_at_end_at),
    enclave_consumed_ballots' = ghost_ballots_at_end_at,
    enclave_computed_tally' = { present: true, value: t },
    contract_ballots' = contract_ballots,
    contract_tally_result' = contract_tally_result,
    registry' = registry,
    block_time' = block_time,
    enclave_has_session_key' = enclave_has_session_key,
    ghost_ballots_at_end_at' = ghost_ballots_at_end_at,
    ghost_ballots_at_end_at_captured' = ghost_ballots_at_end_at_captured,
  }

  // intent §2.5 Block 6 — publish_result. Requires Tallying phase,
  // a computed enclave tally, honest registry, well-formed tally.
  // Action-guard disablement encodes:
  //   B1, B5 (publish at most once): forbid when tally is present
  //   B3 (no premature tally):       require Tallying
  //   B8 (registry honest at fire):  require image_registration_honest
  //   well-formedness check at Block 6: well_formed_tally(t)
  action publish_result: bool = all {
    derived_phase(block_time, contract_tally_result) == Tallying,
    enclave_computed_tally.present,
    image_registration_honest(registry),
    well_formed_tally(enclave_computed_tally.value),
    not(contract_tally_result.present),
    contract_tally_result' = enclave_computed_tally,
    contract_ballots' = contract_ballots,
    registry' = registry,
    block_time' = block_time,
    enclave_has_session_key' = enclave_has_session_key,
    enclave_consumed_ballots' = enclave_consumed_ballots,
    enclave_computed_tally' = enclave_computed_tally,
    ghost_ballots_at_end_at' = ghost_ballots_at_end_at,
    ghost_ballots_at_end_at_captured' = ghost_ballots_at_end_at_captured,
  }

  // ============================================================
  // Step — nondeterministic choice over enabled actions
  // ============================================================

  action step = {
    nondet sender = oneOf(candidate_set.union(NON_CANDIDATES))
    nondet ct = oneOf(CIPHERTEXTS)
    nondet dt = oneOf(TICK_DELTAS)

    // Nondet candidate tally for `enclave_tally`. We pre-bind a
    // small set of well-formed shapes; the action's `is_tally_spec_output`
    // guard filters to those consistent with the captured ballot set.
    //
    // Shape: 0-round (no eliminations, all candidates surviving), or
    // 1-round (one candidate eliminated). Two shapes are enough to
    // exercise S8 (count conservation) and S9 (monotone elimination)
    // without combinatorial explosion in the search.
    nondet winners_pick = oneOf(Set(
      Set(CANDIDATES[0]),
      candidate_set,
    ))
    // bt_tallied covers [0..|candidates|] inclusive. The prior hardcoded
    // set {0,1,2,3,5} omitted 4, creating a state-space gap at exactly
    // 4-of-5 voters (per kimi re-critique of revised canonical, 2026-05-19).
    nondet bt_tallied = oneOf(0.to(CANDIDATES.length()).fold(Set(), (acc, n) => acc.union(Set(n))))
    val zero_counts: Addr -> int = candidate_set.fold(Map(), (acc, c) => acc.put(c, 0))
    val voter_share_counts: Addr -> int = candidate_set.fold(Map(), (acc, c) =>
      if (c == CANDIDATES[0]) acc.put(c, bt_tallied) else acc.put(c, 0))
    // 0-round tally — single round, no eliminations.
    val t_zero_round: TallyResult = {
      winners: winners_pick,
      per_round_counts: List(voter_share_counts),
      eliminated_by_round: List(),
      ballots_tallied: bt_tallied,
      ballots_dropped: 0,
      dropped_voters: Set(),
      non_voters: candidate_set.exclude(ghost_ballots_at_end_at.keys()),
    }
    // 1-round-elimination tally — eliminates last candidate at round 0,
    // round 1 has only the surviving candidates and 0 votes each.
    val survivors_counts: Addr -> int =
      candidate_set.exclude(Set(CANDIDATES[CANDIDATES.length() - 1]))
        .fold(Map(), (acc, c) => acc.put(c, 0))
    val t_one_round: TallyResult = {
      winners: Set(CANDIDATES[0]),
      per_round_counts: List(zero_counts, survivors_counts),
      eliminated_by_round: List(List(CANDIDATES[CANDIDATES.length() - 1])),
      ballots_tallied: 0,
      ballots_dropped: 0,
      dropped_voters: Set(),
      non_voters: candidate_set.exclude(ghost_ballots_at_end_at.keys()),
    }
    nondet tally_shape = oneOf(Set("zero_round", "one_round"))
    val t: TallyResult =
      if (tally_shape == "zero_round") t_zero_round
      else t_one_round

    any {
      tick(dt),
      submit_ballot(sender, ct),
      close_and_tally,
      enclave_tally(t),
      publish_result,
    }
  }

  // ============================================================
  // Invariants
  // ============================================================

  // intent §3.1 S4 + §3.2 B7: every ballot key is a candidate.
  // The `submit_ballot` action guard forbids non-candidates; this
  // invariant says no trace ever reaches a violating state.
  val inv_ballot_keys_are_candidates: bool = {
    contract_ballots.keys().subseteq(candidate_set)
  }

  // intent §3.1 S10: tally_result.is_some() ⇒ block_time ≥ END_AT.
  val inv_s10_resolution_after_end_at: bool = {
    if (contract_tally_result.present) block_time >= END_AT
    else true
  }

  // intent §3.1 S6-S9: when tally_result.is_some(), value is well-formed.
  val inv_published_well_formed: bool = {
    if (contract_tally_result.present)
      well_formed_tally(contract_tally_result.value)
    else true
  }

  // intent §3.2 B2: `ballots@end_at` is frozen at the
  // Voting→Tallying boundary. Two checks:
  //   (i) once captured, the ghost snapshot never changes
  //   (ii) the chain ballots cannot diverge from the snapshot for
  //        any key the snapshot saw, and no new keys are added
  //        post-capture (since `submit_ballot` action requires
  //        Voting phase, which ends at END_AT).
  val inv_b2_ballots_at_end_at_frozen: bool = {
    if (ghost_ballots_at_end_at_captured) {
      and {
        ghost_ballots_at_end_at.keys().forall(k =>
          contract_ballots.keys().contains(k)
            and contract_ballots.get(k) == ghost_ballots_at_end_at.get(k)),
        // No keys added past capture: every chain-ballots key was
        // present at capture.
        contract_ballots.keys().subseteq(ghost_ballots_at_end_at.keys()),
      }
    } else true
  }

  // intent §3.2 B8 (classical-Prop shadow): publish_result only fires
  // when the registry is honest. (The probabilistic content of B8/B9
  // lives outside Quint; this checks the conditional-honesty preamble
  // that B9's antecedent requires.)
  val inv_b8_publish_implies_registry_honest: bool = {
    if (contract_tally_result.present) image_registration_honest(registry)
    else true
  }

  // intent §3.2 B10 chain-side projection: the published tally is a
  // well-formed Tally_spec output over the enclave's consumed input,
  // and `enclave_input_fidelity` held (consumed == ballots@end_at)
  // with KMS-honest session key (dstack_kms_trust).
  // The other 4 links of B10 (B10_lean, image-identity-binding, B8,
  // dstack_kms_trust as a deeper assumption) are off-chain.
  val inv_b10_chain_side_projection: bool = {
    if (contract_tally_result.present)
      and {
        is_tally_spec_output(contract_tally_result.value, enclave_consumed_ballots),
        enclave_consumed_ballots == ghost_ballots_at_end_at,
        enclave_has_session_key,
      }
    else true
  }

  // intent §3.2 B6 contract-layer projection: the writer of every
  // slot equals the slot key. Encoded structurally — `submit_ballot`
  // writes `ballots[sender]` where sender is both the action arg
  // and the slot key — and verified by the model checker noting
  // that no other action mutates `contract_ballots`. The state-side
  // residue is captured by `inv_ballot_keys_are_candidates` plus
  // the well-typedness of writes.

  // Composite — checked via `quint verify --invariant=all_invariants`.
  val all_invariants: bool = all {
    inv_ballot_keys_are_candidates,
    inv_s10_resolution_after_end_at,
    inv_published_well_formed,
    inv_b2_ballots_at_end_at_frozen,
    inv_b8_publish_implies_registry_honest,
    inv_b10_chain_side_projection,
  }

  // ============================================================
  // Reachability witnesses — these should be VIOLATED on some
  // trace; check by `quint verify --invariant=witness_X` and
  // expecting a counterexample.
  // ============================================================

  // Should fail: a Resolved state must be reachable.
  val witness_resolution_reachable: bool = not(contract_tally_result.present)

  // Should fail: at least one ballot must be submittable.
  val witness_ballot_submittable: bool = contract_ballots.keys().size() == 0

  // Should fail: at some reachable state, ballots@end_at is captured
  // (i.e. the Voting→Tallying transition has been observed).
  val witness_end_at_crossing: bool = not(ghost_ballots_at_end_at_captured)

  // Diagnostic: should fail if tick ever fires.
  val witness_tick_fires: bool = block_time == START_AT - 1

  // Diagnostic: should fail if block_time ever reaches or passes END_AT.
  val witness_reaches_end_at: bool = block_time < END_AT
}
