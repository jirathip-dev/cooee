# areas DIR (#729): DIR/areas.rs, a field per file of DIR with a `pub(super) struct State`, built by its `open(seed)` (`?` when it returns a Result) or its default.
areas() {
  dir=$1 ms=
  for m in $(modules "$dir"); do
    if grep -q '^pub(super) struct State ' "$dir/$m.rs" 2>/dev/null; then ms="$ms $m"; fi
  done
  {
    printf 'use super::{HubError, Seed%s};\n\n' "$(for m in $ms; do printf ', %s' "$m"; done)"
    printf 'pub(super) struct Areas {\n'
    for m in $ms; do printf '    pub(super) %s: %s::State,\n' "$m" "$m"; done
    printf '}\n\nimpl Areas {\n'
    printf "    pub(super) fn open(seed: &Seed<'_>) -> Result<Self, HubError> {\n        Ok(Self {\n"
    for m in $ms; do
      if grep -q "fn open(seed: &Seed<'_>) -> Result<Self, HubError>" "$dir/$m.rs"; then
        printf '            %s: %s::State::open(seed)?,\n' "$m" "$m"
      elif grep -q "fn open(seed: &Seed<'_>) -> Self" "$dir/$m.rs"; then
        printf '            %s: %s::State::open(seed),\n' "$m" "$m"
      else
        printf '            %s: %s::State::default(),\n' "$m" "$m"
      fi
    done
    printf '        })\n    }\n}\n'
  } | rustfmt --edition 2024 >"$dir/areas.rs.gen"
  mv "$dir/areas.rs.gen" "$dir/areas.rs"
}
