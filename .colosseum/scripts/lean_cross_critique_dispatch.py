#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# requires-python = ">=3.11"
# ///
"""
Cross-critique dispatch for the multi-voice Lean math specs.

For each (reviewer, target) pair where reviewer != target:
  1. Build a Q1/Q2/Q3 critique prompt that inlines target spec + design-notes
     and the reviewer's own spec for context.
  2. Invoke opencode with --variant high.
  3. Capture the reviewer's structured critique.

3 voices passing the fan-out: kimi, gpt-5.5, magistral. Full pairwise
coverage = 6 pairs. Each pair lands one critique.md.

Output:
  .colosseum/specs/lean-cross-critique-2026-05-20/<reviewer>-reviews-<target>/
    ├── prompt.md
    ├── critique.md
    ├── stdout.log
    └── stderr.log
"""

from __future__ import annotations
import asyncio
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
INTENT = REPO / ".colosseum/intent.md"
CANONICAL_LEAN = REPO / "specs/RcvSpec.lean"
FANOUT_DIR = REPO / ".colosseum/specs/lean-fanout-2026-05-19T201426Z/per-voice"
OUTDIR = REPO / ".colosseum/specs/lean-cross-critique-2026-05-20"
OUTDIR.mkdir(parents=True, exist_ok=True)

# Voices participating. Each entry: (id, model, voice_dir)
VOICES = [
    ("kimi-k2-6",
     "burnt/kimi-k2-6",
     FANOUT_DIR / "kimi-k2-6"),
    ("gpt-5-5-native",
     "openai/gpt-5.5",
     FANOUT_DIR / "gpt-5-5-native"),
    ("magistral-medium-native",
     "mistral/magistral-medium-latest",
     FANOUT_DIR / "magistral-medium-native"),
]

PER_CALL_TIMEOUT = 1800  # 30 min — single-turn but reasoning is heavy


def load_spec(voice_dir: Path) -> dict[str, str]:
    out = {}
    for name in ("RcvSpec.lean", "design-notes.md"):
        p = voice_dir / name
        out[name] = p.read_text() if p.exists() else "(file missing)"
    return out


def build_critique_message(reviewer_id: str, reviewer_files: dict, target_id: str, target_files: dict) -> str:
    pair_outpath = OUTDIR / f"{reviewer_id}-reviews-{target_id}" / "critique.md"
    return f"""You are participating in a multi-voice Lean math-spec convergence experiment for verified-rcv. Three voices each produced a Lean math-layer spec from the same intent at /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md (v0.3.2). All three specs pass `lean RcvSpec.lean` (no errors; only `sorry` warnings) and present the required 5 theorem statements + EnclaveImage axiom + Tally_spec composition.

Your role in this turn: REVIEW another voice's spec as a peer reviewer. You are NOT generating a spec. You are reading two specs (yours and another voice's) and producing a structured critique.

Your voice id: {reviewer_id}
Target voice id: {target_id}

Both specs were generated against the same intent. Key obligations:
- Stage 1 `decrypt_and_validate` and Stage 2 `IRV_spec` are opaque (modeled, not implemented)
- `Tally_spec` is defined as the composition of Stage 1 + Stage 2
- `EnclaveImage` is `axiom` or `opaque` — NEVER `def EnclaveImage := Tally_spec` (instantiating as Tally_spec discharges B10_lean by rfl, a tautological-shadow defect)
- Five theorem statements: s6_winner_subset, s7_voter_partition, s8_round_counts_sum, s9_no_reappearance, B10_lean — each with `:= by sorry` body (NOT `:= rfl` or `:= trivial`)
- §3.1 + §3.2 of the intent state what each theorem must encode

Your spec (for context, no need to defend line-by-line unless asked):

===YOUR RcvSpec.lean===
{reviewer_files["RcvSpec.lean"]}
===END===

===YOUR design-notes.md===
{reviewer_files["design-notes.md"]}
===END===

Target spec to review:

===TARGET RcvSpec.lean===
{target_files["RcvSpec.lean"]}
===END===

===TARGET design-notes.md===
{target_files["design-notes.md"]}
===END===

Read the target spec carefully. You may use your read / bash / grep / glob tools to consult the intent at /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md or to run `lean <target_path>/RcvSpec.lean` yourself. You do not need to use those tools if your analysis can stand on its own.

Then produce a structured critique. Write the critique to {pair_outpath} with these sections in this exact order:

# Cross-critique: {reviewer_id} reviews {target_id}

## Q1. Most material structural divergence

State the single divergence you consider most material between your spec and the target. Be specific: which type, definition, or theorem differs, and why the difference matters for what the spec is claiming. Common divergence axes: type representation of `per_round_counts` (alist of tuples vs struct vs Map vs Finset), `IRV_spec` signature (opaque vs concrete vs partial), out-of-bounds list access encoding (`[i]!` with bounds proof vs `[i]?` with Option pattern-match), Stage 1 return type shape, EnclaveImage typing. Pick one and defend it.

## Q2. Apparent defect in target spec

The target typechecks and presents the required theorems with `sorry` bodies. But typecheck-clean does not mean intent-faithful. Identify one defect, if any: a place where the target spec is technically valid but trivially encoded, semantically wrong, or violates an intent obligation in a way the typechecker cannot catch. Examples:

  - A theorem statement that holds vacuously — e.g., a quantifier whose body is `True` or whose bounds make the statement empty
  - A theorem stated with `:= rfl` or `:= trivial` on the required 5 (look for these even if they typecheck)
  - `EnclaveImage` defined in a way that makes B10_lean trivial (`opaque EnclaveImage := fun _ _ _ => Tally_spec _ _ _`-style indirection, or `axiom EnclaveImage : ... := Tally_spec` via the `=` definition)
  - A `Tally_spec` definition that doesn't actually compose Stage 1 + Stage 2 (e.g., ignores `decrypt_and_validate`'s dropped_voters bookkeeping)
  - A theorem whose statement misses a universal quantifier the intent requires
  - A theorem whose statement encodes a strict bound where the intent permits equality, or vice versa
  - A missing or misnamed obligation

If you find no defect, say so explicitly with one sentence of reasoning.

## Q3. Change to your own spec after reading target

Now identify one change, if any, you would make to YOUR spec after seeing the target. State the change concretely (line, type, theorem) and explain what the target encoded better. If no change is warranted, state that with one sentence.

## Optional notes

Anything else worth recording. Keep this under 200 words. Useful items: methodology observations, places where the intent itself is unclear (encoding-discipline note candidates), places where the target's choice would be better than yours but the reverse holds for a different consideration, places where both specs have the same defect (a sign the intent under-specifies that axis).

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
        "--agent", "lean-spec-generator",
        "--model", reviewer_model,
        "--variant", "high",
        "--format", "default",
        "--dangerously-skip-permissions",
        message,
    ]
    t0 = datetime.now(timezone.utc)
    label = f"{reviewer_id} -> {target_id}"
    print(f"[{datetime.now(timezone.utc).strftime('%H:%M:%S')}Z] {label} dispatching", flush=True)
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
            print(f"  ✗ {label} TIMEOUT after {elapsed:.0f}s", flush=True)
            return {"reviewer": reviewer_id, "target": target_id, "error": f"timeout after {PER_CALL_TIMEOUT}s"}
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        content = stdout.decode("utf-8", errors="replace")
        (pair_dir / "stdout.log").write_text(content)
        stderr_text = stderr.decode("utf-8", errors="replace")
        (pair_dir / "stderr.log").write_text(stderr_text)
        if proc.returncode != 0:
            print(f"  ✗ {label} exit={proc.returncode}", flush=True)
            return {"reviewer": reviewer_id, "target": target_id, "error": f"exit={proc.returncode}", "elapsed_s": elapsed}
        critique_path = pair_dir / "critique.md"
        if critique_path.exists():
            print(f"  ✓ {label} done in {elapsed:.0f}s ({critique_path.stat().st_size} bytes)", flush=True)
            return {"reviewer": reviewer_id, "target": target_id, "elapsed_s": elapsed, "critique": str(critique_path)}
        print(f"  ⚠ {label} completed but no critique.md found", flush=True)
        return {"reviewer": reviewer_id, "target": target_id, "elapsed_s": elapsed, "error": "no critique.md emitted"}
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"reviewer": reviewer_id, "target": target_id, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


async def main():
    pairs = []
    for i, reviewer in enumerate(VOICES):
        for j, target in enumerate(VOICES):
            if i == j:
                continue
            pairs.append((reviewer, target))
    print(f"Output dir: {OUTDIR}")
    print(f"Pairs ({len(pairs)}): {[(p[0][0], p[1][0]) for p in pairs]}")
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
