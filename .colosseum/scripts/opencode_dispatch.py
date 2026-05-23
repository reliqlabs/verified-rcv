#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
OpenCode per-section adversarial dispatch — subagent-dispatch prototype.

Compares against the inlined dispatch at fan_out_dispatch.py: same intent,
same voices (subset), different shape. Each (voice × slice) is one
`opencode run --agent spec-adversary --model <voice> "<message>"` call.
The agent reads the slice from intent.md directly (file access via
permission.read) rather than receiving the slice inlined.

USAGE
    uv run --script opencode_dispatch.py [--voices=A,B,C] [--slices=X,Y]

    Defaults run the full 6-voice × 9-slice matrix (54 calls).
    --voices and --slices accept comma-separated subsets for retry / debug.

OUTPUT
    .colosseum/attacks/<run-tag>/
    ├── per-section/
    │   └── <voice-id-slug>/
    │       └── <slice-name>.md      # one per (voice, slice)
    ├── opencode-<voice-id-slug>.md  # aggregated per-voice
    ├── dispatch.log
    └── run.json                     # initialized by colosseum_run.py init
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
COLOSSEUM = Path("/Users/mvid/Development/reliq/colosseum")
INTENT = REPO / ".colosseum" / "intent.md"

RUN_TAG_ENV = os.environ.get("COLOSSEUM_RUN_TAG")
if RUN_TAG_ENV:
    RUN_TAG = RUN_TAG_ENV
else:
    _ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    RUN_TAG = f"pattern-b-{_ts}"
OUTDIR = REPO / ".colosseum" / "attacks" / RUN_TAG
OUTDIR.mkdir(parents=True, exist_ok=True)
(OUTDIR / "per-section").mkdir(exist_ok=True)

PER_CALL_TIMEOUT = 1800  # 30 min per (voice, slice) call. Local reasoning models (gemma, qwen3.6)
                          # making multi-turn ReAct loops over LM Studio can take 10-25 min per slice
                          # because each tool-use turn burns the model's reasoning budget separately.
                          # Gateway voices (kimi, glm, gpt-oss) typically need 200-400s per slice and
                          # are well-served by the same cap.
MAX_RETRIES = 2

# ─────────────────────────────────────────────────────────────────────────
# Slice plan
# ─────────────────────────────────────────────────────────────────────────
# Each slice names: a human label, the markdown header pattern that starts
# it, the next header pattern that ends it (exclusive). The agent reads
# from intent.md directly using these patterns.

SLICES = [
    {
        "name": "scope",
        "label": "System scope, identity, and external contracts",
        "headers": [
            "## 1. System Identity   → through end of ## 1.",
            "## 5. Non-Goals         → through end of ## 5.",
            "### 6.4 Caller contract → through end of ### 6.6 Output contract",
        ],
        "attack_emphasis": (
            "Identity overreach (is the system claim narrower than the doc admits?); "
            "non-goals that should be in scope; "
            "caller/voter/output-contract clauses that admit malicious clients the doc "
            "intends to exclude."
        ),
    },
    {
        "name": "behaviors-core",
        "label": "Worked examples — happy path, boundary cases, edge cases, explicit failures",
        "headers": [
            "### 2.1 Happy path",
            "### 2.2 Boundary cases",
            "### 2.3 Edge cases",
            "### 2.4 Explicit failures",
            "→ end at: ### 2.5 Structured behavior blocks",
        ],
        "attack_emphasis": (
            "Worked-example arithmetic correctness; "
            "edge-case completeness (empty / max / boundary); "
            "explicit-failure listing completeness; "
            "happy-path implicit assumptions that aren't load-bearing elsewhere."
        ),
    },
    {
        "name": "behaviors-types",
        "label": "Structured behavior blocks — type signatures, canonical_serialization, Tally_spec/IRV_spec, transaction trace model",
        "headers": [
            "### 2.5 Structured behavior blocks",
            "→ end at: ## 3. Invariants",
        ],
        "attack_emphasis": (
            "Type-signature ambiguity (Tally_spec arity / determinism / iteration order over Map<>); "
            "Tally_spec vs IRV_spec split — honest at semantics, not just arity; "
            "canonical_serialization Borsh pin — does it nail down Vec/Map ordering rigorously? "
            "Transaction trace model — does fires_at_transition properly support B6's ∀-per-key form? "
            "EnclaveImage symbol — typed properly or free variable?"
        ),
    },
    {
        "name": "state-invariants",
        "label": "Structural (pointwise) invariants S1..S10",
        "headers": [
            "### 3.1 Structural invariants",
            "→ end at: ### 3.2 Behavioral invariants",
        ],
        "attack_emphasis": (
            "S5 set-once write-discipline restatement — does it correctly stay state-shape (no successor quantification)? "
            "Pointwise evaluability of every claimed-pointwise invariant; "
            "Derived flags (S5, S10) — are derivations sound? "
            "Coverage gaps: any state-shape invariant the doc names elsewhere but doesn't list here?"
        ),
    },
    {
        "name": "temporal-invariants",
        "label": "Behavioral (temporal) invariants B1..B10 + B10_lean + cross-project dependency note",
        "headers": [
            "### 3.2 Behavioral invariants",
            "→ end at: ## 4. Failure Modes",
        ],
        "attack_emphasis": (
            "B6 ∀-per-key formulation correctness vs the trace model in §2.5; "
            "B8 clauses (c) and (d) — composition theorem soundness; "
            "B9 4-summand decomposition + image_registration_honest precondition — does it trivialize? Is the precondition formally defined? "
            "B10 cross-layer factorization (B10 ← B10_lean ∧ image-identity-binding ∧ B8) — does the conjunction logic hold? "
            "B10_lean's EnclaveImage symbol — is it typed properly or free? "
            "tag correctness for each invariant (state vs temporal vs cross-layer)."
        ),
    },
    {
        "name": "failure-modes",
        "label": "Failure modes §4.1..§4.9",
        "headers": [
            "## 4. Failure Modes",
            "→ end at: ## 5. Non-Goals",
        ],
        "attack_emphasis": (
            "In-scope vs out-of-scope classification (does the boundary make sense?); "
            "severity tagging (confidentiality / integrity / liveness — well-placed?); "
            "missing failure modes the methodology should require; "
            "§4.9 deterministic-malformed-tally deadlock — soundness."
        ),
    },
    {
        "name": "trust-quartz",
        "label": "Trust boundaries §6.1..§6.3 (Chain trust, Quartz inheritance, dstack/TDX)",
        "headers": [
            "### 6.1 Chain trust (Xion / CosmWasm)",
            "### 6.2 Quartz inheritance (the cross-project dependency)",
            "### 6.3 dstack / TDX hardware trust",
            "→ end at: ### 6.4 Caller contract (instantiator)",
        ],
        "attack_emphasis": (
            "Quartz inheritance table — composition honest? "
            "[2026-05-15 — de-retracted] status block — framing honest given verified-rcv hasn't re-verified against the de-retracted substrate? "
            "commitHashE row with 'NOT CONSUMED' annotation — does verified-rcv consume something that SHOULD be the commit hash but isn't? "
            "image_registration_honest definition + temporal scope (referenced from §3.2 B9). "
            "Chain trust assumptions — what does verified-rcv assume about Xion?"
        ),
    },
    {
        "name": "scenarios",
        "label": "Concrete scenarios §8.1..§8.6 (witnesses for B1..B6, B10)",
        "headers": [
            "### 8.1 Clean election with clear winner",
            "→ through ### 8.6 Impersonation attempt rejected (witness for B6)",
        ],
        "attack_emphasis": (
            "Witness-correctness per scenario — does the scenario actually witness the named invariant? "
            "Trace coverage — is every B-series invariant given at least one witnessing scenario? "
            "Edge-case witnesses missing — abstention-heavy, premature voting, double-publish."
        ),
    },
    {
        "name": "off-chain-witness",
        "label": "§8.7 — B10 cross-layer discharge (Lean + image-identity binding + B8) — restructured 6-step decomposition",
        "headers": [
            "### 8.7 B10 witness — cross-layer discharge (Lean + image-identity binding + B8)",
            "→ end at: ## Open Questions",
        ],
        "attack_emphasis": (
            "Each step's lemma — actually proven somewhere or hand-waved? "
            "B10 ← B10_lean ∧ image-identity-binding ∧ B8 — does step 6 actually conclude B10? "
            "Step 6 absorbing a fourth trust link (dstack-KMS) — does the decomposition admit it honestly? "
            "EnclaveImage symbol typing — does step 1 or 2 actually pin it down?"
        ),
    },
]

CONTEXT_APPENDIX = """
=== CONTEXT APPENDIX (read-only — DO NOT ATTACK; these sections are attacked in their own invocations) ===

System under review: verified-rcv — IRV CosmWasm smart contract with TDX-enclave tabulation via Quartz/dstack/zkdcap.
Intent doc: version 0.3.0 (SemVer per Colosseum methodology v0.4 candidate Ask N).

State invariants (full bodies in §3.1):
  S1 non-empty candidates,  S2 distinct candidates,  S3 well-ordered voting window,
  S4 ballot-keys-are-candidates,  S5 (derived) set-once write-discipline,
  S6 winner well-formedness,  S7 tally count conservation,  S8 per-round count consistency,
  S9 elimination monotonicity,  S10 (derived) resolution implies past end_at.

Temporal invariants (full bodies in §3.2):
  B1 tally_result monotone-once-set,  B2 no late ballots,  B3 no premature tally (causal),
  B4 no premature voting,  B5 (derived) publish_result fires at most once,
  B6 ballot writer is the ballot voter (∀-per-key),
  B7 terminal-state immutability,  B8 attestation-binds-tally,
  B9 B8 negligibility-budget decomposition (4-summand, conditioned on image_registration_honest),
  B10 tally-correctness (cross-layer; B10 = B10_lean ∧ image-identity-binding ∧ B8),
  B10_lean (off-chain) Lean-internal image-IO obligation.

Type signatures (full def in §2.5):
  Tally_spec : EncryptedBallots × Schedule × ResolverPolicy → TallyResult  (full pipeline)
  IRV_spec   : RankedBallots × CandidateSet → IRVResult                      (inner combinatorial core)
  EnclaveImage : Bytes → Bytes                                                (image_extract)
  canonical_serialization = Borsh
  fires_at_transition(σ → σ′) — multiset of txs causing the state transition

=== END CONTEXT APPENDIX ===
""".strip()

# ─────────────────────────────────────────────────────────────────────────
# Voice roster
# ─────────────────────────────────────────────────────────────────────────

VOICES = [
    # (id_slug, opencode_model_id, note)
    # Excluded:
    #   glm-4-7-flash (Zhipu 30B "flash" tier) — exhibited degenerate-loop behavior in BOTH
    #     inline dispatch (paragraph repetition) and subagent-dispatch parallel runs
    #     (enumerated fake attacks #4-75+ on behaviors-core). The 30B "flash" class is
    #     adversarial-review-unreliable at the gateway scale; if a Zhipu voice is wanted,
    #     run a larger glm tier (full glm-4-7) or pull a comparable model into LM Studio.
    #     Decision 2026-05-16 after Round 3a subagent-dispatch calibration.
    ("claude-opus-4-7", "burnt/claude-opus-4-7", "Anthropic via /openai/v1 — prefer Agent subagent for the Claude voice; gateway path only if cross-harness manifest is in use"),
    ("kimi-k2-6", "burnt/kimi-k2-6", "Moonshot — frontier tier; was 8K truncated under inlined, sequential ReAct produces clean 6.8-15K-char per-slice reports"),
    ("gpt-oss-120b", "burnt/gpt-oss-120b", "OpenAI-OSS — 120B; clean sequential ReAct"),
    ("nemotron-3-120b-a12b", "burnt/cloudflare-100-cf-nvidia-nemotron-3-120b-a12b", "NVIDIA — gateway 120B-A12B MoE, reasoning-on; added 2026-05-16"),
    ("gemma-4-26b-a4b", "lmstudio/google/gemma-4-26b-a4b", "Google — local 26B; needs ≥30min per-call timeout for ReAct loops"),
    ("qwen3.6-27b", "lmstudio/qwen/qwen3.6-27b", "Alibaba — local 27B; may resolve to qwen3.6-27b-mlx via LM Studio"),
    ("mistral-small-4-119b-2603", "lmstudio/mistral-small-4-119b-2603", "Mistral — local 119B non-reasoning; family-diversity gap-filler; performed well in Round 3a inlined dispatch"),
]


def build_message(voice_id: str, slice_spec: dict) -> str:
    headers_block = "\n".join(f"  - {h}" for h in slice_spec["headers"])
    return f"""VOICE_ID: {voice_id}  (use this string as <voice-id> in your output header)

TARGET_SPEC: {INTENT}

TARGET_SLICE: {slice_spec['name']} — {slice_spec['label']}

Read these headers from TARGET_SPEC (use the Read tool):
{headers_block}

Attack only what is INSIDE these header ranges. Do not attack content
outside them.

Attack-category emphasis for this slice:
{slice_spec['attack_emphasis']}

{CONTEXT_APPENDIX}

Begin by reading TARGET_SPEC at the named header ranges, then produce
your attack report per the Output structure in your system prompt."""


_REPORT_MARKERS = ("## Attacks", "## Slice-local summary", "VERDICT", "### 1.")
_MIN_REPORT_CHARS = 500


def _is_truncated_stub(content: str) -> bool:
    """Detect mid-ReAct truncation: model announced intent but didn't produce structured report.

    OpenCode exits cleanly (returncode 0, non-empty stdout) when a model emits its
    "I'll read the file..." preamble and then doesn't continue past the first tool_use.
    The orchestrator can't tell the difference from stdout alone unless we look at
    the content shape. A real adversarial report contains at least one of the
    agent's prescribed structural markers and is large enough to plausibly hold
    one. A stub is short and structureless.

    Threshold rationale: a SURVIVES-SLICE report with brief justification is
    typically 1.5-3K chars; the smallest legitimate output we've seen on this
    methodology is ~700 chars. 500 chars is a safe floor — anything below is
    almost certainly truncation. Markers catch borderline cases where a stub
    happens to be longer.
    """
    if len(content) < _MIN_REPORT_CHARS:
        return True
    return not any(m in content for m in _REPORT_MARKERS)


async def dispatch_one(voice_id: str, model_id: str, slice_spec: dict, attempt: int = 0) -> dict:
    out_dir = OUTDIR / "per-section" / voice_id
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{slice_spec['name']}.md"
    log_prefix = f"[{voice_id}/{slice_spec['name']}]"

    message = build_message(voice_id, slice_spec)
    cmd = [
        "opencode", "run",
        "--agent", "spec-adversary",
        "--model", model_id,
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
            return {"voice": voice_id, "slice": slice_spec["name"], "error": f"timeout after {PER_CALL_TIMEOUT}s", "elapsed_s": elapsed}
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        content = stdout.decode("utf-8", errors="replace")
        # strip ANSI escapes from OpenCode's TTY output
        import re
        content = re.sub(r"\x1b\[[0-9;]*[a-zA-Z]", "", content)
        # strip OpenCode banner ("> agent · model") and trailing whitespace
        lines = content.splitlines()
        if lines and lines[0].startswith("> "):
            # drop banner + blank line
            lines = lines[1:]
            while lines and not lines[0].strip():
                lines = lines[1:]
        content = "\n".join(lines).strip()

        if proc.returncode != 0 or not content:
            err = stderr.decode("utf-8", errors="replace")[:1000]
            return {"voice": voice_id, "slice": slice_spec["name"], "error": f"exit={proc.returncode}: {err}", "elapsed_s": elapsed}

        # Content-quality check: detect mid-ReAct truncation stubs (the
        # "I'll read the file..." announce-intent pattern where the model
        # never produced the structured report). OpenCode exits cleanly
        # with such stubs, but they have none of the agent's prescribed
        # output structure. Treat as error so the retry loop fires.
        if _is_truncated_stub(content):
            return {
                "voice": voice_id,
                "slice": slice_spec["name"],
                "error": f"truncated stub (exit=0, {len(content)} chars, no structural markers — model announced intent but did not produce structured report)",
                "elapsed_s": elapsed,
                "stub_content_preview": content[:200],
            }

        out_path.write_text(content)
        return {
            "voice": voice_id,
            "slice": slice_spec["name"],
            "out_path": str(out_path),
            "elapsed_s": elapsed,
            "chars": len(content),
        }
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"voice": voice_id, "slice": slice_spec["name"], "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


def _ts() -> str:
    """HH:MM:SS UTC timestamp for log lines."""
    return datetime.now(timezone.utc).strftime("%H:%M:%SZ")


def _log_both(msg: str, logf) -> None:
    """Write a timestamped line to both stdout (for overseer visibility) and the log file (for audit)."""
    line = f"{_ts()} {msg}"
    print(line, flush=True)
    logf.write(line + "\n")
    logf.flush()


async def dispatch_voice(voice_id: str, model_id: str, slices: list[dict], log_path: Path) -> list[dict]:
    """Sequential slice dispatch for one voice."""
    results = []
    log_path.parent.mkdir(parents=True, exist_ok=True)
    total = len(slices)
    voice_t0 = datetime.now(timezone.utc)
    with log_path.open("a") as logf:
        _log_both(f"→ voice {voice_id}: starting {total}-slice dispatch", logf)
        for idx, slice_spec in enumerate(slices, start=1):
            slice_name = slice_spec["name"]
            _log_both(f"  ⇢ [{idx}/{total}] {voice_id}/{slice_name} dispatching...", logf)
            for attempt in range(MAX_RETRIES + 1):
                r = await dispatch_one(voice_id, model_id, slice_spec, attempt)
                if "error" not in r:
                    if attempt > 0:
                        _log_both(f"    ↻ {voice_id}/{slice_name} succeeded on attempt {attempt + 1}", logf)
                    break
                _log_both(f"    ⚠ {voice_id}/{slice_name} attempt {attempt + 1} failed ({r['elapsed_s']:.0f}s): {r['error'][:120]}", logf)
            results.append(r)
            if "error" in r:
                _log_both(f"  ✗ [{idx}/{total}] {voice_id}/{slice_name} ABANDONED after {MAX_RETRIES + 1} attempts (cum {r['elapsed_s']:.0f}s)", logf)
            else:
                _log_both(f"  ✓ [{idx}/{total}] {voice_id}/{slice_name} OK ({r['elapsed_s']:.0f}s, {r['chars']:,} chars)", logf)
        voice_elapsed = (datetime.now(timezone.utc) - voice_t0).total_seconds()
        n_ok = sum(1 for r in results if "error" not in r)
        _log_both(f"─ voice {voice_id} done: {n_ok}/{total} slices OK in {voice_elapsed:.0f}s", logf)
    return results


def aggregate_voice(voice_id: str, slices: list[dict], results: list[dict]) -> Path:
    """Concatenate per-slice files into one per-voice file."""
    agg_path = OUTDIR / f"opencode-{voice_id}.md"
    header = f"# {voice_id} — verified-rcv intent v0.3.0 adversarial pass (subagent dispatch, per-section)\n\n"
    header += f"- **Pass**: subagent-dispatch calibration (4th attack against v0.3.0 — same intent, OpenCode + file-access agent loop)\n"
    header += f"- **Slices dispatched**: {len(slices)}\n\n"
    parts = [header]
    for slice_spec, r in zip(slices, results):
        parts.append(f"---\n\n## Slice: {slice_spec['name']}\n\n")
        if "error" in r:
            parts.append(f"**ERROR** ({r['elapsed_s']:.1f}s): {r['error'][:500]}\n")
        else:
            parts.append(f"*Elapsed: {r['elapsed_s']:.1f}s, {r['chars']:,} chars.*\n\n")
            parts.append(Path(r["out_path"]).read_text())
            parts.append("\n")
    agg_path.write_text("".join(parts))
    return agg_path


async def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--voices", default=None, help="comma-separated voice id slugs to dispatch (default: all)")
    ap.add_argument("--slices", default=None, help="comma-separated slice names to dispatch (default: all)")
    ap.add_argument("--sequential", action="store_true", help="run voices sequentially (default: parallel)")
    args = ap.parse_args()

    selected_voices = VOICES
    if args.voices:
        wanted = set(args.voices.split(","))
        selected_voices = [v for v in VOICES if v[0] in wanted]

    selected_slices = SLICES
    if args.slices:
        wanted = set(args.slices.split(","))
        selected_slices = [s for s in SLICES if s["name"] in wanted]

    log_path = OUTDIR / "dispatch.log"
    with log_path.open("a") as logf:
        logf.write(f"\n=== Subagent dispatch starting {datetime.now(timezone.utc).isoformat()} ===\n")
        logf.write(f"Voices: {[v[0] for v in selected_voices]}\n")
        logf.write(f"Slices: {[s['name'] for s in selected_slices]}\n")
        logf.write(f"Output: {OUTDIR}\n\n")

    print(f"Output dir: {OUTDIR}")
    print(f"Voices ({len(selected_voices)}): {[v[0] for v in selected_voices]}")
    print(f"Slices ({len(selected_slices)}): {[s['name'] for s in selected_slices]}")
    print(f"Total calls: {len(selected_voices) * len(selected_slices)}")

    if args.sequential:
        all_results = {}
        for voice_id, model_id, _note in selected_voices:
            print(f"\n→ Dispatching voice {voice_id} sequentially...")
            results = await dispatch_voice(voice_id, model_id, selected_slices, log_path)
            all_results[voice_id] = results
    else:
        print("\nDispatching all voices in parallel...")
        tasks = [
            dispatch_voice(voice_id, model_id, selected_slices, log_path)
            for voice_id, model_id, _note in selected_voices
        ]
        results_list = await asyncio.gather(*tasks, return_exceptions=False)
        all_results = {v[0]: r for v, r in zip(selected_voices, results_list)}

    print("\n=== Aggregating per-voice files ===")
    summary = []
    for voice_id, model_id, _note in selected_voices:
        results = all_results[voice_id]
        agg_path = aggregate_voice(voice_id, selected_slices, results)
        n_ok = sum(1 for r in results if "error" not in r)
        total_elapsed = sum(r["elapsed_s"] for r in results)
        total_chars = sum(r.get("chars", 0) for r in results if "error" not in r)
        summary.append({
            "voice": voice_id,
            "slices_ok": n_ok,
            "slices_total": len(results),
            "total_elapsed_s": total_elapsed,
            "total_chars": total_chars,
        })
        print(f"  {voice_id}: {n_ok}/{len(results)} slices OK, {total_elapsed:.0f}s total, {total_chars:,} chars → {agg_path.name}")

    (OUTDIR / "summary.json").write_text(json.dumps(summary, indent=2))
    print(f"\nSummary written to {OUTDIR}/summary.json")


if __name__ == "__main__":
    asyncio.run(main())
