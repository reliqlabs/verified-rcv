/*
 * main.qnt – concrete instantiation of the verified‑rcv model.
 * The constants below bound the universe for the bounded‑model checker.
 */

module main {
    import rcv

    // Concrete constants (matching the nondet choices in rcv.step)
    const CANDIDATES = Set("A","B","C","D","E")
    const START_AT    = 100
    const END_AT      = 200
    const ENCLAVE_PUB = "pubkey"
    const INIT_TIME   = 0

    // Initialise the global variables
    action init = all {
        block_time' = INIT_TIME,
        contract' = {
            candidates: CANDIDATES,
            start_at: START_AT,
            end_at: END_AT,
            enclave_pubkey: ENCLAVE_PUB,
            ballots: Map(),
            resolved: false,
            tally: empty_tally(),
        }
    }

    // Run the system: start with init, then repeatedly fire `rcv.step`.
    action system = any {
        init,
        any {
            rcv.step,
            // allow the system to continue indefinitely (bounded model checker will limit depth)
            rcv.step,
            rcv.step,
            rcv.step,
            rcv.step
        }
    }
}