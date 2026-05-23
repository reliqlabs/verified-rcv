module rcv {
    // Simple placeholder types
    type Phase = str
    
    // State record type
    type State = { phase: Phase }
    
    // State variable
    var phase: Phase
    
    // Initial state definition
    pure def init(): State = { phase: "init" }
    
    // Actions to reach specific phases (witness states)
    action resolve(): bool = all { phase' = "resolution" }
    action ballot_submittable(): bool = all { phase' = "ballot_submittable" }
    action end_at_crossing(): bool = all { phase' = "end_at_crossing" }
    
    // Generic step action nondeterministically chooses a transition
    action step(): bool = or { resolve(), ballot_submittable(), end_at_crossing() }
}
