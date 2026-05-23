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