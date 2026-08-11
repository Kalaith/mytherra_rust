//! The world chronicle: an append-only log of notable events, shown on the
//! Dashboard and (later) the dedicated Event Log screen (GDD 10).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

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

    /// Weave the most recent tick's events together by kind, so a busy year reads
    /// as a mixture rather than blocks of one kind — all saints, then all refugee
    /// flights, then all deaths — which is only an artifact of the fixed order the
    /// tick's subsystems run in, not the order things "happened". Within each kind
    /// the order is preserved (a death still precedes the sainthood it enables);
    /// only the kinds are round-robined together. Deterministic (no RNG), so a
    /// reload and the determinism tests are unaffected.
    ///
    /// Every event of one tick shares that tick's `year` (the year advances once
    /// per tick), so the trailing run of same-year events is exactly this tick's —
    /// reordering it never disturbs any earlier year. Call once at the end of a
    /// tick, after every subsystem has recorded its events.
    pub fn interleave_latest_tick(&mut self) {
        let Some(latest) = self.events.last().map(|e| e.year) else {
            return;
        };
        let start = self
            .events
            .iter()
            .rposition(|e| e.year != latest)
            .map_or(0, |i| i + 1);
        if self.events.len() - start < 2 {
            return;
        }

        // Bucket the tick's events by kind, preserving within-kind order.
        let mut buckets: Vec<VecDeque<WorldEvent>> =
            EventKind::ALL.iter().map(|_| VecDeque::new()).collect();
        for event in self.events.split_off(start) {
            let kind = EventKind::ALL.iter().position(|k| *k == event.kind);
            buckets[kind.expect("every EventKind is in ALL")].push_back(event);
        }

        // Draw round-robin across kinds until every bucket is drained.
        let mut remaining: usize = buckets.iter().map(VecDeque::len).sum();
        while remaining > 0 {
            for bucket in &mut buckets {
                if let Some(event) = bucket.pop_front() {
                    self.events.push(event);
                    remaining -= 1;
                }
            }
        }
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
mod tests;
