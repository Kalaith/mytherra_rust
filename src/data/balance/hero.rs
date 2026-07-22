//! Hero lifecycle and migration tuning (GDD 5.4).

use crate::data::hero::HeroRole;
use serde::{Deserialize, Serialize};

/// Hero lifecycle tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroBalance {
    pub life_expectancy_base: f32,
    pub life_expectancy_per_level: f32,
    pub level_up: LevelUpCurve,
    pub death: DeathParams,
    pub move_chance: f32,
    pub migration: MigrationBalance,
    pub renown: RenownParams,
    /// Divine resonance each living Cleric raises in their home region per tick
    /// (GDD 5.4 <-> 5.1): a holy servant makes the land faithful, so the gods'
    /// will — and the player's own nudges — take hold there more keenly over time.
    /// This gives the Cleric role a domain of its own, the counterpart to a
    /// Merchant swelling trade and a Scholar hastening magic; unlike a player's
    /// consecration it costs no favor, accruing slowly wherever clerics dwell.
    pub cleric_resonance_per_tick: f32,
    /// Divine resonance a region gains each tick while it is gripped by an
    /// affliction — a famine or an active plague (GDD 5.1 <-> 5.3). Catastrophe
    /// drives the desperate to prayer: the frightened crowd the temples to beg
    /// deliverance, and a suffering land turns to the gods where a comfortable one
    /// forgets them. This is the faith economy's one response to the world's
    /// scourges — suffering as a wellspring of devotion, not only of death.
    pub affliction_resonance_per_tick: f32,
    /// Danger a region loses per tick per level of the living Warriors garrisoned
    /// in it (GDD 5.4 <-> 5.2): fighting heroes keep the everyday peace, so a land
    /// defended by seasoned warriors grows safer over time. This is the passive,
    /// day-to-day counterpart to the conquest might those same warriors lend when
    /// a border war comes (`resident_might`) — the Warrior role's per-tick domain
    /// beside the Cleric's faith and the Merchant's trade.
    pub warrior_danger_relief: f32,
}

/// Hero fame tuning (GDD 5.4): how renown accrues, the danger-death it staves
/// off, and the ascending renown thresholds at which each title in
/// `strings.heroes.renown_titles` is earned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenownParams {
    /// Renown gained each time the hero gains a level.
    pub per_level: f32,
    /// Renown gained for surviving an era transition.
    pub per_era: f32,
    /// Danger-death chance shaved off per point of renown — a legend clings to
    /// life against the odds (never below the death floor).
    pub survival_coeff: f32,
    /// Ascending renown needed for each title (index-aligned with the titles).
    pub thresholds: Vec<f32>,
}

/// Hero migration tuning: where a hero that decides to move goes is no longer
/// uniform-random — each role is drawn to different region stats, so warriors
/// flow toward conflict, mages toward magic, scholars toward settled culture,
/// and rangers toward wilder lands. This ties heroes into the region and genesis
/// systems: heroes abandoning a war-torn region leave it undefended (more
/// conquerable), while thriving regions gather the veterans who found frontiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationBalance {
    /// Baseline pull every region has before stat weighting.
    pub base_weight: f32,
    /// Floor on a region's computed pull, so it is never zero or negative.
    pub min_weight: f32,
    /// Extra pull, per wonder of the hero's own culture standing in a region:
    /// great works draw the kind of people who raised them, so a mage is drawn to
    /// a land of mystical wonders (GDD 5.4 <-> 5.2). This makes a region's wonders,
    /// its heroes, and its culture reinforce one another.
    pub wonder_pull: f32,
    /// Extra pull, per tier of a region's greatest city (GDD 5.4 <-> 5.3): heroes
    /// are drawn to the great cities, where fame, fortune, and patrons gather — so
    /// a metropolis is a beacon a scattering of villages is not, and a region that
    /// nurtures a city draws the heroes who then defend and enrich it.
    pub city_pull: f32,
    /// Extra pull, per point of the renown of a region's most famed living hero
    /// (GDD 5.4): heroes flock to where legends dwell — a land home to a champion
    /// or a living legend draws the ambitious, who come to serve, to learn, and to
    /// share in the glory. So greatness gathers where greatness already is: a
    /// famed hero, a cultivated champion, or the storied scion of a noble house
    /// makes their region a beacon, concentrating the talent that raises the next
    /// legend there in turn.
    pub fame_pull: f32,
    pub roles: RoleMigrationWeights,
}

/// Per-role stat weighting for migration attractiveness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMigrationWeights {
    pub warrior: StatWeights,
    pub mage: StatWeights,
    pub scholar: StatWeights,
    pub ranger: StatWeights,
    pub merchant: StatWeights,
    pub cleric: StatWeights,
}

impl RoleMigrationWeights {
    pub fn get(&self, role: HeroRole) -> &StatWeights {
        match role {
            HeroRole::Warrior => &self.warrior,
            HeroRole::Mage => &self.mage,
            HeroRole::Scholar => &self.scholar,
            HeroRole::Ranger => &self.ranger,
            HeroRole::Merchant => &self.merchant,
            HeroRole::Cleric => &self.cleric,
        }
    }
}

/// How strongly each region stat draws (positive) or repels (negative) a hero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatWeights {
    pub prosperity: f32,
    pub danger: f32,
    pub magic: f32,
    pub culture: f32,
    /// Pull toward a region's divine resonance (GDD 5.4 <-> 5.1): the devout make
    /// pilgrimage to hallowed lands, so a Cleric is drawn to faithful ground above
    /// all — the counterpart to a warrior drawn to danger and a scholar to culture.
    /// It completes the cleric's own loop: they tend a land's faith, faith draws
    /// more of them, and the faithful land tithes its god. `serde(default)` keeps
    /// the other roles (which don't answer this call) at zero without touching
    /// their data.
    #[serde(default)]
    pub resonance: f32,
}

/// Per-tick level-up probability curve: `base * tier_mult * decay^(level-1)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUpCurve {
    pub base_chance: f32,
    pub low_tier_max_level: u32,
    pub high_tier_min_level: u32,
    pub low_tier_mult: f32,
    pub mid_tier_mult: f32,
    pub high_tier_mult: f32,
    pub decay: f32,
    /// Trial by fire (GDD 5.4): a hero forged in a dangerous land grows faster.
    /// Level-up chance is scaled by `1 + region danger * crucible_coeff`, so a
    /// warrior who flows toward peril is tempered by it.
    pub crucible_coeff: f32,
    /// A hero in their element grows faster (GDD 5.4 <-> 5.2): when a hero's kin
    /// culture matches the region's dominant culture — a warrior in a martial
    /// land, a mage in a mystical one — their level-up chance gains this fraction.
    /// So a land's character shapes how fast the heroes who suit it rise.
    pub culture_match_bonus: f32,
    /// Only levels that are a multiple of this are worth a chronicle line, so the
    /// Event Log marks a hero's milestones rather than every step of a steady
    /// climb (GDD 10). Heroes still gain every level and its renown silently.
    pub chronicle_interval: u32,
}

/// Per-tick death roll parameters (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathParams {
    pub elder_roll: f32,
    pub danger_divisor: f32,
    pub level_divisor: f32,
    pub min_chance: f32,
}

/// Noble-house tuning (GDD 5.4): the great bloodlines legends found and their
/// heirs carry across the ages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseBalance {
    /// Prestige a freshly founded house begins with, as a fraction of its
    /// founder's renown — a house is only ever as storied as the legend who
    /// raised it.
    pub found_prestige_fraction: f32,
    /// How fast a house's prestige drifts toward the summed renown of its living
    /// members each tick: it swells while the line thrives and ebbs once its
    /// blood thins.
    pub prestige_rate: f32,
    /// A house with no living members and prestige below this floor is at last
    /// forgotten.
    pub fade_floor: f32,
    /// Renown an heir inherits at birth, as a fraction of their house's prestige
    /// — the blood of legends is a head start toward legend of one's own.
    pub inherit_fraction: f32,
}

/// Great-Order tuning (GDD 5.4): the world's professional fellowships, the
/// institutional counterpart to the hereditary House. Where a House is a
/// bloodline in one region, an Order is a calling spanning every region its kind
/// dwell in — arising when a role reaches a critical mass of the living, drawing
/// its standing from the fellowship's numbers, and lending cultural weight to
/// each region that hosts a chapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBalance {
    /// Living heroes of a calling the world must hold before that Order is
    /// founded — a fellowship needs numbers to become an institution.
    pub found_min_members: usize,
    /// An Order whose living membership falls to this many is dissolved, its
    /// ranks too thin to endure.
    pub dissolve_min_members: usize,
    /// Prestige the Order's standing drifts toward per living member each tick.
    pub prestige_per_member: f32,
    /// How fast prestige drifts toward that target — it swells as the calling
    /// spreads and ebbs as its ranks thin.
    pub prestige_rate: f32,
    /// Ceiling on an Order's prestige, so a populous calling cannot lend
    /// unbounded weight.
    pub prestige_cap: f32,
    /// Cultural influence an Order lends each of its chapter regions per tick per
    /// point of prestige (GDD 5.4 <-> 5.2): an institution is a cultural force,
    /// making the regions it reaches more prominent — drawing heroes and raising
    /// the chance of wonders — without touching the crisis stats.
    pub influence_per_prestige: f32,
    /// Renown each living member gains per tick per point of the Order's prestige
    /// (GDD 5.4): belonging to a great fellowship is itself a distinction, so a
    /// storied Order lends its fame to its own and speeds them toward legend. A
    /// young Order confers little; the effect ramps with the Order's standing.
    pub renown_per_prestige: f32,
}

/// Sainthood tuning (GDD 5.1 <-> 5.4): the veneration of the great dead, the faith
/// legacy to set beside the House's bloodline and the Order's calling. Pitched so
/// only the genuinely holy or the genuinely legendary are ever raised, and the
/// memory that hallows a land fades over generations rather than forever.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaintBalance {
    /// Renown a dead hero must have earned to be considered for sainthood. A
    /// Cleric who clears it is venerated for their holiness; a hero of any other
    /// calling must reach the legend bar besides, so sainthood is the reward of
    /// the holy or the truly great, not of every renowned soul.
    pub renown_threshold: f32,
    /// The veneration a fresh saint is canonized with — the height of the faith's
    /// devotion, before the slow fade of memory begins.
    pub start_veneration: f32,
    /// Veneration a saint loses each tick as memory fades toward the mundane past.
    pub veneration_decay: f32,
    /// A saint whose veneration ebbs below this has passed from living memory and
    /// is forgotten.
    pub forgotten_floor: f32,
    /// Divine resonance a saint lends its home region each tick, per point of its
    /// veneration (GDD 5.1): the remembered example of a holy soul hallows the land
    /// that keeps it, the more so the fresher and fiercer the devotion.
    pub resonance_per_veneration: f32,
}
