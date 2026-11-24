use serde_json::{Value, json};

use crate::setup::{TOKEN_A, setup};

#[test]
fn a_plugin_names_its_push_tag_and_sets_its_dot_only_with_its_token() {
    let s = setup("plugin-dot");
    let a = format!("Bearer {TOKEN_A}");
    let push = json!({ "text": "box offline", "path": "box", "tag": "machine" });
    let (code, note) = s.as_backend("/plugin/push", &a, &push);
    assert_eq!(code, 200, "{note}");
    let note: Value = serde_json::from_str(&note).unwrap();
    assert_eq!(
        (&note["tag"], &note["url"]),
        (&json!("machine"), &json!("/p/a#box"))
    );

    assert_eq!(
        s.as_backend("/plugin/dot", "", &json!({ "on": true })).0,
        401
    );
    for on in [true, false] {
        let (code, body) = s.as_backend("/plugin/dot", &a, &json!({ "on": on }));
        assert_eq!((code, body), (200, on.to_string()));
    }
}
