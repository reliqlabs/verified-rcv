#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# requires-python = ">=3.11"
# ///
"""
Re-cross-critique of the revised canonical specs/rcv.qnt.

The prior cross-critique (2026-05-19) caught two defects:
  - S8/S9 missing real predicates (canonical's TallyResult dropped
    per_round_counts + eliminated_by_round)
  - well_formed_tally missing dropped_voters.size() == ballots_dropped

The canonical was revised. This run verifies no new defects were
introduced and that the fix is structurally sound.

Same harness as before: kimi-k2-6 and gpt-5.5-native review the
canonical against their v11 specs as their own-spec context.
--variant high.
"""

from __future__ import annotations
import asyncio
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
OUTDIR = REPO / ".colosseum/specs/critique-revised-canonical-2026-05-19"
OUTDIR.mkdir(parents=True, exist_ok=True)

V11_DIR = REPO / ".colosseum/specs/v11-intent-tightened/per-voice"
CANONICAL_RCV = (REPO / "specs/rcv.qnt").read_text()
CANONICAL_MAIN = (REPO / "specs/main.qnt").read_text()
KIMI_V11_RCV = (V11_DIR / "kimi-k2-6/rcv.qnt").read_text()
KIMI_V11_MAIN = (V11_DIR / "kimi-k2-6/main.qnt").read_text()
GPT55_V11_RCV = (V11_DIR / "gpt-5-5-native/rcv.qnt").read_text()
GPT55_V11_MAIN = (V11_DIR / "gpt-5-5-native/main.qnt").read_text()

PER_CALL_TIMEOUT = 1800


def build_prompt(reviewer_id: str, reviewer_rcv: str, reviewer_main: str) -> tuple[str, str]:
    out_path = OUTDIR / f"{reviewer_id}-reviews-revised-canonical/critique.md"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    return f"""You are participating in a multi-voice Quint spec convergence experiment for verified-rcv. The canonical spec at /Users/mvid/Development/reliq/verified-rcv/specs/rcv.qnt was just revised in response to a prior cross-critique that identified two defects:

1. TallyResult was missing per_round_counts and eliminated_by_round; S8 and S9 were trivial bounds rather than real predicates. Now restored, with real predicates s8_round_counts_sum and s9_no_reappearance.
2. well_formed_tally was missing dropped_voters.size() == ballots_dropped. Now added.

Your task: review the REVISED canonical to verify the fix is structurally sound and to identify any new defects the revision may have introduced or any remaining defects the prior round missed.

Your voice id: {reviewer_id}

You produced your own Quint spec at v11 (shown below for context). The revised canonical is also shown. Both pass quint typecheck, quint run --invariant=all_invariants (no violation), and all three witness reachability invariants are violated.

Intent doc: /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md (v0.3.2).

Your spec for context:

===YOUR v11 rcv.qnt===
{reviewer_rcv}
===END===

===YOUR v11 main.qnt===
{reviewer_main}
===END===

Revised canonical:

===REVISED CANONICAL rcv.qnt===
{CANONICAL_RCV}
===END===

===REVISED CANONICAL main.qnt===
{CANONICAL_MAIN}
===END===

Read the revised canonical carefully. You may use your read / bash / grep / glob tools to consult the intent or run quint checks. The revision is documented in the canonical's header comments.

Write your critique to {out_path} with these sections:

# Re-critique of revised canonical: {reviewer_id}

## Q1. Is the S8/S9 fix structurally sound?

The prior canonical had S8/S9 as bounds on rounds_played. The revision encodes them as real predicates s8_round_counts_sum and s9_no_reappearance over per_round_counts and eliminated_by_round. Verify the encoding actually matches the intent's claim. Specifically:

- S8 should require: for each round i, sum of per_round_counts[i] values == ballots_tallied.
- S9 should require: any candidate appearing in eliminated_by_round[i] does not appear as a key in per_round_counts[j] for any j > i.

Is the revised encoding correct? If not, state the specific defect.

## Q2. Did the revision introduce any new defects?

Look for new defects added by the revision:
- Predicates that typecheck but are vacuously true on the specific tally shapes the step action generates
- New state-space gaps where the action set doesn't actually exercise the new fields
- Inconsistencies between EMPTY_TALLY's new field defaults and well_formed_tally's expectations
- The step action's nondet tally search picks between t_zero_round and t_one_round — does the search actually cover the elimination case enough to exercise S9? If the model checker only ever samples t_zero_round, S9 is still trivially true.

State at most one new defect. If none, say so.

## Q3. Remaining concerns

The prior cross-critique identified S8/S9 and dropped_voters.size() as defects. Both should now be addressed. Are there other axes where the revised canonical is still under-encoded relative to the intent? Examples: B1 monotone-once-set is encoded as action-guard disablement only (no state-side check); B5 publish-at-most-once is derived from B1; B7 terminal-state-immutability has no state predicate; B6 ballot-writer-equals-voter has no state predicate. Pick the single most material remaining concern.

## Optional notes

Anything else (under 200 words). Useful: methodology observations, places where the revision is better than your v11 spec on a different axis than what the prior critique covered, witness coverage gaps that should be added.

Stop with STATUS: ok or STATUS: error: <reason>.""", str(out_path)


async def dispatch_one(label: str, model: str, prompt: str) -> dict:
    cmd = [
        "opencode", "run",
        "--agent", "quint-spec-generator",
        "--model", model,
        "--variant", "high",
        "--format", "default",
        "--dangerously-skip-permissions",
        prompt,
    ]
    t0 = datetime.now(timezone.utc)
    print(f"[{t0.strftime('%H:%M:%S')}Z] {label} dispatching ({model})", flush=True)
    try:
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE, cwd=str(REPO)
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=PER_CALL_TIMEOUT)
        except asyncio.TimeoutError:
            proc.kill()
            elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
            print(f"  TIMEOUT {label} after {elapsed:.0f}s", flush=True)
            return {"label": label, "error": "timeout"}
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        print(f"  done {label} in {elapsed:.0f}s (exit={proc.returncode})", flush=True)
        return {"label": label, "elapsed_s": elapsed, "exit": proc.returncode}
    except Exception as e:
        return {"label": label, "error": f"{type(e).__name__}: {e}"}


async def main():
    print(f"Output dir: {OUTDIR}")
    print(f"Variant: high")
    kimi_prompt, _ = build_prompt("kimi-k2-6", KIMI_V11_RCV, KIMI_V11_MAIN)
    gpt_prompt, _ = build_prompt("gpt-5-5-native", GPT55_V11_RCV, GPT55_V11_MAIN)
    pairs = [
        ("kimi-reviews-revised-canonical", "burnt/kimi-k2-6", kimi_prompt),
        ("gpt55-reviews-revised-canonical", "openai/gpt-5.5", gpt_prompt),
    ]
    results = await asyncio.gather(*[dispatch_one(l, m, p) for (l, m, p) in pairs])
    print("\n=== Summary ===")
    for r in results:
        if "error" in r:
            print(f"  {r['label']}: {r['error']}")
        else:
            print(f"  {r['label']}: {r['elapsed_s']:.0f}s (exit={r['exit']})")


if __name__ == "__main__":
    asyncio.run(main())
