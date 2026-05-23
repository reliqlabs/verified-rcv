module main {
    import rcv(
        CANDIDATES = ["A", "B", "C"],
        START_AT = 2,
        END_AT = 4,
        MAX_TIME = 6
    ) .* from "./rcv"
}