// -*- mode: Bluespec; -*-
// main.qnt — Instantiation of rcv.qnt with concrete parameters

module main {
  import rcv(
    CANDIDATES = List("A", "B", "C"),
    START_AT = 2,
    END_AT = 5
  ) .* from "./rcv"
}
