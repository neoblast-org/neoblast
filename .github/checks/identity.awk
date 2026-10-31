# One commit as the hygiene job reads it, git log -1 --date=format:%z
# --format='%an <%ae>%x09%cn <%ce>%x09%ad%x09%cd%x09%(trailers:key=Co-authored-by,key=Signed-off-by,valueonly,unfold,separator=%x3B)'.
# Exits 0 only if the project authored and committed it, both in UTC, and
# every Co-authored-by and Signed-off-by trailer names the project (S-008):
# with the project's name and address left out, a trailer value may hold
# only spaces and the semicolons between values. Only one line of exactly
# five fields passes, so a tab in a trailer cannot push a name past the
# fields read (W-034).
BEGIN { FS = "\t"; me = "neoblast maintainers <dev@neoblast.dev>" }
NF != 5 { bad = 1 }
{ tr = $5; gsub(/neoblast maintainers <dev@neoblast\.dev>/, "", tr); gsub(/[; ]/, "", tr) }
$1 != me || $2 != me || $3 != "+0000" || $4 != "+0000" || tr != "" { bad = 1 }
END { exit bad || NR != 1 }
