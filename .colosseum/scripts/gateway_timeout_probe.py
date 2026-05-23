#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.27.0", "python-dotenv>=1.0.0"]
# ///
"""
Discriminate gateway timeout hypotheses by pinging 3 gateway models with the
same 22K-token prompt + max_tokens=16384, all concurrently.

Hypotheses:
  (a) gateway hard cap     → all 3 timeout
  (b) kimi-specific        → only kimi timeouts
  (c) reasoning-model cap  → reasoning models (glm, gpt-oss) timeout; claude-sonnet (non-reasoning) doesn't

Pings:
  - glm-4-7-flash       (Zhipu, reasoning, light)
  - gpt-oss-120b        (OSS, reasoning, heavy)
  - claude-sonnet-4-6   (non-reasoning, fast)

(kimi-k2-6 already confirmed timeout — no need to retest)
"""
from __future__ import annotations

import asyncio
import json
import os
from datetime import datetime, timezone
from pathlib import Path

import httpx
from dotenv import load_dotenv

REPO = Path("/Users/mvid/Development/reliq/verified-rcv")
COLOSSEUM = Path("/Users/mvid/Development/reliq/colosseum")
ENV = REPO / ".env"
INTENT = REPO / ".colosseum" / "intent.md"
SPEC_ADV = COLOSSEUM / "agents" / "colosseum-spec-adversary.md"

load_dotenv(ENV)
URL = os.environ["COLOSSEUM_GATEWAY_BASE_URL"]
KEY = os.environ["COLOSSEUM_GATEWAY_API_KEY"]

MODELS = ["glm-4-7-flash", "gpt-oss-120b", "claude-sonnet-4-6"]
MAX_TOKENS = 16384
TIMEOUT = 600.0


def strip_frontmatter(text: str) -> str:
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            return text[end + 5:]
    return text


def build_prompts() -> tuple[str, str]:
    intent = INTENT.read_text()
    spec = strip_frontmatter(SPEC_ADV.read_text())
    system = (
        "You are a hostile spec reviewer for the Colosseum methodology. "
        "Find spec attacks grounded in cited text.\n\n" + spec
    )
    user = (
        "Adversarial review of the verified-rcv revised intent doc. "
        "Output a structured attack list with critical / serious / cosmetic "
        "tiers and a final VERDICT line.\n\n"
        f"=== INTENT (under review) ===\n{intent}\n=== END INTENT ===\n\n"
        "Begin the attack list."
    )
    return system, user


async def probe(client: httpx.AsyncClient, model: str, system: str, user: str) -> dict:
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": 0.3,
        "max_tokens": MAX_TOKENS,
    }
    t0 = datetime.now(timezone.utc)
    try:
        r = await client.post(
            f"{URL}/chat/completions",
            json=payload,
            headers={"Authorization": f"Bearer {KEY}"},
            timeout=TIMEOUT,
        )
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        if r.status_code != 200:
            return {"model": model, "status": r.status_code, "elapsed_s": elapsed, "err": r.text[:300]}
        data = r.json()
        choice = (data.get("choices") or [{}])[0]
        msg = choice.get("message", {})
        content = msg.get("content") or msg.get("reasoning_content") or ""
        return {
            "model": model,
            "status": 200,
            "elapsed_s": elapsed,
            "finish_reason": choice.get("finish_reason"),
            "usage": data.get("usage", {}),
            "content_len": len(content),
        }
    except Exception as e:
        elapsed = (datetime.now(timezone.utc) - t0).total_seconds()
        return {"model": model, "status": "exception", "elapsed_s": elapsed, "err": f"{type(e).__name__}: {e}"}


async def main() -> None:
    system, user = build_prompts()
    print(f"System: {len(system):,} chars  User: {len(user):,} chars  max_tokens={MAX_TOKENS}")
    print(f"Probing {len(MODELS)} gateway models in parallel: {', '.join(MODELS)}")
    print()

    async with httpx.AsyncClient() as client:
        results = await asyncio.gather(*[probe(client, m, system, user) for m in MODELS])

    for r in results:
        if r["status"] == 200:
            print(f"  {r['model']:24s} OK     {r['elapsed_s']:6.1f}s  finish={r['finish_reason']:8s}  content={r['content_len']:,} chars  usage={r['usage']}")
        else:
            print(f"  {r['model']:24s} {str(r['status']):6s} {r['elapsed_s']:6.1f}s  err={r.get('err','')[:200]}")

    # Hypothesis discriminator
    print()
    timeouts = [r for r in results if r["status"] != 200 and r["elapsed_s"] > 200]
    successes = [r for r in results if r["status"] == 200]
    print(f"Summary: {len(successes)}/{len(MODELS)} succeeded, {len(timeouts)} hit ~240s timeout")

    timeout_models = sorted(r["model"] for r in timeouts)
    success_models = sorted(r["model"] for r in successes)
    if not timeouts:
        print("HYPOTHESIS (b) supported — kimi-k2-6-specific routing slowdown.")
    elif len(timeouts) == len(MODELS):
        print("HYPOTHESIS (a) supported — gateway hard cap (every model).")
    elif sorted(["glm-4-7-flash", "gpt-oss-120b"]) == timeout_models:
        print("HYPOTHESIS (c) supported — reasoning models hit cap; non-reasoning models don't.")
    else:
        print(f"Mixed result — timeouts: {timeout_models}; successes: {success_models}")


if __name__ == "__main__":
    asyncio.run(main())
