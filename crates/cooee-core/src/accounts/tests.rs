use super::*;

#[test]
fn system_first_a_new_account_listed_once_adopted_and_removal() {
    let data = crate::scratch::owned(
        std::env::temp_dir().join(format!("cooee-accounts-{}", std::process::id())),
    );
    let a = Accounts::new(&data);
    let names =
        |a: &Accounts| -> Vec<String> { a.list("claude").into_iter().map(|a| a.name).collect() };
    assert_eq!(names(&a), [""]);
    assert!(a.get("claude", "").unwrap().dir.is_none());

    let (name, dir) = a.pending("claude").unwrap();
    assert_eq!((name.as_str(), dir.is_dir()), ("account-2", true));
    assert_eq!(names(&a), [""], "a pending account isn't listed");
    assert_eq!(a.pending("claude").unwrap().0, "account-3");
    a.drop_pending("claude", "account-3");
    a.adopt("claude", "account-2", 7).unwrap();
    assert_eq!(names(&a), ["", "account-2"]);
    let work = a.get("claude", "account-2").unwrap();
    assert_eq!(work.signed_in_at, Some(7));
    assert_eq!(work.dir, Some(data.join("accounts/claude/account-2")));

    a.remove("claude", "account-2").unwrap();
    assert!(!data.join("accounts/claude/account-2").exists());
    assert_eq!(names(&a), [""]);
    assert!(a.get("claude", "../x").is_none());
    fs::remove_dir_all(&data).unwrap();
}

#[test]
fn a_dead_logins_leftover_folder_never_blocks_a_new_account() {
    let data = crate::scratch::owned(
        std::env::temp_dir().join(format!("cooee-accounts-dead-{}", std::process::id())),
    );
    fs::remove_dir_all(&data).ok();
    let a = Accounts::new(&data);
    fs::create_dir_all(data.join("accounts/claude/.new-account-2")).unwrap();
    let (name, dir) = a.pending("claude").unwrap();
    assert_eq!((name.as_str(), dir.is_dir()), ("account-3", true));
    a.adopt("claude", "account-3", 1).unwrap();
    let names: Vec<String> = a.list("claude").into_iter().map(|a| a.name).collect();
    assert_eq!(names, ["", "account-3"]);
    fs::remove_dir_all(&data).unwrap();
}
