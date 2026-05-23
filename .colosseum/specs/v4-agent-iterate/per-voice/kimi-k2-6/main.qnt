// -*- mode: Bluespec; -*-
// main.qnt — instantiated IRV election model

module main {
  import rcv(
    ADDR_POOL = Set("A", "B", "C"),
    MAX_TIME = 20,
    ENC_PREFS = Set("enc1", "enc2"),
    CANDIDATE_SETS = Set(Set("A"), Set("A", "B"), Set("A", "B", "C"))
  ) .* from "./rcv"
}
