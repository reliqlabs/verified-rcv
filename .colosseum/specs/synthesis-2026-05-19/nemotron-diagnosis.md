# Nemotron-3-120b-a12b stall diagnosis (v9, 2026-05-19)

## What stalled

Opencode session `ses_1c073f0a7ffeCWEuHdq5rqvYYf`, voice
`cloudflare-100-cf-nvidia-nemotron-3-120b-a12b`, step 19. Dispatch killed
it at the 2400s external timeout. Both retries hit the same wall.

## DB-side evidence (the hung message)

Last message `msg_e3f948f58001BQl9gDPCAKBpBx` persisted four parts then went
silent for 30 minutes:

1. `step-start` snapshot.
2. `reasoning`: 5199 bytes streamed in 16.6s (09:32:46→09:33:03 UTC), ending mid-thought with "Let's write them."
3. `text`: empty content, `start` but no `end`.
4. `tool`: `tool=write`, callID set, `state.status=pending`, `input={}`, `raw=""`.

No further SSE chunks reached opencode after 09:33:03. `step-finish` was
never emitted, so DB `msg.tokens` are all zeros. Opencode parsed a
`delta.tool_calls` header (function name `write`) but never received any
`function.arguments` delta.

## Direct probe results (today)

`/Users/mvid/Development/reliq/verified-rcv/.colosseum/scripts/nemotron_probe.py`
exercises six shapes from hello-world up to a verbatim replay of the
40-message step-19 context (reconstructed from the DB). 15+ runs, payload
sizes 200 B → 201 KB, `reasoning_effort=high`:

- First SSE chunk in 3-6s; reasoning streams as `delta.reasoning` (NOT `reasoning_content`) at ~60 tokens/s.
- `delta.tool_calls` arrives after reasoning, with name then args deltas.
- Inter-chunk gaps: median ~6ms during reasoning; consistent 5-30s pause at the reasoning→tool_calls transition; max observed 28.7s.
- All runs return cleanly. Step-19 replay: 11-66s total, every run completes.

The gateway and model behave correctly today. The Bug 8 fix held (no
124s upstream cap).

## Hypothesis verdict

- **(1) Long reasoning silence**: partial. Reasoning runs to 8 KB+, but each token streams individually and opencode does map `delta.reasoning` to reasoning-deltas. Not the proximate cause.
- **(2) Format mismatch on completion**: ruled out. Only standard `finish_reason` values seen.
- **(3) Genuine slow generation**: ruled out. Max-content probes finish in ~70s.
- **(4) NEW: upstream pause mid-stream with kept-alive TCP.** Best fit. Probes show a consistent 5-30s gap at the reasoning→tool_calls transition. In v9 that pause apparently extended past 30 minutes without the upstream emitting any signal. The gateway has no SSE chunks to forward and emits nothing; the HTTPS connection stays nominally healthy, so no error fires on opencode's side. Same behavior class as Bug 8 (silent upstream timeout) even though Bug 8's specific HTTP 408 signature was fixed.

## Layer

Gateway + upstream Workers-AI. Not opencode-side: replay with verbatim
opencode context completes; opencode correctly maps `delta.reasoning`. Not
model-side per se: when the model returns at all, it returns clean
structured output.

## Fix (gateway-maintainer-actionable)

1. **Heartbeat watchdog for nemotron requests.** If upstream is silent for >120s, the gateway should either inject a `: keepalive` SSE comment so consumers know the stream is alive, OR close the connection with a recognizable error envelope (e.g. `{"errors":[{"message":"upstream silent for 120s","code":3047}]}`) so opencode's parser can surface it instead of hanging.

2. **Per-route silent-deadline policy for `@cf/nvidia/nemotron-3-120b-a12b`.** Workers AI appears to silently park requests on queue contention. Either pass `cf-aig-request-timeout` (e.g. 600s) or document the silent-pause ceiling so consumers can configure their own deadline.

3. **Synthetic keepalive deltas during long pauses.** OpenAI-compat clients expect a chunk per ~30s. The gateway could synthesize empty `delta` chunks during upstream pauses >30s to keep parsers awake.

## Notes for v10/v11

- Keep `--variant high`; probes show no latency penalty.
- Agent body and message shape are not at fault; both replay cleanly.
- The failure is intermittent. Until the gateway publishes a heartbeat, a dispatch-side wallclock check (kill if no DB part has been updated in >180s) is a reasonable backstop.

## Artifacts

- `.colosseum/scripts/nemotron_probe.py` (probe with shapes: small / reasoning / tools-t1 / tools-t3 / tools-write / tools-write-big / replay-step19).
- DB: `~/.local/share/opencode/opencode.db`, session `ses_1c073f0a7ffeCWEuHdq5rqvYYf`, message `msg_e3f948f58001BQl9gDPCAKBpBx`.
