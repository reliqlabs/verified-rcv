#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["httpx>=0.27"]
# requires-python = ">=3.11"
# ///
"""
Direct probe of gemini-2-5-flash via burnt gateway.

Tests three scenarios in sequence:
  1. small: minimal prompt, no tools, streaming
  2. large-ctx: 140KB intent.md as user prompt, no tools, streaming
  3. tools-multiturn: emulate opencode's turn 3 — system + initial user
     + assistant(tool_calls for read intent) + tool_result(intent) +
     assistant(tool_calls for read 2 examples) + tool_results (2) +
     ask gemini for the next turn.

For each: print byte-by-byte stream timing so we can see whether the
connection opens, first byte time, etc.
"""

import asyncio, json, os, sys, time
from pathlib import Path
import httpx

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
INTENT = REPO / ".colosseum/intent.md"
REACTOR = Path("/Users/mvid/go/pkg/mod/github.com/cometbft/cometbft@v0.38.21/spec/p2p/reactor-api/reactor.qnt")
MAINQNT = Path("/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/main_n6f1b1.qnt")

BASE_URL = "https://ai-gateway.burnt.com/u/c5nksc/openai/v1"
API_KEY = "xc-bd97faa4aab05df1c20993b2d2713b6e73cab1f48e2d020b"
MODEL = "gemini-2-5-flash"

READ_TOOL = {
    "type": "function",
    "function": {
        "name": "read",
        "description": "Read a file from local disk",
        "parameters": {
            "type": "object",
            "properties": {
                "filePath": {"type": "string"}
            },
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


async def stream_call(messages, label, tools=None, timeout=120):
    """POST chat completion with streaming. Print first-byte time + total."""
    payload = {
        "model": MODEL,
        "messages": messages,
        "stream": True,
        "max_tokens": 16384,
    }
    if tools is not None:
        payload["tools"] = tools
    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json",
    }
    print(f"\n=== {label} ===")
    print(f"  payload size: {len(json.dumps(payload))} bytes")
    t0 = time.monotonic()
    first_byte_at = None
    chunk_count = 0
    finish = None
    accum_text = []
    accum_tool_calls = []
    try:
        async with httpx.AsyncClient(timeout=timeout) as cli:
            async with cli.stream("POST", f"{BASE_URL}/chat/completions", json=payload, headers=headers) as r:
                connect_at = time.monotonic() - t0
                print(f"  connected (status={r.status_code}) at +{connect_at:.2f}s")
                if r.status_code != 200:
                    body = await r.aread()
                    print(f"  ERROR body: {body.decode('utf-8', 'replace')[:1500]}")
                    return
                async for line in r.aiter_lines():
                    if not line.strip():
                        continue
                    if first_byte_at is None:
                        first_byte_at = time.monotonic() - t0
                        print(f"  first SSE line at +{first_byte_at:.2f}s")
                    chunk_count += 1
                    if line.startswith("data: "):
                        data = line[6:]
                        if data == "[DONE]":
                            break
                        try:
                            chunk = json.loads(data)
                            delta = chunk["choices"][0].get("delta", {})
                            if "content" in delta and delta["content"]:
                                accum_text.append(delta["content"])
                            if "tool_calls" in delta and delta["tool_calls"]:
                                accum_tool_calls.append(delta["tool_calls"])
                            fr = chunk["choices"][0].get("finish_reason")
                            if fr:
                                finish = fr
                        except Exception:
                            pass
                total = time.monotonic() - t0
                print(f"  done at +{total:.2f}s. chunks={chunk_count} finish={finish}")
                print(f"  content len: {sum(len(s) for s in accum_text)} chars")
                print(f"  tool_call chunks: {len(accum_tool_calls)}")
                if accum_text:
                    snippet = "".join(accum_text)[:200].replace("\n", " ")
                    print(f"  content head: {snippet}")
                if accum_tool_calls:
                    print(f"  first tool_call chunk: {str(accum_tool_calls[0])[:200]}")
    except httpx.ReadTimeout:
        elapsed = time.monotonic() - t0
        print(f"  TIMEOUT after {elapsed:.1f}s (first_byte_at={first_byte_at}, chunks={chunk_count})")
    except Exception as e:
        elapsed = time.monotonic() - t0
        print(f"  EXC after {elapsed:.1f}s: {type(e).__name__}: {e}")


async def main():
    intent_text = INTENT.read_text()
    reactor_text = REACTOR.read_text()
    mainqnt_text = MAINQNT.read_text()
    print(f"intent.md: {len(intent_text)} chars")
    print(f"reactor.qnt: {len(reactor_text)} chars")
    print(f"main_n6f1b1.qnt: {len(mainqnt_text)} chars")

    test = sys.argv[1] if len(sys.argv) > 1 else "all"

    if test in ("all", "small"):
        await stream_call(
            [{"role": "user", "content": "Say hello in three words."}],
            "T1 small / no tools",
        )

    if test in ("all", "large"):
        await stream_call(
            [{"role": "user", "content": intent_text + "\n\nIn one sentence: what is this document?"}],
            "T2 large-ctx / no tools",
        )

    if test in ("all", "tools-t1"):
        # Just the initial turn with tools — should respond with a tool_call
        await stream_call(
            [
                {"role": "system", "content": "You are a spec generator. Read intent.md first by calling the read tool with filePath /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md."},
                {"role": "user", "content": "Begin."},
            ],
            "T3 tools / turn-1 (expect tool_call)",
            tools=[READ_TOOL, WRITE_TOOL],
        )

    if test in ("all", "tools-t3"):
        # Emulate opencode turn-3: 2 prior tool-call rounds, now expect the model to write.
        messages = [
            {"role": "system", "content": "You are a Quint spec generator. Read the intent and the canonical examples, then write rcv.qnt and main.qnt to disk."},
            {"role": "user", "content": "Generate the Quint spec."},
            {
                "role": "assistant",
                "content": None,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "read", "arguments": json.dumps({"filePath": str(INTENT)})},
                }],
            },
            {"role": "tool", "tool_call_id": "call_1", "content": intent_text},
            {
                "role": "assistant",
                "content": None,
                "tool_calls": [
                    {"id": "call_2", "type": "function", "function": {"name": "read", "arguments": json.dumps({"filePath": str(REACTOR)})}},
                    {"id": "call_3", "type": "function", "function": {"name": "read", "arguments": json.dumps({"filePath": str(MAINQNT)})}},
                ],
            },
            {"role": "tool", "tool_call_id": "call_2", "content": reactor_text},
            {"role": "tool", "tool_call_id": "call_3", "content": mainqnt_text},
        ]
        await stream_call(
            messages,
            "T4 tools / turn-3 (the hang point)",
            tools=[READ_TOOL, WRITE_TOOL],
            timeout=300,
        )

if __name__ == "__main__":
    asyncio.run(main())
