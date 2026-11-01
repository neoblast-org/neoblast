#!/usr/bin/env bash
# Verifies one commit's or one tag's signature against an allowed-signers
# file (S-026). Exits 0 only if it is signed, the signature is good and
# its key is in the file, 1 if not, 2 on a usage error. Prints nothing, so a
# failure never repeats a name or a key in a public log (S-012).
#
#   .github/checks/signed.sh <allowed signers> commit <rev>
#   .github/checks/signed.sh <allowed signers> tag <ref>
#
# git has ssh-keygen verify an SSH signature in the namespace "git"; a good
# one by a key the file does not list fails. The file and the programs are
# set here, so a git config that names others is not read: OpenPGP and
# X.509 signatures, such as GitHub's own, go to `false` and fail, whatever
# keys the caller's GnuPG holds. The revision is never read as an option.
# object.sh checks how a signature is laid out; this checks who made it.
# Works with the bash 3.2 that ships with macOS.
set -euo pipefail

case "${2-}:$#" in
  commit:3) verify=verify-commit ;;
  tag:3) verify=verify-tag ;;
  *)
    echo "usage: signed.sh <allowed signers> commit <rev> | signed.sh <allowed signers> tag <ref>" >&2
    exit 2
    ;;
esac

if git -c gpg.ssh.program=ssh-keygen -c gpg.ssh.allowedSignersFile="$1" \
  -c gpg.openpgp.program=false -c gpg.x509.program=false \
  "$verify" --end-of-options "$3" >/dev/null 2>&1; then
  exit 0
fi
exit 1
