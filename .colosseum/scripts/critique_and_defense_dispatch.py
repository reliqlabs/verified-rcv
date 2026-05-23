#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# requires-python = ">=3.11"
# ///
"""
Two methodology experiments at --variant high, dispatched in parallel:

1. Critique of canonical: kimi and gpt-5.5 each review the handcrafted
   Claude spec (specs/rcv.qnt) which we adopted as canonical after the
   synthesis pass. Output: structured critique per voice.

2. Defense round: each voice is asked to defend its own choice on the
   axis the other voice surfaced as a defect in the earlier
   cross-critique. Three options per voice: defend, concede, or
   describe a third alternative.

Both experiments use --variant high reasoning effort.
"""

from __future__ import annotations
import asyncio
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
OUTDIR = REPO / ".colosseum/specs/critique-and-defense-2026-05-19"
OUTDIR.mkdir(parents=True, exist_ok=True)

V11_DIR = REPO / ".colosseum/specs/v11-intent-tightened/per-voice"
CANONICAL_RCV = (REPO / "specs/rcv.qnt").read_text()
CANONICAL_MAIN = (REPO / "specs/main.qnt").read_text()
KIMI_V11_RCV = (V11_DIR / "kimi-k2-6/rcv.qnt").read_text()
KIMI_V11_MAIN = (V11_DIR / "kimi-k2-6/main.qnt").read_text()
GPT55_V11_RCV = (V11_DIR / "gpt-5-5-native/rcv.qnt").read_text()
GPT55_V11_MAIN = (V11_DIR / "gpt-5-5-native/main.qnt").read_text()

PER_CALL_TIMEOUT = 1800


def critique_of_canonical_prompt(reviewer_id: str, reviewer_rcv: str, reviewer_main: str) -> tuple[str, str]:
    out_path = OUTDIR / f"{reviewer_id}-reviews-canonical/critique.md"
    out_dir = out_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    prompt = f"""You are participating in a multi-voice Quint spec convergence experiment for verified-rcv. You produced a Quint spec from intent v0.3.2 (your spec is shown below). A separate spec was authored by hand at /Users/mvid/Development/reliq/verified-rcv/specs/rcv.qnt and adopted as the canonical artifact after a synthesis pass. Both specs pass `quint typecheck`, `quint run --invariant=all_invariants` (no violation), and all three witness reachability invariants are violated.

Your role: REVIEW the canonical spec as a peer reviewer. You are NOT generating a new spec. You are reading two specs (yours and the canonical) and producing a structured critique.

Your voice id: {reviewer_id}
Target voice id: canonical (handcrafted Claude with S7 synthesis fix)

Intent doc: /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md (v0.3.2 with A2 enclave + A3 B2 encoding-discipline tightenings).

Your spec (context, no need to defend line-by-line):

===YOUR rcv.qnt===
{reviewer_rcv}
===END===

===YOUR main.qnt===
{reviewer_main}
===END===

Canonical spec to review:

===CANONICAL rcv.qnt===
{CANONICAL_RCV}
===END===

===CANONICAL main.qnt===
{CANONICAL_MAIN}
===END===

Read the canonical carefully. You may use your read / bash / grep / glob tools to consult the intent document or to run `quint typecheck` and `quint run` against the canonical. You do not need to use the tools if your analysis stands on its own.

Then write a structured critique to {out_path} with these sections in this exact order:

# Cross-critique: {reviewer_id} reviews canonical

## Q1. Most material structural divergence

State the single structural divergence you consider most material between your spec and the canonical. Be specific about which state variable, action, or invariant differs, and why the difference matters.

## Q2. Apparent defect in canonical

The canonical typechecks and passes validation. Identify one defect, if any: a place where the canonical is technically valid but trivially encoded, semantically wrong, or violates an intent obligation in a way the typechecker cannot catch. Examples: a behavioral invariant declared as a tautological `true` shadow that should be a real predicate; a witness whose negation does not express reachability of the named state; an action guard that admits behaviors the intent forbids; a state variable whose value is never constrained.

If you find no defect after careful review, say so explicitly with one sentence of reasoning.

## Q3. Change to your own spec after reading canonical

Identify one change you would make to YOUR spec after seeing the canonical. State the change concretely. If no change is warranted, state that with one sentence.

## Optional notes

Anything else worth recording (under 200 words). Useful: methodology observations, places where the intent is unclear, places where the canonical's choice is better than yours but the reverse holds for a different consideration, etc.

Stop after writing the critique. Emit STATUS: ok or STATUS: error: <reason> as the last line of output."""
    return prompt, str(out_path)


def defense_prompt(defender_id: str, defender_rcv: str, defender_main: str,
                   defect_quote: str, other_voice: str, target_axis: str) -> tuple[str, str]:
    out_path = OUTDIR / f"{defender_id}-defends-{target_axis}/defense.md"
    out_dir = out_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)
    prompt = f"""You are participating in a multi-voice Quint spec methodology experiment for verified-rcv. You previously generated a Quint spec from intent v0.3.2 (your spec below). Another voice ({other_voice}) reviewed your spec and identified what they called a defect on the axis "{target_axis}". You are now asked to defend your choice, concede it, or propose a third alternative.

Your voice id: {defender_id}
Reviewing voice: {other_voice}
Axis under defense: {target_axis}

Your spec:

===YOUR rcv.qnt===
{defender_rcv}
===END===

===YOUR main.qnt===
{defender_main}
===END===

The critique made against you:

===CRITIQUE BY {other_voice}===
{defect_quote}
===END===

Your task: respond to the critique. Be honest. Three options are valid:

A. Defend the choice. State concrete protocol-layer reasoning for why your encoding is correct. Cite the intent's text if it supports you (intent path: /Users/mvid/Development/reliq/verified-rcv/.colosseum/intent.md, v0.3.2). Acknowledge any limitations of the encoding the critique raises while explaining why those limitations are acceptable.

B. Concede. State that the critique is correct and that you would change your spec. Specify the concrete change you would make.

C. Propose a third option. Neither defend nor concede the original choice — propose a different encoding that addresses the critique's concern but differs from the alternative the critique implicitly proposed. Justify why the third option is better than both.

You may consult the intent doc and your spec by reading the files. You may run `quint typecheck` to verify any proposed change still validates.

Write your response to {out_path} with these sections:

# Defense: {defender_id} on {target_axis}

## Verdict

State A (defend), B (concede), or C (third option) as the first content of this section. Then in 2-3 sentences explain the verdict.

## Reasoning

Detailed reasoning behind the verdict. Cite the intent where relevant. If defending: explain why the critique's premise is wrong, OR why the limitation it raises is acceptable. If conceding: state what the critique got right that you missed. If third option: describe the new encoding and why it is preferable.

## Methodology observation

One short paragraph: was this defense exercise informative? Did stating the defense formally produce reasoning you would not have surfaced without the prompt? Under 100 words.

Stop after writing the response. Emit STATUS: ok as the last line."""
    return prompt, str(out_path)


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

    # Defect quotes from the prior cross-critique
    b8_quote = """gpt-5-5-native reviewed kimi's spec and identified this defect:

"Q2 defect: `b8_attestation_shadow = true` is a tautological behavioral invariant. The design notes say B8 is represented as the shadow that any chain tally came from an enclave-computed tally, but the code does not encode that predicate. A checkable form would be at least `if (tally_result.present) enclave_computed_tally.present else true`, preferably also tying the published value to `enclave_computed_tally.value`. Typechecking and sampled invariant runs cannot detect this because `true` is invariant under every transition."

In the kimi v11 spec, the line is literally `val b8_attestation_shadow = true` and the design notes claim B8 is encoded as a classical shadow. The gpt-5.5 spec encodes B8 as `if (tally_result.present) enclave_computed_tally.present else true` and includes it in `all_invariants`.
"""

    s9_quote = """kimi-k2-6 reviewed gpt-5.5's spec and identified this defect:

"Q2 defect: S9 (`elimination_monotonicity`) is trivially encoded as a tautology. The target defines:
```
val s9_elimination_monotonicity = if (tally_result.present) tally_result.value.eliminated_by_round.length() == 0 else true
```
Since `tallySpec` always returns `eliminated_by_round: List()`, this predicate is always `true` and never exercises the structural constraint the intent actually requires: 'if a candidate appears in `eliminated_by_round[i]`, they appear in no `per_round_counts[j]` for `j > i`.' The typechecker cannot catch this because the expression is syntactically valid and well-typed; it is semantically vacuous."

In the kimi spec, S9 is encoded as a real indexed predicate over per_round_counts and eliminated_by_round."""

    pairs = []

    # Experiment 1: cross-critique of canonical
    kimi_critique_prompt, _ = critique_of_canonical_prompt("kimi-k2-6", KIMI_V11_RCV, KIMI_V11_MAIN)
    pairs.append(("kimi-reviews-canonical", "burnt/kimi-k2-6", kimi_critique_prompt))
    gpt_critique_prompt, _ = critique_of_canonical_prompt("gpt-5-5-native", GPT55_V11_RCV, GPT55_V11_MAIN)
    pairs.append(("gpt55-reviews-canonical", "openai/gpt-5.5", gpt_critique_prompt))

    # Experiment 2: defense round
    kimi_defense_prompt, _ = defense_prompt("kimi-k2-6", KIMI_V11_RCV, KIMI_V11_MAIN,
                                             b8_quote, "gpt-5-5-native", "b8_attestation_shadow")
    pairs.append(("kimi-defends-b8", "burnt/kimi-k2-6", kimi_defense_prompt))
    gpt_defense_prompt, _ = defense_prompt("gpt-5-5-native", GPT55_V11_RCV, GPT55_V11_MAIN,
                                            s9_quote, "kimi-k2-6", "s9_elimination_monotonicity")
    pairs.append(("gpt55-defends-s9", "openai/gpt-5.5", gpt_defense_prompt))

    results = await asyncio.gather(*[dispatch_one(label, model, p) for (label, model, p) in pairs])
    print("\n=== Summary ===")
    for r in results:
        if "error" in r:
            print(f"  {r['label']}: {r['error']}")
        else:
            print(f"  {r['label']}: {r['elapsed_s']:.0f}s (exit={r['exit']})")


if __name__ == "__main__":
    asyncio.run(main())
