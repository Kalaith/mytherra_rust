use super::*;

fn bank() -> EraNameBank {
    EraNameBank {
        prefixes: vec!["Golden".into(), "Ashen".into()],
        titles: vec!["Dawn".into(), "Ruin".into()],
        patterns: vec!["The Age of {title}".into(), "The {prefix} {title}".into()],
        trigger_prefixes: Default::default(),
    }
}

#[test]
fn era_name_is_deterministic_for_a_seed() {
    let b = bank();
    let mut lhs = SeededRng::new(42);
    let mut rhs = SeededRng::new(42);
    assert_eq!(
        generate_era_name(&b, None, &mut lhs),
        generate_era_name(&b, None, &mut rhs)
    );
}

#[test]
fn era_name_fills_every_slot_from_the_pools() {
    let b = bank();
    let mut rng = SeededRng::new(7);
    for _ in 0..50 {
        let name = generate_era_name(&b, None, &mut rng);
        assert!(!name.contains('{'), "left an unfilled slot: {name}");
        assert!(
            b.titles.iter().any(|t| name.contains(t)),
            "name should carry a title: {name}"
        );
    }
}

#[test]
fn empty_patterns_fall_back_to_the_classic_form() {
    let mut b = bank();
    b.patterns.clear();
    let mut rng = SeededRng::new(3);
    let name = generate_era_name(&b, None, &mut rng);
    assert!(
        name.starts_with("The "),
        "expected classic form, got: {name}"
    );
}

#[test]
fn an_age_is_named_after_the_trigger_that_birthed_it() {
    // A bank whose trigger pool for Cataclysm is a single unique word, so the
    // birthing trigger is unmistakable in the name; Collapse's pool is empty
    // and must fall back to the generic prefix.
    let mut b = EraNameBank {
        prefixes: vec!["Generic".into()],
        titles: vec!["Age".into()],
        patterns: vec!["The {prefix} {title}".into()],
        trigger_prefixes: Default::default(),
    };
    b.trigger_prefixes.cataclysm = vec!["Cataclysmic".into()];
    let mut rng = SeededRng::new(11);
    assert_eq!(
        generate_era_name(&b, Some(EraTrigger::Cataclysm), &mut rng),
        "The Cataclysmic Age",
        "a cataclysm-born age draws its prefix from the cataclysm pool"
    );
    assert_eq!(
        generate_era_name(&b, Some(EraTrigger::Collapse), &mut rng),
        "The Generic Age",
        "an empty trigger pool falls back to the generic prefixes"
    );
    assert_eq!(
        generate_era_name(&b, None, &mut rng),
        "The Generic Age",
        "the first age, birthed by no trigger, uses the generic prefixes"
    );
}
