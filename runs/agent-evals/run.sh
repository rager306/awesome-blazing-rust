#!/usr/bin/env bash
# Agent eval harness: run the intake researcher on fixed fixtures and score it.
#
# Outcomes: PASS (all checks), FAIL (a check failed), INCONCLUSIVE (the agent
# produced no parsable output - environment flakiness, not a quality signal).
# See SCENARIO.md. Run from anywhere inside the repository:
#   runs/agent-evals/run.sh            # all fixtures
#   runs/agent-evals/run.sh ripgrep    # one, by substring
set -uo pipefail

ROOT=$(git rev-parse --show-toplevel)
ABR="$ROOT/cli/target/release/abr"
HERE="$(cd "$(dirname "$0")" && pwd)"
FILTER="${1:-}"
MODEL="${ANTHROPIC_MODEL:-glm-5.3-flash}"
pass=0; fail=0; inconclusive=0

[ -x "$ABR" ] || { echo "build first: (cd cli && cargo build --release)" >&2; exit 2; }

url_ok() {
  local code
  code=$(curl -sL -o /dev/null -m 20 -w '%{http_code}' "$1" || echo 000)
  [ "${code:0:1}" = "2" ] || [ "${code:0:1}" = "3" ]
}

scratch_check() { # $1 = card line
  local tmp
  tmp=$(mktemp -d)
  mkdir -p "$tmp/catalog"
  cp "$ROOT/README.md" "$ROOT/PATTERNS.md" "$tmp/"
  cp "$ROOT/catalog/projects.json" "$tmp/catalog/"
  awk -v card="$1" '
    /^## See also/ && !done { print card; print ""; done=1 }
    { print }
  ' "$tmp/README.md" > "$tmp/README.new" && mv "$tmp/README.new" "$tmp/README.md"
  # Mirror what the intake pipeline does: regenerate every projection, then
  # check. A new Arch tag is legitimate for a new project - the pipeline indexes
  # it - so the scratch check must not reject it.
  ( cd "$tmp" && "$ABR" catalog >/dev/null 2>&1       && "$ABR" patterns >/dev/null 2>&1       && "$ABR" llms >/dev/null 2>&1       && "$ABR" check 2>&1 )
  local rc=$?
  rm -rf "$tmp"
  return $rc
}

for fixture in $(jq -r --arg f "$FILTER" '.fixtures[] | select($f == "" or (.repo | contains($f))) | "\(.repo)|\(.expect)"' "$HERE/fixtures.json"); do
  repo="${fixture%%|*}"; expect="${fixture##*|}"
  echo "=== $repo (expect: $expect)"

  prompt="Draft the catalog card for ${repo}. Return ONLY the JSON object described in the policy: {\"skip\": null or reason, \"card\": \"one line\", \"notes\": \"one sentence\"}."

  out=$(timeout 420 claude -p "$prompt" \
    --append-system-prompt "$(cat "$ROOT/docs/agents/intake.md")" \
    --permission-mode dontAsk \
    --allowedTools "Read,Grep,Glob,Bash(gh api *)" \
    --max-turns 20 --output-format json 2>/dev/null | jq -r '.result // empty')

  # The agent may wrap the object in prose or a fence: keep the last flat
  # object that carries a "card" key. The object has no nested braces, which is
  # what makes the cheap extractor safe.
  json=$(tr '\n' ' ' <<< "$out" | grep -oE '\{[^{}]*"card"[^{}]*\}' | tail -1)
  if ! jq -e . >/dev/null 2>&1 <<< "$json"; then
    echo "  INCONCLUSIVE: no parsable JSON in the agent output"
    inconclusive=$((inconclusive+1)); continue
  fi

  skip=$(jq -r '.skip // ""' <<< "$json")
  card=$(jq -r '.card // ""' <<< "$json")

  if [ "$expect" = "skip" ]; then
    if [ -n "$skip" ] && [ -z "$card" ]; then echo "  PASS: refused ($skip)"; pass=$((pass+1));
    else echo "  FAIL: should have been refused, got a card"; fail=$((fail+1)); fi
    continue
  fi

  if [ -n "$skip" ] || [ -z "$card" ]; then
    echo "  FAIL: expected a card, got skip='$skip'"; fail=$((fail+1)); continue
  fi

  bad=0
  for marker in '**Blazing:**' '**Type:**' '**Arch:**'; do
    grep -qF "$marker" <<< "$card" || { echo "  FAIL: missing $marker"; bad=1; }
  done
  if grep -qF '**Type:** measured' <<< "$card" && ! grep -qF '**Measured by:**' <<< "$card"; then
    echo "  FAIL: measured without **Measured by:**"; bad=1
  fi
  while read -r url; do
    [ -z "$url" ] && continue
    url_ok "$url" || { echo "  FAIL: link does not resolve: $url"; bad=1; }
  done < <(grep -oE 'https?://[^) ]+' <<< "$card" | sort -u)

  diag=$(scratch_check "$card"); rc=$?
  if [ "$bad" = 0 ] && [ "$rc" = 0 ]; then
    echo "  PASS"; pass=$((pass+1))
  else
    [ "$bad" = 0 ] && { echo "  FAIL: abr check rejected the card:"; sed 's/^/    /' <<< "$diag" | head -5; }
    fail=$((fail+1))
  fi
done

echo
echo "pass=$pass fail=$fail inconclusive=$inconclusive"
[ "$fail" = 0 ] || exit 1
