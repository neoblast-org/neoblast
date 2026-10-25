# A commit message (git log --format=%B), with every carriage return already
# turned into a newline by object.sh, so a lone one also ends a line (S-018).
# Exits 0 only if every "<key>-by:" line names the project (S-013), wherever
# it is in the message, not only in git's trailer block. The key is matched
# without regard to case, and any byte but a letter, digit, space, tab or
# colon counts as its hyphen, so a Unicode one cannot hide a line (S-018)
# while "Standby:" and the like still pass. Lines git does not read as
# trailers (quoted, bulleted, "Reviewed by:") are not checked.
BEGIN { me = "neoblast maintainers <dev@neoblast.dev>" }
tolower($0) ~ /^[ \t]*[^ \t:]*[^a-z0-9 \t:]by[ \t]*:/ {
  v = $0; sub(/^[^:]*:[ \t]*/, "", v); sub(/[ \t\r]+$/, "", v)
  if (v != me) found = 1
}
END { exit found }
