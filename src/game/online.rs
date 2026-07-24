//! Online play: the client as a thin herald of the authority server (GDD 7.1).
//!
//! The interactive client owns no simulation. It connects to a running
//! `mytherra-server`, polls `GET /view` for its Standing-filtered projection,
//! renders that, and submits every authoritative verb through `POST /action` —
//! the server owns the world, the tick, and the authorization (§7.1, §7.7). This
//! is the only way to play; there is no local-world fallback. (The screenshot
//! capture harness in `capture.rs` is the one exception — it drives a throwaway
//! local world purely to render each screen headlessly, and never touches the
//! network.)
//!
//! Every request is a non-blocking [`net::Pending`] polled once per frame, so a
//! slow or unreachable server never stalls the loop.

use super::Game;
use crate::net::{self, Pending};
use crate::ui::Screen;
use mytherra_core::command::ActionReport;
use mytherra_protocol::{ClientView, EventsDelta, SessionResponse};

/// The most recent stirrings of the world to surface as notifications on any one
/// poll, so a burst of events after a server tick can't flood the screen.
const MAX_STIRRINGS_PER_POLL: usize = 4;

/// How long a single request may be in flight before it's treated as failed. On
/// wasm a refused connection never resolves (quad-net resolves only a 200), so
/// without this the client would hang silently instead of noticing the server is
/// gone. Generous enough not to trip on a merely slow response.
const REQUEST_TIMEOUT: f32 = 6.0;

/// Seconds to wait between reconnection attempts, so a downed server isn't
/// hammered (native transport errors resolve immediately and would otherwise
/// retry every frame).
const RETRY_COOLDOWN: f32 = 2.0;

/// The client's live link to the authority server, surfaced in the header badge.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnlineStatus {
    /// Handshaking, or awaiting the first view — not yet connected.
    Connecting,
    /// The last poll succeeded; the world is live.
    Live,
    /// A poll failed or timed out (the server went away); retrying.
    Reconnecting,
}

/// The client's live connection to one authority server: the in-flight requests
/// and the poll cadence. State only — the driving logic lives on [`Game`].
pub(super) struct OnlineSession {
    client: net::ServerClient,
    /// The `POST /session` handshake in flight, until the server returns a guest
    /// id (GDD 7.7). View/events polling waits on it.
    session_req: Option<Pending<SessionResponse>>,
    /// Set once the guest session id is in hand — the client is then identified
    /// and may fetch its view, poll events, and submit actions.
    identified: bool,
    /// A `GET /view` in flight, if any. At most one is outstanding at a time.
    view_req: Option<Pending<ClientView>>,
    /// A `GET /events?since=` in flight, if any (GDD 7.4).
    events_req: Option<Pending<EventsDelta>>,
    /// `POST /action`s awaiting their reports — several may overlap.
    action_reqs: Vec<Pending<ActionReport>>,
    /// Real seconds since the last poll cycle was started.
    poll_accum: f32,
    /// The chronicle since-cursor: the sequence up to which we've already seen
    /// events, passed to the next `/events` fetch (GDD 7.4).
    cursor: u64,
    /// Set once the first `/events` reply has established the baseline cursor, so
    /// the whole retained backlog isn't surfaced as "just happened" on connect —
    /// only genuinely new stirrings toast thereafter.
    events_synced: bool,
    /// Set once the first `/view` has arrived, so the UI can tell "connecting"
    /// from "connected to an empty world".
    connected: bool,
    /// The live link state shown in the header (§7.4). Drives the badge and the
    /// lost/restored notifications.
    status: OnlineStatus,
    /// Seconds left before the next reconnection attempt (0 = attempt now).
    retry_cooldown: f32,
}

impl OnlineSession {
    fn new(client: net::ServerClient) -> Self {
        Self {
            client,
            session_req: None,
            identified: false,
            view_req: None,
            events_req: None,
            action_reqs: Vec::new(),
            poll_accum: 0.0,
            cursor: 0,
            events_synced: false,
            connected: false,
            status: OnlineStatus::Connecting,
            retry_cooldown: 0.0,
        }
    }

    /// Submit an authoritative command to the server (`POST /action`). The
    /// server checks the player's Standing and applies it; the report arrives on
    /// a later poll. A command beyond the player's Standing comes back an error.
    pub(super) fn submit(&mut self, command: &mytherra_protocol::PlayerAction) {
        self.action_reqs.push(self.client.submit_action(command));
    }
}

impl Game {
    /// Whether the client is currently connected to (or connecting to) a server.
    pub(super) fn is_online(&self) -> bool {
        self.online.is_some()
    }

    /// Open a connection to the authority server and enter the world (§7.1). The
    /// guest-session handshake is kicked immediately; once its id arrives the
    /// first `/view` follows and the screens populate.
    pub(super) fn go_online(&mut self) {
        let mut session =
            OnlineSession::new(net::ServerClient::new(self.data.config.server_url.clone()));
        session.session_req = Some(session.client.create_session());
        self.online = Some(session);
        self.screen = Screen::Dashboard;
        self.notifications
            .info(self.data.strings.notifications.connecting.clone());
    }

    /// Drop the connection and return to the title menu.
    pub(super) fn go_offline(&mut self) {
        self.online = None;
        self.screen = Screen::Title;
    }

    /// Drive the guest-session handshake (§7.7), retrying until it lands. Returns
    /// `true` once the session id is in hand — so normal view/events/action
    /// polling can run — and `false` while still handshaking. Unlike a one-shot
    /// attempt, a failed or timed-out `/session` is re-issued after a cooldown, so
    /// a client that opened before the server was up still connects once it is.
    fn drive_handshake(&mut self, dt: f32) -> bool {
        if self.online.as_ref().is_some_and(|s| s.identified) {
            return true;
        }
        let mut result: Option<Result<SessionResponse, String>> = None;
        {
            let Some(session) = self.online.as_mut() else {
                return false;
            };
            match session.session_req.as_mut() {
                // A handshake is in flight — poll it (with the wasm timeout).
                Some(req) => {
                    if let Some(r) = req.poll_timed(dt, REQUEST_TIMEOUT) {
                        result = Some(r);
                        session.session_req = None;
                    }
                }
                // None in flight: wait out the cooldown, then open a new one.
                None => {
                    if session.retry_cooldown > 0.0 {
                        session.retry_cooldown -= dt;
                    } else {
                        session.session_req = Some(session.client.create_session());
                    }
                }
            }
        }
        match result {
            Some(Ok(session_resp)) => {
                if let Some(s) = self.online.as_mut() {
                    s.client.set_player_id(session_resp.player_id);
                    s.identified = true;
                    s.view_req = Some(s.client.fetch_view());
                    s.events_req = Some(s.client.fetch_events(s.cursor));
                }
                true
            }
            // A failed handshake schedules a silent retry — the badge already
            // reads "connecting", so no per-attempt notification.
            Some(Err(_)) => {
                if let Some(s) = self.online.as_mut() {
                    s.retry_cooldown = RETRY_COOLDOWN;
                }
                false
            }
            None => false,
        }
    }

    /// The link state the header badge reflects (Connecting / Live / Reconnecting),
    /// or `None` when not online at all.
    pub(super) fn online_status(&self) -> Option<OnlineStatus> {
        self.online.as_ref().map(|s| s.status)
    }

    /// A successful poll: the link is live again. Announce the return only if we
    /// had actually dropped to reconnecting.
    fn mark_link_live(&mut self) {
        let recovered = {
            let Some(s) = self.online.as_mut() else {
                return;
            };
            let recovered = s.status == OnlineStatus::Reconnecting;
            s.status = OnlineStatus::Live;
            recovered
        };
        if recovered {
            self.notifications
                .info(self.data.strings.notifications.reconnected.clone());
        }
    }

    /// A poll failed or timed out: the server is unreachable. Drop to reconnecting
    /// (announcing it once, on the fall from a live link) and keep retrying — the
    /// session id survives a server restart (the DB is the save), so the same
    /// player resumes when the server returns.
    fn mark_link_lost(&mut self) {
        let fell = {
            let Some(s) = self.online.as_mut() else {
                return;
            };
            let fell = s.status == OnlineStatus::Live;
            s.status = OnlineStatus::Reconnecting;
            fell
        };
        if fell {
            self.notifications
                .warning(self.data.strings.notifications.connection_lost.clone());
        }
    }

    /// Drive the online session for a frame: keep the `/view` poll cadence, adopt
    /// any freshly-arrived projection, and surface completed action reports.
    /// Splits neatly in two — first everything that borrows the session, then the
    /// application of the owned results to `self`.
    pub(super) fn update_online(&mut self, dt: f32) {
        // Phase 0: complete the guest-session handshake before anything else.
        // Until the id arrives, every other request would be turned away (§7.7).
        if !self.drive_handshake(dt) {
            return;
        }

        let mut fetched: Option<Result<ClientView, String>> = None;
        let mut delta: Option<Result<EventsDelta, String>> = None;
        let mut reports: Vec<Result<ActionReport, String>> = Vec::new();
        {
            let Some(session) = self.online.as_mut() else {
                return;
            };
            session.poll_accum += dt;
            // Start a poll cycle when nothing is in flight and either we've never
            // connected or the cadence has elapsed. After a timeout the failed
            // request is cleared below, so this re-issues and drives the retry.
            let due =
                !session.connected || session.poll_accum >= self.data.config.view_poll_seconds;
            if due && session.view_req.is_none() && session.events_req.is_none() {
                session.poll_accum = 0.0;
                session.view_req = Some(session.client.fetch_view());
                session.events_req = Some(session.client.fetch_events(session.cursor));
            }
            if let Some(req) = session.view_req.as_mut() {
                if let Some(result) = req.poll_timed(dt, REQUEST_TIMEOUT) {
                    fetched = Some(result);
                    session.view_req = None;
                }
            }
            if let Some(req) = session.events_req.as_mut() {
                if let Some(result) = req.poll_timed(dt, REQUEST_TIMEOUT) {
                    delta = Some(result);
                    session.events_req = None;
                }
            }
            session
                .action_reqs
                .retain_mut(|req| match req.poll_timed(dt, REQUEST_TIMEOUT) {
                    Some(result) => {
                        reports.push(result);
                        false
                    }
                    None => true,
                });
        }

        // The `/view` poll is the heartbeat: its success or failure is the link
        // state. `/events` failures ride along silently (the next view drives it).
        if let Some(result) = fetched {
            match result {
                Ok(view) => {
                    self.mark_link_live();
                    self.adopt_view(view);
                }
                Err(_) => self.mark_link_lost(),
            }
        }

        if let Some(Ok(delta)) = delta {
            self.absorb_events(delta);
        }

        let mut acted = false;
        for report in reports {
            match report {
                Ok(report) => {
                    self.surface_feedback(report);
                    acted = true;
                }
                Err(_) => self
                    .notifications
                    .warning(self.data.strings.notifications.action_locked.clone()),
            }
        }
        // A command changed the shared world — re-fetch at once so the screens
        // reflect it without waiting for the next poll tick.
        if acted {
            self.request_view_now();
        }
    }

    /// Fold a `/events` delta into the session: advance the since-cursor and, once
    /// the baseline is established, surface newly-pushed events as notifications —
    /// the live "the world stirs while you watch" feed (GDD 7.4). The first reply
    /// only syncs the cursor, so the retained backlog isn't announced on connect.
    fn absorb_events(&mut self, delta: EventsDelta) {
        let synced = self.online.as_ref().is_some_and(|s| s.events_synced);
        if synced {
            // `since` yields oldest-first; show only the latest few, newest last.
            let skip = delta.events.len().saturating_sub(MAX_STIRRINGS_PER_POLL);
            for event in delta.events.into_iter().skip(skip) {
                self.notifications.info(event.message);
            }
        }
        if let Some(session) = self.online.as_mut() {
            session.cursor = delta.cursor;
            session.events_synced = true;
        }
    }

    /// Adopt a freshly-fetched projection: the world view the UI renders, plus
    /// the player's own private view (favor, level, Standing) — all server-owned.
    fn adopt_view(&mut self, view: ClientView) {
        if let Some(session) = self.online.as_mut() {
            session.connected = true;
        }
        self.view = view.world;
        self.player = view.player.player;
        self.standing = view.player.standing;
    }

    /// Force an immediate `/view` re-fetch if one isn't already outstanding.
    fn request_view_now(&mut self) {
        if let Some(session) = self.online.as_mut() {
            if session.view_req.is_none() {
                session.view_req = Some(session.client.fetch_view());
            }
        }
    }
}
