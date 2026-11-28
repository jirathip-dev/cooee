# today: README.md's list between its markers, from the `## Today` bullets of docs/*.md and docs/*/README.md in file-name order, their ../ links made the README's.
today() {
  for f in $(printf '%s\n' docs/*.md docs/*/README.md | LC_ALL=C sort); do
    awk '/^## / { on = $0 == "## Today"; next } on && /^- / { gsub(/\]\((\.\.\/)+/, "]("); print }' "$f"
  done >README.md.today
  awk -v f=README.md.today '/^<!-- \/today -->$/ { while ((getline l < f) > 0) print l; skip = 0 } !skip { print } /^<!-- today:/ { skip = 1 }' \
    README.md >README.md.gen
  mv README.md.gen README.md
  rm README.md.today
}
