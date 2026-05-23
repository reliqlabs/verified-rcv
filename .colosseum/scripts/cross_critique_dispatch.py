#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# requires-python = ">=3.11"
# ///
"""
Cross-critique dispatch for the v11 multi-voice Quint specs.

For each (reviewer, target) pair where reviewer != target:
  1. Build a critique prompt that inlines the target spec and the reviewer's
     own spec for context.
  2. Invoke opencode with --variant high to apply maximum reasoning effort.
  3. Capture the reviewer's structured response.

Output: per-pair Markdown reports under
  .colosseum/specs/cross-critique-2026-05-19/<reviewer>-reviews-<target>/

Cross-critique is single-shot per pair — the reviewer agent is the existing
quint-spec-generator agent but the user message instructs it to critique
rather than generate.
"""

from __future__ import annotations
import asyncio, os, subprocess
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
INTENT = REPO / ".colosseum/intent.md"
OUTDIR = REPO / ".colosseum/specs/cross-critique-2026-05-19"
OUTDIR.mkdir(parents=True, exist_ok=True)

# Voices participating in cross-critique. Each entry: (id, model, v11_dir)
VOICES = [
    ("kimi-k2-6",
     "burnt/kimi-k2-6",
     REPO / ".colosseum/specs/v11-intent-tightened/per-voice/kimi-k2-6"),
    ("gpt-5-5-native",
     "openai/gpt-5.5",
     REPO / ".colosseum/specs/v11-intent-tightened/per-voice/gpt-5-5-native"),
]

PER_CALL_TIMEOUT = 1800  # 30 min — critique is single-turn, but reasoning takes wall


def load_spec(voice_dir: Path) -> dict[str, str]:
    out = {}
    for name in ("rcv.qnt", "main.qnt", "design-notes.md"):
        p = voice_dir / name
        out[name] = p.read_text() if p.exists() else "(file missing)"
    return out


def build_critique_message(reviewer_id: str, reviewer_files: dict, target_id: str, target_files: dict) -> str:
    return f"""You are participating in a multi-voice Quint spec convergence experiment for verified-rcv. Three voices each produced a protocol-layer Quint spec from the same intent document at /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md (v0.3.2). Both specs you will see in this message pass `quint typecheck`, `quint run --invariant=all_invariants` (no violation), and all three witness reachability invariants are violated as required.

Your role in this turn: REVIEW another voice's spec as a peer reviewer. You are NOT generating a spec. You are reading two specs (yours and another voice's) and producing a structured critique.

Your voice id: {reviewer_id}
Target voice id: {target_id}

Both specs have been generated against the same intent. The intent specifies (among other things):
- Block 1 (instantiate) parameters are bound at instantiate time and modeled as const
- Protocol model MUST include enclave-side state for B10 chain-side projection
- B2 MUST be encoded as a checkable invariant over a snapshot variable

Your spec (for context, no need to defend it line-by-line unless asked):

===YOUR rcv.qnt===
{reviewer_files["rcv.qnt"]}
===END===

===YOUR main.qnt===
{reviewer_files["main.qnt"]}
===END===

Target spec to review:

===TARGET rcv.qnt===
{target_files["rcv.qnt"]}
===END===

===TARGET main.qnt===
{target_files["main.qnt"]}
===END===

===TARGET design-notes.md===
{target_files["design-notes.md"]}
===END===

Read the target spec carefully. You may use your read / bash / grep / glob tools to consult the intent at /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md or to run `quint typecheck` and `quint run --invariant=all_invariants` against the target spec yourself to verify its claims. You do not need to use those tools if your analysis can stand on its own.

Then produce a structured critique. Write the critique to /Users/mvid/Development/reliq/verified-rcv/.colosseum/specs/cross-critique-2026-05-19/{reviewer_id}-reviews-{target_id}/critique.md with these sections in this exact order:

# Cross-critique: {reviewer_id} reviews {target_id}

## Q1. Most material structural divergence

State the single divergence you consider most material between your spec and the target. Be specific: which state variable, action, or invariant differs, and why the difference matters for what the spec is claiming. Do not list multiple divergences here; pick one and defend it.

## Q2. Apparent defect in target spec

The target typechecks and passes validation. But typecheck-clean does not mean intent-faithful. Identify one defect, if any: a place where the target spec is technically valid but trivially encoded, semantically wrong, or violates an intent obligation in a way the typechecker cannot catch. Examples include: a behavioral invariant declared as a tautological `true` shadow that should be a real predicate, a witness invariant whose negation does not actually express reachability of the named state, an action guard that admits behaviors the intent forbids, or a state variable whose value is never constrained by any action. If you find no defect, say so explicitly with one sentence of reasoning.

## Q3. Change to your own spec after reading target

Now identify one change, if any, you would make to YOUR spec after seeing the target. State the change concretely (line, variable, predicate) and explain what the target encoded better. If no change is warranted, state that with one sentence.

## Optional notes

Anything else worth recording. Keep this under 200 words. Useful items: methodology observations, places where the intent itself is unclear, places where the target's choice would be better than yours but the reverse holds for a different consideration, etc.

Stop after writing the critique. Emit STATUS: ok on the last line of your output, or STATUS: error: <reason> if you could not produce the critique."""


async def dispatch_one(reviewer: tuple, target: tuple) -> dict:
    reviewer_id, reviewer_model, reviewer_dir = reviewer
    target_id, target_model, target_dir = target
    pair_dir = OUTDIR / f"{reviewer_id}-reviews-{target_id}"
    pair_dir.mkdir(parents=True, exist_ok=True)
    reviewer_files = load_spec(reviewer_dir)
    target_files = load_spec(target_dir)
    message = build_critique_message(reviewer_id, reviewer_files, target_id, target_files)
    (pair_dir / "prompt.md").write_text(message)
    cmd = [
        "opencode", "run",
        "--agent", "quint-spec-generator",
        "--model", reviewer_model,
        "--variant", "high",
        "--format", "default",
        "--dangerously-skip-permissions",
        message,
    ]
    t0 = datetime.now(timezone.utc)
    print(f"[{datetime.now(timezone.utc).strftime('%H:%M:%S')}Z] {reviewer_id} -> {target_id} dispatching", flush=True)
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
            print(f"  ✗ {reviewer_id} -> {target_id} TIMEOUT after {elapsed:.0f}s", flush=True)
            return {"reviewer": reviewer_id, "target": target_id, "error": f"timeout after {PER_CALL_TIMEOUT}s"}
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        content = stdout.decode("utf-8", errors="replace")
        (pair_dir / "stdout.log").write_text(content)
        stderr_text = stderr.decode("utf-8", errors="replace")
        (pair_dir / "stderr.log").write_text(stderr_text)
        if proc.returncode != 0:
            print(f"  ✗ {reviewer_id} -> {target_id} exit={proc.returncode}", flush=True)
            return {"reviewer": reviewer_id, "target": target_id, "error": f"exit={proc.returncode}", "elapsed_s": elapsed}
        critique_path = pair_dir / "critique.md"
        if critique_path.exists():
            print(f"  ✓ {reviewer_id} -> {target_id} done in {elapsed:.0f}s ({critique_path.stat().st_size} bytes)", flush=True)
            return {"reviewer": reviewer_id, "target": target_id, "elapsed_s": elapsed, "critique": str(critique_path)}
        print(f"  ⚠ {reviewer_id} -> {target_id} completed but no critique.md found", flush=True)
        return {"reviewer": reviewer_id, "target": target_id, "elapsed_s": elapsed, "error": "no critique.md emitted"}
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"reviewer": reviewer_id, "target": target_id, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


async def main():
    # Two pairs: kimi reviews gpt-5.5, gpt-5.5 reviews kimi. Run in parallel.
    pairs = []
    for i, reviewer in enumerate(VOICES):
        for j, target in enumerate(VOICES):
            if i == j:
                continue
            pairs.append((reviewer, target))
    print(f"Output dir: {OUTDIR}")
    print(f"Pairs: {[(p[0][0], p[1][0]) for p in pairs]}")
    print(f"Variant: high reasoning effort per voice")
    results = await asyncio.gather(*[dispatch_one(r, t) for r, t in pairs], return_exceptions=False)
    print("\n=== Summary ===")
    for r in results:
        if "error" in r:
            print(f"  ✗ {r['reviewer']} -> {r['target']}: {r['error']}")
        else:
            print(f"  ✓ {r['reviewer']} -> {r['target']}: {r['critique']}")


if __name__ == "__main__":
    asyncio.run(main())
