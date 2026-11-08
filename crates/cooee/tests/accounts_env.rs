//! `cooee accounts env` and `accounts ls --json`, the credential helper's contract (docs/accounts.md), with a fake
//! provider whose status reads its folder: a signed-in and a signed-out folder account, unknown ones refused, a
//! Claude token file (0600) handed over and a looser one refused, and no folder's secret ever on stdout or stderr.

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

/// Provider `fake` (`FAKE_HOME`; signed in when the folder has `creds`), whose accounts are `work` (signed in) and
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
             [[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\nlogin = [\"true\"]\n\
             status = [\"sh\", \"-c\", \"cat \\\"$FAKE_HOME/creds\\\"\"]\n\
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
                "error": error })
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
