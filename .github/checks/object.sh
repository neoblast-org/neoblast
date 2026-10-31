#!/usr/bin/env bash
# Checks one git object, read as stored from stdin, one commit message, one
# commit's top-level tree, or one commit's identities or time zones as git
# log prints them, against what the hygiene job and the Release workflow
# allow (see commit-object.awk, tag-object.awk, ssh-signature.awk,
# message.awk, tree.awk, identity.awk and utc.awk). Exits 0 if it is
# allowed, 1 if not, 2 on a usage error. Prints nothing, so a failure never
# repeats a name in a public log (S-012).
#
#   git cat-file commit <rev> | .github/checks/object.sh commit
#   git cat-file tag <ref> | .github/checks/object.sh tag <name>
#   git log -1 --format=%B <rev> | .github/checks/object.sh message
#   git ls-tree <rev> | .github/checks/object.sh tree
#   git log -1 --date=format:%z --format=<see identity.awk> <rev> | .github/checks/object.sh identity
#   git log -1 --date=format:%z --format=<see utc.awk> <rev> | .github/checks/object.sh utc
#
# In an object, a NUL becomes \001 first, which no pattern allows, so it
# cannot hide the rest of a line from awk. A message goes to message.awk as
# it is. Works with the bash 3.2 that ships with macOS.
set -euo pipefail
# tr and awk read bytes, whatever the caller's locale: under a UTF-8 one,
# macOS's tr fails on other bytes and says so on stderr. Set here only.
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "${1-}:$#" in
  commit:1) tr '\000' '\001' | awk -f "$here/ssh-signature.awk" -f "$here/commit-object.awk" ;;
  tag:2) tr '\000' '\001' | awk -v name="$2" -f "$here/ssh-signature.awk" -f "$here/tag-object.awk" ;;
  message:1) awk -f "$here/message.awk" ;;
  tree:1) awk -f "$here/tree.awk" ;;
  identity:1) awk -f "$here/identity.awk" ;;
  utc:1) awk -f "$here/utc.awk" ;;
  *)
    echo "usage: object.sh commit | object.sh tag <name> | object.sh message | object.sh tree | object.sh identity | object.sh utc" >&2
    exit 2
    ;;
esac
