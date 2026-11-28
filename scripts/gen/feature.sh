# feature DIR NAME (#743): DIR/../NAME+Areas.swift; each `<Prefix><Area>.swift` of DIR adds its State, Action, reducer or scope.
feature() {
  dir=$1 name=$2 states='' cases='' reducers=''
  for f in "$dir"/*.swift; do
    [ -e "$f" ] || continue
    a=${f##*/} && a=${a%.swift} && a=${a#"${name%Feature}"}
    l=$(printf %s "$a" | cut -c1 | tr '[:upper:]' '[:lower:]')$(printf %s "$a" | cut -c2-)
    if grep -q "struct ${a}State\b" "$f"; then states="$states        public var $l = ${a}State()\n"; fi
    if grep -q "enum ${a}Action\b" "$f"; then cases="$cases        case $l(${a}Action)\n"; fi
    if grep -q "var ${l}Scope\b" "$f"; then reducers="$reducers        ${l}Scope\n"; fi
    if grep -qF "func $l(_ state: inout State, _ action: Action)" "$f"; then
      reducers="$reducers        Reduce { state, action in $l(&state, action) }\n"
    fi
  done
  [ -n "$reducers" ] || reducers='        EmptyReducer()\n'
  {
    printf 'import ComposableArchitecture\n\nextension %s {\n    @ObservableState\n    public struct Areas: Equatable' "$name"
    if [ -n "$states" ]; then printf ' {\n%b    }\n' "$states"; else printf ' {}\n'; fi
    printf '\n    @CasePathable\n    public enum AreaAction: Sendable'
    if [ -n "$cases" ]; then printf ' {\n%b    }\n' "$cases"; else printf ' {}\n'; fi
    printf '\n    @ReducerBuilder<State, Action>\n    internal var areas: some ReducerOf<Self> {\n%b    }\n}\n' "$reducers"
  } >"${dir%/*}/$name+Areas.swift.gen"
  mv "${dir%/*}/$name+Areas.swift.gen" "${dir%/*}/$name+Areas.swift"
}
