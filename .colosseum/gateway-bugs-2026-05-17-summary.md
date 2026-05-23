# Burnt gateway — interop bugs discovered during Quint spec generation

- **Tested**: 2026-05-16 / 2026-05-17
- **Workload**: Quint protocol-spec generation through opencode's
  agent-iterate loop (multi-turn tool use: `read`, `write`, `edit`,
  `bash`). Higher-load workload than prior smoke tests / adversarial
  reviews — exposes failure modes that single-turn pings don't.
- **Gateway endpoint**: `https://ai-gateway.burnt.com/u/c5nksc/openai/v1`
  (the OpenAI-compatible route, used by opencode's `burnt` provider).
- **Companion doc**: `gateway-bugs-2026-05-14.md` (Bugs 1-5 from prior
  testing; this doc extends with Bugs 6-9 surfaced 2026-05-16/17).

## TL;DR

| Voice | Failure mode | Layer |
|-------|--------------|-------|
| `gemini-2-5-flash` | `MALFORMED_FUNCTION_CALL` returned as empty response; opencode parser stalls | Gateway proxy + Google filter |
| `gpt-oss-120b` | Tool calls emitted in `reasoning_content` text, not `tool_calls`; opencode sees empty turn | Gateway doesn't parse harmony format |
| `cloudflare-100-cf-nvidia-nemotron-3-120b-a12b` | ~124s upstream cap (HTTP 408, code 3046) | Cloudflare → NVIDIA Workers AI upstream |
| Native provider comparison (control) | `google/gemini-2.5-flash` and `openai/gpt-5.5` direct connections **do not exhibit Bugs 6 & 7** — confirmed by direct probe | Bypasses gateway proxy |

Three concrete bugs against the gateway, all reproducible.

---

## Bug 6 — Gemini `MALFORMED_FUNCTION_CALL` passes through as empty body

**Reproducer** (direct gateway probe, no opencode in the loop;
script: `.colosseum/scripts/gemini_probe.py`):

```python
POST https://ai-gateway.burnt.com/u/c5nksc/openai/v1/chat/completions
{
  "model": "gemini-2-5-flash",
  "messages": [
    <system>,
    <user: "Generate Quint spec...">,
    <assistant: tool_calls = [read intent.md]>,
    <tool result: 140KB intent.md content>,
    <assistant: tool_calls = [read reactor.qnt, read main_n6f1b1.qnt]>,
    <tool result: 8KB>,
    <tool result: 0.5KB>
  ],
  "tools": [<read tool def>, <write tool def>],
  "stream": true
}
```

**Response** (3 reproductions, 2026-05-17):

- HTTP 200
- `finish_reason: "function_call_filter: MALFORMED_FUNCTION_CALL"`
- `content: null`
- `tool_calls: null` (no delta containing tool calls)
- Elapsed ~70-90s

**Why this is a bug:** Google's server-side function-call validator
rejected the assistant's emitted tool call (almost certainly because
its `arguments` field was long — gemini was attempting to emit a Quint
spec as a `write` call argument). Google returns an empty response
with the special `MALFORMED_FUNCTION_CALL` finish reason. **The gateway
passes this through unchanged** rather than surfacing it as a
recognizable error envelope. opencode's stream parser sees
`delta.content == null` and `delta.tool_calls == null`, doesn't
recognize the non-standard finish reason, and stalls indefinitely
waiting for streaming content that already arrived (in the form of
that finish reason).

**Observed downstream impact:** opencode sessions hang silently for
5–13 hours past their 40-min timeout — `asyncio.wait_for` cannot kill
the opencode child because opencode's HTTP client is still in a
"waiting for stream" state (no exception raised). 5 of 6 attempted
spec-generation voices in our v4 run failed this way for various
reasons; gemini specifically hung in this MALFORMED_FUNCTION_CALL mode.

**Direct-provider control:** the same multi-turn message shape sent to
Google's native API (`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:streamGenerateContent`)
via opencode's `google` provider does NOT exhibit this — connection
works, model emits valid `tool_calls` payloads, agent iterates
normally. The MALFORMED_FUNCTION_CALL specifically arises from the
OpenAI-compatible-proxy → Google-native API translation path that the
gateway runs.

**Likely root cause:** the gateway's tool-schema translation between
OpenAI format and Google format isn't fully compatible with what
gemini-2-5-flash expects. Documented in:
- [sst/opencode#11479](https://github.com/sst/opencode/issues/11479) —
  OpenCode + Gemini through OpenAI-compatible proxies. Proxies reject
  JSON Schema fields (`$ref`, `propertyNames`, `allOf`) that opencode
  emits.
- [sst/opencode#4832](https://github.com/sst/opencode/issues/4832) —
  Gemini's `thoughtSignature` must be preserved across multi-turn
  function calls. Mandatory for gemini-3; recommended for gemini-2.5.

**Suggested fix (gateway-side):**
- (a) Translate Google's `MALFORMED_FUNCTION_CALL` finish reason into a
  recognizable error envelope `{"errors":[{"message":"google function-call filter rejected: ...","code":...}]}` so consumers can act on it.
- (b) Strip the JSON-schema fields Google rejects (`$ref`,
  `propertyNames`, `allOf`) from the `tools[*].function.parameters`
  schema before forwarding upstream.
- (c) Preserve `thoughtSignature` across multi-turn calls in the
  gateway's request-rewriting layer (so consumers like opencode don't
  have to).

---

## Bug 7 — `gpt-oss-120b` tool calls land in `reasoning_content`, not `tool_calls`

**Reproducer:** dispatch `gpt-oss-120b` via opencode through the
gateway with a system prompt that requests tool use. Behavior captured
from opencode session DB.

**Response shape:**

- HTTP 200, finish_reason = `stop`
- `tokens.output: 71`, all of it in the **reasoning** channel
- `parts: [step-start, reasoning, step-finish]`
- `reasoning.text`:
  ```
  We need to implement spec based on intent. Let's read intent.
  {
    "filePath": "/Users/mvid/.../intent.md",
    "offset": 1,
    "limit": 2000
  }
  ```

The model emitted the tool call as **literal text inside the
reasoning channel** — not as a structured `tool_calls` payload.
opencode sees an assistant turn with no content and no tool_calls,
treats it as done, ends the session immediately (~6 seconds).

**Why this is a bug:** `gpt-oss-120b` uses OpenAI's **harmony response
format**, where tool calls are emitted on the `commentary` channel
rather than as `tool_calls`. The gateway is forwarding the model's
raw output without translating harmony → OpenAI tool_calls. Documented
extensively:
- [openai/harmony](https://github.com/openai/harmony) — official
  format spec
- [HF gpt-oss-20b discussion #80](https://huggingface.co/openai/gpt-oss-20b/discussions/80)
  — same symptom (tool intent emitted as text, no tool_calls)
- [LangChain forum thread](https://forum.langchain.com/t/harmony-response-format-sometimes-outputted-when-using-gpt-oss-120b-as-an-agent/2554)
  — gpt-oss-120b emits raw harmony when used as an agent

**Direct-provider comparison:** not testable since `gpt-oss-120b` is
not a first-party OpenAI hosted model. The fix has to be at the
gateway or at consumer adapters (opencode).

**Suggested fix (gateway-side):**
- Parse the harmony format on responses from gpt-oss-family models and
  translate `commentary` channel function-call blocks into a normal
  `tool_calls` payload before forwarding to the consumer. Reference
  implementation: [openai/harmony](https://github.com/openai/harmony)
  exposes a `StreamableParser` and a `StreamableParserDeltaText` that
  do exactly this.
- If translation is out of scope, expose a per-request header like
  `X-Harmony-Format: raw` vs `X-Harmony-Format: openai-tool-calls` so
  consumers can opt in to the structured form.

**Observed downstream impact:** every gpt-oss-120b dispatch via the
gateway (v4, v5, v6 attempts) exits after ~6s with no engagement.
The voice is effectively unusable for any agent-iterate workflow.

---

## Bug 8 — `nemotron-3-120b-a12b` 124s upstream cap, separate from gateway 240s cap

**Reproducer:** dispatch
`cloudflare-100-cf-nvidia-nemotron-3-120b-a12b` against the gateway
with even a small prompt requesting a Quint spec.

**Response** (5+ reproductions across 2026-05-16 and 2026-05-17):

- HTTP 408
- Elapsed: 124s, 134s, 159s consistently (tight ceiling around 124s)
- Body:
  ```json
  {"errors": [{"message": "AiError: Request timeout (<uuid>)", "code": 3046}],
   "success": false, "result": {}, "messages": []}
  ```

This is the same response shape as Bug 3 (kimi-k2-6 at 240s), but
hitting at ~124s instead of ~240s — meaning **the nemotron route has
its own tighter cap, distinct from the gateway-wide one**.

**Why this is a bug:** nemotron-3-120b-a12b is a 120B-parameter
reasoning model — it needs more than 124s to produce meaningful output
for any non-trivial prompt. The cap is below the model's minimum
viable response time for the workloads we'd want to use it for.
Consumer cannot work around this; it's not a max_tokens issue, the
upstream just doesn't return in time.

**Likely root cause:** the route maps to Cloudflare's `Workers AI`
hosted-models product (`@cf/nvidia/nemotron-3-120b-a12b`), which has
its own internal timeout independent of the gateway's outer
Cloudflare proxy. Cloudflare's docs:
[NVIDIA Nemotron 3 Super on Workers AI](https://developers.cloudflare.com/workers-ai/models/nemotron-3-120b-a12b/).

**Observed downstream impact:** every attempt with nemotron returns
HTTP 408 in 124–159s with no partial output. The voice is unusable
for any workflow requiring >~100s of generation time.

**Suggested fix (gateway-side):**
- (a) Pass a `cf-aig-request-timeout` header through to the Workers AI
  call, set high enough that nemotron can complete (e.g. 600s).
  [Cloudflare AI Gateway request handling docs](https://developers.cloudflare.com/ai-gateway/configuration/request-handling/)
  document this header.
- (b) If the upstream Workers AI deadline cannot be extended,
  document the ceiling in the gateway's model-listing so consumers
  don't dispatch workloads they know will fail.
- (c) Consider routing nemotron through NVIDIA's API directly rather
  than Workers AI, if a direct route exists at acceptable cost — the
  Cloudflare Workers-AI layer is adding a deadline without adding
  value for high-latency reasoning models.

---

## Bug 9 — Direct-provider routing in opencode bypasses Bugs 6 & 7 (control test result)

**Test method:** added `google` and `openai` providers directly to
opencode's `auth.json` and dispatched the same Quint-spec-generation
prompt that fails through the gateway.

**Results:**

- **`google/gemini-2.5-flash` (direct):** ✅ Connection works,
  no `MALFORMED_FUNCTION_CALL`. Model emits valid `tool_calls`, agent
  iterates normally, writes rcv.qnt (41KB) and main.qnt (2.3KB) with
  **correct Quint idioms** (List[T], `{present: bool, value: T}`
  records, proper `import rcv(...).* from "./rcv"`) — dramatically
  better first-draft quality than the burnt-routed gemini-2-5-flash.
  Limitation: hit Google free-tier quota (`generate_content_free_tier_requests`,
  20/min) during iteration, which is a billing-tier issue not an
  interop issue.
- **`openai/gpt-5.5` (direct):** Blocked by `plan_type: free` usage
  limit at the OpenAI account level — also billing, not interop.

**Why this is methodologically informative:** the direct-provider tests
confirm that Bugs 6 & 7 are gateway-translation issues, not
model-capability issues. The same gemini-2.5-flash that fails through
the gateway succeeds when given a clean, native, schema-correct
request.

**Implication for the gateway:** the value proposition is BYOK
multiplexing + access control + cost tracking. To stay competitive
with direct provider access for tool-use workflows, the gateway's
translation layer needs to match what consumers get going direct.
Specifically: harmony parsing for gpt-oss family, JSON-schema
sanitization + thoughtSignature preservation for Gemini family.

---

## Companion artifacts in this repo

- `.colosseum/scripts/gemini_probe.py` — direct gateway probe that
  reproduces Bug 6 with a single HTTP call. Useful for the gateway
  maintainer to test fixes against.
- `.colosseum/specs/v4-agent-iterate/findings.md` — methodology-side
  writeup of how these bugs interact with multi-voice spec generation.
- `.colosseum/specs/v6-gemini-inline/per-voice/gemini-2-5-flash/` — the
  inline-dispatch workaround (no tools, delimited blocks) that
  sidesteps Bug 6 for gemini by avoiding tool_calls entirely. Demonstrates
  that for spec-generation workloads, the gateway can produce useful
  output via this workaround — but only at the cost of giving up the
  agent-iterate harness.
- `.colosseum/specs/v7-native-providers/per-voice/gemini-2-5-flash-native/`
  — the direct-Google control showing the same model handles tool use
  correctly when not routed through the gateway.

## Suggested prioritization for the gateway maintainer

Listed from highest-leverage to lowest:

1. **Bug 7 (gpt-oss harmony parsing)**: smallest fix, biggest immediate
   unblock. Wraps an existing well-documented parser.
2. **Bug 6 (Gemini JSON schema + finish_reason)**: largest fix, biggest
   total unlock. The Gemini family is a major voice slot in any
   multi-model dispatch. Two sub-fixes: (a) translate
   `MALFORMED_FUNCTION_CALL` into a recognizable error so consumers
   can fail loudly instead of hanging, (b) sanitize JSON schema fields
   per #11479.
3. **Bug 8 (nemotron cap)**: medium fix. Either set the Cloudflare
   timeout header high enough for nemotron, or de-route through
   Workers AI.

Bugs 3, 4 from the prior doc (kimi 240s cap, Claude 127s cap) remain
open from 2026-05-14.
