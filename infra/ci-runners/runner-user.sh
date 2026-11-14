#!/bin/sh
# #368: a standard macOS user that only the CI runner uses, so apple.yml's mac-setup job (login items, :4747,
# ~/.config/cooee, the simulator) runs on the mini without touching the owner's live cooee. Idempotent.
# Usage: sudo infra/ci-runners/runner-user.sh [--dry-run] [--remove]   (see runner-user.md)
set -eu

repo=jirathip-dev/cooee
ci_user=cooeeci
runner_name=mini-cooee-setup
runner_label=cooee-mac-setup
kc_service=cooee-ci-runner
agent_label=com.jirathip.cooee.ci-runner
system_kc=/Library/Keychains/System.keychain
ci_home=/Users/$ci_user
runner_dir=$ci_home/actions-runner
agent_plist=$ci_home/Library/LaunchAgents/$agent_label.plist
ci_path=$ci_home/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin

dry=0
remove=0
for arg in "$@"; do
  case $arg in
    --dry-run) dry=1 ;;
    --remove) remove=1 ;;
    *) echo "usage: sudo $0 [--dry-run] [--remove]" >&2; exit 2 ;;
  esac
done
[ "$dry" = 1 ] || [ "$(id -u)" = 0 ] || { echo "run it with sudo (or --dry-run)" >&2; exit 2; }
owner=${SUDO_USER:-$(id -un)}

say() { printf '%s\n' "$*"; }
run() {
  say "+ $*"
  [ "$dry" = 1 ] || "$@"
}
as_owner() { if [ "$(id -u)" = 0 ]; then sudo -u "$owner" -H "$@"; else "$@"; fi; }
as_ci() { sudo -u "$ci_user" -H "$@"; }
user_exists() { id -u "$ci_user" >/dev/null 2>&1; }
gui_domain() { user_exists && launchctl print "gui/$(id -u "$ci_user")" >/dev/null 2>&1; }
runner_token() { as_owner gh api -X POST "repos/$repo/actions/runners/$1-token" -q .token; }
# Runs the runner's config.sh as the CI user with the token from stdin, so it is never printed nor in argv.
# shellcheck disable=SC2016
ci_config() { as_ci sh -c 'cd "$0" && ACTIONS_RUNNER_INPUT_TOKEN=$(cat) exec ./config.sh "$@"' "$runner_dir" "$@"; }

# Writes stdin to $1 (owned by the CI user, mode $2) only when the content differs.
put_file() {
  new=$(cat)
  if [ -f "$1" ] && [ "$(cat "$1")" = "$new" ]; then say "ok: $1 unchanged"; return 0; fi
  say "+ write $1"
  [ "$dry" = 1 ] && return 0
  as_ci mkdir -p "$(dirname "$1")"
  printf '%s\n' "$new" | as_ci tee "$1" >/dev/null
  chmod "$2" "$1"
}

remove_all() {
  if gui_domain; then run launchctl bootout "gui/$(id -u "$ci_user")"; else say "ok: no $ci_user session"; fi
  if [ -f "$runner_dir/.runner" ]; then
    say "+ unregister $runner_name from $repo (token fetched by gh, not printed)"
    [ "$dry" = 1 ] || runner_token remove | ci_config remove
  else
    say "ok: no runner registered from $runner_dir"
  fi
  if user_exists; then run sysadminctl -deleteUser "$ci_user"; else say "ok: no user $ci_user"; fi
  if security find-generic-password -s "$kc_service" "$system_kc" >/dev/null 2>&1; then
    run security delete-generic-password -s "$kc_service" "$system_kc"
  else
    say "ok: no $kc_service password in the System keychain"
  fi
}

create_user() {
  if user_exists; then
    say "ok: user $ci_user exists"
    security find-generic-password -s "$kc_service" "$system_kc" >/dev/null 2>&1 ||
      say "warn: no $kc_service password in the System keychain; run --remove, then this again"
    return 0
  fi
  say "+ generate a password, keep it in $system_kc as $kc_service"
  say "+ sysadminctl -addUser $ci_user (standard, not admin); createhomedir -c -u $ci_user"
  [ "$dry" = 1 ] && return 0
  pw=$(openssl rand -base64 24)
  security add-generic-password -U -a "$ci_user" -s "$kc_service" -w "$pw" "$system_kc"
  sysadminctl -addUser "$ci_user" -fullName "cooee CI runner" -password "$pw" -home "$ci_home"
  createhomedir -c -u "$ci_user" >/dev/null
}

developer_group() {
  if dseditgroup -o checkmember -m "$ci_user" _developer >/dev/null 2>&1; then
    say "ok: $ci_user in _developer"
  else
    run dseditgroup -o edit -a "$ci_user" -t user _developer
  fi
}

fast_user_switching() {
  if [ "$(defaults read /Library/Preferences/.GlobalPreferences MultipleSessionEnabled 2>/dev/null)" = 1 ]; then
    say "ok: fast user switching on"
  else
    run defaults write /Library/Preferences/.GlobalPreferences MultipleSessionEnabled -bool true
  fi
}

check_xcode() {
  dev=$(xcode-select -p)
  case $dev in
    /Applications/Xcode.app/*) say "ok: shared Xcode at $dev (xcode-select unchanged)" ;;
    *) say "warn: xcode-select points at $dev, not /Applications/Xcode.app; left unchanged" ;;
  esac
}

rustup_home() {
  if [ -x "$ci_home/.cargo/bin/rustup" ]; then say "ok: rustup in $ci_home/.cargo"; return 0; fi
  say "+ as $ci_user: rustup-init -y --no-modify-path --profile minimal --default-toolchain none"
  [ "$dry" = 1 ] && return 0
  as_ci sh -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none'
}

install_runner() {
  if [ -x "$runner_dir/config.sh" ]; then say "ok: actions-runner in $runner_dir"; return 0; fi
  say "+ download the latest actions-runner (osx-arm64), check its sha256, unpack in $runner_dir as $ci_user"
  [ "$dry" = 1 ] && return 0
  asset=$(as_owner gh api repos/actions/runner/releases/latest \
    -q '.assets[] | select(.name | test("osx-arm64-.*tar.gz$")) | .browser_download_url + " " + .digest')
  tarball=$ci_home/actions-runner.tar.gz
  as_ci curl -sSfL -o "$tarball" "${asset%% *}"
  echo "${asset##*sha256:}  $tarball" | shasum -a 256 -c - >/dev/null
  as_ci mkdir -p "$runner_dir"
  as_ci tar -xzf "$tarball" -C "$runner_dir"
  rm -f "$tarball"
}

register_runner() {
  if [ -f "$runner_dir/.runner" ]; then say "ok: $runner_name registered"; return 0; fi
  say "+ register $runner_name on $repo, labels self-hosted + $runner_label (token fetched by gh, not printed)"
  [ "$dry" = 1 ] && return 0
  runner_token registration | ci_config --unattended --url "https://github.com/$repo" --name "$runner_name" \
    --labels "$runner_label" --work _work --replace
}

launch_agent() {
  put_file "$runner_dir/.path" 644 <<EOF
$ci_path
EOF
  put_file "$agent_plist" 644 <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$agent_label</string>
  <key>ProgramArguments</key><array><string>$runner_dir/run.sh</string></array>
  <key>WorkingDirectory</key><string>$runner_dir</string>
  <key>EnvironmentVariables</key><dict><key>PATH</key><string>$ci_path</string></dict>
  <key>LimitLoadToSessionType</key><string>Aqua</string>
  <key>ProcessType</key><string>Interactive</string>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>$ci_home/Library/Logs/cooee-ci-runner.log</string>
  <key>StandardErrorPath</key><string>$ci_home/Library/Logs/cooee-ci-runner.log</string>
</dict>
</plist>
EOF
  if ! gui_domain; then
    say "ok: $ci_user has no GUI session yet; the agent starts when it logs in (runner-user.md)"
  elif launchctl print "gui/$(id -u "$ci_user")/$agent_label" >/dev/null 2>&1; then
    say "ok: $agent_label loaded"
  else
    run launchctl bootstrap "gui/$(id -u "$ci_user")" "$agent_plist"
  fi
}

[ "$dry" = 1 ] && say "dry run: nothing changes"
if [ "$remove" = 1 ]; then remove_all; exit 0; fi
create_user
developer_group
fast_user_switching
check_xcode
rustup_home
install_runner
register_runner
launch_agent
