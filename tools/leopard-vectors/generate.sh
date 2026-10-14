#!/usr/bin/env bash
# Regenerate spec/test-vectors/erasure-leopard.json from Leopard-RS itself.
#
# Needs: git, a C++14 compiler (c++), python3 with venv, network access.
# Leopard is cloned into a temp dir and never vendored. See README.md.
set -euo pipefail

LEOPARD_REPO="https://github.com/catid/leopard"
LEOPARD_COMMIT="22ddc7804998d31c8f1a2617ee720e063b1fa6cd"
BLAKE3_PY="1.0.11"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="${1:-$root/spec/test-vectors/erasure-leopard.json}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "cloning leopard @ $LEOPARD_COMMIT"
git clone -q "$LEOPARD_REPO" "$work/src"
git -C "$work/src" -c advice.detachedHead=false checkout -q "$LEOPARD_COMMIT"

# Replace exact text, failing unless it occurs exactly `count` times. Python rather
# than sed -i so the script runs the same on macOS and Linux.
replace_text() { # file count old new
  python3 - "$@" <<'PY'
import sys
path, count, old, new = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
text = open(path).read()
found = text.count(old)
if found != count:
    sys.exit(f"{path}: expected {count} of {old!r}, found {found}")
open(path, "w").write(text.replace(old, new))
print(f"  replaced {found}x in {path.rsplit('/', 1)[-1]}")
PY
}

# Build 1, "ff16": stock Leopard with the 8-bit field disabled, so GF(2^16) is
# always used (as reed-solomon-simd does).
cp -R "$work/src" "$work/ff16"
replace_text "$work/ff16/LeopardCommon.h" 1 "#define LEO_HAS_FF8" "// disabled: #define LEO_HAS_FF8"

# Build 2, "ff16-m-gt-k": as above, plus the argument check that refuses
# recovery_count > original_count removed from leo_encode and leo_decode (the same
# line appears in both). The algorithm itself is unchanged.
cp -R "$work/ff16" "$work/ff16x"
replace_text "$work/ff16x/leopard.cpp" 2 \
  "if (recovery_count <= 0 || recovery_count > original_count)" \
  "if (recovery_count <= 0)"

# Leopard's x86 SIMD paths need the target flags; ARM builds need none. The SIMD
# path must not change the bytes, which CXXFLAGS_SIMD="-mssse3" can confirm.
simd=""
case "$(uname -m)" in x86_64|amd64|i?86) simd="${CXXFLAGS_SIMD--mssse3 -mavx2}" ;; esac

for b in ff16 ff16x; do
  echo "building $b"
  # shellcheck disable=SC2086
  c++ -O2 -std=c++14 $simd -I"$work/$b" -o "$work/$b.bin" "$here/harness.cpp" \
    "$work/$b/leopard.cpp" "$work/$b/LeopardCommon.cpp" \
    "$work/$b/LeopardFF8.cpp" "$work/$b/LeopardFF16.cpp"
done

python3 -m venv "$work/venv"
# Pinned by version only; this is a maintainer tool, and its output is the
# committed fixture file, which is reviewed as a diff.
"$work/venv/bin/pip" install -q "blake3==$BLAKE3_PY"

echo "running cases"
"$work/venv/bin/python" - "$here/cases.txt" "$work" "$work/out.json" "$LEOPARD_REPO" "$LEOPARD_COMMIT" <<'PY'
import json, subprocess, sys
from blake3 import blake3

cases_path, work, out, repo, commit = sys.argv[1:]
cases = []
for line in open(cases_path):
    line = line.split("#", 1)[0].strip()
    if not line:
        continue
    k, m, size = (int(x) for x in line.split())
    assert k >= 2 and size % 64 == 0, line
    build = "ff16-m-gt-k" if m > k else "ff16"
    binary = f"{work}/{'ff16x' if m > k else 'ff16'}.bin"
    lines = subprocess.run([binary, str(k), str(m), str(size)], check=True,
                           capture_output=True, text=True).stdout.split()
    assert len(lines) == m, (line, len(lines))
    recovery = b"".join(bytes.fromhex(h) for h in lines)
    assert len(recovery) == m * size
    cases.append({
        "k": k, "m": m, "shard_bytes": size, "leopard_build": build,
        "recovery_head": recovery[:16].hex(),
        "recovery_blake3": blake3(recovery).hexdigest(),
    })
    print(f"  {k}/{m} x {size}: {cases[-1]['recovery_blake3'][:16]}")

doc = {
    "format": "neoblast erasure fixtures v1",
    "note": "Recovery shards produced by Leopard-RS itself. neoblast-erasure must reproduce "
            "every case byte for byte. Regenerate with tools/leopard-vectors/generate.sh.",
    "leopard": {
        "repo": repo,
        "commit": commit,
        "builds": {
            "ff16": "LEO_HAS_FF8 disabled, so GF(2^16) is always used",
            "ff16-m-gt-k": "ff16 plus the recovery_count > original_count argument check "
                           "removed from leo_encode and leo_decode",
        },
    },
    "input": "original shard i (0-based) is the next shard_bytes outputs of one splitmix64 "
             "stream, low byte of each output, shards filled in order; the stream state "
             "starts at k*1000003 + m*1009 + shard_bytes (wrapping u64)",
    "output": "recovery_blake3 = BLAKE3 of all m recovery shards concatenated in index order; "
              "recovery_head = first 16 bytes of recovery shard 0",
    "cases": cases,
}
with open(out, "w") as f:
    json.dump(doc, f, indent=2)
    f.write("\n")
print(f"generated {len(cases)} cases")
PY
mv "$work/out.json" "$out"
echo "wrote $out"
