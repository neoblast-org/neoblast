#!/usr/bin/env bash
# Prints .github/allowed_signers as main holds it, for signed.sh: the file
# in the commit refs/remotes/origin/main names, which actions/checkout
# fetches. That ref alone is read: git rev-parse would read a tag or a
# branch called origin/main before it, and, were it missing, a tag called
# refs/remotes/origin/main (S-035). Exits 1, printing nothing, if there is
# no such ref, or main holds no such file; then nothing can be checked, so
# both workflows fail. A symlink there prints its target, and an empty
# file nothing; then no signature is good, and both workflows fail too. A key that a pull request adds counts only once it
# has merged (S-026), and the same file is read on every event, the release
# too (W-045).
#
#   .github/checks/main-signers.sh >allowed_signers
set -euo pipefail

if [ $# -ne 0 ]; then
  echo "usage: main-signers.sh" >&2
  exit 2
fi
main=$(git show-ref --verify --hash refs/remotes/origin/main 2>/dev/null) || exit 1
# A blob only: a directory or a missing path fails.
git cat-file blob "$main:.github/allowed_signers" 2>/dev/null || exit 1
