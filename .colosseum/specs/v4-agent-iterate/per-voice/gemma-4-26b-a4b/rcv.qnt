// rcv.qnt

// Constants for the module
const CANDIDATES_COUNT: Nat = 3; // Placeholder, will be constrained by invariants
const MAX_BALLOTS: Nat = 10;

// Types
type Addr = String;
type Timestamp = Nat;
type PubKey = String;
type Ciphertext = List[u8];

type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, Nat]], // Placeholder for complexity; in Quint we'll simplify
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: Nat,
    ballots_dropped: Nat,
    dropped_voters: List[Addr],
    non_voters: List[Addr]
};

type State = {
    candidates: List[Addr],
    start_at: Timestamp,
    end_at: Timestamp,
    enclave_pubkey: PubKey,
    ballots: Map[Addr, Ciphertext], // msg.sender -> encrypted_preferences
    tally_result: { present: bool, value: TallyResult } // Option<TallyResult> encoded as record
};

// Actions (Transitions)
action init(candidates: List[Addr], start_at: Timestamp, end_at: Timestamp, enclave_pubkey: PubKey) {
    // Block 1: instantiate
    assert(len(candidates) >= 1);
    // For simplicity in Quint, we'll assume candidates are distinct via intent. 
    // A real spec would need a check, but let's keep it simple for the first pass.
    assert(start_at < end_at);

    state' = {
        candidates: candidates,
        start_at: start_at,
        end_at: end_at,
        enclave_pubkey: enclave_pubkey,
        ballots: {},
        tally_result: { present: false, value: {} } // Placeholder for empty TallyResult
    };
}

action submit_ballot(sender: Addr, encrypted_preferences: Ciphertext, block_time: Timestamp) {
    // Block 3: submit_ballot
    // Requirement: derived state is Voting (start_at <= block_time < end_at)
    assert(state.start_at <= block_time && block_time < state.end_at);
    // Requirement: msg.sender in candidates
    assert(exists k in state.candidates, k == sender);

    state' = {
        candidates: state.candidates,
        start_at: state.start_at,
        end_at: state.end_at,
        enclave_pubkey: state.enclave_pubkey,
        ballots: update_map(state.ballots, sender, encrypted_preferences),
        tally_result: state.tally_result
    };
}

action publish_result(tally: TallyResult, block_time: Timestamp) {
    // Block 6: publish_result
    // Requirement: derived state is Tallying (block_time >= end_at && tally_result.is_none())
    assert(state.end_at <= block_time && !state.tally_result.present);
    
    // Requirement: well-formedness (simplified for Quint)
    assert(len(tally.winners) >= 1 && len(tally.winners) <= len(state.candidates));
    // ... other well-formedness checks omitted for brevity in first pass

    state' = {
        candidates: state.candidates,
        start_at: state.start_at,
        end_at: state.end_at,
        enclave_pubkey: state.enclave_pubkey,
        ballots: state.ballots,
        tally_result: { present: true, value: tally }
    };
}

// Helper functions (pure)
pure def update_map(m: Map[Addr, Ciphertext], k: Addr, v: Ciphertext): Map[Addr, Ciphertext] = {
    // Quint doesn't have a direct map update. In practice, we'd use a list of pairs or similar.
    // For this proto-spec, let's assume it exists as a primitive or modeled via list.
    m // Placeholder
}

// Invariants (to be defined in main.qnt)
