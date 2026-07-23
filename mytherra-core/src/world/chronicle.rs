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
    /// Monotonic count of every event ever pushed — it survives the cap drops
    /// (unlike `events.len()`), so it serves as a stable since-cursor for a
    /// returning player's event delta (GDD 7.4).
    #[serde(default)]
    total_pushed: u64,
}

impl Default for Chronicle {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            cap: 200,
            total_pushed: 0,
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
        self.total_pushed += 1;
        if self.events.len() > self.cap {
            let overflow = self.events.len() - self.cap;
            self.events.drain(0..overflow);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The current since-cursor: pass it back to [`since`](Self::since) to get
    /// only the events pushed after this moment.
    pub fn cursor(&self) -> u64 {
        self.total_pushed
    }

    /// The events newer than `cursor` (chronological, oldest first) paired with
    /// the new cursor to pass next time (GDD 7.4). If `cursor` predates the
    /// retained window, only the still-retained events are returned — older ones
    /// were dropped past the cap.
    pub fn since(&self, cursor: u64) -> (Vec<&WorldEvent>, u64) {
        let oldest_seq = self.total_pushed.saturating_sub(self.events.len() as u64);
        let start = (cursor.saturating_sub(oldest_seq) as usize).min(self.events.len());
        (self.events[start..].iter().collect(), self.total_pushed)
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
            total_pushed: 0,
        };
        for i in 0..5 {
            chronicle.push(i, EventKind::System, format!("e{i}"));
        }
        assert_eq!(chronicle.recent(10).count(), 3);
        let newest = chronicle.recent(1).next().unwrap();
        assert_eq!(newest.message, "e4");
    }

    #[test]
    fn since_returns_only_the_new_events_and_survives_the_cap() {
        let mut chronicle = Chronicle::default();
        chronicle.push(1, EventKind::System, "a");
        chronicle.push(2, EventKind::System, "b");
        let (events, cursor) = chronicle.since(0);
        assert_eq!(
            events
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(cursor, 2);

        // Nothing new since the cursor.
        let (events, cursor2) = chronicle.since(cursor);
        assert!(events.is_empty());
        assert_eq!(cursor2, 2);

        // A fresh push shows up as the only delta.
        chronicle.push(3, EventKind::Hero, "c");
        let (events, cursor3) = chronicle.since(cursor);
        assert_eq!(
            events
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>(),
            vec!["c"]
        );
        assert_eq!(cursor3, 3);

        // A cursor older than the retained window still yields the retained tail,
        // never a panic — even after the cap has dropped the oldest events.
        let mut small = Chronicle {
            events: Vec::new(),
            cap: 2,
            total_pushed: 0,
        };
        for i in 0..5 {
            small.push(i, EventKind::System, format!("e{i}"));
        }
        let (events, cursor) = small.since(0);
        assert_eq!(events.len(), 2, "only the retained tail survives");
        assert_eq!(cursor, 5, "the cursor still counts every event ever pushed");
    }
}
