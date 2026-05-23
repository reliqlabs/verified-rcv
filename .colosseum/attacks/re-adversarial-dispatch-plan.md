# Re-adversarial pass — dispatch plan

- **Gates on:** revised `intent.md` landing (task #25)
- **Fires:** task #26
- **Target file:** `/Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md` (revised)
- **Output location:** `/Users/mvid/Development/reliq/verified-rcv/.colosseum/attacks/intent-revised-<ISO-timestamp>/`
- **Output layout:** multi-model (one `<channel>-<model-id>.md` per voice + `meta.md` + `synthesis.md`)

## Lineup — 6 voices, 6 families

| # | Channel | Model ID | Family | Notes |
|---|---------|----------|--------|-------|
| 1 | gateway | `claude-opus-4-7` | Anthropic | Load-bearing structural reviewer; the strongest long-context voice. Different inference seat from the doc's revising voice. |
| 2 | gateway | `kimi-k2-6` | Moonshot | Reasoning model from non-Western lab; different RLHF lineage. Catches subtle composition failures non-reasoning models miss. |
| 3 | local | `mistral-small-4-119b-2603` | Mistral | 119B local capability; fully independent training lineage. Free. |
| 4 | local | `qwen3.6-27b-mlx` | Alibaba Qwen | Second Chinese frontier voice; MLX-optimized on M-series. |
| 5 | local | `google/gemma-4-26b-a4b` | Google SLM | Different training pipeline from cloud Gemini despite shared brand. |
| 6 | local | `goedel-prover-v2-32b` | Goedel | **Theorem-prover specialist.** Wildcard for the formal aspects (B6 tagging, B8/B9 composition, negligibility decomposition). |

## Dispatch mechanics

All 6 run **blind to each other** — independent fresh attack. After all finish, synthesis surfaces overlap (multi-model consensus = real bug) and divergence (single-voice findings = potential blind-spot escape).

### Channel invocations

```
# Channel 1, 2 — gateway (parallel via fan_out_query or two query_gateway calls)
mcp__external-model__query_gateway(
    prompt=<full adversarial prompt>,
    model="claude-opus-4-7",
    max_tokens=8192
)
mcp__external-model__query_gateway(
    prompt=<full adversarial prompt>,
    model="kimi-k2-6",
    max_tokens=8192
)

# Channels 3-6 — local (parallel via fan_out_local with explicit model list)
mcp__lm-studio__fan_out_local(
    prompt=<full adversarial prompt>,
    models=[
        "mistral-small-4-119b-2603",
        "qwen3.6-27b-mlx",
        "google/gemma-4-26b-a4b",
        "goedel-prover-v2-32b",
    ],
    max_tokens=8192
)
```

`max_tokens=8192` to give reasoning models room (kimi, goedel). Down-scale to 4096 if any voice consistently truncates.

### Adversarial prompt construction

Per `colosseum-adversarial` SKILL.md Step 3: inline the full `colosseum-spec-adversary` agent system prompt, the full revised intent doc, and the brief from the previous adversarial run (with appropriate scope adjustments for revised content).

**Critical: do NOT include the prior adversarial report in the prompt.** Each voice attacks fresh. Comparison against the prior report is a post-process synthesis step, not part of any single voice's input.

Re-use the v0.2 attack-category emphasis from the first-pass brief:
- `temporal_state_mismatch` (Section 3.2 tagging discipline)
- `impossibility-hypothesis-vacuity` (B8 clause (c), Quartz `commitHashE` inheritance)
- `disjunction-vs-decomposition` (B9 summand list)
- `composition_failure` (Section 6.2 Quartz inheritance table)
- All other categories from the agent system prompt

### Blindness restrictions

Same as Round 3a first pass:
- **DO NOT READ** `/Users/mvid/Development/reliq/quartz/examples/ranked-choice/**`
- **DO NOT READ** any file matching `*rcv*` or `*ranked-choice*` under the Quartz tree
- May read Quartz general specs (`proofs/lean/Specs/Quartz/**`) and the Quartz ledger to verify inheritance claims

Note that local models cannot honor blindness via tool restriction (they have no file access). Each prompt must NOT contain any text quoted from the Quartz ranked-choice tree. The local-model channel is inherently blind by construction.

## Synthesis step (orchestrator-side, after all 6 finish)

After all responses land, write `synthesis.md` covering:

1. **Per-voice finding count** (critical / serious / cosmetic)
2. **Overlap matrix** — which findings appear in N voices (1, 2, 3+, 6/6)
3. **Coverage of the prior 42 findings** — for each of the 13 critical + 24 serious from the first pass, was it addressed? Did any voice still flag it?
4. **New findings unique to this pass** — what's surfaced that wasn't in the first pass
5. **Verdict on revision adequacy** — SURVIVES vs BREAKS-AGAIN
6. **Methodology notes for v0.3 back-port** — anything that worked/didn't in this multi-model dispatch

## Open question — claude-opus-4-7 via gateway vs Agent subagent

The first-pass adversary ran as an Agent subagent (Claude-only). For consistency in the re-pass, two options:

- **(a) All 6 voices via MCP** (current plan above) — uniform dispatch shape, no Agent tool involved. Cleaner methodology.
- **(b) Mix: 5 via MCP + 1 via Agent subagent for Claude** — Claude gets file-access tools (Read, Grep, Glob, Bash) which let it cross-verify inheritance claims against the actual Quartz tree. Asymmetric but produces a stronger Claude voice.

Recommend **(b)** for the Claude channel — the first pass surfaced load-bearing findings (Attacks 2, 4) that required reading Quartz's ledger and grep'ing for `DstackKeyManager`. Without file access, a Claude-via-MCP voice loses that capability. The other 5 voices are inherently text-only (local models, plus Kimi via gateway has no file-access mode), so they get the inlined-prompt-only treatment.

Final structure:
- **Claude**: Agent tool → `colosseum-spec-adversary` agent (via general-purpose fallback per the v0.2 finding); full tool access
- **Other 5**: MCP fan-out with full text inlined

## Stop condition

Verdict SURVIVES iff:
- Zero critical findings across all 6 voices, OR
- All critical findings are explicitly out-of-scope per the intent doc's Non-Goals / Trust Boundaries (i.e., known-and-accepted gaps)

Anything less, revise and re-run.

## Expected output paths

```
verified-rcv/.colosseum/attacks/intent-revised-<ISO-timestamp>/
├── meta.md
├── claude.md                          # via Agent subagent
├── gateway-kimi-k2-6.md
├── local-mistral-small-4-119b-2603.md
├── local-qwen3.6-27b-mlx.md
├── local-google-gemma-4-26b-a4b.md
├── local-goedel-prover-v2-32b.md
└── synthesis.md
```
