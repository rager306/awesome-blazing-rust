#!/usr/bin/env python3
"""Produce a read-only intake inventory; never decide whether to retry."""

import json
import os
import re
import subprocess
import sys


def pages(endpoint):
    result = subprocess.run(
        ["gh", "api", "--paginate", "--slurp", endpoint],
        check=True,
        capture_output=True,
        text=True,
    )
    data = json.loads(result.stdout)
    if not isinstance(data, list) or not data or any(not isinstance(p, list) for p in data):
        raise ValueError("invalid API pages")
    return [item for page in data for item in page]


def positive_number(value):
    return type(value) is int and value > 0


def marker_ids(body):
    if body is None:
        return set()
    if not isinstance(body, str):
        raise ValueError("invalid body")
    return {int(value) for value in re.findall(r"<!-- abr-repo-id: ([1-9][0-9]*) -->", body)}


def build_report(issues, pulls):
    issue_numbers = set()
    issue_markers = {}
    for issue in issues:
        if not isinstance(issue, dict):
            raise ValueError("invalid issue")
        if "pull_request" in issue:
            continue
        number = issue.get("number")
        labels = issue.get("labels")
        if (not positive_number(number) or number in issue_numbers or "body" not in issue
                or issue.get("state") != "open" or not isinstance(labels, list)
                or any(not isinstance(label, dict) or not isinstance(label.get("name"), str)
                       for label in labels)
                or not any(label["name"] == "card-intake" for label in labels)):
            raise ValueError("invalid or inconsistent intake issue")
        issue_numbers.add(number)
        issue_markers[number] = marker_ids(issue.get("body"))

    matches = {number: [] for number in issue_numbers}
    seen_pulls = set()
    for pull in pulls:
        if not isinstance(pull, dict):
            raise ValueError("invalid pull request")
        number = pull.get("number")
        head = pull.get("head")
        state = pull.get("state")
        if (not positive_number(number) or number in seen_pulls or "body" not in pull
                or state not in ("open", "closed") or not isinstance(head, dict)
                or not isinstance(head.get("ref"), str)):
            raise ValueError("invalid or inconsistent pull request")
        seen_pulls.add(number)
        legacy = re.fullmatch(r"agent/card-([1-9][0-9]*)-([1-9][0-9]*)", head["ref"])
        branch = re.fullmatch(r"agent/card-repo-([1-9][0-9]*)", head["ref"])
        body = pull.get("body") or ""
        pull_markers = marker_ids(body)
        linked_issues = {int(value) for value in re.findall(
            r"(?im)^(?:Draft card from|Closes)\s+#([1-9][0-9]*)\.?\s*$", body
        )}
        for issue_number in matches:
            reasons = []
            if legacy and int(legacy[1]) == issue_number:
                reasons.append("legacy issue branch")
            if issue_number in linked_issues:
                reasons.append("body issue reference")
            if len(issue_markers[issue_number]) == 1:
                repo_id = next(iter(issue_markers[issue_number]))
                if branch and int(branch[1]) == repo_id:
                    reasons.append("repository ID branch")
                if repo_id in pull_markers:
                    reasons.append("body marker")
            if reasons:
                matches[issue_number].append((number, state, "/".join(reasons)))

    lines = [
        "## Intake reconciliation: HUMAN REVIEW REQUIRED",
        "",
        "Read-only inventory of open `card-intake` issues. No dispatches or GitHub changes were made.",
        "PR matches use legacy branches, repository ID branches, body issue references, or body markers. These are unverified hints, not proof of identity, issue linkage, or processing status.",
        "Missing or ambiguous issue markers are unknown; legacy branch matches may be incomplete. Resolve repository identity against the GitHub API before acting.",
        "Runs, dispatch outcomes, comments, and other PR branches are not checked. API pages are not an atomic snapshot.",
        "Before any manual retry, inspect current issue state, Actions runs, and PRs; an unknown dispatch result may already have started a run.",
        "",
    ]
    if not issue_numbers:
        lines.append("No open intake issues returned.")
    for number in sorted(issue_numbers):
        ids = issue_markers[number]
        marker = str(next(iter(ids))) if len(ids) == 1 else "unknown (missing or ambiguous)"
        prs = ", ".join(f"#{n} ({state}; {reason})" for n, state, reason in sorted(matches[number])) or "none found"
        lines.append(f"- Issue #{number}: HUMAN REVIEW REQUIRED; repository ID marker: {marker}; PR hints: {prs}")
    return "\n".join(lines) + "\n"


def main():
    repository = os.environ["GITHUB_REPOSITORY"]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("invalid GITHUB_REPOSITORY")
    issues = pages(f"repos/{repository}/issues?state=open&labels=card-intake&per_page=100")
    pulls = pages(f"repos/{repository}/pulls?state=all&per_page=100")
    print(build_report(issues, pulls), end="")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f"Intake report failed; no reconciliation decision was made: {type(exc).__name__}", file=sys.stderr)
        sys.exit(1)
