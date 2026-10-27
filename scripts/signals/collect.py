#!/usr/bin/env python3
"""Collect third-party context signals for catalog entries.

These are DATA, not evidence. Nothing here can raise a card's **Type:**; the
flags this script emits are objective facts (a RUSTSEC advisory exists, the
repository is archived), and the pointers it collects are leads a human
promotes - or ignores. See catalog/signals.schema.json and METHODOLOGY.md.

Stdlib only, no keys:
  - crates.io        crate identity, downloads, reverse dependencies, yanks
  - HN Algolia       mention volume + critique pointers (comments, not posts)
  - RUSTSEC          advisories, from the advisory-db index
  - gh CLI           archived / pushed_at / stars (authenticated, no scraping)

Usage:
  python3 scripts/signals/collect.py            # whole catalog
  python3 scripts/signals/collect.py --limit 5  # first N repos (smoke test)
  python3 scripts/signals/collect.py --only BurntSushi/ripgrep

Politeness: crates.io is capped at 1 request/s per its API policy, HN Algolia
at 5/s; both sleeps are unconditional so a rerun is reproducible.
"""
import argparse
import json
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "catalog" / "projects.json"
OUT = ROOT / "catalog" / "signals.json"
UA = "abr-signals/1.0 (+https://github.com/rager306/awesome-blazing-rust)"

# Machine flags are facts only. Anything that needs judgement is a pointer in
# `disputes` or a human `verdict` - never an automatic accusation.
FLAGS = ("security", "unmaintained")

# An advisory older than this is history, not a current risk: RUSTSEC keeps
# every advisory forever (wasmtime alone carries 65), so flagging on presence
# would flag 12 of 94 repositories. The advisory id encodes its year.
SECURITY_WINDOW_YEARS = 2

# Dispute extraction by keyword was measured and rejected: across 94
# repositories it produced 21 pointers, and manual review found none of them to
# be a critique of the project's own numbers (they were comments that mention
# the project and criticise something else). A stricter rule - the comment must
# link the repository and carry the criticism in the project's own sentence -
# returned zero. See runs/signals-2026-10/RESULTS.md. Disputes therefore stay a
# human/agent input, and the collector never fills the field.


def http_json(url, timeout=25):
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read().decode())


def repo_slug(url):
    u = url.strip().rstrip("/")
    if not u.startswith("https://github.com/"):
        return None
    parts = u[len("https://github.com/"):].split("/")
    if len(parts) < 2 or not parts[0] or not parts[1]:
        return None
    return f"{parts[0]}/{parts[1].removesuffix('.git')}"


def norm_github(url):
    if not url:
        return None
    u = url.strip().rstrip("/").lower()
    for prefix in ("http://", "https://", "www."):
        if u.startswith(prefix):
            u = u[len(prefix):]
    if u.startswith("github.com/"):
        u = u[len("github.com/"):]
    return u.removesuffix(".git")


def gh_api(path):
    """GitHub through the authenticated CLI the repository already depends on."""
    r = subprocess.run(
        ["gh", "api", path, "--jq",
         "{archived, pushed_at, stargazers_count, license: .license.spdx_id}"],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        return None
    try:
        return json.loads(r.stdout)
    except json.JSONDecodeError:
        return None


def crates_lookup(name, slug):
    """crates.io by repository name, accepted only when the crate points back
    at this repository - a name collision must not attach someone else's
    downloads to a catalog entry."""
    try:
        data = http_json(f"https://crates.io/api/v1/crates/{urllib.parse.quote(name)}")
    except urllib.error.HTTPError as e:
        return None if e.code == 404 else {"error": f"HTTP {e.code}"}
    except Exception as e:  # noqa: BLE001 - a collector must not die on one source
        return {"error": str(e)[:120]}
    c = data.get("crate") or {}
    if norm_github(c.get("repository")) != norm_github(f"https://github.com/{slug}"):
        return None
    out = {
        "name": c.get("name"),
        "downloads": c.get("downloads"),
        "recent_downloads": c.get("recent_downloads"),
        "versions": c.get("num_versions"),
        "yanked": c.get("yanked"),
        "updated_at": c.get("updated_at"),
    }
    # reverse_dependencies is deliberately not recorded: the endpoint reports
    # meta.total = 1 regardless of real dependents (verified 2026-10-27 on
    # ripgrep), and a wrong adoption figure is worse than a missing one.
    return out


def hn_mentions(name, slug):
    """Volume only - it is a loss function for anomaly detection, never a score."""
    q = urllib.parse.quote(f'"{name}"')
    try:
        stories = http_json(f"https://hn.algolia.com/api/v1/search?query={q}&tags=story&hitsPerPage=1")
        comments = http_json(f"https://hn.algolia.com/api/v1/search?query={q}&tags=comment&hitsPerPage=1")
    except Exception as e:  # noqa: BLE001
        return {"error": str(e)[:120]}
    return {
        "stories": (stories.get("nbHits") or 0),
        "comments": (comments.get("nbHits") or 0),
    }


def machine_flags(github, advisories):
    """Objective facts only: an advisory inside the window, or a dead repo."""
    flags = []
    current_year = datetime.now(timezone.utc).year
    recent = [
        a for a in advisories
        if a.get("id", "").split("-")[1].isdigit()
        and int(a["id"].split("-")[1]) > current_year - SECURITY_WINDOW_YEARS
    ]
    if recent:
        flags.append("security")
    github = github or {}
    if github.get("archived") is True:
        flags.append("unmaintained")
    elif github.get("pushed_at"):
        try:
            pushed = datetime.fromisoformat(github["pushed_at"].replace("Z", "+00:00"))
            if (datetime.now(timezone.utc) - pushed).days > 365:
                flags.append("unmaintained")
        except ValueError:
            pass
    return flags


def rustsec_index():
    """One listing call; per-crate fetches happen only for crates present."""
    try:
        arr = http_json("https://api.github.com/repos/rustsec/advisory-db/contents/crates", 30)
    except Exception:  # noqa: BLE001
        return None
    if not isinstance(arr, list):
        return None
    return {x["name"] for x in arr}


def rustsec_for(name):
    try:
        arr = http_json(f"https://api.github.com/repos/rustsec/advisory-db/contents/crates/{urllib.parse.quote(name)}", 30)
    except Exception:  # noqa: BLE001
        return []
    if not isinstance(arr, list):
        return []
    return [{"id": f.get("name", "").removesuffix(".md")} for f in arr if f.get("name", "").endswith(".md")]


def load_previous():
    """Human verdicts and notes survive a recollect: a maintainer decision is
    not something a scheduled job may quietly overwrite."""
    if not OUT.exists():
        return {}, {}
    try:
        doc = json.loads(OUT.read_text())
    except json.JSONDecodeError:
        return {}, {}
    return (
        {r["repo"]: r.get("verdict") for r in doc.get("repos", []) if r.get("verdict")},
        {r["repo"]: r.get("notes") for r in doc.get("repos", []) if r.get("notes")},
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--only", default="")
    ap.add_argument("--refresh-flags", action="store_true",
                    help="recompute machine flags from the stored file, no network")
    args = ap.parse_args()

    if args.refresh_flags:
        doc = json.loads(OUT.read_text())
        for row in doc["repos"]:
            row["flags"] = machine_flags(row.get("github"), row.get("advisories") or [])
            # Pointers this collector used to emit carry a `marker` field and are
            # the ones the experiment rejected; a human-entered dispute has no
            # marker and survives.
            row["disputes"] = [d for d in (row.get("disputes") or []) if "marker" not in d]
        doc["meta"]["generated_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        OUT.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
        flagged = sum(1 for r in doc["repos"] if r["flags"])
        print(f"refreshed flags: {flagged}/{len(doc['repos'])} repos flagged", file=sys.stderr)
        return

    projects = json.loads(CATALOG.read_text())["projects"]
    slugs = sorted({s for s in (repo_slug(p["url"]) for p in projects) if s})
    if args.only:
        slugs = [s for s in slugs if s.lower() == args.only.lower()]
    if args.limit:
        slugs = slugs[: args.limit]

    verdicts, notes = load_previous()
    index = rustsec_index()
    rows = []
    for i, slug in enumerate(slugs, 1):
        name = slug.split("/")[1]
        owner, repo = slug.split("/")
        meta = gh_api(f"repos/{slug}") or {}
        time.sleep(0.35)
        crate = crates_lookup(name, slug)
        time.sleep(1.05)  # crates.io: 1 request/s
        attention = hn_mentions(name, slug)
        time.sleep(0.2)
        # Disputes are a human/agent input: keyword extraction was measured and
        # rejected (runs/signals-2026-10). The field stays in the schema so a
        # maintainer can record a real one.
        disputes = []

        advisories = []
        if index is not None and name in index:
            advisories = rustsec_for(name)

        flags = machine_flags(meta, advisories)

        row = {
            "repo": slug,
            "observed_at": datetime.now(timezone.utc).strftime("%Y-%m-%d"),
            "github": {
                "archived": meta.get("archived"),
                "pushed_at": pushed,
                "stars": meta.get("stargazers_count"),
                "license": meta.get("license"),
            },
            "crate": crate,
            "attention": attention,
            "advisories": advisories,
            "disputes": disputes,
            "flags": flags,
        }
        if slug in verdicts:
            row["verdict"] = verdicts[slug]
        if slug in notes:
            row["notes"] = notes[slug]
        rows.append(row)
        print(f"[{i}/{len(slugs)}] {slug}: crate={'yes' if crate and 'error' not in crate else 'no'} "
              f"flags={flags or '-'} disputes={len(disputes)}", file=sys.stderr)

    doc = {
        "meta": {
            "format": "awesome-blazing-rust signals v1",
            "rule": "data, not evidence: signals can only hold or demote a card, never raise its Type",
            "sources": ["crates.io", "hn.algolia.com", "rustsec/advisory-db", "api.github.com via gh"],
            "flag_vocabulary": list(FLAGS) + ["contested", "hype-suspect", "astroturf-suspected", "stale-claim"],
            "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "repos": len(rows),
        },
        "repos": sorted(rows, key=lambda r: r["repo"]),
    }
    OUT.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {OUT.relative_to(ROOT)} ({len(rows)} repos)", file=sys.stderr)


if __name__ == "__main__":
    main()
