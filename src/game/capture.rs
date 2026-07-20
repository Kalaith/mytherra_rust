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
            "chronicle" | "event_log" => Screen::Chronicle,
            "regions" => Screen::Regions,
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
        }
        if scene == "eras" {
            // Run past a century so at least one era transition is recorded.
            for _ in 0..110 {
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
            // Seed a few divine works so the forces read-out is meaningful.
            self.create_artifact();
            self.selected_region = 2;
            self.weather_intensity = 2;
            self.shape_weather();
            self.selected_region = 0;
            self.weather_intensity = 0;
            for _ in 0..2 {
                self.run_tick();
            }
            let ids: Vec<String> = self
                .world
                .myth_candidates
                .iter()
                .take(1)
                .map(|c| c.id.clone())
                .collect();
            for id in ids {
                self.promote_myth(&id);
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
                for id in ids {
                    self.designate_champion(&id);
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
