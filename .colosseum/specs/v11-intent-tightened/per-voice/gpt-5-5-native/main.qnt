// -*- mode: Bluespec; -*-
module main {
  import rcv(
    CANDIDATES = List("A", "B", "C"),
    START_AT = 2,
    END_AT = 5,
  ) .* from "./rcv"
}
