#!/usr/bin/env bash
# Tests object.sh: what the project makes must pass, and each way a name
# could ride inside a commit, a tag, a commit message or a commit's tree
# must fail (S-013, S-015, S-017, S-018, S-022, S-025, S-028, S-030, S-031,
# W-035). The hygiene job runs it, then mutate.sh, before the checks
# themselves.
#
#   .github/checks/test.sh        # every case under every awk found
#   .github/checks/test.sh -v     # and print each case
#   CHECKS_AWK=mawk .github/checks/test.sh   # under that awk only
#
# Most cases are built as text, each differing from an allowed one in one
# way; `~` stands for a NUL. The rest are objects git itself writes, in a
# throwaway repository with a throwaway SSH key, so the checks are known to
# pass what `git commit -S` and `git tag -s` make. Those need ssh-keygen;
# without it they are skipped, except in CI, where that fails. Every case
# runs under each awk on PATH (mawk on GitHub's runner, BSD awk on macOS,
# gawk, busybox). The only identity in the cases besides the project's is a
# made-up one. Works with the bash 3.2 that ships with macOS.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
verbose=0
if [ "${1-}" = -v ]; then verbose=1; fi
failures=0
cases=0
awk_name=""

record() { # want label got
  cases=$((cases + 1))
  if [ "$3" = "$1" ]; then
    if [ "$verbose" = 1 ]; then echo "ok    [$awk_name] $2"; fi
  else
    echo "WRONG [$awk_name] $2: expected $1, got $3"
    failures=$((failures + 1))
  fi
}

expect() { # pass|fail label object [object.sh args...]
  local want="$1" label="$2" object="$3" got=fail
  shift 3
  # Only this tr runs in the C locale: object.sh must set its own.
  if printf '%s' "$object" | LC_ALL=C tr '~' '\000' | "$here/object.sh" "$@" 2>/dev/null; then
    got=pass
  fi
  record "$want" "$label" "$got"
}

expect_file() { # pass|fail label file object.sh args...
  local want="$1" label="$2" file="$3" got=fail
  shift 3
  if "$here/object.sh" "$@" <"$file" 2>/dev/null; then
    got=pass
  fi
  record "$want" "$label" "$got"
}

me='neoblast maintainers <dev@neoblast.dev>'
other='Alice Example <alice@example.org>'
hex=5393f6704b5059c4c8ecfbb6adc01718c13c4455
when='1792951223 +0000'
# A UTF-8 locale, where there is one, for the case that needs object.sh to
# set its own (W-036).
utf8=$(locale -a 2>/dev/null | grep -i -E '^(C|en_US)\.utf-?8$' | head -n 1 || true)
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- objects git writes ----------------------------------------------------

# Each is saved as git stores it (or, for a message, as the hygiene job reads
# it) in $work/real/<name>, and checked below under every awk.
real=0
if command -v ssh-keygen >/dev/null 2>&1; then
  real=1
elif [ "${CI-}" = true ]; then
  echo "test.sh: ssh-keygen is needed in CI, for the objects git writes" >&2
  exit 1
else
  echo "skipped: the objects git writes (no ssh-keygen)"
fi

if [ "$real" = 1 ]; then
  # Nothing in the caller's environment may point git at another repository,
  # as inside a git hook.
  unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY \
    GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_COMMON_DIR GIT_NAMESPACE \
    GIT_CONFIG GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_CEILING_DIRECTORIES
  mkdir "$work/real"
  ssh-keygen -q -t ed25519 -N '' -C '' -f "$work/key"
  repo="$work/repo"
  g() { # git in the throwaway repository, as the project in UTC, signing with the throwaway key
    GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_EDITOR=true TZ=UTC \
      GIT_AUTHOR_NAME='neoblast maintainers' GIT_AUTHOR_EMAIL=dev@neoblast.dev \
      GIT_COMMITTER_NAME="${as_name:-neoblast maintainers}" GIT_COMMITTER_EMAIL="${as_email:-dev@neoblast.dev}" \
      GIT_AUTHOR_DATE="@1792951223 +0000" GIT_COMMITTER_DATE="@1792951223 ${zone:-+0000}" \
      git -C "$repo" -c gpg.format=ssh -c user.signingkey="$work/key" -c gpg.ssh.program=ssh-keygen \
      -c commit.gpgsign=false -c tag.gpgsign=false -c init.defaultBranch=main -c advice.nestedTag=false "$@"
  }
  save() { # name kind rev: the object as stored
    g cat-file "$2" "$3" >"$work/real/$1"
  }
  save_message() { # name rev: the message as the hygiene job reads it
    printf '%s\n' "$(g log -1 --format=%B "$2")" >"$work/real/$1"
  }
  mkdir "$repo"
  g init -q
  g commit -q --allow-empty -S -s -m "fix: a subject" -m $'A body\twith a tab.'
  save signed commit HEAD
  save_message signed-message HEAD
  g commit -q --allow-empty -m "docs: unsigned"
  save unsigned commit HEAD
  g tag -s v9.9.9 -m v9.9.9
  save signed-tag tag v9.9.9
  g tag -a v9.9.8 -m v9.9.8
  save unsigned-tag tag v9.9.8
  g tag -a v9.9.7 -m v9.9.7 v9.9.9
  save tag-of-tag tag v9.9.7
  g tag -a v9.9.6 -m v9.9.6 -m "Co-authored-by: $other"
  save two-line-tag tag v9.9.6
  g checkout -q -b side
  g commit -q --allow-empty -S -m "feat: on a side branch"
  as_name='Alice Example' as_email=alice@example.org g tag -s side-tag -m side-tag
  save other-tagger tag side-tag
  g checkout -q main
  g merge -q --no-ff -S -m "merge: a branch" side
  save merge commit HEAD
  g reset -q --hard HEAD^
  g merge -q --no-ff -S -m "merge: a signed tag" side-tag
  save mergetag commit HEAD
  g -c i18n.commitEncoding=ISO-8859-1 commit -q --allow-empty -S -m "fix: another encoding"
  save encoding commit HEAD
  zone=+0200 g commit -q --allow-empty -S -m "fix: not in UTC"
  save not-utc commit HEAD
  g commit -q --allow-empty -S -s -m "fix: a co-author" --trailer "Co-authored-by: $other"
  save_message co-author-message HEAD
  g commit -q --allow-empty -S -s -m "fix: a co-author in prose" -m "Co-authored-by: $other" -m "More prose."
  save_message co-author-in-prose HEAD
  g commit -q --allow-empty -S -s -m "fix: a folded reviewer line" -m "Reviewed-by: $me"$'\n '"$other"
  save_message folded-message HEAD
  printf '%s' "fix: a reviewer line folded with a CR"$'\n\nReviewed-by: '"$me"$'\n\r'"$other"$'\n' >"$work/cr-message"
  g commit -q --allow-empty -S --cleanup=verbatim -F "$work/cr-message"
  save_message cr-folded-message HEAD
  # A certificate's signature carries its key ID and principals (S-030).
  ssh-keygen -q -t ed25519 -N '' -C '' -f "$work/ca"
  ssh-keygen -q -t ed25519 -N '' -C '' -f "$work/ckey"
  ssh-keygen -q -s "$work/ca" -I "$other" -n alice@example.org "$work/ckey.pub"
  g -c user.signingkey="$work/ckey-cert.pub" commit -q --allow-empty -S -m "fix: signed with a certificate"
  save cert-signed commit HEAD
  g -c user.signingkey="$work/ckey-cert.pub" tag -s v9.9.5 -m v9.9.5
  save cert-tag tag v9.9.5
  printf 'A readme.\n' >"$repo/README.md"
  g add README.md
  g commit -q -S -m "docs: a readme"
  g ls-tree HEAD >"$work/real/tree"
  printf '%s %s\n' "$other" "$me" >"$repo/.mailmap"
  g add .mailmap
  g commit -q -S -m "docs: a mailmap"
  g ls-tree HEAD >"$work/real/mailmap-tree"
fi

run_cases() {
  # --- commits -------------------------------------------------------------

  tree="tree $hex"$'\n'
  parent="parent $hex"$'\n'
  author="author $me $when"$'\n'
  committer="committer $me $when"$'\n'
  # A signature as git writes one with a plain ed25519 key, here a throwaway
  # one (S-030); a certificate's and an ECDSA key's first lines; the second
  # line with text in the reserved field, over sha256, or with ssh-ed25518
  # as the signature's type; the first with a = inside, or of version 2.
  s1='U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg0nMBCCi7TezaCrfKe1/jNbNRBI'
  s2='aMvrUYIdW7QcS9TGEAAAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5'
  s3='AAAAQGIui2B3WteCUsOpocsTR9V0RWITdvlCLdh2Q8PvuDvU59vDXVu8C3h503nF7Qwkfe'
  s4='4DB5tjs3LCxzmD2AsWZgc='
  scert='U1NIU0lHAAAAAQAAAeIAAAAgc3NoLWVkMjU1MTktY2VydC12MDFAb3BlbnNzaC5jb20AAA'
  secdsa='U1NIU0lHAAAAAQAAAGgAAAATZWNkc2Etc2hhMi1uaXN0cDI1NgAAAAhuaXN0cDI1NgAAAE'
  s2text='aMvrUYIdW7QcS9TGEAAAADZ2l0AAAAGkFsaWZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5'
  s1eq='U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg0nMBCCi7TezaCrfKe1/jNbNRB='
  s1v2='U1NIU0lHAAAAAgAAADMAAAALc3NoLWVkMjU1MTkAAAAg0nMBCCi7TezaCrfKe1/jNbNRBI'
  s2sha256='aMvrUYIdW7QcS9TGEAAAADZ2l0AAAAAAAAAAZzaGEyNTYAAABTAAAAC3NzaC1lZDI1NTE5'
  s2type='aMvrUYIdW7QcS9TGEAAAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE4'
  cl() { # lines: a commit's signature lines
    printf ' %s\n' "$@"
  }
  cbegin=$'gpgsig -----BEGIN SSH SIGNATURE-----\n'
  cend=$' -----END SSH SIGNATURE-----\n'
  cbody="$(cl "$s1" "$s2" "$s3" "$s4")"$'\n'
  csig="$cbegin$cbody$cend"
  msg=$'\nfix: a subject\n\nA body\twith a tab.\n\nSigned-off-by: '"$me"$'\n'
  good="$tree$parent$author$committer$csig$msg"

  expect pass "commit: signed, as the project, in UTC" "$good" commit
  expect pass "commit: the root commit, with no parent" "$tree$author$committer$csig$msg" commit
  expect pass "commit: a merge, with two parents" "$tree$parent$parent$author$committer$csig$msg" commit
  expect pass "commit: an empty message" "$tree$parent$author$committer$csig"$'\n' commit
  # Allowed until a check verifies signatures (S-026).
  expect pass "commit: unsigned" "$tree$parent$author$committer$msg" commit
  LC_ALL="${utf8:-${LC_ALL-}}" expect pass "commit: a Latin-1 byte in the message, in a UTF-8 locale" "$tree$parent$author$committer$csig"$'\nfix: caf\351\n' commit
  expect pass "commit: a UTF-8 message" "$tree$parent$author$committer$csig"$'\nfix: caf\xc3\xa9 \xe2\x80\x94 a subject\n' commit

  expect fail "commit: another author" "$tree$parent""author $other $when"$'\n'"$committer$csig$msg" commit
  expect fail "commit: another committer" "$tree$parent$author""committer $other $when"$'\n'"$csig$msg" commit
  expect fail "commit: author not in UTC" "$tree$parent""author $me 1792951223 +0200"$'\n'"$committer$csig$msg" commit
  expect fail "commit: a name after a parent's hash" "$tree""parent $hex $other"$'\n'"$author$committer$csig$msg" commit
  expect fail "commit: a parent line inside another header" "$tree""x-$other parent $hex"$'\n'"$author$committer$csig$msg" commit
  expect fail "commit: a name before a signature's start" "$tree$parent$author$committer""gpgsig $other"$'\n'"$cbody$cend$msg" commit
  expect fail "commit: a name before a signature's end" "$tree$parent$author$committer$cbegin$cbody"' '"$other"$' -----END SSH SIGNATURE-----\n'"$msg" commit
  expect fail "commit: an author line inside another header" "$tree$parent""x-$other author $me $when"$'\n'"$author$committer$csig$msg" commit
  expect fail "commit: a NUL in the author line" "$tree$parent""author $me $when~ $other"$'\n'"$committer$csig$msg" commit
  expect fail "commit: a mergetag header" "$tree$parent$author$committer"$'mergetag object '"$hex"$'\n type commit\n tagger '"$other $when"$'\n'"$csig$msg" commit
  expect fail "commit: an encoding header" "$tree$parent$author$committer"$'encoding ISO-8859-1\n'"$csig$msg" commit
  expect fail "commit: an unknown header" "$tree$parent$author$committer"$'x-reviewer '"$other"$'\n'"$csig$msg" commit
  expect fail "commit: two signatures" "$tree$parent$author$committer$csig$csig$msg" commit
  expect fail "commit: a PGP signature" "$tree$parent$author$committer"$'gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQEzBAABCAAdFiEE\n -----END PGP SIGNATURE-----\n'"$msg" commit
  expect fail "commit: a name in the signature" "$tree$parent$author$committer$cbegin$cbody"' '"$other"$'\n'"$cend$msg" commit
  expect fail "commit: an address in the signature" "$tree$parent$author$committer$cbegin$cbody"$' alice@example.org\n'"$cend$msg" commit
  expect fail "commit: a NUL in a signature line" "$tree$parent$author$committer$cbegin$(cl "$s1" "$s2~$other" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature by an SSH certificate (S-030)" "$tree$parent$author$committer$cbegin$(cl "$scert" "$s2" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature by an ECDSA key" "$tree$parent$author$committer$cbegin$(cl "$secdsa" "$s2" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: text in a signature's reserved field (S-030)" "$tree$parent$author$committer$cbegin$(cl "$s1" "$s2text" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature four characters longer" "$tree$parent$author$committer$cbegin$(cl "$s1" "$s2" "$s3" AAAA "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature of version 2" "$tree$parent$author$committer$cbegin$(cl "$s1v2" "$s2" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature over sha256" "$tree$parent$author$committer$cbegin$(cl "$s1" "$s2sha256" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature of another type" "$tree$parent$author$committer$cbegin$(cl "$s1" "$s2type" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a signature that does not end in =" "$tree$parent$author$committer$cbegin$(cl "$s1" "$s2" "$s3" "${s4%=}A")"$'\n'"$cend$msg" commit
  expect fail "commit: a = inside a signature" "$tree$parent$author$committer$cbegin$(cl "$s1eq" "$s2" "$s3" "$s4")"$'\n'"$cend$msg" commit
  expect fail "commit: a header inside the signature" "$tree$parent$author$committer$cbegin$cbody"$'x-reviewer '"$other"$'\n'"$cend$msg" commit
  expect fail "commit: a line after the signature" "$tree$parent$author$committer$csig"$' '"$other"$'\n'"$msg" commit
  expect fail "commit: a signature that never ends" "$tree$parent$author$committer$cbegin$cbody" commit
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
  tl() { # lines: a tag's signature lines
    printf '%s\n' "$@"
  }
  tbegin=$'-----BEGIN SSH SIGNATURE-----\n'
  tend=$'-----END SSH SIGNATURE-----\n'
  tbody="$(tl "$s1" "$s2" "$s3" "$s4")"$'\n'
  tsig="$tbegin$tbody$tend"
  tag() { # name: the tag and tagger headers and the message
    printf 'tag %s\n%s\n%s\n' "$1" "$tagger" "$1"
  }
  v=v9.9.9
  tgood="$object$type$(tag $v)"$'\n'"$tsig"

  expect pass "tag: signed, as the project, in UTC" "$tgood" tag $v
  expect pass "tag: a name with a slash" "$object$type$(tag rc/v9.9.9)"$'\n'"$tsig" tag rc/v9.9.9
  expect pass "tag: a name that is a number" "$object$type"$'tag 1\n'"$tagger"$'\n1\n'"$tsig" tag 1
  # Allowed until a check verifies signatures (S-026).
  expect pass "tag: unsigned" "$object$type$(tag $v)"$'\n' tag $v

  expect fail "tag: checked under another name" "$tgood" tag v9.9.8
  expect fail "tag: a name after the tag's name" "$object$type"$'tag '"$v $other"$'\n'"$tagger"$'\n'"$v"$'\n'"$tsig" tag $v
  expect fail "tag: a name after the object's hash" "object $hex $other"$'\n'"$type$(tag $v)"$'\n'"$tsig" tag $v
  expect fail "tag: an object line inside another header" "x-$other object $hex"$'\n'"$type$(tag $v)"$'\n'"$tsig" tag $v
  expect fail "tag: a tagger line inside another header" "$object$type"$'tag '"$v"$'\nx-'"$other tagger $me $when"$'\n\n'"$v"$'\n'"$tsig" tag $v
  expect fail "tag: a name after a signature's start" "$object$type$(tag $v)"$'\n-----BEGIN SSH SIGNATURE----- '"$other"$'\n'"$tbody$tend" tag $v
  expect fail "tag: a name before a signature's end" "$object$type$(tag $v)"$'\n'"$tbegin$tbody$other"$' -----END SSH SIGNATURE-----\n' tag $v
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
  expect fail "tag: two type lines" "$object$type$type$(tag $v)"$'\n'"$tsig" tag $v
  expect fail "tag: two tag headers" "$object$type"$'tag '"$v"$'\n'"$(tag $v)"$'\n'"$tsig" tag $v
  expect fail "tag: two tagger lines" "$object$type"$'tag '"$v"$'\n'"$tagger$tagger"$'\n'"$v"$'\n'"$tsig" tag $v
  expect fail "tag: an unknown header" "$object$type"$'tag '"$v"$'\n'"$tagger"$'x-reviewer '"$other"$'\n\n'"$v"$'\n'"$tsig" tag $v
  expect fail "tag: the number in the name spelt another way" "$object$type"$'tag 1\n'"$tagger"$'\n1.0\n'"$tsig" tag 1
  expect fail "tag: a message other than the name" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\nrelease '"$v"$'\n'"$tsig" tag $v
  expect fail "tag: a second message line" "$object$type$(tag $v)"$'\nCo-authored-by: '"$other"$'\n'"$tsig" tag $v
  expect fail "tag: a NUL in the message" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n'"$v~ $other"$'\n'"$tsig" tag $v
  expect fail "tag: a carriage return after the name" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n'"$v"$'\r\n'"$tsig" tag $v
  expect fail "tag: no message" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n' tag $v
  expect fail "tag: a signature without the name" "$object$type"$'tag '"$v"$'\n'"$tagger"$'\n'"$tsig" tag $v
  expect fail "tag: the name twice" "$object$type$(tag $v)"$'\n'"$v"$'\n'"$tsig" tag $v
  expect fail "tag: a PGP signature" "$object$type$(tag $v)"$'\n-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEE\n-----END PGP SIGNATURE-----\n' tag $v
  expect fail "tag: a name in the signature" "$object$type$(tag $v)"$'\n'"$tbegin$tbody$other"$'\n'"$tend" tag $v
  expect fail "tag: an address in the signature" "$object$type$(tag $v)"$'\n'"$tbegin$tbody"$'alice@example.org\n'"$tend" tag $v
  expect fail "tag: a signature by an SSH certificate (S-030)" "$object$type$(tag $v)"$'\n'"$tbegin$(tl "$scert" "$s2" "$s3" "$s4")"$'\n'"$tend" tag $v
  expect fail "tag: text in a signature's reserved field (S-030)" "$object$type$(tag $v)"$'\n'"$tbegin$(tl "$s1" "$s2text" "$s3" "$s4")"$'\n'"$tend" tag $v
  expect fail "tag: a line after the signature" "$tgood$other"$'\n' tag $v
  expect fail "tag: two signatures" "$tgood$tsig" tag $v
  expect fail "tag: a signature that never ends" "$object$type$(tag $v)"$'\n'"$tbegin$tbody" tag $v
  expect fail "tag: a signature with no lines" "$object$type$(tag $v)"$'\n'"$tbegin" tag $v

  # --- messages --------------------------------------------------------------

  m=$'fix: a subject\n\nA body.\n'
  expect pass "message: no attribution lines" "$m" message
  expect pass "message: signed off by the project" "$m"$'\nSigned-off-by: '"$me"$'\n' message
  expect pass "message: a sign-off ending in CRLF" "$m"$'\nSigned-off-by: '"$me"$'\r\n' message
  expect pass "message: trailing spaces after the project" "$m"$'\nSigned-off-by: '"$me"$'  \t\n' message
  expect pass "message: Standby: is no key" "$m"$'\nStandby: the next release\n' message
  expect pass "message: a URL holding -by:" "$m"$'\nhttps://example.org/stand-by:now\n' message
  # Lines git does not read as trailers are not checked (S-013).
  expect pass "message: a quoted line (not checked)" "$m"$'\n> Co-authored-by: '"$other"$'\n' message
  expect pass "message: a bulleted line (not checked)" "$m"$'\n- Co-authored-by: '"$other"$'\n' message
  expect pass "message: Reviewed by: with a space (not checked)" "$m"$'\nReviewed by: '"$other"$'\n' message
  expect pass "message: an indented line after a blank line" "$m"$'\nReviewed-by: '"$me"$'\n\n '"$other"$'\n' message
  expect pass "message: a line of blanks after a reviewer line" "$m"$'\nReviewed-by: '"$me"$'\n \t\n' message
  expect pass "message: a line of blanks and a CR after a reviewer line" "$m"$'\nReviewed-by: '"$me"$'\n \r\n' message

  expect fail "message: a co-author" "$m"$'\nCo-authored-by: '"$other"$'\n' message
  expect fail "message: a sign-off by someone else" "$m"$'\nSigned-off-by: '"$other"$'\n' message
  expect fail "message: the project and someone else" "$m"$'\nSigned-off-by: '"$me"$' and '"$other"$'\n' message
  expect fail "message: a reviewer" "$m"$'\nReviewed-by: '"$other"$'\n' message
  expect fail "message: an empty value" "$m"$'\nAcked-by:\n' message
  expect fail "message: a key in capitals" "$m"$'\nCO-AUTHORED-BY: '"$other"$'\n' message
  expect fail "message: an indented line" "$m"$'\n  \tCo-authored-by: '"$other"$'\n' message
  expect fail "message: a space before the colon" "$m"$'\nCo-authored-by : '"$other"$'\n' message
  expect fail "message: in an earlier paragraph (S-013)" $'fix: a subject\n\nCo-authored-by: '"$other"$'\n\nMore prose.\n' message
  expect fail "message: after a lone carriage return (S-018)" $'fix: a subject\rCo-authored-by: '"$other"$'\n' message
  expect fail "message: a U+2010 hyphen (S-018)" "$m"$'\nCo\xe2\x80\x90authored\xe2\x80\x90by: '"$other"$'\n' message
  expect fail "message: a U+FF0D hyphen (S-018)" "$m"$'\nCo\xef\xbc\x8dauthored\xef\xbc\x8dby: '"$other"$'\n' message
  expect fail "message: a reviewer line folded onto the next (S-028)" "$m"$'\nReviewed-by: '"$me"$'\n '"$other"$'\n' message
  expect fail "message: a reviewer line folded with a tab" "$m"$'\nReviewed-by: '"$me"$'\n\t'"$other"$'\n' message
  expect fail "message: a reviewer line folded in CRLF" "$m"$'\nReviewed-by: '"$me"$'\r\n '"$other"$'\r\n' message
  # Git folds each of these too; they once passed, as CRs were turned into
  # newlines before the fold rule ran (W-035).
  expect fail "message: folded after two CRs" "$m"$'\nReviewed-by: '"$me"$'\r\r\n '"$other"$'\n' message
  expect fail "message: folded with a CR" "$m"$'\nReviewed-by: '"$me"$'\n\r'"$other"$'\n' message
  expect fail "message: folded with a space and a CR" "$m"$'\nReviewed-by: '"$me"$'\n \r'"$other"$'\n' message
  expect fail "message: folded with a CR and a space" "$m"$'\nReviewed-by: '"$me"$'\n\r '"$other"$'\n' message
  expect fail "message: a name after a CR in a reviewer line" "$m"$'\nReviewed-by: '"$me"$'\r '"$other"$'\n' message
  expect fail "message: an underscore as the hyphen" "$m"$'\nCo_authored_by: '"$other"$'\n' message

  # --- trees ---------------------------------------------------------------

  entry() { # mode type name: one line of git ls-tree
    printf '%s %s %s\t%s\n' "$1" "$2" "$hex" "$3"
  }
  # As ls-tree quotes a name with a byte above DEL: café.md.
  files="$(entry 100644 blob README.md)"$'\n'"$(entry 040000 tree crates)"$'\n'"$(entry 100644 blob '"caf\303\251.md"')"$'\n'
  expect pass "tree: the project's files" "$files" tree
  expect pass "tree: a name that ends in .mailmap" "$files$(entry 100644 blob docs.mailmap)" tree
  expect fail "tree: a .mailmap (S-031)" "$files$(entry 100644 blob .mailmap)" tree
  expect fail "tree: a .mailmap in capitals" "$files$(entry 100644 blob .MailMap)" tree
  expect fail "tree: a .mailmap with a trailing dot" "$files$(entry 100644 blob .mailmap.)" tree
  # As ls-tree quotes a name with a byte above DEL, and as it writes one with
  # core.quotePath off: U+200C, invisible, with .mailmap.
  expect fail "tree: a quoted .mailmap" "$files$(entry 100644 blob '".mailmap\342\200\214"')" tree
  expect fail "tree: a quoted .mailmap after an invisible character" "$files$(entry 100644 blob '"\342\200\214.mailmap"')" tree
  expect fail "tree: a .mailmap after an invisible character" "$files$(entry 100644 blob $'\342\200\214.mailmap')" tree

  # --- objects git writes ----------------------------------------------------

  if [ "$real" = 1 ]; then
    r="$work/real"
    expect_file pass "git: a signed commit" "$r/signed" commit
    expect_file pass "git: an unsigned commit (S-026)" "$r/unsigned" commit
    expect_file pass "git: a merge" "$r/merge" commit
    expect_file pass "git: a signed tag" "$r/signed-tag" tag v9.9.9
    expect_file pass "git: an unsigned tag (S-026)" "$r/unsigned-tag" tag v9.9.8
    expect_file pass "git: a signed-off message" "$r/signed-message" message
    expect_file fail "git: a merge of someone's signed tag (mergetag)" "$r/mergetag" commit
    expect_file fail "git: another encoding" "$r/encoding" commit
    expect_file fail "git: a commit not in UTC" "$r/not-utc" commit
    expect_file fail "git: a tag of a tag (S-017)" "$r/tag-of-tag" tag v9.9.7
    expect_file fail "git: a tag with a second message line" "$r/two-line-tag" tag v9.9.6
    expect_file fail "git: a tag by someone else" "$r/other-tagger" tag side-tag
    expect_file fail "git: a co-author trailer" "$r/co-author-message" message
    expect_file fail "git: a co-author in prose (S-013)" "$r/co-author-in-prose" message
    expect_file fail "git: a folded reviewer line (S-028)" "$r/folded-message" message
    expect_file fail "git: a reviewer line folded with a CR (W-035)" "$r/cr-folded-message" message
    expect_file fail "git: a commit signed with an SSH certificate (S-030)" "$r/cert-signed" commit
    expect_file fail "git: a tag signed with an SSH certificate (S-030)" "$r/cert-tag" tag v9.9.5
    expect_file pass "git: a tree" "$r/tree" tree
    expect_file fail "git: a tree with a .mailmap (S-031)" "$r/mailmap-tree" tree
  fi

  # --- usage ---------------------------------------------------------------

  expect fail "usage: no kind" "$good"
  expect fail "usage: a tag without its name" "$tgood" tag
  expect fail "usage: a commit with an argument" "$good" commit x
  expect fail "usage: a tag with two names" "$tgood" tag $v $v
  expect fail "usage: a message with an argument" "$m" message x
  expect fail "usage: a tree with an argument" "$files" tree x

}

# --- every awk -------------------------------------------------------------

# Each awk found runs every case once, through a directory on PATH that holds
# it as `awk`. Two names for the same awk (nawk and gawk, say) run once.
seen="|"
runs=0
for a in mawk gawk original-awk nawk busybox awk; do
  command -v "$a" >/dev/null 2>&1 || continue
  if [ -n "${CHECKS_AWK-}" ] && [ "$a" != "$CHECKS_AWK" ]; then continue; fi
  shim="$work/awk-$a"
  mkdir "$shim"
  if [ "$a" = busybox ]; then
    busybox awk 'BEGIN { exit 0 }' >/dev/null 2>&1 || continue
    printf '#!/bin/sh\nexec busybox awk "$@"\n' >"$shim/awk"
    chmod +x "$shim/awk"
  else
    ln -s "$(command -v "$a")" "$shim/awk"
  fi
  id=$("$shim/awk" --version </dev/null 2>&1 | head -n 1 || true)
  id="${id:-$a}"
  case "$seen" in *"|$id|"*) continue ;; esac
  seen="$seen$id|"
  awk_name="$a"
  before=$failures
  PATH="$shim:$PATH" run_cases
  runs=$((runs + 1))
  echo "$a ($id): $((failures - before)) wrong"
done
if [ "$runs" -eq 0 ]; then
  echo "test.sh: no awk found" >&2
  exit 1
fi

echo "$cases cases under $runs awk(s), $failures wrong"
[ "$failures" -eq 0 ]
