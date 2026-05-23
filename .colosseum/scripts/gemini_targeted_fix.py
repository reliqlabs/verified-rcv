#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["httpx>=0.27"]
# requires-python = ">=3.11"
# ///
"""
One sharp fix turn for gemini-2-5-flash.

Loads the v6 conversation state, sends a single prescriptive fix message
with concrete code snippets (not just gotcha labels), and writes the
result back into the per-voice dir.
"""

import asyncio, json, subprocess
from pathlib import Path
import httpx

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
VOICE_DIR = REPO / ".colosseum/specs/v6-gemini-inline/per-voice/gemini-2-5-flash"
BASE_URL = "https://ai-gateway.burnt.com/u/c5nksc/openai/v1"
API_KEY = "xc-bd97faa4aab05df1c20993b2d2713b6e73cab1f48e2d020b"
MODEL = "gemini-2-5-flash"

# Rebuild the conversation history from the saved raw responses.
INTENT = (REPO / ".colosseum/intent.md").read_text()
INITIAL_RESPONSE = (VOICE_DIR / "raw-response.md").read_text()
FIX1_RESPONSE = (VOICE_DIR / "raw-response-fix1.md").read_text()
FIX2_RESPONSE = (VOICE_DIR / "raw-response-fix2.md").read_text()

# Get current typecheck stderr (post fix2)
tc = subprocess.run(["quint", "typecheck", str(VOICE_DIR / "main.qnt")],
                    capture_output=True, text=True)
tc_stderr = (tc.stderr or "")[-2000:]
print(f"Current typecheck exit={tc.returncode}, stderr {len(tc_stderr)} chars")

SYSTEM = """You are a Quint protocol-spec generator. Emit ONLY the three required files, delimited:

===BEGIN FILE: rcv.qnt===
... full content ...
===END FILE===

===BEGIN FILE: main.qnt===
... full content ...
===END FILE===

===BEGIN FILE: design-notes.md===
... under 600 words ...
===END FILE===

No prose outside the delimiters.
"""

INITIAL_USER = f"""Generate a Quint protocol spec for the verified-rcv system.

INTENT:
{INTENT}

Encode §2.5 actions, §3.1 structural invariants S1-S10, §3.2 behavioral
invariants B1-B7 + B10 chain-side. Phrase witnesses as reachability
negations (the witness invariant must be violated on some trace).
"""

SHARP_FIX_V2 = f"""One more issue. Quint's if-then-else syntax does NOT use the `then` keyword. It is:

    if (cond) value_if_true else value_if_false

NOT:

    if (cond) then value_if_true else value_if_false

Find every occurrence of `if (...) then` in rcv.qnt and remove the `then` keyword (there are ~6 of them). Leave everything else unchanged. Re-emit ALL THREE files with BEGIN/END FILE delimiters, no prose outside.
"""

SHARP_FIX = f"""Your spec still does not typecheck. The remaining errors are caused by **Rust idioms that do not exist in Quint**. Stop trying to fix them with parametric type aliases or SumType definitions — Quint does not support that. Use the literal encodings below.

Current typecheck output (last 2000 chars):
{tc_stderr}

REQUIRED REPLACEMENTS (apply mechanically across rcv.qnt and main.qnt):

1. `Option[T]` and `OptionT[T]`: delete these entirely. Replace every occurrence of `Option[T]` with the inline record type `{{ present: bool, value: T }}`. So:
     var tally_result: Option[TallyResult]
   becomes:
     var tally_result: {{ present: bool, value: TallyResult }}
   And `Set[Option[X]]` becomes `Set[{{ present: bool, value: X }}]`.

2. `some(x)` -> `{{ present: true, value: x }}`. `none()` -> `{{ present: false, value: <ANY-PLACEHOLDER> }}` where <ANY-PLACEHOLDER> is any value of the right type (e.g. an empty record `{{}}` won't work — use a typed default like an empty Set, empty Map, or the empty-record-literal for that TallyResult type).

3. `x.is_some()` -> `x.present`. `x.get()` -> `x.value`.

4. `Vec[T]` -> `List[T]` everywhere. `Vec(a, b, c)` -> `[a, b, c]` or `List(a, b, c)`.

5. `Vec` methods: `.zip()` does not exist — use `foldl` to walk in parallel. Sequence indexing is `xs[i]`. `xs.length()` not `xs.len()`.

6. `.toStr()` does not exist. Remove every `.toStr()` call. If you need string concatenation for debugging just delete the debug helper — invariants don't need string output.

7. SumType / parametric type aliases: Quint does **not** allow `type Foo[T] = SumType[...]`. If you used `type OptionT[T] = SumType[...]` to model Option, delete the alias and use the literal `{{ present: bool, value: T }}` record form everywhere.

8. If anywhere you wrote `let foo = bar` inside an expression, Quint uses `val foo = bar` for pure bindings.

9. `Map[K, V]`: construct with `Map()` (empty) or `Map(k1 -> v1, ...)`. Read with `m.get(k)` returning `V` (no Option wrapping in Quint — guard with `m.keys().contains(k)` first).

This is the LAST fix round. Be conservative: prefer dropping optional invariants over leaving a non-compiling spec. Re-emit ALL THREE files in full with the BEGIN/END FILE delimiters. No prose outside the delimiters."""


async def main():
    fix3_path = VOICE_DIR / "raw-response-fix3-sharp.md"
    use_v2 = fix3_path.exists()
    messages = [
        {"role": "system", "content": SYSTEM},
        {"role": "user", "content": INITIAL_USER},
        {"role": "assistant", "content": INITIAL_RESPONSE},
        {"role": "user", "content": "Your spec failed to typecheck. Fix and re-emit all three files."},
        {"role": "assistant", "content": FIX1_RESPONSE},
        {"role": "user", "content": "Still failing. Re-emit all three files."},
        {"role": "assistant", "content": FIX2_RESPONSE},
        {"role": "user", "content": SHARP_FIX},
    ]
    if use_v2:
        messages.append({"role": "assistant", "content": fix3_path.read_text()})
        messages.append({"role": "user", "content": SHARP_FIX_V2})
        save_name = "raw-response-fix4-then.md"
    else:
        save_name = "raw-response-fix3-sharp.md"
    payload = {"model": MODEL, "messages": messages, "max_tokens": 65536, "stream": False}
    headers = {"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"}
    print(f"Total messages: {len(messages)}, payload size: {len(json.dumps(payload))} bytes")
    async with httpx.AsyncClient(timeout=300) as cli:
        # Retry a couple of times on the ReadError pattern.
        for attempt in range(3):
            try:
                r = await cli.post(f"{BASE_URL}/chat/completions", json=payload, headers=headers)
                if r.status_code != 200:
                    print(f"attempt {attempt+1}: HTTP {r.status_code}: {r.text[:400]}")
                    continue
                data = r.json()
                choice = data["choices"][0]
                content = choice["message"].get("content") or ""
                finish = choice.get("finish_reason")
                usage = data.get("usage", {})
                print(f"attempt {attempt+1}: ok, finish={finish}, output={usage.get('completion_tokens','?')} tokens, content {len(content)} chars")
                if not content:
                    print(f"  empty content; finish={finish}; raw choice keys: {list(choice.get('message',{}).keys())}")
                    continue
                (VOICE_DIR / save_name).write_text(content)
                # Parse delimited blocks
                import re
                FILE_RE = re.compile(r"===BEGIN FILE:\s*(?P<name>[^=]+?)\s*===\s*\n(?P<content>.*?)\n===END FILE===", re.DOTALL)
                files = {m.group("name").strip(): m.group("content") for m in FILE_RE.finditer(content)}
                print(f"  parsed files: {sorted(files.keys())}")
                for name, body in files.items():
                    # Strip code fences
                    lines = body.splitlines()
                    if lines and lines[0].lstrip().startswith("```"):
                        lines = lines[1:]
                    while lines and lines[-1].rstrip().startswith("```"):
                        lines = lines[:-1]
                    (VOICE_DIR / name).write_text("\n".join(lines))
                return
            except Exception as e:
                print(f"attempt {attempt+1}: {type(e).__name__}: {e}")
        print("all attempts failed")

asyncio.run(main())
