#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Multi-voice Quint spec generation dispatch.

Fans out the `quint-spec-generator` agent across the same gateway+local
voice roster used for adversarial review. Each voice produces its own
encoding of the verified-rcv intent into a Quint spec. After dispatch,
each spec is auto-validated via `quint typecheck` + `quint run` against
the mandated safety invariant and reachability witnesses. Specs that
don't survive validation are recorded as failures but not deleted.

USAGE
    uv run --script quint_spec_dispatch.py [--voices=A,B,C] [--sequential]

OUTPUT
    .colosseum/specs/<run-tag>/
    ├── per-voice/
    │   └── <voice-id>/
    │       ├── rcv.qnt
    │       ├── main.qnt
    │       ├── design-notes.md
    │       ├── stdout.log              # raw OpenCode output
    │       └── validation.json         # quint typecheck/run results
    ├── results.json                    # cross-voice manifest
    └── dispatch.log
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
COLOSSEUM = Path("/Users/mvid/Development/reliq/colosseum")
INTENT = REPO / ".colosseum" / "intent.md"

# Canonical Quint examples — Informal Systems + production teams.
# Excluded: any sealed-auction / ranked-choice / verified-rcv-derived spec.
CANONICAL_EXAMPLES = [
    "/Users/mvid/go/pkg/mod/github.com/cometbft/cometbft@v0.38.21/spec/p2p/reactor-api/reactor.qnt",
    "/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/replica.qnt",
    "/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/types.qnt",
    "/Users/mvid/Development/burnt/commonware/pipeline/minimmit/quint/main_n6f1b1.qnt",
    "/opt/homebrew/lib/node_modules/@informalsystems/quint/dist/src/builtin.qnt",
]

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
    RUN_TAG = f"quint-fanout-{_ts}"
OUTDIR = REPO / ".colosseum" / "specs" / RUN_TAG
OUTDIR.mkdir(parents=True, exist_ok=True)
(OUTDIR / "per-voice").mkdir(exist_ok=True)

PER_CALL_TIMEOUT = 2400  # 40 min — generation + self-validation (re-typecheck loop)
MAX_RETRIES = 1          # generation is slow; retry once on hard failure

# ─────────────────────────────────────────────────────────────────────────
# Voice roster — same as adversarial Pattern B (glm-4-7-flash excluded
# per Round 3a calibration; claude excluded here because the handcrafted
# spec at specs/rcv.qnt serves as the claude-voice baseline).
# ─────────────────────────────────────────────────────────────────────────

VOICES = [
    # (id_slug, opencode_model_id, note)
    #
    # Cloud — gateway (burnt). Limited to kimi + nemotron per
    # 2026-05-19 strategy: gemini and gpt-class moved to native
    # opencode providers (see below) to avoid gateway proxy/quota
    # interop issues.
    ("kimi-k2-6",                 "burnt/kimi-k2-6",                                              "Moonshot — frontier; gateway"),
    ("nemotron-3-120b-a12b",      "burnt/cloudflare-100-cf-nvidia-nemotron-3-120b-a12b",          "NVIDIA — gateway 120B-A12B MoE, reasoning-on"),
    ("claude-opus-4-7",           "burnt/claude-opus-4-7",                                        "Anthropic — gateway Opus (Bug 4 risk on long prompts)"),
    # Cloud — native opencode providers (BYOK).
    ("gemini-2-5-flash-native",   "google/gemini-2.5-flash",                                      "Google — native via opencode google provider"),
    ("gpt-5-5-native",            "openai/gpt-5.5",                                               "OpenAI — native gpt-5.5 via opencode openai provider"),
    ("magistral-medium-native",   "mistral/magistral-medium-latest",                              "Mistral — native magistral-medium (reasoning + tool_call)"),
    # Local — LM Studio (M5 Max).
    ("gemma-4-26b-a4b",           "lmstudio/google/gemma-4-26b-a4b",                              "Google — local 26B MoE"),
    ("qwen3.6-27b",               "lmstudio/qwen/qwen3.6-27b",                                    "Alibaba — local 27B"),
    ("mistral-small-4-119b-2603", "lmstudio/mistral-small-4-119b-2603",                           "Mistral — local 119B non-reasoning"),
]


def build_message(voice_id: str, voice_outdir: Path) -> str:
    examples_block = "\n".join(f"  - {p}" for p in CANONICAL_EXAMPLES)
    witnesses_block = "\n".join(f"  - {w}" for w in WITNESS_SPECS)
    return f"""VOICE_ID: {voice_id}

INTENT_PATH: {INTENT}

OUTPUT_DIR: {voice_outdir}

SPEC_FILENAME: rcv.qnt

CANONICAL_EXAMPLES:
{examples_block}

WITNESS_SPECS (each must be VIOLATED — i.e., the system reaches the named state):
{witnesses_block}

SAFETY_INVARIANT (must HOLD across all sampled traces): {SAFETY_INVARIANT}

Produce {voice_outdir}/rcv.qnt + {voice_outdir}/main.qnt + {voice_outdir}/design-notes.md.

After writing, self-verify by running:
  quint typecheck {voice_outdir}/main.qnt
  quint run --invariant={SAFETY_INVARIANT} --max-steps=30 --max-samples=100 {voice_outdir}/main.qnt
  quint run --invariant=<each witness> --max-steps=30 --max-samples=100 {voice_outdir}/main.qnt

If any of these fail, debug and re-write. Emit STATUS: ok or STATUS: error: <reason> as the last line of output.
"""


def _ts() -> str:
    return datetime.now(timezone.utc).strftime("%H:%M:%SZ")


def _log_both(msg: str, logf) -> None:
    line = f"{_ts()} {msg}"
    print(line, flush=True)
    logf.write(line + "\n")
    logf.flush()


async def dispatch_one(voice_id: str, model_id: str, attempt: int = 0) -> dict:
    voice_outdir = OUTDIR / "per-voice" / voice_id
    voice_outdir.mkdir(parents=True, exist_ok=True)
    log_prefix = f"[{voice_id}]"
    message = build_message(voice_id, voice_outdir)
    # variant=high requests provider-specific high reasoning effort.
    # opencode silently falls back to default if the model has no high variant.
    cmd = [
        "opencode", "run",
        "--agent", "quint-spec-generator",
        "--model", model_id,
        "--variant", "high",
        "--format", "default",
        "--dangerously-skip-permissions",
        message,
    ]
    t0 = datetime.now(timezone.utc)
    try:
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=str(REPO),
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=PER_CALL_TIMEOUT)
        except asyncio.TimeoutError:
            proc.kill()
            elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
            return {"voice": voice_id, "error": f"timeout after {PER_CALL_TIMEOUT}s", "elapsed_s": elapsed}
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        content = stdout.decode("utf-8", errors="replace")
        import re
        content = re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", content)
        lines = content.splitlines()
        if lines and lines[0].startswith("> "):
            lines = lines[1:]
            while lines and not lines[0].strip():
                lines = lines[1:]
        content = "\n".join(lines).strip()
        (voice_outdir / "stdout.log").write_text(content)
        stderr_text = stderr.decode("utf-8", errors="replace")
        (voice_outdir / "stderr.log").write_text(stderr_text)
        if proc.returncode != 0:
            return {"voice": voice_id, "error": f"exit={proc.returncode}: {stderr_text[:1500]}", "elapsed_s": elapsed}
        # Confirm files exist
        missing = [
            f for f in ("rcv.qnt", "main.qnt", "design-notes.md")
            if not (voice_outdir / f).exists()
        ]
        if missing:
            return {"voice": voice_id, "error": f"missing output files: {missing}", "elapsed_s": elapsed}
        return {"voice": voice_id, "elapsed_s": elapsed, "out_dir": str(voice_outdir)}
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"voice": voice_id, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


def validate_voice_spec(voice_outdir: Path) -> dict:
    """Run quint typecheck + invariant + witness checks. Returns dict per check."""
    main_qnt = voice_outdir / "main.qnt"
    if not main_qnt.exists():
        return {"typecheck": "missing main.qnt", "all_invariants": None, "witnesses": {}}

    result = {}
    # typecheck
    r = subprocess.run(
        ["quint", "typecheck", str(main_qnt)],
        capture_output=True, text=True, timeout=120,
    )
    result["typecheck"] = {"exit": r.returncode, "stderr": r.stderr[-500:] if r.stderr else ""}

    # all_invariants — should HOLD (no violation)
    r = subprocess.run(
        ["quint", "run", f"--invariant={SAFETY_INVARIANT}",
         "--max-steps=30", "--max-samples=100", str(main_qnt)],
        capture_output=True, text=True, timeout=300,
    )
    out = (r.stdout or "") + (r.stderr or "")
    holds = "No violation found" in out
    result[SAFETY_INVARIANT] = {
        "exit": r.returncode,
        "holds": holds,
        "expected": "holds (no violation)",
        "tail": out[-400:],
    }

    # witnesses — each should be VIOLATED
    result["witnesses"] = {}
    for w in WITNESS_SPECS:
        r = subprocess.run(
            ["quint", "run", f"--invariant={w}",
             "--max-steps=30", "--max-samples=100", str(main_qnt)],
            capture_output=True, text=True, timeout=300,
        )
        out = (r.stdout or "") + (r.stderr or "")
        violated = "Invariant violated" in out or "Found an issue" in out
        result["witnesses"][w] = {
            "exit": r.returncode,
            "violated": violated,
            "expected": "violated (reachability witness)",
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


async def dispatch_voice(voice_id: str, model_id: str, log_path: Path) -> dict:
    """One voice: generate + validate, with one retry on hard failure."""
    log_path.parent.mkdir(parents=True, exist_ok=True)
    with log_path.open("a") as logf:
        _log_both(f"→ voice {voice_id}: dispatching", logf)
        for attempt in range(MAX_RETRIES + 1):
            gen = await dispatch_one(voice_id, model_id, attempt)
            if "error" not in gen:
                if attempt > 0:
                    _log_both(f"    ↻ {voice_id} generated on attempt {attempt + 1}", logf)
                break
            _log_both(f"    ⚠ {voice_id} attempt {attempt + 1} failed ({gen['elapsed_s']:.0f}s): {gen['error'][:200]}", logf)
        if "error" in gen:
            _log_both(f"  ✗ {voice_id} ABANDONED after {MAX_RETRIES + 1} attempts", logf)
            return {"voice": voice_id, "generation": gen, "validation": None, "passes": False}

        _log_both(f"  ↳ {voice_id} generated in {gen['elapsed_s']:.0f}s, validating...", logf)
        validation = validate_voice_spec(Path(gen["out_dir"]))
        passes = voice_passes_all(validation)
        # Write per-voice validation manifest
        (Path(gen["out_dir"]) / "validation.json").write_text(
            json.dumps(validation, indent=2, default=str)
        )
        status = "✓ PASS" if passes else "✗ VALIDATION FAILED"
        _log_both(f"  {status} {voice_id}: typecheck={validation.get('typecheck', {}).get('exit')} "
                  f"safety_holds={validation.get(SAFETY_INVARIANT, {}).get('holds')} "
                  f"witnesses_violated={sum(1 for r in validation.get('witnesses', {}).values() if r.get('violated'))}/{len(WITNESS_SPECS)}",
                  logf)
        return {
            "voice": voice_id,
            "generation": gen,
            "validation": validation,
            "passes": passes,
        }


async def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", default=None, help="comma-separated voice ids (default: all)")
    ap.add_argument("--sequential", action="store_true", help="run voices sequentially (default: parallel — but generation is heavy so sequential may be safer with LM Studio voices)")
    args = ap.parse_args()

    selected = VOICES
    if args.voices:
        wanted = set(args.voices.split(","))
        selected = [v for v in VOICES if v[0] in wanted]

    log_path = OUTDIR / "dispatch.log"
    with log_path.open("a") as logf:
        logf.write(f"\n=== Quint fan-out dispatch starting {datetime.now(timezone.utc).isoformat()} ===\n")
        logf.write(f"Voices: {[v[0] for v in selected]}\n")
        logf.write(f"Output: {OUTDIR}\n\n")

    print(f"Output dir: {OUTDIR}")
    print(f"Voices ({len(selected)}): {[v[0] for v in selected]}")

    if args.sequential:
        all_results = []
        for voice_id, model_id, _note in selected:
            r = await dispatch_voice(voice_id, model_id, log_path)
            all_results.append(r)
    else:
        tasks = [dispatch_voice(v, m, log_path) for v, m, _n in selected]
        all_results = await asyncio.gather(*tasks, return_exceptions=False)

    manifest = {
        "run_tag": RUN_TAG,
        "intent": str(INTENT),
        "canonical_examples": CANONICAL_EXAMPLES,
        "witness_specs": WITNESS_SPECS,
        "safety_invariant": SAFETY_INVARIANT,
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
            err = r.get("generation", {}).get("error") if "error" in r.get("generation", {}) else "validation failed (see per-voice validation.json)"
            print(f"  ✗ {v}: {err[:200] if isinstance(err, str) else err}")


if __name__ == "__main__":
    asyncio.run(main())
