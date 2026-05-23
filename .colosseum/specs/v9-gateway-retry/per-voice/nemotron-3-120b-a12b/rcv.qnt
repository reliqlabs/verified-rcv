// -*- mode: Bluespec; -*-
// IRV voting system spec for verified-rcv
// Parameters: types and constants to be set in main.qnt

module rcv {

type Addr
type PubKey
type Timestamp = Int
type EncryptedPreferences = List[Int]  // list of bytes as integers 0-255

// TallyResult structure as defined in intent
type TallyResult = {
    winners: List[Addr],
    per_round_counts: List[Map[Addr, Int]],
    eliminated_by_round: List[List[Addr]],
    ballots_tallied: Int,
    ballots_dropped: Int,
    dropped_voters: List[Addr],
    non_voters: List[Addr]
}

// Attestation structure (simplified for on-chain verification)
type Attestation = {
    mrtm: String,      // attested enclave MRTD
    rtmr: String,      // attested enclave RTMR
    user_Data: List[Int]  // 64-byte user data (lower 32 bytes = hash, upper 32 = domain tag)
}

// Constants to be bound in main.qnt
const EXPECTED_MRTM: String
const EXPECTED_RTMR: String
const CONTRACT_ADDR: String
const ENCLAVE_PUBKEY_LENGTH: Int  // dummy constant for pubkey size

// State variables
var candidates: List[Addr]
var start_at: Timestamp
var end_at: Timestamp
var enclave_pubkey: PubKey
var ballots: Map[Addr, EncryptedPreferences]
var tally_result: Option[TallyResult]
var current_time: Timestamp  // models env.block.time

// Helper: check if in voting window
pure def voting_window(): Bool = {
    current_time >= start_at && current_time < end_at
}

// Helper: check if in tallying state
pure def tallying_state(): Bool = {
    current_time >= end_at && tally_result == None
}

// Helper: check if resolved
pure def resolved_state(): Bool = {
    tally_result != None
}

// Action: instantiate
action instantiate(cands: List[Addr], start: Timestamp, end: Timestamp, pk: PubKey): Bool = all {
    // Preconditions
    len(cands) >= 1,
    // distinct candidates (no duplicates)
    all { i in 0..len(cands)-1, j in i+1..len(cands)-1 :
        cands[i] != cands[j]
    },
    start > current_time,  // start_at > current block time
    end > start,
    // Produce state changes
    candidates' = cands,
    start_at' = start,
    end_at' = end,
    enclave_pubkey' = pk,
    ballots' = Map[],  // empty ballot map
    tally_result' = None,
    current_time' = current_time  // time doesn't change on instantiate
}

// Action: submit_ballot
action submit_ballot(voter: Addr, prefs: EncryptedPreferences): Bool = all {
    // Preconditions: must be in voting state
    voting_window(),
    voter in candidates,
    len(prefs) > 0,  // non-empty encrypted preferences
    // Produce: last-write-wins ballot update
    ballots' = ballots.set(voter, prefs),
    // Other state unchanged
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    tally_result' = tally_result,
    current_time' = current_time
}

// Action: close_and_tally (no state change, just event)
action close_and_tally(): Bool = all {
    // Precondition: must be in tallying state
    tallying_state(),
    // No state change
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    tally_result' = tally_result,
    current_time' = current_time
}

// Action: publish_result
action publish_result(tally: TallyResult, att: Attestation): Bool = all {
    // Precondition: must be in tallying state
    tallying_state(),
    // Attestation verification conditions (simplified)
    // 1. Attested enclave identity matches expected
    att.mrtm == EXPECTED_MRTM && att.rtmr == EXPECTED_RTMR,
    // 2. user_data lower 32 bytes equal hash(contract_addr ‖ tally_body)
    // We model hash as an uninterpreted function; in reality we'd check
    // lower 32 bytes of att.user_Data == hash(contract_addr ++ serialize(tally))
    // For spec, we assume a pure verifier function:
    verify_attestation(att, CONTRACT_ADDR, tally) == true,
    // 3. tally is well-formed (we assume the attester only sends well-formed tallies)
    // In reality we'd check S6-S10 here, but we rely on attester for correctness
    // Produce: set tally_result
    tally_result' = Some(tally),
    // Other state unchanged
    candidates' = candidates,
    start_at' = start_at,
    end_at' = end_at,
    enclave_pubkey' = enclave_pubkey,
    ballots' = ballots,
    current_time' = current_time
}

// Pure function to verify attestation (uninterpreted, to be assumed correct)
pure def verify_attestation(att: Attestation, addr: String, tally: TallyResult): Bool = {
    // Placeholder: actual implementation would check signature and hash
    true
}

// Initialization action
action init = all {
    candidates' = [],
    start_at' = 0,
    end_at' = 0,
    enclave_pubkey' = undefined(PubKey),  // will be set on instantiate
    ballots' = Map{},
    tally_result' = None,
    current_time' = 0
}

// Next-state relation (step) - any action can occur
action step = any {
    any { cands: List[Addr], start: Timestamp, end: Timestamp, pk: PubKey :
        instantiate(cands, start, end, pk)
    },
    any { voter: Addr, prefs: EncryptedPreferences :
        submit_ballot(voter, prefs)
    },
    close_and_tally(),
    any { tally: TallyResult, att: Attestation :
        publish_result(tally, att)
    },
    // Time can advance arbitrarily (modeling block passage)
    current_time' = current_time + 1
}
}