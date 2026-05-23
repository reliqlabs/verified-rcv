// -*- mode: Bluespec; -*-
//
// Concrete instantiation of verified-rcv for finite model checking.
// Five candidates A..E, one non-candidate Eve, voting window
// [START_AT, END_AT) = [60, 3660).

module main {
  import rcv(
    CANDIDATES = ["A", "B", "C", "D", "E"],
    NON_CANDIDATES = Set("Eve"),
    START_AT = 60,
    END_AT = 3660,
    CIPHERTEXTS = Set("ct1", "ct2", "ct3"),
    TICK_DELTAS = Set(30, 600, 3700),
  ).* from "./rcv"
}
