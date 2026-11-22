use super::*;

fn offered(ids: &[&str]) -> Offered {
    let models = ids.iter().map(|i| [(*i).to_owned(), (*i).to_owned()]);
    Offered {
        models: models.collect(),
        ..Offered::default()
    }
}

fn id(o: &Offered, pick: &str) -> Option<(String, Option<String>)> {
    o.resolve(pick).map(|r| (r.id, r.fallback))
}

#[test]
fn a_short_name_is_its_familys_newest_and_an_id_is_itself() {
    let o = offered(&[
        "claude-opus-4-20250514",
        "claude-opus-4-5-20251101",
        "claude-opus-4-1-20250805",
        "claude-sonnet-4-5",
        "gpt-5-codex",
        "gpt-5.1-codex",
    ]);
    let newest = "claude-opus-4-5-20251101".to_owned();
    assert_eq!(id(&o, "opus"), Some((newest.clone(), None)));
    assert_eq!(id(&o, "claude-opus"), Some((newest.clone(), None)));
    assert_eq!(id(&o, "gpt-5"), Some(("gpt-5.1-codex".to_owned(), None)));
    let pinned = "claude-opus-4-1-20250805";
    assert_eq!(id(&o, pinned), Some((pinned.to_owned(), None)));
    assert_eq!(
        id(&o, "claude-opus-3-20240229"),
        Some((newest, Some("claude-opus".to_owned())))
    );
    assert_eq!(id(&o, "haiku"), None);
    assert_eq!(
        o.picks()[..3],
        ["claude-opus", "claude-sonnet", "gpt-codex"]
    );
}
