#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# requires-python = ">=3.11"
# ///
"""
Re-cross-critique of the revised canonical specs/RcvSpec.lean.

After the cross-critique (2026-05-20) caught a convergent composition
defect (Stage 1/2 type conflation; non_voters not threaded) and an
encoding upgrade (Option-pattern for S9), the canonical was revised.

This run dispatches each of the 3 voices to review the REVISED canonical
against their own fan-out spec, and identify (Q1) whether the fix is
structurally sound, (Q2) any new defects the revision introduced, (Q3)
remaining concerns.

Same harness as the cross-critique. Variant high.
"""

from __future__ import annotations
import asyncio
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
INTENT = REPO / ".colosseum/intent.md"
CANONICAL_LEAN = REPO / "specs/RcvSpec.lean"
FANOUT_DIR = REPO / ".colosseum/specs/lean-fanout-2026-05-19T201426Z/per-voice"
OUTDIR = REPO / ".colosseum/specs/lean-critique-revised-canonical-2026-05-20"
OUTDIR.mkdir(parents=True, exist_ok=True)

VOICES = [
    ("kimi-k2-6",                "burnt/kimi-k2-6",                  FANOUT_DIR / "kimi-k2-6"),
    ("gpt-5-5-native",           "openai/gpt-5.5",                   FANOUT_DIR / "gpt-5-5-native"),
    ("magistral-medium-native",  "mistral/magistral-medium-latest",  FANOUT_DIR / "magistral-medium-native"),
]

PER_CALL_TIMEOUT = 1800


def load_spec(voice_dir: Path) -> dict[str, str]:
    out = {}
    for name in ("RcvSpec.lean", "design-notes.md"):
        p = voice_dir / name
        out[name] = p.read_text() if p.exists() else "(file missing)"
    return out


def build_message(voice_id: str, voice_files: dict) -> str:
    canonical_text = CANONICAL_LEAN.read_text()
    out_path = OUTDIR / f"{voice_id}-reviews-revised-canonical" / "critique.md"
    return f"""You are participating in a multi-voice Lean math-spec convergence experiment for verified-rcv. The canonical spec at /Users/mvid/Development/reliq/verified-rcv/specs/RcvSpec.lean was just revised in response to a cross-critique round that identified two findings:

1. **Composition defect**: prior canonical's `Tally_spec` did not thread `non_voters` from Stage 1 — its `DecryptedSet` lacked the field, and `IRV_spec` (which only sees `d.valid`) had no way to compute `non_voters = candidates \\ raw_ballots.keys` per intent §2.5. Fix: split Stage 2 output into a separate `IRVResult` type (only `winners + per_round_counts + eliminated_by_round + ballots_tallied`); added `non_voters` to `DecryptedSet`; `Tally_spec` now constructs `TallyResult` by composing both.

2. **Encoding upgrade**: prior canonical's `s9_no_reappearance` used `[i]!`-with-bounds-check; revised version uses `[i]? = some ...`-Option-pattern (gpt-5-5's encoding). Convergent finding across reviewers.

Your task: review the REVISED canonical to verify the fix is structurally sound and to identify any new defects the revision may have introduced.

Your voice id: {voice_id}

You produced your own Lean spec during fan-out (shown below for context). The revised canonical is also shown. Both typecheck cleanly under `lean`.

Intent doc: /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md (v0.3.2).

Your fan-out spec for context:

===YOUR RcvSpec.lean===
{voice_files["RcvSpec.lean"]}
===END===

===YOUR design-notes.md===
{voice_files["design-notes.md"]}
===END===

Revised canonical:

===REVISED CANONICAL RcvSpec.lean===
{canonical_text}
===END===

Read the revised canonical carefully. You may use your read / bash / grep / glob tools to consult the intent or run `lean` checks. The revisions are documented in the canonical's header comment block.

Write your critique to {out_path} with these sections:

# Re-critique of revised canonical: {voice_id}

## Q1. Is the fix structurally sound?

Verify the revised encoding actually addresses the cross-critique findings:

- `IRVResult` is a distinct structure that contains ONLY the IRV-core fields (winners, per_round_counts, eliminated_by_round, ballots_tallied).
- `IRV_spec` returns `IRVResult`, not `TallyResult`.
- `DecryptedSet` now contains `non_voters` as a field.
- `Tally_spec` composes `IRVResult` + `DecryptedSet` (threading dropped, dropped_voters, non_voters from Stage 1; threading winners, per_round_counts, eliminated_by_round, ballots_tallied from Stage 2; computing ballots_dropped from d.dropped.length).
- `s9_no_reappearance` uses the `[i]? = some ...` Option pattern (no `[i]!` left).

Is the revised encoding correct? If not, state the specific defect.

## Q2. Did the revision introduce any new defects?

Look for:

- A field on `Tally_spec`'s output that is no longer well-defined after the refactor (e.g., if `Tally_spec` no longer threads some field correctly).
- Theorem statements that now reference fields with the wrong source (e.g., `s7_voter_partition` was about `t.non_voters`; verify that `t.non_voters` in the revised `Tally_spec` is the Stage-1-sourced `d.non_voters`, not the prior opaque inheritance).
- An off-by-one or fence-post error in the new Option-pattern S9 statement (was `i < j` retained correctly; do the hypotheses match the intent's strict-inequality requirement).
- A type that derives `Inhabited` but should not (or vice versa).
- A regression from the prior canonical that the cross-critique didn't anticipate.

State at most one new defect. If none, say so.

## Q3. Remaining concerns

The cross-critique surfaced 3 intent-level under-specification candidates that the revision did NOT address: (a) CandidateSet distinctness not encoded at type level, (b) `ballots_tallied = d.valid.length` not axiomatized, (c) "voters = candidates" identification implicit not explicit. Are any of these load-bearing enough that the revision should have addressed them? Or are there other axes where the revised canonical is still under-encoded? Pick the single most material remaining concern.

## Optional notes

Anything else (under 200 words). Useful: methodology observations, places where the revision is better than your spec on a different axis than what the cross-critique covered, encoding-discipline candidates for intent v0.3.3.

Stop with STATUS: ok or STATUS: error: <reason>."""


async def dispatch_one(voice: tuple) -> dict:
    voice_id, model_id, voice_dir = voice
    out_dir = OUTDIR / f"{voice_id}-reviews-revised-canonical"
    out_dir.mkdir(parents=True, exist_ok=True)
    voice_files = load_spec(voice_dir)
    message = build_message(voice_id, voice_files)
    (out_dir / "prompt.md").write_text(message)
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
    print(f"[{datetime.now(timezone.utc).strftime('%H:%M:%S')}Z] {voice_id} dispatching", flush=True)
    try:
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE, cwd=str(REPO)
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=PER_CALL_TIMEOUT)
        except asyncio.TimeoutError:
            proc.kill()
            elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
            print(f"  ✗ {voice_id} TIMEOUT after {elapsed:.0f}s", flush=True)
            return {"voice": voice_id, "error": f"timeout after {PER_CALL_TIMEOUT}s"}
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        (out_dir / "stdout.log").write_bytes(stdout)
        (out_dir / "stderr.log").write_bytes(stderr)
        critique_path = out_dir / "critique.md"
        # Fallback: salvage critique from stdout if file not written
        if not critique_path.exists():
            text = stdout.decode("utf-8", errors="replace")
            import re
            m = re.search(r"^# Re-critique of revised canonical:.*?STATUS: (ok|error.*)$",
                          text, re.MULTILINE | re.DOTALL)
            if m:
                critique_path.write_text(m.group(0))
        if critique_path.exists():
            print(f"  ✓ {voice_id} done in {elapsed:.0f}s ({critique_path.stat().st_size} bytes)", flush=True)
            return {"voice": voice_id, "elapsed_s": elapsed, "critique": str(critique_path)}
        print(f"  ⚠ {voice_id} completed but no critique.md", flush=True)
        return {"voice": voice_id, "error": "no critique.md emitted", "elapsed_s": elapsed}
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"voice": voice_id, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


async def main():
    print(f"Output dir: {OUTDIR}")
    print(f"Voices ({len(VOICES)}): {[v[0] for v in VOICES]}")
    print(f"Variant: high")
    results = await asyncio.gather(*[dispatch_one(v) for v in VOICES], return_exceptions=False)
    print("\n=== Summary ===")
    for r in results:
        if "error" in r:
            print(f"  ✗ {r['voice']}: {r['error']}")
        else:
            print(f"  ✓ {r['voice']}: {r['critique']}")


if __name__ == "__main__":
    asyncio.run(main())
