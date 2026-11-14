#!/usr/bin/env python3
"""Validate an agent card and bind its project link to the requested repository."""

import json
import os
import re
import subprocess
import sys


def main():
    with open("card.json", encoding="utf-8") as source:
        card = json.load(source)["card"]
    if not isinstance(card, str) or not card or any(char in card for char in "\r\n\x00"):
        raise ValueError("card must be a nonempty single line")
    match = re.match(r"^- \[[^\]\r\n]+\]\(https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)\)", card, re.I)
    if not match:
        raise ValueError("card must start with a GitHub repository link")
    owner, name = match.group(1), match.group(2)
    if name.lower().endswith('.git'):
        name = name[:-4]
    slug = f"{owner}/{name}"
    result = subprocess.run(["gh", "api", f"repos/{slug}"], text=True, capture_output=True, check=True)
    identity = json.loads(result.stdout).get("id")
    expected = int(os.environ["REPO_ID"])
    if type(identity) is not int or identity <= 0 or identity != expected:
        raise ValueError("card repository does not match the requested repository")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"::error::invalid card: {error}", file=sys.stderr)
        sys.exit(1)
