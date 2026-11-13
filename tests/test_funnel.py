"""Offline tests that execute the actual funnel workflow's report step."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


def report_script():
    source = (ROOT / ".github/workflows/funnel.yml").read_text()
    step = source.split("      - name: Build the funnel table\n", 1)[1]
    block = step.split("        run: |\n", 1)[1].split("      - name:", 1)[0]
    return "\n".join(line[10:] if line else "" for line in block.splitlines()) + "\n"


class FunnelTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cwd = Path(self.tmp.name)
        binary = self.cwd / "bin"
        binary.mkdir()
        gh = binary / "gh"
        gh.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
root = pathlib.Path(os.environ["TEST_ROOT"])
args = sys.argv[1:]
with (root / "calls.jsonl").open("a") as f:
    f.write(json.dumps(args) + "\\n")
if args[:2] == ["issue", "list"]:
    print((root / "issue-fixture.json").read_text())
elif args[:2] == ["pr", "list"]:
    print((root / "pr-fixture.json").read_text())
else:
    sys.exit(1)
''')
        gh.chmod(0o755)
        self.env = os.environ.copy()
        self.env.update(PATH=f"{binary}:{self.env['PATH']}", TEST_ROOT=str(self.cwd))

    def run_report(self, issues, prs):
        (self.cwd / "issue-fixture.json").write_text(json.dumps(issues))
        (self.cwd / "pr-fixture.json").write_text(json.dumps(prs))
        result = subprocess.run(["bash", "-e", "-c", report_script()], cwd=self.cwd,
                                env=self.env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return (self.cwd / "funnel.md").read_text()

    def test_counts_unique_issues_per_stage_and_ignores_unmatched_prs(self):
        issues = [
            {"number": 1, "body": "**Found via**\nExa"},
            {"number": 2, "body": "**Found via**\nexa"},
            {"number": 3, "body": None},
            {"number": 4, "body": "**Found via**\nTavily"},
        ]
        prs = [
            {"title": "draft issue #1", "mergedAt": None},
            {"title": "retry issue #1", "mergedAt": "2025-01-01"},
            {"title": "another issue #1", "mergedAt": "2025-01-02"},
            {"title": "draft issue #2", "mergedAt": None},
            {"title": "draft issue #3", "mergedAt": "2025-01-01"},
            {"title": "draft issue #999", "mergedAt": "2025-01-01"},
            {"title": "no issue number", "mergedAt": "2025-01-01"},
        ]
        report = self.run_report(issues, prs)
        self.assertIn("| exa | 2 | 2 | 1 | 100% | 50% |", report)
        self.assertIn("| unknown | 1 | 1 | 1 | 100% | 100% |", report)
        self.assertIn("| tavily | 1 | 0 | 0 | 0% | 0% |", report)
        self.assertIn("| found (issues) | drafted (issues) | merged (issues) |", report)
        self.assertIn("4 intake issues and 7 card-draft PRs", report)

    def test_caps_are_disclosed_and_used_in_both_queries(self):
        report = self.run_report([], [])
        self.assertIn("Each list is capped at 500", report)
        self.assertIn("Rates apply only to these retrieved records", report)
        calls = [json.loads(line) for line in (self.cwd / "calls.jsonl").read_text().splitlines()]
        self.assertEqual([c[:2] for c in calls], [["issue", "list"], ["pr", "list"]])
        for call in calls:
            self.assertEqual(call[call.index("--limit") + 1], "500")
            self.assertEqual(call[call.index("--state") + 1], "all")


if __name__ == "__main__":
    unittest.main()
