//! `cooee accounts env` and `accounts ls --json`, the credential helper's contract (docs/accounts.md), with a fake
//! provider whose status reads its folder: a signed-in and a signed-out folder account, unknown ones refused, a
//! Claude token file (0600) handed over and a looser one refused, and no folder's secret ever on stdout or stderr.
//! #183: an existing folder adopted and dropped (the folder stays), a token inherited from the caller never signing a
//! folder in, and usage per account: a fresh reading, and a stored one past its reset at 0 % with its age.

use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");

struct Fixture {
    dir: PathBuf,
    /// A secret-shaped value kept in the signed-in account's folder, built at runtime.
    secret: String,
}

/// Provider `fake` (`FAKE_HOME`; signed in when the folder has `creds`, or when a token is inherited: it must not
/// be; usage from the folder's `usage.txt`), whose accounts are `work` (signed in) and
/// `idle` (signed out), and provider `claude` with token files `ci` (0600) and `loose` (0644).
fn fixture(name: &str) -> Fixture {
    let dir = env::temp_dir().join(format!("cooee-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let accounts = dir.join("data/accounts");
    fs::create_dir_all(accounts.join("fake/idle")).unwrap();
    fs::create_dir_all(accounts.join("fake/work")).unwrap();
    fs::create_dir_all(accounts.join("claude")).unwrap();
    let secret = format!("sk-{}", "q7".repeat(16));
    fs::write(accounts.join("fake/work/creds"), &secret).unwrap();
    let token = |file: &str, mode| {
        let path = accounts.join("claude").join(file);
        fs::write(&path, format!("oat-{file}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    };
    token("ci.token", 0o600);
    token("loose.token", 0o644);
    fs::write(
        dir.join("config.toml"),
        format!(
            "data = {:?}\n\
             [[providers]]\nname = \"fake\"\nbase = \"claude\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\nlogin = [\"true\"]\n\
             status = [\"sh\", \"-c\", \"test -n \\\"$CLAUDE_CODE_OAUTH_TOKEN\\\" || cat \\\"$FAKE_HOME/creds\\\"\"]\n\
             usage = [\"sh\", \"-c\", \"cat \\\"$FAKE_HOME/usage.txt\\\"\"]\n\
             [[providers]]\nname = \"claude\"\nagent = \"claude-acp\"\nhome = \"CLAUDE_CONFIG_DIR\"\n\
             login = [\"true\"]\nstatus = [\"false\"]\n",
            dir.join("data").to_string_lossy()
        ),
    )
    .unwrap();
    Fixture { dir, secret }
}

impl Fixture {
    /// `cooee accounts <args>`: its exit code, stdout and stderr; neither ever holds the folder's secret.
    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(BIN)
            .arg("accounts")
            .args(args)
            .arg("--config")
            .arg(self.dir.join("config.toml"))
            .env("CLAUDE_CODE_OAUTH_TOKEN", "inherited-login")
            .output()
            .unwrap();
        let (stdout, stderr) = (
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        );
        assert!(
            !stdout.contains(&self.secret),
            "a secret on stdout: {stdout}"
        );
        assert!(
            !stderr.contains(&self.secret),
            "a secret on stderr: {stderr}"
        );
        (out.status.code().unwrap(), stdout, stderr)
    }

    fn folder(&self, name: &str) -> PathBuf {
        self.dir.join("data/accounts/fake").join(name)
    }
}

fn parse(stdout: &str) -> Value {
    serde_json::from_str(stdout).unwrap()
}

#[test]
fn env_prints_a_folder_accounts_variable_and_reports_signed_out() {
    let f = fixture("accounts-env");
    let (code, out, err) = f.run(&["env", "fake", "work"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(parse(&out), json!({ "FAKE_HOME": f.folder("work") }));

    let (code, out, err) = f.run(&["env", "fake", "idle"]);
    assert_eq!(code, 0);
    assert_eq!(parse(&out), json!({ "FAKE_HOME": f.folder("idle") }));
    assert_eq!(err, "cooee: fake/idle is signed out\n");

    let (code, out, err) = f.run(&["env", "fake", "idle", "--require-signed-in"]);
    assert_eq!((code, out.as_str()), (3, ""));
    assert_eq!(err, "cooee: fake/idle is signed out\n");
    assert_eq!(f.run(&["env", "fake", "work", "--require-signed-in"]).0, 0);

    // The system default: no folder, no variable.
    assert_eq!(parse(&f.run(&["env", "fake", ""]).1), json!({}));
}

#[test]
fn unknown_provider_or_account_refuses_on_one_line() {
    let f = fixture("accounts-unknown");
    let (code, out, err) = f.run(&["env", "nope", "work"]);
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (1, "", "cooee: no provider `nope`\n")
    );
    let (code, out, err) = f.run(&["env", "fake", "gone"]);
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (1, "", "cooee: no account `gone` of provider `fake`\n")
    );
    assert_eq!(f.run(&["env", "fake", "../fake/work"]).0, 1);
    assert_eq!(f.run(&["env", "fake"]).0, 2);
}

#[test]
fn a_token_file_goes_in_the_token_variable_only_at_0600_or_tighter() {
    let f = fixture("accounts-token");
    let (code, out, _) = f.run(&["env", "claude", "ci"]);
    assert_eq!(code, 0);
    assert_eq!(
        parse(&out),
        json!({ "CLAUDE_CODE_OAUTH_TOKEN": "oat-ci.token" })
    );

    let (code, out, err) = f.run(&["env", "claude", "loose"]);
    assert_eq!((code, out.as_str()), (1, ""));
    assert!(err.contains("must be 0600 or tighter, not 644"), "{err}");
    assert_eq!(err.lines().count(), 1);
}

#[test]
fn ls_json_lists_every_account_without_its_token() {
    let f = fixture("accounts-ls");
    let (code, out, _) = f.run(&["ls", "--json"]);
    assert_eq!(code, 0);
    assert!(!out.contains("oat-"), "a token file's value: {out}");
    let claude = f.dir.join("data/accounts/claude");
    let row = |provider: &str,
               name: &str,
               folder: Option<&Path>,
               signed_in,
               token: Option<&str>,
               error: Value| {
        let token = token.map(|t| claude.join(t));
        json!({ "provider": provider, "name": name, "folder": folder, "signed_in": signed_in,
                "token_file": token, "token_age_days": if token.is_some() && error.is_null() { json!(0) } else { json!(null) },
                "error": error, "usage": null })
    };
    let loose = format!(
        "{}: a token file must be 0600 or tighter, not 644",
        claude.join("loose.token").display()
    );
    assert_eq!(
        parse(&out),
        json!([
            row("fake", "", None, false, None, Value::Null),
            row(
                "fake",
                "idle",
                Some(&f.folder("idle")),
                false,
                None,
                Value::Null
            ),
            row(
                "fake",
                "work",
                Some(&f.folder("work")),
                true,
                None,
                Value::Null
            ),
            row("claude", "", None, false, None, Value::Null),
            row("claude", "ci", None, true, Some("ci.token"), Value::Null),
            row(
                "claude",
                "loose",
                None,
                false,
                Some("loose.token"),
                json!(loose)
            ),
        ])
    );
}

#[test]
fn an_adopted_folder_is_an_account_until_dropped_and_the_folder_stays() {
    let f = fixture("accounts-adopt");
    let a = f.dir.join("herdsman/A");
    fs::create_dir_all(&a).unwrap();
    fs::write(a.join("creds"), "signed in").unwrap();
    let a = a.canonicalize().unwrap();
    let add = |folder: &Path| f.run(&["add", "fake", "A", "--folder", folder.to_str().unwrap()]);
    assert_eq!(add(&a), (0, String::new(), String::new()));
    assert_eq!(add(&a).0, 0, "adopting the same folder again is a no-op");
    let (code, _, err) = add(&f.folder("idle"));
    assert_eq!((code, err.as_str()), (1, "cooee: fake has an account A\n"));
    assert_eq!(add(&f.dir.join("nope")).0, 1);
    let (code, _, err) = f.run(&["add", "fake", "../x", "--folder", a.to_str().unwrap()]);
    assert_eq!(code, 1, "{err}");

    // Its own folder, not the link: the CLI keys its keychain login by the folder's path.
    let (code, out, _) = f.run(&["env", "fake", "A", "--require-signed-in"]);
    assert_eq!((code, parse(&out)), (0, json!({ "FAKE_HOME": a })));
    let ls = parse(&f.run(&["ls", "--json"]).1);
    let row = ls
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "A")
        .unwrap();
    assert_eq!(
        (&row["folder"], &row["signed_in"]),
        (&json!(a), &json!(true))
    );

    assert_eq!(f.run(&["rm", "fake", "A"]).0, 0);
    assert!(a.join("creds").is_file(), "the adopted folder stays");
    assert_eq!(f.run(&["env", "fake", "A"]).0, 1);
    assert_eq!(f.run(&["rm", "fake", "A"]).0, 1);
}

#[test]
fn an_inherited_token_never_signs_a_folder_in() {
    let f = fixture("accounts-inherited");
    let (code, out, err) = f.run(&["env", "fake", "idle", "--require-signed-in"]);
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (3, "", "cooee: fake/idle is signed out\n")
    );
}

#[test]
fn ls_json_carries_usage_and_a_reading_past_its_reset_reads_zero_with_its_age() {
    let f = fixture("accounts-usage");
    fs::write(
        f.folder("work").join("usage.txt"),
        "Current session: 12% used · resets 3pm (UTC)\n\
         Current week (all models): 40% used · resets Nov 16 at 2am (UTC)\n\
         Current week (Fable): 0% used · resets Nov 16 at 2am (UTC)\n",
    )
    .unwrap();
    let usage = |f: &Fixture| {
        let ls = parse(&f.run(&["ls", "--json"]).1);
        let rows = ls.as_array().unwrap().clone();
        let work = rows.iter().find(|r| r["name"] == "work").unwrap()["usage"].clone();
        let idle = rows.iter().find(|r| r["name"] == "idle").unwrap()["usage"].clone();
        assert_eq!(idle, Value::Null, "a signed-out account has no reading");
        work
    };
    let u = usage(&f);
    assert_eq!(
        (&u["five_hour"]["used_pct"], &u["seven_day"]["used_pct"]),
        (&json!(12), &json!(40))
    );
    assert!(u["age_s"].as_u64().unwrap() < 60, "{u}");

    // A stored reading 2 minutes old (fresh: not read again) whose 5 h window reset a minute ago.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let w = |used, resets_at| json!({ "used_pct": used, "resets_at": resets_at });
    let reading = json!({ "usage": { "work": {
        "five_hour": w(97, now - 60), "seven_day": w(55, now + 86_400), "read_at": now - 120 } } });
    fs::write(
        f.dir.join("data/accounts/fake/.cooee.json"),
        reading.to_string(),
    )
    .unwrap();
    let u = usage(&f);
    assert_eq!(
        (&u["five_hour"]["used_pct"], &u["seven_day"]["used_pct"]),
        (&json!(0), &json!(55))
    );
    assert!((120..180).contains(&u["age_s"].as_u64().unwrap()), "{u}");
}
