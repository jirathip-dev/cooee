use super::*;

const MIN: u64 = 60 * 1000;

#[test]
fn backoff_doubles_and_the_fifth_crash_in_ten_minutes_turns_it_off() {
    let mut b = Backoff::default();
    let waits: Vec<_> = [0, 1, 2, 3, 4].map(|m| b.crash(m * MIN)).into();
    assert_eq!(
        waits,
        [Some(1000), Some(2000), Some(4000), Some(8000), None]
    );
    let mut b = Backoff::default();
    assert!((0..20).all(|i| b.crash(i * 11 * MIN) == Some(1000)));
}

fn installed(test: &str, run: &str, port: u16) -> (PathBuf, PathBuf, Arc<Manager>) {
    let data = cooee_core::scratch::owned(
        std::env::temp_dir().join(format!("cooee-{test}-{}", std::process::id())),
    );
    fs::remove_dir_all(&data).ok();
    let build = data.join("plugins/x/builds/c0");
    fs::create_dir_all(&build).unwrap();
    let manifest = format!(
        "name = \"x\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"{run}\"\n\
         [[tabs]]\nname = \"x\"\nlabel = \"X\"\nmodule = \"/tab.js\"\n"
    );
    fs::write(build.join("cooee-plugin.toml"), manifest).unwrap();
    let tab = Plugin {
        name: "x".into(),
        label: "X".into(),
        url: format!("http://127.0.0.1:{port}"),
        module: "/tab.js".into(),
        token: "0123456789abcdef".into(),
    };
    plugins::write_entries(&data, &[tab]).unwrap();
    let x = Locked {
        name: "x".into(),
        commit: "c0".into(),
        tabs: vec!["x".into()],
        port,
        enabled: true,
        ..Locked::default()
    };
    plugins::write_lock(&data, &[x]).unwrap();
    let config = Config {
        data: Some(data.clone()),
        ..Config::default()
    };
    (data, build, Manager::new(&config).unwrap())
}

fn runs(build: &Path) -> usize {
    fs::read_to_string(build.join("runs"))
        .unwrap_or_default()
        .lines()
        .count()
}

#[test]
fn the_supervisor_restarts_a_hung_backend_after_three_failed_checks() {
    let silent = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = silent.local_addr().unwrap().port();
    let (data, build, m) = installed("hung", "echo ran >> runs; exec sleep 600", port);
    m.tick(0);
    cooee_core::clock::sleep(Duration::from_millis(200));
    for at in [1, 2] {
        m.tick(at * CHECK_EVERY);
        assert_eq!(lock(&m.runs)["x"].state, "running");
    }
    m.tick(3 * CHECK_EVERY);
    assert!(lock(&m.runs)["x"].state.contains("hung"));
    m.tick(3 * CHECK_EVERY + 1000);
    cooee_core::clock::sleep(Duration::from_millis(200));
    assert_eq!(runs(&build), 2);
    assert_eq!(lock(&m.runs)["x"].state, "running");
    let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
    assert!(
        log.contains("the backend stopped: hung, 3 health checks in a row failed"),
        "{log}"
    );
    assert_eq!(log.matches("== cooee: start c0").count(), 2, "{log}");
    m.stop_all();
    fs::remove_dir_all(&data).unwrap();
}

#[test]
fn the_supervisor_never_restarts_a_healthy_backend() {
    let ok = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = ok.local_addr().unwrap().port();
    let served = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = Arc::clone(&served);
    thread::spawn(move || {
        for mut s in ok.incoming().flatten() {
            let mut buf = [0; 1024];
            std::io::Read::read(&mut s, &mut buf).ok();
            s.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n")
                .ok();
            count.fetch_add(1, Ordering::SeqCst);
        }
    });
    let (data, build, m) = installed("healthy", "echo ran >> runs; exec sleep 600", port);
    for at in 0..=60 {
        m.tick(at * CHECK_EVERY);
    }
    cooee_core::clock::sleep(Duration::from_millis(200));
    assert_eq!(served.load(Ordering::SeqCst), 60);
    assert_eq!(runs(&build), 1);
    assert_eq!(lock(&m.runs)["x"].state, "running");
    let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
    assert!(!log.contains("stopped"), "{log}");
    m.stop_all();
    fs::remove_dir_all(&data).unwrap();
}

#[test]
fn a_backend_failing_with_address_in_use_while_its_attach_port_is_up_ends_attached() {
    let attach = free_port().unwrap();
    let run = "echo ran >> runs; echo 'drover: Address already in use (os error 48)'; exit 1";
    let (data, build, m) = installed("eaddrinuse", run, 1);
    let manifest = fs::read_to_string(build.join("cooee-plugin.toml")).unwrap();
    let manifest = format!("attach = \"http://127.0.0.1:{attach}\"\n{manifest}");
    fs::write(build.join("cooee-plugin.toml"), manifest).unwrap();
    m.tick(0);
    assert_eq!(lock(&m.runs)["x"].state, "running");
    let _serve = TcpListener::bind((Ipv4Addr::LOCALHOST, attach)).unwrap();
    for at in 1..100 {
        m.tick(at * 1000);
        if lock(&m.runs)["x"].state != "running" {
            break;
        }
        cooee_core::clock::sleep(Duration::from_millis(20));
    }
    assert_eq!(lock(&m.runs)["x"].state, "attached");
    for at in 100..120 {
        assert_eq!(m.tick(at * 1000), Vec::<String>::new());
    }
    assert_eq!(lock(&m.runs)["x"].state, "attached");
    assert_eq!(runs(&build), 1);
    assert!(m.installed().unwrap()[0].enabled);
    let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
    assert!(!log.contains("the backend stopped:"), "{log}");
    m.stop_all();
    fs::remove_dir_all(&data).unwrap();
}

#[test]
fn the_supervisor_restarts_a_crashing_backend_then_turns_it_off_with_an_alert() {
    let (data, build, m) = installed("supervise", "echo ran >> runs; exit 3", 1);
    let runs = || runs(&build);
    let mut at = 0;
    let mut alerts = Vec::new();
    for _ in 0..200 {
        alerts = m.tick(at);
        if !alerts.is_empty() {
            break;
        }
        let next = lock(&m.runs).get("x").map_or(at, |r| r.next);
        at = next.max(at) + 1;
        cooee_core::clock::sleep(Duration::from_millis(20));
    }
    assert_eq!(runs(), 5, "{alerts:?}");
    assert_eq!(
        alerts,
        ["plugin x: off after 5 crashes in 10 minutes; its log is in Settings › Plugins"]
    );
    let x = &m.installed().unwrap()[0];
    assert!(!x.enabled && x.note.contains("5 crashes"), "{x:?}");
    assert!(m.list().unwrap()[0].state.contains("5 crashes"));
    m.tick(at + 60 * MIN);
    assert_eq!(runs(), 5);
    let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
    assert_eq!(log.matches("the backend stopped").count(), 5, "{log}");
    fs::remove_dir_all(&data).unwrap();
}

#[test]
fn the_supervisor_keeps_a_plugin_off_when_it_or_cooee_is_too_old_for_the_other() {
    for (field, needed) in [
        ("version = \"0.0.9\"", "it needs x 0.1.0 or newer"),
        (
            "version = \"1\"\nmin_cooee = \"99.0.0\"",
            "x needs cooee 99.0.0 or newer",
        ),
    ] {
        let (data, build, m) = installed("unfit", "echo ran >> runs; exec sleep 30", 1);
        let file = build.join("cooee-plugin.toml");
        let manifest = fs::read_to_string(&file).unwrap();
        fs::write(&file, manifest.replace("version = \"1\"", field)).unwrap();
        let alerts = m.tick(0);
        assert!(
            alerts.len() == 1 && alerts[0].contains(needed),
            "{alerts:?}"
        );
        let x = &m.installed().unwrap()[0];
        assert!(!x.enabled && x.note.contains(needed), "{x:?}");
        assert!(m.list().unwrap()[0].state.contains(needed));
        m.tick(60 * MIN);
        assert_eq!(runs(&build), 0);
        fs::remove_dir_all(&data).unwrap();
    }
}

#[test]
fn an_idle_supervisor_reads_the_lock_only_once_it_changed_within_five_seconds() {
    let (data, _, m) = installed("idle", "exec sleep 600", 1);
    m.tick(0);
    assert_eq!(lock(&m.runs)["x"].state, "running");
    let file = plugins::lock_path(&data);
    let was = fs::metadata(&file).unwrap();
    fs::write(&file, "x".repeat(usize::try_from(was.len()).unwrap())).unwrap();
    let f = fs::File::options().write(true).open(&file).unwrap();
    f.set_modified(was.modified().unwrap()).unwrap();
    for at in 1..=20 {
        m.tick(at * 500);
        assert_eq!(lock(&m.runs)["x"].state, "running");
    }
    assert!(m.installed().is_err());
    let x = Locked {
        name: "x".into(),
        commit: "c0".into(),
        tabs: vec!["x".into()],
        port: 1,
        ..Locked::default()
    };
    plugins::write_lock(&data, &[x]).unwrap();
    for at in 21..30 {
        m.tick(at * 500);
        assert!(lock(&m.runs).contains_key("x"), "{at}");
    }
    m.tick(15_000);
    assert!(!lock(&m.runs).contains_key("x"));
    m.enable("x", true).unwrap();
    m.tick(15_001);
    assert_eq!(lock(&m.runs)["x"].state, "running");
    m.stop_all();
    fs::remove_dir_all(&data).unwrap();
}
