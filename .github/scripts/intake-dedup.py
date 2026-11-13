#!/usr/bin/env python3
"""Fail closed before creating an intake branch or draft PR."""

import json
import os
import re
import subprocess
import sys


MARKER = re.compile(r"<!--\s*abr-repo-id:\s*(\d+)\s*-->", re.I)
LEGACY_BRANCH = re.compile(r"^agent/card-(\d+)-(\d+)$")
STABLE_BRANCH = re.compile(r"^agent/card-repo-(\d+)$")
FIELD = re.compile(
    r"(?im)^(?:\*\*Repository URL\*\*|###[ \t]+Repository URL)[ \t]*\r?\n"
    r"(?:[ \t]*\r?\n)?([^\r\n]*)"
)
URL = re.compile(r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)", re.I)


def api(path, paginate=False):
    args = ["gh", "api", path]
    if paginate:
        args += ["--paginate", "--slurp"]
    result = subprocess.run(args, capture_output=True, text=True, check=True)
    data = json.loads(result.stdout)
    if paginate:
        if not isinstance(data, list) or any(not isinstance(page, list) for page in data):
            raise ValueError(f"unexpected paginated response for {path}")
        return [item for page in data for item in page]
    if not isinstance(data, dict):
        raise ValueError(f"unexpected response for {path}")
    return data


def repo_slug(match):
    owner, name = match.group(1), match.group(2)
    if name.lower().endswith('.git'):
        name = name[:-4]
    return f"{owner}/{name}"

def issue_id(target, number, cache):
    if number in cache:
        return cache[number]
    try:
        issue = api(f"repos/{target}/issues/{number}")
    except (subprocess.CalledProcessError, OSError, ValueError) as error:
        raise ValueError(f"cannot read legacy issue #{number}; maintainer must inspect it before retrying") from error
    body = issue.get("body") or ""
    if not isinstance(body, str):
        raise ValueError(f"invalid body for issue #{number}")
    field = FIELD.search(body)
    url = URL.search(field.group(1)) if field else None
    if not url:
        raise ValueError(f"cannot identify repository in legacy issue #{number}")
    slug = repo_slug(url)
    try:
        repo = api(f"repos/{slug}")
    except (subprocess.CalledProcessError, OSError, ValueError) as error:
        raise ValueError(f"cannot resolve repository in legacy issue #{number} ({slug}); maintainer must inspect it before retrying") from error
    identity = repo.get("id")
    if type(identity) is not int or identity <= 0:
        raise ValueError(f"invalid repository ID for legacy issue #{number}")
    markers = {int(value) for value in MARKER.findall(body)}
    if markers and markers != {identity}:
        raise ValueError(f"conflicting repository ID in issue #{number}")
    cache[number] = identity
    return identity


def main():
    target = os.environ["TARGET_REPOSITORY"]
    slug = os.environ["REPO_SLUG"]
    repo_id = int(os.environ["REPO_ID"])
    issue = int(os.environ["ISSUE_NUMBER"])
    if repo_id <= 0 or issue <= 0 or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", target) or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", slug):
        raise ValueError("invalid intake identity")

    pulls = api(f"repos/{target}/pulls?state=open&per_page=100", paginate=True)
    matches = []
    cache = {}
    for pr in pulls:
        if not isinstance(pr, dict) or not isinstance(pr.get("number"), int) or not isinstance(pr.get("head"), dict):
            raise ValueError("incomplete pull request response")
        body = pr.get("body") or ""
        head = pr["head"].get("ref")
        if not isinstance(body, str) or not isinstance(head, str):
            raise ValueError("invalid pull request response")
        markers = {int(value) for value in MARKER.findall(body)}
        urls = {repo_slug(m).lower() for m in URL.finditer(body)}
        linked = slug.lower() in urls
        head_repo = pr["head"].get("repo")
        head_repo_name = head_repo.get("full_name") if isinstance(head_repo, dict) else None
        if head.startswith("agent/card-") and not isinstance(head_repo_name, str):
            raise ValueError(f"missing head repository in PR #{pr['number']}; cannot verify branch identity")
        local_branch = isinstance(head_repo_name, str) and head_repo_name.lower() == target.lower()
        legacy = LEGACY_BRANCH.fullmatch(head) if local_branch else None
        stable = STABLE_BRANCH.fullmatch(head) if local_branch else None
        stable_id = int(stable.group(1)) if stable else None
        labels = pr.get("labels", [])
        if not isinstance(labels, list) or any(not isinstance(label, dict) or not isinstance(label.get("name"), str) for label in labels):
            raise ValueError(f"invalid labels in PR #{pr['number']}")
        intake_candidate = bool(legacy or (local_branch and head.startswith("agent/card-repo-")) or any(label["name"] == "card-draft" for label in labels))
        if not intake_candidate:
            if linked or repo_id in markers:
                raise ValueError(f"PR #{pr['number']} mentions this repository but is not an identified intake; maintainer review required")
            continue
        if len(markers) > 1:
            raise ValueError(f"ambiguous IDs in PR #{pr['number']}")
        legacy_id = None
        if legacy:
            try:
                legacy_id = issue_id(target, int(legacy.group(1)), cache)
            except ValueError as error:
                raise ValueError(f"cannot verify legacy PR #{pr['number']}: {error}") from error
        identity_fields = re.findall(r"(?im)^Repository:\s*(https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)\s*$", body)
        if len(identity_fields) > 1:
            raise ValueError(f"ambiguous repository field in PR #{pr['number']}")
        body_id = None
        if identity_fields:
            match = URL.fullmatch(identity_fields[0])
            body_slug = repo_slug(match)
            body_id = api(f"repos/{body_slug}").get("id")
            if type(body_id) is not int or body_id <= 0:
                raise ValueError(f"invalid repository identity in PR #{pr['number']}")
        if markers and (body_id is None and legacy_id is None or markers != {body_id or legacy_id}):
            raise ValueError(f"unverified or conflicting identity in PR #{pr['number']}")
        if legacy_id is not None and body_id is not None and legacy_id != body_id:
            raise ValueError(f"conflicting repository in PR #{pr['number']}")
        if stable_id is not None and body_id is not None and stable_id != body_id:
            raise ValueError(f"conflicting branch and body repository IDs in PR #{pr['number']}")
        if legacy_id == repo_id or body_id == repo_id or stable_id == repo_id:
            matches.append(pr["number"])
        elif linked:
            raise ValueError(f"PR #{pr['number']} mentions this repository but its subject is unverified; maintainer review required")

    if len(matches) > 1:
        raise ValueError(f"multiple open PRs match repository: {matches}")
    if matches:
        url = f"https://github.com/{target}/pull/{matches[0]}"
        print(f"::notice::Existing PR matches this repository: {url}; leaving it unchanged.")
        summary = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary:
            with open(summary, "a", encoding="utf-8") as output:
                output.write(f"Existing PR matches this repository: {url}. No changes made.\n")
        exists = "true"
    else:
        branches = api(f"repos/{target}/branches?per_page=100", paginate=True)
        if any(not isinstance(branch, dict) or not isinstance(branch.get("name"), str) for branch in branches):
            raise ValueError("incomplete branch response")
        if any(branch["name"] == f"agent/card-repo-{repo_id}" for branch in branches):
            raise ValueError("intake branch already exists without a matching open PR; refusing to overwrite")
        exists = "false"
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"exists={exists}\n")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"::error::intake duplicate check failed: {error}", file=sys.stderr)
        sys.exit(1)
