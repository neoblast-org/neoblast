#!/usr/bin/env bash
# Checks that test.sh would catch the checks breaking: each mutant below
# brings back a weakness a review closed, in a copy of this directory, and
# test.sh must then fail. The hygiene job runs it after test.sh.
#
#   .github/checks/mutate.sh
#
# A mutant replaces one exact piece of text, which must occur once in its
# file: if a change to a check moves that text, this fails until the mutant
# is updated, so no mutant silently stops applying. Add one with each new
# rule in a check. Works with the bash 3.2 that ships with macOS.
#
# Not here, as no test can tell them apart from the checks as they are:
# - removing `sig` from commit-object.awk's END: a signature that never ends
#   also leaves no message, which `!body` already fails;
# - dropping `m == 1` from the rule that starts a tag's signature: a tag
#   whose signature comes first never counts its name, which `m != 1` fails;
# - removing LC_ALL=C or the NUL translation for tags: they matter only
#   under a locale or an awk (BSD awk stops a line at a NUL) that the run
#   may not have (test.sh's Latin-1 case catches the first on macOS);
# - the workflows' own lines around object.sh (an `if !` dropped, say):
#   test.sh runs object.sh, not the workflows.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# One awk is enough here: every mutant is caught by cases that do not depend
# on the awk, and test.sh itself runs them all.
for a in mawk gawk original-awk nawk awk; do
  if command -v "$a" >/dev/null 2>&1; then
    CHECKS_AWK="$a"
    break
  fi
done
export CHECKS_AWK
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mutants=0
survived=0

# The checks as they are must pass first: otherwise a test.sh that fails for
# another reason (no ssh-keygen, a broken git) would kill every mutant.
if ! "$here/test.sh" >/dev/null 2>&1; then
  echo "mutate.sh: test.sh fails on the checks as they are; run it to see why" >&2
  exit 1
fi

mutant() { # label file old new
  local label="$1" file="$2" old="$3" new="$4" text rest
  mutants=$((mutants + 1))
  rm -rf "$work/checks"
  cp -R "$here" "$work/checks"
  text="$(cat "$here/$file")"
  rest="${text#*"$old"}"
  if [ "$rest" = "$text" ]; then
    echo "STALE $label: the text to replace is not in $file"
    survived=$((survived + 1))
    return
  fi
  case "$rest" in
    *"$old"*)
      echo "STALE $label: the text to replace is in $file more than once"
      survived=$((survived + 1))
      return
      ;;
  esac
  printf '%s\n' "${text%%"$old"*}$new$rest" >"$work/checks/$file"
  if "$work/checks/test.sh" >/dev/null 2>&1; then
    echo "SURVIVED $label"
    survived=$((survived + 1))
  fi
}

# --- commit-object.awk (S-022, S-025) --------------------------------------

c=commit-object.awk
mutant "commit: any header" $c $'{ bad = 1 }\nEND' $'{ }\nEND'
mutant "commit: a name after a parent's hash" $c '/^(tree|parent) [0-9a-f]+$/' '/^(tree|parent) [0-9a-f]+/'
mutant "commit: a parent line inside another header" $c '/^(tree|parent) ' '/(tree|parent) '
mutant "commit: any signature start" $c '$0 == "gpgsig -----BEGIN SSH SIGNATURE-----"' '/^gpgsig /'
mutant "commit: any signature end" $c 'sig && $0 == " -----END SSH SIGNATURE-----"' 'sig && /END/'
mutant "commit: an author line inside another header" $c '/^(author|committer) ' '/(author|committer) '
mutant "commit: any author" $c 'committer) neoblast maintainers <dev@neoblast\.dev> [0-9]+' 'committer) .* [0-9]+'
mutant "commit: any time zone" $c '[0-9]+ \+0000$/' '[0-9]+ [+-][0-9]+$/'
mutant "commit: two signatures" $c ' && !s++ { sig = 1' ' { sig = 1'
mutant "commit: any line in a signature" $c 'sig { bad = 1; sig = 0 }' 'sig { next }'
mutant "commit: any word in a signature" $c '/^ [A-Za-z0-9+\/=]+$/' '/^ [^ ]+$/'
mutant "commit: no message separator needed" $c 'bad || !body || sig' 'bad || sig'
mutant "commit: control bytes in a message (S-025)" $c 'body && /[\001-\010\013-\037\177]/ { bad = 1 }' ''
mutant "commit: backspace in a message" $c '\001-\010' '\001-\007'
mutant "commit: vertical tab in a message" $c '\013-\037' '\016-\037'
mutant "commit: DEL in a message" $c '\037\177]' '\037]'
mutant "commit: bytes above DEL in a message" $c '\037\177]' '\037\177-\377]'

# --- tag-object.awk (S-015, S-017, S-022) ----------------------------------

t=tag-object.awk
mutant "tag: any header" $t $'{ bad = 1 }\nEND' $'{ }\nEND'
mutant "tag: two object lines" $t '!o++' '++o'
mutant "tag: two type lines" $t '!t++' '++t'
mutant "tag: two tag headers" $t '!n++' '++n'
mutant "tag: two tagger lines" $t '!g++' '++g'
mutant "tag: an object line inside another header" $t '/^object ' '/object '
mutant "tag: a tagger line inside another header" $t '/^tagger ' '/tagger '
mutant "tag: any signature start" $t '$0 == "-----BEGIN SSH SIGNATURE-----"' '/BEGIN/'
mutant "tag: any signature end" $t '$0 == "-----END SSH SIGNATURE-----"' '/END/'
mutant "tag: a name after the object's hash" $t '/^object [0-9a-f]+$/' '/^object [0-9a-f]+/'
mutant "tag: any type (S-017)" $t '$0 == "type commit"' '/^type /'
mutant "tag: any tag header" $t '$0 == "tag " name' '/^tag /'
mutant "tag: a name after the tag's name" $t '$0 == "tag " name' 'index($0, "tag " name) == 1'
mutant "tag: any tagger" $t 'tagger neoblast maintainers <dev@neoblast\.dev> [0-9]+ \+0000$/' 'tagger /'
mutant "tag: any time zone" $t '[0-9]+ \+0000$/' '[0-9]+ [+-][0-9]+$/'
mutant "tag: a header may be missing" $t '!(o && t && n && g) || ' ''
mutant "tag: no message needed" $t ' || m != 1' ''
mutant "tag: the name twice" $t '!m++ && $0 == name ""' '$0 == name "" && (m = 1)'
mutant "tag: a message other than the name" $t '!m++ && $0 == name ""' '!m++'
mutant "tag: the name compared as a number" $t '$0 == name ""' '$0 == name'
mutant "tag: any word in a signature" $t '/^[A-Za-z0-9+\/=]+$/' '/^[^ ]+$/'
mutant "tag: lines after the signature" $t '{ part = 3; next }' '{ part = 1; next }'
mutant "tag: a signature that never ends" $t ' || part == 2 }' ' }'

# --- message.awk (S-013, S-018, S-028) ------------------------------------

g=message.awk
mutant "message: only an ASCII hyphen (S-018)" $g '[^a-z0-9 \t:]by' '-by'
mutant "message: case matters" $g 'tolower($0) ~' '$0 ~'
mutant "message: no indented lines" $g '/^[ \t]*[^ \t:]*' '/^[^ \t:]*'
mutant "message: no space before the colon" $g 'by[ \t]*:/' 'by:/'
mutant "message: any value" $g 'if (v != me) found = 1' 'found = 0'
mutant "message: a prefix of the project" $g 'if (v != me)' 'if (index(v, me) != 1)'
mutant "message: trailing blanks count" $g 'sub(/[ \t\r]+$/, "", v)' 'v = v'
mutant "message: folded lines (S-028)" $g 'by && /^[ \t]+[^ \t]/ { found = 1 }' ''
mutant "message: a fold after any line" $g '{ by = 0 }' '{ }'
mutant "message: only a space folds" $g '/^[ \t]+[^ \t]/' '/^ +[^ \t]/'
mutant "message: a line of blanks folds" $g '/^[ \t]+[^ \t]/' '/^[ \t]+/'

# --- object.sh -------------------------------------------------------------

o=object.sh
mutant "object.sh: NULs reach the commit check" $o "commit:1) tr '\\000' '\\001' | " 'commit:1) '
mutant "object.sh: a lone CR does not end a line (S-018)" $o "| tr '\\r' '\\n' | " '| '
mutant "object.sh: a CR before a newline is kept (S-028)" $o "message:1) LC_ALL=C awk '{ sub(/\\r\$/, \"\") } 1' | " 'message:1) '
mutant "object.sh: commit takes any arguments" $o 'commit:1)' 'commit:*)'
mutant "object.sh: tag takes any arguments" $o 'tag:2)' 'tag:*)'
mutant "object.sh: message takes any arguments" $o 'message:1)' 'message:*)'

echo "$mutants mutants, $survived survived or stale"
[ "$survived" -eq 0 ]
