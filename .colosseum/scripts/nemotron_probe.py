#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["httpx>=0.27"]
# requires-python = ">=3.11"
# ///
"""
Direct probe of cloudflare-100-cf-nvidia-nemotron-3-120b-a12b via burnt gateway.

Modeled after gemini_probe.py. The Gemini probe disambiguated MALFORMED_FUNCTION_CALL;
this one disambiguates the nemotron silent-stall.

What we already know (from sqlite session ses_1c073f0a7ffeCWEuHdq5rqvYYf):
 - Step 19 began streaming 09:32:41Z.
 - Reasoning streamed for ~22s and ended (5199 bytes).
 - A `text` part with empty content was created.
 - A `tool` part for `write` was created with state.status=pending and empty input.
 - Then nothing more — opencode sat for ~30 minutes until the dispatch timeout killed it.
 - The DB-level msg.tokens are all zeros (the stream never completed metadata).

Diagnostic angles per probe target:
  small      - hello-world streaming sanity
  reasoning  - prompt that should force reasoning_content
  tools-t1   - one-shot tool-call (does it stream tool_calls correctly?)
  tools-t3   - emulate opencode turn 19: multi-turn with prior tool_calls, ask the
               model to issue a fresh `write` tool_call (the hang shape).

For each: capture delta fields verbatim and print byte-by-byte timing. The key
question is what fields appear in deltas: `content`, `tool_calls`, `reasoning_content`,
`reasoning`, or some vendor-specific field. If reasoning streams during the long
silence, hypothesis 1 (long reasoning) is confirmed. If a finish_reason arrives but
opencode can't parse it, hypothesis 2. If only a single tool_call header arrives and
arguments never stream, that's the actual failure mode (a 4th hypothesis: streamed
tool_call arguments truncated).
"""

import asyncio, json, os, sys, time
from collections import Counter
from pathlib import Path
import httpx

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
INTENT = REPO / ".colosseum/intent.md"
REACTOR = Path("/Users/mvid/go/pkg/mod/github.com/cometbft/cometbft@v0.38.21/spec/p2p/reactor-api/reactor.qnt")
MAINQNT = Path("/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/main_n6f1b1.qnt")
REPLICA = Path("/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/replica.qnt")
TYPES = Path("/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/types.qnt")
BUILTIN = Path("/opt/homebrew/lib/node_modules/@informalsystems/quint/dist/src/builtin.qnt")

BASE_URL = "https://ai-gateway.burnt.com/u/c5nksc/openai/v1"
API_KEY = "xc-bd97faa4aab05df1c20993b2d2713b6e73cab1f48e2d020b"
MODEL = "cloudflare-100-cf-nvidia-nemotron-3-120b-a12b"

READ_TOOL = {
    "type": "function",
    "function": {
        "name": "read",
        "description": "Read a file from local disk",
        "parameters": {
            "type": "object",
            "properties": {"filePath": {"type": "string"}},
            "required": ["filePath"],
        },
    },
}

WRITE_TOOL = {
    "type": "function",
    "function": {
        "name": "write",
        "description": "Write content to a file on local disk",
        "parameters": {
            "type": "object",
            "properties": {
                "filePath": {"type": "string"},
                "content": {"type": "string"},
            },
            "required": ["filePath", "content"],
        },
    },
}

BASH_TOOL = {
    "type": "function",
    "function": {
        "name": "bash",
        "description": "Run a shell command",
        "parameters": {
            "type": "object",
            "properties": {"command": {"type": "string"}},
            "required": ["command"],
        },
    },
}


async def stream_call(messages, label, tools=None, timeout=1200, max_tokens=16384):
    """POST chat completion with streaming and full delta tracing."""
    payload = {
        "model": MODEL,
        "messages": messages,
        "stream": True,
        "max_tokens": max_tokens,
    }
    if tools is not None:
        payload["tools"] = tools
    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json",
    }
    print(f"\n=== {label} ===", flush=True)
    print(f"  payload size: {len(json.dumps(payload))} bytes", flush=True)
    print(f"  message count: {len(messages)}", flush=True)
    t0 = time.monotonic()
    first_byte_at = None
    chunk_count = 0
    finish = None

    # Track all distinct delta fields encountered
    delta_field_counts = Counter()
    delta_field_first_seen = {}
    delta_field_last_seen = {}

    accum_content = []
    accum_reasoning = []          # delta.reasoning_content
    accum_reasoning_alt = []      # delta.reasoning (Nvidia/OpenAI-compat variant)
    accum_tool_calls = []
    tool_call_args_by_index = {}
    tool_call_name_by_index = {}

    # For periodic progress reporting during long silences:
    last_chunk_at = t0
    quiet_warn_threshold = 30.0   # seconds without a chunk before we warn

    progress_task = None

    async def progress_reporter():
        """Print a heartbeat while we're streaming so long silences are visible."""
        while True:
            await asyncio.sleep(10.0)
            now = time.monotonic()
            quiet = now - last_chunk_at
            elapsed = now - t0
            print(f"  [+{elapsed:6.1f}s] heartbeat: chunks={chunk_count} quiet={quiet:.1f}s "
                  f"fields={dict(delta_field_counts)} finish={finish}", flush=True)

    try:
        async with httpx.AsyncClient(timeout=timeout) as cli:
            async with cli.stream("POST", f"{BASE_URL}/chat/completions",
                                  json=payload, headers=headers) as r:
                connect_at = time.monotonic() - t0
                print(f"  connected (status={r.status_code}) at +{connect_at:.2f}s "
                      f"headers={dict(r.headers)}", flush=True)
                if r.status_code != 200:
                    body = await r.aread()
                    print(f"  ERROR body: {body.decode('utf-8', 'replace')[:2000]}", flush=True)
                    return

                progress_task = asyncio.create_task(progress_reporter())

                async for line in r.aiter_lines():
                    last_chunk_at = time.monotonic()
                    if not line.strip():
                        continue
                    if first_byte_at is None:
                        first_byte_at = time.monotonic() - t0
                        print(f"  first SSE line at +{first_byte_at:.2f}s: "
                              f"{line[:150]!r}", flush=True)
                    chunk_count += 1
                    if not line.startswith("data: "):
                        # Non-data lines: id:, event:, retry:, comments. Log them.
                        print(f"  [+{(time.monotonic()-t0):.2f}s] non-data SSE: {line[:200]!r}",
                              flush=True)
                        continue
                    data = line[6:]
                    if data == "[DONE]":
                        elapsed = time.monotonic() - t0
                        print(f"  [+{elapsed:.2f}s] [DONE] marker", flush=True)
                        break
                    try:
                        chunk = json.loads(data)
                    except Exception as e:
                        print(f"  [+{(time.monotonic()-t0):.2f}s] non-JSON: {data[:200]!r} ({e})",
                              flush=True)
                        continue

                    choices = chunk.get("choices", [])
                    if not choices:
                        # Usage chunks sometimes come without choices
                        if "usage" in chunk:
                            print(f"  [+{(time.monotonic()-t0):.2f}s] usage: {chunk['usage']}",
                                  flush=True)
                        else:
                            print(f"  [+{(time.monotonic()-t0):.2f}s] no-choices chunk: "
                                  f"{json.dumps(chunk)[:300]}", flush=True)
                        continue

                    choice = choices[0]
                    delta = choice.get("delta", {})

                    # Record every field we see in the delta
                    for k in delta.keys():
                        delta_field_counts[k] += 1
                        if k not in delta_field_first_seen:
                            delta_field_first_seen[k] = time.monotonic() - t0
                            print(f"  [+{delta_field_first_seen[k]:.2f}s] FIRST delta field "
                                  f"`{k}` -> {repr(delta[k])[:200]}", flush=True)
                        delta_field_last_seen[k] = time.monotonic() - t0

                    # Accumulate content
                    if "content" in delta and delta["content"]:
                        accum_content.append(delta["content"])
                    if "reasoning_content" in delta and delta["reasoning_content"]:
                        accum_reasoning.append(delta["reasoning_content"])
                    if "reasoning" in delta and delta["reasoning"]:
                        accum_reasoning_alt.append(str(delta["reasoning"]))
                    if "tool_calls" in delta and delta["tool_calls"]:
                        accum_tool_calls.append(delta["tool_calls"])
                        for tc in delta["tool_calls"]:
                            idx = tc.get("index", 0)
                            if "function" in tc:
                                fn = tc["function"]
                                if "name" in fn and fn["name"]:
                                    tool_call_name_by_index[idx] = fn["name"]
                                if "arguments" in fn and fn["arguments"]:
                                    tool_call_args_by_index.setdefault(idx, []).append(fn["arguments"])

                    fr = choice.get("finish_reason")
                    if fr is not None:
                        finish = fr
                        elapsed = time.monotonic() - t0
                        print(f"  [+{elapsed:.2f}s] finish_reason={fr!r} "
                              f"(full choice: {json.dumps(choice)[:400]})", flush=True)

                total = time.monotonic() - t0
                print(f"  done at +{total:.2f}s. chunks={chunk_count} finish={finish}",
                      flush=True)
                print(f"  delta fields: {dict(delta_field_counts)}", flush=True)
                print(f"  content len: {sum(len(s) for s in accum_content)} chars", flush=True)
                print(f"  reasoning_content len: {sum(len(s) for s in accum_reasoning)} chars",
                      flush=True)
                print(f"  reasoning len: {sum(len(s) for s in accum_reasoning_alt)} chars",
                      flush=True)
                print(f"  tool_call delta chunks: {len(accum_tool_calls)}", flush=True)
                if tool_call_name_by_index:
                    print(f"  tool_call names by index: {tool_call_name_by_index}", flush=True)
                for idx, frags in tool_call_args_by_index.items():
                    joined = "".join(frags)
                    print(f"  tool_call[{idx}] args ({len(joined)} chars): "
                          f"{joined[:300]!r}", flush=True)
                if accum_content:
                    snippet = "".join(accum_content)[:300].replace("\n", "\\n")
                    print(f"  content head: {snippet}", flush=True)
                if accum_reasoning:
                    snippet = "".join(accum_reasoning)[:300].replace("\n", "\\n")
                    print(f"  reasoning_content head: {snippet}", flush=True)

    except httpx.ReadTimeout:
        elapsed = time.monotonic() - t0
        print(f"  TIMEOUT after {elapsed:.1f}s (first_byte_at={first_byte_at}, "
              f"chunks={chunk_count}, finish={finish})", flush=True)
        print(f"  partial delta fields: {dict(delta_field_counts)}", flush=True)
        print(f"  partial content len: {sum(len(s) for s in accum_content)} chars", flush=True)
        print(f"  partial reasoning_content len: {sum(len(s) for s in accum_reasoning)} chars",
              flush=True)
        print(f"  partial reasoning len: {sum(len(s) for s in accum_reasoning_alt)} chars",
              flush=True)
        for idx, frags in tool_call_args_by_index.items():
            joined = "".join(frags)
            print(f"  partial tool_call[{idx}] args ({len(joined)} chars)", flush=True)
    except Exception as e:
        elapsed = time.monotonic() - t0
        print(f"  EXC after {elapsed:.1f}s: {type(e).__name__}: {e}", flush=True)
    finally:
        if progress_task is not None:
            progress_task.cancel()
            try:
                await progress_task
            except asyncio.CancelledError:
                pass


def build_turn3_messages(intent_text, reactor_text, mainqnt_text, replica_text, types_text):
    """
    Approximate opencode step ~19 shape: a long iteration where the model has
    already done multiple reads, attempted writes, and is now being asked to
    fix a typecheck failure and re-write main.qnt with corrected braces.

    The actual step 19 input was 68k tokens. We approximate with a similarly
    shaped multi-turn that includes:
      - system: agent body
      - user: dispatch prompt
      - assistant: read intent
      - tool: intent contents
      - assistant: read 2 examples
      - tool: reactor + main_n6f1b1
      - assistant: write rcv.qnt v1 + write main.qnt v1
      - tool: write results (ok)
      - assistant: run quint typecheck
      - tool: errors (Type alias Int not found; missing '{' at const)
      - assistant: read replica + types
      - tool: file contents
      - user (synthetic): "fix the typecheck errors and re-emit main.qnt"

    Then we want to see whether the next assistant turn streams cleanly.
    """
    return [
        {"role": "system", "content": (
            "You are a Quint spec generator. Read the intent and canonical examples, "
            "then write rcv.qnt and main.qnt to disk. After writing, run quint typecheck. "
            "If errors, debug and rewrite. Tools: read, write, bash."
        )},
        {"role": "user", "content": (
            "VOICE_ID: nemotron-3-120b-a12b\n\n"
            f"INTENT_PATH: {INTENT}\n\n"
            "OUTPUT_DIR: /tmp/probe\n\n"
            "SPEC_FILENAME: rcv.qnt\n\n"
            "Generate the Quint spec for IRV per intent."
        )},
        # Turn 1: read intent
        {
            "role": "assistant", "content": None,
            "tool_calls": [{
                "id": "call_r1", "type": "function",
                "function": {"name": "read", "arguments": json.dumps({"filePath": str(INTENT)})},
            }],
        },
        {"role": "tool", "tool_call_id": "call_r1", "content": intent_text},
        # Turn 2: read canonical examples
        {
            "role": "assistant", "content": None,
            "tool_calls": [
                {"id": "call_r2", "type": "function",
                 "function": {"name": "read", "arguments": json.dumps({"filePath": str(REACTOR)})}},
                {"id": "call_r3", "type": "function",
                 "function": {"name": "read", "arguments": json.dumps({"filePath": str(MAINQNT)})}},
            ],
        },
        {"role": "tool", "tool_call_id": "call_r2", "content": reactor_text},
        {"role": "tool", "tool_call_id": "call_r3", "content": mainqnt_text},
        # Turn 3: write rcv.qnt v1 + main.qnt v1 (slightly buggy)
        {
            "role": "assistant", "content": None,
            "tool_calls": [
                {"id": "call_w1", "type": "function",
                 "function": {"name": "write", "arguments": json.dumps({
                     "filePath": "/tmp/probe/rcv.qnt",
                     "content": "module rcv {\n  type Addr\n  type Timestamp = Int\n  var current_time: Timestamp\n  action init = current_time' = 0\n  action step = current_time' = current_time + 1\n}\n",
                 })}},
                {"id": "call_w2", "type": "function",
                 "function": {"name": "write", "arguments": json.dumps({
                     "filePath": "/tmp/probe/main.qnt",
                     "content": "module main\n    const TEST_START: Timestamp = 100\n    import rcv.* from \"./rcv\"\n",
                 })}},
            ],
        },
        {"role": "tool", "tool_call_id": "call_w1", "content": "File written successfully"},
        {"role": "tool", "tool_call_id": "call_w2", "content": "File written successfully"},
        # Turn 4: run quint typecheck
        {
            "role": "assistant", "content": None,
            "tool_calls": [{
                "id": "call_b1", "type": "function",
                "function": {"name": "bash", "arguments": json.dumps({
                    "command": "quint typecheck /tmp/probe/main.qnt",
                })},
            }],
        },
        {"role": "tool", "tool_call_id": "call_b1", "content": (
            "error: [QNT008] Couldn't find a definition for: Timestamp\n"
            "  --> main.qnt:2:24:\n"
            "  |\n"
            "2 |     const TEST_START: Timestamp = 100\n"
            "  |                       ^^^^^^^^^\n"
            "  |\n"
            "error: [QNT008] missing '{' at 'const'\n"
            "  --> main.qnt:1:1:\n"
            "  |\n"
            "1 | module main\n"
            "  | ^^^^^^\n"
        )},
        # Turn 5: read additional examples
        {
            "role": "assistant", "content": None,
            "tool_calls": [
                {"id": "call_r4", "type": "function",
                 "function": {"name": "read", "arguments": json.dumps({"filePath": str(REPLICA)})}},
                {"id": "call_r5", "type": "function",
                 "function": {"name": "read", "arguments": json.dumps({"filePath": str(TYPES)})}},
            ],
        },
        {"role": "tool", "tool_call_id": "call_r4", "content": replica_text},
        {"role": "tool", "tool_call_id": "call_r5", "content": types_text},
        # Final user turn: ask for the fix (this is the "hang point")
        {"role": "user", "content": (
            "The typecheck failed. Fix the type errors and module brace, then re-emit "
            "rcv.qnt and main.qnt by calling write. Wrap the module body in braces. "
            "Don't redefine Int as a type alias."
        )},
    ]


async def main():
    intent_text = INTENT.read_text()
    reactor_text = REACTOR.read_text()
    mainqnt_text = MAINQNT.read_text()
    replica_text = REPLICA.read_text() if REPLICA.exists() else ""
    types_text = TYPES.read_text() if TYPES.exists() else ""
    print(f"intent.md: {len(intent_text)} chars", flush=True)
    print(f"reactor.qnt: {len(reactor_text)} chars", flush=True)
    print(f"main_n6f1b1.qnt: {len(mainqnt_text)} chars", flush=True)
    print(f"replica.qnt: {len(replica_text)} chars", flush=True)
    print(f"types.qnt: {len(types_text)} chars", flush=True)

    test = sys.argv[1] if len(sys.argv) > 1 else "all"

    if test in ("all", "small"):
        await stream_call(
            [{"role": "user", "content": "Say hello in three words."}],
            "T1 small / no tools",
            timeout=120,
        )

    if test in ("all", "reasoning"):
        # Force the model to use its reasoning channel.
        await stream_call(
            [{"role": "user", "content": (
                "Think step by step. What is 17 * 23, and why? "
                "Show your reasoning, then give the final answer."
            )}],
            "T2 reasoning-probe / no tools",
            timeout=180,
        )

    if test in ("all", "tools-t1"):
        # Single-turn tool call: does the model emit clean tool_calls deltas?
        await stream_call(
            [
                {"role": "system", "content": (
                    "You are a spec generator. Read intent.md first by calling the read tool "
                    f"with filePath {INTENT}."
                )},
                {"role": "user", "content": "Begin."},
            ],
            "T3 tools / turn-1 (expect tool_call)",
            tools=[READ_TOOL, WRITE_TOOL, BASH_TOOL],
            timeout=180,
        )

    if test in ("all", "tools-t3"):
        # The "hang point" shape.
        messages = build_turn3_messages(intent_text, reactor_text, mainqnt_text,
                                        replica_text, types_text)
        await stream_call(
            messages,
            "T4 tools / turn-N (the hang shape: long context + ask to write)",
            tools=[READ_TOOL, WRITE_TOOL, BASH_TOOL],
            timeout=900,
        )

    if test in ("all", "tools-write"):
        # Force a write with a large content argument — this is the exact shape
        # of opencode step 19 (write main.qnt with a multi-KB content arg).
        messages = build_turn3_messages(intent_text, reactor_text, mainqnt_text,
                                        replica_text, types_text)
        # Replace final user with a directive that prohibits bash / read and
        # demands an immediate write.
        messages[-1] = {"role": "user", "content": (
            "Do NOT call bash or read. Your next action MUST be a single `write` "
            "tool call to /tmp/probe/main.qnt with a corrected main module body "
            "(use braces around the module body, define TEST_START and TEST_END "
            "as Int, and import rcv.* from ./rcv). The full file content must be "
            "in the `content` argument of the write tool call."
        )}
        await stream_call(
            messages,
            "T5 tools / forced-write (large arguments delta stream)",
            tools=[READ_TOOL, WRITE_TOOL, BASH_TOOL],
            timeout=1200,
        )

    if test == "replay-step19":
        # Replay the exact context opencode sent at step 19 of the stalled session
        # (reconstructed from sqlite DB ses_1c073f0a7ffeCWEuHdq5rqvYYf).
        replay_path = Path("/tmp/nemotron_step19_messages.json")
        if not replay_path.exists():
            print(f"  ERROR: {replay_path} missing — run /tmp/reconstruct_messages.py first",
                  flush=True)
            return
        replay_messages = json.loads(replay_path.read_text())
        # Prepend agent system prompt
        agent_path = Path("/Users/mvid/Development/reliq/verified-rcv/.opencode/agent/"
                          "quint-spec-generator.md")
        if agent_path.exists():
            body = agent_path.read_text()
            # Drop the YAML frontmatter
            if body.startswith("---") or body.startswith("description:"):
                # frontmatter ends at second `---` or first blank line after metadata
                lines = body.splitlines()
                # find end of frontmatter
                start = 0
                if lines and lines[0].startswith("description:"):
                    # Walk until first blank line
                    for i, line in enumerate(lines):
                        if not line.strip():
                            start = i + 1
                            break
                body = "\n".join(lines[start:])
            replay_messages = [{"role": "system", "content": body}] + replay_messages
        print(f"  replay messages: {len(replay_messages)} "
              f"(~{sum(len(str(m)) for m in replay_messages)} chars)", flush=True)
        await stream_call(
            replay_messages,
            "T7 REPLAY of opencode step 19 (the exact stall point)",
            tools=[READ_TOOL, WRITE_TOOL, BASH_TOOL],
            timeout=1200,
            max_tokens=8192,
        )

    if test in ("all", "tools-write-big"):
        # As above but request both rcv.qnt and main.qnt in one assistant turn
        # with full file contents — mirrors the multi-KB write_call pattern.
        messages = build_turn3_messages(intent_text, reactor_text, mainqnt_text,
                                        replica_text, types_text)
        messages[-1] = {"role": "user", "content": (
            "Do NOT call bash or read. Your next action MUST be TWO `write` tool "
            "calls in parallel:\n"
            "  1. /tmp/probe/rcv.qnt — the full corrected spec module (must include "
            "actions instantiate, submit_ballot, close_and_tally, publish_result, "
            "init, step, and pure defs voting_window, tallying_state, resolved_state)\n"
            "  2. /tmp/probe/main.qnt — the corrected main module that imports rcv "
            "with concrete constants.\n"
            "Both file bodies must be complete and in the `content` argument of "
            "the respective write tool call."
        )}
        await stream_call(
            messages,
            "T6 tools / forced-double-write (very large arguments delta stream)",
            tools=[READ_TOOL, WRITE_TOOL, BASH_TOOL],
            timeout=1200,
            max_tokens=32768,
        )


if __name__ == "__main__":
    asyncio.run(main())
