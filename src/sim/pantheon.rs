//! Per-tick pantheon behaviour (GDD 5.6): each deity's pressure drifts toward a
//! baseline, and a roused deity presses its domain upon every region scaled by
//! its pressure tier. Deterministic: no RNG.

use crate::data::{PantheonBalance, PantheonStat, RegionBalance};
use crate::world::{PantheonDeity, Region};
use macroquad_toolkit::math::approach;

/// Advance every deity by one tick and apply their domain pressure.
pub fn tick_pantheon(
    deities: &mut [PantheonDeity],
    regions: &mut [Region],
    balance: &PantheonBalance,
    region_balance: &RegionBalance,
) {
    // Snapshot every deity's pressure at tick start so the ally/rival coupling is
    // order-independent: each deity reacts to the others as they stood this tick,
    // not to whichever neighbours the loop happened to update first.
    let snapshot: Vec<(String, f32)> = deities.iter().map(|d| (d.id.clone(), d.pressure)).collect();
    let pressure_of = |id: &str| snapshot.iter().find(|(sid, _)| sid == id).map(|(_, p)| *p);

    for deity in deities.iter_mut() {
        deity.cooldown = (deity.cooldown - 1).max(0);
        // A deity stirs toward a baseline shifted by how ascendant its domain is
        // across the world, so the state of the world rouses the gods.
        let domain = domain_average(regions, deity.effect_stat);
        // The diamond pulls too: a rival's agitation above the resting baseline
        // provokes, an ally's pressure draws toward solidarity.
        let rival = pressure_of(&deity.rival_id).unwrap_or(balance.drift_target);
        let ally = pressure_of(&deity.ally_id).unwrap_or(deity.pressure);
        let target = (balance.drift_target
            + (domain - 50.0) * balance.domain_response
            + (rival - balance.drift_target) * balance.rival_coupling
            + (ally - deity.pressure) * balance.ally_coupling)
            .clamp(0.0, 100.0);
        deity.pressure = approach(deity.pressure, target, balance.drift_rate);

        let scale = deity.tier_multiplier(balance);
        if scale > 0.0 {
            for region in regions.iter_mut() {
                // The gods reshape the faithful lands more than the faithless
                // ones: a deity's pressure lands scaled by the region's divine
                // resonance — the same receptiveness the player's own nudges obey
                // (GDD 5.6 <-> 5.2), so one rule governs how divine will takes
                // hold, whoever wields it.
                let resonance = region.effect_multiplier(region_balance);
                let (dp, dc, dd, dm) =
                    stat_deltas(deity.effect_stat, deity.effect_amount * scale * resonance);
                region.apply_deltas(dp, dc, dd, dm, region_balance);
            }
        }
    }
}

/// Names of deities that have crested into the top pressure tier this tick but
/// hadn't before — a god roused to the height of its wrath, worth chronicling.
/// `before` is each deity's tier at tick start (index-aligned with `deities`).
pub fn deities_cresting(
    before: &[usize],
    deities: &[PantheonDeity],
    balance: &PantheonBalance,
) -> Vec<String> {
    let top = balance.tiers.len();
    deities
        .iter()
        .enumerate()
        .filter(|(i, d)| d.tier(balance) >= top && before.get(*i).copied().unwrap_or(0) < top)
        .map(|(_, d)| d.name.clone())
        .collect()
}

/// The world's average value of the stat a deity holds domain over.
fn domain_average(regions: &[Region], stat: PantheonStat) -> f32 {
    if regions.is_empty() {
        return 50.0;
    }
    let sum: f32 = regions
        .iter()
        .map(|r| match stat {
            PantheonStat::Prosperity => r.prosperity,
            PantheonStat::Chaos => r.chaos,
            PantheonStat::Danger => r.danger,
            PantheonStat::Magic => r.magic_affinity,
        })
        .sum();
    sum / regions.len() as f32
}

/// Map a pantheon stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: PantheonStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        PantheonStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        PantheonStat::Chaos => (0.0, amount, 0.0, 0.0),
        PantheonStat::Danger => (0.0, 0.0, amount, 0.0),
        PantheonStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn only_a_fresh_crest_into_wrath_is_reported() {
        let data = GameData::load().unwrap();
        let balance = &data.balance.pantheon;
        let top = balance.tiers.len();
        let mut world = WorldState::new(&data);

        // Two deities now sit at the apex; the rest stay calm.
        let apex_pressure = *balance.tiers.last().unwrap() + 5.0;
        world.pantheon[0].pressure = apex_pressure;
        world.pantheon[1].pressure = apex_pressure;

        // But only the first was below the apex last tick.
        let mut before: Vec<usize> = world.pantheon.iter().map(|d| d.tier(balance)).collect();
        before[0] = top - 1;
        before[1] = top;

        let cresting = deities_cresting(&before, &world.pantheon, balance);
        assert!(
            cresting.contains(&world.pantheon[0].name),
            "a fresh crest is reported"
        );
        assert!(
            !cresting.contains(&world.pantheon[1].name),
            "a deity already at the apex isn't re-reported"
        );
    }

    #[test]
    fn pressure_drifts_toward_baseline() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.pantheon[0].pressure = 95.0;
        tick_pantheon(
            &mut world.pantheon,
            &mut world.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );
        assert!(world.pantheon[0].pressure < 95.0);
    }

    #[test]
    fn roused_deity_presses_its_domain() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Aurex (prosperity) at full pressure should raise prosperity.
        let idx = world.pantheon.iter().position(|d| d.id == "aurex").unwrap();
        world.pantheon[idx].pressure = 100.0;
        let before = world.regions[0].prosperity;
        tick_pantheon(
            &mut world.pantheon,
            &mut world.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );
        assert!(world.regions[0].prosperity >= before);
    }

    #[test]
    fn the_gods_press_a_faithful_region_harder_than_a_faithless_one() {
        // Two regions identical but for their divine resonance; a roused deity of
        // prosperity should lift the high-resonance land more than the deaf one
        // (GDD 5.6 <-> 5.2).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let idx = world.pantheon.iter().position(|d| d.id == "aurex").unwrap();
        world.pantheon[idx].pressure = 100.0;

        // Isolate two regions with the same starting prosperity, opposite faith.
        world.regions.truncate(2);
        for r in &mut world.regions {
            r.prosperity = 50.0;
        }
        world.regions[0].divine_resonance = 100.0; // steeped in the divine
        world.regions[1].divine_resonance = 0.0; // deaf to the gods

        tick_pantheon(
            &mut world.pantheon,
            &mut world.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );

        let faithful_gain = world.regions[0].prosperity - 50.0;
        let faithless_gain = world.regions[1].prosperity - 50.0;
        assert!(
            faithful_gain > faithless_gain,
            "the divine should shape the faithful land more: {faithful_gain} vs {faithless_gain}"
        );
    }

    #[test]
    fn an_agitated_rival_provokes_its_nemesis() {
        let data = GameData::load().unwrap();
        let baseline = data.balance.pantheon.drift_target;

        // Tick the first deity with its rival calm vs. inflamed, holding every
        // region neutral so only the rivalry coupling differs between the runs.
        let run = |rival_pressure: f32| {
            let mut w = WorldState::new(&data);
            for r in &mut w.regions {
                r.prosperity = 50.0;
                r.chaos = 50.0;
                r.danger = 50.0;
                r.magic_affinity = 50.0;
            }
            w.pantheon[0].pressure = baseline;
            let rival_id = w.pantheon[0].rival_id.clone();
            if let Some(rival) = w.pantheon.iter_mut().find(|d| d.id == rival_id) {
                rival.pressure = rival_pressure;
            }
            tick_pantheon(
                &mut w.pantheon,
                &mut w.regions,
                &data.balance.pantheon,
                &data.balance.region,
            );
            w.pantheon[0].pressure
        };

        assert!(
            run(90.0) > run(40.0),
            "an agitated rival should provoke its nemesis"
        );
    }

    #[test]
    fn an_ascendant_domain_rouses_its_deity() {
        let data = GameData::load().unwrap();
        let baseline = data.balance.pantheon.drift_target;

        // Mordath holds domain over danger. A world steeped in danger should pull
        // its pressure above the calm baseline...
        let mut dangerous = WorldState::new(&data);
        let idx = dangerous
            .pantheon
            .iter()
            .position(|d| d.effect_stat == PantheonStat::Danger)
            .unwrap();
        for r in &mut dangerous.regions {
            r.danger = 95.0;
        }
        dangerous.pantheon[idx].pressure = baseline;
        tick_pantheon(
            &mut dangerous.pantheon,
            &mut dangerous.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );

        // ...while a placid world lets it settle back down.
        let mut calm = WorldState::new(&data);
        for r in &mut calm.regions {
            r.danger = 5.0;
        }
        calm.pantheon[idx].pressure = baseline;
        tick_pantheon(
            &mut calm.pantheon,
            &mut calm.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );

        assert!(dangerous.pantheon[idx].pressure > baseline);
        assert!(calm.pantheon[idx].pressure < baseline);
        assert!(dangerous.pantheon[idx].pressure > calm.pantheon[idx].pressure);
    }

    #[test]
    fn every_ally_and_rival_id_resolves() {
        // The ally/rival web is hand-wired; a typo would silently render as a raw
        // id in the UI. Guard that every reference points at a real deity.
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let ids: Vec<&str> = world.pantheon.iter().map(|d| d.id.as_str()).collect();
        for deity in &world.pantheon {
            assert!(
                ids.contains(&deity.ally_id.as_str()),
                "{} has unknown ally {}",
                deity.id,
                deity.ally_id
            );
            assert!(
                ids.contains(&deity.rival_id.as_str()),
                "{} has unknown rival {}",
                deity.id,
                deity.rival_id
            );
            assert_ne!(deity.ally_id, deity.id, "{} allies itself", deity.id);
            assert_ne!(deity.rival_id, deity.id, "{} rivals itself", deity.id);
        }
    }
}
