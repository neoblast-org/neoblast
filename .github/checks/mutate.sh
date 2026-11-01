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
# - removing the NUL translation for tags: it matters only under an awk
#   (BSD awk stops a line at a NUL) that the run may not have;
# - removing LC_ALL=C where neither tr nor this run's awk reads a byte
#   above DEL differently in a UTF-8 locale (GNU's tr with mawk, as in CI):
#   no case then gives another answer. Where tr refuses one (macOS's), the
#   Latin-1 case fails; where the awk reads such bytes as characters (macOS
#   awk) or refuses the byte range (gawk), the UTF-8 tree cases do; there it
#   is a mutant below (W-036, W-040);
# - message.awk reading a value from one byte before or after its part's
#   start: a value is cut at its first colon, the byte before a part is a
#   CR, and its first is never a colon (W-042);
# - the workflows' own lines around object.sh and signed.sh (an `if !`
#   dropped, say, or which allowed-signers file the signature step reads):
#   test.sh runs the checks, not the workflows;
# - signed.sh's stdout alone left open: git's verify-commit and verify-tag
#   print only to stderr, which a mutant above opens.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# One awk is enough here: every mutant but the locale one is caught by cases
# that do not depend on the awk, and test.sh itself runs them all.
for a in mawk gawk original-awk nawk awk; do
  if command -v "$a" >/dev/null 2>&1; then
    CHECKS_AWK="$a"
    break
  fi
done
export CHECKS_AWK
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# test.sh's UTF-8 locale, found once here so the locale mutant below runs
# under the one its cases use (W-040), and git's objects, made by the first
# run of test.sh and read by the others (W-043).
CHECKS_UTF8=$(locale -a 2>/dev/null | grep -i -E '^(C|en_US)\.utf-?8$' | head -n 1 || true)
export CHECKS_UTF8
mkdir "$work/real"
export CHECKS_REAL="$work/real"
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

# --- commit-object.awk (S-022, S-025, S-030) -------------------------------

c=commit-object.awk
mutant "commit: any header" $c $'{ bad = 1 }\nEND' $'{ }\nEND'
mutant "commit: a name after a parent's hash" $c '/^(tree|parent) [0-9a-f]+$/' '/^(tree|parent) [0-9a-f]+/'
mutant "commit: a parent line inside another header" $c '/^(tree|parent) ' '/(tree|parent) '
mutant "commit: any signature start" $c '$0 == "gpgsig -----BEGIN SSH SIGNATURE-----"' '/^gpgsig /'
mutant "commit: any signature end" $c 'sig && $0 == " -----END SSH SIGNATURE-----"' 'sig && /END/'
mutant "commit: a signature not checked (S-030)" $c 'if (!ssh_signature_ok(b)) bad = 1; ' ''
mutant "commit: an author line inside another header" $c '/^(author|committer) ' '/(author|committer) '
mutant "commit: any author" $c 'committer) neoblast maintainers <dev@neoblast\.dev> [0-9]+' 'committer) .* [0-9]+'
mutant "commit: any time zone" $c '[0-9]+ \+0000$/' '[0-9]+ [+-][0-9]+$/'
mutant "commit: two signatures" $c ' && !s++ { sig = 1' ' { sig = 1'
mutant "commit: any line in a signature" $c 'sig { bad = 1; sig = 0 }' 'sig { next }'
mutant "commit: no message separator needed" $c 'bad || !body || sig' 'bad || sig'
mutant "commit: control bytes in a message (S-025)" $c 'body && /[\001-\010\013-\037\177]/ { bad = 1 }' ''
mutant "commit: backspace in a message" $c '\001-\010' '\001-\007'
mutant "commit: vertical tab in a message" $c '\013-\037' '\016-\037'
mutant "commit: DEL in a message" $c '\037\177]' '\037]'
mutant "commit: bytes above DEL in a message" $c '\037\177]' '\037\177-\377]'

# --- tag-object.awk (S-015, S-017, S-022, S-030) ---------------------------

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
mutant "tag: a signature not checked (S-030)" $t 'if (!ssh_signature_ok(b)) bad = 1; ' ''
mutant "tag: lines after the signature" $t '{ part = 3; if' '{ part = 1; if'
mutant "tag: a signature that never ends" $t ' || part == 2 }' ' }'

# --- message.awk (S-013, S-018, S-028, W-035) ------------------------------

g=message.awk
mutant "message: only an ASCII hyphen (S-018)" $g '[^a-z0-9 \t:]by' '-by'
mutant "message: case matters" $g 'tolower(part[i]) ~' 'part[i] ~'
mutant "message: no indented lines" $g '/^[ \t]*[^ \t:]*' '/^[^ \t:]*'
mutant "message: no space before the colon" $g 'by[ \t]*:/' 'by:/'
mutant "message: any value" $g $'      if (v !~ /^neoblast maintainers <dev@neoblast\\.dev>[ \\t\\r]*$/) exit 1\n' ''
mutant "message: a prefix of the project" $g '<dev@neoblast\.dev>[ \t\r]*$/' '<dev@neoblast\.dev>/'
mutant "message: trailing blanks count" $g '<dev@neoblast\.dev>[ \t\r]*$/' '<dev@neoblast\.dev>$/'
mutant "message: a trailing CR counts" $g '<dev@neoblast\.dev>[ \t\r]*$/' '<dev@neoblast\.dev>[ \t]*$/'
mutant "message: any project-like name" $g 'neoblast maintainers <dev@neoblast\.dev>' 'neoblast maintainers <dev@neoblast.dev>'
mutant "message: a value ends at a CR (W-035)" $g 'v = substr($0, off)' 'v = part[i]'
mutant "message: a value from the line's start (W-042)" $g $'    off += length(part[i]) + 1\n' ''
mutant "message: an offset a byte short (W-042)" $g 'off += length(part[i]) + 1' 'off += length(part[i])'
mutant "message: an offset a byte long (W-042)" $g 'off += length(part[i]) + 1' 'off += length(part[i]) + 2'
mutant "message: folded lines (S-028)" $g 'by && /^[ \t\r]+[^ \t\r]/ { exit 1 }' ''
mutant "message: a fold after any line" $g $'  by = 0\n' ''
mutant "message: only a space folds" $g '/^[ \t\r]+[^ \t\r]/' '/^ +[^ \t\r]/'
mutant "message: a line of blanks folds" $g '/^[ \t\r]+[^ \t\r]/' '/^[ \t\r]+/'
mutant "message: a line of blanks and a CR folds" $g '/^[ \t\r]+[^ \t\r]/' '/^[ \t\r]+[^ \t]/'
mutant "message: a CR does not fold (W-035)" $g '/^[ \t\r]+[^ \t\r]/' '/^[ \t]+[^ \t]/'
mutant "message: a lone CR does not end a line (S-018)" $g 'split($0, part, "\r")' 'split($0, part, "\n")'

# --- ssh-signature.awk (S-030) ---------------------------------------------

v=ssh-signature.awk
mutant "signature: any length" $v 'length(s) == 232 &&' '1 &&'
mutant "signature: any key type (S-030)" $v 'substr(s, 1, 44) == "U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg" &&' '1 &&'
mutant "signature: any reserved field (S-030)" $v 'substr(s, 89, 56) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA" &&' '1 &&'
mutant "signature: any characters" $v 's ~ /^[A-Za-z0-9+\/]+=$/' '1'
mutant "signature: any last character" $v '+=$/' '+.$/'
mutant "signature: any magic (W-038)" $v 'substr(s, 1, 44) == "U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg"' 'substr(s, 9, 36) == "AAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg"'
mutant "signature: any version" $v 'substr(s, 1, 44) == "U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg"' 'substr(s, 1, 8) == "U1NIU0lH" && substr(s, 17, 28) == "ADMAAAALc3NoLWVkMjU1MTkAAAAg"'
mutant "signature: any hash" $v 'substr(s, 89, 56) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA"' 'substr(s, 89, 18) == "AAADZ2l0AAAAAAAAAA"'
mutant "signature: any namespace (W-038)" $v 'substr(s, 89, 56) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA"' 'substr(s, 89, 4) == "AAAD" && substr(s, 97, 48) == "AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA"'
mutant "signature: the namespace length's high byte, low bits (S-034)" $v ' && substr(s, 88, 1) == "A"' ''
mutant "signature: the namespace length's high byte, high bits (S-034)" $v 'index("AEIMQUYcgkosw048", substr(s, 87, 1)) && ' ''
mutant "signature: the signature length's low byte, high bits (S-034)" $v 'substr(s, 145, 1) == "Q" && ' ''
mutant "signature: the signature length's low byte, low bits (S-034)" $v ' && index("ABCDEFGHIJKLMNOP", substr(s, 146, 1))' ''
mutant "signature: the namespace length's high byte, its top bit (S-034)" $v 'index("AEIMQUYcgkosw048", substr(s, 87, 1))' 'index("ACEGIKMOQSUWYacegikmoqsuwy02468+", substr(s, 87, 1))'
mutant "signature: the namespace length's high byte, its second bit (S-034)" $v 'index("AEIMQUYcgkosw048", substr(s, 87, 1))' 'index("ABEFIJMNQRUVYZcdghklopstwx014589", substr(s, 87, 1))'
mutant "signature: the signature length's low byte, its second bit (S-034)" $v 'index("ABCDEFGHIJKLMNOP", substr(s, 146, 1))' 'index("ABCDEFGHIJKLMNOPghijklmnopqrstuv", substr(s, 146, 1))'
mutant "signature: the signature length's low byte, its low bit (S-034)" $v 'index("ABCDEFGHIJKLMNOP", substr(s, 146, 1))' 'index("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef", substr(s, 146, 1))'
mutant "signature: one padding bit (S-034)" $v 'index("AEIMQUYcgkosw048", substr(s, 231, 1))' 'index("ACEGIKMOQSUWYacegikmoqsuwy02468+", substr(s, 231, 1))'
mutant "signature: the other padding bit (S-034)" $v 'index("AEIMQUYcgkosw048", substr(s, 231, 1))' 'index("ABEFIJMNQRUVYZcdghklopstwx014589", substr(s, 231, 1))'
mutant "signature: any padding bits (S-034)" $v 'index("AEIMQUYcgkosw048", substr(s, 231, 1)) &&' '1 &&'
mutant "signature: any signature type" $v 'substr(s, 89, 56) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAA"' 'substr(s, 89, 32) == "AAADZ2l0AAAAAAAAAAZzaGE1MTIAAABT"'

# --- tree.awk (S-031) ------------------------------------------------------

r=tree.awk
mutant "tree: a .mailmap (S-031)" $r 'n ~ /^"?\.mailmap/ { bad = 1 }' ''
mutant "tree: case matters" $r 'tolower($2)' '$2'
mutant "tree: octal escapes kept" $r 'gsub(/\\[0-7][0-7][0-7]/, "", n)' ''
mutant "tree: bytes above DEL kept" $r 'gsub(/[\200-\377]/, "", n)' ''
mutant "tree: only the name .mailmap" $r '/^"?\.mailmap/' '/^"?\.mailmap$/'
mutant "tree: quoted names pass" $r '/^"?\.mailmap/' '/^\.mailmap/'
mutant "tree: .mailmap anywhere in a name" $r '/^"?\.mailmap/' '/\.mailmap/'
mutant "tree: fields split at blanks" $r 'BEGIN { FS = "\t" }' ''
mutant "tree: dot-names outside ASCII pass" $r $'n ~ /^"?\\./ && $2 ~ /\\\\[0-7][0-7][0-7]|[\\200-\\377]/ { bad = 1 }\n' ''
mutant "tree: quoted dot-names outside ASCII pass" $r '/\\[0-7][0-7][0-7]|[\200-\377]/ { bad' '/[\200-\377]/ { bad'
mutant "tree: raw dot-names outside ASCII pass" $r '/\\[0-7][0-7][0-7]|[\200-\377]/ { bad' '/\\[0-7][0-7][0-7]/ { bad'
mutant "tree: any name outside ASCII" $r 'n ~ /^"?\./ && ' ''
mutant "tree: the dot looked for before bytes outside ASCII are left out" $r 'n ~ /^"?\./ && ' '$2 ~ /^"?\./ && '
mutant "tree: quoted dot-names pass" $r 'n ~ /^"?\./ && ' 'n ~ /^\./ && '

# --- identity.awk (S-008, W-034) -------------------------------------------

i=identity.awk
mutant "identity: any author" $i '$1 != me || ' ''
mutant "identity: any committer" $i '$2 != me || ' ''
mutant "identity: the author in any time zone" $i '$3 != "+0000" || ' ''
mutant "identity: the committer in any time zone" $i '$4 != "+0000" || ' ''
mutant "identity: any trailers" $i ' || tr != ""' ''
mutant "identity: the project's trailers kept" $i 'gsub(/neoblast maintainers <dev@neoblast\.dev>/, "", tr); ' ''
mutant "identity: any project-like trailer" $i 'gsub(/neoblast maintainers <dev@neoblast\.dev>/' 'gsub(/neoblast maintainers <dev@neoblast.dev>/'
mutant "identity: separators kept" $i 'gsub(/[; ]/, "", tr)' 'tr = tr'
mutant "identity: spaces kept" $i 'gsub(/[; ]/, "", tr)' 'gsub(/;/, "", tr)'
mutant "identity: more fields" $i 'NF != 5 {' 'NF < 5 {'
mutant "identity: fewer fields" $i 'NF != 5 {' 'NF > 5 {'
mutant "identity: more lines" $i '|| NR != 1 }' '|| NR < 1 }'
mutant "identity: no line" $i '|| NR != 1 }' '|| NR > 1 }'
mutant "identity: fields split at blanks" $i 'FS = "\t"; ' ''

# --- utc.awk (S-008, W-034) ------------------------------------------------

u=utc.awk
mutant "utc: every author" $u '$1 == "dev@neoblast.dev" && ' ''
mutant "utc: GitHub's commits too" $u '$2 != "noreply@github.com" && ' ''
mutant "utc: any address holding GitHub's" $u '$2 != "noreply@github.com"' '$2 !~ /noreply@github\.com/'
mutant "utc: the author in any time zone" $u '$3 != "+0000" || ' ''
mutant "utc: the committer in any time zone" $u ' || $4 != "+0000"' ''
mutant "utc: more fields" $u 'NF != 4 {' 'NF < 4 {'
mutant "utc: fewer fields" $u 'NF != 4 {' 'NF > 4 {'
mutant "utc: more lines" $u '|| NR != 1 }' '|| NR < 1 }'
mutant "utc: no line" $u '|| NR != 1 }' '|| NR > 1 }'
mutant "utc: fields split at blanks" $u 'BEGIN { FS = "\t" }' ''

# --- object.sh -------------------------------------------------------------

o=object.sh
mutant "object.sh: NULs reach the commit check" $o "commit:1) tr '\\000' '\\001' | " 'commit:1) '
mutant "object.sh: commit takes any arguments" $o 'commit:1)' 'commit:*)'
mutant "object.sh: tag takes any arguments" $o 'tag:2)' 'tag:*)'
mutant "object.sh: message takes any arguments" $o 'message:1)' 'message:*)'
mutant "object.sh: tree takes any arguments" $o 'tree:1)' 'tree:*)'
mutant "object.sh: identity takes any arguments" $o 'identity:1)' 'identity:*)'
mutant "object.sh: utc takes any arguments" $o 'utc:1)' 'utc:*)'

if [ -n "$CHECKS_UTF8" ] && {
  ! printf '\351' | LC_ALL="$CHECKS_UTF8" tr a a >/dev/null 2>&1 ||
    ! printf '\342\200\214x\n' | LC_ALL="$CHECKS_UTF8" "$CHECKS_AWK" '{ gsub(/[\200-\377]/, "") } length($0) != 1 { exit 1 }' >/dev/null 2>&1
}; then
  mutant "object.sh: the caller's locale (W-036, W-040)" $o 'export LC_ALL=C' ':'
fi

# --- signed.sh (S-026) -----------------------------------------------------

s=signed.sh
mutant "signed: any signature passes" $s $'fi\nexit 1' $'fi\nexit 0'
mutant "signed: any object passes" $s '"$verify" --end-of-options "$3"' 'cat-file -e "$3"'
mutant "signed: a revision read as an option" $s '--end-of-options ' ''
mutant "signed: a configured ssh program" $s '-c gpg.ssh.program=ssh-keygen ' ''
mutant "signed: a configured allowed-signers file" $s '-c gpg.ssh.allowedSignersFile="$1" ' ''
mutant "signed: a configured GnuPG" $s '-c gpg.openpgp.program=false ' ''
mutant "signed: a configured gpgsm" $s '-c gpg.x509.program=false' ''
mutant "signed: git's output printed (S-012)" $s ' >/dev/null 2>&1; then' '; then'
mutant "signed: a commit verified as a tag" $s 'verify=verify-commit' 'verify=verify-tag'
mutant "signed: a tag verified as a commit" $s 'verify=verify-tag' 'verify=verify-commit'
mutant "signed: commit takes any arguments" $s 'commit:3)' 'commit:*)'
mutant "signed: tag takes any arguments" $s 'tag:3)' 'tag:*)'
mutant "signed: a usage error exits 1" $s 'exit 2' 'exit 1'

echo "$mutants mutants, $survived survived or stale"
[ "$survived" -eq 0 ]
