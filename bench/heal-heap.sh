#!/usr/bin/env bash
# Measures heal's peak live heap exactly, at several file and segment sizes, to
# show that it grows with the file only by the manifest. See bench/README.md.
#
#   bench/heal-heap.sh                   # 256 and 1024 MiB files; 1 and 4 MiB segments
#   bench/heal-heap.sh "256 1024" "1 4"
#
# Each file is sealed at 20-of-50, then healed with n - k = 30 shards of every
# segment treated as missing (bench/heal-heap/ counts the heap with dhat and
# never changes the store). For each segment size, the growth from the
# smallest file to each larger one must not exceed the manifest's own growth
# by more than 64 KiB; it needs at least two file sizes. Works with the bash
# 3.2 that ships with macOS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
files="${1:-256 1024}"
segments="${2:-1 4}"
k=20
n=50

cargo build --release --locked -p neoblast --manifest-path "$root/Cargo.toml" >/dev/null
cargo build --release --locked --manifest-path "$here/heal-heap/Cargo.toml" >/dev/null
target_dir=$(cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
heap_dir=$(cargo metadata --format-version 1 --no-deps --manifest-path "$here/heal-heap/Cargo.toml" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
bin="$target_dir/release/neoblast"
heap="$heap_dir/release/neoblast-heal-heap"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
trap 'exit 130' INT TERM

: >"$work/results.txt"
for f in $files; do
  head -c "$((f * 1024 * 1024))" /dev/urandom >"$work/in.bin"
  for s in $segments; do
    rm -rf "$work/s"
    link=$("$bin" seal "$work/in.bin" --out "$work/s" --k "$k" --n "$n" --segment-size "$((s * 1024 * 1024))" 2>"$work/t.txt") || {
      echo "seal failed ($f MiB, $s MiB segments):" >&2
      cat "$work/t.txt" >&2
      exit 1
    }
    verify=$(printf '%s' "$link" | sed 's|^nbl://f/|nbl://fv/|; s|#.*||')
    out=$("$heap" "$verify" "$work/s" "$((n - k))" 2>"$work/t.txt") || {
      echo "heal-heap failed ($f MiB, $s MiB segments):" >&2
      cat "$work/t.txt" >&2
      exit 1
    }
    set -- "$work/s"/seg-*
    printf '%s %s %s %s\n' "$f" "$s" "$(($# * (n - k)))" "$out" >>"$work/results.txt"
  done
done

python3 - "$work/results.txt" <<'PY'
import re, sys
mib = 1024 * 1024
rows = []
for line in open(sys.argv[1]):
    f, s, want, rest = line.split(maxsplit=3)
    rebuilt = int(re.search(r"rebuilt=(\d+)", rest).group(1))
    if rebuilt != int(want):
        sys.exit(f"{f} MiB, {s} MiB segments: rebuilt {rebuilt} shards, expected {want}")
    peak = int(re.search(r"peak_heap=(\d+)", rest).group(1))
    manifest = int(re.search(r"manifest=(\d+)", rest).group(1))
    rows.append((int(f), int(s), peak, manifest))
for s in {r[1] for r in rows}:
    if len({r[0] for r in rows if r[1] == s}) < 2:
        sys.exit(f"{s} MiB segments: give at least two file sizes to compare")
print(f"{'file':>8} {'segment':>8} {'heap MiB':>10} {'manifest MiB':>13} {'vs smallest file':>18}")
failed = False
for f, s, peak, manifest in rows:
    base = min((r for r in rows if r[1] == s), key=lambda r: r[0])
    if (f, s) == base[:2]:
        verdict = "baseline"
    else:
        # Growth beyond the manifest's own growth.
        extra = (peak - base[2]) - (manifest - base[3])
        ok = extra <= 64 * 1024
        failed |= not ok
        verdict = f"{'ok' if ok else 'GROWS'} ({extra:+d} B)"
    print(f"{f:>4} MiB {s:>4} MiB {peak / mib:10.2f} {manifest / mib:13.2f} {verdict:>18}")
sys.exit(1 if failed else 0)
PY
