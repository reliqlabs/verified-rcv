#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Multi-voice Lean math spec generation dispatch.

Fans out the `lean-spec-generator` agent across 4 frontier voices. Each
voice produces its own Lean encoding of the verified-rcv intent's §2.5
types + Tally_spec composition + §3.1 well-formedness theorems + §3.2
B10_lean image-IO obligation. Proof bodies are all `sorry`; the
methodology question is whether the statements encode the intent.

The Claude-authored baseline lives at specs/RcvSpec.lean and is NOT
re-dispatched (same pattern as the Quint canonical at specs/rcv.qnt).

Voice roster: 4 frontier voices (no local — Lean spec authoring is
sensitive to model quality on math-formal-language tasks). Cross-critique
+ defense + re-cross-critique (Asks O–R) run as a follow-up pass after
fan-out lands.

USAGE
    uv run --script lean_spec_dispatch.py [--voices=A,B,C] [--sequential]

OUTPUT
    .colosseum/specs/<run-tag>/
    ├── per-voice/
    │   └── <voice-id>/
    │       ├── RcvSpec.lean
    │       ├── design-notes.md
    │       ├── stdout.log
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

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
INTENT = REPO / ".colosseum" / "intent.md"
CANONICAL_LEAN = REPO / "specs" / "RcvSpec.lean"

PER_CALL_TIMEOUT = 2400  # 40 min — generation + self-typecheck loop
MAX_RETRIES = 1

RUN_TAG_ENV = os.environ.get("COLOSSEUM_LEAN_RUN_TAG")
if RUN_TAG_ENV:
    RUN_TAG = RUN_TAG_ENV
else:
    _ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    RUN_TAG = f"lean-fanout-{_ts}"
OUTDIR = REPO / ".colosseum" / "specs" / RUN_TAG
OUTDIR.mkdir(parents=True, exist_ok=True)
(OUTDIR / "per-voice").mkdir(exist_ok=True)

# Required symbols — at least these names (or voice-documented equivalents)
# should be present in each spec for the cross-critique to have anchors.
REQUIRED_THEOREMS = [
    "s6_winner_subset",
    "s7_voter_partition",
    "s8_round_counts_sum",
    "s9_no_reappearance",
    "B10_lean",
]
REQUIRED_SYMBOLS = ["EnclaveImage", "Tally_spec"]

# ─────────────────────────────────────────────────────────────────────────
# Voice roster — 4 frontier voices. Claude excluded (baseline is
# specs/RcvSpec.lean). No local voices on this pass — Lean is too
# sensitive to model quality on math-formal-language work.
# Nemotron parked pending the upstream-stall investigation.
# ─────────────────────────────────────────────────────────────────────────

VOICES = [
    ("kimi-k2-6",                 "burnt/kimi-k2-6",                  "Moonshot — frontier; gateway"),
    ("gpt-5-5-native",            "openai/gpt-5.5",                   "OpenAI — native gpt-5.5"),
    ("gemini-2-5-flash-native",   "google/gemini-2.5-flash",          "Google — native gemini 2.5 flash"),
    ("magistral-medium-native",   "mistral/magistral-medium-latest",  "Mistral — magistral-medium (Lean lineage)"),
]


def build_message(voice_id: str, voice_outdir: Path) -> str:
    return f"""VOICE_ID: {voice_id}

INTENT_PATH: {INTENT}

OUTPUT_DIR: {voice_outdir}

SPEC_FILENAME: RcvSpec.lean

CANONICAL_LEAN_EXAMPLE: {CANONICAL_LEAN}

REQUIRED_THEOREMS (each must be stated with `sorry` body, NOT discharged by `rfl`/`trivial`):
  - s6_winner_subset      — §3.1 winner well-formedness
  - s7_voter_partition    — §3.1 voter partition (conservation eqn)
  - s8_round_counts_sum   — §3.1 per-round counts sum to ballots_tallied
  - s9_no_reappearance    — §3.1 eliminated candidates do not reappear
  - B10_lean              — §3.2 EnclaveImage = Tally_spec image-IO obligation

REQUIRED_SYMBOLS:
  - EnclaveImage  (declared as `axiom` or `opaque`, NEVER `def EnclaveImage := Tally_spec`)
  - Tally_spec    (declared as `def`, composes Stage 1 + Stage 2)

Produce {voice_outdir}/RcvSpec.lean + {voice_outdir}/design-notes.md.

After writing, self-verify by running:
  lean {voice_outdir}/RcvSpec.lean

It must exit 0 with only `sorry` warnings (no errors).

Emit STATUS: ok or STATUS: error: <reason> as the last line of output.
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
    message = build_message(voice_id, voice_outdir)
    cmd = [
        "opencode", "run",
        "--agent", "lean-spec-generator",
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
        missing = [f for f in ("RcvSpec.lean", "design-notes.md") if not (voice_outdir / f).exists()]
        if missing:
            return {"voice": voice_id, "error": f"missing output files: {missing}", "elapsed_s": elapsed}
        return {"voice": voice_id, "elapsed_s": elapsed, "out_dir": str(voice_outdir)}
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"voice": voice_id, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


def validate_voice_spec(voice_outdir: Path) -> dict:
    """Run lean typecheck + presence checks. Returns dict per check."""
    spec_path = voice_outdir / "RcvSpec.lean"
    if not spec_path.exists():
        return {"typecheck": "missing RcvSpec.lean", "presence": {}, "no_tautologies": None}

    result = {}
    # typecheck — lean exits 0 even with sorry warnings; only true errors fail
    r = subprocess.run(
        ["lean", str(spec_path)],
        capture_output=True, text=True, timeout=600,
    )
    out = (r.stdout or "") + (r.stderr or "")
    has_errors = bool(re.search(r":\s*error:", out))
    result["typecheck"] = {
        "exit": r.returncode,
        "has_errors": has_errors,
        "sorry_count": len(re.findall(r"declaration uses `sorry`", out)),
        "tail": out[-800:],
    }

    # Presence — required theorems + symbols appear as declarations
    text = spec_path.read_text()
    presence = {}
    for name in REQUIRED_THEOREMS + REQUIRED_SYMBOLS:
        # match `theorem|axiom|opaque|def <name>` ignoring whitespace
        pattern = rf"^\s*(theorem|axiom|opaque|def)\s+{re.escape(name)}\b"
        presence[name] = bool(re.search(pattern, text, re.MULTILINE))
    result["presence"] = presence

    # Tautology check — no theorem in REQUIRED_THEOREMS discharged by rfl/trivial
    tautology_hits = []
    for name in REQUIRED_THEOREMS:
        # match `theorem <name> ... := rfl|trivial|True.intro` on same or next few lines
        pattern = rf"theorem\s+{re.escape(name)}[^\n]*(?:\n[^\n]*?){{0,8}}?:=\s*(rfl|trivial|True\.intro|by\s+(rfl|trivial))\b"
        if re.search(pattern, text):
            tautology_hits.append(name)
    # Also flag: EnclaveImage defined as a def equal to Tally_spec
    enclave_image_tautology = bool(
        re.search(r"^\s*def\s+EnclaveImage\b[^\n]*:=\s*Tally_spec\b", text, re.MULTILINE)
    )
    result["no_tautologies"] = {
        "tautology_hits": tautology_hits,
        "enclave_image_is_tally_spec": enclave_image_tautology,
    }

    return result


def voice_passes_all(validation: dict) -> bool:
    tc = validation.get("typecheck")
    if not isinstance(tc, dict) or tc.get("has_errors") or tc.get("exit") != 0:
        return False
    presence = validation.get("presence", {})
    if not all(presence.get(n) for n in REQUIRED_THEOREMS + REQUIRED_SYMBOLS):
        return False
    nt = validation.get("no_tautologies", {})
    if nt.get("tautology_hits") or nt.get("enclave_image_is_tally_spec"):
        return False
    return True


async def dispatch_voice(voice_id: str, model_id: str, log_path: Path) -> dict:
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
        (Path(gen["out_dir"]) / "validation.json").write_text(
            json.dumps(validation, indent=2, default=str)
        )
        status = "✓ PASS" if passes else "✗ VALIDATION FAILED"
        tc = validation.get("typecheck", {})
        presence = validation.get("presence", {})
        nt = validation.get("no_tautologies", {})
        _log_both(
            f"  {status} {voice_id}: typecheck_errors={tc.get('has_errors')} "
            f"sorries={tc.get('sorry_count')} "
            f"present={sum(presence.values())}/{len(presence)} "
            f"tautologies={len(nt.get('tautology_hits', []))} "
            f"enclave_eq_tally={nt.get('enclave_image_is_tally_spec')}",
            logf,
        )
        return {
            "voice": voice_id,
            "generation": gen,
            "validation": validation,
            "passes": passes,
        }


async def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", default=None, help="comma-separated voice ids (default: all)")
    ap.add_argument("--sequential", action="store_true", help="run voices sequentially")
    args = ap.parse_args()

    selected = VOICES
    if args.voices:
        wanted = set(args.voices.split(","))
        selected = [v for v in VOICES if v[0] in wanted]

    log_path = OUTDIR / "dispatch.log"
    with log_path.open("a") as logf:
        logf.write(f"\n=== Lean fan-out dispatch starting {datetime.now(timezone.utc).isoformat()} ===\n")
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
        "canonical_lean": str(CANONICAL_LEAN),
        "required_theorems": REQUIRED_THEOREMS,
        "required_symbols": REQUIRED_SYMBOLS,
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
            err = r.get("generation", {}).get("error") if "error" in (r.get("generation") or {}) else "validation failed (see per-voice validation.json)"
            print(f"  ✗ {v}: {err[:200] if isinstance(err, str) else err}")


if __name__ == "__main__":
    asyncio.run(main())
