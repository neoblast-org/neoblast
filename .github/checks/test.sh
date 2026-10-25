#!/usr/bin/env bash
# Tests object.sh on crafted commit and tag objects: what the project makes
# must pass, and each way a name could ride inside an object must fail
# (S-015, S-017, S-022, S-025). The hygiene job runs it before the checks
# themselves.
#
#   .github/checks/test.sh
#
# Objects are built as text, so no repository is needed; `~` stands for a
# NUL. Prints one line per case and exits 1 if any case gets the wrong
# answer. The only identity in the cases besides the project's is a made-up
# one. Works with the bash 3.2 that ships with macOS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
failures=0
cases=0

expect() { # pass|fail label object [object.sh args...]
  local want="$1" label="$2" object="$3" got
  shift 3
  cases=$((cases + 1))
  if printf '%s' "$object" | tr '~' '\000' | "$here/object.sh" "$@" 2>/dev/null; then
    got=pass
  else
    got=fail
  fi
  if [ "$got" = "$want" ]; then
    echo "ok    $label"
  else
    echo "WRONG $label: expected $want, got $got"
    failures=$((failures + 1))
  fi
}

me='neoblast maintainers <dev@neoblast.dev>'
other='Alice Example <alice@example.org>'
hex=5393f6704b5059c4c8ecfbb6adc01718c13c4455
when='1792951223 +0000'

# --- commits -------------------------------------------------------------

tree="tree $hex"$'\n'
parent="parent $hex"$'\n'
author="author $me $when"$'\n'
committer="committer $me $when"$'\n'
csig=$'gpgsig -----BEGIN SSH SIGNATURE-----\n U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgJy2p9tl3mZ2G2Rk+TJgwL9C6aP\n AAAAQDnJ+I+WpBqdU2ih4c5TjOVX27iOQRgTv5R2/CKmQeCR5/y5zmL6h27RFiPT9s==\n -----END SSH SIGNATURE-----\n'
msg=$'\nfix: a subject\n\nA body\twith a tab.\n\nSigned-off-by: '"$me"$'\n'
good="$tree$parent$author$committer$csig$msg"

expect pass "commit: signed, as the project, in UTC" "$good" commit
expect pass "commit: the root commit, with no parent" "$tree$author$committer$csig$msg" commit
expect pass "commit: a merge, with two parents" "$tree$parent$parent$author$committer$csig$msg" commit
expect pass "commit: an empty message" "$tree$parent$author$committer$csig"$'\n' commit
# Allowed until a check verifies signatures (S-026).
expect pass "commit: unsigned" "$tree$parent$author$committer$msg" commit

expect fail "commit: another author" "$tree$parent""author $other $when"$'\n'"$committer$csig$msg" commit
expect fail "commit: another committer" "$tree$parent$author""committer $other $when"$'\n'"$csig$msg" commit
expect fail "commit: author not in UTC" "$tree$parent""author $me 1792951223 +0200"$'\n'"$committer$csig$msg" commit
expect fail "commit: a name after a parent's hash" "$tree""parent $hex $other"$'\n'"$author$committer$csig$msg" commit
expect fail "commit: an author line inside another header" "$tree$parent""x-$other author $me $when"$'\n'"$author$committer$csig$msg" commit
expect fail "commit: a NUL in the author line" "$tree$parent""author $me $when~ $other"$'\n'"$committer$csig$msg" commit
expect fail "commit: a mergetag header" "$tree$parent$author$committer"$'mergetag object '"$hex"$'\n type commit\n tagger '"$other $when"$'\n'"$csig$msg" commit
expect fail "commit: an encoding header" "$tree$parent$author$committer"$'encoding ISO-8859-1\n'"$csig$msg" commit
expect fail "commit: an unknown header" "$tree$parent$author$committer"$'x-reviewer '"$other"$'\n'"$csig$msg" commit
expect fail "commit: two signatures" "$tree$parent$author$committer$csig$csig$msg" commit
expect fail "commit: a PGP signature" "$tree$parent$author$committer"$'gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQEzBAABCAAdFiEE\n -----END PGP SIGNATURE-----\n'"$msg" commit
expect fail "commit: a name in the signature" "$tree$parent$author$committer"$'gpgsig -----BEGIN SSH SIGNATURE-----\n '"$other"$'\n -----END SSH SIGNATURE-----\n'"$msg" commit
expect fail "commit: an address in the signature" "$tree$parent$author$committer"$'gpgsig -----BEGIN SSH SIGNATURE-----\n alice@example.org\n -----END SSH SIGNATURE-----\n'"$msg" commit
expect fail "commit: a NUL in a signature line" "$tree$parent$author$committer"$'gpgsig -----BEGIN SSH SIGNATURE-----\n U1NIU0lH~'"$other"$'\n -----END SSH SIGNATURE-----\n'"$msg" commit
expect fail "commit: a line after the signature" "$tree$parent$author$committer$csig"$' '"$other"$'\n'"$msg" commit
expect fail "commit: a signature that never ends" "$tree$parent$author$committer"$'gpgsig -----BEGIN SSH SIGNATURE-----\n U1NIU0lH\n' commit
expect fail "commit: no message separator" "$tree$parent$author$committer$csig" commit
expect fail "commit: a NUL in the message (S-025)" "$tree$parent$author$committer$csig"$'\nsubject\n\nhidden~\nCo-authored-by: '"$other"$'\n' commit
expect fail "commit: a carriage return in the message" "$tree$parent$author$committer$csig"$'\nsubject\r\n' commit
expect fail "commit: a DEL in the message" "$tree$parent$author$committer$csig"$'\nsubject\177\n' commit
expect fail "commit: a vertical tab in the message" "$tree$parent$author$committer$csig"$'\nsubject\013\n' commit
expect fail "commit: a backspace in the message" "$tree$parent$author$committer$csig"$'\nsubject\010\n' commit
expect fail "commit: an escape byte in the message" "$tree$parent$author$committer$csig"$'\nsubject \033[8m'"$other"$'\033[0m\n' commit

# --- tags ----------------------------------------------------------------

object="object $hex"$'\n'
type=$'type commit\n'
tagger="tagger $me $when"$'\n'
tsig=$'-----BEGIN SSH SIGNATURE-----\nU1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgJy2p9tl3mZ2G2Rk+TJgwL9C6aP\nAAAAQDnJ+I+WpBqdU2ih4c5TjOVX27iOQRgTv5R2/CKmQeCR5/y5zmL6h27RFiPT9s==\n-----END SSH SIGNATURE-----\n'
tag() { # name: the tag and tagger headers and the message
  printf 'tag %s\n%s\n%s\n' "$1" "$tagger" "$1"
}
v=v9.9.9
tgood="$object$type$(tag $v)"$'\n'"$tsig"

expect pass "tag: signed, as the project, in UTC" "$tgood" tag $v
expect pass "tag: a name with a slash" "$object$type$(tag rc/v9.9.9)"$'\n'"$tsig" tag rc/v9.9.9
# Allowed until a check verifies signatures (S-026).
expect pass "tag: unsigned" "$object$type$(tag $v)"$'\n' tag $v

expect fail "tag: checked under another name" "$tgood" tag v9.9.8
expect fail "tag: a name after the tag's name" "$object$type"$'tag '"$v $other"$'\n'"$tagger"$'\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: a name after the object's hash" "object $hex $other"$'\n'"$type$(tag $v)"$'\n'"$tsig" tag $v
expect fail "tag: no object" "$type$(tag $v)"$'\n'"$tsig" tag $v
expect fail "tag: no type" "$object$(tag $v)"$'\n'"$tsig" tag $v
expect fail "tag: no tag header" "$object$type$tagger"$'\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: a header naming another tag" "$object$type"$'tag v9.9.8\n'"$tagger"$'\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: a tag of a tag (S-017)" "$object"$'type tag\n'"$(tag $v)"$'\n'"$tsig" tag $v
expect fail "tag: a tag of a tree" "$object"$'type tree\n'"$(tag $v)"$'\n'"$tsig" tag $v
expect fail "tag: another tagger" "$object$type"$'tag '"$v"$'\ntagger '"$other $when"$'\n\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: tagger not in UTC" "$object$type"$'tag '"$v"$'\ntagger '"$me"$' 1792951223 -0500\n\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: no tagger" "$object$type"$'tag '"$v"$'\n\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: a NUL in the tagger line" "$object$type"$'tag '"$v"$'\ntagger '"$me $when~ $other"$'\n\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: two object lines" "$object$object$type$(tag $v)"$'\n'"$tsig" tag $v
expect fail "tag: an unknown header" "$object$type"$'tag '"$v"$'\n'"$tagger"$'x-reviewer '"$other"$'\n\n'"$v"$'\n'"$tsig" tag $v
expect fail "tag: a message other than the name" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\nrelease '"$v"$'\n'"$tsig" tag $v
expect fail "tag: a second message line" "$object$type$(tag $v)"$'\nCo-authored-by: '"$other"$'\n'"$tsig" tag $v
expect fail "tag: a NUL in the message" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n'"$v~ $other"$'\n'"$tsig" tag $v
expect fail "tag: a carriage return after the name" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n'"$v"$'\r\n'"$tsig" tag $v
expect fail "tag: no message" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n' tag $v
expect fail "tag: a PGP signature" "$object$type$(tag $v)"$'\n-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEE\n-----END PGP SIGNATURE-----\n' tag $v
expect fail "tag: a name in the signature" "$object$type$(tag $v)"$'\n-----BEGIN SSH SIGNATURE-----\n'"$other"$'\n-----END SSH SIGNATURE-----\n' tag $v
expect fail "tag: an address in the signature" "$object$type$(tag $v)"$'\n-----BEGIN SSH SIGNATURE-----\nalice@example.org\n-----END SSH SIGNATURE-----\n' tag $v
expect fail "tag: a line after the signature" "$tgood$other"$'\n' tag $v
expect fail "tag: two signatures" "$tgood$tsig" tag $v
expect fail "tag: a signature that never ends" "$object$type$(tag $v)"$'\n-----BEGIN SSH SIGNATURE-----\nU1NIU0lH\n' tag $v

# --- usage ---------------------------------------------------------------

expect fail "usage: no kind" "$good"
expect fail "usage: a tag without its name" "$tgood" tag
expect fail "usage: a commit with an argument" "$good" commit x
expect fail "usage: a tag with two names" "$tgood" tag $v $v

echo "$cases cases, $failures wrong"
[ "$failures" -eq 0 ]
