# Multi-model gateway — bug report

- **Tested**: 2026-05-14
- **Endpoint shape**: OpenAI-compatible (`/models`, `/chat/completions`, `Authorization: Bearer …`)
- **Tester**: Colosseum methodology agent, smoke-testing for the `external-model-mcp` `query_gateway` integration
- **Test method**: `curl` from the local machine; OpenAI-format chat completions with a trivial `pong` prompt at `max_tokens: 15–20`

The URL and access token are deliberately not included in this report; they live in `.env` only.

## Bug 1 — Gemini routes check the OpenAI provider's BYOK slot

**Reproducer:** request any Gemini model via `/chat/completions`:

```http
POST /chat/completions
Content-Type: application/json

{ "model": "gemini-2-5-flash", "messages": [...] }
```

**Response:**

```json
{ "error": { "message": "no OpenAI BYOK key configured", "type": "invalid_request_error", "code": "no_provider_available" } }
```

**Why this is a bug:** Gemini routes should look up the Google provider's BYOK slot, not the OpenAI provider's. The error message names OpenAI explicitly. Adding a Google BYOK key will not fix this until the routing table is corrected — the gateway is asking the wrong provider for credentials.

**Confirmed on:**
- `gemini-2-5-flash`
- `gemini-2-5-flash-native-audio-latest`
- `gemini-2-5-pro`
- `gemini-3-1-flash-lite`
- `gemini-3-1-flash-lite-preview`
- `gemini-3-1-flash-live-preview`
- `gemini-3-1-flash-tts-preview`
- `gemini-3-1-pro-preview`
- `gemini-3-1-pro-preview-customtools`

All 9 listed Gemini variants return the same OpenAI-BYOK error.

**Suggested fix:** check the provider-routing table for the `gemini-*` prefix family; ensure the lookup key is the Google provider's BYOK slot, not the OpenAI slot.

## Bug 2 — "ReadableStream is disturbed" on all non-Claude/non-OpenAI providers

**Reproducer:** request any of the following models:

| Provider family | Tested models |
|----|----|
| Meta / Llama | `llama-3.3-70b-instruct`, `llama-4-maverick`, `llama-4-scout` |
| Mistral | `mistral-large-latest`, `codestral-latest`, `mistral-medium-3`, `mixtral-8x22b` |
| DeepSeek | `deepseek-chat`, `deepseek-v3`, `deepseek-r1` |
| xAI | `grok-2`, `grok-3`, `grok-4` |
| Cohere | `command-r-plus`, `command-a-2025` |
| Qwen | `qwen-3-coder`, `qwen3-72b-instruct` |
| Microsoft | `phi-4` |
| Amazon | `nova-pro` |
| OpenAI (only `gpt-5`) | `gpt-5` |

**Response:**

```json
{ "error": { "message": "This ReadableStream is disturbed (has already been read from), and cannot be used as a body." } }
```

**Why this is a bug:** this is a JavaScript-runtime error from attempting to read a `Response` body twice. It indicates the gateway is *attempting to route* the request (so the model name is recognized), the upstream call fails (probably "model not found" or "no BYOK key for that provider"), and the gateway's error-handling code crashes trying to read the upstream error response. The actual upstream error is hidden behind the JS stream error.

`gpt-4o-mini` / `gpt-4.1` / `o4-mini` return a clean "OpenAI BYOK required" error, but `gpt-5` returns the stream error — suggesting `gpt-5` is recognized as a different provider/route than the other OpenAI-prefixed models.

**Suggested fix:** in the gateway's upstream-response handling, either (a) clone the Response before reading, or (b) buffer the body once before attempting to parse-or-rethrow. The fix is on the gateway-internal-code side; the consumer cannot work around it.

## Not-a-bug: model-listing changes are intentional operator action

Earlier draft of this report classified the changing model set across test sessions (11 → 9 → 7 over a few hours) as a bug. **The operator clarified that this is deliberate curation** — models are added and removed from the gateway profile as part of normal configuration. The "drift" was the consumer (this report's author) lacking visibility into operator intent, not a gateway misbehaviour.

The consumer-side discipline this implies still stands: **pin specific model IDs in reproducibility-sensitive contexts** (e.g., adversarial-pass artifacts that need to be re-runnable). The MCP's `query_gateway` and `query_*` tools accept an explicit `model` parameter for exactly this reason.

No methodology consequence as a gateway bug; methodology consequence as a discipline: when a Colosseum compose-ledger records a multi-model adversarial result, the **specific model IDs invoked** must be part of the ledger entry, not just the gateway endpoint identifier.

## Resolution status (2026-05-14 evening retest)

Both bugs above were resolved by the operator the same day:

- **Bug 1**: Gemini routes now use the correct provider slot. `gemini-2-5-flash` and `gemini-3-1-flash-lite` route cleanly and return `pong` via `message.content`. The pro Gemini variants were temporarily blocked by Google free-tier quota (`RESOURCE_EXHAUSTED` 429) and were subsequently removed from the listing by the operator.
- **Bug 2**: `ReadableStream is disturbed` no longer appears on the current model set. Three new providers were added in its place and all route cleanly: `glm-4-7-flash` (Zhipu), `gpt-oss-120b` (open-source GPT), `kimi-k2-6` (Moonshot).

**Current working surface (7 models across 4 provider families):**

| Model | Tier | Family | Verified |
|----|----|----|----|
| `claude-opus-4-7` | advanced | Anthropic | ✓ |
| `claude-sonnet-4-6` | advanced | Anthropic | ✓ |
| `gemini-2-5-flash` | basic | Google | ✓ |
| `gemini-3-1-flash-lite` | basic | Google | ✓ |
| `glm-4-7-flash` | basic | Zhipu | ✓ (reasoning model; needs `max_tokens ≥ ~200`) |
| `gpt-oss-120b` | standard | OSS-GPT | ✓ (reasoning model) |
| `kimi-k2-6` | standard | Moonshot | ✓ (reasoning model) |

This is the family-diverse fan-out surface Colosseum's multi-model adversarial work was designed around.

## Bug 3 — Gateway-wide ~240s upstream cap (initially mis-diagnosed as kimi-specific)

**Reproducer:** invoke `kimi-k2-6` on the gateway with `max_tokens ≥ ~12000` on a ~22K-token prompt:

```http
POST /chat/completions
{ "model": "kimi-k2-6",
  "messages": [<22K-token system+user prompt>],
  "max_tokens": 16384 }
```

**Response** (consistent across 3 attempts on 2026-05-14 with max_tokens ∈ {16384, 16384, 32768}):

```json
{ "errors": [{ "message": "AiError: AiError: Request timeout (<uuid>)", "code": 3046 }],
  "success": false, "result": {}, "messages": [] }
```

with HTTP status 408 and elapsed ∈ {238.6, 239.3, 239.8} seconds — a tight ~240s ceiling.

**Discriminator probe** (`.colosseum/scripts/gateway_timeout_probe.py`, 2026-05-14): same 22K-prompt + max_tokens=16384 against three other gateway models:

| Model | Status | Elapsed | Notes |
|---|---|---|---|
| `glm-4-7-flash` (Zhipu, reasoning) | OK | 152.6s | finish=length, 16K tokens generated |
| `gpt-oss-120b` (OSS, reasoning) | OK | 47.7s | finish=stop, 5K tokens generated |
| `claude-sonnet-4-6` (Anthropic, non-reasoning) | HTTP 524 | 127.6s | distinct failure mode — see Bug 4 |

**Initial mis-diagnosis (now corrected)**: this table looked like it ruled out a gateway-wide cap, since glm and gpt-oss completed under 16K. But a follow-up `glm-4-7-flash @ max_tokens=24576` retry on 2026-05-14 (same 22K-token prompt) also hit HTTP 408 at 238.7s — same failure shape, same ~240s elapsed. Glm at 16K only worked because it happened to finish in 152s; at 24K it needed >240s and tripped the same cap. **The 240s ceiling is gateway-wide** (very likely a Cloudflare proxy timeout on the inbound side), not per-route. Different upstream models just have different generation speeds, so different `max_tokens` settings fit within the same budget:

| Model | `max_tokens` headroom under ~240s @ 22K prompt | Source |
|---|---|---|
| `gpt-oss-120b` | high — finished 16K in 47s | discriminator probe |
| `glm-4-7-flash` | ~16K is safe; 24K tips into 408 | discriminator + retry |
| `kimi-k2-6` | ~8K is safe (133s); 16K tips into 408 | three reproductions + retry |

**Why this is a bug:** kimi-k2-6 itself supports up to 256K context, glm-4-7-flash supports long outputs, etc. The 240s ceiling is an inbound-proxy deadline, not a model limit. Consumers wanting long-output adversarial reports must either accept truncated structured content per route, or split into multiple shorter calls and stitch them.

**Consumer-side workaround in place:** the verified-rcv `fan_out_dispatch.py` wrapper currently uses `max_tokens=8192` for kimi-k2-6 and accepts truncated structured output (consistently cut off mid-attack-list — requires a follow-up "continue your prior response" round to extract a verdict line).

**Suggested fix:** raise the deadline on the Moonshot/kimi route specifically to align with other gateway routes (`gpt-oss-120b` and `glm-4-7-flash` both completed inside 160s on the discriminator), or expose the deadline as a per-request header so consumers can opt in to longer completions when they need them.

**Methodology consequence:** for the family-diversity slot kimi-k2-6 covers (Moonshot, non-Western reasoning), there is currently no clean substitute at ≥ 12K visible output budget on a single-request dispatch shape. `glm-4-7-flash` (Zhipu) is the closest swap if a non-Western reasoning voice is needed and full-length output is required from a single inlined call.

**Structural mitigation discovered 2026-05-16 — subagent dispatch (OpenCode + spec-adversary agent + tool-use ReAct loop):** the 240s cap is **per HTTP request**, not per logical model invocation. When the same kimi-k2-6 voice is dispatched through OpenCode with the `spec-adversary` agent and `read,grep,glob` tool permissions, the ReAct loop breaks one "model call" into N independent HTTP requests:

- Turn 1: model emits `tool_use(read)` → HTTP request 1 (~1-5s) → file content cached locally by OpenCode
- (OpenCode executes Read tool locally — no HTTP hop)
- Turn 2: model emits Markdown analysis text → HTTP request 2 (up to ~239s before hitting the cap)
- Optional Turn 3+: model decides to grep / re-read / continue analysis → more HTTP requests

Observed in calibration run 2026-05-16 — `pattern-b-v0.3.0-*` run dir under verified-rcv `.colosseum/attacks/`:

- kimi-k2-6 produced 6,770–15,065 char structured reports per slice
- Total session times of 200–370s **with no individual HTTP request hitting the cap**
- "First attempt fails, retry succeeds" pattern (observed at 16K output cap) **disappeared entirely** when output cap raised to 65,536+ — the model writes its complete thorough response in one turn 2 and finishes naturally rather than getting truncated

The structural mitigation is the ReAct multi-turn shape, not the slice-targeting (the model often reads the whole intent doc even when asked to target a slice). What slice-targeting does buy you is *focused* attention; the cap-avoidance comes from the multi-turn loop.

**Recommended consumer-side dispatch for Bug-3-prone voices** (kimi-k2-6, glm-4-7-flash, gpt-oss-120b):
1. Use OpenCode with the `spec-adversary` agent (file in `.opencode/agent/spec-adversary.md`), not bare HTTP fan-out
2. Set `limit.output ≥ 65,536` (recommend `131,072`) in opencode.jsonc per-model so the agent's Turn-2 response budget never hits the cap mid-report
3. Sequential per-voice dispatch (parallel runs aggravate truncation; observed in calibration when 6 voices ran concurrently — first attempts failed on most slices, succeeded sequentially)
4. Retain a single retry in the orchestrator for incidental sampling flakes; expect retries on ~10% of calls vs ~60% under inlined dispatch

## Bug 4 — Anthropic gateway route (both `claude-opus-4-7` and `claude-sonnet-4-6`) returns Cloudflare 524 at ~127s

**Reproducer:** invoke either Anthropic model on the gateway with `max_tokens=16384` on a ~22K-token prompt:

```http
POST /chat/completions
{ "model": "claude-{opus|sonnet}-4-...",
  "messages": [<22K-token system+user prompt>],
  "max_tokens": 16384 }
```

**Response (both models, 2026-05-14 — two separate dispatches):**

| Model | HTTP | Elapsed | Body |
|---|---|---|---|
| `claude-sonnet-4-6` | 524 | 127.6s | Cloudflare HTML "burnt.com \| 524: A timeout occurred" |
| `claude-opus-4-7` | 524 | 127.7s | Same Cloudflare HTML page |

HTTP 524 is Cloudflare's "A timeout occurred — origin web server didn't respond" code. The body is the Cloudflare branded error template, not a JSON error envelope. The near-identical 127s elapsed across both models suggests a single shared Cloudflare proxy timeout on the Anthropic upstream route.

**Why this is a bug:** the Anthropic route appears to have a ~127s Cloudflare-proxy ceiling that's much tighter than other routes on the gateway (kimi: ~240s; glm: ~152s; gpt-oss: ~48s). Both opus and sonnet should be capable of completing a 16K-max_tokens response within a generous window — the issue is the route's proxy deadline, not the model. Returning Cloudflare HTML rather than the gateway's normal `{"errors": […]}` envelope also breaks any JSON-only consumer.

**Important note on smoke-test misleading earlier conclusion:** earlier in this document (Bug-2 retest), `claude-opus-4-7` was verified working at `max_tokens=20`. That tiny ping succeeded in <10s. The 127s ceiling is therefore not visible until prompts get long enough or max_tokens get high enough for total time to push past ~127s. Smoke-tests with trivial prompts do not exercise this failure mode.

**Consumer-side workaround:** for the Anthropic-via-gateway slot on long prompts, no clean workaround exists at >127s total time. Specifically:
- If the goal is "a Claude voice for adversarial work", run Claude via the Agent subagent locally (works, with full tool access). The gateway adds nothing here.
- If the goal is "a different inference seat from the Agent subagent" for ensemble diversity, the gateway is currently unusable for long-output dispatches; substitute another Anthropic-adjacent or Claude-family voice from a non-gateway path, or accept ensemble redundancy.
- For non-adversarial short queries, `claude-opus-4-7`/`claude-sonnet-4-6` work fine through the gateway under the ~127s ceiling.

**Suggested fix:** (a) raise the Cloudflare proxy timeout for the Anthropic route to match the kimi route (~240s), *or* (b) wrap origin timeouts in the gateway's standard JSON error envelope so consumers don't have to parse HTML, *or* (c) both.

**Methodology consequence:** the gateway's failure modes are heterogeneous per provider route — *what's safe at one model is unsafe at another even for the same shape*. Pin model ids in `meta.md`, record HTTP status + elapsed per dispatch, and treat the failure-shape itself as part of the adversarial-pass artifact. For "Claude voice in a multi-model ensemble" specifically, prefer Agent-subagent dispatch over gateway dispatch until Bug 4 is resolved.

## Bug 5 — `gpt-oss-120b` transient outage on `/colosseum/v1` route 2026-05-16

**Symptom (observed 2026-05-16 ~12:14Z):** dispatching `gpt-oss-120b` against `https://ai-gateway.burnt.com/u/c5nksc/colosseum/v1/chat/completions` returned:

```
HTTP 400: {"errors":[{"message":"AiError: No such model: No such model gpt-oss-120b or task
(<request-id>)","code":5007}],"success":false,"result":{},"messages":[]}
```

Elapsed: ~4s (the gateway rejected fast, didn't proxy to upstream).

**Resolution:** operator confirmed transient outage, not a permanent removal. The route was restored later 2026-05-16 (~14:00Z). Subagent dispatch via the `/openai/v1` route via OpenCode succeeded on the same model during the outage window, so the failure was scoped to the `/colosseum/v1` route specifically.

**Context:** `gpt-oss-120b` was a working voice in the 2026-05-14T200029Z run (47s @ 16K max_tokens, finish=stop, full content). On 2026-05-16 ~12:14Z the same model id against the same route + key returned "No such model" for an unspecified maintenance / provisioning window. The outage resolved without consumer action.

**Cross-reference:** the OpenCode provider config at `~/.config/opencode/opencode.jsonc` still lists `gpt-oss-120b` under the `burnt` provider, but pointed at a DIFFERENT route: `https://ai-gateway.burnt.com/u/c5nksc/openai/v1` (note `/openai/v1` vs `/colosseum/v1`). With different API keys. Whether `gpt-oss-120b` is still reachable on the `/openai/v1` route is an open empirical question (not tested in this run).

**Why this is still a bug (even after resolution):** the operator-curated model roster on the `/colosseum/v1` route can have transient outages, and during the outage there's no consumer-facing signal distinguishing "permanently removed" from "temporarily down." No health-check API enumerates "what's currently routable" or "estimated time to restore." The 2026-05-14 dispatch's `meta.md` recorded `gpt-oss-120b` as part of the family-diversity panel; the 2026-05-16 dispatch's manifest recorded it as `error`. A consumer cold-starting an adversarial run during the outage cannot distinguish the two cases, and is forced to either skip the voice or wait an unknown duration.

**Consumer-side workarounds:**
- **Health-probe before dispatch.** Add a startup ping against the configured model list; surface missing routes before kicking off a long fan-out.
- **Re-attempt via the `/openai/v1` route** (different proxy, different key). If `gpt-oss-120b` is reachable there, the family-diversity panel can re-include it through OpenCode rather than the colosseum MCP.
- **Substitute another OSS-frontier voice** from LM Studio (e.g., `qwen/qwen3-coder-next` at 80B) for the family-diversity slot. Less ideal — qwen and gpt-oss are different families, and the substitution doesn't preserve the OpenAI-OSS lineage that gpt-oss represents.

**Suggested gateway-side fix:**
- (a) publish a `GET /models` endpoint enumerating currently-routable model ids per provider, OR
- (b) when a model id is removed, return a structured 410 Gone with a `Sunset` header rather than a generic 400, OR
- (c) maintain a deprecation window — keep the route returning 200s on small requests with a deprecation warning in `usage.deprecation_notice` for some period before removing.

**Methodology consequence:** pin model ids in `meta.md` per dispatch (already a stable-ask discipline — Ask H in v0.3 back-port). When a model is dropped between dispatches, the manifest's `voices[*].metadata.model` field is the reproducibility anchor. Future health-probe tooling (post-Pattern-B) should run before fan-out begins, not as a post-hoc check.

## Operational notes for the consumer (still load-bearing)

1. **`max_tokens` should be ≥ 1024 for reasoning models** (`glm-4-7-flash`, `gpt-oss-120b`, `kimi-k2-6`). Below ~150 tokens, content ends up empty because all the budget went to hidden reasoning. The MCP's default `max_tokens=4096` is fine; just don't set it too low explicitly when targeting these.

2. **Calibrate `max_tokens` per upstream model speed to fit under the gateway-wide ~240s cap** (Bug 3). Empirical 2026-05-14 values for a ~22K-prompt: `kimi-k2-6` ≤ 8K (133s); `glm-4-7-flash` ≤ 16K (152s) but 24K blows the cap (239s); `gpt-oss-120b` 16K finished in 47s (much higher should be safe). Numbers shift with prompt length; pin observed elapsed in your meta.md.

3. **`claude-sonnet-4-6` route appears flaky at long prompt + high max_tokens** (Bug 4). Prefer `claude-opus-4-7` for the Anthropic-via-gateway slot until resolved.

3. **Pin model IDs in adversarial-pass artifacts.** The operator curates the model set deliberately; a pass using `kimi-k2-6` today should record that specific id so future re-runs can detect if it's still available.

4. **Reasoning-model output now lands in `message.content`** (the gateway post-processes), so no special parsing is needed at the MCP layer. Earlier-observed `reasoning_content`-only responses were max-tokens-budget issues, not response-shape issues.

## Consumer-side configuration in place

`external-model-mcp` v0.2 (this commit) loads the gateway URL and key from `.env` only (gitignored), validates that both are present before activating the gateway provider, and returns clear configuration errors when either is missing. The MCP does not hardcode the gateway URL or any operator-identifying information; the code is transferable to any OpenAI-format gateway by setting the two env vars.
