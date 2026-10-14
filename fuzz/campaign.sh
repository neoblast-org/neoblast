#!/usr/bin/env bash
# Run fuzz campaigns and record each run (see fuzz/README.md).
#
#   fuzz/campaign.sh <target|all> <hours>    run, then write one record per target
#   fuzz/campaign.sh summary                 hours per target on the current code
#
# New records go to fuzz/runs/ (not tracked by git); copy them into
# fuzz/campaigns/ in a development clone to commit them. Runs are recorded
# against committed code, so the tree must be clean. Records hold no host
# names, user names or paths. Works with the bash 3.2 that ships with macOS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
cd "$here"

all_targets() {
  sed -n 's/^name = "\(.*\)"$/\1/p' Cargo.toml | grep -v '^neoblast-fuzz$'
}

# Everything that decides what a fuzz binary contains: library sources, both
# manifests and both lock files, and the target's own source. Hours only add up
# across runs with the same code_id.
code_id() { # target
  local p
  for p in crates Cargo.toml Cargo.lock fuzz/Cargo.toml fuzz/Cargo.lock "fuzz/fuzz_targets/$1.rs"; do
    git -C "$root" rev-parse "HEAD:$p"
  done | shasum -a 256 | cut -c1-16
}

summary() {
  local ids="" t
  for t in $(all_targets); do ids="$ids $t=$(code_id "$t")"; done
  # shellcheck disable=SC2086
  python3 - "$here" $ids <<'PY'
import json, pathlib, sys
here = pathlib.Path(sys.argv[1])
current = dict(arg.split("=", 1) for arg in sys.argv[2:])
files = {}
for folder in ("campaigns", "runs"):  # committed records, then this machine's new ones
    for p in sorted((here / folder).glob("*.json")):
        files.setdefault(p.name, p)
runs = [json.loads(p.read_text()) for p in files.values()]
counted = ("clean", "stopped")
print(f"{'target':22} {'h, current code':>16} {'h, all':>7} {'crashes':>8}  gate (24 h, no crash)")
for target, cid in current.items():
    mine = [r for r in runs if r["target"] == target]
    now = [r for r in mine if r["code_id"] == cid]
    hours_now = sum(r["seconds_run"] for r in now if r["result"] in counted) / 3600
    hours_all = sum(r["seconds_run"] for r in mine if r["result"] in counted) / 3600
    crashes = sum(1 for r in mine if r["result"] == "crash")
    met = hours_now >= 24 and not any(r["result"] in ("crash", "error") for r in now)
    print(f"{target:22} {hours_now:16.1f} {hours_all:7.1f} {crashes:8d}  {'met' if met else '-'}")
PY
}

# Writes fuzz/runs/<start>-<target>.json for one finished run.
record() { # target started_utc start_epoch exit_code log artifacts_before planned code_id commit
  python3 - "$@" <<'PY'
import datetime, json, os, pathlib, platform, re, subprocess, sys
t, started, start_epoch, rc, log, before, planned, cid, commit = sys.argv[1:]
text = pathlib.Path(log).read_text(errors="replace")

def run(*cmd):
    return subprocess.run(list(cmd), capture_output=True, text=True).stdout.strip()

def stat(name):
    m = re.search(rf"stat::{name}:\s+(\d+)", text)
    return int(m.group(1)) if m else None

# The fuzzer writes its log until it exits, so the log's modification time is
# when this target stopped, even if other targets ran longer.
ended = os.path.getmtime(log)
progress = re.findall(r"cov: (\d+) ft: (\d+) corp: (\d+)/", text)
cov, ft, corp = (int(x) for x in progress[-1]) if progress else (None, None, None)
new = sorted(set(p.name for p in pathlib.Path("artifacts", t).iterdir()) - set(before.split()))
rc = int(rc)
crashed = bool(new) or re.search(r"==\d+== ERROR: libFuzzer|deadly signal|panicked at", text)
# libFuzzer exits 0 when its time is up and 72 when interrupted. 143 means
# SIGTERM arrived before libFuzzer had installed its handler.
if crashed:
    result = "crash"
elif rc == 0:
    result = "clean"
elif rc in (72, 143):
    result = "stopped"
else:
    result = "error"
cpu = run("sysctl", "-n", "machdep.cpu.brand_string") if platform.system() == "Darwin" else ""
record = {
    "target": t,
    "result": result,
    "exit_code": rc,
    "started_utc": started,
    "ended_utc": datetime.datetime.fromtimestamp(ended, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "seconds_planned": int(planned),
    "seconds_run": max(0, int(ended - int(start_epoch))),
    "code_id": cid,
    "commit": commit,
    "executions": stat("number_of_executed_units"),
    "execs_per_sec": stat("average_exec_per_sec"),
    "peak_rss_mb": stat("peak_rss_mb"),
    "coverage": {"cov": cov, "ft": ft},
    "corpus_entries": corp,
    "new_artifacts": new,
    "toolchain": run("rustc", "+nightly", "-V"),
    "machine": {"os": platform.system().lower(), "arch": platform.machine(), "cpu": cpu},
}
out = pathlib.Path("runs") / (started.replace(":", "").replace("-", "") + f"-{t}.json")
out.write_text(json.dumps(record, indent=2) + "\n")
print(f"{t}: {result}, {record['seconds_run'] / 3600:.2f} h, {record['executions']} execs -> fuzz/{out}")
PY
}

if [ "${1:-}" = "summary" ]; then
  summary
  exit 0
fi
if [ $# -ne 2 ]; then
  sed -n '2,10p' "$here/campaign.sh"
  exit 1
fi
hours="$2"
seconds=$(python3 -c "import sys; print(int(float(sys.argv[1]) * 3600))" "$hours")
if [ "$seconds" -lt 60 ]; then
  # libFuzzer treats -max_total_time=0 as "run forever".
  echo "hours must be at least 1/60 (one minute)" >&2
  exit 1
fi
if [ "$1" = "all" ]; then targets=$(all_targets); else targets="$1"; fi

dirty=$(git -C "$root" status --porcelain -- crates fuzz Cargo.toml Cargo.lock)
if [ -n "$dirty" ]; then
  echo "uncommitted changes in crates/, fuzz/, Cargo.toml or Cargo.lock; commit or stash them first:" >&2
  echo "$dirty" >&2
  exit 1
fi
commit=$(git -C "$root" rev-parse HEAD)

# Keep the machine awake on macOS until this script exits.
if command -v caffeinate >/dev/null; then
  caffeinate -i -w $$ &
fi

names=() pids=() logs=() epochs=() befores=() started=() ids=()
stopping=""
trap 'stopping=1; kill -TERM ${pids[@]+"${pids[@]}"} 2>/dev/null || true' INT TERM HUP

mkdir -p runs logs
echo "building: $(echo $targets)"
for t in $targets; do cargo +nightly fuzz build "$t" >/dev/null; done
if [ -n "$stopping" ]; then
  echo "stopped before fuzzing started; nothing recorded" >&2
  exit 1
fi
if [ -n "$(git -C "$root" status --porcelain -- fuzz/Cargo.lock)" ]; then
  echo "the build changed fuzz/Cargo.lock; commit the updated lock file first" >&2
  exit 1
fi
host=$(rustc +nightly -vV | sed -n 's/^host: //p')

# Run each fuzzer binary directly rather than through `cargo fuzz run`, so a
# stop request reaches it: libFuzzer stops on SIGTERM, prints its stats and
# exits with 72.
for t in $targets; do
  mkdir -p "artifacts/$t" "corpus/$t"
  log="logs/$(date -u +%Y%m%dT%H%M%SZ)-$t.log"
  names+=("$t")
  ids+=("$(code_id "$t")")
  befores+=("$(ls "artifacts/$t" | tr '\n' ' ')")
  started+=("$(date -u +%Y-%m-%dT%H:%M:%SZ)")
  epochs+=("$(date +%s)")
  logs+=("$log")
  "target/$host/release/$t" -artifact_prefix="artifacts/$t/" -max_total_time="$seconds" \
    -print_final_stats=1 "corpus/$t" >"$log" 2>&1 &
  pids+=("$!")
done
# A stop request that arrived while the fuzzers were starting.
if [ -n "$stopping" ]; then
  kill -TERM "${pids[@]}" 2>/dev/null || true
fi
echo "running for $hours h: ${names[*]}"
echo "Ctrl-C stops them early; the hours run so far are still recorded."

for i in "${!names[@]}"; do
  set +e
  # Once stopping, signal each fuzzer again before waiting: a signal sent while
  # it was still starting can be lost.
  [ -n "$stopping" ] && kill -TERM "${pids[$i]}" 2>/dev/null
  wait "${pids[$i]}"
  rc=$?
  # A trapped signal interrupts `wait` (status > 128). Wait again for the
  # fuzzer's own exit status; bash keeps it even if the child was already reaped.
  tries=0
  while [ -n "$stopping" ] && [ "$rc" -gt 128 ] && [ "$tries" -lt 5 ]; do
    kill -TERM "${pids[$i]}" 2>/dev/null
    wait "${pids[$i]}"
    r=$?
    [ "$r" -eq 127 ] && break
    rc=$r
    tries=$((tries + 1))
  done
  set -e
  record "${names[$i]}" "${started[$i]}" "${epochs[$i]}" "$rc" "${logs[$i]}" \
    "${befores[$i]}" "$seconds" "${ids[$i]}" "$commit"
done
echo
summary
echo "New records are in fuzz/runs/. Copy them to fuzz/campaigns/ in your development clone to commit."
