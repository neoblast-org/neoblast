# A commit object as stored (git cat-file commit), with every NUL already
# turned into \001 by object.sh. Exits 0 only if the commit holds nothing but
# what the project allows (S-022):
# - only tree, parent, author and committer headers, the author and committer
#   being the project in UTC, and at most one gpgsig header, an SSH-armored
#   block laid out as a plain ed25519 key's signature (ssh-signature.awk,
#   S-030);
# - a message without control bytes but a tab (S-025): git reads a message
#   only up to its first NUL, so the message checks would not see the rest.
# Nothing here verifies the signature (S-026).
body && /[\001-\010\013-\037\177]/ { bad = 1 }
body { next }
sig && $0 == " -----END SSH SIGNATURE-----" { sig = 0; if (!ssh_signature_ok(b)) bad = 1; next }
sig && /^ / { b = b substr($0, 2); next }
sig { bad = 1; sig = 0 }
/^$/ { body = 1; next }
/^(tree|parent) [0-9a-f]+$/ { next }
/^(author|committer) neoblast maintainers <dev@neoblast\.dev> [0-9]+ \+0000$/ { next }
$0 == "gpgsig -----BEGIN SSH SIGNATURE-----" && !s++ { sig = 1; b = ""; next }
{ bad = 1 }
END { exit bad || !body || sig }
