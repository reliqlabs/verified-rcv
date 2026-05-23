// -*- mode: Bluespec; -*-
module main {
  import rcv(
    CANDIDATES = Set("A", "B", "C"),
    START_AT = 1,
    END_AT = 3,
  ).* from "./rcv"
}
