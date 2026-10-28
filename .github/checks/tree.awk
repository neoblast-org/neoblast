# A commit's top-level tree as git ls-tree prints it, "<mode> <type>
# <hash>\t<name>". Exits 0 only if no name starts with .mailmap, in any
# case, quoted or not, once every byte outside ASCII is left out, as ls-tree
# writes it in octal or as it is (S-031): git log, shortlog and blame show
# the names and addresses that file maps the project's to, though the
# identity step reads the commits' own. A file system that folds case, or
# ignores an invisible character or a trailing dot or space, could read
# such a name as .mailmap.
BEGIN { FS = "\t" }
{
  n = tolower($2)
  gsub(/\\[0-7][0-7][0-7]/, "", n)
  gsub(/[\200-\377]/, "", n)
}
n ~ /^"?\.mailmap/ { bad = 1 }
END { exit bad }
