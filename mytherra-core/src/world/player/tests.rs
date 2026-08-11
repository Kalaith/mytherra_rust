use super::*;

fn config() -> GameConfig {
    GameConfig {
        game_name: "mytherra".to_owned(),
        display_name: "Mytherra".to_owned(),
        save_slot: "world".to_owned(),
        version: "0.1.0".to_owned(),
        starting_favor: 140,
        favor_per_tick: 10,
        max_favor: 600,
        seconds_per_tick: 10.0,
        autosave_every_ticks: 6,
        start_year: 1,
        world_seed: 1,
        server_url: "http://127.0.0.1:8791".to_owned(),
        gateway_url: String::new(),
        server_listen_addr: "127.0.0.1:8791".to_owned(),
        view_poll_seconds: 2.0,
    }
}

fn player_balance() -> PlayerBalance {
    PlayerBalance {
        level_base_cost: 100,
        level_cost_step: 60,
        max_favor_per_level: 40,
        favor_per_tick_per_level: 1,
        achievement_experience: 60,
        favor_per_resonance: 0.03,
        favor_tithe_baseline: 50.0,
        tier_unlock_levels: vec![2, 4, 7],
    }
}

#[test]
fn spending_debits_and_tracks() {
    let mut player = PlayerState::new(&config());
    assert!(player.spend(15, &player_balance()));
    assert_eq!(player.favor, 125);
    assert_eq!(player.favor_spent, 15);
    assert_eq!(player.nudges, 1);
}

#[test]
fn cannot_overspend() {
    let mut player = PlayerState::new(&config());
    assert!(!player.spend(10_000, &player_balance()));
    assert_eq!(player.favor, 140);
}

#[test]
fn recovery_respects_ceiling() {
    let cfg = config();
    let bal = player_balance();
    let mut player = PlayerState::new(&cfg);
    player.favor = cfg.max_favor - 5;
    player.recover(0, &cfg, &bal);
    assert_eq!(player.favor, cfg.max_favor);
}

#[test]
fn a_higher_standing_holds_and_recovers_more_favor() {
    let cfg = config();
    let bal = player_balance();
    let mut player = PlayerState::new(&cfg);
    let base_cap = player.max_favor(&cfg, &bal);
    let base_recovery = player.favor_recovery(&cfg, &bal);

    player.level = 4; // three levels past the first
    assert_eq!(
        player.max_favor(&cfg, &bal),
        base_cap + 3 * bal.max_favor_per_level
    );
    assert_eq!(
        player.favor_recovery(&cfg, &bal),
        base_recovery + 3 * bal.favor_per_tick_per_level
    );

    // Recovery now fills toward the raised ceiling, not the base one.
    player.favor = base_cap;
    player.recover(0, &cfg, &bal);
    assert!(player.favor > base_cap);
}

#[test]
fn spending_grants_levels() {
    let mut player = PlayerState::new(&config());
    player.favor = 10_000;
    for _ in 0..20 {
        player.spend(30, &player_balance());
    }
    assert!(player.level > 1);
}
