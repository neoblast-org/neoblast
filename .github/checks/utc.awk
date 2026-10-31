# One commit as the hygiene job reads it, git log -1 --date=format:%z
# --format='%ae%x09%ce%x09%ad%x09%cd'. Exits 0 unless the project authored
# it, someone other than GitHub committed it, and either time is not in
# UTC: commits GitHub makes itself (the pull-request test merge, squash
# merges) carry the account's time zone, so they are skipped. Only one line
# of exactly four fields, split at tabs, passes (W-034): an address with a
# | in it no longer shifts the fields, as it did when the step split at |.
BEGIN { FS = "\t" }
NF != 4 { bad = 1 }
$1 == "dev@neoblast.dev" && $2 != "noreply@github.com" && ($3 != "+0000" || $4 != "+0000") { bad = 1 }
END { exit bad || NR != 1 }
