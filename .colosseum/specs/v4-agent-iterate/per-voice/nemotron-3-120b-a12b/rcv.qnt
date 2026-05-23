module rcv {
    // Types
    type Addr = Bytes
    type PubKey = Bytes
    type Timestamp = int
    type Byte = int
    type Bytes = List[Byte]

    // Parameters (to be set in main via import renaming)
    const candidates: List[Addr]
    const start_at: Timestamp
    const end_at: Timestamp
    const enclave_pubkey: PubKey
    const contract_addr: Addr

    // TallyResult structure
    def TallyResult = {
        winners: List[Addr],
        per_round_counts: List[Map[Addr, int]],
        eliminated_by_round: List[List[Addr]],
        ballots_tallied: int,
        ballots_dropped: int,
        dropped_voters: List[Addr],
        non_voters: List[Addr]
    }

    // State
    var ballots: Map[Addr, Bytes]
    var tally_result: { present: bool, value: TallyResult }
    var now: Timestamp

    // Helper: derived state
    pure def is_created(): bool = now < start_at
    pure def is_voting(): bool = start_at <= now and now < end_at
    pure def is_tallying(): bool = now >= end_at and not(tally_result.present)
    pure def is_resolved(): bool = tally_result.present

    // Empty TallyResult for initialization
    pure def emptyTallyResult(): TallyResult = {
        winners: [],
        per_round_counts: [],
        eliminated_by_round: [],
        ballots_tallied: 0,
        ballots_dropped: 0,
        dropped_voters: [],
        non_voters: []
    }

    // Initialization
    action init = {
        ballots' = {}
        tally_result' = { present: false, value: emptyTallyResult() }
        now' = start_at - 1   // start before voting window
    }

    // Time advances (implicitly, but we model as action)
    action tick = {
        now' = now + 1
    }

    // Submit ballot
    action submit_ballot(voter: Addr, encrypted: Bytes) = {
        // Precondition: voting window open
        is_voting()
        // Voter must be a candidate
        voter in candidates
        // Encrypted preferences non-empty
        len(encrypted) > 0
        // Effect: last-write-wins
        ballots' = ballots.set(voter, encrypted)
    }

    // Close and tally (event only)
    action close_and_tally = {
        // Precondition: in tallying state
        is_tallying()
        // No state change
    }

    // Publish result
    action publish_result(tally: TallyResult, attestation: Bytes) = {
        // Precondition: in tallying state
        is_tallying()
        // Attestation must be valid (we model as an uninterpreted predicate)
        // For now, we assume any attestation is allowed; we will later constrain via invariants
        // Effect: set tally_result
        tally_result' = { present: true, value: tally }
    }
}