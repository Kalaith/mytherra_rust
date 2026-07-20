//! The world chronicle: an append-only log of notable events, shown on the
//! Dashboard and (later) the dedicated Event Log screen (GDD 10).

use serde::{Deserialize, Serialize};

/// Category of a chronicle entry, used for color-coding and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A divine nudge by the player (visible manipulation, GDD Pillar 4).
    Divine,
    /// An emergent region change (status shift, crisis).
    Region,
    /// A hero lifecycle event (level-up, death).
    Hero,
    /// System / bookkeeping messages.
    System,
}

impl EventKind {
    pub const ALL: [EventKind; 4] = [
        EventKind::Divine,
        EventKind::Region,
        EventKind::Hero,
        EventKind::System,
    ];

    /// Canonical display name (the Event Log filter chips, GDD 10). Type
    /// formatting stays in code; authored copy lives in `strings.json`.
    pub fn label(self) -> &'static str {
        match self {
            EventKind::Divine => "Divine",
            EventKind::Region => "Regions",
            EventKind::Hero => "Heroes",
            EventKind::System => "System",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    pub year: u32,
    pub kind: EventKind,
    pub message: String,
}

/// A bounded, append-only event history. Oldest entries are dropped past `cap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chronicle {
    events: Vec<WorldEvent>,
    cap: usize,
}

impl Default for Chronicle {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            cap: 200,
        }
    }
}

impl Chronicle {
    pub fn push(&mut self, year: u32, kind: EventKind, message: impl Into<String>) {
        self.events.push(WorldEvent {
            year,
            kind,
            message: message.into(),
        });
        if self.events.len() > self.cap {
            let overflow = self.events.len() - self.cap;
            self.events.drain(0..overflow);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The most recent `count` events, newest first.
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &WorldEvent> {
        self.events.iter().rev().take(count)
    }

    /// Every retained event, newest first — the Event Log screen (GDD 10)
    /// filters this by kind.
    pub fn iter_newest(&self) -> impl Iterator<Item = &WorldEvent> {
        self.events.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_returns_newest_first() {
        let mut chronicle = Chronicle::default();
        chronicle.push(1, EventKind::System, "first");
        chronicle.push(2, EventKind::System, "second");
        let recent: Vec<&str> = chronicle.recent(2).map(|e| e.message.as_str()).collect();
        assert_eq!(recent, vec!["second", "first"]);
    }

    #[test]
    fn cap_drops_oldest() {
        let mut chronicle = Chronicle {
            events: Vec::new(),
            cap: 3,
        };
        for i in 0..5 {
            chronicle.push(i, EventKind::System, format!("e{i}"));
        }
        assert_eq!(chronicle.recent(10).count(), 3);
        let newest = chronicle.recent(1).next().unwrap();
        assert_eq!(newest.message, "e4");
    }
}
