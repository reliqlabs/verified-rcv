# Quint multi-voice fan-out — empirical findings

Round 3a, task #16. Round tag `v4-agent-iterate`. Run dates 2026-05-16 / 2026-05-17.

## Question under test

The Colosseum adversarial pass relies on multi-voice fan-out (different
models surface different concerns from the same intent). Round 3a asks:
does the *same* fan-out structure produce useful signal when the artifact
is a Quint spec rather than an adversarial review?

Prior attempts (`v1-fanout`, `v2-fanout`, `v3-with-tc-retry`,
`quint-inline-*`) used single-shot inline-dispatch HTTP calls. None
produced a typechecking spec. The hypothesis the user proposed:

> if the frontiers can't one shot it, have we tried kicking them off in
> an opencode agent that is allowed to iterate? to work on and test
> their spec until they believe it is complete? the claude agent also
> doesn't one shot, it works on it for multiple iterations.

`v4-agent-iterate` tests that hypothesis: same intent, same canonical
examples, same validation harness, but each voice runs inside an
opencode agent with `read/write/edit/bash` permissions and the
canonical `quint-spec-generator` agent body. The agent decides when it
is done.

## Voices attempted

Six voices were dispatched via `quint_spec_dispatch.py` (opencode +
subagent dispatch):

| Voice                       | Outcome              | rcv.qnt | main.qnt | notes |
|-----------------------------|----------------------|---------|----------|-------|
| kimi-k2-6                   | **PASS** (530s)      | 270L    | 11L      | typecheck=0, safety holds, 3/3 witnesses violated |
| gpt-oss-120b                | immediate exit (~6s) | 0L      | 0L       | no engagement; agent body exited without writing |
| nemotron-3-120b-a12b        | hang / partial       | 88L     | 15L      | wrote partial files then stopped; never validated |
| gemini-2-5-flash            | hang / partial       | 172L    | 17L      | wrote partial files then stopped |
| gemma-4-26b-a4b             | hang / partial       | 93L     | 26L      | wrote partial files then stopped |
| qwen3.6-27b                 | hang (empty)         | 1L      | 1L       | wrote stub files only, then idle 20+ min |

Plus the structurally-independent **handcrafted Claude voice** at
`specs/{rcv.qnt, main.qnt}` (also validates).

## Empirical findings

**F1. Agent-iterate dispatch outperforms inline-dispatch for spec
generation.** Across all v1–v3 inline-dispatch runs (12+ voice-attempts)
no model produced a typechecking spec. In v4-agent-iterate one voice
(kimi) produced a fully-validating spec on first attempt. The
difference is not the model — earlier runs included the same kimi
build — it is the harness: agent-iterate gives the model a tool loop
that includes the typechecker, and the model uses it.

**F2. Spec generation is harder than adversarial review for the same
voices.** The v0.3.0 adversarial fan-out got useful output from all six
voices. The same voices, run against the spec-generator agent body,
fall into the table above (1/6 success). Quint's surface area
(unfamiliar idioms, multi-module structure, primed-variable transition
syntax, witness encoding) appears to exceed what frontier models
internalize from one or two canonical examples. Producing a spec is a
write-then-debug task; producing an adversarial review is a
read-then-react task. The two are not comparable difficulty.

**F3. The same harness yields binary outcomes per voice (pass / no
engagement) rather than graded quality.** No voice in v4 produced a
partially-broken spec that the typechecker could meaningfully critique.
Voices either iterated to a passing spec (kimi) or stopped after
writing a stub. There is no middle band where the harness extracts
value from a "trying but failing" voice. This is the opposite of
adversarial review, where every voice produces *some* signal.

**F4. Infrastructure unreliability dominates voice diversity at this
scale.** `asyncio.wait_for` did not enforce the 40-minute
PER_CALL_TIMEOUT against opencode (nemotron ran 93 min past timeout).
`pkill -f opencode` did not kill child processes. Two duplicate
opencode processes for one voice were observed competing for LM Studio.
Cloudflare gateway routing has a 124s upstream cap on at least one
model id (nemotron via gateway) that is not surfaced as a timeout but
as HTTP 408 / error code 3046. These problems are infrastructure-level,
not methodology-level, but they cap how many voices can actually run
in a wall-clock-bounded fan-out.

## What the two-voice comparison is worth

The methodology asks for fan-out so different encodings of the same
intent become visible. We have two structurally-different working
encodings:

- **handcrafted Claude (specs/):** 340 lines total. One ghost variable
  (`ghost_ballots_at_end_at`) for the B2 final-count anchor. Witnesses
  are positive reachability claims phrased as `not(state)`. Type
  layering keeps `rcv.qnt` parameterized by const and instantiates
  through `main.qnt` per the minimmit idiom.
- **kimi (v4/.../kimi-k2-6/):** 281 lines total. Encodes the same S1–S10
  state machine and B1–B10 invariants but with a different structural
  choice for B2 (see `kimi-k2-6/design-notes.md`). Witnesses are
  individually scoped.

The diff between these two is the signal the methodology was supposed
to extract. The methodology produced it. The cost is that we have two
working voices, not six.

## v5-cloud-fill follow-up (2026-05-17)

After F2/F3, attempted gemini-2-5-flash, nemotron-3-120b-a12b, and
gpt-oss-120b a second time under the same agent-iterate harness, on
the hypothesis that the action-oriented body rewrite (the change that
unblocked kimi) might also unblock these. It didn't, and the failure
modes were each distinct and structural:

| Voice | Attempts | Wall-clock per attempt | Root cause |
|-------|----------|------------------------|------------|
| nemotron-3-120b-a12b | 2 | 134s, 159s | Cloudflare gateway 124s upstream cap (HTTP 408, code 3046) — fires before any tool call lands; agent-iterate cannot help because the first turn dies in-flight |
| gpt-oss-120b | 2 | 6s, 6s | Tool call emitted as literal text in reasoning channel (71 output tokens, all reasoning), no OpenAI `tool_calls` payload. opencode sees `finish=stop` with no content, exits clean. Gateway/harmony-format mismatch |
| gemini-2-5-flash | 2 | 20059s, 26536s | `asyncio.wait_for` failed to enforce 2400s deadline against the opencode subprocess (same infra bug as v4 nemotron). Per-voice dir empty — model session never wrote files in 13+ hours |

These outcomes split F4 into three sub-findings:

- **F4a (gateway 124s cap)** is a hard wall for any spec-generation
  voice whose first useful turn requires more than that. Single-turn
  reasoning models routed through the Cloudflare gateway are
  effectively blocked from spec generation.
- **F4b (harmony format)** is a per-model gateway/opencode interop
  issue. gpt-oss-120b's reasoning-channel tool calls aren't surfaced
  as OpenAI `tool_calls`, so opencode never sees the model's intent.
  Fixing this requires either a different gateway transform or a
  different opencode tool-call parser.
- **F4c (asyncio.wait_for)** is local-harness — the 2400s budget is
  not actually enforced. Voices that get into a slow-progress state
  burn unbounded wall-clock.

Net result of v5: zero new validating voices. Fan-out remains at 2:
handcrafted Claude + kimi (via agent-iterate).

## v6 — gemini root-cause diagnosis (2026-05-17)

User pushed: "these models *should* be capable of generating this spec
... iterate on the gemini model, I want to know why it doesn't work."

Three layers, isolated bottom-up:

**Layer 1 — gateway / Google function-call filter (the original hang).**
Direct HTTP probe (`gemini_probe.py`) reproducing opencode's turn-3
message shape returned `finish_reason: function_call_filter: MALFORMED_FUNCTION_CALL`
in 70s, with zero `content` and zero `tool_calls`. Google's server-side
function-call validator rejects gemini-2-5-flash's emitted tool call as
malformed when the call's `arguments` field is long (e.g. an entire
Quint spec as `write` content). The gateway propagates this as an
unusual finish reason; opencode's parser doesn't recognize it, sees an
empty assistant turn, and waits forever for streaming content that
never comes. **This is the v4/v5 hang root cause, gateway-side, not
opencode-side.** v4 productive session passed turns 1-7 because the
individual tool calls there were small (single reads, then a
2310-token write that squeaked under the filter, then 16 small edits);
turn 8 needed another sizable write and tripped the filter.

**Layer 2 — model lineage idiom (under the gateway bug).**
Bypassing tools via inline dispatch (`v6-gemini-inline`), gemini emits
all three files cleanly in 94s. But the spec transplants Rust idioms
into Quint syntax: `Option[T]`, `Vec[T]`, `some()`, `none()`,
`.is_some()`, `.get()`, `.toStr()`, `.toString()`, `if () then`. The
default fix message (gotchas as labeled rules) didn't converge — over
2 rounds gemini invented `type OptionT[T] = SumType[...]` to "fix"
Option, shifting the error from "Option not found" to "OptionT not
found." A prescriptive fix message with concrete substitution rules
(`Option[T]` → `{present: bool, value: T}`, `.is_some()` → `.present`,
etc.) reduced the spec from ~20 errors to one syntactic class
(`if () then` → `if ()`). Fixing that introduced `.toString()` back
into a model-invented `canonical_serialization` helper.

**Layer 3 — domain-modeling overreach.**
After layer 2's fix loop, the remaining errors come from gemini trying
to model byte-level hash semantics (`canonical_serialization`,
`hash_placeholder`) in Quint. This is gemini choosing to over-formalize
the §6.2 domain-separated hash rather than abstract it at the protocol
layer (where the only obligation is "honest registry & enclave-bound
inputs produced this tally"). The model knows Quint roughly but not
the protocol-layer-modeling discipline; it keeps reintroducing
helpers it doesn't have the language fluency to write correctly.

**Net answer: gemini-2-5-flash doesn't land a working Quint spec
because two stacked ceilings compound.** Ceiling 1 (the gateway
MALFORMED_FUNCTION_CALL filter) is fixable by switching to inline
dispatch. Ceiling 2 (Quint idiom + protocol-layer modeling discipline)
is a model-capability limit specific to gemini-2-5-flash. The
prescriptive-fix-message escalation converged from "incoherent spec"
to "spec with a handful of self-introduced helpers that don't
compile" — which means a 3rd voice is achievable with **hand-finish**
of gemini's structural choices, but not with iteration alone.

The methodology-relevant finding: **inline dispatch with a
prescriptive fix message is a real recovery path for gemini-class
models**, but the methodology layer needs to choose between (a) accept
2 voices when fan-out doesn't yield a 3rd, (b) hand-finish a partial
3rd voice to extract its structural choices as diff signal, (c) raise
voice count expectations and accept proportional human-in-loop cost.

Artifacts: `scripts/gemini_probe.py` (gateway probe),
`scripts/gemini_targeted_fix.py` (prescriptive fix turns),
`v6-gemini-inline/per-voice/gemini-2-5-flash/raw-response-fix4-then.md`
(gemini's last delivered spec — closest-to-typechecking).

## Prior art — these are documented opencode/model issues

We hit known problems, not novel diagnostic ground. Folding in the
references so a future session doesn't redo the diagnosis:

**Gemini MALFORMED_FUNCTION_CALL (our layer 1):**

- [sst/opencode#11479](https://github.com/sst/opencode/issues/11479) —
  OpenCode + Gemini through OpenAI-compatible proxies. Proxies reject
  advanced JSON Schema fields (`$ref`, `propertyNames`, `allOf`) that
  opencode emits in tool definitions. Closed as "discussion."
  Workaround the issue author shipped: local Node proxy that strips
  the offending schema fields. **This is most likely our exact bug** —
  burnt is an OpenAI-compatible proxy in front of Google, and the
  tool schema opencode generates for `read`/`write`/`edit`/`bash`
  may include fields Google's filter rejects.
- [sst/opencode#4832](https://github.com/sst/opencode/issues/4832) —
  OpenCode doesn't preserve Gemini's `thoughtSignature` across
  multi-turn function calls. Mandatory for gemini-3-pro; "optional but
  recommended" for gemini-2.5-flash. So *not* the primary cause for
  our 2.5-flash hangs but contributes to degraded reasoning quality
  across tool-call turns. Open.
- [google-gemini/gemini-cli#5705, #12924, #13989, #17303, #5760](https://github.com/google-gemini/gemini-cli/issues) —
  Google's own CLI hits MALFORMED_FUNCTION_CALL with the same
  symptoms. Server-side filter, not client-fixable.
- [langchain-ai/deepagents#417](https://github.com/langchain-ai/deepagents/issues/417) —
  Identical empty-response + MALFORMED_FUNCTION_CALL pattern when
  using Gemini under LangChain Deep Agents. Confirms cross-framework
  scope.

**gpt-oss harmony / reasoning-channel tool calls (our gpt-oss layer):**

- [openai/harmony](https://github.com/openai/harmony) — official
  rendering format. Tool calls go to `commentary` channel, not
  `tool_calls`. Most OpenAI-compatible adapters don't parse it.
- [huggingface gpt-oss-120b discussion #69](https://huggingface.co/openai/gpt-oss-120b/discussions/69) —
  errors in chat template vs spec.
- [huggingface gpt-oss-20b discussion #80](https://huggingface.co/openai/gpt-oss-20b/discussions/80) —
  "tool calling not working as expected" — model emits intent in
  reasoning_content text rather than calling tool. **Identical to our
  diagnosis** (model emitted `Let's read intent.{filePath:...}` as
  literal text in reasoning channel, 71 output tokens, no tool_calls).
- [langchain forum, gpt-oss-120b as agent](https://forum.langchain.com/t/harmony-response-format-sometimes-outputted-when-using-gpt-oss-120b-as-an-agent/2554) —
  beyond ~5 tool calls the model regresses to raw harmony text.

**Cloudflare AI Gateway / nemotron timeout (our nemotron layer):**

- [sst/opencode#16180](https://github.com/sst/opencode/issues/16180) —
  related but distinct: opencode's `prompt_async` endpoint trips
  Cloudflare's 100s edge timeout because the stream() wrapper opens a
  response that never writes. Our nemotron case is the upstream cap
  (NVIDIA Workers AI provider behind the gateway), not the
  prompt_async path.
- [Cloudflare AI Gateway timeout docs](https://developers.cloudflare.com/ai-gateway/configuration/request-handling/) —
  the default 100s ceiling has since been extended; per-request
  `cf-aig-request-timeout` header supported. Burnt's gateway config
  determines whether this is set; our 124s observation is consistent
  with NVIDIA's upstream cap *not* with Cloudflare's edge.

**Net interpretation for the methodology:**

The Quint fan-out exposed the model+harness compatibility frontier:
spec generation is a tool-use-heavy workflow, and tool-use is exactly
where the documented opencode interop issues live. The diagnoses
above are all reproducible in any project — they're not specific to
verified-rcv or to Colosseum. This means **the v0.4 voice-budget
realism finding (F2/F3 above) generalizes: any methodology that
fan-outs spec generation through opencode + a mixed-provider gateway
will yield roughly 1-2 of N voices producing landing-quality
artifacts, with the failures matching the issues above.**

## v0.4 back-port candidates

1. **Spec generation needs an iteration harness contract, not a single
   HTTP contract.** The colosseum-adversarial SKILL.md treats fan-out
   as a single-call abstraction. For spec-class artifacts the
   abstraction has to include "agent loop with the verifier as a tool."
   Add a `spec-generator` dispatch mode distinct from
   `adversary-reviewer`.

2. **Voice-budget realism.** v0.4 should not assume `N voices in =
   N voices out` for spec generation. Recommended planning factor for
   Quint specs at current frontier capability: **expect 1–2 voices in
   3–6 attempts to produce a validating artifact.** Compose pyramid
   weights accordingly.

3. **Infrastructure hardening before scaling voice count.** The async
   timeout + zombie-opencode-process pattern observed here will swamp
   any larger fan-out. Either fix at the dispatch-harness level
   (process-group kill, hard wall-clock budget) or document the manual
   intervention pattern.

4. **Acceptance of binary success.** v0.4 should explicitly accept that
   spec-generator fan-out is allowed to produce "no spec" as the
   outcome from a voice and still count as a valid voice attempt. The
   adversarial-fan-out idiom of "every voice contributes something" does
   not transfer to write-tasks.

## Round 3a #16 status

Two structurally-independent specs available for downstream pyramid
(#18) and ledger (#19) work:

- `specs/rcv.qnt` + `specs/main.qnt` (handcrafted Claude voice)
- `.colosseum/specs/v4-agent-iterate/per-voice/kimi-k2-6/{rcv,main}.qnt`
  (kimi voice via opencode agent-iterate)

The mistral voice was never successfully dispatched (always blocked by
upstream voices hanging). Re-attempting it would cost wall-clock
without expected methodological return given F2/F3. Calling task #16
**complete** as a two-voice comparison; back-porting findings F1–F4 as
candidate v0.4 asks under task tracking.
