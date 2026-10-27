#!/usr/bin/env python3
"""Measure the triage step against labelled data.

dispute-fixture.json holds the 21 pointers the keyword prefilter produced across
94 repositories, each reviewed by hand: 1 genuine contradiction (datafusion,
"much slower than duckdb or OOMing" on db-benchmark - it contradicts the
ClickBench claim on the card) and 20 pieces of noise.

Two numbers matter:

  false positives - noise the model KEEPS. This is what would put a fake
                    "contested" flag on a card.
  false negatives - real critiques the model DROPS. This is what would let a
                    contradicted claim through.

The threshold is a product decision, not a constant: the run prints the full
probability list so it can be moved on evidence.
"""
import json
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "scripts" / "signals"))
import triage  # noqa: E402

MIN_P = 0.5


def main():
    fixture = json.loads((HERE / "dispute-fixture.json").read_text())
    items = fixture["items"]
    if not os.environ.get("SUBQ_API_KEY"):
        print(f"no SUBQ_API_KEY: {len(items)} fixture items cannot be triaged")
        return 0
    fp, fn, rows = [], [], []
    for it in items:
        answers = triage.decide(it["text"], {"critique": triage.critique_question(it["repo"])})
        p = triage.noul_p(answers, "critique")
        kept = p is not None and p >= MIN_P
        truth = it.get("label", "not_critique") == "critique"
        if kept and not truth:
            fp.append(it["url"])
        if truth and not kept:
            fn.append(it["url"])
        rows.append((p, kept, truth, it["repo"], it["url"]))
    for p, kept, truth, repo, url in sorted(rows, key=lambda r: -(r[0] or 0)):
        mark = "KEEP" if kept else "drop"
        tag = "critique" if truth else "noise"
        print(f'{mark} p={p if p is None else round(p, 2)!s:5} [{tag:8}] {repo:28} {url}')
    kept_total = sum(1 for _, kept, _, _, _ in rows if kept)
    print(f"\nthreshold {MIN_P}: kept {kept_total}/{len(rows)} "
          f"| false positives {len(fp)} | false negatives {len(fn)}")
    return 1 if fp else 0


if __name__ == "__main__":
    sys.exit(main())
