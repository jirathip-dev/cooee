# cmds DIR: DIR/mod.rs, a `mod` line per command file and the table `ALL`, one entry per file in name order.
cmds() {
  dir=$1
  {
    for m in $(modules "$dir"); do printf 'mod %s;\n' "$m"; done
    printf '\nuse crate::cli::Command;\n\npub(crate) const ALL: &[Command] = &[\n'
    for m in $(modules "$dir"); do
      printf '    Command {\n        name: "%s",\n        usage: %s::USAGE,\n        flags: %s::FLAGS,\n        run: %s::run,\n    },\n' "$m" "$m" "$m" "$m"
    done
    printf '];\n'
  } >"$dir/mod.rs.gen"
  mv "$dir/mod.rs.gen" "$dir/mod.rs"
}
