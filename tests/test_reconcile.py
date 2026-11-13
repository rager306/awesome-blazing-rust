import importlib.util
import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "intake_reconcile", Path(__file__).resolve().parents[1] / "scripts/intake_reconcile.py"
)
reconcile = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reconcile)


def issue(number):
    return {"number": number, "state": "open", "labels": [{"name": "card-intake"}], "body": None}


def pull(number, branch, state="open"):
    return {"number": number, "state": state, "head": {"ref": branch}, "body": None}


class ReconcileTest(unittest.TestCase):
    def test_inventory_is_always_human_review(self):
        report = reconcile.build_report(
            [issue(2), issue(1), {"pull_request": {}}],
            [pull(12, "agent/card-1-123"), pull(13, "agent/card-1-124", "closed"),
             pull(14, "agent/card-2-123-other")],
        )
        self.assertIn("Issue #1: HUMAN REVIEW REQUIRED; repository ID marker: unknown (missing or ambiguous); PR hints: #12 (open; legacy issue branch), #13 (closed; legacy issue branch)", report)
        self.assertIn("Issue #2: HUMAN REVIEW REQUIRED; repository ID marker: unknown (missing or ambiguous); PR hints: none found", report)
        self.assertIn("not proof", report)
        self.assertIn("unknown dispatch result", report)

    def test_repository_id_hints_and_ambiguous_markers(self):
        first = issue(1)
        first["body"] = "<!-- abr-repo-id: 42 -->"
        second = issue(2)
        second["body"] = "<!-- abr-repo-id: 42 --> <!-- abr-repo-id: 43 -->"
        by_body = pull(11, "custom")
        by_body["body"] = "<!-- abr-repo-id: 42 -->"
        report = reconcile.build_report([first, second], [pull(10, "agent/card-repo-42"), by_body])
        self.assertIn("repository ID marker: 42; PR hints: #10 (open; repository ID branch), #11 (open; body marker)", report)
        self.assertIn("Issue #2: HUMAN REVIEW REQUIRED; repository ID marker: unknown (missing or ambiguous); PR hints: none found", report)
        self.assertIn("unverified hints", report)

    def test_new_pr_body_links_unmarked_issue_as_hint(self):
        candidate = issue(42)
        draft = pull(777, "agent/card-repo-123")
        draft["body"] = "Draft card from #42.\n\n<!-- abr-repo-id: 123 -->\n\nCloses #42"
        report = reconcile.build_report([candidate], [draft])
        self.assertIn("Issue #42: HUMAN REVIEW REQUIRED; repository ID marker: unknown (missing or ambiguous); PR hints: #777 (open; body issue reference)", report)
        self.assertIn("unverified hints", report)

    def test_empty_is_explicit(self):
        self.assertIn("No open intake issues returned", reconcile.build_report([], []))

    def test_rejects_bad_or_inconsistent_records(self):
        for issues, pulls in [
            ([issue(1), issue(1)], []),
            ([{"number": 1, "state": "closed", "labels": []}], []),
            ([issue(True)], []),
            ([issue(1)], [{"number": 1}]),
            ([issue(1)], [pull(2, "x"), pull(2, "y")]),
        ]:
            with self.subTest(issues=issues, pulls=pulls), self.assertRaises(ValueError):
                reconcile.build_report(issues, pulls)

    def test_page_shape_and_command_failure(self):
        with patch.object(reconcile.subprocess, "run") as run:
            run.return_value.stdout = json.dumps([[issue(1)], [issue(2)]])
            self.assertEqual(reconcile.pages("endpoint"), [issue(1), issue(2)])
            for malformed in ([], {}, [[issue(1)], {}]):
                run.return_value.stdout = json.dumps(malformed)
                with self.assertRaises(ValueError):
                    reconcile.pages("endpoint")
            run.side_effect = subprocess.CalledProcessError(1, "gh")
            with self.assertRaises(subprocess.CalledProcessError):
                reconcile.pages("endpoint")


if __name__ == "__main__":
    unittest.main()
