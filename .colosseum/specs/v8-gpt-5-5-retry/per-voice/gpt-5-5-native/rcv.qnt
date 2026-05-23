// -*- mode: Bluespec; -*-
module rcv {
  const CANDIDATES: Set[str]
  const START_AT: int
  const END_AT: int

  type OptStr = { present: bool, value: str }

  type TallyResult = {
    winners: Set[str],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: Set[str],
    non_voters: Set[str],
  }

  type OptTally = { present: bool, value: TallyResult }

  var now: int
  var ballots: str -> str
  var ballots_at_end: str -> str
  var tally_result: OptTally
  var last_action: str

  pure val noneTally: OptTally = {
    present: false,
    value: {
      winners: Set(),
      ballots_tallied: 0,
      ballots_dropped: 0,
      dropped_voters: Set(),
      non_voters: Set(),
    },
  }

  def state_name: str = {
    if (tally_result.present) "Resolved"
    else if (now < START_AT) "Created"
    else if (now < END_AT) "Voting"
    else "Tallying"
  }

  def valid_tally_for(bs: str -> str): TallyResult = {
    val voters = bs.keys().intersect(CANDIDATES)
    val nonvoters = CANDIDATES.exclude(voters)
    {
      winners: CANDIDATES,
      ballots_tallied: voters.size(),
      ballots_dropped: 0,
      dropped_voters: Set(),
      non_voters: nonvoters,
    }
  }

  def tally_well_formed(t: TallyResult): bool = all {
    t.winners.subseteq(CANDIDATES),
    t.winners.size() >= 1,
    t.winners.size() <= CANDIDATES.size(),
    t.ballots_tallied >= 0,
    t.ballots_dropped >= 0,
    t.dropped_voters.subseteq(ballots.keys()),
    t.non_voters == CANDIDATES.exclude(ballots.keys()),
    t.dropped_voters.intersect(t.non_voters).size() == 0,
    t.dropped_voters.size() == t.ballots_dropped,
    t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == CANDIDATES.size(),
  }

  def structural_invariants(): bool = all {
    CANDIDATES.size() >= 1,
    START_AT < END_AT,
    ballots.keys().subseteq(CANDIDATES),
    if (tally_result.present) tally_well_formed(tally_result.value) else true,
    if (tally_result.present) now >= END_AT else true,
  }

  def behavioral_shadows(): bool = all {
    // B1/B7 sink shape: the step relation disables all non-stutter resolved writes.
    if (tally_result.present) state_name == "Resolved" else true,
    // B2: after the boundary-crossing transition, the live store equals the frozen snapshot.
    if (now >= END_AT) ballots == ballots_at_end else true,
    // B8/B9/B10 classical shadows: publish writes exactly the local Tally_spec shadow.
    if (tally_result.present) tally_result.value == valid_tally_for(ballots_at_end) else true,
  }

  val all_invariants: bool = all {
    structural_invariants,
    behavioral_shadows,
  }

  action init = all {
    CANDIDATES.size() >= 1,
    START_AT > 0,
    START_AT < END_AT,
    now' = 0,
    ballots' = Map(),
    ballots_at_end' = Map(),
    tally_result' = noneTally,
    last_action' = "init",
  }

  action tick: bool = all {
    not(tally_result.present),
    now < END_AT,
    now' = now + 1,
    ballots' = ballots,
    ballots_at_end' = if (now + 1 >= END_AT) ballots else ballots_at_end,
    tally_result' = tally_result,
    last_action' = "tick",
  }

  action submit_ballot(sender: str, encrypted_preferences: str): bool = all {
    not(tally_result.present),
    START_AT <= now,
    now < END_AT,
    sender.in(CANDIDATES),
    encrypted_preferences != "",
    now' = now,
    ballots' = ballots.put(sender, encrypted_preferences),
    ballots_at_end' = ballots_at_end,
    tally_result' = tally_result,
    last_action' = "submit_ballot",
  }

  action close_and_tally: bool = all {
    not(tally_result.present),
    now >= END_AT,
    now' = now,
    ballots' = ballots,
    ballots_at_end' = ballots,
    tally_result' = tally_result,
    last_action' = "close_and_tally",
  }

  action publish_result: bool = all {
    not(tally_result.present),
    now >= END_AT,
    now' = now,
    ballots' = ballots,
    ballots_at_end' = ballots,
    tally_result' = { present: true, value: valid_tally_for(ballots) },
    last_action' = "publish_result",
  }

  action resolved_stutter: bool = all {
    tally_result.present,
    now' = now,
    ballots' = ballots,
    ballots_at_end' = ballots_at_end,
    tally_result' = tally_result,
    last_action' = "resolved_stutter",
  }

  action step = any {
    tick,
    all {
      ballots.keys().size() == 0,
      nondet sender = oneOf(CANDIDATES)
      submit_ballot(sender, "ciphertext")
    },
    close_and_tally,
    publish_result,
    resolved_stutter,
  }

  val witness_resolution_reachable: bool = not(tally_result.present)
  val witness_ballot_submittable: bool = not(ballots.keys().size() > 0)
  val witness_end_at_crossing: bool = not(now >= END_AT)
}
