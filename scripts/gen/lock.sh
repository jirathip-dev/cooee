# lock: Cargo.lock, the workspace's own crates brought up to date.
lock() {
  cargo update --workspace --quiet
}
