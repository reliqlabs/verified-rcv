module rcv {
  type Addr = str

  type TallyResult = {
    winners: Set[Addr],
    ballots_tallied: int,
    ballots_dropped: int,
    non_voters: Set[Addr]
  }

  type MaybeTally = { present: bool, value: TallyResult }

  const CANDIDATES: List[Addr]
  const START_AT: int
  const END_AT: int
  const MAX_TIME: int

  pure val candidates_set: Set[Addr] =
    CANDIDATES.foldl(Set(), (s, c) => s.union(Set(c)))

  pure def list_length(l: List[Addr]): int =
    l.foldl(0, (acc, x) => acc + 1)

  pure def list_all(l: List[Addr], pred: Addr => bool): bool =
    l.foldl(true, (acc, x) => acc and pred(x))

  pure def first_vote(ballot: List[Addr]): Addr =
    ballot.foldl("", (acc, c) => if (acc == "") c else acc)

  pure def count_votes_for(voter_set: Set[Addr], ballots_map: Map[Addr, List[Addr]], candidate: Addr): int =
    voter_set.filter(v => first_vote(ballots_map.get(v)) == candidate).size()

  pure def tally_spec(ballots_map: Map[Addr, List[Addr]]): TallyResult = {
    val voters = ballots_map.keys()
    val non_voters = candidates_set.exclude(voters)
    val counts = CANDIDATES.foldl(Map(), (m, c) => m.set(c, count_votes_for(voters, ballots_map, c)))
    val max_votes = CANDIDATES.foldl(0, (acc, c) => {
      val v = counts.get(c)
      if (v > acc) v else acc
    })
    val winners = CANDIDATES.foldl(Set(), (s, c) => {
      val v = counts.get(c)
      if (v == max_votes) s.union(Set(c)) else s
    })
    {
      winners: winners,
      ballots_tallied: voters.size(),
      ballots_dropped: 0,
      non_voters: non_voters
    }
  }

  var ballots: Map[Addr, List[Addr]]
  var tally_result: MaybeTally
  var time: int
  var was_resolved: bool
  var end_crossed: bool
  var ballots_at_end: Map[Addr, List[Addr]]

  val s1 = list_length(CANDIDATES) >= 1
  val s2 = candidates_set.size() == list_length(CANDIDATES)
  val s3 = START_AT < END_AT
  val s4 = ballots.keys().exclude(candidates_set).size() == 0
  val s6 = not(tally_result.present) or (tally_result.value.winners.exclude(candidates_set).size() == 0 and tally_result.value.winners.size() >= 1 and tally_result.value.winners.size() <= candidates_set.size())
  val s7 = not(tally_result.present) or (tally_result.value.ballots_tallied + tally_result.value.ballots_dropped + tally_result.value.non_voters.size() == candidates_set.size())
  val s10 = not(tally_result.present) or (time >= END_AT)

  val b1 = not(was_resolved) or tally_result.present
  val b2 = not(end_crossed) or (ballots == ballots_at_end)
  val b3 = not(was_resolved) or end_crossed
  val b10 = not(tally_result.present) or (tally_result.value == tally_spec(ballots))

  val all_invariants = s1 and s2 and s3 and s4 and s6 and s7 and s10 and b1 and b2 and b3 and b10

  val witness_resolution_reachable = not(was_resolved)
  val witness_ballot_submittable = ballots.keys().size() == 0
  val witness_end_at_crossing = not(end_crossed)

  action init = all {
    ballots' = Map(),
    tally_result' = { present: false, value: { winners: Set(), ballots_tallied: 0, ballots_dropped: 0, non_voters: candidates_set } },
    time' = 0,
    was_resolved' = false,
    end_crossed' = (0 >= END_AT),
    ballots_at_end' = Map()
  }

  action tick = all {
    time < MAX_TIME,
    time' = time + 1,
    end_crossed' = if (not(end_crossed) and (time + 1 >= END_AT) and (time < END_AT)) true else end_crossed,
    ballots_at_end' = if (not(end_crossed) and (time + 1 >= END_AT) and (time < END_AT)) ballots else ballots_at_end,
    ballots' = ballots,
    tally_result' = tally_result,
    was_resolved' = was_resolved
  }

  action submit_ballot(voter: Addr, ballot: List[Addr]) = all {
    START_AT <= time,
    time < END_AT,
    voter.in(candidates_set),
    list_length(ballot) > 0,
    list_all(ballot, c => c.in(candidates_set)),
    ballots' = ballots.set(voter, ballot),
    tally_result' = tally_result,
    time' = time,
    was_resolved' = was_resolved,
    end_crossed' = end_crossed,
    ballots_at_end' = ballots_at_end
  }

  action publish_result = all {
    time >= END_AT,
    not(was_resolved),
    val expected = tally_spec(ballots)
    all {
      expected.winners.exclude(candidates_set).size() == 0,
      expected.winners.size() >= 1,
      expected.ballots_tallied + expected.ballots_dropped + expected.non_voters.size() == candidates_set.size(),
      tally_result' = { present: true, value: expected },
      was_resolved' = true,
      ballots' = ballots,
      time' = time,
      end_crossed' = end_crossed,
      ballots_at_end' = ballots_at_end
    }
  }

  action step = any {
    tick,
    nondet voter = oneOf(candidates_set)
    all {
      submit_ballot(voter, CANDIDATES)
    },
    publish_result
  }
}