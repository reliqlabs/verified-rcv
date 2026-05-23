#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27.0", "python-dotenv>=1.0.0"]
# ///
"""
Inline-context Quint spec generation dispatch.

Unlike the subagent-loop variant in quint_spec_dispatch.py (which used
OpenCode + ReAct + tool-use), this script issues a single HTTP request
per voice with intent.md + a canonical Quint example inlined into the
user prompt. This sidesteps the gateway 240s wall-time cap that was
killing the synthesis turn in the subagent variant — the model gets
all context at once and emits one response containing the spec files,
delimited by `===BEGIN FILE: <name>===` / `===END FILE===` markers.

Self-validation (quint typecheck / quint run --invariant) is run
post-hoc on each parsed file set.

USAGE
    uv run --script fan_out_quint_dispatch.py [--voices=A,B,C]

OUTPUT
    .colosseum/specs/<run-tag>/
    ├── per-voice/
    │   └── <voice-id>/
    │       ├── rcv.qnt
    │       ├── main.qnt
    │       ├── design-notes.md
    │       ├── raw-response.md
    │       └── validation.json
    ├── results.json
    └── dispatch.log
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

import httpx
from dotenv import load_dotenv

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
COLOSSEUM = Path("/Users/mvid/Development/reliq/colosseum")
INTENT = REPO / ".colosseum" / "intent.md"
CANONICAL_EXAMPLE = Path("/Users/mvid/go/pkg/mod/github.com/cometbft/cometbft@v0.38.21/spec/p2p/reactor-api/reactor.qnt")
CANONICAL_MAIN = Path("/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/main_n6f1b1.qnt")
ENV = REPO / ".env"

load_dotenv(ENV)
GATEWAY_URL = os.environ["COLOSSEUM_GATEWAY_BASE_URL"]
GATEWAY_KEY = os.environ["COLOSSEUM_GATEWAY_API_KEY"]
LOCAL_ENDPOINT = "http://localhost:1234/v1/chat/completions"

WITNESS_SPECS = [
    "witness_resolution_reachable",
    "witness_ballot_submittable",
    "witness_end_at_crossing",
]
SAFETY_INVARIANT = "all_invariants"

RUN_TAG_ENV = os.environ.get("COLOSSEUM_QUINT_RUN_TAG")
if RUN_TAG_ENV:
    RUN_TAG = RUN_TAG_ENV
else:
    _ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    RUN_TAG = f"quint-inline-{_ts}"
OUTDIR = REPO / ".colosseum" / "specs" / RUN_TAG
OUTDIR.mkdir(parents=True, exist_ok=True)
(OUTDIR / "per-voice").mkdir(exist_ok=True)

TIMEOUT = 600.0  # 10min per HTTP request — gives reasoning models room
MAX_RETRIES = 1                    # HTTP-level retries on network/timeout failure
MAX_TYPECHECK_FIX_ROUNDS = 2       # additional rounds where typecheck stderr is fed back as user message


# ─────────────────────────────────────────────────────────────────────────
# Voice roster — same as adversarial inline dispatch, plus nemotron.
# ─────────────────────────────────────────────────────────────────────────

VOICES = [
    # (id_slug, endpoint, model_id, gateway?)
    ("kimi-k2-6",                  "gateway", "kimi-k2-6"),
    ("gpt-oss-120b",               "gateway", "gpt-oss-120b"),
    ("nemotron-3-120b-a12b",       "gateway", "cloudflare-100-cf-nvidia-nemotron-3-120b-a12b"),
    ("gemini-2-5-flash",           "gateway", "gemini-2-5-flash"),
    ("gemma-4-26b-a4b",            "local",   "google/gemma-4-26b-a4b"),
    ("qwen3.6-27b",                "local",   "qwen/qwen3.6-27b"),
    ("mistral-small-4-119b-2603",  "local",   "mistral-small-4-119b-2603"),
]


def build_system_prompt() -> str:
    return """You are a Quint protocol-spec generator for the Colosseum verification methodology. Your job is to translate a validated intent document into an executable Quint specification.

You are one voice in a multi-model fan-out: independent voices each produce a spec from the same intent, and a downstream synthesis pass compares structural choices. Your goal is to produce an honest, idiomatic Quint encoding that reflects how *you* read the intent. Different encodings across voices is the methodology signal.

You will be given:
- The full intent document (the source of truth).
- A canonical Quint example for idiom calibration.
- A list of MANDATORY invariant names that must appear in your spec.

Hard output contract:

1. Emit exactly three files, each enclosed in delimiters that the dispatcher parses:

   ===BEGIN FILE: rcv.qnt===
   <the protocol module — defines types, state variables, parameters (const), pure predicates, actions, the step relation, the safety invariant, and the witness invariants>
   ===END FILE===

   ===BEGIN FILE: main.qnt===
   <a tiny module that imports rcv with concrete const values for the bounded universe>
   ===END FILE===

   ===BEGIN FILE: design-notes.md===
   <under 600 words: which §2.5 blocks map to which actions; how you encoded each §3.1 + §3.2 invariant; which invariants you omitted and why; any non-obvious encoding choices>
   ===END FILE===

2. Output NOTHING outside these delimiters except an optional final line `STATUS: <ok|partial|error>`.

   **No tool calls.** This invocation has no execution sandbox attached. Emit the file contents as plain text inside the delimiters; do NOT emit JSON tool-call shapes (`tool_calls`, function-call structures, etc.). The dispatcher writes files from the parsed delimited blocks.

   **Bound your reasoning.** If you are a reasoning-mode model, keep internal reasoning short. The token budget covers reasoning + output combined; long reasoning leaves no room for the actual files.

3. The spec MUST contain (with these exact names) a composite `all_invariants` definition AND `witness_resolution_reachable`, `witness_ballot_submittable`, `witness_end_at_crossing` as separate `val name: bool = ...` definitions. The witnesses are reachability claims — phrase each as the NEGATION of a property that requires a trace to witness (so they fail under `quint run`, exhibiting a counterexample trace that reaches the named state).

4. The spec MUST be parameterized over const declarations and instantiated in main.qnt with concrete values, mirroring the canonical example's pattern.

5. Do NOT copy conventions from sealed-auction or ranked-choice Quint specs — those were LLM-generated and their idioms are not load-bearing. Use the canonical example provided.

6. The dispatcher will auto-run `quint typecheck` on main.qnt + `quint run --invariant=all_invariants` + `quint run --invariant=<each witness>` and record results. Aim for: typecheck passes, all_invariants holds across sampled traces (no violation), each witness is violated (system reaches that state). Specs that fail validation are not deleted, but only passing specs go into synthesis.

7. Encoding latitude: how you encode temporal invariants (action-guard disablement, ghost variables, Quint temporal operators) is your choice. Different voices choosing differently is the point.

8. Cross-layer / meta-security invariants (B8 / B9 / B10): Quint cannot directly encode probabilistic or off-chain claims. Encode the classical-Prop shadow (e.g., "if tally_result.is_some, then registry was honest at firing transition") and note the omission in design-notes.md.
"""


def build_user_prompt() -> str:
    intent_text = INTENT.read_text()
    canonical = CANONICAL_EXAMPLE.read_text()
    canonical_main = CANONICAL_MAIN.read_text()
    witnesses_block = "\n".join(f"- {w}" for w in WITNESS_SPECS)
    return f"""Produce a Quint spec for the verified-rcv system per the intent doc below.

The verified-rcv system is a three-component IRV protocol: CosmWasm chain contract + dstack-TDX enclave + zkdcap attestation. The intent doc is the source of truth.

MANDATORY invariant names:
- Safety invariant (must HOLD across all sampled traces): `{SAFETY_INVARIANT}` (composite of all S-series + B-series claims you encode)
- Reachability witnesses (each must be VIOLATED — i.e., system reaches the named state):
{witnesses_block}

The dispatcher will run `quint typecheck` + `quint run --invariant=...` to validate.

## Required module / instantiation idiom (LOAD-BEARING — first-pass failures clustered here)

Your `rcv.qnt` declares the protocol module with `const` parameters; your `main.qnt` instantiates them with concrete values via `import rcv(PARAM = VALUE, ...).* from "./rcv"`. The instantiation file is the model-checking entry point — it must define no new actions or state, just the import line wrapped in `module main {{ ... }}`.

Concrete Quint syntax pitfalls to AVOID (all observed in prior failed first-pass specs):
- `const NAME = value` is WRONG. Use `const NAME: Type` in `rcv.qnt`; bind the value in `main.qnt` via `import rcv(NAME = value).* from "./rcv"`.
- `import rcv hiding {{ init, step }}` is WRONG. Use `import rcv(...).* from "./rcv"` to instantiate const-parameterized modules. The `from` clause is mandatory; `./rcv` is the path to the sibling `rcv.qnt` file (no extension).
- `action init = rcv::init` is WRONG. Quint does not allow re-exporting actions across modules; if you used `import rcv.* from "./rcv"`, `init` and `step` are already in scope and runnable from `main` directly. Do not re-declare them.

The minimmit `main_n6f1b1.qnt` example below shows the exact pattern. Mirror it.

=== CANONICAL QUINT MODULE (reactor.qnt, by Informal Systems — single-module idiom reference) ===
```quint
{canonical}
```
=== END CANONICAL MODULE ===

=== CANONICAL MAIN.QNT (commonware minimmit main_n6f1b1.qnt — multi-module instantiation idiom) ===
```quint
{canonical_main}
```
=== END CANONICAL MAIN ===

=== INTENT DOCUMENT (source of truth) ===
{intent_text}
=== END INTENT DOCUMENT ===

Emit your three files now using the BEGIN/END FILE delimiters from the system prompt. End with a `STATUS:` line.
"""


_FILE_RE = re.compile(
    r"===BEGIN FILE:\s*(?P<name>[^=]+?)\s*===\s*\n(?P<content>.*?)\n===END FILE===",
    re.DOTALL,
)


def parse_response(response_text: str) -> dict[str, str]:
    """Extract files from BEGIN/END FILE blocks. Returns {filename: content}."""
    files = {}
    for m in _FILE_RE.finditer(response_text):
        name = m.group("name").strip()
        content = m.group("content")
        # Strip leading code fence if model wrapped content in ```quint ... ```
        lines = content.splitlines()
        if lines and lines[0].lstrip().startswith("```"):
            lines = lines[1:]
        while lines and lines[-1].rstrip().startswith("```"):
            lines = lines[:-1]
        files[name] = "\n".join(lines)
    return files


async def call_voice(client: httpx.AsyncClient, voice: tuple, messages: list[dict]) -> dict:
    """Issue one HTTP request with the given messages array. Caller owns the conversation history."""
    voice_id, endpoint_kind, model_id = voice
    if endpoint_kind == "gateway":
        url = f"{GATEWAY_URL}/chat/completions"
        headers = {"Authorization": f"Bearer {GATEWAY_KEY}", "Content-Type": "application/json"}
    else:
        url = LOCAL_ENDPOINT
        headers = {"Content-Type": "application/json"}
    payload = {
        "model": model_id,
        "messages": messages,
        "max_tokens": 65536,
        "stream": False,
    }
    t0 = datetime.now(timezone.utc)
    try:
        r = await client.post(url, json=payload, headers=headers, timeout=TIMEOUT)
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        if r.status_code != 200:
            return {"voice": voice_id, "error": f"HTTP {r.status_code}: {r.text[:500]}", "elapsed_s": elapsed}
        data = r.json()
        try:
            choice = data["choices"][0]
            message = choice.get("message", {})
            content = message.get("content")
            finish = choice.get("finish_reason", "?")
            usage = data.get("usage", {})
        except (KeyError, IndexError, TypeError) as parse_err:
            return {
                "voice": voice_id,
                "error": f"unparseable response: {parse_err}",
                "elapsed_s": elapsed,
                "raw_response": json.dumps(data)[:2000],
            }
        if not content:
            # Some gateways return null content with finish_reason=tool_calls etc.
            rc = message.get("reasoning_content")
            tc = message.get("tool_calls")
            extra = []
            if rc:
                extra.append(f"reasoning_content={len(rc)}chars")
            if tc:
                extra.append(f"tool_calls=present({len(tc)})")
            return {
                "voice": voice_id,
                "error": f"empty content (finish_reason={finish}, " + ", ".join(extra) + f", usage={usage})",
                "elapsed_s": elapsed,
                "raw_response": json.dumps(data),
                "finish_reason": finish,
                "usage": usage,
            }
        return {
            "voice": voice_id,
            "elapsed_s": elapsed,
            "content": content,
            "finish_reason": finish,
            "usage": usage,
        }
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"voice": voice_id, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


def _ts() -> str:
    return datetime.now(timezone.utc).strftime("%H:%M:%SZ")


def _log_both(msg: str, logf) -> None:
    line = f"{_ts()} {msg}"
    print(line, flush=True)
    logf.write(line + "\n")
    logf.flush()


def typecheck_only(voice_outdir: Path) -> tuple[int, str]:
    """Fast typecheck pass for the retry loop. Returns (exit_code, stderr_tail)."""
    main_qnt = voice_outdir / "main.qnt"
    if not main_qnt.exists():
        return (1, "main.qnt is missing")
    r = subprocess.run(["quint", "typecheck", str(main_qnt)], capture_output=True, text=True, timeout=120)
    return (r.returncode, (r.stderr or "")[-1500:])


def validate_voice_spec(voice_outdir: Path) -> dict:
    main_qnt = voice_outdir / "main.qnt"
    if not main_qnt.exists():
        return {"typecheck": "missing main.qnt"}
    result = {}
    r = subprocess.run(["quint", "typecheck", str(main_qnt)], capture_output=True, text=True, timeout=120)
    result["typecheck"] = {"exit": r.returncode, "stderr": (r.stderr or "")[-500:]}
    r = subprocess.run(
        ["quint", "run", f"--invariant={SAFETY_INVARIANT}", "--max-steps=30", "--max-samples=100", str(main_qnt)],
        capture_output=True, text=True, timeout=300,
    )
    out = (r.stdout or "") + (r.stderr or "")
    result[SAFETY_INVARIANT] = {
        "exit": r.returncode,
        "holds": "No violation found" in out,
        "tail": out[-400:],
    }
    result["witnesses"] = {}
    for w in WITNESS_SPECS:
        r = subprocess.run(
            ["quint", "run", f"--invariant={w}", "--max-steps=30", "--max-samples=100", str(main_qnt)],
            capture_output=True, text=True, timeout=300,
        )
        out = (r.stdout or "") + (r.stderr or "")
        result["witnesses"][w] = {
            "exit": r.returncode,
            "violated": "Invariant violated" in out or "Found an issue" in out,
            "tail": out[-400:],
        }
    return result


def voice_passes_all(validation: dict) -> bool:
    tc = validation.get("typecheck")
    if not isinstance(tc, dict) or tc.get("exit") != 0:
        return False
    inv = validation.get(SAFETY_INVARIANT)
    if not isinstance(inv, dict) or not inv.get("holds"):
        return False
    for w, r in validation.get("witnesses", {}).items():
        if not r.get("violated"):
            return False
    return True


def _build_fix_message(tc_exit: int, tc_stderr: str, round_idx: int, max_rounds: int) -> str:
    return f"""The Quint typechecker rejected the spec you just emitted. Fix it and re-emit ALL THREE files (full content, not a diff) using the same BEGIN/END FILE delimiters from the system prompt.

`quint typecheck main.qnt` exit code: {tc_exit}
stderr (last 1500 chars):
{tc_stderr}

Notes for the fix:
- Quint has no `Option`/`None` built-in. Encode optional values as `{{ present: bool, value: T }}` records, or as a sum type you declare yourself.
- Quint uses `List[T]`, not `Vec[T]`. Lists are constructed with `[a, b, c]` or `List(a, b, c)`. No `.zip()` method — use `fold`/`foldl` for parallel iteration.
- Multi-module: in `main.qnt`, use `import rcv(PARAM = value, ...).* from "./rcv"`. After that import, `init` and `step` from `rcv` are runnable directly — do not re-declare or re-export them with `::`.
- `const` declares a parameter: `const NAME: Type` in `rcv.qnt`; bind the value in `main.qnt` via the `import rcv(NAME = value)` form.
- `pure def` for deterministic state-update functions; `action` for the model-checking transition relation with guards + primed-variable assignments.

This is fix round {round_idx} of {max_rounds}. After this, the run is abandoned regardless of typecheck status. Be conservative: prefer minimal changes that compile over additional invariants. If you can't fit a working full spec in one response, drop optional invariants / scenarios to stay under the output budget; the structural invariants S1-S9 and the temporal claims B1-B7 + B10 chain-side projection are what matters.

Emit your corrected files now."""


async def dispatch_voice(client: httpx.AsyncClient, voice: tuple, system: str, user: str, log_path: Path) -> dict:
    voice_id = voice[0]
    voice_outdir = OUTDIR / "per-voice" / voice_id
    voice_outdir.mkdir(parents=True, exist_ok=True)
    messages: list[dict] = [
        {"role": "system", "content": system},
        {"role": "user", "content": user},
    ]
    fix_round_history: list[dict] = []  # per-round records for the audit trail
    with log_path.open("a") as logf:
        _log_both(f"→ {voice_id}: dispatching ({voice[1]}/{voice[2]})", logf)

        # Initial generation (with HTTP-level retries on network/timeout failure).
        for attempt in range(MAX_RETRIES + 1):
            r = await call_voice(client, voice, messages)
            if "error" not in r:
                break
            _log_both(f"    ⚠ {voice_id} attempt {attempt + 1} failed ({r['elapsed_s']:.0f}s): {r['error'][:200]}", logf)
        if "error" in r:
            _log_both(f"  ✗ {voice_id} ABANDONED after {MAX_RETRIES + 1} attempts", logf)
            if r.get("raw_response"):
                (voice_outdir / "raw-response-error.json").write_text(r["raw_response"])
            return {"voice": voice_id, "call": r, "validation": None, "passes": False}

        # Save raw response, parse files
        (voice_outdir / "raw-response.md").write_text(r["content"])
        files = parse_response(r["content"])
        for name, content in files.items():
            (voice_outdir / name).write_text(content)
        parsed_names = sorted(files.keys())
        _log_both(f"  ↳ {voice_id} got {r['elapsed_s']:.0f}s response (finish={r['finish_reason']}), parsed files: {parsed_names}", logf)

        missing = [f for f in ("rcv.qnt", "main.qnt") if f not in files]
        if missing:
            _log_both(f"  ✗ {voice_id} missing required files: {missing}", logf)
            return {"voice": voice_id, "call": r, "files_parsed": parsed_names, "validation": None, "passes": False}

        # Typecheck-fix loop: feed errors back, ask model to fix.
        messages.append({"role": "assistant", "content": r["content"]})
        last_call = r
        for fix_round in range(1, MAX_TYPECHECK_FIX_ROUNDS + 1):
            tc_exit, tc_stderr = typecheck_only(voice_outdir)
            if tc_exit == 0:
                _log_both(f"    ✓ {voice_id} typecheck passed{' on first attempt' if fix_round == 1 else f' after {fix_round - 1} fix round(s)'}", logf)
                break
            _log_both(f"    ⟳ {voice_id} typecheck failed (exit={tc_exit}), sending fix request round {fix_round}/{MAX_TYPECHECK_FIX_ROUNDS}", logf)
            fix_user_msg = _build_fix_message(tc_exit, tc_stderr, fix_round, MAX_TYPECHECK_FIX_ROUNDS)
            messages.append({"role": "user", "content": fix_user_msg})
            for attempt in range(MAX_RETRIES + 1):
                fr = await call_voice(client, voice, messages)
                if "error" not in fr:
                    break
                _log_both(f"      ⚠ {voice_id} fix-round {fix_round} HTTP attempt {attempt + 1} failed ({fr['elapsed_s']:.0f}s): {fr['error'][:200]}", logf)
            fix_round_history.append({"round": fix_round, "tc_exit_before": tc_exit, "call": {k: v for k, v in fr.items() if k != "content"}})
            if "error" in fr:
                _log_both(f"    ⚠ {voice_id} fix round {fix_round} HTTP abandoned; keeping last spec", logf)
                break
            (voice_outdir / f"raw-response-fix{fix_round}.md").write_text(fr["content"])
            fix_files = parse_response(fr["content"])
            if not fix_files:
                _log_both(f"    ⚠ {voice_id} fix round {fix_round} response had no parseable files; keeping last spec", logf)
                break
            for name, content in fix_files.items():
                (voice_outdir / name).write_text(content)
            _log_both(f"    ↳ {voice_id} fix round {fix_round} got {fr['elapsed_s']:.0f}s response (finish={fr['finish_reason']}), parsed: {sorted(fix_files.keys())}", logf)
            messages.append({"role": "assistant", "content": fr["content"]})
            last_call = fr

        # Final full validation pass (typecheck + invariant + witnesses).
        validation = validate_voice_spec(voice_outdir)
        validation["fix_rounds"] = fix_round_history
        (voice_outdir / "validation.json").write_text(json.dumps(validation, indent=2, default=str))
        passes = voice_passes_all(validation)
        status = "✓ PASS" if passes else "✗ VALIDATION FAILED"
        n_w_violated = sum(1 for r2 in validation.get("witnesses", {}).values() if r2.get("violated"))
        _log_both(
            f"  {status} {voice_id}: typecheck={validation.get('typecheck', {}).get('exit')} "
            f"safety_holds={validation.get(SAFETY_INVARIANT, {}).get('holds')} "
            f"witnesses_violated={n_w_violated}/{len(WITNESS_SPECS)} "
            f"(fix_rounds={len(fix_round_history)})",
            logf,
        )
        return {
            "voice": voice_id,
            "call": {k: v for k, v in last_call.items() if k != "content"},
            "files_parsed": parsed_names,
            "fix_rounds_count": len(fix_round_history),
            "validation": validation,
            "passes": passes,
        }


async def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", default=None, help="comma-separated voice ids (default: all)")
    ap.add_argument("--sequential", action="store_true", help="sequential dispatch (default: parallel)")
    args = ap.parse_args()

    selected = VOICES
    if args.voices:
        wanted = set(args.voices.split(","))
        selected = [v for v in VOICES if v[0] in wanted]

    system = build_system_prompt()
    user = build_user_prompt()
    print(f"Output dir: {OUTDIR}")
    print(f"Voices ({len(selected)}): {[v[0] for v in selected]}")
    print(f"System prompt: {len(system):,} chars")
    print(f"User prompt: {len(user):,} chars")

    log_path = OUTDIR / "dispatch.log"
    with log_path.open("a") as logf:
        logf.write(f"\n=== Inline Quint dispatch starting {datetime.now(timezone.utc).isoformat()} ===\n")
        logf.write(f"Voices: {[v[0] for v in selected]}\n")
        logf.write(f"Output: {OUTDIR}\n\n")

    async with httpx.AsyncClient() as client:
        if args.sequential:
            all_results = []
            for v in selected:
                r = await dispatch_voice(client, v, system, user, log_path)
                all_results.append(r)
        else:
            tasks = [dispatch_voice(client, v, system, user, log_path) for v in selected]
            all_results = await asyncio.gather(*tasks, return_exceptions=False)

    manifest = {
        "run_tag": RUN_TAG,
        "intent": str(INTENT),
        "canonical_example": str(CANONICAL_EXAMPLE),
        "witness_specs": WITNESS_SPECS,
        "safety_invariant": SAFETY_INVARIANT,
        "system_prompt_chars": len(system),
        "user_prompt_chars": len(user),
        "voices": all_results,
        "n_pass": sum(1 for r in all_results if r.get("passes")),
        "n_total": len(all_results),
    }
    (OUTDIR / "results.json").write_text(json.dumps(manifest, indent=2, default=str))

    print(f"\n=== Summary ===")
    print(f"{manifest['n_pass']}/{manifest['n_total']} voices produced specs that pass all validation checks.")
    for r in all_results:
        v = r["voice"]
        if r.get("passes"):
            print(f"  ✓ {v}")
        else:
            err = r.get("call", {}).get("error") or "validation failed (see per-voice validation.json)"
            print(f"  ✗ {v}: {err[:200] if isinstance(err, str) else err}")


if __name__ == "__main__":
    asyncio.run(main())
