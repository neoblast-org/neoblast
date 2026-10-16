#!/usr/bin/env bash
# Measures heal's peak memory at several file and segment sizes, to see whether
# it grows with the file. Diagnostic for open item 2; see bench/README.md.
#
#   bench/heal-check.sh                  # 256 and 1024 MiB files; 1 and 4 MiB segments
#   bench/heal-check.sh "256 1024" "1 4"
#
# Each run seals a random file at 20-of-50, deletes n - k = 30 shards in every
# segment, and heals. Prints peak RSS and, on macOS, peak memory footprint.
# Works with the bash 3.2 that ships with macOS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
files="${1:-256 1024}"
segments="${2:-1 4}"
k=20
n=50
case "$(uname -s)" in
  Darwin) flag=-l ;;
  Linux) flag=-v ;;
  *) echo "unsupported OS" >&2; exit 1 ;;
esac
if [ ! -x /usr/bin/time ]; then
  echo "needs /usr/bin/time (on Debian or Ubuntu: apt install time)" >&2
  exit 1
fi

cargo build --release --locked -p neoblast --manifest-path "$root/Cargo.toml" >/dev/null
target_dir=$(cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
bin="$target_dir/release/neoblast"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
trap 'exit 130' INT TERM

printf '%8s %8s %10s %14s\n' "file" "segment" "RSS MiB" "footprint MiB"
for f in 0 $files; do
  if [ "$f" = 0 ]; then head -c 1000 /dev/urandom >"$work/in.bin"; label="1 KB"
  else head -c "$((f * 1024 * 1024))" /dev/urandom >"$work/in.bin"; label="$f MiB"; fi
  for s in $segments; do
    rm -rf "$work/s"
    link=$("$bin" seal "$work/in.bin" --out "$work/s" --k "$k" --n "$n" --segment-size "$((s * 1024 * 1024))" 2>"$work/t.txt") || {
      echo "seal failed ($label, $s MiB segments):" >&2
      cat "$work/t.txt" >&2
      exit 1
    }
    verify=$(printf '%s' "$link" | sed 's|^nbl://f/|nbl://fv/|; s|#.*||')
    python3 - "$work/s" "$((n - k))" <<'PY'
import pathlib, sys
for seg in pathlib.Path(sys.argv[1]).glob("seg-*"):
    for i in range(int(sys.argv[2])):
        (seg / f"{i:03d}.shard").unlink(missing_ok=True)
PY
    /usr/bin/time "$flag" "$bin" heal "$verify" --from "$work/s" 2>"$work/t.txt" >/dev/null || {
      echo "heal failed ($label, $s MiB segments):" >&2
      cat "$work/t.txt" >&2
      exit 1
    }
    python3 - "$work/t.txt" "$label" "$s" <<'PY'
import re, sys
text, label, seg = open(sys.argv[1]).read(), sys.argv[2], sys.argv[3]
mib = 1024 * 1024
m = re.search(r"(\d+)\s+maximum resident set size", text)
rss = int(m.group(1)) / mib if m else int(re.search(r"Maximum resident set size \(kbytes\): (\d+)", text).group(1)) / 1024
f = re.search(r"(\d+)\s+peak memory footprint", text)
foot = f"{int(f.group(1)) / mib:14.1f}" if f else f"{'-':>14}"
print(f"{label:>8} {seg + ' MiB':>8} {rss:10.1f} {foot}")
PY
  done
done
