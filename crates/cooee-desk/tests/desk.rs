//! The desk provider against a fake `container` CLI that keeps each desk's status in a file beside it: the lifecycle,
//! the caps, the host-only network and the reach probe that stops a desk reaching the LAN or the tailnet.

use std::{
    env, fs,
    net::SocketAddr,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::OnceLock,
};

use cooee_desk::{
    Caps, Desk, Desks, Error, MAX_CPUS, MAX_MEMORY_MIB, MIN_MEMORY_MIB, NETWORK, Name, PROVIDER,
    State,
};

const CONTAINER: &str = r#"#!/bin/sh
d=$(dirname "$0")
printf '%s\n' "$*" >>"$d/calls.log"
case "$1 $2" in
"network ls") cat "$d/networks.json" 2>/dev/null || echo '[{"id":"default","configuration":{"mode":"nat"}}]' ;;
"network create")
  mode=nat; [ "$3" = --internal ] && mode=hostOnly
  printf '[{"id":"%s","configuration":{"mode":"%s"}}]' "$4" "$mode" >"$d/networks.json" ;;
"ls --all")
  printf '['; sep=
  for f in "$d"/c/*; do
    [ -e "$f" ] || continue
    n=$(basename "$f"); l=
    [ -e "$d/l/$n" ] && l=$(sed 's/\(.*\)=\(.*\)/"\1":"\2"/' "$d/l/$n")
    printf '%s{"configuration":{"id":"%s","labels":{%s}},"status":"%s"}' "$sep" "$n" "$l" "$(cat "$f")"; sep=,
  done
  printf ']' ;;
"create --name") echo stopped >"$d/c/$3"; mkdir -p "$d/l"; [ "$4" = --label ] && printf '%s' "$5" >"$d/l/$3" ;;
"start "*) [ -e "$d/c/$2" ] || { echo "no such container" >&2; exit 1; }; echo running >"$d/c/$2" ;;
"stop "*) echo stopped >"$d/c/$2" ;;
"delete --force") rm "$d/c/$3" ;;
"exec "*)
  eval probe=\${$#}
  [ -s "$d/open" ] || exit 1
  case "$probe" in *"$(cat "$d/open")") exit 0 ;; esac
  exit 1 ;;
esac
"#;

fn container() -> &'static PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let bin = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cooee-desk-container");
        fs::write(&bin, CONTAINER).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        bin
    })
}

struct Fake {
    dir: PathBuf,
}

impl Fake {
    fn new(name: &str) -> Self {
        let dir = cooee_core::scratch::owned(
            env::temp_dir().join(format!("cooee-desk-{name}-{}", std::process::id())),
        );
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(dir.join("c")).unwrap();
        symlink(container(), dir.join("container")).unwrap();
        Self { dir }
    }

    fn desks(&self, blocked: &[&str]) -> Desks {
        let blocked = blocked
            .iter()
            .map(|a| a.parse::<SocketAddr>().unwrap())
            .collect();
        Desks::new(self.dir.join("container"), blocked)
    }

    fn calls(&self, prefix: &str) -> Vec<String> {
        let log = fs::read_to_string(self.dir.join("calls.log")).unwrap_or_default();
        log.lines()
            .filter(|l| l.starts_with(prefix))
            .map(str::to_owned)
            .collect()
    }
}

fn caps() -> Caps {
    Caps::new(2, 2048).unwrap()
}

#[test]
fn a_desk_is_created_woken_put_to_sleep_and_destroyed_and_each_step_repeats_safely() {
    let fake = Fake::new("life");
    let desks = fake.desks(&[]);
    let desk = Name::new("research-1").unwrap();
    assert_eq!(desks.state(&desk).unwrap(), State::Gone);

    desks.create(&desk, "linuxserver/webtop", caps()).unwrap();
    desks.create(&desk, "linuxserver/webtop", caps()).unwrap();
    assert_eq!(desks.state(&desk).unwrap(), State::Asleep);
    assert_eq!(fake.calls("create ").len(), 1);

    desks.wake(&desk).unwrap();
    assert_eq!(desks.state(&desk).unwrap(), State::Awake);

    desks.sleep(&desk).unwrap();
    desks.sleep(&desk).unwrap();
    assert_eq!(desks.state(&desk).unwrap(), State::Asleep);
    assert_eq!(fake.calls("stop ").len(), 1);

    desks.destroy(&desk).unwrap();
    desks.destroy(&desk).unwrap();
    assert_eq!(desks.state(&desk).unwrap(), State::Gone);
    assert_eq!(fake.calls("delete ").len(), 1);

    let err = desks.wake(&desk).unwrap_err();
    assert!(
        matches!(err, Error::Cli { ref why, .. } if why == "no such container"),
        "{err}"
    );
}

#[test]
fn a_desk_gets_its_caps_and_caps_outside_the_limits_are_refused() {
    for (cpus, mib) in [
        (0, 2048),
        (MAX_CPUS + 1, 2048),
        (1, MIN_MEMORY_MIB - 1),
        (1, MAX_MEMORY_MIB + 1),
    ] {
        assert!(
            matches!(Caps::new(cpus, mib), Err(Error::Caps)),
            "{cpus} CPUs, {mib} MiB"
        );
    }
    Caps::new(1, MIN_MEMORY_MIB).unwrap();
    Caps::new(MAX_CPUS, MAX_MEMORY_MIB).unwrap();

    let fake = Fake::new("caps");
    fake.desks(&[])
        .create(&Name::new("capped").unwrap(), "img", caps())
        .unwrap();
    assert_eq!(
        fake.calls("create "),
        [format!(
            "create --name capped --label cooee.provider=container --cpus 2 --memory 2048M --network {NETWORK} img"
        )]
    );
}

#[test]
fn desks_join_only_a_host_only_network_and_refuse_one_that_is_not() {
    let fake = Fake::new("net");
    let desks = fake.desks(&[]);
    desks
        .create(&Name::new("a").unwrap(), "img", caps())
        .unwrap();
    desks
        .create(&Name::new("b").unwrap(), "img", caps())
        .unwrap();
    assert_eq!(
        fake.calls("network create"),
        [format!("network create --internal {NETWORK}")]
    );

    let nat = Fake::new("nat");
    let networks = format!(r#"[{{"id":"{NETWORK}","configuration":{{"mode":"nat"}}}}]"#);
    fs::write(nat.dir.join("networks.json"), networks).unwrap();
    let err = nat
        .desks(&[])
        .create(&Name::new("a").unwrap(), "img", caps())
        .unwrap_err();
    assert!(matches!(err, Error::NotHostOnly), "{err}");
    assert_eq!(nat.calls("create "), Vec::<String>::new());
}

#[test]
fn a_desk_that_reaches_the_lan_or_the_tailnet_is_stopped_and_one_that_does_not_stays_awake() {
    let blocked = ["203.0.113.7:80", "198.51.100.7:22"];
    for (open, target) in [
        ("203.0.113.7/80", blocked[0]),
        ("198.51.100.7/22", blocked[1]),
    ] {
        let fake = Fake::new("reach");
        let desks = fake.desks(&blocked);
        let desk = Name::new("leaky").unwrap();
        desks.create(&desk, "img", caps()).unwrap();
        fs::write(fake.dir.join("open"), open).unwrap();

        let err = desks.wake(&desk).unwrap_err();
        assert!(
            matches!(err, Error::Reached { target: t, .. } if t.to_string() == target),
            "{err}"
        );
        assert_eq!(desks.state(&desk).unwrap(), State::Asleep);
    }

    let fake = Fake::new("sealed");
    let desks = fake.desks(&blocked);
    let desk = Name::new("sealed").unwrap();
    desks.create(&desk, "img", caps()).unwrap();
    desks.wake(&desk).unwrap();
    assert_eq!(desks.state(&desk).unwrap(), State::Awake);
    assert_eq!(fake.calls("exec ").len(), 2);
}

#[test]
fn only_the_desks_the_provider_created_are_listed_with_their_provider() {
    let fake = Fake::new("made");
    let desks = fake.desks(&[]);
    fs::write(fake.dir.join("c/by-hand"), "running").unwrap();
    let desk = Name::new("made-1").unwrap();
    desks.create(&desk, "img", caps()).unwrap();
    desks.wake(&desk).unwrap();
    assert_eq!(
        desks.made().unwrap(),
        [Desk {
            name: "made-1".to_owned(),
            provider: PROVIDER.to_owned(),
            state: State::Awake,
        }]
    );
}

#[test]
fn a_name_that_could_be_read_as_a_flag_or_path_is_refused() {
    for bad in ["", "-rm", "Desk", "a/b", "1desk", &"a".repeat(41)] {
        assert!(matches!(Name::new(bad), Err(Error::Name(_))), "{bad:?}");
    }
}
