use super::*;

const MIN: u64 = 60_000;
const MON: u64 = 1_731_283_200_000;

fn ask(when: &str, place: &str) -> Ask {
    Ask {
        name: " Monday renewals ".to_owned(),
        when: Some(when.to_owned()),
        github: None,
        webhook: None,
        prompt: "Renewals due {{due}}?".to_owned(),
        script: None,
        place: place.to_owned(),
        until: None,
    }
}

fn chat() -> Thread {
    Thread {
        id: "t-1".to_owned(),
        agent: "fake-acp".to_owned(),
        model: "m".to_owned(),
        ..Thread::default()
    }
}

#[test]
fn a_routine_is_due_once_per_due_time_after_it_was_made() {
    let made = MON - 3 * 1440 * MIN;
    let mut r = Routine::make(&[], ask("0 9 * * 1", ""), &chat(), (made, 0)).unwrap();
    assert_eq!(
        (r.name.as_str(), r.when().as_str(), &r.place),
        (
            "Monday renewals",
            "Mondays 09:00",
            &Place::Chat {
                thread: "t-1".to_owned()
            }
        )
    );
    let nine = MON + 9 * 60 * MIN;
    assert_eq!(r.due(nine - MIN, 0), None);
    assert_eq!(r.due(nine + 5 * MIN, 0), Some(nine));
    assert_eq!(
        r.text(&Fields::Due(nine, 0)),
        "Renewals due 2024-11-11 09:00?"
    );
    r.last_due = nine;
    assert_eq!(r.due(nine + 5 * MIN, 0), None);
    assert_eq!(
        r.due(nine + 14 * 1440 * MIN, 0),
        Some(nine + 14 * 1440 * MIN)
    );
    assert_eq!(r.key(nine), format!("routine-{}-{nine}", r.id));
}

#[test]
fn make_and_edit_refuse_what_is_wrong_and_change_nothing() {
    let list = vec![Routine::make(&[], ask("0 9 * * 1", ""), &chat(), (MON, 0)).unwrap()];
    let other = Routine::make(&list, ask("0 9 * * 1", "new_thread"), &chat(), (MON, 0)).unwrap();
    assert_ne!(other.id, list[0].id);
    let Place::NewThread { who } = &other.place else {
        panic!("{:?}", other.place)
    };
    assert_eq!((who.agent.as_str(), who.model.as_str()), ("fake-acp", "m"));
    for (a, why) in [
        (ask("*/5 * * * *", ""), "BAD_SCHEDULE"),
        (ask("0 9 * * 1", "somewhere"), "BAD_PLACE"),
        (
            Ask {
                name: " ".to_owned(),
                ..ask("0 9 * * 1", "")
            },
            "BAD_NAME",
        ),
    ] {
        let e = Routine::make(&list, a, &chat(), (MON, 0)).unwrap_err();
        assert!(e.starts_with(why), "{e}");
    }
    let full = vec![list[0].clone(); MAX];
    let e = Routine::make(&full, ask("0 9 * * 1", ""), &chat(), (MON, 0)).unwrap_err();
    assert!(e.starts_with("TOO_MANY"), "{e}");
    let mut r = list[0].clone();
    let bad = Edit {
        name: Some("x".repeat(61)),
        ..Edit::default()
    };
    assert!(r.edit(bad, MON + 1).is_err());
    assert_eq!(r, list[0]);
    let e = Edit {
        when: Some("30 18 * * *".to_owned()),
        paused: Some(true),
        ..Edit::default()
    };
    r.edit(e, MON + 1).unwrap();
    assert_eq!(
        (r.when().as_str(), r.last_due),
        ("every day 18:30", MON + 1)
    );
    assert_eq!(r.paused, Some(Paused::Owner));
}

#[test]
fn the_file_round_trips_and_refuses_a_newer_format() {
    let dir = crate::scratch::owned(
        std::env::temp_dir().join(format!("cooee-routines-{}", std::process::id())),
    );
    let file = Routines::new(&dir, &dir);
    assert_eq!(file.load().unwrap(), Book::default());
    let mut r = Routine::make(&[], ask("0 9 * * 1", ""), &chat(), (MON, 0)).unwrap();
    r.last_run = Some(Run {
        at: MON,
        key: r.key(MON),
        outcome: Outcome::Ran {
            thread: "t-1".to_owned(),
        },
    });
    let github = Ask {
        when: None,
        github: serde_json::from_value(serde_json::json!({
            "repo": "o/app", "events": ["push"], "match": { "ref": "refs/heads/main" } }))
        .unwrap(),
        ..ask("", "")
    };
    let mut g = Routine::make(&[], github, &chat(), (MON + 1, 0)).unwrap();
    g.recent = vec![MON];
    g.ends_at = Some(MON + HOUR);
    g.paused = Some(Paused::Ended(Ended {
        at: MON,
        by: End::StopRoutine,
    }));
    let mut book = Book {
        routines: vec![r, g],
        ..Book::default()
    };
    assert!(book.see("g-1", MON) && !book.see("g-1", MON + 1));
    file.save(&book).unwrap();
    let text = std::fs::read_to_string(dir.join("routines.toml")).unwrap();
    assert!(text.contains("when = \"0 9 * * 1\""), "{text}");
    assert!(text.contains("[routine.github.match]"), "{text}");
    let runs = std::fs::read_to_string(dir.join("routine-runs.json")).unwrap();
    assert!(runs.contains("\"outcome\": \"ran\""), "{runs}");
    assert!(runs.contains("\"g-1\""), "{runs}");
    assert_eq!(file.load().unwrap(), book);
    std::fs::write(dir.join("routine-runs.json"), r#"{"format":9,"runs":{}}"#).unwrap();
    assert!(file.load().is_err());
    std::fs::remove_dir_all(&dir).unwrap();
}
