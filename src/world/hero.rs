//! Runtime hero state: the mutable, simulated form of a `HeroSeed`.

use crate::data::{HeroBalance, HeroRole, HeroSeed};
use serde::{Deserialize, Serialize};

/// The live state of one hero in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hero {
    pub id: String,
    pub name: String,
    pub role: HeroRole,
    /// Id of the region the hero currently resides in.
    pub region_id: String,
    pub level: u32,
    pub age: u32,
    pub is_alive: bool,
    /// Accrued fame (GDD 5.4): rises with each level gained and each era
    /// survived, earning a title and helping a legend cheat death.
    #[serde(default)]
    pub renown: f32,
}

impl Hero {
    pub fn from_seed(seed: &HeroSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            role: seed.role,
            region_id: seed.region_id.clone(),
            level: seed.level.max(1),
            age: seed.age,
            is_alive: true,
            renown: 0.0,
        }
    }

    /// The hero's earned title: the highest-threshold tier its renown clears, or
    /// "" if still unknown. `titles[i]` is earned at `thresholds[i]` (ascending).
    pub fn title<'a>(&self, titles: &'a [String], thresholds: &[f32]) -> &'a str {
        let mut earned = "";
        for (title, threshold) in titles.iter().zip(thresholds) {
            if self.renown >= *threshold {
                earned = title;
            }
        }
        earned
    }

    /// Expected lifespan in years: grows with level (GDD 5.4).
    pub fn life_expectancy(&self, balance: &HeroBalance) -> f32 {
        balance.life_expectancy_base + self.level as f32 * balance.life_expectancy_per_level
    }

    /// Per-tick chance of gaining a level. Falls off with level, with three
    /// tiers (early / mid / veteran) tuned in `balance.json` (GDD 5.4).
    pub fn level_up_chance(&self, balance: &HeroBalance) -> f32 {
        let curve = &balance.level_up;
        let tier_mult = if self.level <= curve.low_tier_max_level {
            curve.low_tier_mult
        } else if self.level >= curve.high_tier_min_level {
            curve.high_tier_mult
        } else {
            curve.mid_tier_mult
        };
        let decay = curve.decay.powi(self.level as i32 - 1);
        curve.base_chance * tier_mult * decay
    }

    /// Level-up chance tempered by the peril of the hero's region: a hero forged
    /// in a dangerous land grows faster than one in a placid one (GDD 5.4).
    pub fn level_up_chance_in(&self, region_danger: f32, balance: &HeroBalance) -> f32 {
        self.level_up_chance(balance) * (1.0 + region_danger * balance.level_up.crucible_coeff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balance() -> HeroBalance {
        crate::data::GameData::load().unwrap().balance.hero
    }

    fn hero(level: u32) -> Hero {
        Hero::from_seed(&HeroSeed {
            id: "h".to_owned(),
            name: "H".to_owned(),
            role: HeroRole::Warrior,
            region_id: "r".to_owned(),
            level,
            age: 30,
        })
    }

    #[test]
    fn life_expectancy_grows_with_level() {
        let b = balance();
        assert!(hero(10).life_expectancy(&b) > hero(1).life_expectancy(&b));
    }

    #[test]
    fn level_up_chance_falls_off_with_level() {
        let b = balance();
        assert!(hero(1).level_up_chance(&b) > hero(20).level_up_chance(&b));
        assert!(hero(20).level_up_chance(&b) > hero(60).level_up_chance(&b));
    }

    #[test]
    fn peril_quickens_a_heros_growth() {
        let b = balance();
        let h = hero(5);
        assert!(
            h.level_up_chance_in(100.0, &b) > h.level_up_chance_in(0.0, &b),
            "a hero in a dangerous land should grow faster than one at peace"
        );
        assert_eq!(
            h.level_up_chance_in(0.0, &b),
            h.level_up_chance(&b),
            "a placid region leaves the base growth chance untouched"
        );
    }

    #[test]
    fn renown_earns_titles_in_ascending_tiers() {
        let data = crate::data::GameData::load().unwrap();
        let titles = &data.strings.heroes.renown_titles;
        let thresholds = &data.balance.hero.renown.thresholds;
        let mut h = hero(1);

        h.renown = 0.0;
        assert_eq!(
            h.title(titles, thresholds),
            "",
            "an unknown hero has no title"
        );
        h.renown = thresholds[0];
        assert_eq!(h.title(titles, thresholds), titles[0].as_str());
        h.renown = *thresholds.last().unwrap() + 1_000.0;
        assert_eq!(
            h.title(titles, thresholds),
            titles.last().unwrap().as_str(),
            "a hero past the top threshold earns the highest title"
        );
    }
}
