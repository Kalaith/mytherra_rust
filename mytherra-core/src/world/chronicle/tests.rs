use super::*;

#[test]
fn recent_returns_newest_first() {
    let mut chronicle = Chronicle::default();
    chronicle.push(1, EventKind::System, "first");
    chronicle.push(2, EventKind::System, "second");
    let recent: Vec<&str> = chronicle.recent(2).map(|e| e.message.as_str()).collect();
    assert_eq!(recent, vec!["second", "first"]);
}

#[test]
fn interleave_weaves_a_tick_by_kind_and_leaves_earlier_years_be() {
    let mut chronicle = Chronicle::default();
    // An earlier year, which must be left untouched.
    chronicle.push(1, EventKind::System, "old");
    // A busy year recorded as blocks (three Hero, then two Region), the way the
    // fixed subsystem order would produce it.
    chronicle.push(2, EventKind::Hero, "h1");
    chronicle.push(2, EventKind::Hero, "h2");
    chronicle.push(2, EventKind::Hero, "h3");
    chronicle.push(2, EventKind::Region, "r1");
    chronicle.push(2, EventKind::Region, "r2");

    chronicle.interleave_latest_tick();

    // Kinds are round-robined (ALL order is Divine, Region, Hero, System, so
    // Region draws before Hero each round) and within-kind order is preserved;
    // the surplus Hero trails once Region is spent.
    let year_two: Vec<&str> = chronicle.events[1..]
        .iter()
        .map(|e| e.message.as_str())
        .collect();
    assert_eq!(year_two, vec!["r1", "h1", "r2", "h2", "h3"]);
    assert_eq!(
        chronicle.events[0].message, "old",
        "an earlier year is undisturbed"
    );
}

#[test]
fn cap_drops_oldest() {
    let mut chronicle = Chronicle {
        events: Vec::new(),
        cap: 3,
        total_pushed: 0,
    };
    for i in 0..5 {
        chronicle.push(i, EventKind::System, format!("e{i}"));
    }
    assert_eq!(chronicle.recent(10).count(), 3);
    let newest = chronicle.recent(1).next().unwrap();
    assert_eq!(newest.message, "e4");
}

#[test]
fn since_returns_only_the_new_events_and_survives_the_cap() {
    let mut chronicle = Chronicle::default();
    chronicle.push(1, EventKind::System, "a");
    chronicle.push(2, EventKind::System, "b");
    let (events, cursor) = chronicle.since(0);
    assert_eq!(
        events
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(cursor, 2);

    // Nothing new since the cursor.
    let (events, cursor2) = chronicle.since(cursor);
    assert!(events.is_empty());
    assert_eq!(cursor2, 2);

    // A fresh push shows up as the only delta.
    chronicle.push(3, EventKind::Hero, "c");
    let (events, cursor3) = chronicle.since(cursor);
    assert_eq!(
        events
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>(),
        vec!["c"]
    );
    assert_eq!(cursor3, 3);

    // A cursor older than the retained window still yields the retained tail,
    // never a panic — even after the cap has dropped the oldest events.
    let mut small = Chronicle {
        events: Vec::new(),
        cap: 2,
        total_pushed: 0,
    };
    for i in 0..5 {
        small.push(i, EventKind::System, format!("e{i}"));
    }
    let (events, cursor) = small.since(0);
    assert_eq!(events.len(), 2, "only the retained tail survives");
    assert_eq!(cursor, 5, "the cursor still counts every event ever pushed");
}
