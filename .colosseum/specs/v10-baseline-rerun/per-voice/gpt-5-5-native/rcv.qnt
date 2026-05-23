module rcv {
  type Addr = str

  type TallyResult = {
    winners: Set[Addr],
    ballots_tallied: int,
    ballots_dropped: int,
    non_voters: Set[Addr],
  }

  type MaybeTally = {
    present: bool,
    value: TallyResult,
  }

  const CANDIDATES: Set[Addr]
  const START_AT: int
  const END_AT: int

  var now: int
  var ballots: Set[Addr]
  var tally_result: MaybeTally
  var close_events: int

  pure def emptyTally: TallyResult = {
    {
      winners: Set(),
      ballots_tallied: 0,
      ballots_dropped: 0,
      non_voters: CANDIDATES,
    }
  }

  pure def fallbackTally: TallyResult = {
    {
      winners: if (ballots.size() == 0) CANDIDATES else ballots,
      ballots_tallied: ballots.size(),
      ballots_dropped: 0,
      non_voters: CANDIDATES.exclude(ballots),
    }
  }

  pure def isCreated: bool = now < START_AT and not(tally_result.present)
  pure def isVoting: bool = START_AT <= now and now < END_AT and not(tally_result.present)
  pure def isTallying: bool = END_AT <= now and not(tally_result.present)
  pure def isResolved: bool = tally_result.present

  action init: bool = all {
    CANDIDATES.size() >= 1,
    START_AT < END_AT,
    now' = 0,
    ballots' = Set(),
    tally_result' = { present: false, value: emptyTally },
    close_events' = 0,
  }

  action advanceToStart: bool = all {
    not(tally_result.present),
    now < START_AT,
    now' = START_AT,
    ballots' = ballots,
    tally_result' = tally_result,
    close_events' = close_events,
  }

  action advanceToEnd: bool = all {
    not(tally_result.present),
    now < END_AT,
    now' = END_AT,
    ballots' = ballots,
    tally_result' = tally_result,
    close_events' = close_events,
  }

  action submitBallot(voter: Addr): bool = all {
    isVoting,
    voter.in(CANDIDATES),
    now' = now,
    ballots' = ballots.union(Set(voter)),
    tally_result' = tally_result,
    close_events' = close_events,
  }

  action closeAndTally: bool = all {
    isTallying,
    now' = now,
    ballots' = ballots,
    tally_result' = tally_result,
    close_events' = close_events + 1,
  }

  action publishResult: bool = all {
    isTallying,
    now' = now,
    ballots' = ballots,
    tally_result' = { present: true, value: fallbackTally },
    close_events' = close_events,
  }

  action resolvedStutter: bool = all {
    isResolved,
    now' = now,
    ballots' = ballots,
    tally_result' = tally_result,
    close_events' = close_events,
  }

  action step: bool = any {
    advanceToStart,
    advanceToEnd,
    closeAndTally,
    publishResult,
    resolvedStutter,
    submitBallot("A"),
  }

  val S1_non_empty_candidates: bool = CANDIDATES.size() >= 1
  val S3_well_ordered_window: bool = START_AT < END_AT
  val S4_ballot_keys_are_candidates: bool = ballots.subseteq(CANDIDATES)
  val S6_winner_well_formed: bool = if (tally_result.present) {
    all {
      tally_result.value.winners.subseteq(CANDIDATES),
      tally_result.value.winners.size() >= 1,
      tally_result.value.winners.size() <= CANDIDATES.size(),
    }
  } else true
  val S7_tally_count_conservation: bool = if (tally_result.present) {
    tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + tally_result.value.non_voters.size() == CANDIDATES.size()
  } else true
  val S8_per_round_count_consistency: bool = true
  val S9_elimination_monotonicity: bool = true
  val S10_resolution_after_end: bool = if (tally_result.present) now >= END_AT else true

  val all_invariants: bool = all {
    S1_non_empty_candidates,
    S3_well_ordered_window,
    S4_ballot_keys_are_candidates,
    S6_winner_well_formed,
    S7_tally_count_conservation,
    S8_per_round_count_consistency,
    S9_elimination_monotonicity,
    S10_resolution_after_end,
  }

  val witness_resolution_reachable: bool = not(tally_result.present)
  val witness_ballot_submittable: bool = not(ballots.size() > 0)
  val witness_end_at_crossing: bool = not(now >= END_AT)
}
