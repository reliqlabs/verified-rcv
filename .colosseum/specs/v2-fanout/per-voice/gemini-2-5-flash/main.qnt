module main {
  import rcv(
    CANDIDATE_SET_MAX_SIZE = 3,         // Limit candidate count for model checking
    DST_VERIFIED_RCV_TALLY_V1 = "00000000000000000000000000000000", // Example 32-byte (hex) tag
    CANONICAL_VERIFIED_RCV_VKEY = "11111111111111111111111111111111", // Example 32-byte (hex) vkey
    CANONICAL_VERIFIED_RCV_MRTD = "22222222222222222222222222222222", // Example 32-byte (hex) MRTD
    CANONICAL_VERIFIED_RCV_RTMR = "33333333333333333333333333333333", // Example 32-byte (hex) RTMR
    IS_IMAGE_REGISTRATION_HONEST = true // Assume honest registration for basic checks
  ).* from "./rcv"

  // Initializer for the system
  action init = rcv_init

  // Main step action for model checking
  action step = rcv_init.or(rcv_step_one_contract)
}