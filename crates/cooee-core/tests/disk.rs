//! [`cooee_core::disk::patch`] changes only the keys that differ (#360).

use cooee_core::disk::patch;

#[test]
fn a_patch_changes_only_the_keys_that_differ() {
    let text = "# mine\nname = 'A' # kept\n\nmodel = \"x\" # why\n\n[env]\n# a var\nA = \"1\"\n\n\
                    [[r]]\nid = \"1\"\n# note\nn = 1\n\n[[r]]\nid = \"2\"\nn = 2\n";
    let was = "name = \"A\"\nmodel = \"x\"\n[env]\nA = \"1\"\n[[r]]\nid = \"1\"\nn = 1\n[[r]]\nid = \"2\"\nn = 2\n";
    let now = "name = \"A\"\nmodel = \"y\"\npinned = true\n[env]\nA = \"1\"\nB = \"2\"\n[[r]]\nid = \"1\"\nn = 3\n";
    let out = patch(text, was, now).unwrap();
    assert_eq!(
        out,
        "# mine\nname = 'A' # kept\n\nmodel = \"y\" # why\npinned = true\n\n[env]\n# a var\nA = \"1\"\nB = \"2\"\n\n\
             [[r]]\nid = \"1\"\n# note\nn = 3\n"
    );
    assert_eq!(patch(&out, now, now).unwrap(), out);
}

#[test]
fn a_routine_save_keeps_the_comments() {
    let dir = cooee_core::scratch::owned(
        std::env::temp_dir().join(format!("cooee-disk-{}", std::process::id())),
    );
    let mine = "# mine\n\n[[routine]]\nid = \"r1\" # first\nname = \"Renewals\"\n\
                when = \"0 9 * * 1\"\nprompt = \"Renewals due?\"\nplace = { in = \"chat\", thread = \"t-1\" }\n";
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("routines.toml"), mine).unwrap();
    let file = cooee_core::routines::Routines::new(&dir, &dir);
    let mut r = file.load().unwrap();
    r[0].prompt = "Renewals?".to_owned();
    file.save(&r).unwrap();
    let now = std::fs::read_to_string(dir.join("routines.toml")).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(now, mine.replace("Renewals due?", "Renewals?"));
}
