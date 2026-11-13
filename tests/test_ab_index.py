import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "runs/agent-evals/ab-index.sh"


class AbIndexTest(unittest.TestCase):
    def test_offline_scoring_and_artifacts(self):
        with tempfile.TemporaryDirectory() as temp:
            tmp = Path(temp)
            fake = tmp / "claude"
            fake.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
log = pathlib.Path(os.environ["FAKE_LOG"])
calls = json.loads(log.read_text()) if log.exists() else []
calls.append({"cwd": os.getcwd(), "args": sys.argv[1:]})
log.write_text(json.dumps(calls))
responses = json.loads(os.environ["FAKE_RESPONSES"])
response = responses[len(calls) - 1]
print(json.dumps(response))
print("credential: " + os.environ["TEST_API_KEY"], file=sys.stderr)
''')
            fake.chmod(0o755)
            def envelope(result):
                return {"result": json.dumps(result)}

            responses = [
                envelope({"card": "a card", "skip": None, "notes": "ok"}),
                envelope({"card": "a card", "skip": "reason", "notes": "ok"}),
                envelope({"card": "  ", "skip": None, "notes": "ok"}),
                envelope({"card": "a card", "notes": "ok"}),
                envelope({"card": "a card", "skip": None}),
                {"result": 'prose {"card":"a card","skip":null,"notes":"ok"}'},
                envelope({"card": "", "skip": "reason", "notes": "ok"}),
                envelope({"card": " ", "skip": "reason", "notes": "ok"}),
                envelope({"card": "card", "skip": "reason", "notes": "ok"}),
                {"is_error": True, "result": "{}"},
            ]
            env = dict(os.environ, PATH=f"{tmp}{os.pathsep}{os.environ['PATH']}",
                       FAKE_LOG=str(tmp / "calls.json"),
                       FAKE_RESPONSES=json.dumps(responses), TEST_API_KEY="test-secret-12345")
            proc = subprocess.run([str(SCRIPT), str(tmp)], cwd=tmp, env=env,
                                  capture_output=True, text=True, check=True)
            output = Path(next(line.split(": ", 1)[1] for line in proc.stdout.splitlines()
                               if line.startswith("Output directory: ")))
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o700)
            cells = sorted(output.glob("*-*/evidence.json"))
            evidence = [json.loads(path.read_text()) for path in cells]
            self.assertEqual([item["verdict"] for item in evidence], [
                "PASS", "FAIL", "FAIL", "INCONCLUSIVE", "INCONCLUSIVE",
                "INCONCLUSIVE", "PASS", "PASS", "FAIL", "INCONCLUSIVE",
            ])
            calls = json.loads((tmp / "calls.json").read_text())
            self.assertEqual(len(calls), 10)
            self.assertTrue(all(call["cwd"] == str(ROOT) for call in calls))
            policy = (ROOT / "docs/agents/intake.md").read_text()
            index = (ROOT / "llms.txt").read_text()
            for n, call in enumerate(calls):
                args = call["args"]
                prompt = args[args.index("--append-system-prompt") + 1]
                self.assertEqual(prompt, policy if n % 2 == 0 else policy + "\n" + index)
            self.assertNotIn("test-secret-12345", "".join(
                path.read_text() for path in output.rglob("*") if path.is_file()))
            self.assertEqual(len(list(output.glob("*-*/stdout.json"))), 10)
            self.assertTrue((output / "report.md").is_file())


if __name__ == "__main__":
    unittest.main()
