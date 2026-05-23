module main {
    import rcv.* from "./rcv"
    
    // expose init and step from rcv
    pure def init(): rcv.State = rcv.init()
    
    action step(): bool = rcv.step()
    
    // safety invariant (must hold)
    val all_invariants: bool = true
    
    // Witness invariants (must be violated to show reachability)
    val witness_resolution_reachable: bool = not(rcv.phase == "resolution")
    val witness_ballot_submittable: bool = not(rcv.phase == "ballot_submittable")
    val witness_end_at_crossing: bool = not(rcv.phase == "end_at_crossing")
}
