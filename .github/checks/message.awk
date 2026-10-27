# A commit message (git log --format=%B), with every carriage return already
# dropped before a newline or turned into one by object.sh, so a lone one
# also ends a line (S-018).
# Exits 0 only if every "<key>-by:" line names the project (S-013), wherever
# it is in the message, not only in git's trailer block. The key is matched
# without regard to case, and any byte but a letter, digit, space, tab or
# colon counts as its hyphen, so a Unicode one cannot hide a line (S-018)
# while "Standby:" and the like still pass. Git reads a line that starts with
# a space or tab, right after a trailer, as part of it, so such a line after
# a "<key>-by:" line fails too (S-028). Lines git does not read as trailers
# (quoted, bulleted, "Reviewed by:") are not checked.
BEGIN { me = "neoblast maintainers <dev@neoblast.dev>" }
by && /^[ \t]+[^ \t]/ { found = 1 }
{ by = 0 }
tolower($0) ~ /^[ \t]*[^ \t:]*[^a-z0-9 \t:]by[ \t]*:/ {
  by = 1
  v = $0; sub(/^[^:]*:[ \t]*/, "", v); sub(/[ \t\r]+$/, "", v)
  if (v != me) found = 1
}
END { exit found }
