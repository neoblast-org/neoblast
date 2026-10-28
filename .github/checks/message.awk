# A commit message (git log --format=%B), read as git splits it: at each
# newline only.
# Exits 0 only if every "<key>-by:" line names the project (S-013), wherever
# it is in the message, not only in git's trailer block. A carriage return
# also ends a line here, as a terminal shows it (S-018), though a value
# runs on to the end of git's line, CRs and all. The key is matched
# without regard to case, and any byte but a letter, digit, space, tab or
# colon counts as its hyphen, so a Unicode one cannot hide a line (S-018)
# while "Standby:" and the like still pass. Git reads a line that starts
# with a space, tab or carriage return, right after a trailer, as part of
# it, so such a line after a "<key>-by:" line fails too, wherever it is
# (S-028, W-035). Lines git does not read as trailers (quoted, bulleted,
# "Reviewed by:") are not checked.
BEGIN { me = "neoblast maintainers <dev@neoblast.dev>" }
by && /^[ \t\r]+[^ \t\r]/ { found = 1 }
{
  by = 0
  n = split($0, part, "\r")
  for (i = 1; i <= n; i++) {
    if (tolower(part[i]) !~ /^[ \t]*[^ \t:]*[^a-z0-9 \t:]by[ \t]*:/) continue
    by = 1
    v = part[i]
    for (j = i + 1; j <= n; j++) v = v "\r" part[j]
    sub(/^[^:]*:[ \t]*/, "", v); sub(/[ \t\r]+$/, "", v)
    if (v != me) found = 1
  }
}
END { exit found }
