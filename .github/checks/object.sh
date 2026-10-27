#!/usr/bin/env bash
# Checks one git object, read as stored from stdin, or one commit message,
# against what the hygiene job and the Release workflow allow (see
# commit-object.awk, tag-object.awk and message.awk). Exits 0 if it is
# allowed, 1 if not, 2 on a usage error. Prints nothing, so a failure never
# repeats a name in a public log (S-012).
#
#   git cat-file commit <rev> | .github/checks/object.sh commit
#   git cat-file tag <ref> | .github/checks/object.sh tag <name>
#   git log -1 --format=%B <rev> | .github/checks/object.sh message
#
# In an object, a NUL becomes \001 first, which no pattern allows, so it
# cannot hide the rest of a line from awk. In a message, a carriage return
# just before a newline is dropped, so a CRLF message folds as git reads it
# (S-028), and any other becomes a newline. Works with the bash 3.2 that
# ships with macOS.
set -euo pipefail
# tr and awk read bytes, whatever the caller's locale: under a UTF-8 one,
# macOS's tr fails on other bytes and says so on stderr.
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1-}:$#" in
  commit:1) tr '\000' '\001' | LC_ALL=C awk -f "$here/commit-object.awk" ;;
  tag:2) tr '\000' '\001' | LC_ALL=C awk -v name="$2" -f "$here/tag-object.awk" ;;
  message:1) LC_ALL=C awk '{ sub(/\r$/, "") } 1' | tr '\r' '\n' | LC_ALL=C awk -f "$here/message.awk" ;;
  *)
    echo "usage: object.sh commit | object.sh tag <name> | object.sh message" >&2
    exit 2
    ;;
esac
