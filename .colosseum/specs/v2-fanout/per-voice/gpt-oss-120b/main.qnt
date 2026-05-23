module main {
    import rcv(
        CANDIDATES       = Vec("A", "B", "C", "D", "E"),
        START_AT         = 100,
        END_AT           = 200,
        ENCLAVE_PUBKEY   = "enclave_pubkey_placeholder",
        SAMPLE_CIPHERS   = Set("cipher1", "cipher2", "cipher3")
    ) .* from "./rcv"
}