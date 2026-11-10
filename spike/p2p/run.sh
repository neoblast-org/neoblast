#!/usr/bin/env bash
# Runs the libp2p or iroh spike's attempts between two machines (plan:
# plans/todo/p2p-library-spike.md; how-to: spike/p2p/README.md).
#
#   bash spike/p2p/run.sh a --mailbox <ssh host|local>
#   bash spike/p2p/run.sh b --mailbox <ssh host|local> --pair <pair> --run <n> \
#                      --first libp2p|iroh [--attempts <n>] [--ip v4|dual]
#
# The A end answers; the B end drives, and sends A the IP mode and iroh's
# port-mapping setting with each request. B writes one record per attempt to
# spike/p2p/records/run<n>-<pair>-<ip>.jsonl and, when restarted, carries on
# from the attempts already recorded. The two ends meet through a mailbox
# directory on the VPS, reached over ssh, so neither needs to reach the
# other. Records hold no dates, times, addresses, hostnames or node IDs;
# logs, which may, go to spike/p2p/logs/ and are not committed.
#
# Both ends read spike/p2p/relays.env (not committed), which sets:
#   LIBP2P_RELAY=/ip4/<vps ip>/udp/<port>/quic-v1/p2p/<relay peer id>
#   IROH_RELAY=https://<relay hostname>/
#   IROH_PORTMAP=on|off
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
bin="$here/target/release"
mailbox_dir='p2p-spike-mailbox'
pause=10           # rule 8: seconds between attempts, after both processes exit
answer_within=45   # how long B waits for A's address: A's own limit is 30 s
dial_guard=150     # B stops itself by 95 s; this kills it if it hangs

die() { echo "run.sh: $*" >&2; exit 2; }

role=${1:-}; shift || true
mailbox='' pair='' run='' first='' attempts='' ip='v4'
while [ $# -gt 0 ]; do
    case $1 in
        --mailbox) mailbox=$2; shift 2 ;;
        --pair) pair=$2; shift 2 ;;
        --run) run=$2; shift 2 ;;
        --first) first=$2; shift 2 ;;
        --attempts) attempts=$2; shift 2 ;;
        --ip) ip=$2; shift 2 ;;
        *) die "unknown argument: $1" ;;
    esac
done
[ -n "$mailbox" ] || die "--mailbox is required"
case $ip in v4|dual) ;; *) die "--ip must be v4 or dual" ;; esac
[ -f "$here/relays.env" ] || die "spike/p2p/relays.env is missing; see README.md"
# shellcheck source=/dev/null
. "$here/relays.env"
[ -n "${LIBP2P_RELAY:-}" ] && [ -n "${IROH_RELAY:-}" ] || die "relays.env must set LIBP2P_RELAY and IROH_RELAY"
case ${IROH_PORTMAP:-} in on|off) ;; *) die "relays.env must set IROH_PORTMAP to on or off" ;; esac
for b in spike-libp2p spike-iroh; do
    [ -x "$bin/$b" ] || die "$bin/$b is missing: cargo build --release in spike/p2p"
done
mkdir -p "$here/logs"

# The mailbox: a directory on the VPS, or here when this end is the VPS.
# ssh's own failures (exit 255) are retried, so a dropped link on the
# hotspot does not stop either end.
ssh_opts=(-o BatchMode=yes -o ControlMaster=auto -o "ControlPath=$HOME/.ssh/p2p-spike-%C" -o ControlPersist=10m)
box() { # box <shell command run in the mailbox directory>
    if [ "$mailbox" = local ]; then
        (mkdir -p "$HOME/$mailbox_dir" && cd "$HOME/$mailbox_dir" && sh -c "$1")
        return
    fi
    local i s
    for i in 1 2 3 4 5; do
        s=0
        ssh -n "${ssh_opts[@]}" "$mailbox" "mkdir -p $mailbox_dir && cd $mailbox_dir && $1" || s=$?
        [ "$s" -ne 255 ] && return "$s"
        sleep 2
    done
    return 255
}
box_put() { # box_put <name> <line>, written whole or not at all
    local i cmd="mkdir -p $mailbox_dir && cd $mailbox_dir && cat > $1.tmp && mv $1.tmp $1"
    if [ "$mailbox" = local ]; then
        printf '%s\n' "$2" | (mkdir -p "$HOME/$mailbox_dir" && cd "$HOME/$mailbox_dir" && cat >"$1.tmp" && mv "$1.tmp" "$1")
        return
    fi
    for i in 1 2 3 4 5; do
        printf '%s\n' "$2" | ssh "${ssh_opts[@]}" "$mailbox" "$cmd" && return 0
        sleep 2
    done
    return 1
}
box_has() { box "test -e $1" 2>/dev/null; }
box_get() { box "cat $1"; }

# The flags a program takes. B sends its IP mode and port-mapping setting
# with each request, so both ends always use the same.
flags() { # flags <library> <ip: v4|dual> <iroh port mapping: on|off>
    local f=()
    [ "$2" = dual ] && f+=(--dual)
    [ "$1" = iroh ] && [ "$3" = off ] && f+=(--no-portmap)
    echo "${f[@]:-}"
}

# ---- A: answer B's requests until stopped ---------------------------------
run_a() {
    echo "A: waiting for requests in the mailbox" >&2
    while true; do
        local newest seq
        # Only the newest request, and only if it is under 2 minutes old: an
        # older one was left by a B that stopped, and answering it would
        # hold A up for nothing.
        newest=$(box "find . -maxdepth 1 -name '*.req' -mmin -2 -exec ls -t {} + | head -n 1") || { sleep 1; continue; }
        seq=${newest#./}; seq=${seq%.req}
        # Answered already, or being answered by an earlier A.
        if [ -n "$seq" ] && ! box "test -e $seq.exited || test -e $seq.addr || test -e $seq.fail"; then
            answer "$seq"
        fi
        sleep 1
    done
}

answer() { # answer <seq>
    local seq=$1 req lib rip rpm out pid line='' waited=0
    req=$(box_get "$seq.req") || return 0
    read -r lib rip rpm <<<"$req"
    out=$(mktemp "$here/logs/a.XXXXXX")
    case "$lib.$rip.$rpm" in
        libp2p.v4.on|libp2p.v4.off|libp2p.dual.on|libp2p.dual.off)
            "$bin/spike-libp2p" listen --relay "$LIBP2P_RELAY" $(flags libp2p "$rip" "$rpm") >"$out" 2>>"$here/logs/a-$seq.log" & ;;
        iroh.v4.on|iroh.v4.off|iroh.dual.on|iroh.dual.off)
            "$bin/spike-iroh" listen --relay "$IROH_RELAY" $(flags iroh "$rip" "$rpm") >"$out" 2>>"$here/logs/a-$seq.log" & ;;
        *) rm -f "$out"; box_put "$seq.fail" bad-request || true; return 0 ;;
    esac
    pid=$!
    while [ -z "$line" ] && kill -0 "$pid" 2>/dev/null && [ "$waited" -lt $((answer_within * 2)) ]; do
        line=$(head -n 1 "$out")
        sleep 0.5; waited=$((waited + 1))
    done
    [ -n "$line" ] || line=$(head -n 1 "$out")
    if [ -z "$line" ]; then
        kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
        box_put "$seq.fail" a-not-ready || true
    else
        box_put "$seq.addr" "$line" || true
        # B takes at most about 95 s, then says it is done. A newer request
        # means this B was stopped and a new one has started: stop waiting.
        waited=0
        while [ "$waited" -lt 130 ] &&
            ! box "test -e $seq.done || test -n \"\$(find . -maxdepth 1 -name '*.req' -newer $seq.req)\"" 2>/dev/null; do
            sleep 1; waited=$((waited + 1))
        done
        kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    fi
    rm -f "$out"
    box_put "$seq.exited" yes || true
}

# ---- B: run the attempts and write the records ------------------------------
version_of() { # the measured library's version, from the spike's Cargo.lock
    awk -v n="$1" '$0 == "name = \"" n "\"" { getline; gsub(/version = |"/, ""); print; exit }' "$here/Cargo.lock"
}

probe_both() {
    "$bin/spike-libp2p" probe --relay "$LIBP2P_RELAY" 2>>"$here/logs/b-probe.log" &&
        "$bin/spike-iroh" probe --relay "$IROH_RELAY" 2>>"$here/logs/b-probe.log"
}

# record <library> <attempt> <try> <report json or empty> <harness reason or empty>
record() {
    python3 -I - "$records" "$run" "$pair" "$ip" "$1" "$(version_of "$1")" "$2" "$3" "$IROH_PORTMAP" "$4" "$5" <<'PY'
import json, sys
path, run, pair, ip, lib, version, attempt, tries, portmap, report, reason = sys.argv[1:]
rec = {"run": int(run), "pair": pair, "ip": ip, "library": lib, "version": version,
       "attempt": int(attempt), "try": int(tries), "iroh_portmap": portmap}
if reason:
    rec.update({"outcome": "harness-error", "reason": reason})
elif report:
    rec.update(json.loads(report))
else:
    # B produced no report after both had started: it counts (rule 10).
    rec.update({"outcome": "failed", "b_crashed": True})
with open(path, "a", encoding="utf-8") as f:
    f.write(json.dumps(rec, sort_keys=True) + "\n")
print(rec["outcome"] + (" (" + reason + ")" if reason else ""))
PY
}

# How many tries an attempt has had, and whether it is done (not a harness error).
tries_of() { python3 -I - "$records" "$1" "$2" <<'PY'
import json, sys
path, lib, attempt = sys.argv[1], sys.argv[2], int(sys.argv[3])
tries, done = 0, False
try:
    for line in open(path, encoding="utf-8"):
        r = json.loads(line)
        if r["library"] == lib and r["attempt"] == attempt:
            tries += 1
            done = done or r["outcome"] != "harness-error"
except FileNotFoundError:
    pass
print(tries, "done" if done else "open")
PY
}

one_attempt() { # one_attempt <library> <attempt> <try>
    local lib=$1 n=$2 t=$3 seq addr report='' waited=0 pid guard
    # A new name for every try, even one rerun after B was stopped, so B
    # never meets an old A's answer. It never reaches the records.
    seq="run$run.$pair.$ip.$lib.$n.$t.$$.$RANDOM"
    if ! probe_both; then record "$lib" "$n" "$t" '' relay-before; sleep 30; return; fi
    # The mailbox unreachable after retries is the harness's fault (rule 10).
    if ! box_put "$seq.req" "$lib $ip $IROH_PORTMAP"; then
        record "$lib" "$n" "$t" '' mailbox; sleep "$pause"; return
    fi
    while ! box_has "$seq.addr" && ! box_has "$seq.fail" && [ "$waited" -lt $((answer_within + 15)) ]; do
        sleep 1; waited=$((waited + 1))
    done
    if ! box_has "$seq.addr"; then
        # Rule 10 as the plan's step 2 notes read it: B cannot start.
        record "$lib" "$n" "$t" '' a-not-ready; sleep "$pause"; return
    fi
    if ! addr=$(box_get "$seq.addr") || [ -z "$addr" ]; then
        record "$lib" "$n" "$t" '' mailbox; sleep "$pause"; return
    fi
    case $lib in
        libp2p) "$bin/spike-libp2p" dial --to "$addr" $(flags libp2p "$ip" "$IROH_PORTMAP") >"$here/logs/b.out" 2>>"$here/logs/b-$seq.log" & ;;
        iroh) "$bin/spike-iroh" dial --to "$addr" --relay "$IROH_RELAY" $(flags iroh "$ip" "$IROH_PORTMAP") >"$here/logs/b.out" 2>>"$here/logs/b-$seq.log" & ;;
    esac
    pid=$!
    ( trap 'kill "$s" 2>/dev/null; exit 0' TERM; sleep "$dial_guard" & s=$!; wait "$s"; kill "$pid" 2>/dev/null ) &
    guard=$!
    wait "$pid" 2>/dev/null || true
    kill "$guard" 2>/dev/null || true
    report=$(head -n 1 "$here/logs/b.out")
    box_put "$seq.done" yes || true
    waited=0
    while ! box_has "$seq.exited" && [ "$waited" -lt 60 ]; do sleep 1; waited=$((waited + 1)); done
    if ! probe_both; then record "$lib" "$n" "$t" '' relay-after; sleep "$pause"; return; fi
    record "$lib" "$n" "$t" "$report" ''
    sleep "$pause"
}

run_b() {
    # Rules 5, 6 and 8: 50 attempts on a decisive pair, 20 on a control,
    # 10 in the separate IPv6 runs.
    [ "$ip" = dual ] && : "${attempts:=10}"
    case $pair in home1-cgnat|home1-home2) : "${attempts:=50}" ;; home1-vps|cgnat-vps) : "${attempts:=20}" ;;
        *) die "--pair must be home1-cgnat, home1-home2, home1-vps or cgnat-vps" ;; esac
    case $first in libp2p) second=iroh ;; iroh) second=libp2p ;; *) die "--first must be libp2p or iroh" ;; esac
    [[ $run =~ ^[0-9]+$ ]] || die "--run must be a number"
    mkdir -p "$here/records"
    records="$here/records/run$run-$pair-$ip.jsonl"
    local n lib state
    for n in $(seq 1 "$attempts"); do
        for lib in "$first" "$second"; do
            while true; do
                read -r t state < <(tries_of "$lib" "$n")
                [ "$state" = done ] && break
                printf 'attempt %s %s (try %s): ' "$n" "$lib" "$((t + 1))"
                one_attempt "$lib" "$n" "$((t + 1))"
            done
        done
    done
    echo "run $run, $pair, $ip: all $attempts attempts recorded for both libraries"
}

case $role in
    a) run_a ;;
    b) run_b ;;
    *) die "the first argument must be a or b" ;;
esac
