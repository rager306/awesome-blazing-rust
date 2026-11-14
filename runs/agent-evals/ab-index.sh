#!/usr/bin/env bash
# Compare the intake prompt with and without the catalog index. See SCENARIO.md.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$HERE" rev-parse --show-toplevel)
exec python3 - "$ROOT" "$HERE" "${1:-/tmp}" <<'PY'
import datetime
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

root, here, output_parent = map(Path, sys.argv[1:])
fixtures = json.loads((here / "fixtures.json").read_text())["fixtures"]
policy = (root / "docs/agents/intake.md").read_text()
index = (root / "llms.txt").read_text()
# Never persist environment variables. Remove known credential values from all
# captured text before writing it; this cannot identify every possible secret.
secrets = sorted((value for key, value in os.environ.items()
                  if re.search(r"TOKEN|SECRET|PASSWORD|PASSWD|API_KEY|AUTH|CREDENTIAL", key, re.I)
                  and len(value) >= 4), key=len, reverse=True)
def redact(text):
    for secret in secrets:
        text = text.replace(secret, "[REDACTED]")
    return re.sub(r"\b(?:sk-ant-[\w-]+|sk-[\w-]{20,}|gh[pousr]_[\w]+|github_pat_[\w]+)\b",
                  "[REDACTED]", text)

def parse_result(stdout):
    try:
        envelope = json.loads(stdout)
        if envelope.get("is_error") is True:
            return None, "CLI response reports a model error"
        result = envelope.get("result")
        if not isinstance(result, str):
            return None, "CLI response has no text result"
        try:
            obj = json.loads(result)
        except json.JSONDecodeError:
            return None, "Model result is not a JSON object"
        if not isinstance(obj, dict):
            return None, "Model result is not a JSON object"
        return obj, None
    except (json.JSONDecodeError, AttributeError):
        return None, "CLI response is not a JSON object"

def run_cell(repo, expect, prompt, cell):
    cell.mkdir()
    command = ["claude", "-p", f'Draft the catalog card for {repo}. Return ONLY the JSON object described in the policy: {{"skip": null or reason, "card": "one line", "notes": "one sentence"}}.',
               "--append-system-prompt", prompt, "--permission-mode", "dontAsk",
               "--allowedTools", "Read,Grep,Glob,Bash(gh api *)", "--max-turns", "20", "--output-format", "json"]
    try:
        proc = subprocess.run(command, cwd=root, stdin=subprocess.DEVNULL, capture_output=True, timeout=420)
        stdout = proc.stdout.decode("utf-8", errors="replace")
        stderr = proc.stderr.decode("utf-8", errors="replace")
        status, timed_out = proc.returncode, False
    except subprocess.TimeoutExpired as exc:
        stdout = (exc.stdout or b"").decode("utf-8", errors="replace")
        stderr = (exc.stderr or b"").decode("utf-8", errors="replace")
        status, timed_out = None, True
    except OSError as exc:
        stdout, stderr, status, timed_out = "", str(exc), None, False
    result, error = parse_result(stdout)
    (cell / "stdout.json").write_text(redact(stdout))
    (cell / "stderr.txt").write_text(redact(stderr))
    if timed_out:
        verdict, reason = "INCONCLUSIVE", "Process exceeded 420 seconds"
    elif status != 0:
        verdict, reason = "INCONCLUSIVE", f"Process exit status: {status}"
    elif error:
        verdict, reason = "INCONCLUSIVE", error
    elif (not isinstance(result.get("card"), str) or "skip" not in result
          or not (result["skip"] is None or isinstance(result["skip"], str))
          or not isinstance(result.get("notes"), str)):
        verdict, reason = "INCONCLUSIVE", "Output card, skip or notes is missing or has an invalid type"
    else:
        card, skip = result["card"].strip(), (result["skip"] or "").strip()
        passed = bool(card) and not skip if expect == "card" else bool(skip) and not card
        verdict = "PASS" if passed else "FAIL"
        reason = f"expected {expect}; card {'nonempty' if card else 'empty'}; skip {'nonempty' if skip else 'empty'}"
    evidence = {"repo": repo, "expected": expect, "verdict": verdict, "reason": reason,
                "exit_status": status, "timed_out": timed_out, "parsed_result": result}
    def redact_value(value):
        if isinstance(value, str):
            return redact(value)
        if isinstance(value, list):
            return [redact_value(item) for item in value]
        if isinstance(value, dict):
            return {redact(str(key)): redact_value(item) for key, item in value.items()}
        return value
    (cell / "evidence.json").write_text(json.dumps(redact_value(evidence), indent=2, ensure_ascii=False) + "\n")
    return verdict

# mkdtemp prevents accidental replacement of previous runs and uses mode 0700.
output = Path(tempfile.mkdtemp(prefix="ab-index-", dir=output_parent))
print(f"Output directory: {output}", flush=True)
requested_model = redact(os.environ.get("ANTHROPIC_MODEL", "default"))
rows = ["# A/B: catalog index in the prompt", "",
        f"Requested model: {requested_model} | one run per cell | {datetime.datetime.now(datetime.timezone.utc).date()}", "",
        "| fixture | expected | baseline | with index |", "| --- | --- | --- | --- |"]
for number, fixture in enumerate(fixtures, 1):
    repo, expect = fixture["repo"], fixture["expect"]
    if expect not in ("card", "skip"):
        raise ValueError(f"Invalid expectation for fixture {number}")
    cells = []
    for name, prompt in (("baseline", policy), ("with-index", policy + "\n" + index)):
        cell = output / f"{number:02d}-{name}"
        verdict = run_cell(repo, expect, prompt, cell)
        cells.append(f"[{verdict}]({cell.name}/evidence.json)")
    rows.append(f"| {redact(repo)} | {expect} | {cells[0]} | {cells[1]} |")
    print(f"{redact(repo)}: baseline={cells[0]} with_index={cells[1]}", flush=True)
(output / "report.md").write_text("\n".join(rows) + "\n")
print(f"Report: {output / 'report.md'}")
PY
