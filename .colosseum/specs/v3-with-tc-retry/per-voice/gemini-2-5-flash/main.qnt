// -*- mode: Bluespec; -*-

module main {
  import rcv(
    CANDIDATES = List("A", "B", "C"),
    START_AT = 10,
    END_AT = 20,
    DSTACK_KMS_KEY_DERIVABLE = true, // Assume KMS is always available and correct for this model
    ENCLAVE_IDENTITY_MATCHES = true, // Assume enclave identity always matches
    ZKDCAP_PROOF_VERIFIES = true, // Assume ZK proof always verifies
    TDX_QUOTE_VALIDATES = true, // Assume TDX quote always validates
    DOMAIN_SEPARATION_TAG = "DST_VERIFIED_RCV_TALLY_V1", // Example tag
    CONTRACT_ADDR = "rcv_contract_address_xyz" // Example contract address
  ).* from "./rcv"
}