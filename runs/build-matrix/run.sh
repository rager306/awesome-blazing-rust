#!/usr/bin/env bash
# Build-artifact matrix for one crate: separates the debuginfo and
# incremental factors instead of bundling them. Usage: run.sh <crate-dir>
# Prints one TSV line per config: config, target_KiB, binary_bytes, wall_s.
set -euo pipefail
crate="${1:?usage: run.sh <crate-dir>}"
cd "$crate"
rustc -Vv | grep -m1 "^rustc "
run_one() {
  local name="$1" debug="$2" incr="$3" strip="${4:-none}"
  local dir; dir="$(mktemp -d)"
  local t0=$SECONDS
  CARGO_TARGET_DIR="$dir" \
  CARGO_PROFILE_DEV_DEBUG="$debug" \
  CARGO_PROFILE_DEV_INCREMENTAL="$incr" \
  CARGO_PROFILE_DEV_STRIP="$strip" \
    cargo build >/dev/null 2>&1
  local wall=$((SECONDS - t0))
  local bin; bin="$(find "$dir/debug" -maxdepth 1 -type f -executable | head -1)"
  printf '%s\t%s\t%s\t%s\n' "$name" \
    "$(du -sk "$dir" | cut -f1) KiB" \
    "$(stat -c %s "$bin") B" \
    "${wall}s"
  rm -rf "$dir"
}
run_one A_full_incr          2    true  none
run_one B_full_noincr        2    false none
run_one C_debug0_incr        0    true  none
run_one D_debug0_noincr      0    false none
run_one E_linetables_incr    line-tables-only true none
run_one F_linetables_strip   line-tables-only true debuginfo
