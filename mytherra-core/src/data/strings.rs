//! All user-facing text, loaded from `strings.json`.
//!
//! Copy and templated messages live in JSON, not Rust literals, per the
//! data-driven rule. Templates use `{name}` placeholders filled at runtime by
//! [`fill`]. Enum-variant display names (climate, culture, status) stay as
//! `.label()` methods on their types — those are canonical type formatting, not
//! authored content.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strings {
    pub panels: Panels,
    pub stats: Stats,
    pub ui: UiText,
    pub heroes: HeroText,
    pub betting: BettingText,
    pub divine: DivineText,
    pub eras: EraText,
    pub event_log: EventLogText,
    pub settings: SettingsText,
    pub notifications: Notifications,
    pub chronicle: ChronicleText,
    pub genesis: GenesisText,
    pub title: TitleText,
    pub orders: OrderNames,
    pub prophecies: ProphecyNames,
    pub festivals: FestivalNames,
}

/// The names the world's great celebrations take, drawn from in turn (GDD 5.2 <->
/// 6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FestivalNames {
    pub names: Vec<String>,
}

impl FestivalNames {
    /// The name for the `seq`-th festival held, cycling through the bank so a long
    /// age draws them in a fixed, repeatable order — deterministic, no roll.
    pub fn pick(&self, seq: u64) -> &str {
        if self.names.is_empty() {
            return "the Grand Festival";
        }
        &self.names[(seq as usize - 1) % self.names.len()]
    }
}

/// The name each pole of prophecy takes when spoken (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProphecyNames {
    pub doom: String,
    pub golden_age: String,
    pub age_of_magic: String,
}

impl ProphecyNames {
    pub fn for_kind(&self, kind: crate::world::ProphecyKind) -> &str {
        use crate::world::ProphecyKind;
        match kind {
            ProphecyKind::Doom => &self.doom,
            ProphecyKind::GoldenAge => &self.golden_age,
            ProphecyKind::AgeOfMagic => &self.age_of_magic,
        }
    }
}

/// The name each of the six great Orders takes, one per hero calling (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderNames {
    pub warrior: String,
    pub mage: String,
    pub scholar: String,
    pub ranger: String,
    pub merchant: String,
    pub cleric: String,
}

impl OrderNames {
    /// The name of the Order bound to a given calling.
    pub fn for_role(&self, role: crate::data::HeroRole) -> &str {
        use crate::data::HeroRole;
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

/// Copy for the title / main menu screen (GDD 10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleText {
    pub game_title: String,
    pub tagline: String,
    pub new_game: String,
    #[serde(rename = "continue")]
    pub continue_game: String,
    pub settings: String,
    pub exit: String,
    pub main_menu: String,
    pub no_save: String,
}

/// Copy for region genesis — breakaway naming and the region-detail strife line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisText {
    /// Name templates for a breakaway region; `{parent}` is the origin's name.
    pub breakaway_names: Vec<String>,
    /// Name templates for a founded frontier; `{parent}` and `{hero}` fill in.
    pub frontier_names: Vec<String>,
    /// Region-detail line shown while secession pressure is brewing.
    pub strife_line: String,
    /// Word shown after `strife_line` describing how close a fracture is.
    pub strife_simmering: String,
    pub strife_seething: String,
    pub strife_breaking: String,
    /// Region-detail military-might line; `{might}` is the computed value.
    pub might_line: String,
    /// Region-detail line naming the region's current dominant agenda (its
    /// prevailing course); `{agenda}` is the agenda name.
    pub course_line: String,
    /// Genesis-outlook lines on the region detail (one shown at a time).
    pub outlook_frontier: String,
    pub outlook_frontier_eager: String,
    pub outlook_vulnerable: String,
    pub outlook_bracing: String,
    pub outlook_defended: String,
    pub outlook_warded: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsText {
    pub panel: String,
    pub tick_speed_title: String,
    pub tick_speed_hint: String,
    pub speed_chip: String,
    pub pacing_title: String,
    pub pause: String,
    pub resume: String,
    pub status_running: String,
    pub status_paused: String,
    pub world_title: String,
    pub info_display: String,
    pub info_version: String,
    pub info_seed: String,
    pub info_year: String,
    /// Achievements column header; slots `{done}` / `{total}`.
    pub achievements_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLogText {
    pub panel: String,
    pub filter_all: String,
    pub filter_label: String,
    pub count_line: String,
    pub empty_filtered: String,
    /// Pager: "Page {page} / {pages}" and the two nav buttons.
    pub page_label: String,
    pub prev_page: String,
    pub next_page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panels {
    pub world: String,
    pub chronicle: String,
    pub regions: String,
    pub standing: String,
    pub divine_actions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub prosperity: String,
    pub chaos: String,
    pub danger: String,
    pub magic: String,
    pub culture: String,
    pub resonance: String,
    pub population: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraText {
    pub current_title: String,
    pub triggers_title: String,
    pub history_title: String,
    pub no_history: String,
    pub era_line: String,
    pub since: String,
    pub pressure: String,
    pub breaking: String,
    pub holding: String,
    /// Dashboard portent: which trigger the era pressure is building toward.
    pub trending: String,
    pub record_line: String,
    pub record_span: String,
    /// The age's human toll, shown beneath each chronicle record.
    pub record_toll: String,
    pub record_wonders: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroText {
    pub panel: String,
    pub empty: String,
    pub count: String,
    pub level: String,
    pub life: String,
    pub alive: String,
    pub fallen: String,
    pub champions_title: String,
    pub no_champions: String,
    pub designate: String,
    pub champion_tag: String,
    pub cultivate: String,
    pub focus_cycle: String,
    pub champion_meta: String,
    pub quest: String,
    pub roster_label: String,
    pub filter_all: String,
    pub focus_line: String,
    pub focus_effect_valor: String,
    pub focus_effect_wisdom: String,
    pub focus_effect_devotion: String,
    /// Suffix on a champion's focus line when its focus suits its hero's role, so
    /// the synergy bonus (GDD 5.4) is visible as a reason to match them.
    pub focus_in_tune: String,
    /// Earned renown titles, ascending (index-aligned with hero.renown.thresholds).
    pub renown_titles: Vec<String>,
    /// Roster meta line for a titled hero.
    pub titled_meta: String,
    pub untitled_meta: String,
    /// Appended to a living hero's meta line when their calling suits their land's
    /// dominant culture — they grow faster there (GDD 5.4).
    pub in_element: String,
    /// Renown meter labels: climbing toward the next title, or already a legend.
    pub renown_meter: String,
    pub renown_meter_max: String,
    /// Roster pager: "Page {page} / {pages}" and the two nav buttons.
    pub page_label: String,
    pub prev_page: String,
    pub next_page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingText {
    pub panel_events: String,
    pub panel_bets: String,
    pub no_events: String,
    pub no_bets: String,
    pub confidence_btn: String,
    pub stake_btn: String,
    pub place: String,
    pub odds: String,
    pub deadline: String,
    pub pending: String,
    pub won: String,
    pub lost: String,
    pub bet_line: String,
    pub bet_meta: String,
    pub bets_more: String,
    /// Track-record header: wins/losses, net favor, pending count.
    pub record: String,
    /// Target label for the world-scale "age ends" wager (no entity target).
    pub age_target: String,
    /// Target label for the world-scale "a new land rises" wager.
    pub frontier_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivineText {
    pub artifacts_panel: String,
    pub artifacts_empty: String,
    pub create_focus: String,
    pub create: String,
    pub empower: String,
    pub stabilize: String,
    pub transfer: String,
    pub artifact_meta: String,
    pub instability: String,
    /// Instability meter label when a relic is close to backlash — a call to stabilize.
    pub instability_critical: String,
    pub omens_panel: String,
    pub omens_intro: String,
    pub omen_line: String,
    pub omen_calm: String,
    pub omen_stirring: String,
    pub omen_turbulent: String,
    pub omen_dire: String,
    pub omen_forces: String,
    pub omen_no_forces: String,
    /// A present war, plague, or beast afflicting a region — the most concrete
    /// omen of all, shown in the forces slot in place of the divine-work tally
    /// when a land is under threat (GDD 5.6 <-> 5.2/5.3).
    pub omen_war: String,
    pub omen_plague: String,
    pub omen_beast: String,
    /// Generational-horizon forecast line and its three outlook words.
    pub omen_horizon: String,
    pub omen_deepening: String,
    pub omen_easing: String,
    pub omen_holding: String,
    pub omen_coming_scar: String,
    pub omen_coming_harvest: String,
    pub tool_todo: String,
    pub new_artifact_name: String,
    pub weather_panel: String,
    pub weather_pattern: String,
    pub weather_intensity: String,
    pub shape: String,
    pub weather_empty: String,
    pub weather_meta: String,
    pub weather_magnitude: String,
    pub weather_boon: String,
    pub weather_bane: String,
    pub magic_panel: String,
    pub magic_intro: String,
    pub research: String,
    pub magic_progress: String,
    pub magic_evidence: String,
    pub magic_scholars: String,
    pub magic_no_scholars: String,
    /// Magic-panel line noting Knowledge relics feeding research; `{count}`.
    pub magic_relics: String,
    pub magic_dormant: String,
    pub magic_emerging: String,
    pub magic_known: String,
    pub myths_candidates: String,
    pub myths_active: String,
    pub myths_no_candidates: String,
    pub myths_no_myths: String,
    pub promote: String,
    pub myth_meta: String,
    pub myth_resonance: String,
    pub myth_echo_in: String,
    pub myth_faint: String,
    /// Note on a living myth that has living heroes in its home region to
    /// inspire when it echoes (GDD 5.6 <-> 5.4); slot `{count}`.
    pub myth_inspires: String,
    pub new_myth_title: String,
    /// Title of a myth born from a hero's passage into legend; slot `{hero}`.
    pub legend_myth_title: String,
    /// Title of a myth born when a god crests to wrath (GDD 5.6 pantheon <->
    /// myths); slots `{deity}`, `{region}`.
    pub divine_myth_title: String,
    /// Title of a myth born when a hero slays a beast (GDD 5.2 <-> 5.6); slots
    /// `{hero}`, `{beast}`.
    pub beast_myth_title: String,
    /// Title of a myth born when the holy dead are raised to sainthood (GDD 5.1
    /// <-> 5.6); slot `{saint}`.
    pub saint_myth_title: String,
    /// Title of a myth born when a great festival passes into memory (GDD 5.2 <->
    /// 6); slots `{festival}`, `{region}`.
    pub festival_myth_title: String,
    pub civ_panel: String,
    pub civ_region: String,
    pub civ_intro: String,
    pub advance: String,
    pub agenda_active: String,
    pub agenda_dormant: String,
    pub agenda_score: String,
    pub agenda_spillover: String,
    pub civ_cooldown: String,
    pub civ_ready: String,
    pub pantheon_panel: String,
    pub appease: String,
    pub challenge: String,
    pub deity_meta: String,
    pub deity_pressure: String,
    pub deity_effect: String,
    pub verb_raises: String,
    pub verb_lowers: String,
    pub pantheon_cooldown: String,
    pub mood_dormant: String,
    pub mood_stirring: String,
    pub mood_roused: String,
    pub mood_wrathful: String,
    pub mood_ascendant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiText {
    pub footer_hint: String,
    pub empty_chronicle: String,
    pub no_region: String,
    pub world_summary: String,
    pub world_works: String,
    pub standing_summary: String,
    pub tenor_labels: Vec<String>,
    pub tenor_line: String,
    pub settlement_tiers: Vec<String>,
    pub region_meta: String,
    pub region_subtitle: String,
    pub favor_meter: String,
    pub level_meter: String,
    pub action_cost: String,
    pub year_badge: String,
    pub favor_badge: String,
    pub level_badge: String,
    pub tick_badge: String,
    pub tick_paused: String,
    pub save: String,
    pub load: String,
    pub new_world: String,
    pub holdings: String,
    /// Dashboard portent lines surfacing the reactive pantheon.
    pub heavens_roused: String,
    pub heavens_calm: String,
    pub settlements_line: String,
    pub resources_line: String,
    pub landmarks_line: String,
    pub trade_line: String,
    pub buildings_line: String,
    /// Region-detail line naming the living heroes who dwell in a region and the
    /// callings they lend it (GDD 5.4); slot `{list}`.
    pub heroes_line: String,
    pub holdings_more: String,
    pub no_holdings: String,
    /// Town browser (drill-in from a region's holdings): title, close, the
    /// selected town's stat lines, its works list, and the clickable hint on the
    /// region-detail towns line.
    pub town_browser_title: String,
    pub town_close: String,
    pub town_population: String,
    pub town_prosperity: String,
    pub town_tier: String,
    pub town_buildings: String,
    pub town_no_buildings: String,
    pub towns_hint: String,
    /// Generic list pager, reusable across screens: "Page {page} / {pages}".
    pub page_label: String,
    pub page_prev: String,
    pub page_next: String,
    /// Region-detail warning of scheduled backlash/weather aftermaths; `{count}`.
    pub aftermath_looms: String,
    pub boon_ripens: String,
    /// Region-detail line naming the weather front now over the region (GDD 5.6);
    /// slots `{intensity}`, `{pattern}`. Coloured fair/foul by the front's tenor.
    pub weather_over: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notifications {
    pub awaken: String,
    /// Shown when an achievement is unlocked; slot `{name}`.
    pub achievement_unlocked: String,
    /// Shown when an era transition ushers in a new age; slot `{era}`.
    pub era_dawns: String,
    pub not_enough_favor: String,
    pub action_success: String,
    pub advance_tick: String,
    pub world_saved: String,
    pub world_autosaved: String,
    pub save_failed: String,
    pub world_restored: String,
    pub load_failed: String,
    pub new_world: String,
    pub unknown_action: String,
    pub champion_designated: String,
    pub champion_designate_failed: String,
    pub champion_cultivated: String,
    pub champion_focus_changed: String,
    pub bet_placed: String,
    pub bet_unaffordable: String,
    pub bet_closed: String,
    pub artifact_created: String,
    pub artifact_max: String,
    pub artifact_empowered: String,
    pub artifact_stabilized: String,
    pub artifact_transferred: String,
    pub weather_shaped: String,
    pub weather_max: String,
    pub magic_researched: String,
    pub myth_promoted: String,
    pub myth_cap: String,
    pub agenda_advanced: String,
    pub agenda_cooldown: String,
    pub deity_appeased: String,
    pub deity_challenged: String,
    pub deity_cooldown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleText {
    pub world_awakens: String,
    pub crisis: String,
    pub divine_action: String,
    pub hero_level_up: String,
    pub hero_death: String,
    /// A hero crossing into the top renown title.
    pub hero_legend: String,
    /// The death of a hero who had already passed into legend.
    pub hero_legend_death: String,
    /// A legend founds a noble house (GDD 5.4); slots `{house}`, `{region}`.
    pub house_founded: String,
    /// An heir of an established house is born at an age's turn; slots `{house}`.
    pub house_heir: String,
    /// A house whose blood has run out is forgotten; slot `{house}`.
    pub house_fades: String,
    /// A house whose seat was lost reestablishes where its blood dwells (GDD 5.4
    /// <-> 5.2); slots `{house}`, `{region}`.
    pub house_reseated: String,
    /// Name pattern for a founded house; slot `{founder}`.
    pub house_name: String,
    /// A great Order is founded as its calling reaches critical mass, or
    /// dissolved as its ranks thin (GDD 5.4); slot `{order}`.
    pub order_founded: String,
    pub order_dissolved: String,
    /// A prophecy of doom or plenty is spoken from the world's drift, then comes
    /// to pass as that drift holds or passes unfulfilled as it turns (GDD 5.6);
    /// slot `{prophecy}`.
    pub prophecy_foretold: String,
    pub prophecy_fulfilled: String,
    pub prophecy_averted: String,
    /// A great dead soul is raised to sainthood by the faithful of its home land,
    /// or at last fades from living memory (GDD 5.1 <-> 5.4); slots `{saint}`,
    /// `{region}`. `saint_name` is the pattern the venerated name takes; slot
    /// `{hero}`.
    pub saint_canonized: String,
    pub saint_forgotten: String,
    pub saint_name: String,
    /// A flourishing realm throws open its gates for a great festival, which in
    /// time passes into memory (GDD 5.2 <-> 6); slots `{festival}`, `{region}`.
    pub festival_begins: String,
    pub festival_ends: String,
    /// A pantheon deity cresting into the height of its wrath.
    pub deity_ascendant: String,
    pub champion_resolved: String,
    pub champion_escalated: String,
    pub champion_retired: String,
    pub bet_won: String,
    pub bet_lost: String,
    pub artifact_backlash: String,
    /// Delayed backlash aftermath: a blighted settlement, heroes shaken by the
    /// arcane shockwave, then regional unrest.
    pub aftermath_blight: String,
    /// Heroes stripped of renown by a shattering's shockwave (GDD 5.6 <-> 5.4);
    /// slots `{source}`, `{region}`.
    pub aftermath_heroes_shaken: String,
    pub aftermath_unrest: String,
    /// The delayed bounty that follows fair weather.
    pub aftermath_bloom: String,
    pub magic_known: String,
    pub myth_echo: String,
    pub myth_faded: String,
    pub era_transition: String,
    /// The turning of an age sweeps away the plagues and beasts of the old world
    /// (GDD 5.7 <-> 5.3/5.2). Pushed only when there were afflictions to sweep.
    pub age_sweeps_afflictions: String,
    pub culture_shift: String,
    pub agenda_shift: String,
    pub settlement_built: String,
    pub settlement_abandoned: String,
    pub settlement_founded: String,
    pub settlement_ascends: String,
    pub settlement_declines: String,
    pub landmark_raised: String,
    pub landmark_razed: String,
    /// The greatest wonder of a conquered region, thrown down in the sack (GDD
    /// 5.2 <-> 5.7); slots `{landmark}`, `{region}`.
    pub landmark_sacked: String,
    /// A resource node crossing into one of its dramatic states (GDD 5.3):
    /// flourishing to its peak, falling to corruption, or run dry.
    pub resource_flourishes: String,
    pub resource_corrupts: String,
    pub resource_depletes: String,
    /// Prospectors open a newly discovered resource node (GDD 5.3); slots
    /// `{node}`, `{region}`, `{type}`.
    pub resource_discovered: String,
    /// Name pattern for a discovered node; slots `{region}`, `{type}`.
    pub resource_node_name: String,
    /// A new trade route is forged between two prospering regions (GDD 5.2);
    /// slots `{route}`, `{region_a}`, `{region_b}`.
    pub trade_route_forged: String,
    /// Name pattern for a forged route; slots `{region_a}`, `{region_b}`.
    pub trade_route_name: String,
    /// A plague's course (GDD 5.3): it breaks out, spreads along the roads to a
    /// new region, and finally burns out. Slots `{plague}`, `{region}`.
    pub plague_outbreak: String,
    pub plague_spread: String,
    pub plague_fades: String,
    /// Name pattern for an outbreak; slots `{pestilence}`, `{region}`.
    pub plague_name: String,
    /// A beast's course (GDD 5.2): it emerges from the wild to prey on the land,
    /// then is slain by a named hunter, or driven off unsung. Slots `{monster}`,
    /// `{hero}`.
    pub monster_emergence: String,
    pub monster_slain: String,
    pub monster_driven_off: String,
    /// Name pattern for an emerging beast; slots `{beast}`, `{region}`.
    pub monster_name: String,
    /// A beast left unopposed so long it swells into a named legendary terror
    /// (GDD 5.2): the portent of its ascension, the epithet-bearing name it takes,
    /// and the greater renown of the hero who at last brings it down. Slots
    /// `{monster}`, `{epithet}`, `{hero}`.
    pub monster_ascends: String,
    pub monster_ascends_name: String,
    pub monster_apex_slain: String,
    /// Legendary epithets a beast may take on ascending to a great terror, one
    /// chosen deterministically per beast (GDD 5.2).
    pub monster_epithets: Vec<String>,
    /// A war's course (GDD 5.2): it is declared, then ends in a victor or an
    /// exhausted stalemate. Slots `{aggressor}`, `{defender}`, `{victor}`,
    /// `{loser}`.
    pub war_declared: String,
    pub war_won: String,
    pub war_stalemate: String,
    /// An alliance forms between two regions, or dissolves as they drift apart
    /// (GDD 5.2); slots `{region_a}`, `{region_b}`.
    pub pact_formed: String,
    pub pact_dissolved: String,
    /// A stronger region subordinates a weaker one as its vassal, or the vassal
    /// throws off the yoke (GDD 5.2); slots `{overlord}`, `{vassal}`.
    pub vassalage_sworn: String,
    pub vassalage_broken: String,
    /// A notable flight of refugees from a perilous settlement to a safe haven
    /// (GDD 5.3); slots `{source}`, `{haven}`.
    pub refugee_flight: String,
    /// A region's granaries run dry and famine takes hold, or the dearth finally
    /// breaks as the harvest returns (GDD 5.3); slots `{region}`.
    pub famine_begins: String,
    pub famine_breaks: String,
    pub region_fracture: String,
    pub region_conquest: String,
    pub region_sack: String,
    pub region_founded: String,
    pub weather_natural: String,
    /// A weather front drives a resource node to ruin or into full flourish
    /// (GDD 5.6 <-> 5.3); slots `{pattern}`, `{node}`, `{region}`.
    pub weather_withered: String,
    pub weather_quickened: String,
}

/// Fill `{name}` placeholders in a template with the given key/value pairs.
/// Unreferenced placeholders are left as-is; extra args are ignored.
pub fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (key, value) in args {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_named_placeholders() {
        let result = fill(
            "{action} on {region} ({cost} favor).",
            &[
                ("action", "Bless".to_owned()),
                ("region", "Aldermoor".to_owned()),
                ("cost", "15".to_owned()),
            ],
        );
        assert_eq!(result, "Bless on Aldermoor (15 favor).");
    }
}
