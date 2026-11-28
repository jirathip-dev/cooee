# The modules of a folder: each `<name>.rs` and each `<name>/mod.rs`, once, in C-locale order.
modules() {
  for p in "$1"/*.rs "$1"/*/mod.rs; do
    [ -e "$p" ] || continue
    p=${p#"$1"/}
    case $p in lib.rs | main.rs | mod.rs) ;; */mod.rs) echo "${p%/mod.rs}" ;; *) echo "${p%.rs}" ;; esac
  done | LC_ALL=C sort -u
}
