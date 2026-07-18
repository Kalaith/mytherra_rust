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
    pub notifications: Notifications,
    pub chronicle: ChronicleText,
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
    pub record_line: String,
    pub record_span: String,
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
    pub omens_panel: String,
    pub omens_intro: String,
    pub omen_line: String,
    pub omen_calm: String,
    pub omen_stirring: String,
    pub omen_turbulent: String,
    pub omen_dire: String,
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
    pub civ_panel: String,
    pub civ_region: String,
    pub civ_intro: String,
    pub advance: String,
    pub agenda_active: String,
    pub agenda_dormant: String,
    pub agenda_score: String,
    pub civ_cooldown: String,
    pub civ_ready: String,
    pub pantheon_panel: String,
    pub appease: String,
    pub challenge: String,
    pub deity_meta: String,
    pub deity_pressure: String,
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
    pub save: String,
    pub load: String,
    pub new_world: String,
    pub holdings: String,
    pub settlements_line: String,
    pub resources_line: String,
    pub landmarks_line: String,
    pub trade_line: String,
    pub no_holdings: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notifications {
    pub awaken: String,
    pub not_enough_favor: String,
    pub action_success: String,
    pub advance_tick: String,
    pub world_saved: String,
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
    pub champion_resolved: String,
    pub champion_escalated: String,
    pub bet_won: String,
    pub bet_lost: String,
    pub artifact_backlash: String,
    pub magic_known: String,
    pub myth_echo: String,
    pub era_transition: String,
    pub culture_shift: String,
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
