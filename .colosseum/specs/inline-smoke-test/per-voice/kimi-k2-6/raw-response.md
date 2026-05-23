===BEGIN FILE: rcv.qnt===
// -*- mode: Bluespec; -*-
module rcv {
  // Types
  type TallyResult = {
    winners: Set[str],
    per_round_counts: List[Map[str, int]],
    eliminated_by_round: List[Set[str]],
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: Set[str],
    non_voters: Set[str],
  }

  // Parameters
  const ADDR: Set[str]
  const CANDIDATE_LIST: List[str]
  const START_AT: int
  const END_AT: int
  const REGISTRY_HONEST: bool

  // State variables
  var block_time: int
  var initialized: bool
  var ballots: str -> List[str]
  var tally_result: Option[TallyResult]

  // Ghost / history variables for temporal shadows
  var prev_ballots: str -> List[str]
  var prev_tally: Option[TallyResult]
  var prev_block_time: int
  var last_action: str
  var last_sender: str

  // Pure helpers
  pure def listToSet(lst: List[str]): Set[str] = {
    lst.foldl(Set(), (acc, x) => acc.union(Set(x)))
  }

  pure def candidateSet(): Set[str] = listToSet(CANDIDATE_LIST)

  pure def isPermutation(p: List[str], candidates: List[str]): bool = {
    p.length == candidates.length &&
    listToSet(p) == listToSet(candidates)
  }

  pure def firstSurvivor(prefs: List[str], remaining: Set[str>): str = {
    prefs.filter(x => x.in(remaining)).get(0)
  }

  pure def countFirstPreferences(valid: Map[str, List[str]], remaining: Set[str>): Map[str, int] = {
    remaining.mapBy(c => valid.keys().filter(v => firstSurvivor(valid.get(v), remaining) == c).size())
  }

  pure def snoc(l: List[t], e: t): List[t] = l.concat(List(e))

  pure def irv_rec(valid: Map[str, List[str>>, remaining: Set[str>, counts_acc: List[Map[str, int>>, elim_acc: List[Set[str>>): (List[Map[str, int>>, List[Set[str>>, Set[str>) = {
    val counts = countFirstPreferences(valid, remaining)
    val total = counts.values().sum()
    val maxCount = counts.values().foldl(0, (acc, v) => if (v > acc) v else acc)
    val minCount = counts.values().foldl(maxCount, (acc, v) => if (v < acc) v else acc)
    if (remaining.size() == 1) {
      (snoc(counts_acc, counts), elim_acc, remaining)
    } else if (maxCount > total / 2) {
      val winner = remaining.filter(c => counts.get(c) > total / 2)
      (snoc(counts_acc, counts), elim_acc, winner)
    } else if (minCount == maxCount) {
      (snoc(counts_acc, counts), elim_acc, remaining)
    } else {
      val losers = remaining.filter(c => counts.get(c) == minCount)
      irv_rec(valid, remaining.exclude(losers), snoc(counts_acc, counts), snoc(elim_acc, losers))
    }
  }

  pure def irv_spec(valid: Map[str, List[str>>, candidates: Set[str>): (List[Map[str, int>>, List[Set[str>>, Set[str>) = {
    irv_rec(valid, candidates, List(), List())
  }

  pure def partitionBallots(ballots: str -> List[str>, candidates: List[str>): (Map[str, List[str>>, Set[str>, Set[str>) = {
    candidates.foldl((Map(), Set(), Set()), (acc, c) => {
      val (valid, dropped, non) = acc
      if (not(c.in(ballots.keys()))) {
        (valid, dropped, non.union(Set(c)))
      } else {
        val p = ballots.get(c)
        if (isPermutation(p, candidates)) {
          (valid.put(c, p), dropped, non)
        } else {
          (valid, dropped.union(Set(c)), non)
        }
      }
    })
  }

  pure def tally_spec(ballots: str -> List[str>, candidates: List[str>): TallyResult = {
    val cset = candidateSet()
    val (valid, dropped, non) = partitionBallots(ballots, candidates)
    val (counts, elim, winners) = irv_spec(valid, cset)
    {
      winners: winners,
      per_round_counts: counts,
      eliminated_by_round: elim,
      ballots_tallied: valid.keys().size(),
      ballots_dropped: dropped.size(),
      dropped_voters: dropped,
      non_voters: non,
    }
  }

  pure def ballotChanged(prev: str -> List[str>, cur: str -> List[str>, k: str): bool = {
    if (not(k.in(prev.keys())) && not(k.in(cur.keys()))) false
    else if (k.in(prev.keys()) && not(k.in(cur.keys()))) true
    else if (not(k.in(prev.keys())) && k.in(cur.keys())) true
    else prev.get(k) != cur.get(k)
  }

  // Save history (factored into all mutating actions)
  action saveHistory = all {
    prev_ballots' = ballots,
    prev_tally' = tally_result,
    prev_block_time' = block_time,
  }

  // Actions
  action init = all {
    block_time' = 0,
    initialized' = false,
    ballots' = Map(),
    tally_result' = None,
    prev_ballots' = Map(),
    prev_tally' = None,
    prev_block_time' = 0,
    last_action' = "none",
    last_sender' = "none",
  }

  action instantiate = all {
    not(initialized),
    block_time < START_AT,
    initialized' = true,
    block_time' = block_time,
    ballots' = ballots,
    tally_result' = tally_result,
    saveHistory,
    last_action' = "Instantiate",
    last_sender' = "",
  }

  action tick = all {
    block_time' = block_time + 1,
    initialized' = initialized,
    ballots' = ballots,
    tally_result' = tally_result,
    saveHistory,
    last_action' = "Tick",
    last_sender' = "",
  }

  action submit_ballot(sender: str, prefs: List[str>) = all {
    initialized,
    block_time >= START_AT,
    block_time < END_AT,
    sender.in(candidateSet()),
    prefs.length() > 0,
    ballots' = ballots.put(sender, prefs),
    block_time' = block_time,
    initialized' = initialized,
    tally_result' = tally_result,
    saveHistory,
    last_action' = "SubmitBallot",
    last_sender' = sender,
  }

  action close_and_tally = all {
    initialized,
    block_time >= END_AT,
    tally_result == None,
    // no storage change
    block_time' = block_time,
    initialized' = initialized,
    ballots' = ballots,
    tally_result' = tally_result,
    saveHistory,
    last_action' = "CloseAndTally",
    last_sender' = "",
  }

  action publish_result = all {
    initialized,
    block_time >= END_AT,
    tally_result == None,
    REGISTRY_HONEST,
    tally_result' = Some(tally_spec(ballots, CANDIDATE_LIST)),
    block_time' = block_time,
    initialized' = initialized,
    ballots' = ballots,
    saveHistory,
    last_action' = "PublishResult",
    last_sender' = "",
  }

  action step = any {
    instantiate,
    tick,
    close_and_tally,
    publish_result,
    nondet sender = oneOf(ADDR)
    nondet prefs = oneOf(Set(CANDIDATE_LIST, CANDIDATE_LIST)) // dummy, will override in main
    submit_ballot(sender, CANDIDATE_LIST), // fallback; overridden below? Actually need concrete prefs.
  }

  // Invariants
  val s1: bool = CANDIDATE_LIST.length() > 0
  val s2: bool = candidateSet().size() == CANDIDATE_LIST.length()
  val s3: bool = START_AT < END_AT

  val s4: bool = if (initialized)
    ballots.keys().forall(k => k.in(candidateSet()))
  else true

  val s6: bool = if (tally_result.isSome()) {
    val tr = tally_result.get()
    tr.winners.size() > 0 && tr.winners.subset(candidateSet())
  } else true

  val s7: bool = if (tally_result.isSome()) {
    val tr = tally_result.get()
    tr.ballots_tallied + tr.ballots_dropped + tr.non_voters.size() == CANDIDATE_LIST.length()
  } else true

  val s8: bool = if (tally_result.isSome()) {
    val tr = tally_result.get()
    tr.per_round_counts.forall(m => m.values().sum() == tr.ballots_tallied)
  } else true

  val s9: bool = if (tally_result.isSome()) {
    val tr = tally_result.get()
    tr.eliminated_by_round.indices().forall(i => {
      val elim_i = tr.eliminated_by_round.get(i)
      tr.eliminated_by_round.indices().forall(j => {
        if (j > i) {
          elim_i.forall(c => not(c.in(tr.per_round_counts.get(j).keys())))
        } else true
      })
    })
  } else true

  val s10: bool = if (tally_result.isSome()) block_time >= END_AT else true

  // Temporal shadows using history variables
  val b1: bool = prev_tally.isSome() => (tally_result == prev_tally)
  val b2: bool = prev_block_time >= END_AT => (ballots == prev_ballots)
  val b3: bool = (tally_result.isSome() && prev_tally.isNone()) => (block_time >= END_AT)
  val b4: bool = (ballots != prev_ballots) => (START_AT <= block_time && block_time < END_AT)
  val b6: bool = ADDR.forall(k => ballotChanged(prev_ballots, ballots, k) => (last_action == "SubmitBallot" && last_sender == k))
  val b7: bool = prev_tally.isSome() => tally_result.isSome()
  val b8_shadow: bool = tally_result.isSome() => REGISTRY_HONEST

  val all_invariants: bool = s1 && s2 && s3 && s4 && s6 && s7 && s8 && s9 && s10 &&
                             b1 && b2 && b3 && b4 && b6 && b7 && b8_shadow

  // Witnesses: negations of properties we want to see violated
  val witness_resolution_reachable: bool = not(tally_result.isSome())
  val witness_ballot_submittable: bool = ballots.keys().size() == 0
  val witness_end_at_crossing: bool = block_time < END_AT
}
===END FILE===

===BEGIN FILE: main.qnt===
// Concrete bounded universe for verified-rcv
module main {
  import rcv.*

  // Concrete parameters
  pure val ADDR = Set("A", "B", "C", "Eve")
  pure val CANDIDATE_LIST = List("A", "B", "C")
  pure val START_AT = 2
  pure val END_AT = 5
  pure val REGISTRY_HONEST = true

  // Override step to provide concrete ballot preferences
  action step = any {
    instantiate,
    tick,
    close_and_tally,
    publish_result,
    nondet sender = oneOf(candidateSet())
    any {
      // valid full-ranking ballots for the 3-candidate universe
      submit_ballot(sender, List("A", "B", "C")),
      submit_ballot(sender, List("A", "C", "B")),
      submit_ballot(sender, List("B", "A", "C")),
      submit_ballot(sender, List("B", "C", "A")),
      submit_ballot(sender, List("C", "A", "B")),
      submit_ballot(sender, List("C", "B", "A")),
    }
  }

  action init = rcv::init
}
===END FILE===

===BEGIN FILE: design-notes.md===
**Block-to-action mapping (§2.5)**
- Block 1 (`instantiate`) → `instantiate` action: sets `initialized`, seeds constants.
- Block 2/4 (time advance) → `tick` action: increments `block_time`; derived states (Created/Voting/Tallying) are guards on other actions.
- Block 3 (`submit_ballot`) → `submit_ballot(sender,prefs)`: requires `START_AT ≤ block_time < END_AT` and `sender ∈ candidates`.
- Block 5 (`close_and_tally`) → `close_and_tally` no-op action: requires Tallying, preserves state.
- Block 6 (`publish_result`) → `publish_result`: deterministically computes `tally_spec(ballots, CANDIDATE_LIST)` and stores it. This encodes B10’s classical shadow: the chain only accepts the semantically correct result.
- Block E1 (enclave) is elided; its effect is the pure def `tally_spec`.

**Invariant encoding**
- **S-series (structural):** S1–S3 are const-level predicates; S4, S6–S10 are state predicates evaluated over current `tally_result` and `ballots`. S5 (handler-set write discipline) is enforced by construction—only `publish_result` writes `tally_result` and only when `None`.
- **B-series (temporal):** Encoded via explicit history variables (`prev_ballots`, `prev_tally`, `prev_block_time`, `last_action`, `last_sender`) updated on every transition. B1 (`tally_result` immutable once set) becomes `prev_tally.isSome() ⇒ tally_result == prev_tally`. B2 (frozen ballots after `end_at`) becomes `prev_block_time ≥ END_AT ⇒ ballots == prev_ballots`. B3/B4 use the same history pattern. B6 uses `last_action`/`last_sender` to attribute each ballot change to a unique `SubmitBallot` event.
- **B8/B9/B10:** Quint cannot model cryptography or probabilities. B8 is reduced to the classical shadow `tally_result.isSome() ⇒ REGISTRY_HONEST`, enforced by the `publish_result` guard. B9 is omitted entirely (meta-security/probabilistic). B10 is enforced by construction: `publish_result` stores exactly `Some(tally_spec(...))`, making the tally correctness a deterministic state更新 rather than an off-chain existential.
- **B10_lean:** Omitted—Lean discharge is outside Quint’s scope.

**Omissions**
- Encryption/decryption is abstracted away: ballots are stored as plaintext `List[str]` rankings; the ECIES layer is trusted to round-trip correctly via the off-chain Quartz theorem.
- `DstackAttestation`, `zkdcap`, and SHA-256 hashing are not modeled; B8 clauses (a)–(d) collapse to the boolean `REGISTRY_HONEST`.
- Canonical serialization order and `ballots@end_at` are implicit in the deterministic pure def `tally_spec`; no explicit `end_at` snapshot variable is needed because B2 guarantees immutability.
- `Set<str>` is used instead of `Vec<Addr>` where declaration order is not load-bearing for the Quint model checker (structural invariants care about membership, not serialization).

**Witnesses**
- Each witness is the negation of the desired reachability property, so `quint run` produces a violating trace that demonstrates the state is reachable.
===END FILE===

STATUS: ok