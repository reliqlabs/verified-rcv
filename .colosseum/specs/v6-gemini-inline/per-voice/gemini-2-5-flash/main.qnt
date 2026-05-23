module main {
  import rcv(
    MIN_CANDIDATES = 1,
    CONTRACT_ADDR = "rcv_contract_addr_1",
    CANONICAL_VERIFIED_RCV_VKEY = "dummy_vkey_bytes",
    CANONICAL_VERIFIED_RCV_MRTD = "dummy_mrtd_bytes",
    CANONICAL_VERIFIED_RCV_RTMR = "dummy_rtmr_bytes",
    DST_VERIFIED_RCV_TALLY_V1 = "DST_VERIFIED_RCV_TALLY_V1",
    ENCLAVE_ID_NAME = "verified-rcv",

    INITIAL_CANDIDATES = List("A", "B", "C"),
    INITIAL_START_AT = 10, // Must be > env_block_time' (which starts at 0)
    INITIAL_END_AT = 20,  // Must be > INITIAL_START_AT
    INITIAL_ENCLAVE_PUBKEY = "dummy_enclave_pubkey_bytes",

    SOME_ADDRS = Set("A", "B", "C", "D", "E", "non_candidate_X"),
    SOME_CIPHERTEXTS = Set("ballot_A_enc", "ballot_B_enc", "ballot_C_enc", "bad_ballot_enc"),
    SOME_PUBKEYS = Set("pubkey1", "pubkey2"),
    SOME_TIMESTAMPS = Set(5, 10, 15, 20, 25, 30)
  ).* from "./rcv"
}