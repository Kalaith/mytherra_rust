//! Region, culture, and trade tuning (GDD 5.2).

use crate::data::ClimateType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionBalance {
    pub cost_multiplier: MultiplierCurve,
    pub effect_multiplier: MultiplierCurve,
    /// Divine resonance a region gains each time the player acts on it directly
    /// (Bless/Corrupt/Guide) — a god's repeated touch consecrates the land (GDD
    /// 5.2), making it cheaper and more responsive to future nudges (and more
    /// keenly felt by a roused pantheon). Player-driven only; the world's own
    /// drift never touches resonance.
    pub resonance_per_action: f32,
    pub status: StatusThresholds,
    pub drift: DriftParams,
}

/// A resonance-scaled multiplier: `clamp(min, max, 1 +/- (resonance-50) * coeff)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplierCurve {
    pub coeff: f32,
    pub min: f32,
    pub max: f32,
}

/// Thresholds that derive a region's status band from its stats (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusThresholds {
    pub wartorn_danger: f32,
    pub wartorn_chaos: f32,
    pub unrest_chaos: f32,
    pub thriving_prosperity: f32,
    pub thriving_chaos_max: f32,
    pub prospering_prosperity: f32,
    pub struggling_prosperity: f32,
}

/// Per-tick region drift parameters (GDD 5.2). Prosperity mean-reverts toward a
/// chaos/danger-derived equilibrium so the world settles dynamically instead of
/// climbing to the ceiling as every system stacks positive contributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftParams {
    pub prosperity_target_base: f32,
    pub prosperity_chaos_weight: f32,
    pub prosperity_danger_weight: f32,
    pub prosperity_reversion_rate: f32,
    pub chaos_target: f32,
    pub chaos_rate: f32,
    pub danger_target: f32,
    pub danger_rate: f32,
    /// Per-climate offset to the danger equilibrium (GDD 5.2): harsh lands
    /// (frozen, arid) settle more dangerous than hospitable ones, so an untended
    /// region keeps the character of its climate instead of every region
    /// relaxing to one shared baseline.
    pub climate_danger: ClimateDrift,
    pub magic_target: f32,
    /// Proportional pull toward `magic_target`, so magic — pushed up by
    /// knowledge artifacts / divination / the Growth deity — settles rather than
    /// pinning at the ceiling (mirrors the prosperity mean-reversion).
    pub magic_reversion_rate: f32,
}

/// A per-climate value, one field per `ClimateType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateDrift {
    pub temperate: f32,
    pub arid: f32,
    pub tropical: f32,
    pub frozen: f32,
    pub coastal: f32,
    pub highland: f32,
}

impl ClimateDrift {
    pub fn danger_offset(&self, climate: ClimateType) -> f32 {
        match climate {
            ClimateType::Temperate => self.temperate,
            ClimateType::Arid => self.arid,
            ClimateType::Tropical => self.tropical,
            ClimateType::Frozen => self.frozen,
            ClimateType::Coastal => self.coastal,
            ClimateType::Highland => self.highland,
        }
    }
}

/// Dynamic culture-scoring tuning (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultureBalance {
    /// A challenger must beat the incumbent by this to flip the dominant culture.
    pub inertia: f32,
    pub hero_weight: f32,
    pub landmark_weight: f32,
    pub resource_weight: f32,
    pub settlement_weight: f32,
    /// Culture score a building lends its region toward the culture it embodies
    /// (GDD 6 <-> 5.2): the works a people raise express and reinforce their
    /// character, so a land of forges hardens martial and one of temples turns
    /// mystical — a slow feedback the player can lean on or fight.
    pub building_weight: f32,
    /// How much a settlement's *size tier* amplifies its Mercantile pull (GDD
    /// 5.2): each tier above a hamlet adds this fraction, so a great city is a
    /// far stronger engine of commerce than a village of the same prosperity —
    /// urbanization erodes a region's older, rural identity. At 0 size is
    /// ignored (the old tier-blind behaviour).
    pub settlement_tier_weight: f32,
    /// Mercantile score per trade route touching a region (weighted by volume).
    pub trade_weight: f32,
    /// Culture score a living myth lends its home region, per myth, scaled by its
    /// resonance (GDD 5.2 <-> 5.6): a land's legends shape its character, so tales
    /// of valor make a martial people and tales of wonder a mystical one. The
    /// myth reinforces the culture its theme embodies.
    pub myth_weight: f32,
    /// Cultural-influence baseline and per-landmark bonus (the reversion target).
    pub influence_base: f32,
    pub influence_per_landmark: f32,
    /// Cultural influence a region's seated noble houses lend it, per point of
    /// their prestige (GDD 5.2 <-> 5.4): a land that is the seat of a great house
    /// is a renowned place, its lords' fame drawing eyes and envy. Folds into the
    /// same reversion target as landmark density, so it never accumulates without
    /// bound and ebbs as a house fades.
    pub influence_per_house_prestige: f32,
    pub influence_rate: f32,
    /// Per-tick stat aura a landmark radiates into its region, per point of its
    /// influence (GDD 5.2): a scholarly or mystical site deepens the arcane, a
    /// mercantile or pastoral one enriches, a martial one makes the land more
    /// perilous — so a notable place shapes its region's character, not just its
    /// culture.
    pub landmark_aura: f32,
    /// A flourishing, culturally-vibrant region raises a wonder over time (GDD
    /// 5.2): each tick an eligible region rolls `landmark_found_chance`; it must
    /// hold at least `landmark_found_prosperity` prosperity and
    /// `landmark_found_influence_min` cultural influence, and never grows past
    /// `landmark_max_per_region` wonders. A new wonder takes the region's culture
    /// and `landmark_found_influence`.
    pub landmark_found_chance: f32,
    pub landmark_found_prosperity: f32,
    pub landmark_found_influence_min: f32,
    pub landmark_max_per_region: usize,
    pub landmark_found_influence: f32,
    /// Fractional per-tick growth of a standing wonder's cultural stature (GDD
    /// 5.2): a landmark grows more storied the longer it endures, so an ancient
    /// wonder anchors its region's *identity* far more than a freshly-raised one.
    /// Stature starts at 1.0 and multiplies only a wonder's pull on the culture
    /// its theme embodies — its physical aura (the stat radiance of the structure
    /// itself) is unchanged, so a storied wonder shapes who a people are, not the
    /// weather over their heads.
    pub landmark_stature_growth: f32,
    /// The cultural stature a wonder tops out at, as a multiple of its founding
    /// pull.
    pub landmark_stature_cap: f32,
}

/// Trade-route tuning (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeBalance {
    /// Prosperity added to each endpoint per tick, per unit of route volume.
    pub prosperity_bonus: f32,
    /// Fraction each endpoint drifts toward the pair's average prosperity.
    pub equalize_rate: f32,
    /// Cultural influence added to each endpoint per tick, per unit of volume:
    /// ideas travel the trade network alongside wealth (GDD 5.2).
    pub culture_bonus: f32,
    /// Fraction each endpoint drifts toward the pair's average cultural influence.
    pub culture_equalize: f32,
    /// Fraction each endpoint drifts toward the pair's average magic affinity:
    /// arcana travels the roads too, so a connected, attuned land shares its
    /// arcane current with its partners (GDD 5.2 <-> 5.6). Trade only spreads
    /// magic between regions, never creates it — no flat bonus.
    pub magic_equalize: f32,
    /// Fraction each endpoint's granary drifts toward the pair's average harvest,
    /// throttled by route safety (GDD 5.2 <-> 5.3): grain travels the roads too,
    /// so a land with full stores feeds a hungrier trade partner — the grain trade
    /// that is a starving region's lifeline. Because it is throttled by the same
    /// peril that throttles wealth, war severing a road also severs the food that
    /// road carried, leaving a besieged land to starve alone. Trade only shares
    /// food between regions, never conjures it — no flat bonus.
    pub harvest_equalize: f32,
    /// How much the more perilous endpoint's danger throttles trade income:
    /// a route is only as safe as its worst leg, so caravans falter where the
    /// road runs through peril (GDD 5.2). Route safety is
    /// `clamp(1 - peril * peril_penalty - storm * storm_penalty, min_safety, 1)`.
    pub peril_penalty: f32,
    /// How much a foul weather front over either endpoint throttles trade, per
    /// unit of its magnitude (GDD 5.2 <-> 5.6): a storm blocks the roads and mires
    /// the caravans, so a tempest sitting over a trade partner cuts the wealth and
    /// the grain a route carries until it passes. War severs a road for good; a
    /// storm only closes it for a season.
    pub storm_penalty: f32,
    pub min_safety: f32,
    /// Effective route volume each living Merchant hero at either endpoint adds
    /// (GDD 5.2 <-> 5.4): a merchant plies the road, so a land's caravans carry
    /// more wealth for every trader who calls it home. This gives the Merchant
    /// role real economic weight — the counterpart to how a Warrior lends conquest
    /// might — so hero role now shapes the trade network, not only culture.
    pub merchant_volume_bonus: f32,
    /// Effective route volume each producing resource node at either endpoint adds
    /// (GDD 5.2 <-> 5.3): trade thrives where there is something to trade, so a
    /// road running between resource-rich lands carries fuller caravans than one
    /// between barren ones. A node run dry (Depleted) lends nothing, so the route's
    /// wealth rises and falls with the fortunes of the mines, farms, and forests
    /// that feed it — and a resource-rich region becomes a natural trade hub.
    pub resource_volume_bonus: f32,
    /// Per-tick chance a prospering region forges a new trade route (GDD 5.2):
    /// the trade network was the last part of the world to stay fixed while the
    /// map itself grows — a fractured, conquered, or frontier region was born
    /// economically isolated and never joined the roads. Now wealth reaches for
    /// wealth, so the caravan network grows with the map, the way towns, wonders,
    /// and resource nodes already do.
    pub found_chance: f32,
    /// Both endpoints of a newly forged route must clear this prosperity.
    pub found_min_prosperity: f32,
    /// A region joins at most this many routes, so the network densifies without
    /// every land wiring to every other.
    pub found_max_routes_per_region: usize,
    /// Starting volume of a forged route — thinner than the seeded arteries, a
    /// young road that thickens as its endpoints prosper and merchants ply it.
    pub found_volume: f32,
}

/// Bestiary tuning (GDD 5.2): the beasts that stalk the wild places. Peril breeds
/// them, they menace the land and raid its towns, and resident Warriors and
/// Rangers hunt them down — the embodied threat behind the abstract danger stat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterBalance {
    /// Base per-tick chance a beast emerges in an eligible region.
    pub emergence_chance: f32,
    /// Extra emergence chance per point of the region's danger — beasts breed
    /// where the wilds are perilous.
    pub emergence_danger_coeff: f32,
    /// A region needs at least this much danger to be wild enough to breed a
    /// beast at all.
    pub emergence_min_danger: f32,
    /// Emergence chance a region loses per level of the living Rangers dwelling in
    /// it (GDD 5.2 <-> 5.4): rangers ward the wilds, culling nascent threats and
    /// keeping the beasts at bay before they ever stalk forth. This is the Ranger
    /// role's own per-tick domain — the prevention that complements the hunting
    /// they lend against a beast already loose, and the wilderness counterpart to
    /// the Warrior's garrison and the Cleric's tended faith.
    pub ranger_ward: f32,
    /// A region whose magic affinity clears this line breeds arcane beasts
    /// (wyrms, shades) rather than natural predators.
    pub arcane_magic_threshold: f32,
    /// Most beasts that may stalk the world at once.
    pub max_active: usize,
    /// Ferocity a beast gains each tick it goes unopposed — an unchallenged beast
    /// grows into a terror.
    pub ferocity_growth: f32,
    /// Ferocity a beast loses per tick per level of the resident Warriors and
    /// Rangers hunting it.
    pub slay_per_might: f32,
    /// A beast worn below this ferocity has been slain or driven off.
    pub min_ferocity: f32,
    /// Renown the mightiest resident hunter earns for felling a beast (GDD 5.2
    /// <-> 5.4): a slain terror is a deed that makes a legend.
    pub slay_renown: f32,
    /// How effective a martial hunter (Warrior or Ranger) is against an *arcane*
    /// beast, as a fraction of their effect on a natural one (GDD 5.2 <-> 5.4):
    /// a wyrm or shade is a creature of magic that steel bites only weakly, so a
    /// purely martial land struggles to bring one down and may see it grow
    /// unchecked. Below 1.0; against natural beasts martial hunters are always
    /// fully effective.
    pub arcane_martial_effectiveness: f32,
    /// How effective a Mage is against an arcane beast, per level — the arcane is
    /// answered in kind, so a Mage is the surest bane of a wyrm. Mages lend
    /// nothing against a natural predator (a spell is no substitute for a spear).
    pub mage_arcane_effectiveness: f32,
    /// Ferocity at which a beast left unopposed too long ascends into a named
    /// legendary terror (GDD 5.2). Pitched high above any beast's starting
    /// ferocity, so it takes an age of unchecked growth in a land with no hunter
    /// to reach — the mark of a region abandoned to the wild, not an ordinary
    /// menace.
    pub apex_ferocity: f32,
    /// How much deadlier an ascended terror is: its per-tick danger and its raids
    /// on the towns are both multiplied by this, so a legendary beast ravages far
    /// beyond an ordinary one.
    pub apex_menace_mult: f32,
    /// Renown the hunter who fells an ascended terror earns instead of the
    /// ordinary `slay_renown` — slaying a legendary beast is the deed of a
    /// lifetime, worth a long stride toward legend on its own.
    pub apex_slay_renown: f32,
}

/// Inter-region war tuning (GDD 5.2): the prolonged conflicts that break out
/// between regions, draining both until one prevails. War fills the space between
/// the civilization system's one-sided rivalry pressure and the outright annexation
/// of conquest — a war doesn't remove a region, it wears one down, leaving the
/// loser scarred and ripe for the conquest that may follow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarBalance {
    /// Per-tick chance a belligerent region declares war on the realm's richest
    /// other region.
    pub ignite_chance: f32,
    /// A region's chaos + danger must reach this for it to be belligerent enough
    /// to make war.
    pub ignite_min_belligerence: f32,
    /// Most wars that may rage at once.
    pub max_active: usize,
    /// Intensity a fresh war ignites at.
    pub start_intensity: f32,
    /// Intensity lost each tick as both sides tire — war-weariness that ends every
    /// war in time.
    pub intensity_decay: f32,
    /// A war below this intensity has burned out and is resolved.
    pub min_intensity: f32,
    /// Prosperity a region at war loses each tick, per unit of the war's intensity.
    pub prosperity_toll: f32,
    /// Danger and chaos a region at war gains each tick, per unit of intensity.
    pub danger_toll: f32,
    pub chaos_toll: f32,
    /// Extra damage a side takes each tick per point of its opponent's war might
    /// (the combined levels of the enemy's Warriors and Rangers), scaled by
    /// intensity: facing a mightier foe is costlier, so martial strength decides
    /// who bleeds most. Applied to both prosperity (as loss) and danger (as gain).
    pub might_damage: f32,
    /// Fraction of a warring region's largest settlement razed each tick, per unit
    /// of intensity — the human cost of the fighting.
    pub raid_population: f32,
    /// War might within this margin of the enemy's counts as an even match, so the
    /// war ends in an exhausted stalemate rather than a clear victor.
    pub stalemate_margin: f32,
    /// Prosperity the loser forfeits and danger it takes on when a war is decided
    /// against it — the scar of defeat that leaves it ripe for conquest.
    pub loser_scar_prosperity: f32,
    pub loser_scar_danger: f32,
    /// War might a region gains per point of the power of the War-focus artifacts
    /// bound to it (GDD 5.2 <-> 5.6): a war relic is a weapon of the divine, so it
    /// lends its land strength in war as it already does in conquest — the player's
    /// lever over who prevails when regions come to blows.
    pub artifact_might: f32,
    /// Fraction of a sworn ally's own war might that it lends to a region fighting
    /// a war (GDD 5.2 <-> 5.2): an alliance is not only a promise not to fight but a
    /// pledge to fight beside — allies march to each other's defence, so a region
    /// with strong friends prevails in wars it would have lost alone. A region
    /// stands the more secure the more, and the mightier, its allies.
    pub ally_aid: f32,
}

/// Alliance tuning (GDD 5.2): the pacts that form between like-cultured,
/// trade-linked, peaceful regions. Amity to war's enmity — allies do not fall
/// upon one another, and each stands the more secure for the alliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PactBalance {
    /// Per-tick chance an eligible pair of regions forms an alliance.
    pub form_chance: f32,
    /// Both regions' chaos + danger must sit below this to be peaceable enough to
    /// forge a pact — the belligerent make no friends.
    pub form_max_belligerence: f32,
    /// Most alliances that may stand at once.
    pub max_active: usize,
    /// Chaos an allied region sheds each tick — the security of standing together,
    /// which also cools the belligerence that would drive it to war (GDD 5.2).
    pub chaos_relief: f32,
}
