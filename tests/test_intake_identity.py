import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


card = load("card_identity", ".github/scripts/intake-card-identity.py")
issue = load("issue_identity", ".github/scripts/intake-issue-identity.py")


class CardIdentityTests(unittest.TestCase):
    def check(self, value, identity=42):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "card.json"
            path.write_text(json.dumps({"card": value}))
            completed = subprocess.CompletedProcess([], 0, stdout=json.dumps({"id": identity}))
            with patch("builtins.open", side_effect=lambda *a, **kw: path.open(*a[1:], **kw)), patch.dict(os.environ, {"REPO_ID": "42"}), patch.object(card.subprocess, "run", return_value=completed):
                card.main()

    def test_alias_with_same_id(self):
        self.check("- [name](https://github.com/old/name) - description")

    def test_uppercase_git_suffix_is_normalized(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "card.json"
            path.write_text(json.dumps({"card": "- [name](HTTPS://GITHUB.COM/Old/Name.GIT) - description"}))
            completed = subprocess.CompletedProcess([], 0, stdout=json.dumps({"id": 42}))
            with patch("builtins.open", side_effect=lambda *a, **kw: path.open(*a[1:], **kw)), patch.dict(os.environ, {"REPO_ID": "42"}), patch.object(card.subprocess, "run", return_value=completed) as run:
                card.main()
            self.assertEqual(run.call_args.args[0], ["gh", "api", "repos/Old/Name"])

    def test_wrong_id(self):
        with self.assertRaisesRegex(ValueError, "does not match"):
            self.check("- [name](https://github.com/old/name) - description", 43)

    def test_multiline_card_rejected(self):
        with self.assertRaisesRegex(ValueError, "single line"):
            self.check("- [name](https://github.com/old/name) - description\n## Injected")

    def test_non_github_subject_rejected(self):
        with self.assertRaisesRegex(ValueError, "GitHub repository"):
            self.check("- [name](https://example.com/old/name) - description")

    def test_github_api_failure_is_not_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "card.json"
            path.write_text(json.dumps({"card": "- [name](https://github.com/old/name) - description"}))
            with patch("builtins.open", side_effect=lambda *a, **kw: path.open(*a[1:], **kw)), patch.object(card.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "gh")):
                with self.assertRaises(subprocess.CalledProcessError):
                    card.main()


class IssueIdentityTests(unittest.TestCase):
    def test_changed_issue_is_rejected(self):
        with patch.dict(os.environ, {"TARGET_REPOSITORY": "org/catalog", "ISSUE_NUMBER": "77", "REPO_ID": "42"}), patch.object(issue.dedup, "issue_id", return_value=43):
            with self.assertRaisesRegex(ValueError, "different repository"):
                issue.main()

    def test_unchanged_issue_passes(self):
        with patch.dict(os.environ, {"TARGET_REPOSITORY": "org/catalog", "ISSUE_NUMBER": "77", "REPO_ID": "42"}), patch.object(issue.dedup, "issue_id", return_value=42):
            issue.main()


if __name__ == "__main__":
    unittest.main()
