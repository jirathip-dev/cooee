# arch: .drover/arch.toml (#729): every `frozen`, `generated` and `shared` line of .drover/arch/*.list, sorted, then each *.toml in name order.
arch() {
  {
    printf '# Written by scripts/gen.sh from .drover/arch/ (docs/architecture/hotspots-2026-11-16). Never edit it.\n'
    for k in frozen generated shared; do
      printf '%s = [%s]\n' "$k" "$(awk -v k="$k" '$1 == k { print $2 }' .drover/arch/*.list | LC_ALL=C sort -u | awk '{ printf "%s\"%s\"", (NR > 1 ? ", " : ""), $0 }')"
    done
    printf 'generate = "scripts/gen.sh"\n'
    for f in .drover/arch/*.toml; do
      printf '\n'
      cat "$f"
    done
  } >.drover/arch.toml.gen
  mv .drover/arch.toml.gen .drover/arch.toml
}
