# A commit's top-level tree as git ls-tree prints it, "<mode> <type>
# <hash>\t<name>". Exits 0 only if no name starts with .mailmap, in any
# case, quoted or not, once every byte outside ASCII, as ls-tree writes it
# in octal or as it is, and every other octal escape, is left out
# (S-031): git log, shortlog and blame show
# the names and addresses that file maps the project's to, though the
# identity step reads the commits' own. A file system that folds case, or
# ignores an invisible character or a trailing dot or space, could read
# such a name as .mailmap. Nor may a name that starts with a dot, once
# those bytes are left out, hold a byte outside ASCII, so no file system's
# folding of another letter, such as the dotless i in .maılmap, need be
# known, nor whether it ignores an invisible character before the dot.
# So é.md fails too, and a dot-name with a backslash before three digits,
# as ls-tree writes it "\\101": the check fails closed, and the project
# has no such name.
BEGIN { FS = "\t" }
{
  n = tolower($2)
  gsub(/\\[0-7][0-7][0-7]/, "", n)
  gsub(/[\200-\377]/, "", n)
}
n ~ /^"?\.mailmap/ { bad = 1 }
n ~ /^"?\./ && $2 ~ /\\[0-7][0-7][0-7]|[\200-\377]/ { bad = 1 }
END { exit bad }
