module rcv {

	// ----------------------------------------------------------------
	// Types
	// ----------------------------------------------------------------
	type Addr      = str
	type Timestamp = int
	type Cipher    = str
	type Nat       = int

	// Tally result schema (simplified for verification)
	type TallyResult = {
		winners               : Set[Addr],
		per_round_counts      : Set[Map[Addr, Nat]],
		eliminated_by_round   : Set[Set[Addr]],
		ballots_tallied       : Nat,
		ballots_dropped       : Nat,
		dropped_voters        : Set[Addr],
		non_voters            : Set[Addr],
	}

	// ----------------------------------------------------------------
	// Const parameters (instantiated in main.qnt)
	// ----------------------------------------------------------------
	const CANDIDATES       : Vec[Addr]
	const START_AT         : Timestamp
	const END_AT           : Timestamp
	const ENCLAVE_PUBKEY   : str
	const SAMPLE_CIPHERS   : Set[Cipher]

	// ----------------------------------------------------------------
	// Global mutable state
	// ----------------------------------------------------------------
	var candidates      : Vec[Addr]               // immutable after init
	var start_at        : Timestamp               // immutable after init
	var end_at          : Timestamp               // immutable after init
	var enclave_pubkey  : str                     // immutable after init
	var ballots         : Map[Addr, Cipher]       // mutable via SubmitBallot
	var tally_result    : Option[TallyResult]     // written once
	var env_time        : Timestamp               // simulated block time

	// ----------------------------------------------------------------
	// Helper pure definitions
	// ----------------------------------------------------------------
	pure def distinct(v: Vec[Addr]) : bool = {
		val s = Set()
		all {
			size(v) == size(v.filter(x => s = s.union(Set(x)); true))
		}
	}

	pure def winners_well_formed(tr: TallyResult) : bool = {
		size(tr.winners) >= 1 && size(tr.winners) <= size(candidates) &&
		tr.winners.subset(Set.fromVec(candidates))
	}

	pure def tally_conservation(tr: TallyResult) : bool = {
		val non_v = Set.fromVec(candidates).exclude(ballots.keys())
		tr.ballots_tallied + tr.ballots_dropped + size(non_v) == size(candidates)
	}

	pure def resolved_implies_time(tr_opt: Option[TallyResult]) : bool = {
		match tr_opt {
			Some(tr) => env_time >= end_at,
			None     => true
		}
	}

	// ----------------------------------------------------------------
	// Derived‑state predicates (pure, refer to current env_time)
	// ----------------------------------------------------------------
	pure def isCreated()   : bool = env_time < start_at
	pure def isVoting()    : bool = start_at <= env_time && env_time < end_at
	pure def isTallying()  : bool = env_time >= end_at && tally_result.is_none()
	pure def isResolved()  : bool = tally_result.is_some()

	// ----------------------------------------------------------------
	// Well‑formed tally (used by publish_result)
	// ----------------------------------------------------------------
	pure def well_formed_tally(tr: TallyResult) : bool = {
		winners_well_formed(tr) &&
		tally_conservation(tr)
	}

	// ----------------------------------------------------------------
	// Actions
	// ----------------------------------------------------------------
	action init = all {
		candidates'      = CANDIDATES,
		start_at'        = START_AT,
		end_at'          = END_AT,
		enclave_pubkey'  = ENCLAVE_PUBKEY,
		ballots'         = Map(),
		tally_result'    = None,
		env_time'        = 0
	}

	// Advance time by a nondeterministic delta (1..10)
	action advance_time = all {
		val delta = oneOf(1..10)
		env_time' = env_time + delta
	}

	// Submit a ballot (last‑write‑wins)
	action submit_ballot = any {
		val voter = oneOf(candidates)
		val ct    = oneOf(SAMPLE_CIPHERS)
		all {
			isVoting(),
			voter.in(Set.fromVec(candidates)),               // S4 guard
			ballots' = ballots.set(voter, ct)
		}
	}

	// Close and tally (no state change, just emits an event – omitted)
	action close_and_tally = all {
		isTallying()
		// no state mutation
	}

	// Publish the result (writes tally_result once)
	action publish_result = any {
		// Build a dummy well‑formed tally
		val non_v = Set.fromVec(candidates).exclude(ballots.keys())
		val tr : TallyResult = {
			winners               = Set.fromVec(candidates).filter(_ => true).take(1), // pick first candidate
			per_round_counts      = Set(),
			eliminated_by_round   = Set(),
			ballots_tallied       = size(ballots.keys()),
			ballots_dropped       = 0,
			dropped_voters        = Set(),
			non_voters            = non_v
		}
		all {
			isTallying(),
			tally_result.is_none(),
			well_formed_tally(tr),
			tally_result' = Some(tr)
		}
	}

	// ----------------------------------------------------------------
	// Step relation (nondeterministic choice of actions)
	// ----------------------------------------------------------------
	action step = any {
		init,
		advance_time,
		submit_ballot,
		close_and_tally,
		publish_result
	}

	// ----------------------------------------------------------------
	// Invariants (composite)
	// ----------------------------------------------------------------
	val all_invariants : bool = {
		// S‑series (structural) -------------------------------------------------
		// S1: at least one candidate
		size(candidates) >= 1 &&

		// S2: candidates are distinct
		distinct(candidates) &&

		// S3: voting window well‑ordered
		start_at < end_at &&

		// S4: ballot keys are candidates (enforced by submit_ballot guard, re‑checked)
		all k in ballots.keys() . k.in(Set.fromVec(candidates)) &&

		// S6: winner well‑formedness (if resolved)
		(tally_result.is_some() -> winners_well_formed(tally_result.get())) &&

		// S7: tally count conservation (if resolved)
		(tally_result.is_some() -> tally_conservation(tally_result.get())) &&

		// S10: resolution implies past end_at
		(tally_result.is_some() -> env_time >= end_at) &&

		// B‑series (temporal) --------------------------------------------------
		// B1: tally_result is monotone‑once‑set
		always (tally_result.is_some() -> next.tally_result = tally_result) &&

		// B2: ballot store freezes after end_at
		always (env_time >= end_at -> next.ballots = ballots) &&

		// B3: no premature tally
		always (next.tally_result.is_some() && tally_result.is_none() -> env_time >= end_at) &&

		// B4: ballots only change during voting window
		always (next.ballots != ballots -> start_at <= env_time && env_time < end_at) &&

		// B7: Resolved is a sink
		always (isResolved() -> always isResolved())
	}

	// ----------------------------------------------------------------
	// Reachability witnesses (must be violated => system can reach the state)
	// ----------------------------------------------------------------
	// witness that the election can reach a resolved state
	val witness_resolution_reachable : bool = not(tally_result.is_some())

	// witness that a ballot can be submitted (i.e., voting window is open)
	val witness_ballot_submittable : bool = not(isVoting())

	// witness that the block time can cross the end_at boundary
	val witness_end_at_crossing : bool = not(env_time >= end_at)
}