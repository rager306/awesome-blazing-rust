#!/usr/bin/env bash
# A/B: does an inlined catalog index in the system prompt change the outcome?
#
# Config A (baseline): the policy file only.
# Config B (treatment): the policy file + llms.txt appended.
#
# Each fixture runs once per configuration; the verdict and the card are kept
# for comparison. This is a measurement, not a gate: run it, write the numbers
# into RESULTS.md, and let the reader decide.
#
# Needs: ANTHROPIC_* env, the abr binary built, jq.
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
ABR="$ROOT/cli/target/release/abr"
HERE="$(cd "$(dirname "$0")" && pwd)"
POLICY="$ROOT/docs/agents/intake.md"
INDEX="$ROOT/llms.txt"
OUT="${1:-/tmp/ab-index-results.md}"

[ -x "$ABR" ] || { echo "build abr first" >&2; exit 2; }

run_one() { # $1 policy text, $2 repo
  local out
  out=$(timeout 420 claude -p "Draft the catalog card for $2. Return ONLY the JSON object described in the policy: {\"skip\": null or reason, \"card\": \"one line\", \"notes\": \"one sentence\"}." \
    --append-system-prompt "$1" \
    --permission-mode dontAsk \
    --allowedTools "Read,Grep,Glob,Bash(gh api *)" \
    --max-turns 20 --output-format json </dev/null 2>/dev/null | jq -r '.result // empty')
  # last flat JSON object with a "card" key
  printf '%s' "$out" | tr '\n' ' ' | grep -oE '\{[^{}]*"card"[^{}]*\}' | tail -1
}

verdict_of() { # $1 json, $2 expected
  local json="$1" expect="$2"
  if ! jq -e . >/dev/null 2>&1 <<<"$json"; then echo INCONCLUSIVE; return; fi
  local skip card
  skip=$(jq -r '.skip // ""' <<<"$json"); card=$(jq -r '.card // ""' <<<"$json")
  if [ "$expect" = "card" ]; then
    [ -n "$card" ] && echo PASS || echo FAIL
  else
    { [ -n "$skip" ] && [ -z "$card" ]; } && echo PASS || echo FAIL
  fi
}

{
  echo "# A/B: catalog index in the prompt"
  echo
  echo "Model: ${ANTHROPIC_MODEL:-default} | one run per cell | $(date -u +%F)"
  echo
  echo "| fixture | expected | baseline | with index |"
  echo "| --- | --- | --- | --- |"
} > "$OUT"

while IFS='|' read -r repo expect; do
  [ -z "$repo" ] && continue
  a_json=$(run_one "$(cat "$POLICY")" "$repo"); a=$(verdict_of "$a_json" "$expect")
  b_json=$(run_one "$(cat "$POLICY" "$INDEX")" "$repo"); b=$(verdict_of "$b_json" "$expect")
  echo "| $repo | $expect | $a | $b |" >> "$OUT"
  echo "$repo: baseline=$a with_index=$b"
done < <(jq -r '.fixtures[] | "\(.repo)|\(.expect)"' "$HERE/fixtures.json")

echo "wrote $OUT"
