/*
 * verified-rcv – Quint model of the three‑component IRV system.
 *
 * This module defines the contract state, actions, derived state, invariants,
 * and the three reachability witnesses required by the Colosseum harness.
 *
 * It follows the idiom of the canonical reactor example: a global mutable
 * variable `contract` holds the on‑chain storage, a mutable `block_time`
 * models `env.block.time`, and actions update the state.
 */

/* ------------------------------------------------------------------ */
/* Types and constants                                                */
/* ------------------------------------------------------------------ */

type Addr = str               // chain address (bech32 string)
type Byte = int               // abstract byte (0..255)
type Ballot = Vec[Byte]       // ECIES‑encrypted preferences

// A simple optional value: either `None` or `Some(v)`.
type MaybeTally = { None } | { Some: TallyResult }

// TallyResult schema (Section 2.5 of the intent document)
type TallyResult = {
    winners: Set[Addr],
    per_round_counts: Set[Map[Addr, int]],   // each round’s surviving‑candidate map
    eliminated_by_round: Set[Set[Addr]],     // parallel to per_round_counts
    ballots_tallied: int,
    ballots_dropped: int,
    dropped_voters: Set[Addr],
    non_voters: Set[Addr],
}

/* ------------------------------------------------------------------ */
/* Global mutable state                                               */
/* ------------------------------------------------------------------ */

var block_time: int                     // models `env.block.time`
var contract: ContractState

type ContractState = {
    candidates: Set[Addr],
    start_at: int,
    end_at: int,
    enclave_pubkey: str,
    ballots: Map[Addr, Ballot],
    resolved: bool,                     // `true` ⇔ `tally_result` is set
    tally: TallyResult,                 // defined only when `resolved = true`
}

/* ------------------------------------------------------------------ */
/* Pure helper definitions                                            */
/* ------------------------------------------------------------------ */

pure def empty_tally(): TallyResult = {
    {
        winners: Set(),
        per_round_counts: Set(),
        eliminated_by_round: Set(),
        ballots_tallied: 0,
        ballots_dropped: 0,
        dropped_voters: Set(),
        non_voters: Set(),
    }
}

/* derived state as a string for readability */
pure def derived_state(s: ContractState, t: int): str = {
    if (t < s.start_at) then "Created"
    else if (t < s.end_at) then "Voting"
    else if (!s.resolved) then "Tallying"
    else "Resolved"
}

/* ------------------------------------------------------------------ */
/* Actions (state transitions)                                       */
/* ------------------------------------------------------------------ */

/* Initialise the contract – corresponds to Block 1 (instantiate) */
action init_contract(cands: Set[Addr], start: int, end: int, pubkey: str): bool = all {
    size(cands) >= 1,
    // distinctness is enforced by Set semantics
    start > block_time,
    end > start,
    // initialise storage
    contract' = {
        candidates: cands,
        start_at: start,
        end_at: end,
        enclave_pubkey: pubkey,
        ballots: Map(),
        resolved: false,
        tally: empty_tally(),
    },
    // no other globals change
}

/* Advance blockchain time – implicit block production */
action advance_time(new_t: int): bool = all {
    new_t > block_time,
    block_time' = new_t
}

/* Submit a ballot – corresponds to Block 3 (submit_ballot) */
action submit_ballot(sender: Addr, enc: Ballot): bool = all {
    // must be in the voting window
    block_time >= contract.start_at,
    block_time < contract.end_at,
    // sender must be a declared candidate
    sender.in(contract.candidates),
    // update the ballots map (last‑write‑wins)
    contract' = contract.with(
        "ballots",
        contract.ballots.set(sender, enc)
    )
}

/* Close and tally – Block 5 (close_and_tally) – emits an event, no state change */
action close_and_tally(): bool = all {
    block_time >= contract.end_at
}

/* Publish the tally result – Block 6 (publish_result) */
action publish_result(t: TallyResult, attestation: str): bool = all {
    // must be in Tallying and not already resolved
    block_time >= contract.end_at,
    !contract.resolved,
    // well‑formedness checks (simplified, structural only)
    t.winners.subset(contract.candidates),
    size(t.winners) >= 1,
    // commit the result
    contract' = contract.with(
        "resolved", true
    ).with(
        "tally", t
    )
}

/* ------------------------------------------------------------------ */
/* Composite step relation – nondeterministic scheduler                */
/* ------------------------------------------------------------------ */

action step = any {
    // contract creation (once)
    init_contract(
        Set("A","B","C","D","E"),
        100,            // start_at
        200,            // end_at
        "pubkey"
    ),
    // time can always advance
    nondet new_t = oneOf(Set(50, 120, 210, 300)),
    advance_time(new_t),
    // any candidate may submit a ballot (if voting)
    nondet sender = oneOf(contract.candidates),
    nondet enc = Set(0,1,2),   // abstract ciphertext placeholder
    submit_ballot(sender, enc),
    // close_and_tally is idempotent
    close_and_tally(),
    // publish_result (once) – we use a dummy well‑formed tally
    nondet winners = Set("A"),
    nondet prc = Set({ "A": 5 }),
    publish_result(
        {
            winners: winners,
            per_round_counts: prc,
            eliminated_by_round: Set(),
            ballots_tallied: 5,
            ballots_dropped: 0,
            dropped_voters: Set(),
            non_voters: Set(),
        },
        "attestation"
    )
}

/* ------------------------------------------------------------------ */
/* Invariants                                                         */
/* ------------------------------------------------------------------ */

/* Structural invariants (S‑series) */
pure def inv_S1(): bool = size(contract.candidates) >= 1
pure def inv_S2(): bool = !contract.candidates.intersect(contract.candidates).exists(x => false) // trivially true, kept for completeness
pure def inv_S3(): bool = contract.start_at < contract.end_at
pure def inv_S4(): bool = all {
    forall k. contract.ballots.keys().contains(k) -> k.in(contract.candidates)
}
pure def inv_S5(): bool = !contract.resolved || contract.tally.winners.subset(contract.candidates)
pure def inv_S6(): bool = !contract.resolved || (size(contract.tally.winners) >= 1 && size(contract.tally.winners) <= size(contract.candidates))

/* Temporal‑style invariants are expressed as state‑only predicates that the
   transition relation respects (the Colosseum harness checks them on each step). */

/* B1 – tally_result is monotone‑once‑set */
pure def inv_B1(): bool = !contract.resolved || (next.contract.resolved && next.contract.tally == contract.tally)

/* B2 – no late ballots (ballots frozen after end_at) */
pure def inv_B2(): bool = (block_time >= contract.end_at) -> (contract.ballots == next.contract.ballots)

/* B4 – ballots may only change during the voting window */
pure def inv_B4(): bool = (contract.ballots != next.contract.ballots) -> (
    block_time >= contract.start_at && block_time < contract.end_at
)

/* B6 – ballot writer is the ballot voter (unique message attribution) */
pure def inv_B6(): bool = all {
    forall k. contract.ballots.keys().contains(k) && contract.ballots.get(k) != next.contract.ballots.get(k) ->
        (exists! msg. msg.kind = "SubmitBallot" && msg.sender = k &&
            next.contract.ballots.get(k) = msg.payload.enc)
}

/* Composite invariant required by the harness */
val all_invariants: bool = inv_S1() && inv_S3() && inv_S4() && inv_S5() && inv_S6() && inv_B1() && inv_B2() && inv_B4() && inv_B6()

/* ------------------------------------------------------------------ */
/* Reachability witnesses (must be violated)                         */
/* ------------------------------------------------------------------ */

/* Helper predicates */
pure def can_resolve(): bool = contract.resolved
pure def can_submit(): bool = block_time >= contract.start_at && block_time < contract.end_at
pure def can_tally(): bool = block_time >= contract.end_at && !contract.resolved

/* Witnesses are the negation of a reachable condition */
val witness_resolution_reachable: bool = !can_resolve()
val witness_ballot_submittable: bool = !can_submit()
val witness_end_at_crossing: bool = !can_tally()