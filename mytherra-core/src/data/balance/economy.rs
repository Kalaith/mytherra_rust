//! Resource-node and settlement tuning (GDD 5.3).

use crate::data::resource::ResourceStatus;
use serde::{Deserialize, Serialize};

/// Pestilence tuning (GDD 5.3): the dark counterweight to the world's growth
/// systems. Crowded, squalid lands breed disease; it saps their people and
/// wealth, leaps along the trade roads that carry everything else, and burns out
/// as immunity builds — fastest where the land is prosperous enough to tend its
/// sick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlagueBalance {
    /// Base per-tick chance a plague breaks out in an eligible region.
    pub outbreak_chance: f32,
    /// A region needs at least this population for an epidemic to take hold —
    /// disease needs a crowd.
    pub outbreak_min_population: f32,
    /// Prosperity at or below which squalor breeds pestilence; above it the
    /// squalor term contributes nothing.
    pub squalor_prosperity: f32,
    /// How steeply the outbreak chance rises per point of prosperity below the
    /// squalor line — a destitute, crowded land is a tinderbox.
    pub squalor_coeff: f32,
    /// Extra outbreak chance in a region gripped by famine (GDD 5.3 <-> 5.3):
    /// the starving are weakened and packed into whatever haven still has bread,
    /// so pestilence takes hold far more readily. Famine and plague ride together.
    pub famine_outbreak_chance: f32,
    /// Multiplier on the demographic toll a plague exacts in a famine-struck
    /// region: a weakened, starving people dies of disease faster than a fed one.
    pub famine_toll_mult: f32,
    /// Severity a fresh outbreak begins at.
    pub start_severity: f32,
    /// Population fraction the region's largest settlement loses per tick per
    /// unit of severity — the pestilence's demographic toll.
    pub toll_population: f32,
    /// Prosperity the region loses per tick per unit of severity.
    pub toll_prosperity: f32,
    /// Danger the region gains per tick per unit of severity — a plague-stricken
    /// land is a perilous one.
    pub toll_danger: f32,
    /// Per-tick chance an active plague leaps down a trade route to an
    /// unafflicted connected region (GDD 5.3 <-> 5.2): contagion travels the same
    /// caravan roads that carry wealth, ideas, and arcana.
    pub spread_chance: f32,
    /// Severity a spread outbreak begins at, as a fraction of its parent's.
    pub spread_severity_fraction: f32,
    /// Severity lost each tick as the sick recover or die and immunity builds.
    pub decay_base: f32,
    /// Extra severity decay per point of the region's prosperity — a wealthy land
    /// tends its sick and throws off the pestilence sooner.
    pub decay_prosperity_coeff: f32,
    /// Extra severity decay per living Cleric dwelling in the afflicted region
    /// (GDD 5.3 <-> 5.4): the devout tend the sick, so a land served by healers
    /// throws off a plague faster. This gives the Cleric role a second domain
    /// beside the faith it already nurtures — the counterpart to the Merchant's
    /// trade weight and the Warrior's conquest might.
    pub cleric_relief: f32,
    /// A plague below this severity has burned out and is forgotten.
    pub min_severity: f32,
}

/// Refugee tuning (GDD 5.3): when a land grows too perilous to bear — wracked by
/// danger, gripped by plague, or stalked by a beast — its people don't only die
/// in place, they flee. The masses flow toward the safest, most prosperous haven,
/// so the threats reshape the map's population, not merely thin it. The
/// population-flow counterpart to trade's wealth-flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefugeeBalance {
    /// Region peril at or above which its settlements begin to shed refugees.
    /// Peril is `danger + plague_peril (if plagued) + monster_peril (if stalked)`.
    pub flee_threshold: f32,
    /// Peril a present plague adds to a region.
    pub plague_peril: f32,
    /// Peril a stalking beast adds to a region.
    pub monster_peril: f32,
    /// Peril a region in the grip of famine adds (GDD 5.3 <-> 5.3): the starving
    /// take to the roads, so a dearth drives flight just as danger and plague do.
    pub famine_peril: f32,
    /// Fraction of a fleeing settlement's people who leave each tick, per unit of
    /// the region's peril as a fraction of 100.
    pub flee_rate: f32,
    /// A region must sit below this peril to be a haven refugees will flee to.
    pub haven_max_peril: f32,
    /// A single tick's flight from one settlement this large or larger is worth a
    /// line in the chronicle; smaller trickles pass unremarked.
    pub notable_flight: f32,
    /// Prosperity the haven region loses per refugee it takes in each tick (GDD
    /// 5.3): a mass influx strains the local economy — more mouths than the land
    /// was feeding. Because the haven is chosen by prosperity, this strain is also
    /// the brake on runaway concentration: a swollen haven's falling prosperity
    /// eventually cedes haven status to somewhere less crowded, spreading the
    /// flow rather than piling every refugee into one city forever.
    pub haven_strain: f32,
}

/// Resource-node tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBalance {
    pub stress_chaos: f32,
    pub stress_danger: f32,
    pub degrade_base: f32,
    pub degrade_stress: f32,
    pub recover_base: f32,
    pub improve_base: f32,
    pub contest_chaos_threshold: f32,
    pub corrupt_base: f32,
    pub corrupt_danger: f32,
    pub region_output_scale: f32,
    /// A manaspring is a wellspring of the arcane, not the granary a farm or mine
    /// is (GDD 5.3 <-> 5.6): its yield feeds the region's magic affinity rather
    /// than its prosperity, scaled by this. So an arcane resource makes a mystical
    /// land — and a corrupted manaspring drains it — giving the resource type a
    /// role beyond its economic output.
    pub manaspring_magic_scale: f32,
    /// A hazardous node poisons its region, not just its ledger (GDD 5.3): a
    /// corrupted node bleeds chaos as the taint spreads, an unstable one bleeds
    /// danger. This feeds the very stress that degraded it, so a neglected node
    /// can drag its region down with it until the region is calmed.
    pub corrupted_chaos: f32,
    pub unstable_danger: f32,
    /// Resource discovery (GDD 5.3): a prospering, populous region occasionally
    /// opens a wholly new node — the counterpart to settlement founding, and the
    /// way a frontier region born resource-barren eventually develops its own
    /// wealth. Per-region chance each tick, gated on prosperity and population,
    /// capped per region. A discovered node starts Active (output 1.0, so it adds
    /// nothing at once — only the potential to flourish), and its type follows the
    /// region's culture (`Culture::favored_resource`).
    pub discovery_chance: f32,
    pub discovery_min_prosperity: f32,
    pub discovery_min_population: f32,
    pub discovery_max_per_region: usize,
    pub outputs: ResourceOutputs,
}

/// Output multiplier per resource status (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOutputs {
    pub active: f32,
    pub blessed: f32,
    pub flourishing: f32,
    pub overworked: f32,
    pub contested: f32,
    pub corrupted: f32,
    pub unstable: f32,
    pub depleted: f32,
}

impl ResourceOutputs {
    pub fn get(&self, status: ResourceStatus) -> f32 {
        match status {
            ResourceStatus::Active => self.active,
            ResourceStatus::Blessed => self.blessed,
            ResourceStatus::Flourishing => self.flourishing,
            ResourceStatus::Overworked => self.overworked,
            ResourceStatus::Contested => self.contested,
            ResourceStatus::Corrupted => self.corrupted,
            ResourceStatus::Unstable => self.unstable,
            ResourceStatus::Depleted => self.depleted,
        }
    }
}

/// Settlement growth tuning (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBalance {
    pub base_growth: f32,
    pub self_prosperity_div: f32,
    pub region_prosperity_div: f32,
    pub region_chaos_div: f32,
    pub growth_min: f32,
    pub growth_max: f32,
    /// Extra prosperity a building lends its settlement when its region holds a
    /// producing node of the resource it draws on (GDD 6 <-> 5.3): a Forge over
    /// ore, a Harbor over a fishery. Industry pays off most where its raw material
    /// lies at hand, so resource-rich regions reward building to match.
    pub building_synergy_bonus: f32,
    /// Carrying capacity per point of the settlement's supporting prosperity
    /// (region prosperity + its buildings): the land feeds only so many, so
    /// growth eases to nothing as population nears capacity (GDD 5.3).
    pub capacity_per_prosperity: f32,
    /// Population below which a settlement is abandoned and removed — a town bled
    /// dry by an age of war and famine finally empties out, rather than lingering
    /// forever as a near-empty ghost town (GDD 5.3).
    pub abandon_population: f32,
    /// A prosperous, populous region founds a new town over time (GDD 5.3): each
    /// tick an eligible region rolls `found_chance`; it must be at least
    /// `found_status_min` prosperity and hold more than `found_min_region_pop`
    /// souls, and never grows past `found_max_per_region` towns. A new town starts
    /// with `found_population` settlers, drawn from the region's people.
    pub found_chance: f32,
    pub found_status_min: f32,
    pub found_min_region_pop: f32,
    pub found_max_per_region: usize,
    pub found_population: f32,
    pub prosperity_drift_rate: f32,
    pub region_contribution: f32,
    /// A settlement builds a new building only once its prosperity and
    /// population clear these floors (GDD 6 — buildings grow with settlements).
    pub construction_prosperity_min: f32,
    pub construction_population_min: f32,
    /// Per-tick chance an eligible settlement raises one new building.
    pub construction_chance: f32,
    /// Extra selection weight a building type gets when it matches its region's
    /// dominant culture, so a martial land forges and a mercantile one trades.
    pub culture_affinity_weight: f32,
    /// Ascending population thresholds that sort a settlement into a size tier
    /// (GDD 5.3): with N thresholds there are N+1 tiers, named by
    /// `strings.ui.settlement_tiers`. A settlement's tier is the count of
    /// thresholds its population meets or exceeds, so crossing one — a village
    /// swelling into a town, or a city dwindling back — is a chronicled milestone.
    pub tier_thresholds: Vec<f32>,
}

/// Famine tuning (GDD 5.3): the food economy beneath every other system. A
/// region's granaries fill from fair weather, prosperity, and a farming culture,
/// and empty under chaos and the sheer weight of its people. When they run dry
/// the land starves — restive, poorer, and shedding its people to safer ground —
/// until the harvest recovers. Deterministic: harvest is read straight from
/// world state, no roll decides a dearth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamineBalance {
    /// Baseline granary refill each tick — the land's own fertility, before any
    /// strain. A calm, tolerably prosperous region has no strain, so this alone
    /// fills its granary back to plenty.
    pub base_regrowth: f32,
    /// Harvest each producing farmland or fishery adds per tick, scaled by the
    /// node's status output multiplier (a flourishing field feeds more than a
    /// struggling one, a depleted node nothing). The fields and the sea are what
    /// truly fill a granary; a food-rich region rarely starves.
    pub harvest_per_food_node: f32,
    /// Extra harvest a pastoral (farming/herding) region gathers each tick.
    pub pastoral_bonus: f32,
    /// Divine resonance above which a land's harvest is blessed (GDD 5.3 <-> 5.1);
    /// set above the neutral 50 so only genuine devotion counts, not an ordinary
    /// land's baseline faith.
    pub resonance_blessing_floor: f32,
    /// Harvest gained per point of resonance above the blessing floor — the gods'
    /// answer to the prayers a famine stirs. Kept gentle: the resonance a dearth
    /// raises accrues slowly, so this is a tailwind out of a long famine for the
    /// devout, never immunity from its onset.
    pub harvest_per_resonance: f32,
    /// Harvest gained per unit of a fair weather front's prosperity effect times
    /// its magnitude (lost to a foul one): a good season feeds, a storm blights.
    pub weather_coeff: f32,
    /// Chaos below which disorder costs the granary nothing; only war and unrest
    /// *past* this comfort line spoil stores and leave fields untended. The world
    /// self-regulates into a moderate band, so a flat chaos drain would either
    /// never bite or starve everyone — the threshold makes famine a mark of the
    /// genuinely troubled land, not the ordinary one.
    pub chaos_comfort: f32,
    /// Harvest drawn down per point of chaos above `chaos_comfort`.
    pub chaos_strain: f32,
    /// Prosperity above which want costs the granary nothing; only true poverty
    /// *below* this line starves a land faster than it can farm.
    pub prosperity_comfort: f32,
    /// Harvest drawn down per point of prosperity below `prosperity_comfort`.
    pub dearth_strain: f32,
    /// Harvest at or below which a fed region tips into famine.
    pub onset: f32,
    /// Harvest a starving region must climb back to before the famine breaks;
    /// kept above `onset` so a dearth doesn't flicker on the threshold.
    pub relief: f32,
    /// Chaos a famine adds to its region each tick — hunger breeds unrest.
    pub famine_chaos: f32,
    /// Prosperity a famine strips from its region each tick.
    pub famine_prosperity: f32,
    /// Fraction of a settlement's people lost each tick of famine — the slow toll
    /// of starvation, on top of the flight the refugee system drives.
    pub famine_mortality: f32,
}

/// Lore tuning (GDD 5.6 <-> 5.3): the accumulated practical knowledge of a
/// civilization — the mundane wisdom, distinct from arcane power, that lets a
/// learned land tend its sick through a plague and store its grain against a
/// dearth. Each region's lore drifts toward a target set by its scholars, its
/// libraries, the magic its world has mastered, and the wealth that affords study.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreBalance {
    /// The knowledge every settled land holds simply by being inhabited — the
    /// baseline the target is built up from.
    pub base: f32,
    /// Lore-target contribution per living Scholar or Mage dwelling in the region:
    /// the learned are the wellspring of a land's knowledge.
    pub per_scholar: f32,
    /// Lore-target contribution per scholarly or mystical landmark in the region,
    /// scaled by its cultural weight (influence x stature): the great libraries and
    /// colleges are the storehouses of a civilization's learning.
    pub per_learned_landmark: f32,
    /// Lore-target contribution per magic path the world has brought to Known: the
    /// mastery of the arcane lifts the whole world's understanding.
    pub per_known_path: f32,
    /// Lore-target contribution per point of region prosperity above the neutral
    /// baseline: a wealthy land can afford scholars, schools, and the leisure to
    /// learn, where a destitute one cannot.
    pub prosperity_coeff: f32,
    /// How fast a region's lore drifts toward its target each tick — knowledge is
    /// slow to gather and slow to lose.
    pub drift_rate: f32,
    /// Fraction of a plague's demographic toll a fully-learned region (lore 100)
    /// averts: a land that knows medicine tends its sick, so fewer die (GDD 5.6 <->
    /// 5.3). Scaled linearly by lore, so a half-learned land averts half as much.
    pub plague_toll_relief: f32,
    /// Fraction of a famine's mortality a fully-learned region averts: a land that
    /// knows to store grain, rotate fields, and ration in dearth loses fewer to
    /// starvation (GDD 5.6 <-> 5.3). Scaled linearly by lore.
    pub famine_mortality_relief: f32,
}
