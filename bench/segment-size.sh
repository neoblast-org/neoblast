#!/usr/bin/env bash
# Segment-size benchmark for ROADMAP open item 2 (see bench/README.md).
#
#   bench/segment-size.sh [file_mib] [segment sizes in MiB...]
#   bench/segment-size.sh                 # 1024 MiB file; 1, 4, 8 and 16 MiB segments
#   bench/segment-size.sh 256 1 4         # quicker run
#
# For each segment size it seals a random file, then measures wall time and
# peak memory for: seal, open with every shard present, heal after losing
# n - k shards per segment, and open after losing all k data shards (the
# worst case, decoding everything from parity). Each operation is also run on
# a 1 KB file; "working" memory is the difference, the part that grows with the
# segment size. The result goes to
# bench/results/<UTC time>-segment-size.json with no host names, user names
# or paths. Works with the bash 3.2 that ships with macOS.
# Python runs isolated (-I), so a stray .py file in the working directory
# cannot stand in for a module it imports (review S-021).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
file_mib="${1:-1024}"
shift || true
sizes="${*:-1 4 8 16}"
k=20
n=50

case "$(uname -s)" in
  Darwin) time_flag=-l ;;
  Linux) time_flag=-v ;;
  *) echo "unsupported OS" >&2; exit 1 ;;
esac
if [ ! -x /usr/bin/time ]; then
  echo "needs /usr/bin/time (on Debian or Ubuntu: apt install time)" >&2
  exit 1
fi

started_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "building the release binary"
cargo build --release --locked -p neoblast --manifest-path "$root/Cargo.toml" >/dev/null
# Honour CARGO_TARGET_DIR and build.target-dir instead of assuming target/.
target_dir=$(cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" |
  python3 -I -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
bin="$target_dir/release/neoblast"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
trap 'exit 130' INT TERM
echo "writing a ${file_mib} MiB random input"
head -c "$((file_mib * 1024 * 1024))" /dev/urandom >"$work/in.bin"
head -c 1000 /dev/urandom >"$work/tiny.bin"
results="$work/results.tsv"
: >"$results"

# Runs a command under /usr/bin/time and appends "<label> <seconds> <peak bytes>".
measure() { # segment_mib label command...
  local seg="$1" label="$2" start end
  shift 2
  start=$(python3 -I -c 'import time; print(time.time())')
  /usr/bin/time "$time_flag" "$@" 2>"$work/time.txt" >"$work/stdout.txt" || {
    cat "$work/time.txt" >&2
    exit 1
  }
  end=$(python3 -I -c 'import time; print(time.time())')
  python3 -I - "$seg" "$label" "$start" "$end" "$work/time.txt" >>"$results" <<'PY'
import re, sys
seg, label, start, end, path = sys.argv[1:]
text = open(path).read()
m = re.search(r"(\d+)\s+maximum resident set size", text)  # macOS: bytes
if m:
    peak = int(m.group(1))
else:
    m = re.search(r"Maximum resident set size \(kbytes\): (\d+)", text)  # GNU time
    peak = int(m.group(1)) * 1024
# Wall time as measured by time(1) itself; fall back to our own clock.
m = re.search(r"([\d.]+) real", text)  # macOS
g = re.search(r"Elapsed \(wall clock\) time.*: (?:(\d+):)?(\d+):([\d.]+)", text)  # GNU
if m:
    seconds = float(m.group(1))
elif g:
    seconds = int(g.group(1) or 0) * 3600 + int(g.group(2)) * 60 + float(g.group(3))
else:
    seconds = float(end) - float(start)
print(seg, label, round(seconds, 3), peak, sep="\t")
PY
}

drop_shards() { # store first last: delete shard indices first..last in every segment
  python3 -I - "$@" <<'PY'
import pathlib, sys
store, first, last = pathlib.Path(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
for seg in store.glob("seg-*"):
    for i in range(first, last + 1):
        (seg / f"{i:03d}.shard").unlink(missing_ok=True)
PY
}

# Seal, open, lose n - k shards and heal, lose the k data shards and open again.
run_ops() { # segment_mib input label_prefix
  local seg="$1" input="$2" prefix="$3" bytes store link verify
  bytes=$((seg * 1024 * 1024))
  store="$work/store"
  rm -rf "$store"
  measure "$seg" "${prefix}seal" "$bin" seal "$input" --out "$store" \
    --k "$k" --n "$n" --segment-size "$bytes"
  link=$(cat "$work/stdout.txt")
  verify=$(printf '%s' "$link" | sed 's|^nbl://f/|nbl://fv/|; s|#.*||')
  measure "$seg" "${prefix}open" "$bin" open "$link" --from "$store" --out "$work/out.bin"
  cmp -s "$input" "$work/out.bin" || { echo "open returned different bytes" >&2; exit 1; }
  rm -f "$work/out.bin"
  drop_shards "$store" 0 $((n - k - 1))
  measure "$seg" "${prefix}heal" "$bin" heal "$verify" --from "$store"
  drop_shards "$store" 0 $((k - 1))
  measure "$seg" "${prefix}open_from_parity" "$bin" open "$link" --from "$store" --out "$work/out.bin"
  cmp -s "$input" "$work/out.bin" || { echo "open returned different bytes" >&2; exit 1; }
  rm -rf "$store" "$work/out.bin"
}

for s in $sizes; do
  echo "segment size ${s} MiB"
  # The same operations on a 1 KB file measure the fixed cost (binary and codec
  # tables), so "working" memory is what grows with the segment size.
  run_ops "$s" "$work/tiny.bin" "baseline_"
  run_ops "$s" "$work/in.bin" ""
done

mkdir -p "$here/results"
python3 -I - "$results" "$here/results" "$file_mib" "$k" "$n" "$(git -C "$root" rev-parse HEAD)" \
  "$(git -C "$root" status --porcelain -- crates Cargo.toml Cargo.lock | wc -l | tr -d ' ')" \
  "$started_utc" <<'PY'
import json, pathlib, platform, subprocess, sys
tsv, out_dir, file_mib, k, n, commit, dirty, started = sys.argv[1:]
mib = 1024 * 1024
raw = {}
for line in open(tsv):
    seg, label, seconds, peak = line.split("\t")
    raw.setdefault(seg, {})[label] = (float(seconds), int(peak))
runs = {}
for seg, r in raw.items():
    runs[seg] = {}
    for label, (seconds, peak) in r.items():
        v = {"seconds": seconds, "peak_mib": round(peak / mib, 1)}
        if not label.startswith("baseline_"):
            # Computed from the unrounded peaks.
            v["working_mib"] = round((peak - r["baseline_" + label][1]) / mib, 1)
        runs[seg][label] = v
    done = runs[seg]
    runs[seg] = {
        "fixed": {label[len("baseline_"):]: v for label, v in done.items() if label.startswith("baseline_")},
        **{label: v for label, v in done.items() if not label.startswith("baseline_")},
    }
cpu = ""
if platform.system() == "Darwin":
    cpu = subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip()
record = {
    "benchmark": "segment-size",
    "started_utc": started,
    "commit": commit,
    "uncommitted_changes": int(dirty) > 0,
    "file_mib": int(file_mib),
    "k": int(k),
    "n": int(n),
    "machine": {"os": platform.system().lower(), "arch": platform.machine(), "cpu": cpu},
    "segment_mib": runs,
}
out = pathlib.Path(out_dir) / (started.replace(":", "").replace("-", "") + "-segment-size.json")
out.write_text(json.dumps(record, indent=2) + "\n")

print()
print(f"{'segment':>8} {'operation':>17} {'seconds':>8} {'peak MiB':>9} {'working MiB':>12}")
for seg, r in runs.items():
    for label, v in r.items():
        if label != "fixed":
            print(f"{seg + ' MiB':>8} {label:>17} {v['seconds']:8.2f} {v['peak_mib']:9.1f} {v['working_mib']:12.1f}")
print(f"\nwrote bench/results/{out.name}")
PY
