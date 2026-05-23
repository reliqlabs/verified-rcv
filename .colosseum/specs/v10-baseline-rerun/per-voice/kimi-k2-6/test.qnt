module test {
  import rcv(CANDIDATES = List("A", "B", "C"), START_AT = 2, END_AT = 5).* from "./rcv"
  action myinit = init
  action mystep = step
}
