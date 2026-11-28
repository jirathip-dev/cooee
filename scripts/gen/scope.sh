# scope: README.md's Scope and Non-goals between their markers, from docs/scope.md's `## ` sections, its ../ links made the README's.
scope() {
  awk '/^## / { on = 1 } on { gsub(/\]\((\.\.\/)+/, "]("); print }' docs/scope.md >README.md.scope
  awk -v f=README.md.scope '/^<!-- \/scope -->$/ { while ((getline l < f) > 0) print l; skip = 0 } !skip { print } /^<!-- scope:/ { skip = 1 }' \
    README.md >README.md.gen
  mv README.md.gen README.md
  rm README.md.scope
}
