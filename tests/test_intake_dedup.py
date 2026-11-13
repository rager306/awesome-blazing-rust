import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "intake_dedup", Path(__file__).resolve().parents[1] / ".github/scripts/intake-dedup.py"
)
dedup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(dedup)


class IntakeDedupTests(unittest.TestCase):
    def run_check(self, responses, repo_id="42", args=()):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "output"
            env = {"TARGET_REPOSITORY": "org/catalog", "REPO_SLUG": "new/name", "REPO_ID": repo_id,
                   "ISSUE_NUMBER": "77", "GITHUB_OUTPUT": str(output)}
            def api(path, paginate=False):
                value = responses[path]
                if isinstance(value, Exception):
                    raise value
                return value
            with patch.dict(os.environ, env), patch.object(dedup, "api", side_effect=api), patch.object(dedup.sys, "argv", ["script", *args]):
                dedup.main()
            return output.read_text()

    def test_legacy_issue_resolves_alias(self):
        responses = {
            "repos/org/catalog/pulls?state=open&per_page=100": [
                {"number": 90, "head": {"ref": "agent/card-32-555", "repo": {"full_name": "org/catalog"}}, "body": "Draft card from #32."}
            ],
            "repos/org/catalog/issues/32": {"body": "### Repository URL\n\nhttps://github.com/old/name\n"},
            "repos/old/name": {"id": 42},
        }
        self.assertEqual(self.run_check(responses), "exists=true\n")

    def test_inaccessible_legacy_issue_reports_blocking_pr(self):
        responses = {
            "repos/org/catalog/pulls?state=open&per_page=100": [
                {"number": 90, "head": {"ref": "agent/card-32-555", "repo": {"full_name": "org/catalog"}}, "body": ""}
            ],
            "repos/org/catalog/issues/32": subprocess.CalledProcessError(1, "gh"),
        }
        with self.assertRaisesRegex(ValueError, r"legacy PR #90: cannot read legacy issue #32"):
            self.run_check(responses)

    def test_legacy_issue_normalizes_uppercase_git_suffix(self):
        responses = {
            "repos/org/catalog/pulls?state=open&per_page=100": [
                {"number": 90, "head": {"ref": "agent/card-32-555", "repo": {"full_name": "org/catalog"}}, "body": ""}
            ],
            "repos/org/catalog/issues/32": {"body": "### Repository URL\n\nHTTPS://GITHUB.COM/Old/Name.GIT/\n"},
            "repos/Old/Name": {"id": 42},
        }
        self.assertEqual(self.run_check(responses), "exists=true\n")

    def test_stable_branch_conflicting_body_fails_for_either_requested_id(self):
        for marker in ("", "\n<!-- abr-repo-id: 23 -->"):
            responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
                {"number": 90, "head": {"ref": "agent/card-repo-42", "repo": {"full_name": "org/catalog"}},
                 "body": "Repository: https://github.com/old/name" + marker}],
                "repos/old/name": {"id": 23}}
            for repo_id in ("42", "23", "99"):
                with self.subTest(marker=marker, repo_id=repo_id), self.assertRaisesRegex(ValueError, "conflicting branch and body repository IDs"):
                    self.run_check(responses, repo_id=repo_id)

    def test_stable_branch_agrees_with_body(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "agent/card-repo-42", "repo": {"full_name": "org/catalog"}},
             "body": "Repository: https://github.com/old/name\n<!-- abr-repo-id: 42 -->"}],
            "repos/old/name": {"id": 42}}
        self.assertEqual(self.run_check(responses), "exists=true\n")

    def test_stable_branch_without_body_identity_still_matches(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "agent/card-repo-42", "repo": {"full_name": "org/catalog"}}, "body": ""}]}
        self.assertEqual(self.run_check(responses), "exists=true\n")

    def test_fork_stable_branch_does_not_conflict_with_body(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "agent/card-repo-23", "repo": {"full_name": "fork/catalog"}},
             "labels": [{"name": "card-draft"}], "body": "Repository: https://github.com/old/name"}],
            "repos/old/name": {"id": 42}}
        self.assertEqual(self.run_check(responses), "exists=true\n")

    def test_existing_branch_is_never_overwritten(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [],
                     "repos/org/catalog/branches?per_page=100": [{"name": "agent/card-repo-42"}]}
        with self.assertRaisesRegex(ValueError, "refusing to overwrite"):
            self.run_check(responses)

    def test_missing_branch_allows_creation(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [],
                     "repos/org/catalog/branches?per_page=100": []}
        self.assertEqual(self.run_check(responses), "exists=false\n")

    def test_marker_without_confirming_url_fails_closed(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "someone"}, "labels": [{"name": "card-draft"}], "body": "<!-- abr-repo-id: 42 -->"}]}
        with self.assertRaisesRegex(ValueError, "unverified"):
            self.run_check(responses)

    def test_conflicting_marker_fails_closed(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "someone"}, "labels": [{"name": "card-draft"}], "body": "Repository: https://github.com/old/name\n<!-- abr-repo-id: 42 -->"}],
            "repos/old/name": {"id": 23}}
        with self.assertRaisesRegex(ValueError, "conflicting"):
            self.run_check(responses)

    def test_api_failure_fails_closed(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": subprocess.CalledProcessError(1, "gh")}
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_check(responses)

    def test_empty_lease_does_not_replace_existing_branch(self):
        with tempfile.TemporaryDirectory() as directory:
            remote = Path(directory) / "remote.git"
            local = Path(directory) / "local"
            def git(*args, cwd=None, check=True):
                return subprocess.run(["git", *map(str, args)], cwd=cwd, check=check,
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            git("init", "--bare", remote)
            git("init", local)
            git("config", "user.name", "Test", cwd=local)
            git("config", "user.email", "test@example.com", cwd=local)
            (local / "file").write_text("first")
            git("add", "file", cwd=local)
            git("commit", "-m", "first", cwd=local)
            git("remote", "add", "origin", remote, cwd=local)
            ref = "refs/heads/agent/card-repo-42"
            git("push", "origin", f"HEAD:{ref}", cwd=local)
            before = git("rev-parse", ref, cwd=remote).stdout
            (local / "file").write_text("second")
            git("commit", "-am", "second", cwd=local)
            result = git("push", f"--force-with-lease={ref}:", "origin", f"HEAD:{ref}", cwd=local, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(before, git("rev-parse", ref, cwd=remote).stdout)

    def test_fork_cannot_spoof_legacy_or_stable_branch(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "agent/card-32-555", "repo": {"full_name": "fork/catalog"}}, "body": ""},
            {"number": 91, "head": {"ref": "agent/card-repo-42", "repo": {"full_name": "fork/catalog"}}, "body": ""}],
            "repos/org/catalog/branches?per_page=100": []}
        self.assertEqual(self.run_check(responses), "exists=false\n")

    def test_missing_head_repo_fails_closed(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "agent/card-repo-42", "repo": None}, "body": ""}]}
        with self.assertRaisesRegex(ValueError, "missing head repository"):
            self.run_check(responses)

    def test_unrelated_marker_does_not_block_intake(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "human-work"}, "body": "Repository: https://github.com/unknown/other\n<!-- abr-repo-id: 91 -->"}],
            "repos/org/catalog/branches?per_page=100": []}
        self.assertEqual(self.run_check(responses), "exists=false\n")

    def test_unrelated_pr_mention_requires_review_not_duplicate(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 777, "head": {"ref": "unrelated", "repo": {"full_name": "fork/catalog"}},
             "labels": [], "body": "Benchmark comparison: https://github.com/new/name"}]}
        with self.assertRaisesRegex(ValueError, "not an identified intake; maintainer review required"):
            self.run_check(responses)

    def test_intake_pr_mention_is_not_subject_proof(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 778, "head": {"ref": "other"}, "labels": [{"name": "card-draft"}],
             "body": "Benchmark comparison: https://github.com/new/name"}]}
        with self.assertRaisesRegex(ValueError, "subject is unverified"):
            self.run_check(responses)

    def test_multiple_matches_are_ambiguous(self):
        responses = {"repos/org/catalog/pulls?state=open&per_page=100": [
            {"number": 90, "head": {"ref": "a"}, "labels": [{"name": "card-draft"}], "body": "Repository: https://github.com/old/name"},
            {"number": 91, "head": {"ref": "b"}, "labels": [{"name": "card-draft"}], "body": "Repository: https://github.com/old/name"}],
            "repos/old/name": {"id": 42}}
        with self.assertRaisesRegex(ValueError, "multiple open PRs"):
            self.run_check(responses)


if __name__ == "__main__":
    unittest.main()
