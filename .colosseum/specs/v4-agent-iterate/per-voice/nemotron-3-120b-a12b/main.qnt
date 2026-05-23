module main {
    // Import the rcv module and bind the parameters
    import rcv(
        candidates = [0x01, 0x02, 0x03],
        start_at = 0,
        end_at = 10,
        enclave_pubkey = 0xabcd,
        contract_addr = 0x9999
    ) .* from "./rcv"

    // Define the step action that non-deterministically chooses an action
    // For now, we only allow time to advance to keep it simple
    action step = any {
        tick()
    }
}