#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27.0", "python-dotenv>=1.0.0"]
# ///
"""
Re-adversarial dispatch for verified-rcv intent.md (any version).

Channels:
  1. gateway: kimi-k2-6 / glm-4-7-flash / gpt-oss-120b
              (claude is run separately via Agent subagent)
  2. local:   mistral-small-4-119b-2603
  3. local:   qwen3.6-27b-mlx
  4. local:   google/gemma-4-26b-a4b
  5. local:   (goedel-prover-v2-32b excluded per v0.3 Ask C — theorem-prover
              specialists degenerate at the spec-adversarial layer.)

Each voice attacks the intent doc BLIND to the prior adversarial report.
The prior report is NOT inlined; comparison is a post-process synthesis step.

Run-tag selection (output dir):
  $COLOSSEUM_RUN_TAG (env)             — preferred, e.g. "intent-v0.3.0-2026-05-16T..."
  sys.argv[1] when not a model id      — back-compat
  default                              — UTC-now timestamp with intent-revised- prefix

Outputs land in:
  .colosseum/attacks/<run-tag>/{gateway-,local-}<model>.md

Env: COLOSSEUM_GATEWAY_BASE_URL + COLOSSEUM_GATEWAY_API_KEY loaded from
     verified-rcv/.env (gitignored).
"""
from __future__ import annotations

import asyncio
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

import httpx
from dotenv import load_dotenv

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
COLOSSEUM = Path("/Users/mvid/Development/reliq/colosseum")
ENV = REPO / ".env"
INTENT = REPO / ".colosseum" / "intent.md"
SPEC_ADV = COLOSSEUM / "agents" / "colosseum-spec-adversary.md"

# Output directory — run tag derived from env or defaulted to UTC-now.
_RUN_TAG_ENV = os.environ.get("COLOSSEUM_RUN_TAG")
if _RUN_TAG_ENV:
    RUN_TAG = _RUN_TAG_ENV
else:
    _ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    RUN_TAG = f"intent-revised-{_ts}"
OUTDIR = REPO / ".colosseum" / "attacks" / RUN_TAG
OUTDIR.mkdir(parents=True, exist_ok=True)

LOCAL_ENDPOINT = "http://localhost:1234/v1/chat/completions"
TIMEOUT = 1800.0  # 30min — local reasoning models on 27B+ params can take a long time

load_dotenv(ENV)
GATEWAY_URL = os.environ["COLOSSEUM_GATEWAY_BASE_URL"]
GATEWAY_KEY = os.environ["COLOSSEUM_GATEWAY_API_KEY"]


def strip_frontmatter(text: str) -> str:
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            return text[end + 5:]
    return text


def build_system_prompt() -> str:
    spec = strip_frontmatter(SPEC_ADV.read_text())
    return (
        "You are a hostile spec reviewer for the Colosseum methodology. "
        "Your job is to find ways the specification under review is wrong, weak, "
        "or misleading. Be paranoid; surface attacks you can ground in specific "
        "text. Do NOT soften findings to be polite. Do NOT invent attacks.\n\n"
        + spec
    )


ATTACK_CATEGORY_BRIEF = """
Attack-category emphasis (v0.3 — third adversarial pass; intent is at version v0.3.0):
- `temporal_state_mismatch`: an invariant tagged `state` that actually quantifies over history (or vice versa). Section 3.2 tagging discipline is the central test. Specifically: B6 was re-formulated to ∀-per-key in the v0.3.0 revision — did the new shape introduce a state/temporal mismatch?
- `impossibility-hypothesis-vacuity`: composition theorems whose hypotheses can never be satisfied by any well-typed witness. Examine B8 clause (c), the de-retracted Quartz `commitHashE` inheritance (Section 6.2 status block was UPDATED in v0.3.0), B9's `image_registration_honest` hypothesis (newly added in v0.3.0), and B10's `cross-layer` decomposition into `B10_lean ∧ image-identity-binding ∧ B8`.
- `disjunction-vs-decomposition`: a "decomposition" that is actually a disjunction (or fails to decompose at all). Examine the B10 cross-layer factorization and B9's negligibility decomposition.
- `composition_failure`: cross-project inheritance claims that don't actually compose at the layer claimed. Examine Section 6.2's Quartz inheritance table — particularly the de-retracted status (Quartz cycles 6.4-6.11 landed) and the `commitHashE` row's "NOT CONSUMED by verified-rcv" annotation (does this honestly explain why? what does verified-rcv consume?).
- All other categories from the agent system prompt above.

Other emphases specific to v0.3.0 revisions:
- `canonical_serialization` is now pinned to Borsh in Section 2.5 — does the doc explain why Borsh over alternatives? Are there ambiguity-free guarantees?
- `transaction trace model` is a new paragraph in Section 2.5 — does B6's ∀-per-key formulation correctly tie back to this trace model?
- `Tally_spec(...)` vs `IRV_spec(...)` split — is the boundary between the full-pipeline (decrypt+validate+tabulate) and the inner combinatorial core honest? Do the arities match Section 2.5?
- `S5` was restated as set-once write-discipline (no successor-state quantifier) — did the restatement preserve the original intent? Is "set-once" actually enforced or just asserted?
- `B10_lean` row added in Section 3.2 — does it correctly capture the Lean-internal image-IO obligation as distinct from the on-chain B10?
- `B9` conditioned on `image_registration_honest` — does this hypothesis correctly carve out the threat model, or does it accidentally trivialize B9?
- Section 6.2 `[2026-05-15 — de-retracted]` status block — is the framing honest given that the Quartz substrate is now production-quality but verified-rcv's spec layer hasn't been re-verified against the de-retracted substrate yet?
- Section 8.7's 6-numbered-step decomposition with `B10 ← B10_lean ∧ image-identity-binding ∧ B8` — is each step's lemma actually proven somewhere, or are some still informal?
- The SemVer header at the top of the doc — does the revision history accurately classify the v0.2.0 → v0.2.1 → v0.3.0 bumps under Ask N's MAJOR/MINOR/PATCH rules?
"""


def build_user_prompt(shortened: bool = False) -> str:
    intent = INTENT.read_text()
    if shortened:
        # Goedel has ~16K ctx — give it a slim prompt focused on the formal aspects.
        return f"""You are running a fresh adversarial attack on the revised intent document for
verified-rcv (a CosmWasm IRV smart contract with TDX-enclave tabulation,
attested via Quartz's dstack + zkdcap stack).

You are a theorem-prover specialist. Focus narrowly on the FORMAL aspects:
- B6 tagging (state vs temporal) in Section 3.2
- B8 composition theorem and the B9 4-summand negligibility decomposition
- B10 tally-correctness and Section 2.5's Tally_spec / IRV_spec split
- Section 8.7's off-chain witness decomposition
- Section 6.2 Quartz inheritance and the retracted-substrate status block

Output a list of attacks. For each: a short title, the affected spec
location (Section X.Y or invariant label), what's wrong, and one cite of
specific text from the intent doc.

Do NOT include the prior adversarial report; this is a fresh attack.

=== REVISED INTENT DOCUMENT (under review) ===
{intent}
=== END REVISED INTENT DOCUMENT ===

Begin your attack list now.
"""
    return f"""You are running a fresh adversarial attack on the revised intent document for
verified-rcv (a CosmWasm IRV smart contract with TDX-enclave tabulation,
attested via Quartz's dstack + zkdcap stack).

Your job is to surface specification-layer attacks that the methodology
should catch before downstream Quint / Lean / contract work. Be paranoid
but grounded; every attack must cite specific text in the intent doc.

{ATTACK_CATEGORY_BRIEF}

Report structure:
1. **Per-attack entries** — title, severity (critical / serious / cosmetic),
   affected location, what's wrong, cite of intent-doc text.
2. **Summary**: count by severity.
3. **VERDICT line at end**: SURVIVES (zero critical findings) /
   BREAKS-AGAIN (criticals exist) / INDETERMINATE.

Constraints:
- Do NOT read or assume access to the prior adversarial report. Attack fresh.
- Do NOT invent text; cite only what's actually in the doc.
- You may not read Quartz's ranked-choice tree; you may reason about
  Quartz's general specs from what's stated in the intent doc.

=== REVISED INTENT DOCUMENT (under review) ===
{intent}
=== END REVISED INTENT DOCUMENT ===

Output your adversarial report now. End with the VERDICT line.
"""


_NO_TEMPERATURE_MODELS = {
    # Newer Anthropic models on the gateway reject the `temperature` parameter.
    "claude-opus-4-7",
}


async def call_endpoint(
    client: httpx.AsyncClient,
    endpoint: str,
    auth_header: dict[str, str],
    model: str,
    system: str,
    user: str,
    max_tokens: int = 8192,
) -> dict:
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "max_tokens": max_tokens,
    }
    if model not in _NO_TEMPERATURE_MODELS:
        payload["temperature"] = 0.3
    t0 = datetime.now(timezone.utc)
    try:
        r = await client.post(endpoint, json=payload, headers=auth_header, timeout=TIMEOUT)
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        if r.status_code != 200:
            return {"model": model, "error": f"HTTP {r.status_code}: {r.text[:500]}", "elapsed_s": elapsed}
        data = r.json()
        choice = (data.get("choices") or [{}])[0]
        msg = choice.get("message", {})
        content = msg.get("content") or msg.get("reasoning_content") or ""
        return {
            "model": model,
            "content": content,
            "finish_reason": choice.get("finish_reason"),
            "usage": data.get("usage", {}),
            "elapsed_s": elapsed,
        }
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"model": model, "error": f"{type(e).__name__}: {e}", "elapsed_s": elapsed}


def write_result(channel: str, model: str, result: dict) -> Path:
    slug = model.replace("/", "-").replace(":", "-")
    outpath = OUTDIR / f"{channel}-{slug}.md"
    if "error" in result:
        outpath.write_text(
            f"# {model} ({channel}) — ERROR\n\n"
            f"- Elapsed: {result['elapsed_s']:.1f}s\n\n"
            f"```\n{result['error']}\n```\n"
        )
    else:
        header = (
            f"# {model} ({channel}) — verified-rcv revised intent adversarial pass\n\n"
            f"- **Elapsed**: {result['elapsed_s']:.1f}s\n"
            f"- **Finish reason**: {result['finish_reason']}\n"
            f"- **Usage**: {json.dumps(result['usage'])}\n\n"
            f"---\n\n"
        )
        outpath.write_text(header + result["content"])
    return outpath


def lms_load(model: str) -> bool:
    """Explicitly load a model via `lms load` before requesting it.

    LM Studio's JIT auto-evict can race with chat-completion requests on
    consumer hardware where only one model fits in memory at a time. A
    synchronous `lms load` ahead of the request prevents mid-request
    eviction.
    """
    print(f"  ... `lms load {model}` ...")
    r = subprocess.run(
        ["lms", "load", model, "--gpu", "max"],
        capture_output=True,
        text=True,
        timeout=300,
    )
    ok = r.returncode == 0 and "loaded successfully" in (r.stdout + r.stderr).lower()
    if not ok:
        print(f"  ... lms load FAILED ({r.returncode}): {r.stderr[:300] or r.stdout[:300]}")
    return ok


def extract_verdict(content: str) -> str:
    for line in reversed(content.splitlines()):
        line = line.strip()
        if "VERDICT:" in line.upper() or "VERDICT " in line.upper():
            return line[:200]
    return "<no verdict line found>"


async def main() -> None:
    system = build_system_prompt()
    user_full = build_user_prompt(shortened=False)

    print(f"System prompt: {len(system):,} chars")
    print(f"User prompt:   {len(user_full):,} chars (full)")
    print(f"Output dir:    {OUTDIR}")

    # Per-model max_tokens. Reasoning models (qwen3.6, goedel, kimi, gpt-oss, glm)
    # burn a large share on hidden reasoning_content before producing visible
    # content; give them substantially more budget. Non-reasoning models
    # (mistral, gemma) only need enough room for the report body.
    REASONING_MAX_TOKENS = 32768
    NON_REASONING_MAX_TOKENS = 16384

    LOCAL_VOICES = [
        # (model, prompt, max_tokens)
        # Note: gemma-4-26b-a4b has internal thinking despite the small-ish
        # activation; treat as reasoning. Mistral-119b is non-reasoning.
        # goedel-prover-v2-32b is excluded per v0.3 Ask C (theorem-prover
        # specialists degenerate into tautology loops at the spec-adversarial
        # layer; their place is at verify-pyramid via mcp__goedel__propose_lean_tactic).
        ("mistral-small-4-119b-2603", user_full, NON_REASONING_MAX_TOKENS),
        ("qwen3.6-27b-mlx", user_full, REASONING_MAX_TOKENS),
        ("google/gemma-4-26b-a4b", user_full, REASONING_MAX_TOKENS),
    ]
    # Gateway voices, per-route timeout calibrated against the 2026-05-14
    # discriminator probe + glm@24K retry. The 240s cap is GATEWAY-WIDE
    # (Cloudflare proxy timeout), NOT per-route — different models just have
    # different generation speeds and fit different max_tokens within 240s:
    #   - gpt-oss-120b: 47s @ 16K — could go much higher
    #   - glm-4-7-flash: 152s @ 16K, 238s @ 24K (timeout)
    #   - kimi-k2-6: 175s @ 8K, 238s @ 16K (timeout)
    # Anthropic gateway routes (claude-opus-4-7, claude-sonnet-4-6) hit a
    # SEPARATE Cloudflare 524 at ~127s (Bug 4) — Anthropic upstream has its
    # own tighter ceiling. Run the Claude voice via Agent subagent instead.
    GATEWAY_VOICES = [
        ("kimi-k2-6", user_full, 8192),
        ("glm-4-7-flash", user_full, 16384),
        ("gpt-oss-120b", user_full, 16384),  # discriminator: 47s @ 16K
    ]

    # Optional CLI arg: restrict to a single model id (for re-runs after a
    # truncation). Usage: uv run --script fan_out_dispatch.py qwen3.6-27b-mlx
    only_model = sys.argv[1] if len(sys.argv) > 1 else None

    def _ts() -> str:
        return datetime.now(timezone.utc).strftime("%H:%M:%SZ")

    def _say(msg: str) -> None:
        print(f"{_ts()} {msg}", flush=True)

    local_to_run = [v for v in LOCAL_VOICES if not only_model or v[0] == only_model]
    gateway_to_run = [v for v in GATEWAY_VOICES if not only_model or v[0] == only_model]
    total = len(local_to_run) + len(gateway_to_run)
    completed = 0
    _say(f"Dispatching {total} voice(s): {len(local_to_run)} local, {len(gateway_to_run)} gateway")

    async with httpx.AsyncClient() as client:
        # Local fan-out runs sequentially to avoid GPU contention.
        # Each local call: `lms load` the model first, then chat-complete; on
        # "Model unloaded" error (cross-session eviction), retry up to 2 times.
        for model, user, max_tok in local_to_run:
            completed += 1
            _say(f"→ [{completed}/{total}] local/{model} (max_tokens={max_tok}) dispatching...")
            r = None
            for attempt in range(3):
                lms_load(model)
                r = await call_endpoint(client, LOCAL_ENDPOINT, {}, model, system, user, max_tokens=max_tok)
                err_text = r.get("error", "")
                if "Model unloaded" not in err_text and "has not started loading" not in err_text:
                    break
                _say(f"  ↻ attempt {attempt+1} got unload error; retrying after reload")
            out = write_result("local", model, r)
            if "error" in r:
                _say(f"  ✗ ERR ({r['elapsed_s']:.1f}s): {r['error'][:200]}")
            else:
                _say(f"  ✓ OK ({r['elapsed_s']:.1f}s, {len(r['content']):,} chars, finish={r['finish_reason']}) → {out.name}")
                _say(f"    verdict: {extract_verdict(r['content'])}")

        for model, user, max_tok in gateway_to_run:
            completed += 1
            _say(f"→ [{completed}/{total}] gateway/{model} (max_tokens={max_tok}) dispatching...")
            r = await call_endpoint(
                client,
                f"{GATEWAY_URL}/chat/completions",
                {"Authorization": f"Bearer {GATEWAY_KEY}"},
                model,
                system,
                user,
                max_tokens=max_tok,
            )
            out = write_result("gateway", model, r)
            if "error" in r:
                _say(f"  ✗ ERR ({r['elapsed_s']:.1f}s): {r['error'][:200]}")
            else:
                _say(f"  ✓ OK ({r['elapsed_s']:.1f}s, {len(r['content']):,} chars, finish={r['finish_reason']}) → {out.name}")
                _say(f"    verdict: {extract_verdict(r['content'])}")


if __name__ == "__main__":
    asyncio.run(main())
