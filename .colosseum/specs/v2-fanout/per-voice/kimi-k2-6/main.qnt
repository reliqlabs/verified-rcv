module main {
  import rcv(
    ADDR_SET = Set("A", "B", "C"),
    TIME_MAX = 5
  ).* from "./rcv"
}