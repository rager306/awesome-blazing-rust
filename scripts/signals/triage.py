#!/usr/bin/env python3
"""Typed decisions for the signals layer, on a small model instead of a big one.

SubQ's Decisions API returns probabilities for noul (yes/no), choice and score
questions - the shape this repository actually needs. A judgment like "is this
HN comment a critique of *this project's own numbers*" is one noul question,
not a chat completion, so it costs a fraction of a cent and cannot produce
prose that has to be parsed twice.

Contract: https://docs.subq.ai/markdown/endpoints/decisions-api.md
  POST https://api.subq.ai/v1/systemone
  {"model": "subq-snap-1.1", "state": "...", "questions": {...}}
  -> {"answers": {"key": {"noul": 0.93, "confidence": 0.8}}}

No key means no judgment: `decide()` returns None and every caller degrades to
"no signal", which is the safe direction (signals can only hold or demote).
"""
import json
import os
import sys
import urllib.error
import urllib.request

DEFAULT_BASE = "https://api.subq.ai/v1"
DEFAULT_MODEL = "subq-snap-1.1"
UA = "abr-signals/1.0 (+https://github.com/rager306/awesome-blazing-rust)"

# Decisions the catalog asks. Keep them few, narrow and falsifiable: a question
# whose criteria cannot be written down is not a decision, it is an opinion.
def critique_question(slug):
    return {
        "type": "noul",
        "instructions": (
            f"Does this comment contradict or dispute the performance claims of the project "
            f"github.com/{slug} - its own benchmark numbers, its methodology, or the speed it advertises?"
        ),
        "criteria": {
            "true": (
                "The comment says this project is slower, wrong or misleading about its speed - its own "
                "numbers, its benchmark method, or results on another public benchmark that contradict "
                "what the project claims"
            ),
            "false": (
                "The comment only mentions the project in passing, compares it without disputing its "
                "claims, criticises a different project or topic, or reports a measurement without "
                "challenging the project's own"
            ),
        },
    }


def hype_question(slug, window_days=7, baseline_days=90):
    return {
        "type": "noul",
        "instructions": (
            f"Is the recent surge of mentions of github.com/{slug} explained by a release, "
            f"a benchmark result or a talk - rather than by coordinated promotion?"
        ),
        "criteria": {
            "true": "A concrete artifact explains the attention: a release, a published benchmark, a conference talk",
            "false": "No artifact; the attention looks promoted, repeated or paid for",
        },
    }


def scope_question():
    return {
        "type": "noul",
        "instructions": (
            "Is this repository a Rust project whose speed claim can be checked against "
            "a primary source - a benchmark, a methodology page or a published result?"
        ),
        "criteria": {
            "true": "Rust or Rust-core, with a benchmark or methodology that can be inspected",
            "false": "Not Rust, or no checkable speed claim at all",
        },
    }


def decide(state, questions, *, key=None, base=None, model=DEFAULT_MODEL, timeout=30):
    """Return the answers dict, or None when there is no key or the call fails."""
    key = key or os.environ.get("SUBQ_API_KEY", "")
    if not key:
        return None
    base = (base or os.environ.get("SUBQ_BASE_URL") or DEFAULT_BASE).rstrip("/")
    body = json.dumps({"model": model, "state": state, "questions": questions}).encode()
    req = urllib.request.Request(
        f"{base}/systemone",
        data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json", "User-Agent": UA},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            doc = json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        detail = ""
        try:
            detail = json.dumps(json.loads(e.read().decode()).get("detail", {}))[:200]
        except Exception:  # noqa: BLE001
            pass
        # 402 means the account is out of credits: every later request will fail
        # the same way, so the caller must stop rather than loop politely.
        if e.code == 402:
            raise NoCredits(detail or "insufficient credits") from e
        print(f"subq: HTTP {e.code} {detail}", file=sys.stderr)
        return None
    except Exception as e:  # noqa: BLE001 - a decision service must not break collection
        print(f"subq: {e}", file=sys.stderr)
        return None
    return doc.get("answers")


class NoCredits(RuntimeError):
    """402 from the API: stop asking, every call would fail the same way."""


def available_models(*, key=None, base=None, timeout=15):
    """Model ids this key may use. Docs: a key without a model gets 403."""
    key = key or os.environ.get("SUBQ_API_KEY", "")
    if not key:
        return []
    base = (base or os.environ.get("SUBQ_BASE_URL") or DEFAULT_BASE).rstrip("/")
    req = urllib.request.Request(
        f"{base}/models", headers={"Authorization": f"Bearer {key}", "User-Agent": UA}
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            doc = json.loads(r.read().decode())
    except Exception as e:  # noqa: BLE001
        print(f"subq: models lookup failed: {e}", file=sys.stderr)
        return []
    return [m.get("id") for m in doc.get("data", [])]


def noul_p(answers, name):
    """Probability of yes for one question, or None when the model was not asked."""
    if not answers or name not in answers:
        return None
    v = answers[name]
    if isinstance(v, dict):
        v = v.get("noul")
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def _self_test():
    sample = {"answers": {"critique": {"noul": 0.93, "confidence": 0.81}}}
    assert noul_p(sample["answers"], "critique") == 0.93
    assert noul_p(sample["answers"], "missing") is None
    assert noul_p(None, "critique") is None
    q = critique_question("owner/repo")
    assert q["type"] == "noul" and "owner/repo" in q["instructions"]
    # no key -> no judgment, never a guess
    assert decide("text", {"a": q}, key="") is None
    assert available_models(key="") == []
    print("triage self-test: OK")


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        _self_test()
    else:
        print(__doc__.strip().splitlines()[0])
        print("usage: triage.py --self-test   (called as a module by collect.py)")
