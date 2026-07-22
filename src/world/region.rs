//! Runtime region state: the mutable, simulated form of a `RegionSeed`.

use crate::data::{
    ClimateType, ConquestBalance, Culture, HeroMightWeights, RegionActionDef, RegionBalance,
    RegionSeed,
};
use crate::world::Hero;
use serde::{Deserialize, Serialize};

/// Derived, at-a-glance health of a region. Recomputed from stats each tick
/// rather than stored authoritatively (GDD 5.2 crisis/thriving detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionStatus {
    Thriving,
    Prospering,
    Peaceful,
    Unrest,
    Struggling,
    WarTorn,
}

impl RegionStatus {
    pub fn label(self) -> &'static str {
        match self {
            RegionStatus::Thriving => "Thriving",
            RegionStatus::Prospering => "Prospering",
            RegionStatus::Peaceful => "Peaceful",
            RegionStatus::Unrest => "Unrest",
            RegionStatus::Struggling => "Struggling",
            RegionStatus::WarTorn => "War-torn",
        }
    }

    pub fn is_crisis(self) -> bool {
        matches!(self, RegionStatus::WarTorn | RegionStatus::Struggling)
    }
}

/// A 0-100 clamped world stat helper.
fn clamp_stat(value: f32) -> f32 {
    value.clamp(0.0, 100.0)
}

/// A region's core stats captured at the start of a tick, so the UI can show
/// which way each stat is currently moving (GDD 4 — surface cause and effect).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StatSnapshot {
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic_affinity: f32,
}

/// The live, simulated state of one region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub climate: ClimateType,
    pub culture: Culture,
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic_affinity: f32,
    pub population: f32,
    pub cultural_influence: f32,
    pub divine_resonance: f32,
    pub status: RegionStatus,
    /// Secession pressure (GDD 5.2): accumulates while the region is racked by
    /// chaos and danger, bleeds off when calm, and — given a hero to lead the
    /// revolt — eventually fractures the region in two (`sim/genesis.rs`).
    #[serde(default)]
    pub strife: f32,
    /// Food security (GDD 5.3): the fullness of the region's granaries, 0-100.
    /// Replenished by fair weather, prosperity, and a farming culture; drawn down
    /// by chaos and by more mouths than the land can feed. When it falls too low
    /// the region enters `famine`; `sim/famine.rs` owns the whole cycle.
    #[serde(default = "default_harvest")]
    pub harvest: f32,
    /// Whether the region is presently gripped by famine — starving, restive, and
    /// bleeding its people to safer lands (GDD 5.3). Hysteretic: set when harvest
    /// falls past the onset floor, cleared only once it climbs back to relief.
    #[serde(default)]
    pub famine: bool,
    /// Stats at the start of the current tick; `stat - prev.stat` is its trend.
    pub prev: StatSnapshot,
}

/// A region's starting granary fullness when a save predates the harvest field —
/// a comfortable surplus, so an old world doesn't load straight into famine.
fn default_harvest() -> f32 {
    70.0
}

impl Region {
    pub fn from_seed(seed: &RegionSeed, balance: &RegionBalance) -> Self {
        let prosperity = clamp_stat(seed.prosperity);
        let chaos = clamp_stat(seed.chaos);
        let danger = clamp_stat(seed.danger);
        let magic_affinity = clamp_stat(seed.magic_affinity);
        let mut region = Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            climate: seed.climate,
            culture: seed.culture,
            prosperity,
            chaos,
            danger,
            magic_affinity,
            population: seed.population.max(0.0),
            cultural_influence: clamp_stat(seed.cultural_influence),
            divine_resonance: clamp_stat(seed.divine_resonance),
            status: RegionStatus::Peaceful,
            strife: 0.0,
            harvest: default_harvest(),
            famine: false,
            prev: StatSnapshot {
                prosperity,
                chaos,
                danger,
                magic_affinity,
            },
        };
        region.refresh_status(balance);
        region
    }

    /// Record the current core stats as the trend baseline for this tick.
    pub fn snapshot_trend(&mut self) {
        self.prev = StatSnapshot {
            prosperity: self.prosperity,
            chaos: self.chaos,
            danger: self.danger,
            magic_affinity: self.magic_affinity,
        };
    }

    /// Cost multiplier from divine resonance: high-resonance regions are cheaper
    /// to nudge (GDD 5.2, tuned in `balance.json`).
    pub fn cost_multiplier(&self, balance: &RegionBalance) -> f32 {
        let curve = &balance.cost_multiplier;
        (1.0 - (self.divine_resonance - 50.0) * curve.coeff).clamp(curve.min, curve.max)
    }

    /// Effect multiplier from divine resonance: high-resonance regions respond
    /// more strongly (GDD 5.2, tuned in `balance.json`).
    pub fn effect_multiplier(&self, balance: &RegionBalance) -> f32 {
        let curve = &balance.effect_multiplier;
        (1.0 + (self.divine_resonance - 50.0) * curve.coeff).clamp(curve.min, curve.max)
    }

    /// Final favor cost of an action against this region.
    pub fn action_cost(&self, def: &RegionActionDef, balance: &RegionBalance) -> i64 {
        ((def.cost as f32 * self.cost_multiplier(balance)).round() as i64).max(1)
    }

    /// Apply an action's resonance-scaled stat deltas. Does not touch favor;
    /// callers debit the player after confirming affordability.
    pub fn apply_action(&mut self, def: &RegionActionDef, balance: &RegionBalance) {
        let mult = self.effect_multiplier(balance);
        self.prosperity = clamp_stat(self.prosperity + scaled(def.prosperity, mult));
        self.chaos = clamp_stat(self.chaos + scaled(def.chaos, mult));
        self.danger = clamp_stat(self.danger + scaled(def.danger, mult));
        self.magic_affinity = clamp_stat(self.magic_affinity + scaled(def.magic_affinity, mult));
        // Every divine touch attunes the land a little more (GDD 5.2): a region a
        // god shapes often grows in divine resonance, becoming cheaper and more
        // responsive to future nudges — and more keenly felt by a roused pantheon.
        // So concentrating favor on a land consecrates it, at the cost of tying
        // its fate more tightly to the heavens.
        self.divine_resonance = clamp_stat(self.divine_resonance + balance.resonance_per_action);
        self.refresh_status(balance);
    }

    /// Nudge cultural influence (from myth echoes), clamped 0-100.
    pub fn adjust_culture(&mut self, amount: f32) {
        self.cultural_influence = clamp_stat(self.cultural_influence + amount);
    }

    /// Nudge secession pressure, clamped at zero. Champions holding a region
    /// bleed strife; a champion's defeat feeds it (GDD 5.4 ↔ 5.2).
    pub fn adjust_strife(&mut self, amount: f32) {
        self.strife = (self.strife + amount).max(0.0);
    }

    /// Raise (or lower) divine resonance, clamped 0-100. A resident Cleric tends
    /// the land's faith without the player spending favor (GDD 5.4 ↔ 5.1) — the
    /// passive counterpart to consecration in `apply_action`.
    pub fn add_resonance(&mut self, amount: f32) {
        self.divine_resonance = clamp_stat(self.divine_resonance + amount);
    }

    /// Composite unrest pressure (GDD 5.6 omen formula), reused by champion
    /// rivalry resolution as the region's threat baseline.
    pub fn pressure(&self) -> f32 {
        Self::pressure_of(self.chaos, self.danger, self.prosperity)
    }

    /// The same pressure a tick ago (from the `prev` snapshot); `pressure() -
    /// prev_pressure()` is the drift the Omens horizon extrapolates.
    pub fn prev_pressure(&self) -> f32 {
        Self::pressure_of(self.prev.chaos, self.prev.danger, self.prev.prosperity)
    }

    fn pressure_of(chaos: f32, danger: f32, prosperity: f32) -> f32 {
        chaos * 0.38 + danger * 0.42 + (100.0 - prosperity) * 0.2
    }

    /// Projected military might (GDD 5.2): drawn from wealth, numbers, standing
    /// threat, and — for martial cultures — a warlike bonus. The currency of
    /// conquest; the single source of truth shared by the sim and the UI.
    pub fn might(&self, balance: &ConquestBalance) -> f32 {
        let martial = if self.culture == Culture::Martial {
            balance.might_martial_bonus
        } else {
            0.0
        };
        self.prosperity * balance.might_prosperity
            + self.population * balance.might_population
            + self.danger * balance.might_danger
            + martial
    }
}

/// The military might a region's resident heroes lend it (GDD 5.2): the summed
/// levels of its living heroes, each weighted by how martial its role is (a
/// warrior counts full, a loremaster barely) and scaled — a land guarded by many
/// capable *fighters* is mightier than its bare wealth and numbers imply, while
/// one held only by scholars and merchants, or whose champions have all fallen, is
/// easier to overrun. A free function because heroes live outside the region,
/// shared by the conquest sim and the region-detail readout so the shown might is
/// exactly the might that decides wars.
pub fn resident_might(
    heroes: &[Hero],
    region_id: &str,
    per_level: f32,
    weights: &HeroMightWeights,
) -> f32 {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.region_id == region_id)
        .map(|h| h.level as f32 * per_level * weights.get(h.role))
        .sum()
}

impl Region {
    /// Apply raw (already-computed) stat deltas, clamp, and refresh status.
    /// Used by systems other than divine actions (champion rivalries, artifacts).
    pub fn apply_deltas(
        &mut self,
        prosperity: f32,
        chaos: f32,
        danger: f32,
        magic: f32,
        balance: &RegionBalance,
    ) {
        self.prosperity = clamp_stat(self.prosperity + prosperity);
        self.chaos = clamp_stat(self.chaos + chaos);
        self.danger = clamp_stat(self.danger + danger);
        self.magic_affinity = clamp_stat(self.magic_affinity + magic);
        self.refresh_status(balance);
    }

    /// Recompute the derived status band from current stats (thresholds from
    /// `balance.json`).
    pub fn refresh_status(&mut self, balance: &RegionBalance) {
        let t = &balance.status;
        self.status = if self.danger >= t.wartorn_danger && self.chaos >= t.wartorn_chaos {
            RegionStatus::WarTorn
        } else if self.chaos >= t.unrest_chaos {
            RegionStatus::Unrest
        } else if self.prosperity >= t.thriving_prosperity && self.chaos < t.thriving_chaos_max {
            RegionStatus::Thriving
        } else if self.prosperity < t.struggling_prosperity {
            RegionStatus::Struggling
        } else if self.prosperity >= t.prospering_prosperity {
            RegionStatus::Prospering
        } else {
            RegionStatus::Peaceful
        };
    }
}

/// Scale a signed stat delta by an effect multiplier, preserving sign and
/// keeping a minimum magnitude of 1 for non-zero deltas (GDD 5.2).
fn scaled(delta: f32, mult: f32) -> f32 {
    if delta == 0.0 {
        return 0.0;
    }
    let magnitude = (delta.abs() * mult).round().max(1.0);
    magnitude.copysign(delta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> RegionSeed {
        RegionSeed {
            id: "t".to_owned(),
            name: "Test".to_owned(),
            climate: ClimateType::Temperate,
            culture: Culture::Scholarly,
            prosperity: 50.0,
            chaos: 50.0,
            danger: 50.0,
            magic_affinity: 50.0,
            population: 1000.0,
            cultural_influence: 50.0,
            divine_resonance: 50.0,
        }
    }

    fn even_weights(w: f32) -> HeroMightWeights {
        HeroMightWeights {
            warrior: w,
            mage: w,
            scholar: w,
            ranger: w,
            merchant: w,
            cleric: w,
        }
    }

    fn hero_of(region_id: &str, role: crate::data::HeroRole, level: u32, alive: bool) -> Hero {
        Hero {
            id: "h".to_owned(),
            name: "H".to_owned(),
            role,
            region_id: region_id.to_owned(),
            level,
            age: 30,
            is_alive: alive,
            renown: 0.0,
        }
    }

    #[test]
    fn resident_might_counts_only_living_heroes_at_home() {
        use crate::data::HeroRole::Warrior;
        let heroes = vec![
            hero_of("home", Warrior, 10, true),
            hero_of("home", Warrior, 4, true), // 14 living levels at home
            hero_of("home", Warrior, 100, false), // dead: lends no might
            hero_of("away", Warrior, 50, true), // elsewhere: lends no might here
        ];
        // Warrior weight 1.0 here, so (10 + 4) * 0.5 * 1.0 = 7.0.
        let w = even_weights(1.0);
        assert_eq!(resident_might(&heroes, "home", 0.5, &w), 7.0);
        assert_eq!(resident_might(&heroes, "nowhere", 0.5, &w), 0.0);
    }

    #[test]
    fn martial_roles_lend_more_might_than_scholarly_ones() {
        use crate::data::HeroRole::{Scholar, Warrior};
        let weights = HeroMightWeights {
            warrior: 1.0,
            scholar: 0.2,
            ..even_weights(0.5)
        };
        let warrior_land = vec![hero_of("r", Warrior, 10, true)];
        let scholar_land = vec![hero_of("r", Scholar, 10, true)];
        assert!(
            resident_might(&warrior_land, "r", 1.0, &weights)
                > resident_might(&scholar_land, "r", 1.0, &weights),
            "a warrior should lend more military might than a scholar of equal level"
        );
    }

    #[test]
    fn pressure_drift_tracks_worsening_stats() {
        let balance = balance().region;
        let mut region = Region::from_seed(&seed(), &balance);
        // Snapshot the calm baseline, then let danger and chaos climb.
        region.prev = StatSnapshot {
            prosperity: region.prosperity,
            chaos: region.chaos,
            danger: region.danger,
            magic_affinity: region.magic_affinity,
        };
        region.danger = 90.0;
        region.chaos = 80.0;
        // Pressure now exceeds the snapshot's, so the drift the omens read is
        // positive — the age is deepening.
        assert!(region.pressure() > region.prev_pressure());
    }

    fn bless() -> RegionActionDef {
        RegionActionDef {
            id: "bless".to_owned(),
            name: "Bless".to_owned(),
            description: String::new(),
            cost: 15,
            prosperity: 8.0,
            chaos: -4.0,
            danger: -3.0,
            magic_affinity: 0.0,
        }
    }

    fn balance() -> crate::data::Balance {
        crate::data::GameData::load().unwrap().balance
    }

    #[test]
    fn neutral_resonance_gives_base_cost_and_effect() {
        let b = balance();
        let mut region = Region::from_seed(&seed(), &b.region);
        assert_eq!(region.action_cost(&bless(), &b.region), 15);
        region.apply_action(&bless(), &b.region);
        assert!((region.prosperity - 58.0).abs() < f32::EPSILON);
        assert!((region.chaos - 46.0).abs() < f32::EPSILON);
        assert!((region.danger - 47.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_divine_touch_consecrates_the_land() {
        // Acting on a region raises its divine resonance, so a god's repeated
        // attention makes future nudges there cheaper and stronger (GDD 5.2).
        let b = balance();
        let mut region = Region::from_seed(&seed(), &b.region);
        let before = region.divine_resonance;
        let cost_before = region.action_cost(&bless(), &b.region);
        region.apply_action(&bless(), &b.region);
        assert!(
            region.divine_resonance > before,
            "a divine act should attune the land"
        );
        // Enough repeated attention lowers the cost of acting there.
        for _ in 0..20 {
            region.apply_action(&bless(), &b.region);
        }
        assert!(
            region.action_cost(&bless(), &b.region) < cost_before,
            "a consecrated land should be cheaper to nudge"
        );
        assert!(region.divine_resonance <= 100.0, "resonance stays clamped");
    }

    #[test]
    fn high_resonance_is_cheaper_and_stronger() {
        let b = balance();
        let mut s = seed();
        s.divine_resonance = 100.0;
        let region = Region::from_seed(&s, &b.region);
        assert!(region.action_cost(&bless(), &b.region) < 15);
        assert!(region.effect_multiplier(&b.region) > 1.0);
    }

    #[test]
    fn stats_clamp_to_valid_range() {
        let b = balance();
        let mut s = seed();
        s.prosperity = 98.0;
        let mut region = Region::from_seed(&s, &b.region);
        for _ in 0..10 {
            region.apply_action(&bless(), &b.region);
        }
        assert!(region.prosperity <= 100.0);
        assert!(region.danger >= 0.0);
    }
}
