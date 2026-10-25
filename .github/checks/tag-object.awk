# A tag object as stored (git cat-file tag), with every NUL already turned
# into \001 by object.sh, and the tag's name in `name`. Exits 0 only if the
# tag holds nothing but what the project allows (S-015, S-017, S-022):
# - exactly one each of object, `type commit` (not a tag of a tag), `tag
#   <name>` and the project's tagger in UTC, and no other header;
# - as message, only the tag's name, then at most an SSH-armored block whose
#   lines hold only base64.
# Nothing here verifies the signature (S-026).
part == 0 && /^object [0-9a-f]+$/ && !o++ { next }
part == 0 && $0 == "type commit" && !t++ { next }
part == 0 && $0 == "tag " name && !n++ { next }
part == 0 && /^tagger neoblast maintainers <dev@neoblast\.dev> [0-9]+ \+0000$/ && !g++ { next }
part == 0 && $0 == "" { part = 1; next }
part == 1 && m == 1 && $0 == "-----BEGIN SSH SIGNATURE-----" { part = 2; next }
part == 1 && !m++ && $0 == name { next }
part == 2 && /^[A-Za-z0-9+\/=]+$/ { next }
part == 2 && $0 == "-----END SSH SIGNATURE-----" { part = 3; next }
{ bad = 1 }
END { exit bad || !(o && t && n && g) || m != 1 || part == 2 }
