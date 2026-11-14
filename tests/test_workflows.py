"""Offline tests of selected GitHub Actions shell steps.

Run with: python3 -m unittest discover -s tests -v
The runner needs bash, jq, and the other standard Ubuntu shell utilities.
"""

import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[1]


def workflow_step(workflow, name):
    """Read the actual literal run block, without requiring a YAML library."""
    source = (ROOT / ".github" / "workflows" / workflow).read_text()
    step = source.split(f"      - name: {name}\n", 1)[1]
    step = step.split("      - name:", 1)[0]
    block = step.split("        run: |\n", 1)[1]
    lines = []
    for line in block.splitlines():
        if line and not line.startswith("          "):
            break
        lines.append(line[10:] if line else "")
    script = "\n".join(lines) + "\n"
    return re.sub(r"\$\{\{.*?\}\}", "fixture", script)


class WorkflowTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cwd = Path(self.tmp.name)
        (self.cwd / "bin").mkdir()
        (self.cwd / "catalog").mkdir()
        (self.cwd / "catalog/projects.json").write_text(json.dumps({"projects": []}))
        self.env = os.environ.copy()
        self.env.update({
            "PATH": f"{self.cwd / 'bin'}:{self.env['PATH']}",
            "TEST_ROOT": str(self.cwd),
            "ISSUE_NUMBER": "42",
            "GITHUB_REPOSITORY": "example/catalog",
            "GITHUB_OUTPUT": str(self.cwd / "output"),
            "GITHUB_ENV": str(self.cwd / "github_env"),
        })

    def executable(self, name, content):
        path = self.cwd / "bin" / name
        path.write_text(textwrap.dedent(content).lstrip())
        path.chmod(0o755)

    def run_step(self, workflow, name):
        return subprocess.run(
            ["bash", "-e", "-o", "pipefail", "-c", workflow_step(workflow, name)],
            cwd=self.cwd, env=self.env, text=True, capture_output=True, timeout=15,
        )

    def events(self):
        path = self.cwd / "events.jsonl"
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def mock_gh(self):
        self.executable("gh", '''
            #!/usr/bin/env python3
            import json, os, pathlib, sys
            root = pathlib.Path(os.environ["TEST_ROOT"])
            args = sys.argv[1:]
            with (root / "events.jsonl").open("a") as f:
                f.write(json.dumps(args) + "\\n")
            if args[:2] == ["issue", "view"]:
                print((root / "issue.txt").read_text())
            elif args[:1] == ["api"]:
                fixture = root / "api.json"
                if not fixture.exists():
                    sys.exit(1)
                print(fixture.read_text())
        ''')


@unittest.skipUnless(shutil.which("jq"), "jq is required by the workflow")
class IntakeGateTests(WorkflowTest):
    STEP = "Freshness and duplicate gate (pipeline-side, deterministic)"

    def setUp(self):
        super().setUp()
        self.mock_gh()
        (self.cwd / "issue.txt").write_text(
            "**Repository URL**\nhttps://github.com/Owner/Project.git/\n"
        )

    def output(self):
        path = self.cwd / "output"
        return path.read_text() if path.exists() else ""

    def test_duplicate_is_closed_after_identity_lookup(self):
        (self.cwd / "catalog/projects.json").write_text(json.dumps({
            "projects": [{"url": "https://github.com/owner/project"}]
        }))
        (self.cwd / "api.json").write_text(json.dumps({"id": 123, "full_name": "Owner/Project", "archived": False, "pushed_at": None}))
        result = self.run_step("intake.yml", self.STEP)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("skip=duplicate", self.output())
        self.assertTrue(any(e[:2] == ["issue", "close"] for e in self.events()))
        self.assertTrue(any(e[:1] == ["api"] for e in self.events()))

    def test_issue_form_repository_url_is_recognized(self):
        (self.cwd / "issue.txt").write_text(
            "### Repository URL\n\nhttps://github.com/Owner/Project.git/\n"
        )
        (self.cwd / "api.json").write_text(json.dumps({"id": 123, "full_name": "Owner/Project", "archived": True, "pushed_at": None}))
        result = self.run_step("intake.yml", self.STEP)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("skip=archived", self.output())
        self.assertTrue(any(e[:1] == ["api"] for e in self.events()))

    def test_repository_url_scheme_host_and_git_suffix_are_case_insensitive(self):
        (self.cwd / "issue.txt").write_text(
            "**Repository URL**\nHTTPS://GITHUB.COM/Owner/Project.GIT/\n"
        )
        (self.cwd / "api.json").write_text(json.dumps({"id": 123, "full_name": "Owner/Project", "archived": True, "pushed_at": None}))
        result = self.run_step("intake.yml", self.STEP)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("repo=Owner/Project\n", self.output())
        self.assertIn("skip=archived", self.output())
        self.assertTrue(any(e[:2] == ["api", "repos/Owner/Project"] for e in self.events()))

    def test_metadata_failure_does_not_close_or_continue(self):
        result = self.run_step("intake.yml", self.STEP)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(any(e[:2] == ["issue", "close"] for e in self.events()))
        self.assertNotIn("skip=\n", self.output())

    def test_bad_metadata_does_not_close_issue(self):
        for meta in [{}, {"id": 123, "full_name": "Owner/Project", "archived": False, "pushed_at": "not a date"},
                     {"id": 123, "full_name": "Owner/Project", "archived": "false", "pushed_at": "2025-01-01T00:00:00Z"}]:
            with self.subTest(meta=meta):
                (self.cwd / "api.json").write_text(json.dumps(meta))
                result = self.run_step("intake.yml", self.STEP)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertFalse(any(e[:2] == ["issue", "close"] for e in self.events()))
                self.assertNotIn("skip=\n", self.output())

    def test_archived_is_closed(self):
        (self.cwd / "api.json").write_text(json.dumps({"id": 123, "full_name": "Owner/Project", "archived": True, "pushed_at": None}))
        result = self.run_step("intake.yml", self.STEP)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("skip=archived", self.output())
        self.assertTrue(any(e[:2] == ["issue", "close"] for e in self.events()))


@unittest.skipUnless(shutil.which("jq"), "jq is required by the workflow")
class IntakeInsertionTests(WorkflowTest):
    def setUp(self):
        super().setUp()
        self.mock_gh()
        self.env["REPO_SLUG"] = "owner/new"
        self.env["REPO_ID"] = "123"
        (self.cwd / "api.json").write_text(json.dumps({"id": 123}))
        scripts = self.cwd / ".github/scripts"
        scripts.mkdir(parents=True)
        shutil.copy(ROOT / ".github/scripts/intake-card-identity.py", scripts)
        self.executable("git", '''
            #!/bin/bash
            echo "git $*" >> "$TEST_ROOT/order"
        ''')
        self.executable("cargo", '''
            #!/bin/bash
            echo "cargo $*" >> "$TEST_ROOT/order"
        ''')
        self.executable("npx", '''
            #!/bin/bash
            echo "npx $*" >> "$TEST_ROOT/order"
        ''')
        abr = self.cwd / "cli/target/release/abr"
        abr.parent.mkdir(parents=True)
        abr.write_text('#!/bin/bash\necho "abr $*" >> "$TEST_ROOT/order"\n')
        abr.chmod(0o755)
        (self.cwd / "README.md").write_text("# Index\n\n## Candidates\n\n## End\n")
        (self.cwd / "card.json").write_text(json.dumps({
            "card": "- [New](https://github.com/owner/new) - **Blazing:** speed. **Type:** tool. **Arch:** `cli`.",
            "skip": None, "what": "utility", "why": "speed", "weak": "review"}))

    def test_model_skip_does_not_close_issue_or_create_branch(self):
        (self.cwd / "card.json").write_text(json.dumps({"card": "", "skip": "not a fit"}))
        result = self.run_step("intake.yml", "Insert the card and run the gates")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        events = self.events()
        self.assertTrue(any(e[:2] == ["issue", "comment"] for e in events))
        self.assertFalse(any(e[:2] == ["issue", "close"] for e in events))
        self.assertIn("skip=agent", (self.cwd / "github_env").read_text())
        self.assertFalse((self.cwd / "order").exists())

    def test_latest_base_and_projections_are_prepared_before_gates(self):
        refresh = self.run_step("intake.yml", "Refresh main for finalization")
        self.assertEqual(refresh.returncode, 0, refresh.stdout + refresh.stderr)
        result = self.run_step("intake.yml", "Insert the card and run the gates")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        order = (self.cwd / "order").read_text().splitlines()
        checkout = next(i for i, event in enumerate(order) if event.startswith("git checkout --detach"))
        stage = next(i for i, event in enumerate(order) if event.startswith("git add"))
        gate = order.index("abr check")
        self.assertLess(checkout, order.index("abr catalog"))
        self.assertLess(order.index("abr llms"), stage)
        self.assertLess(stage, gate)
        self.assertLess(gate, next(i for i, event in enumerate(order) if event.startswith("npx ")))
        readme = (self.cwd / "README.md").read_text()
        self.assertLess(readme.index("[New]"), readme.index("## End"))


@unittest.skipUnless(shutil.which("jq"), "jq is required by the workflow")
class DiscoveryTests(WorkflowTest):
    STEP = "Open intake issues for new candidates"

    def setUp(self):
        super().setUp()
        self.executable("gh", '''
            #!/usr/bin/env python3
            import json, os, pathlib, sys
            root = pathlib.Path(os.environ["TEST_ROOT"])
            args = sys.argv[1:]
            with (root / "events.jsonl").open("a") as f:
                f.write(json.dumps(args) + "\\n")
            if args[0] == "api":
                fixture = root / "pages.json"
                if not fixture.exists():
                    sys.exit(1)
                print(fixture.read_text())
            elif args[:2] == ["issue", "create"]:
                if os.environ.get("FAIL_CREATE"):
                    sys.exit(1)
                print("https://github.com/example/catalog/issues/123")
            elif args[:2] == ["workflow", "run"] and os.environ.get("FAIL_DISPATCH"):
                sys.exit(1)
        ''')
        (self.cwd / "pages.json").write_text("[[]]")

    def candidate(self, slug):
        return {"name": slug, "repo_url": f"https://github.com/{slug}",
                "benchmark_url": "https://example.com/bench", "what": "utility",
                "why": "benchmark", "source_url": "https://example.com/source", "provider": "exa"}

    def candidates(self, candidates):
        (self.cwd / "candidates.json").write_text(json.dumps(candidates))

    def created(self):
        return [e for e in self.events() if e[:2] == ["issue", "create"]]

    def test_pages_formats_and_in_batch_dedup(self):
        pages = [[{"body": "**Repository URL**\nhttps://github.com/A/Old.git/"}],
                 [{"body": "### Repository URL\n\nhttps://github.com/B/Decided"}]]
        (self.cwd / "pages.json").write_text(json.dumps(pages))
        self.candidates([self.candidate(s) for s in
                         ["a/old", "b/decided", "C/New.git/", "c/new", "d/fresh"]])
        result = self.run_step("discovery.yml", self.STEP)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(self.created()), 2)
        dispatches = [e for e in self.events() if e[:2] == ["workflow", "run"]]
        self.assertEqual(len(dispatches), 2)
        self.assertTrue(any("--paginate" in e and "--slurp" in e for e in self.events()))

    def test_invalid_history_or_lookup_failure_creates_nothing(self):
        self.candidates([self.candidate("x/new")])
        for history in [None, "not json", "{}", "[{}]"]:
            with self.subTest(history=history):
                path = self.cwd / "pages.json"
                if history is None:
                    path.unlink(missing_ok=True)
                else:
                    path.write_text(history)
                result = self.run_step("discovery.yml", self.STEP)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.created(), [])

    def test_create_failure_does_not_dispatch(self):
        self.env["FAIL_CREATE"] = "1"
        self.candidates([self.candidate("x/new")])
        result = self.run_step("discovery.yml", self.STEP)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(self.created()), 1)
        self.assertFalse(any(e[:2] == ["workflow", "run"] for e in self.events()))

    def test_dispatch_failure_alerts_without_retry_or_closing_issue(self):
        self.env["FAIL_DISPATCH"] = "1"
        self.candidates([self.candidate("x/new")])
        result = self.run_step("discovery.yml", self.STEP)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("::error::intake dispatch failed", result.stdout)
        events = self.events()
        self.assertEqual(len([e for e in events if e[:2] == ["workflow", "run"]]), 1)
        comments = [e for e in events if e[:2] == ["issue", "comment"]]
        self.assertEqual(len(comments), 1)
        self.assertEqual(comments[0][2], "123")
        self.assertIn("workflow_dispatch", comments[0][-1])
        self.assertIn("issue_number=123", comments[0][-1])
        self.assertIn("Check Actions before retrying", comments[0][-1])
        self.assertFalse(any(e[:2] in (["issue", "close"], ["issue", "delete"]) for e in events))

    def test_invalid_candidates_and_three_issue_cap(self):
        invalid = self.candidate("invalid/name")
        invalid["why"] = ""
        self.candidates([invalid] + [self.candidate(f"owner/{i}") for i in range(4)])
        result = self.run_step("discovery.yml", self.STEP)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(self.created()), 3)


if __name__ == "__main__":
    unittest.main()
