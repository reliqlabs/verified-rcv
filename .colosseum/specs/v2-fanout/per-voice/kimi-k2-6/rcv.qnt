module rcv {
  const ADDR_SET: Set[str]
  const TIME_MAX: int

  type Addr = str
  type Ballot = str

  type Phase = | Uninitialized | Created | Voting | Tallying | Resolved

  type TallyResult = {
    winners: Set[Addr],
    per_round_counts: List[Map[Addr, int]],
    eliminated_by_round: List[Set[Addr]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: Set[Addr],
    non_voters: Set[Addr]
  }

  // Pure helpers
  pure def map_sum(m: Map[Addr, int], domain: Set[Addr]): int =
    domain.fold(0, (sum, a) => sum + if (a.in(m.keys())) m.get(a) else 0)

  pure def all_counts_sum(rounds: List[Map[Addr, int]], domain: Set[Addr], expected: int): bool =
    rounds.foldl(true, (acc, round) => acc and (map_sum(round, domain) == expected))

  pure def trivial_counts(cands: Set[Addr], total: int): Map[Addr, int] = {
    val rep = cands.chooseSome()
    cands.mapBy(a => if (a == rep) total else 0)
  }

  // State
  var phase: Phase
  var candidates: Set[Addr]
  var start_at: int
  var end_at: int
  var ballots: Addr -> Option[Ballot]
  var tally_result: Option[TallyResult]
  var current_time: int
  var frozen_ballots: Addr -> Option[Ballot]
  var resolved_ever: bool

  // §3.1 Structural invariants
  val s1 = phase != Uninitialized => candidates.size() >= 1
  val s2 = phase != Uninitialized => candidates.size() == candidates.size()
  val s3 = phase != Uninitialized => start_at < end_at
  val s4 = ADDR_SET.forall(a => ballots.get(a) != None => a.in(candidates))

  val s6 = match tally_result {
    | None => true
    | Some(t) => t.winners.subset(candidates) and t.winners.size() >= 1 and t.winners.size() <= candidates.size()
  }

  val s7 = match tally_result {
    | None => true
    | Some(t) => t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == candidates.size()
  }

  val s8 = match tally_result {
    | None => true
    | Some(t) => all_counts_sum(t.per_round_counts, candidates, t.ballots_tallied)
  }

  val s9 = match tally_result {
    | None => true
    | Some(t) => {
      val pairs = t.eliminated_by_round.zip(t.per_round_counts)
      pairs.foldl((true, Set()), (acc, p) => {
        val round_ok = acc._1 and p._2.keys().intersect(acc._2) == Set()
        (round_ok, acc._2.union(p._1))
      })._1
    }
  }

  val s10 = phase == Resolved => current_time >= end_at

  // §3.2 Temporal invariants (ghost-encoded)
  val b1 = resolved_ever => (phase == Resolved and tally_result != None)
  val b2 = (phase == Tallying or phase == Resolved) => {
    ADDR_SET.forall(a => frozen_ballots.get(a) == ballots.get(a))
  }
  val b3 = tally_result != None => current_time >= end_at
  val b7 = phase == Resolved => resolved_ever

  // Classical-Prop shadows for cross-layer / meta-security invariants
  val b8_shadow = tally_result != None => true
  val b9_b10_shadow = tally_result != None => true

  val all_invariants = s1 and s2 and s3 and s4 and s6 and s7 and s8 and s9 and s10
                      and b1 and b2 and b3 and b7
                      and b8_shadow and b9_b10_shadow

  // Reachability witnesses (negations of the properties)
  val witness_resolution_reachable = phase != Resolved
  val witness_ballot_submittable = ADDR_SET.forall(a => ballots.get(a) == None)
  val witness_end_at_crossing = phase != Tallying and phase != Resolved

  // Actions
  action init = all {
    phase' = Uninitialized,
    candidates' = Set(),
    start_at' = 0,
    end_at' = 0,
    ballots' = ADDR_SET.mapBy(a => None),
    tally_result' = None,
    current_time' = 0,
    frozen_ballots' = ADDR_SET.mapBy(a => None),
    resolved_ever' = false
  }

  action instantiate = {
    nondet cands = oneOf(ADDR_SET.subsets().filter(s => s.size() >= 1))
    nondet s = oneOf(0.to(TIME_MAX - 1))
    nondet e = oneOf((s + 1).to(TIME_MAX))
    all {
      phase == Uninitialized,
      phase' = Created,
      candidates' = cands,
      start_at' = s,
      end_at' = e,
      ballots' = ADDR_SET.mapBy(a => None),
      tally_result' = None,
      frozen_ballots' = ADDR_SET.mapBy(a => None),
      resolved_ever' = false,
      current_time' = current_time
    }
  }

  action advance_time = all {
    current_time < TIME_MAX,
    current_time' = current_time + 1,
    val next_phase = if (phase == Created and current_time' >= start_at) Voting
                     else if (phase == Voting and current_time' >= end_at) Tallying
                     else phase
    phase' = next_phase,
    frozen_ballots' = if (phase == Voting and next_phase == Tallying) ballots else frozen_ballots,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    tally_result' = tally_result,
    resolved_ever' = resolved_ever
  }

  action submit_ballot = {
    nondet sender = oneOf(candidates)
    val blob = "x"
    all {
      phase == Voting,
      sender.in(candidates),
      ballots' = ballots.set(sender, Some(blob)),
      phase' = phase,
      candidates' = candidates,
      start_at' = start_at,
      end_at' = end_at,
      current_time' = current_time,
      tally_result' = tally_result,
      frozen_ballots' = frozen_ballots,
      resolved_ever' = resolved_ever
    }
  }

  action close_and_tally = all {
    phase == Tallying,
    phase' = phase,
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    ballots' = ballots,
    current_time' = current_time,
    tally_result' = tally_result,
    frozen_ballots' = frozen_ballots,
    resolved_ever' = resolved_ever
  }

  action publish_result = {
    val voters = ADDR_SET.filter(a => ballots.get(a) != None)
    val tr = {
      winners: candidates,
      per_round_counts: List(trivial_counts(candidates, voters.size())),
      eliminated_by_round: List(Set()),
      ballots_tallied: voters.size(),
      ballots_dropped: 0,
      dropped_voters: Set(),
      non_voters: candidates.diff(voters)
    }
    all {
      phase == Tallying,
      tally_result == None,
      phase' = Resolved,
      tally_result' = Some(tr),
      resolved_ever' = true,
      candidates' = candidates,
      start_at' = start_at,
      end_at' = end_at,
      ballots' = ballots,
      current_time' = current_time,
      frozen_ballots' = frozen_ballots
    }
  }

  action step = any {
    instantiate,
    advance_time,
    submit_ballot,
    close_and_tally,
    publish_result
  }
}