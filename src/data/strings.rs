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
    /// Genesis-outlook lines on the region detail (one shown at a time).
    pub outlook_frontier: String,
    pub outlook_vulnerable: String,
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
    /// Earned renown titles, ascending (index-aligned with hero.renown.thresholds).
    pub renown_titles: Vec<String>,
    /// Roster meta line for a titled hero.
    pub titled_meta: String,
    pub untitled_meta: String,
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
    pub magic_panel: String,
    pub magic_intro: String,
    pub research: String,
    pub magic_progress: String,
    pub magic_evidence: String,
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
    pub new_myth_title: String,
    /// Title of a myth born from a hero's passage into legend; slot `{hero}`.
    pub legend_myth_title: String,
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
    pub standing_summary: String,
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
    pub no_holdings: String,
    /// Generic list pager, reusable across screens: "Page {page} / {pages}".
    pub page_label: String,
    pub page_prev: String,
    pub page_next: String,
    /// Region-detail warning of scheduled backlash/weather aftermaths; `{count}`.
    pub aftermath_looms: String,
    pub boon_ripens: String,
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
    pub year_dawns: String,
    pub crisis: String,
    pub divine_action: String,
    pub hero_level_up: String,
    pub hero_death: String,
    /// A hero crossing into the top renown title.
    pub hero_legend: String,
    /// The death of a hero who had already passed into legend.
    pub hero_legend_death: String,
    /// A pantheon deity cresting into the height of its wrath.
    pub deity_ascendant: String,
    pub champion_resolved: String,
    pub champion_escalated: String,
    pub bet_won: String,
    pub bet_lost: String,
    pub artifact_backlash: String,
    /// Delayed backlash aftermath: a blighted settlement, then regional unrest.
    pub aftermath_blight: String,
    pub aftermath_unrest: String,
    /// The delayed bounty that follows fair weather.
    pub aftermath_bloom: String,
    pub magic_known: String,
    pub myth_echo: String,
    pub myth_faded: String,
    pub era_transition: String,
    pub culture_shift: String,
    pub settlement_built: String,
    pub region_fracture: String,
    pub region_conquest: String,
    pub region_founded: String,
    pub weather_natural: String,
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
