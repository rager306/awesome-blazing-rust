#!/usr/bin/env python3
"""Stop if an intake issue changed repository after research."""

import importlib.util
import os
from pathlib import Path
import subprocess
import sys


spec = importlib.util.spec_from_file_location("intake_dedup", Path(__file__).with_name("intake-dedup.py"))
dedup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dedup)


def main():
    target = os.environ["TARGET_REPOSITORY"]
    issue = int(os.environ["ISSUE_NUMBER"])
    expected = int(os.environ["REPO_ID"])
    if issue <= 0 or expected <= 0:
        raise ValueError("invalid issue or repository ID")
    if dedup.issue_id(target, issue, {}) != expected:
        raise ValueError(f"issue #{issue} now points to a different repository; retry intake")


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"::error::issue identity check failed: {error}", file=sys.stderr)
        sys.exit(1)
