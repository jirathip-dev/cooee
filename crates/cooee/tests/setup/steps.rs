use std::{fs, net::TcpListener};

use cooee_core::SHED_MIN_PROTOCOL;

use crate::setup::{LAUNCHD, Mac, files, job_file, load_line, terminal};

fn assert_jobs_keep_alive_and_repeat(mac: &Mac) {
    let (keep, every) = if LAUNCHD {
        (
            "<key>KeepAlive</key>\n  <true/>",
            "<key>StartInterval</key>\n  <integer>900</integer>",
        )
    } else {
        ("Restart=always", "OnUnitActiveSec=900")
    };
    let serve = fs::read_to_string(job_file(&mac.home, "dev.cooee.cooee", "service")).unwrap();
    assert!(serve.contains(keep), "{serve}");
    let timer = job_file(&mac.home, "dev.cooee.cooee.update", "timer");
    let timer = fs::read_to_string(timer).unwrap();
    assert!(timer.contains(every), "{timer}");
}

fn assert_second_run_skips_every_step(mac: &Mac, chief: &str) {
    let port = mac.port;
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(mac.changes(), Vec::<String>::new());
    for step in [
        &format!("shed protocol {SHED_MIN_PROTOCOL}+"),
        &format!("config {}", mac.config().display()),
        "agents",
        "chief",
        "cooee service dev.cooee.cooee",
        "cooee service dev.cooee.cooee.update",
        &format!("tailscale serve {port}"),
    ] {
        assert!(
            stdout.contains(&format!("{step}: done, skipped\n")),
            "{step} not skipped in {stdout}"
        );
    }
    assert!(
        stdout.contains("pair your phone:\nType the link") && stdout.contains("\ncode  "),
        "{stdout}"
    );
    assert_eq!(fs::read_to_string(mac.chief()).unwrap(), chief);
}

#[test]
fn a_fresh_machine_runs_every_step_once_in_order_and_a_second_run_skips_them() {
    let mac = Mac::new("su-fresh", true);
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let port = mac.port;
    assert_eq!(
        mac.changes(),
        [
            "shed agents add claude-acp".to_owned(),
            load_line(&mac.home, "dev.cooee.cooee", "service"),
            load_line(&mac.home, "dev.cooee.cooee.update", "timer"),
            format!("tailscale serve --bg --https={port} http://127.0.0.1:{port}"),
        ]
    );
    let ignore = fs::read_to_string(mac.config().with_file_name(".gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "secrets/"), "{ignore}");
    let config = fs::read_to_string(mac.config()).unwrap();
    assert_eq!(
        config,
        format!(
            "port = {port}\nallowed_hosts = [\"mac.tailnet.example\"]\nroots = [\"~/Projects\"]\n{}",
            terminal(port, &config)
        )
    );
    assert_jobs_keep_alive_and_repeat(&mac);
    let chief = fs::read_to_string(mac.chief()).unwrap();
    assert!(
        chief.contains("role = \"chief\"") && chief.contains("agent = \"claude-acp\""),
        "{chief}"
    );
    assert!(
        stdout.contains("doctor:\nok   service dev.cooee.cooee loaded")
            && stdout.contains(&format!(
                "link  https://mac.tailnet.example:{port}/pair?code="
            )),
        "{stdout}"
    );

    assert_second_run_skips_every_step(&mac, &chief);
}

#[test]
fn yes_with_no_chief_and_no_picks_fails_before_any_step_and_changes_nothing() {
    let mac = Mac::new("su-chief-yes", true);
    let before = files(&mac.home);
    let out = mac.cooee(&["setup", "--port", &mac.port.to_string(), "--yes"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("there is no Chief yet and --yes can't pick for you: add --chief-agent <id> --chief-machine <name|here>\nnothing was changed"),
        "{stderr}"
    );
    assert_eq!(files(&mac.home), before);
    assert_eq!(mac.changes(), Vec::<String>::new());
}

#[test]
fn dry_run_writes_nothing_and_prints_every_command() {
    let mac = Mac::new("su-dry", true);
    let mut want = files(&mac.home);
    let out = mac.setup(&["--dry-run", "--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(mac.changes(), Vec::<String>::new());
    want.push("calls.log".to_owned());
    want.sort();
    assert_eq!(files(&mac.home), want);
    let port = mac.port;
    for line in [
        format!("would write {}:\nport = {port}\n", mac.config().display()),
        "would run: shed agents add claude-acp".to_owned(),
        format!(
            "would write {}",
            job_file(&mac.home, "dev.cooee.cooee", "service").display()
        ),
        format!("would run: tailscale serve --bg --https={port} http://127.0.0.1:{port}"),
        "would run: cooee doctor, then cooee pair".to_owned(),
    ] {
        assert!(stdout.contains(&line), "{line} not in {stdout}");
    }
}

#[test]
fn a_tailnet_url_answering_405_on_get_pair_passes_doctor() {
    let mac = Mac::new("su-405", true);
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(
        mac.home.join("tailnet"),
        server.local_addr().unwrap().to_string(),
    )
    .unwrap();
    std::thread::spawn(move || {
        for mut conn in server.incoming().flatten() {
            let mut buf = [0; 4096];
            std::io::Read::read(&mut conn, &mut buf).unwrap();
            std::io::Write::write_all(
                &mut conn,
                b"HTTP/1.1 405 Method Not Allowed\r\nallow: POST\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            )
            .unwrap();
        }
    });
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let port = mac.port;
    assert!(
        stdout.contains(&format!(
            "ok   https://mac.tailnet.example:{port}/ (else: tailscale serve --bg --https={port} http://127.0.0.1:{port})"
        )),
        "{stdout}"
    );
}

#[test]
fn a_doctor_failure_stops_with_its_fix() {
    let mac = Mac::new("su-doctor", true);
    let unused = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    fs::write(mac.home.join("tailnet"), unused.to_string()).unwrap();
    let out = mac.setup(&["--yes"]);
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let port = mac.port;
    let fail = format!(
        "FAIL https://mac.tailnet.example:{port}/ (else: tailscale serve --bg --https={port} http://127.0.0.1:{port})"
    );
    assert!(
        stderr.contains(&format!("doctor: 1 check(s) failed:\n{fail}\n")),
        "{stderr}"
    );
    assert!(!stdout.contains("code  "), "{stdout}");
}

#[test]
fn an_existing_config_keeps_its_keys() {
    let mac = Mac::new("su-keep", true);
    let old = format!(
        "port = {}\nroots = [\"/srv/code\"]\ntheme = \"farm\"\n\n[[plugins]]\nname = \"board\"\nlabel = \"Board\"\n\
         url = \"http://127.0.0.1:1\"\nmodule = \"/tab.js\"\ntoken = \"0123456789abcdef\"\n",
        mac.port
    );
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(mac.config(), &old).unwrap();
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    let config = fs::read_to_string(mac.config()).unwrap();
    let old = format!("{old}{}", terminal(mac.port, &config));
    assert_eq!(
        config,
        format!("allowed_hosts = [\"mac.tailnet.example\"]\n{old}")
    );
    assert!(
        stdout.contains("adding allowed_hosts = [\"mac.tailnet.example\"], the terminal plugin\n"),
        "{stdout}"
    );

    let out = mac.cooee(&["setup", "--yes", "--port", "1"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(&format!("says port = {}", mac.port)),
        "{out:?}"
    );
    assert_eq!(
        fs::read_to_string(mac.config()).unwrap(),
        format!("allowed_hosts = [\"mac.tailnet.example\"]\n{old}")
    );
}

#[test]
fn setup_on_a_mac_whose_app_ran_the_server_reuses_its_config_and_data() {
    let mac = Mac::new("su-app", true);
    for item in ["com.jirathip.cooee.serve", "com.jirathip.cooee.shed"] {
        fs::write(mac.home.join(format!("loaded-{item}")), "").unwrap();
    }
    let kept = format!(
        "roots = [\"~/Projects\"]\nport = {}\nallowed_hosts = [\"mac.tailnet.example\"]\n",
        mac.port
    );
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(
        mac.config(),
        format!("shed = \"/Applications/Cooee.app/Contents/Helpers/shed\"\n{kept}"),
    )
    .unwrap();
    fs::create_dir_all(mac.data()).unwrap();
    let hash = "ab".repeat(32);
    let devices = format!(
        r#"{{"devices":[{{"id":"abababab","label":"iPhone","hash":"{hash}","created":1,"used":1}}]}}"#
    );
    fs::write(mac.data().join("devices.json"), devices).unwrap();
    let out = mac.setup(&["--yes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{out:?}");
    assert!(
        stdout.contains("adding the terminal plugin, shed from PATH, not the old Mac app's\n"),
        "{stdout}"
    );
    let config = fs::read_to_string(mac.config()).unwrap();
    assert_eq!(config, format!("{kept}{}", terminal(mac.port, &config)));
    let listed = mac.cooee(&["devices"]);
    let listed = String::from_utf8_lossy(&listed.stdout);
    assert!(
        listed.contains("abababab") && listed.contains("iPhone"),
        "{listed}"
    );
    assert!(mac.home.join("loaded-dev.cooee.cooee").exists());
    assert!(mac.setup(&["--yes"]).status.success());
    assert_eq!(fs::read_to_string(mac.config()).unwrap(), config);
}

#[test]
fn doctor_warns_when_both_the_cli_and_the_mac_app_are_installed() {
    let mac = Mac::new("su-both", true);
    let doctor = |mac: &Mac| {
        let out = mac.cooee(&["doctor", "--config", &mac.config().display().to_string()]);
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    fs::create_dir_all(mac.config().parent().unwrap()).unwrap();
    fs::write(mac.config(), format!("port = {}\n", mac.port)).unwrap();
    fs::write(mac.home.join("loaded-dev.cooee.cooee"), "").unwrap();
    assert!(!doctor(&mac).contains("warn both"));
    fs::write(mac.home.join("loaded-com.jirathip.cooee.serve"), "").unwrap();
    let stdout = doctor(&mac);
    assert!(
        stdout.contains(
            "ok   service dev.cooee.cooee loaded (else: cooee install)\nwarn both dev.cooee.cooee and the Mac \
             app's login items (com.jirathip.cooee.serve) are installed: two servers on one Mac\n"
        ),
        "{stdout}"
    );
}
