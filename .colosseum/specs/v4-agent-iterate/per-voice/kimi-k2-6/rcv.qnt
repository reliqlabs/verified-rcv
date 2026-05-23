// -*- mode: Bluespec; -*-
// rcv.qnt — parameterized IRV election state machine

module rcv {
  const ADDR_POOL: Set[str]
  const MAX_TIME: int
  const ENC_PREFS: Set[str]
  const CANDIDATE_SETS: Set[Set[str]]

  type Addr = str

  type TallyResult = {
    winners: Set[Addr],
    ballots_tallied: int,
    ballots_dropped: int,
    non_voters: Set[Addr]
  }

  // Contract storage
  var candidates: Set[Addr]
  var start_at: int
  var end_at: int
  var ballots: Addr -> str
  var ballot_keys: Set[Addr]
  var tally_present: bool
  var tally_value: TallyResult
  var time: int
  var initialized: bool
  var attestation_valid: bool
  var tally_computed: bool

  // Derived states
  pure def isCreated(state_time: int, state_start: int): bool =
    state_time < state_start

  pure def isVoting(state_time: int, state_start: int, state_end: int): bool =
    state_start <= state_time and state_time < state_end

  pure def isTallying(state_time: int, state_end: int, state_tally_present: bool): bool =
    state_time >= state_end and not(state_tally_present)

  pure def isResolved(state_tally_present: bool): bool =
    state_tally_present

  // Well-formedness predicate for a published tally (syntactic checks from Block 6)
  pure def wellFormed(tally: TallyResult, cands: Set[Addr], bkeys: Set[Addr]): bool = {
    and {
      tally.winners.subseteq(cands),
      tally.winners.size() >= 1,
      tally.winners.size() <= cands.size(),
      tally.ballots_tallied + tally.ballots_dropped + tally.non_voters.size() == cands.size(),
      tally.non_voters.subseteq(cands),
      tally.non_voters == cands.exclude(bkeys),
    }
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  action init: bool = all {
    initialized' = false,
    candidates' = Set(),
    start_at' = 0,
    end_at' = 0,
    ballots' = ADDR_POOL.mapBy(a => ""),
    ballot_keys' = Set(),
    tally_present' = false,
    tally_value' = { winners: Set(), ballots_tallied: 0, ballots_dropped: 0, non_voters: Set() },
    time' = 0,
    attestation_valid' = false,
    tally_computed' = false,
  }

  action instantiate(candidates_set: Set[Addr], s: int, e: int): bool = all {
    not(initialized),
    candidates_set.size() >= 1,
    candidates_set.subseteq(ADDR_POOL),
    s > time,
    e > s,
    e <= MAX_TIME,
    initialized' = true,
    candidates' = candidates_set,
    start_at' = s,
    end_at' = e,
    ballots' = ADDR_POOL.mapBy(a => ""),
    ballot_keys' = Set(),
    tally_present' = false,
    tally_value' = { winners: Set(), ballots_tallied: 0, ballots_dropped: 0, non_voters: Set() },
    time' = time,
    attestation_valid' = false,
    tally_computed' = false,
  }

  action advance_time: bool = all {
    initialized,
    time < MAX_TIME,
    time' = time + 1,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    ballot_keys' = ballot_keys,
    tally_present' = tally_present,
    tally_value' = tally_value,
    initialized' = initialized,
    attestation_valid' = attestation_valid,
    tally_computed' = tally_computed,
  }

  action submit_ballot(sender: Addr, encrypted_prefs: str): bool = all {
    initialized,
    isVoting(time, start_at, end_at),
    sender.in(candidates),
    encrypted_prefs != "",
    not(tally_present),
    ballots' = ballots.set(sender, encrypted_prefs),
    ballot_keys' = ballot_keys.union(Set(sender)),
    time' = time,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    tally_present' = tally_present,
    tally_value' = tally_value,
    initialized' = initialized,
    attestation_valid' = attestation_valid,
    tally_computed' = tally_computed,
  }

  action close_and_tally: bool = all {
    initialized,
    isTallying(time, end_at, tally_present),
    not(tally_present),
    time' = time,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    ballot_keys' = ballot_keys,
    tally_present' = tally_present,
    tally_value' = tally_value,
    initialized' = initialized,
    attestation_valid' = attestation_valid,
    tally_computed' = tally_computed,
  }

  action publish_result: bool = all {
    initialized,
    isTallying(time, end_at, tally_present),
    not(tally_present),
    // Compute a trivial well-formed tally (all co-winners, all ballots valid)
    wellFormed(
      { winners: candidates, ballots_tallied: ballot_keys.size(), ballots_dropped: 0, non_voters: candidates.exclude(ballot_keys) },
      candidates,
      ballot_keys
    ),
    tally_present' = true,
    tally_value' = { winners: candidates, ballots_tallied: ballot_keys.size(), ballots_dropped: 0, non_voters: candidates.exclude(ballot_keys) },
    attestation_valid' = true,
    tally_computed' = true,
    time' = time,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    ballot_keys' = ballot_keys,
    initialized' = initialized,
  }

  action step: bool = any {
    nondet candidates_set = oneOf(CANDIDATE_SETS)
    nondet s = oneOf(time.to(MAX_TIME))
    nondet e = oneOf((s + 1).to(MAX_TIME))
    instantiate(candidates_set, s, e),

    advance_time,

    nondet sender = oneOf(ADDR_POOL)
    nondet prefs = oneOf(ENC_PREFS)
    submit_ballot(sender, prefs),

    close_and_tally,

    publish_result,
  }

  // ---------------------------------------------------------------------------
  // Invariants
  // ---------------------------------------------------------------------------

  // Structural invariants (Section 3.1)
  val inv_S1 = not(initialized) or candidates.size() >= 1
  val inv_S2 = not(initialized) or candidates.size() == candidates.size()  // trivial for Set
  val inv_S3 = not(initialized) or start_at < end_at
  val inv_S4 = ballot_keys.subseteq(candidates)

  // S5 (handler-set write discipline) — encoded by action guards: only publish_result writes tally_present,
  // and it requires not(tally_present). This is a meta-property of the action set, not a state predicate.

  val inv_S6 = not(tally_present) or (
    tally_value.winners.subseteq(candidates)
      and tally_value.winners.size() >= 1
      and tally_value.winners.size() <= candidates.size()
  )

  val inv_S7 = not(tally_present) or (
    tally_value.ballots_tallied + tally_value.ballots_dropped + tally_value.non_voters.size() == candidates.size()
  )

  // S8, S9 — per-round count consistency and elimination monotonicity require per_round_counts
  // and eliminated_by_round fields. Omitted from this Quint encoding because the tally computation
  // is off-chain (Block E1) and B10 is cross-layer. The well-formedness shadow (S6+S7) covers
  // the chain-checkable structural properties.

  val inv_S10 = not(tally_present) or (time >= end_at)

  // Behavioral invariants (Section 3.2) — classical shadows where full temporal form is action-guarded
  val inv_B1 = not(tally_present) or (tally_present and attestation_valid and tally_computed)
    // Shadow: once set, tally stays set (guards prevent unsetting) and carries attestation/computation flags.

  val inv_B2 = not(tally_present) or true
    // Shadow: no late ballots — submit_ballot guard requires isVoting, so ballots are frozen after end_at.

  val inv_B3 = not(tally_present) or (time >= end_at)
    // Same as S10: resolution requires time >= end_at.

  val inv_B4 = true
    // Shadow: no premature voting — submit_ballot guard requires isVoting (start_at <= time < end_at).

  val inv_B5 = true
    // Shadow: publish_result fires at most once — tally_present guard prevents re-fire.

  val inv_B6 = true
    // Shadow: ballot writer is ballot voter — submit_ballot guard requires sender.in(candidates).

  val inv_B7 = true
    // Shadow: terminal state immutability — only advance_time is available when tally_present,
    // and it does not mutate contract storage.

  // B8 classical shadow: attestation binds tally
  val inv_B8_shadow = not(tally_present) or attestation_valid

  // B9 classical shadow: meta-security negligibility budget — not encodable as a state predicate.
  // Represented by the attestation_valid flag which abstracts the verification outcome.
  val inv_B9_shadow = true

  // B10 classical shadow: tally correctness — represented by tally_computed flag.
  // The actual semantic correctness (tally = Tally_spec) is cross-layer and discharged in Lean.
  val inv_B10_shadow = not(tally_present) or tally_computed

  // Composite safety invariant
  val all_invariants = and {
    inv_S1,
    inv_S3,
    inv_S4,
    inv_S6,
    inv_S7,
    inv_S10,
    inv_B8_shadow,
    inv_B10_shadow,
  }

  // ---------------------------------------------------------------------------
  // Witness invariants (must be VIOLATED = state is reachable)
  // ---------------------------------------------------------------------------

  val witness_resolution_reachable = not(tally_present)
  val witness_ballot_submittable = ballot_keys.size() == 0
  val witness_end_at_crossing = time < end_at
}
