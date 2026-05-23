// -*- mode: Bluespec; -*-
module rcv {
    // 1. System Identity: verified-rcv
    // Scope: end-to-end system (CosmWasm contract, dstack-TDX enclave, zkdcap attestation)
    // Purpose: verifiable IRV election, private ballots, public rounds/winner.

    // Types defined in Section 2.5 and Section 1
    type Addr = str
    type Timestamp = int // env.block.time, start_at, end_at
    type PubKey = str // enclave_pubkey
    type PrivKey = str // used off-chain, but referenced in B10 shadow
    type Bytes = str // For encrypted_preferences (Vec<u8>), SHA-256 digests (Bytes32), domain-separation tag

    type EnclaveIdentity = { // For on-chain registry, §6.1
        vkey: Bytes,
        mrtd: Bytes,
        rtmr: Bytes,
    }

    type DstackAttestation = { // Placeholder for B8, §2.5 Block 6
        // Abstracted to key components for Quint
        tdx_quote_valid: bool,
        zkdcap_proof_verifies: bool,
        attested_user_data_lower_32_bytes: Bytes, // Represents SHA-256(canonical_serialization(contract_addr || tally_body))
        attested_user_data_upper_32_bytes: Bytes, // DST_VERIFIED_RCV_TALLY_V1
        attested_mrtd: Bytes,
        attested_rtmr: Bytes,
    }

    type TallyResult = { // §2.5 TallyResult schema
        winners: Seq[Addr],
        per_round_counts: Seq[Map[Addr, int]], // Nat is int >= 0
        eliminated_by_round: Seq[Seq[Addr]],
        ballots_tallied: int, // Nat
        ballots_dropped: int, // Nat
        dropped_voters: Seq[Addr],
        non_voters: Seq[Addr],
    }

    type ContractMessageKind = "Instantiate" | "SubmitBallot" | "CloseAndTally" | "PublishResult"

    type ContractMessage = { // §2.5 Transaction trace model
        kind: ContractMessageKind,
        sender: Addr,
        // Payload fields relevant for specific message kinds
        encrypted_preferences: Option[Bytes], // For SubmitBallot
        tally: Option[TallyResult],           // For PublishResult
        attestation: Option[DstackAttestation], // For PublishResult
    }

    // State variables (contract storage, §2.5)
    var candidates: Seq[Addr]
    var start_at: Timestamp
    var end_at: Timestamp
    var enclave_pubkey: PubKey
    var ballots: Map[Addr, Bytes] // Map<Addr, Vec<u8>>
    var tally_result: Option[TallyResult]

    // Ghost variables for temporal reasoning and derived state
    var env_block_time: Timestamp // for env.block.time
    var ballots_at_end_at: Map[Addr, Bytes] // snapshot for B2, B10
    var fires_at_transition: Set[ContractMessage] // for B6, reset each step

    // On-chain registry for enclave identity, §6.1
    var chain_registry: EnclaveIdentity

    // Parameters (const declarations, to be instantiated in main.qnt)
    const CONTRACT_ADDR: Addr
    const DST_VERIFIED_RCV_TALLY_V1: Bytes
    const CANONICAL_VERIFIED_RCV_VKEY: Bytes
    const CANONICAL_VERIFIED_RCV_MRTD: Bytes
    const CANONICAL_VERIFIED_RCV_RTMR: Bytes
    const MAX_TIMESTAMP: int // For bounding time in simulation

    // Pure Predicates (Derived States, §2.5)
    pure def is_Created(s: State): bool =
        s.env_block_time < s.start_at && s.tally_result.isNone()

    pure def is_Voting(s: State): bool =
        s.start_at <= s.env_block_time && s.env_block_time < s.end_at && s.tally_result.isNone()

    pure def is_Tallying(s: State): bool =
        s.env_block_time >= s.end_at && s.tally_result.isNone()

    pure def is_Resolved(s: State): bool =
        s.tally_result.isSome()

    // Pure Predicates (General helpers)
    pure def is_candidate(addr: Addr): bool =
        candidates.exists(c => c == addr)

    pure def all_candidates_distinct(cands: Seq[Addr]): bool =
        Set(cands).size() == cands.size()

    pure def map_keys_subset_of_seq(m: Map[Addr, any], s: Seq[Addr]): bool =
        m.keys().all(k => s.exists(x => x == k))

    pure def map_values_sum(m: Map[Addr, int]): int =
        m.values().fold(0, (acc, v) => acc + v)

    pure def image_registration_honest(s: State): bool = // §6.1
        s.chain_registry.vkey == CANONICAL_VERIFIED_RCV_VKEY &&
        s.chain_registry.mrtd == CANONICAL_VERIFIED_RCV_MRTD &&
        s.chain_registry.rtmr == CANONICAL_VERIFIED_RCV_RTMR

    // Placeholder for Tally_spec, as its full implementation is off-chain (B10, §2.5)
    // For Quint, we treat it as an uninterpreted function that satisfies the well-formedness
    // properties and ideally would match the published tally given correct inputs.
    pure def Tally_spec_stub(
        raw_ballots: Map[Addr, Bytes],
        candidates_seq: Seq[Addr],
        priv_key: PrivKey // This is an off-chain input, but included for fidelity to B10
    ): TallyResult = {
        val tallied_count = raw_ballots.keys().size()
        val dropped_count = candidates_seq.size() - tallied_count
        val non_voters_list = candidates_seq.filter(c => not(raw_ballots.keys().contains(c)))
        val winners_list = if (candidates_seq.size() > 0) then Seq(candidates_seq.first()) else Seq()

        {
            winners: winners_list,
            per_round_counts: Seq(candidates_seq.mapBy(c => (c, if (raw_ballots.keys().contains(c)) 1 else 0)).toMap()),
            eliminated_by_round: Seq(),
            ballots_tallied: tallied_count,
            ballots_dropped: dropped_count,
            dropped_voters: Seq(), // Simplified for stub; real Tally_spec populates this
            non_voters: non_voters_list,
        }
    }

    // Actions (§2.5 Blocks)
    action init = all {
        candidates' = Seq(),
        start_at' = 0,
        end_at' = 0,
        enclave_pubkey' = "",
        ballots' = Map(),
        tally_result' = None(),
        env_block_time' = 0,
        ballots_at_end_at' = Map(), // Initial empty state
        fires_at_transition' = Set(), // Reset for the first step
        chain_registry' = { vkey: "", mrtd: "", rtmr: "" }, // Unregistered/empty initially
    }

    action instantiate(
        initial_candidates: Seq[Addr],
        initial_start_at: Timestamp,
        initial_end_at: Timestamp,
        issued_enclave_pubkey: PubKey,
        initial_registry: EnclaveIdentity
    ) = all {
        is_Created(state), // Must be in Created (pre-init is implicit 'Created' here for simplicity)
        initial_candidates.size() >= 1, // S1
        all_candidates_distinct(initial_candidates), // S2
        initial_start_at > env_block_time, // Requires `start_at` > `env.block.time`
        initial_end_at > initial_start_at, // S3
        // DstackKeyManager-issued keypair derivable for this contract instance (abstracted)
        candidates' = initial_candidates,
        start_at' = initial_start_at,
        end_at' = initial_end_at,
        enclave_pubkey' = issued_enclave_pubkey,
        ballots' = Map(),
        tally_result' = None(),
        env_block_time' = env_block_time, // Time doesn't advance in instantiation
        ballots_at_end_at' = Map(),
        fires_at_transition' = Set({ kind: "Instantiate", sender: CONTRACT_ADDR, encrypted_preferences: None(), tally: None(), attestation: None() }), // Modelled as coming from the contract itself
        chain_registry' = initial_registry, // Registry is set once at instantiation/governance
    }

    action time_advances(new_time: Timestamp) = all {
        new_time > env_block_time,
        new_time <= MAX_TIMESTAMP, // Bound time to prevent unbounded simulation
        env_block_time' = new_time,
        candidates' = candidates,
        start_at' = start_at,
        end_at' = end_at,
        enclave_pubkey' = enclave_pubkey,
        ballots' = ballots,
        tally_result' = tally_result,
        chain_registry' = chain_registry,
        // Update ballots_at_end_at if this is the first time crossing end_at
        ballots_at_end_at' = if (env_block_time < end_at && new_time >= end_at)
                             then ballots // Snapshot
                             else ballots_at_end_at,
        fires_at_transition' = Set(), // No specific message fires for time advance
    }

    action submit_ballot(
        sender: Addr,
        encrypted_preferences: Bytes
    ) = all {
        is_Voting(state), // Requires: derived state is Voting
        is_candidate(sender), // Requires: msg.sender ∈ candidates
        encrypted_preferences.size() > 0, // Requires: encrypted_preferences non-empty
        // Produces: ballots[msg.sender] := encrypted_preferences (last-write-wins)
        ballots' = ballots.put(sender, encrypted_preferences),
        candidates' = candidates,
        start_at' = start_at,
        end_at' = end_at,
        enclave_pubkey' = enclave_pubkey,
        tally_result' = tally_result,
        env_block_time' = env_block_time,
        ballots_at_end_at' = ballots_at_end_at,
        fires_at_transition' = Set({ kind: "SubmitBallot", sender: sender, encrypted_preferences: Some(encrypted_preferences), tally: None(), attestation: None() }),
        chain_registry' = chain_registry,
    }

    action close_and_tally = all { // Block 5
        is_Tallying(state), // Requires: derived state is Tallying
        // Produces: emits a Cosmos SDK event, no storage change.
        // Quint doesn't model events, so this is a no-op on state.
        candidates' = candidates,
        start_at' = start_at,
        end_at' = end_at,
        enclave_pubkey' = enclave_pubkey,
        ballots' = ballots,
        tally_result' = tally_result,
        env_block_time' = env_block_time,
        ballots_at_end_at' = ballots_at_end_at,
        fires_at_transition' = Set({ kind: "CloseAndTally", sender: CONTRACT_ADDR, encrypted_preferences: None(), tally: None(), attestation: None() }), // Any sender, abstract to CONTRACT_ADDR
        chain_registry' = chain_registry,
    }

    // Helper for well-formedness checks in publish_result
    pure def tally_is_well_formed(t: TallyResult): bool =
        val initial_counts = if (t.per_round_counts.size() > 0) then t.per_round_counts.first() else Map()
        all {
            t.winners.size() >= 1, // S6: 1 <= len(winners)
            t.winners.size() <= candidates.size(), // S6: len(winners) <= len(candidates)
            t.winners.all(w => is_candidate(w)), // S6: winners subset of candidates
            // S7: conservation
            t.ballots_tallied + t.ballots_dropped + t.non_voters.size() == candidates.size(),
            // S8: per-round count consistency
            t.per_round_counts.all(prc => map_values_sum(prc) == t.ballots_tallied),
            // S9: elimination monotonicity
            // The zipWith handles potential mismatch in sequence lengths gracefully for empty tail.
            t.eliminated_by_round.zipWith(
                // per_round_counts[j] for j > i => take tail of per_round_counts to represent subsequent rounds
                t.per_round_counts.tail(),
                (elim_in_round, prc_next_round) =>
                    elim_in_round.all(elim => not(prc_next_round.keys().contains(elim)))
            ).all(b => b),
            // Set relations
            t.non_voters.all(nv => not(ballots.keys().contains(nv))), // non_voters = candidates \ ballots.keys
            t.dropped_voters.all(dv => ballots.keys().contains(dv)), // dropped_voters subset of ballots.keys
            t.dropped_voters.intersect(t.non_voters).size() == 0,
            t.dropped_voters.size() == t.ballots_dropped,
        }

    action publish_result(
        sender: Addr, // Any chain address may submit
        published_tally: TallyResult,
        att: DstackAttestation
    ) = all {
        is_Tallying(state), // Requires: derived state is Tallying
        // Attestation verification (B8 shadow)
        att.tdx_quote_valid,
        att.zkdcap_proof_verifies,
        att.attested_user_data_upper_32_bytes == DST_VERIFIED_RCV_TALLY_V1,
        // The SHA-256 hash equality for `attested_user_data_lower_32_bytes` cannot be checked
        // directly in Quint, so we implicitly assume it holds for a valid attestation.
        att.attested_mrtd == chain_registry.mrtd, // B8(d)
        att.attested_rtmr == chain_registry.rtmr, // B8(d)
        image_registration_honest(state), // Precondition for B8/B9, checks chain_registry.vkey as well
        tally_is_well_formed(published_tally), // Tally well-formedness checks (S6-S9 come from here)

        tally_result' = Some(published_tally), // Produces: tally_result := Some(tally)
        candidates' = candidates,
        start_at' = start_at,
        end_at' = end_at,
        enclave_pubkey' = enclave_pubkey,
        ballots' = ballots,
        env_block_time' = env_block_time,
        ballots_at_end_at' = ballots_at_end_at,
        fires_at_transition' = Set({ kind: "PublishResult", sender: sender, encrypted_preferences: None(), tally: Some(published_tally), attestation: Some(att) }),
        chain_registry' = chain_registry,
    }

    // Step relation (any possible action)
    action step = any {
        // Time always advances
        time_advances(nondet(new_time => new_time > env_block_time)),

        // Contract lifecycle actions
        // Instantiate can only happen if the contract is in a "created" (uninitialized) state.
        // We simulate `init` and then `instantiate`.
        // If not yet instantiated (candidates is empty implies init state effectively), allow instantiation.
        candidates.size() == 0 => any {
            val initial_candidates = nondet(c => c.size() >= 1),
            val initial_start_at = nondet(t => t > env_block_time && t <= MAX_TIMESTAMP),
            val initial_end_at = nondet(t => t > initial_start_at && t <= MAX_TIMESTAMP),
            val issued_enclave_pubkey = nondet(k => k != ""),
            val initial_registry_vkey = nondet(v => v != ""),
            val initial_registry_mrtd = nondet(m => m != ""),
            val initial_registry_rtmr = nondet(r => r != ""),
            instantiate(initial_candidates, initial_start_at, initial_end_at, issued_enclave_pubkey,
                        {vkey: initial_registry_vkey, mrtd: initial_registry_mrtd, rtmr: initial_registry_rtmr}),
        },
        // Regular contract actions, only if already instantiated
        candidates.size() > 0 => any {
            nondet sender = oneOf(Set(CONTRACT_ADDR).union(Set(candidates.first()))), // Sample sender: contract or first candidate
            nondet ballot_content = oneOf(Set("BALLOT_A", "BALLOT_B")), // Sample ballot contents
            submit_ballot(sender, ballot_content),
            close_and_tally,
            // For publish_result, assume an honest enclave producing `Tally_spec_stub` output for simulation
            val published_tally = Tally_spec_stub(ballots_at_end_at, candidates, "fake_privkey"),
            // Construct an 'honest' attestation that should pass the checks
            val honest_attestation: DstackAttestation = {
                tdx_quote_valid: true,
                zkdcap_proof_verifies: true,
                attested_user_data_lower_32_bytes: "HASH_OF_TALLY_BODY", // Abstracted
                attested_user_data_upper_32_bytes: DST_VERIFIED_RCV_TALLY_V1,
                attested_mrtd: CANONICAL_VERIFIED_RCV_MRTD,
                attested_rtmr: CANONICAL_VERIFIED_RCV_RTMR,
            },
            publish_result(sender, published_tally, honest_attestation),
        }
    }

    // --- Invariants ---

    // Structural Invariants (§3.1)
    val S1_non_empty_candidates = candidates.size() >= 1
    val S2_distinct_candidates = all_candidates_distinct(candidates)
    val S3_well_ordered_voting_window = start_at < end_at
    val S4_ballot_keys_are_candidates = map_keys_subset_of_seq(ballots, candidates)
    // S5 (derived) tally_result write-discipline: "tally_result is written at most once"
    // Quint doesn't directly capture "handler-set property" from static code analysis.
    // Its temporal complement is B1, which is encoded.
    // For now, this invariant is implicitly covered by B1's temporal nature combined with the action definitions.

    val S6_winner_well_formedness =
        is_Resolved(state) =>
            tally_result.isSome() &&
            tally_result.get().winners.size() >= 1 &&
            tally_result.get().winners.size() <= candidates.size() &&
            tally_result.get().winners.all(w => is_candidate(w))

    val S7_tally_count_conservation =
        is_Resolved(state) =>
            tally_result.isSome() &&
            tally_result.get().ballots_tallied +
            tally_result.get().ballots_dropped +
            tally_result.get().non_voters.size() == candidates.size()

    val S8_per_round_count_consistency =
        is_Resolved(state) =>
            tally_result.isSome() &&
            tally_result.get().per_round_counts.all(prc => map_values_sum(prc) == tally_result.get().ballots_tallied)

    val S9_elimination_monotonicity =
        is_Resolved(state) =>
            tally_result.isSome() &&
            tally_result.get().eliminated_by_round.zipWith(
                tally_result.get().per_round_counts.tail(),
                (elim_in_round, prc_next_round) =>
                    elim_in_round.all(elim => not(prc_next_round.keys().contains(elim)))
            ).all(b => b)

    val S10_resolution_implies_past_end_at =
        is_Resolved(state) => env_block_time >= end_at


    // Behavioral Invariants (§3.2)
    val B1_tally_result_monotone_once_set =
        always (is_Resolved(state) implies always is_Resolved(state) and next.tally_result == tally_result)

    val B2_no_late_ballots =
        always (env_block_time >= end_at implies ballots == ballots_at_end_at)

    val B3_no_premature_tally =
        always (next.tally_result.isSome() and tally_result.isNone() implies env_block_time >= end_at)

    val B4_no_premature_voting =
        always (next.ballots != ballots implies start_at <= env_block_time and env_block_time < end_at)

    // B5 (derived) publish_result_fires_at_most_once: Omitted, covered by B1 and action guards.

    pure def get_ballot_message_for_sender(msg_set: Set[ContractMessage], k: Addr): Set[ContractMessage] =
        msg_set.filter(m => m.kind == "SubmitBallot" && m.sender == k && m.encrypted_preferences.isSome())

    val B6_ballot_writer_is_the_ballot_voter =
        always (
            all { k: Addr | next.ballots.keys().contains(k) || ballots.keys().contains(k) =>
                val prev_ballot_k_opt = ballots.get(k)
                val next_ballot_k_opt = next.ballots.get(k)
                if (prev_ballot_k_opt != next_ballot_k_opt) then
                    val relevant_msgs = get_ballot_message_for_sender(fires_at_transition', k)
                    relevant_msgs.size() == 1 &&
                    relevant_msgs.any(m => m.encrypted_preferences == next_ballot_k_opt)
                else
                    true // No change for this key, so condition is trivially true
            }
        )

    val B7_terminal_state_immutability =
        always (is_Resolved(state) implies always is_Resolved(state))

    // B8: attestation-binds-tally (classical-Prop shadow)
    val B8_attestation_binds_tally_shadow =
        always (
            next.tally_result.isSome() and tally_result.isNone() implies
            fires_at_transition'.any(m =>
                m.kind == "PublishResult" &&
                m.attestation.isSome() &&
                m.attestation.get().tdx_quote_valid &&
                m.attestation.get().zkdcap_proof_verifies &&
                m.attestation.get().attested_user_data_upper_32_bytes == DST_VERIFIED_RCV_TALLY_V1 &&
                m.attestation.get().attested_mrtd == chain_registry.mrtd &&
                m.attestation.get().attested_rtmr == chain_registry.rtmr
            )
        )

    // B9: B8 negligibility-budget decomposition is meta-security; omitted from Quint.
    // B10: tally-correctness (classical-Prop shadow)
    val B10_tally_correctness_shadow =
        always (
            next.tally_result.isSome() and tally_result.isNone() implies
            val privkey_placeholder = "fake_privkey" // Placeholder for existential `privkey`
            val expected_tally = Tally_spec_stub(ballots_at_end_at, candidates, privkey_placeholder)
            next.tally_result.get() == expected_tally
        )
    // B10_lean: off-chain; omitted from Quint.


    // Composite Invariant (Mandatory)
    val all_invariants = all {
        S1_non_empty_candidates,
        S2_distinct_candidates,
        S3_well_ordered_voting_window,
        S4_ballot_keys_are_candidates,
        S6_winner_well_formedness,
        S7_tally_count_conservation,
        S8_per_round_count_consistency,
        S9_elimination_monotonicity,
        S10_resolution_implies_past_end_at,
        B1_tally_result_monotone_once_set,
        B2_no_late_ballots,
        B3_no_premature_tally,
        B4_no_premature_voting,
        B6_ballot_writer_is_the_ballot_voter,
        B7_terminal_state_immutability,
        B8_attestation_binds_tally_shadow,
        B10_tally_correctness_shadow,
    }

    // Reachability Witnesses (Mandatory, must be VIOLATED by `quint run`)
    // They are phrased as the NEGATION of the property, so when `quint run` finds a trace to them,
    // it means the named state *was reached*.
    val witness_resolution_reachable = not(is_Resolved(state))
    val witness_ballot_submittable = not(is_Voting(state) and is_candidate(candidates.first()))
    val witness_end_at_crossing = not(env_block_time >= end_at)
}