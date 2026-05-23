module rcv {

	// ---------- Types ----------
	type Addr = str
	type Timestamp = int
	type Ciphertext = str
	type Nat = int   // non‑negative integer

	// Generic optional type
	type Opt[T] = { present: bool, value: T }

	// TallyResult as specified in the intent document
	type TallyResult = {
		winners: Set[Addr],
		per_round_counts: List[Map[Addr, Nat]],
		eliminated_by_round: List[Set[Addr]],
		ballots_tallied: Nat,
		ballots_dropped: Nat,
		dropped_voters: Set[Addr],
		non_voters: Set[Addr],
	}

	// Contract storage
	type ContractState = {
		candidates: Set[Addr],
		start_at: Timestamp,
		end_at: Timestamp,
		enclave_pubkey: str,
		ballots: Map[Addr, Ciphertext],
		tally_result: Opt[TallyResult],
	}

	// ---------- Const parameters (instantiated in main.qnt) ----------
	const MAX_CANDIDATES: int
	const START_AT: Timestamp
	const END_AT: Timestamp
	const CANDIDATE_SET: Set[Addr]
	const ENCLAVE_PUBKEY: str

	// ---------- Global mutable variables ----------
	var state: ContractState
	var time: Timestamp

	// ---------- Pure helpers ----------
	pure def derived_state(s: ContractState, t: Timestamp): str = {
		if (t < s.start_at) "Created"
		else if (t < s.end_at) "Voting"
		else if (s.tally_result.present == false) "Tallying"
		else "Resolved"
	}

	// Default (dummy) tally result used for the empty optional value
	pure val default_tally: TallyResult = {
		winners: Set(),
		per_round_counts: List(),
		eliminated_by_round: List(),
		ballots_tallied: 0,
		ballots_dropped: 0,
		dropped_voters: Set(),
		non_voters: Set(),
	}

	// ---------- Actions ----------
	action init = all {
		state' = {
			candidates: CANDIDATE_SET,
			start_at: START_AT,
			end_at: END_AT,
			enclave_pubkey: ENCLAVE_PUBKEY,
			ballots: Map(),
			tally_result: { present: false, value: default_tally },
		},
		time' = START_AT - 1   // just before the voting window
	}

	// nondeterministic time advance (keeps the state space small)
	action tick = any {
		time' = time + oneOf(Set(0, 1, 2, 5, 10))
	}

	// submit a ballot (last‑write‑wins)
	action submit_ballot = any {
		val sender = oneOf(state.candidates)
		val ct = oneOf(Set("ct1", "ct2", "ct3"))   // placeholder ciphertext
		derived_state(state, time) == "Voting",
		ct != "",
		state' = state.with("ballots", state.ballots.set(sender, ct))
	}

	// signal that the voting window has closed; no state change
	action close_and_tally = any {
		derived_state(state, time) == "Tallying"
	}

	// publish the tally result together with an attestation
	action publish_result = any {
		val tr = oneOf(sample_tallies),          // a concrete well‑formed result
		val att = "attestation_placeholder",    // placeholder (not model‑checked)
		derived_state(state, time) == "Tallying",
		state.tally_result.present == false,
		// well‑formedness checks (lightweight, enough for the invariant)
		all w in tr.winners => w in state.candidates,
		state' = state.with("tally_result", { present: true, value: tr })
	}

	// a sample set of well‑formed tallies used only for modelling
	pure val sample_tallies: Set[TallyResult] = {
		{
			winners: Set("A"),
			per_round_counts: List(Map("A" -> 5)),
			eliminated_by_round: List(),
			ballots_tallied: 5,
			ballots_dropped: 0,
			dropped_voters: Set(),
			non_voters: Set(),
		}
	}

	// ---------- Step relation ----------
	action step = any {
		init,
		tick,
		submit_ballot,
		close_and_tally,
		publish_result
	}

	// ---------- Structural invariants (pointwise) ----------
	pure val inv_nonempty_candidates: bool = size(state.candidates) >= 1

	// distinctness is inherent to Set; keep as trivially true
	pure val inv_distinct_candidates: bool = true

	pure val inv_window_order: bool = state.start_at < state.end_at

	pure val inv_ballot_keys_are_candidates: bool =
		all k in state.ballots.keys => k in state.candidates

	pure val inv_winner_subset: bool = if state.tally_result.present then
		all w in state.tally_result.value.winners => w in state.candidates
	else true

	pure val inv_ballot_conservation: bool = if state.tally_result.present then
		val tr = state.tally_result.value;
		tr.ballots_tallied + tr.ballots_dropped + size(tr.non_voters) == size(state.candidates)
	else true

	// ---------- Composite invariant required by the dispatcher ----------
	val all_invariants: bool =
		inv_nonempty_candidates &&
		inv_distinct_candidates &&
		inv_window_order &&
		inv_ballot_keys_are_candidates &&
		inv_winner_subset &&
		inv_ballot_conservation

	// ---------- Reachability witnesses (must be violated) ----------
	// 1. Reach a resolved state (tally_result becomes present)
	val witness_resolution_reachable: bool = !state.tally_result.present

	// 2. Submit at least one ballot
	val witness_ballot_submittable: bool = state.ballots = Map()

	// 3. Let time cross the end of the voting window
	val witness_end_at_crossing: bool = time < END_AT
}