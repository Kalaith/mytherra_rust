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
    pub placeholders: Placeholders,
    pub ui: UiText,
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
pub struct PlaceholderText {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placeholders {
    pub heroes: PlaceholderText,
    pub divine_tools: PlaceholderText,
    pub betting: PlaceholderText,
    pub eras: PlaceholderText,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleText {
    pub world_awakens: String,
    pub year_dawns: String,
    pub crisis: String,
    pub divine_action: String,
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
