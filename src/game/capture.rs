//! Screenshot-harness scene seeding for `Game`. Maps a scene name to a screen
//! (and some demo world history) so each UI can be captured headlessly. Split
//! from `game.rs` to keep the core loop focused; this is another `impl Game`
//! block reaching the same private fields, and it only runs under the capture
//! harness — never during normal play.

use super::Game;
use crate::ui::Screen;

impl Game {
    /// Seed a named screen (and some world history) for the screenshot harness.
    pub fn begin_capture_scene(&mut self, scene: &str) {
        self.screen = match scene {
            "title" | "menu" => Screen::Title,
            "chronicle" | "event_log" => Screen::Chronicle,
            "regions" | "town" => Screen::Regions,
            "heroes" => Screen::Heroes,
            "divine_tools" | "artifacts" | "omens" | "weather" | "magic" | "myths"
            | "civilization" | "pantheon" => Screen::DivineTools,
            "betting" => Screen::Betting,
            "eras" => Screen::Eras,
            "settings" => Screen::Settings,
            _ => Screen::Dashboard,
        };
        self.divine_tab = match scene {
            "weather" => 1,
            "omens" => 2,
            "magic" => 3,
            "myths" => 4,
            "civilization" => 5,
            "pantheon" => 6,
            _ => 0,
        };
        if scene == "town" {
            // Grow towns into cities with works raised, then drill into the first.
            for _ in 0..80 {
                self.run_tick();
            }
            self.selected_region = 0;
            let region_id = self.world.regions[0].id.clone();
            if let Some(first) = self
                .world
                .settlements
                .iter()
                .find(|s| s.region_id == region_id)
            {
                self.selected_town = Some(first.id.clone());
            }
        }
        if scene == "weather" {
            self.weather_intensity = 2;
            self.shape_weather();
            self.selected_region = 1;
            self.weather_pattern = 2;
            self.shape_weather();
            self.selected_region = 0;
            self.weather_pattern = 0;
            self.weather_intensity = 0;
        }
        if scene == "magic" {
            for _ in 0..4 {
                self.research_magic("restoration");
            }
            for _ in 0..45 {
                self.run_tick();
            }
            // A standing Knowledge relic so the research-momentum line noting
            // relics of knowledge is visible in the capture (GDD 5.6).
            self.world.artifacts.push(crate::world::Artifact {
                id: "capture-knowledge-relic".to_owned(),
                name: "Codex of the Deep".to_owned(),
                focus: crate::data::ArtifactFocus::Knowledge,
                power: 5,
                instability: 0.0,
                region_id: self.world.regions[0].id.clone(),
            });
        }
        if scene == "myths" {
            for _ in 0..2 {
                self.run_tick();
            }
            let ids: Vec<String> = self
                .world
                .myth_candidates
                .iter()
                .take(2)
                .map(|c| c.id.clone())
                .collect();
            for id in ids {
                self.promote_myth(&id);
            }
            for _ in 0..6 {
                self.run_tick();
            }
        }
        if scene == "civilization" {
            self.advance_agenda(0);
            for _ in 0..3 {
                self.run_tick();
            }
        }
        if scene == "pantheon" {
            self.challenge_deity("aurex");
            self.appease_deity("mordath");
            for _ in 0..2 {
                self.run_tick();
            }
        }
        if scene == "regions" {
            // Let the selected region grow thick with towns and wonders, so its
            // detail panel shows a matured holdings list (GDD 5.2/5.3).
            for _ in 0..90 {
                self.run_tick();
            }
            // Shape a front over the selected region so the active-skies line
            // (the weather now reshaping this land) is visible in the capture.
            self.selected_region = 0;
            self.weather_pattern = 0;
            self.weather_intensity = 2;
            self.shape_weather();
            self.weather_intensity = 0;
        }
        if scene == "eras" {
            // Run through a few ages so the chronicle of eras fills — long enough
            // that wonders arise and some are thrown down at a transition.
            for _ in 0..240 {
                self.run_tick();
            }
        }
        if scene == "longrun" {
            // A long unmanaged run to inspect the world's settled state.
            for _ in 0..150 {
                self.run_tick();
            }
        }
        if scene == "omens" {
            // Run long enough that region genesis grows the map past one page, so
            // the forecast's pagination shows, and seed a fresh divine work or two.
            self.create_artifact();
            for _ in 0..120 {
                self.run_tick();
            }
            // A scheduled consequence so the horizon's coming-scar forecast shows.
            self.world
                .pending_consequences
                .push(crate::world::DelayedConsequence {
                    region_id: self.world.regions[0].id.clone(),
                    source: "The Sunken Storm".to_owned(),
                    delay: 42,
                    effect: crate::world::ConsequenceEffect::SettlementBlight(6.0),
                });
            // A present plague and a stalking beast so the forecast surfaces its
            // afflictions line (GDD 5.6 <-> 5.3/5.2).
            self.world.plagues.push(crate::world::Plague {
                id: "capture-plague".to_owned(),
                name: "The Grey Fever of the North".to_owned(),
                region_id: self.world.regions[0].id.clone(),
                severity: 1.5,
                age: 3,
            });
            if self.world.regions.len() > 1 {
                self.world.monsters.push(crate::world::Monster {
                    id: "capture-monster".to_owned(),
                    name: "The Shadow Wyrm".to_owned(),
                    type_id: "shadow_wyrm".to_owned(),
                    region_id: self.world.regions[1].id.clone(),
                    ferocity: 2.5,
                    age: 5,
                    apex: false,
                });
            }
            // A war between two further regions so the forecast surfaces it too.
            if self.world.regions.len() > 3 {
                self.world.wars.push(crate::world::War {
                    id: "capture-war".to_owned(),
                    aggressor_id: self.world.regions[2].id.clone(),
                    defender_id: self.world.regions[3].id.clone(),
                    intensity: 1.0,
                    age: 4,
                });
            }
        }
        if scene == "settings" {
            // Demonstrate the paused state (Resume control + "Paused" header).
            self.paused = true;
        }
        match self.screen {
            // Demo a couple of champions so the heroes screen shows the roster.
            Screen::Heroes => {
                let ids: Vec<String> = self
                    .world
                    .heroes
                    .iter()
                    .filter(|h| h.is_alive)
                    .take(2)
                    .map(|h| h.id.clone())
                    .collect();
                for id in &ids {
                    self.designate_champion(id);
                }
                // Align the first champion's focus to its hero's calling so the
                // "in tune" synergy cue is visible in the capture (GDD 5.4).
                let roles: std::collections::HashMap<String, crate::data::HeroRole> = self
                    .world
                    .heroes
                    .iter()
                    .map(|h| (h.id.clone(), h.role))
                    .collect();
                for champ in self.player.champions.iter_mut() {
                    if let Some(&role) = roles.get(&champ.hero_id) {
                        while !champ.focus.suits(role) {
                            champ.focus = champ.focus.next();
                        }
                    }
                }
                for _ in 0..8 {
                    self.run_tick();
                }
            }
            // Demo a couple of wagers so the Observatory shows events and bets.
            Screen::Betting => {
                for _ in 0..3 {
                    self.run_tick();
                }
                let ids: Vec<String> = self
                    .world
                    .speculations
                    .iter()
                    .filter(|e| e.is_active())
                    .take(2)
                    .map(|e| e.id.clone())
                    .collect();
                for id in ids {
                    self.place_bet(&id);
                }
                for _ in 0..12 {
                    self.run_tick();
                }
            }
            _ => {
                for _ in 0..5 {
                    self.run_tick();
                }
            }
        }
    }
}
