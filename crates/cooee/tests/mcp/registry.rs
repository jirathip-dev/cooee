use std::{fs, process::Command};

use crate::support::scratch;

#[test]
fn a_literal_token_in_the_registry_is_refused_at_load_naming_the_key_only() {
    let dir = scratch("mcp-literal");
    let config = dir.join("config.toml");
    let token = format!("ghp_{}", "z9".repeat(14));
    let refused = |text: &str| {
        fs::write(&config, text).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .arg("--config")
            .arg(&config)
            .args(["mcp", "ls"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        String::from_utf8(out.stderr).unwrap()
    };
    let e = refused(&format!(
        "[[mcp]]\nname = \"gh\"\ncommand = \"gh-mcp\"\nenv = {{ GH_TOKEN = \"{token}\" }}\n"
    ));
    assert!(
        e.contains("mcp gh: env GH_TOKEN: only a reference") && !e.contains(&token),
        "{e}"
    );
    let e = refused(&format!(
        "[[mcp]]\nname = \"gh\"\ncommand = \"gh-mcp\"\nargs = [\"--token\", \"{token}\"]\n"
    ));
    assert!(
        e.contains("looks like a token") && !e.contains(&token),
        "{e}"
    );
    let e = refused(&format!(
        "[[mcp]]\nname = \"w\"\ntype = \"http\"\nurl = \"https://x/mcp\"\nheaders = {{ Authorization = \"Bearer {token}\" }}\n"
    ));
    assert!(
        e.contains("headers Authorization: only a reference") && !e.contains(&token),
        "{e}"
    );
}

#[test]
fn the_mac_adds_toggles_and_removes_servers_with_references_only() {
    let dir = scratch("mcp-cli");
    let config = dir.join("config.toml");
    fs::write(&config, "# mine\nport = 1\n").unwrap();
    let cooee = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .arg("--config")
            .arg(&config)
            .args(args)
            .output()
            .unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned()
                + &String::from_utf8_lossy(&out.stderr),
        )
    };
    let add = [
        "mcp",
        "add",
        "docs",
        "--type",
        "sse",
        "--url",
        "https://docs.example/sse",
        "--header",
        "Authorization=secret:DOCS",
    ];
    assert_eq!(
        cooee(&add),
        (
            true,
            "added docs: [sse] https://docs.example/sse\n".to_owned()
        )
    );
    let (ok, said) = cooee(&[
        "mcp",
        "add",
        "git",
        "uvx",
        "mcp-server-git",
        "--env",
        "GIT_TOKEN=env:GIT_TOKEN",
    ]);
    assert!(ok, "{said}");
    let (ok, said) = cooee(&["mcp", "add", "bad", "x", "--env", "T=literal"]);
    assert!(!ok && said.contains("env T: only a reference"), "{said}");
    assert!(cooee(&["mcp", "disable", "git"]).0);
    assert_eq!(
        cooee(&["mcp", "ls"]).1,
        "[sse] docs · on · https://docs.example/sse\n[stdio] git · off · uvx mcp-server-git\n"
    );
    assert!(cooee(&["mcp", "rm", "docs"]).0);
    let text = fs::read_to_string(&config).unwrap();
    assert!(
        text.starts_with("# mine\nport = 1\n\n[[mcp]]\nname = \"git\"\n") && !text.contains("docs"),
        "{text}"
    );
    assert!(
        text.contains(r#"env = { "GIT_TOKEN" = "env:GIT_TOKEN" }"#),
        "{text}"
    );
    fs::write(&config, text + "provides = [\"web\"]\n").unwrap();
    assert!(cooee(&["mcp", "enable", "git"]).0);
    let text = fs::read_to_string(&config).unwrap();
    assert!(
        text.ends_with("enabled = true\nprovides = [\"web\"]\n"),
        "{text}"
    );
}
