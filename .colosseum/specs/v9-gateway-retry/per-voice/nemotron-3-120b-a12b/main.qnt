// -*- mode: Bluespec; -*-
// Main module for verified-rcv IRV system
// Binds parameters for rcv.qnt

module main
    const TEST_START: Timestamp = 100
    const TEST_END: Timestamp = 200

    import rcv(
        EXPECTED_MRTM = "expected_mrtm_value",
        EXPECTED_RTMR = "expected_rtmr_value", 
        CONTRACT_ADDR = "contract_addr_value",
        ENCLAVE_PUBKEY_LENGTH = 32
    ) .* from "./rcv"

// We can now use the actions and state from rcv directly
// No need to redeclare init/step - they come from rcv