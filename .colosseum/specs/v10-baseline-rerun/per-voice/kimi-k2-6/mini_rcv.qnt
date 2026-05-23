module mini_rcv {
  const CANDIDATES: List[str]
  var x: int
  action init = all { x' = 0 }
  action step = any { all { x' = x + 1 } }
}
