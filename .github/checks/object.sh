#!/usr/bin/env bash
# Checks one git object, read as stored from stdin, against what the hygiene
# job and the Release workflow allow (see commit-object.awk and
# tag-object.awk). Exits 0 if it is allowed, 1 if not, 2 on a usage error.
# Prints nothing, so a failure never repeats a name in a public log (S-012).
#
#   git cat-file commit <rev> | .github/checks/object.sh commit
#   git cat-file tag <ref> | .github/checks/object.sh tag <name>
#
# A NUL becomes \001 first, which no pattern allows, so it cannot hide the
# rest of a line from awk. Works with the bash 3.2 that ships with macOS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1-}:$#" in
  commit:1) tr '\000' '\001' | LC_ALL=C awk -f "$here/commit-object.awk" ;;
  tag:2) tr '\000' '\001' | LC_ALL=C awk -v name="$2" -f "$here/tag-object.awk" ;;
  *)
    echo "usage: object.sh commit | object.sh tag <name>" >&2
    exit 2
    ;;
esac
