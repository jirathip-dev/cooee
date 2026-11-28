# screen DIR NAME FEATURE (#744): DIR/../NAME+Areas.swift, one `.modifier` per `<File>Presenting` view modifier of DIR.
screen() {
  dir=$1 name=$2 feature=$3 mods=''
  for f in "$dir"/*.swift; do
    [ -e "$f" ] || continue
    a=${f##*/} && a=${a%.swift}
    if grep -q "struct ${a}Presenting: ViewModifier" "$f"; then mods="$mods\n            .modifier(${a}Presenting(store: store))"; fi
  done
  {
    printf 'import ComposableArchitecture\nimport CooeeFeatures\nimport SwiftUI\n\n'
    printf 'internal struct %sAreas: ViewModifier {\n    internal let store: StoreOf<%s>\n\n' "$name" "$feature"
    printf '    internal func body(content: Content) -> some View {\n        content%b\n    }\n}\n' "$mods"
  } >"${dir%/*}/$name+Areas.swift.gen"
  mv "${dir%/*}/$name+Areas.swift.gen" "${dir%/*}/$name+Areas.swift"
}
