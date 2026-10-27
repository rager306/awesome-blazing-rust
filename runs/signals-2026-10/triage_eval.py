#!/usr/bin/env python3
"""Measure the triage step on a known set.

dispute-fixture.json holds the 21 pointers the keyword prefilter produced across
94 repositories. Each was reviewed by hand: none is a critique of the project's
own numbers. So every item the model keeps is a FALSE POSITIVE, and the score of
this file is "how many false positives survived triage".

Without SUBQ_API_KEY the harness cannot judge anything and says so instead of
inventing a number.
"""
import json
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts" / "signals"))
import triage  # noqa: E402

MIN_P = 0.8


def main():
    fixture = json.loads((HERE / "dispute-fixture.json").read_text())
    items = fixture["items"]
    if not os.environ.get("SUBQ_API_KEY"):
        print(f"no SUBQ_API_KEY: {len(items)} fixture items cannot be triaged")
        print("expected outcome once a key exists: 0 kept (all are known false positives)")
        return 0
    kept = []
    for it in items:
        answers = triage.decide(it["text"], {"critique": triage.critique_question(it["repo"])})
        p = triage.noul_p(answers, "critique")
        verdict = "kept" if (p is not None and p >= MIN_P) else "dropped"
        print(f'{verdict:8} p={p if p is None else round(p, 2)!s:5} {it["repo"]:30} {it["url"]}')
        if verdict == "kept":
            kept.append(it["url"])
    print(f"\nkept {len(kept)}/{len(items)} - every kept item is a false positive by construction")
    return 1 if kept else 0


if __name__ == "__main__":
    sys.exit(main())
