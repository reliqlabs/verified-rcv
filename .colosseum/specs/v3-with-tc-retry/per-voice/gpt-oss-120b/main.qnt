module main {
    import rcv(
        MAX_CANDIDATES = 5,
        START_AT = 10,
        END_AT = 20,
        CANDIDATE_SET = Set("A", "B", "C", "D", "E"),
        ENCLAVE_PUBKEY = "pubkey_placeholder"
    ) .* from "./rcv"
}