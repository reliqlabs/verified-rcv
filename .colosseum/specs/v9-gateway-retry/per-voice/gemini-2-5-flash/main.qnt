module main {
  import rcv(
    NUM_CANDIDATES = 3,
    INITIAL_TIME = 0,
    VOTING_DURATION = 10,
    ENCLAVE_PUBKEY_STR = "test_pubkey"
  ).* from "./rcv"
}
