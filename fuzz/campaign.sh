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
import datetime, json, pathlib, sys
here = pathlib.Path(sys.argv[1])
current = dict(arg.split("=", 1) for arg in sys.argv[2:])
files = {}
for folder in ("campaigns", "runs"):  # committed records, then this machine's new ones
    for p in sorted((here / folder).glob("*.json")):
        files.setdefault(p.name, p)
runs = [json.loads(p.read_text()) for p in files.values()]
counted = ("clean", "stopped")

def hours(runs):
    # Wall-clock hours covered by the runs. Runs of one target that overlap in
    # time (two campaigns at once) count once.
    spans = []
    for r in runs:
        if r["result"] in counted:
            start = datetime.datetime.strptime(r["started_utc"], "%Y-%m-%dT%H:%M:%SZ")
            start = start.replace(tzinfo=datetime.timezone.utc).timestamp()
            spans.append((start, start + r["seconds_run"]))
    total, end = 0.0, None
    for s, e in sorted(spans):
        if end is None or s >= end:
            total, end = total + e - s, e
        elif e > end:
            total, end = total + e - end, e
    return total / 3600

print(f"{'target':22} {'h, current code':>16} {'h, all':>7} {'crashes':>8}  gate (24 h, no crash)")
mixed = []
for target, cid in current.items():
    mine = [r for r in runs if r["target"] == target]
    now = [r for r in mine if r["code_id"] == cid]
    hours_now = hours(now)
    hours_all = hours(mine)
    crashes = sum(1 for r in mine if r["result"] == "crash")
    met = hours_now >= 24 and not any(r["result"] in ("crash", "error") for r in now)
    print(f"{target:22} {hours_now:16.1f} {hours_all:7.1f} {crashes:8d}  {'met' if met else '-'}")
    # code_id does not cover the compiler, so hours from different ones add up.
    compilers = sorted({r.get("toolchain", "") for r in now if r["result"] in counted})
    if len(compilers) > 1:
        mixed.append(f"  {target}: " + "; ".join(c or "(not recorded)" for c in compilers))
if mixed:
    print("\nwarning: these hours on the current code come from more than one compiler:")
    print("\n".join(mixed))
PY
}

# Writes fuzz/runs/<start>-<target>.json for one finished run.
record() { # target started_utc start_epoch exit_code log artifacts_before planned code_id commit toolchain cargo_fuzz
  python3 - "$@" <<'PY'
import datetime, json, os, pathlib, platform, re, subprocess, sys
t, started, start_epoch, rc, log, before, planned, cid, commit, toolchain, cargo_fuzz = sys.argv[1:]
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
    "toolchain": toolchain,
    "cargo_fuzz": cargo_fuzz,
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

# One campaign at a time: two at once would share the corpora and their hours
# would overlap. The lock is a flock on fd 9, which the fuzzers inherit, so it
# is held until the last of them exits, even if this script is killed. Nothing
# else long-lived may inherit fd 9.
mkdir -p runs
exec 9>>runs/.campaign.lock
locked=0
python3 - <<'PY' || locked=$?
import errno, fcntl, sys
try:
    fcntl.flock(9, fcntl.LOCK_EX | fcntl.LOCK_NB)
except OSError as e:
    if e.errno in (errno.EWOULDBLOCK, errno.EAGAIN):
        sys.exit(1)
    print(f"cannot lock fuzz/runs/.campaign.lock: {e.strerror}", file=sys.stderr)
    sys.exit(2)
PY
if [ "$locked" -eq 1 ]; then
  echo "another campaign, or a fuzzer left running by one, is using this clone; stop it first" >&2
  echo "(lsof fuzz/runs/.campaign.lock lists them)" >&2
  exit 1
elif [ "$locked" -ne 0 ]; then
  exit 1
fi

# Keep the machine awake on macOS until this script exits.
if command -v caffeinate >/dev/null; then
  caffeinate -i -w $$ 9>&- &
fi

names=() pids=() logs=() epochs=() befores=() started=() ids=()
stopping=""
trap 'stopping=1; kill -TERM ${pids[@]+"${pids[@]}"} 2>/dev/null || true' INT TERM HUP

mkdir -p runs logs
echo "building: $(echo $targets)"
# Without fd 9: a build server (sccache) would otherwise hold the lock.
for t in $targets; do cargo +nightly fuzz build "$t" >/dev/null 9>&-; done
if [ -n "$stopping" ]; then
  echo "stopped before fuzzing started; nothing recorded" >&2
  exit 1
fi
# Without fd 9 too: git may start its fsmonitor daemon.
if [ -n "$(git -C "$root" status --porcelain -- fuzz/Cargo.lock 9>&-)" ]; then
  echo "the build changed fuzz/Cargo.lock; commit the updated lock file first" >&2
  exit 1
fi
host=$(rustc +nightly -vV 9>&- | sed -n 's/^host: //p')
# What built the binaries, read now: a nightly updated during the campaign
# must not be named in its records.
toolchain=$(rustc +nightly -V 9>&-)
cargo_fuzz=$(cargo +nightly fuzz --version 9>&- || true)

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
    "${befores[$i]}" "$seconds" "${ids[$i]}" "$commit" "$toolchain" "$cargo_fuzz"
done
echo
summary
echo "New records are in fuzz/runs/. Copy them to fuzz/campaigns/ in your development clone to commit."
