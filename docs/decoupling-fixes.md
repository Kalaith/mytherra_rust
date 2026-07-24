# Client/server decoupling — findings & work order (2026-07-24)

Review scope: verify the client renders and acts only on server-provided data in
online play, and that the server enforces visibility tiers on everything it
sends. The architecture is sound (server owns world + tick, `authorize`+`apply`
server-side, UI renders from `WorldView` only); the four issues below are the
leaks found. Fix them **in order** — 1 and 2 are correctness/design-integrity
bugs, 4 breaks progression online, 3 is a display bug.

Each issue is self-contained: symptom, cause with exact locations, the fix, and
acceptance criteria. General constraints at the bottom apply to all of them.

---

## Issue 1 — Online commands resolve targets from the stale local world (P0)

**Symptom (online play):**
- `TransferArtifact` silently does nothing (the button never sends a command).
- Once the server's world gains a region via genesis (fracture / conquest /
  frontier), the client cannot select it, and region-targeted commands
  (`RegionAction`, `CreateArtifact`, `ShapeWeather`, `AdvanceAgenda`) can be
  sent against the wrong region id.

**Cause:** three places read `self.world` — the local `WorldState` seeded at
startup that **never ticks while online** — instead of `self.view`, the
`WorldView` the server sends:

- `src/game/command.rs:69-78` — `selected_region_id()` indexes into
  `self.world.regions`.
- `src/game/command.rs:83-97` — `next_region_for_artifact()` reads
  `self.world.artifacts` and `self.world.regions`. Online the local world has
  zero artifacts, so it always returns `None` → no command is submitted.
- `src/game.rs:347` — `UiAction::SelectRegion(index)` bounds-checks against
  `self.world.regions.len()`; server-created regions beyond the local seed
  count can never be selected.

**Fix:** switch all three to `self.view` (`WorldView` already carries `regions`
and `artifacts`, same field names/shapes — see
`mytherra-protocol/src/view.rs`). Do NOT touch `authorized()` or
`apply_player_action()` in the same file — those are offline/capture-only and
correctly use the local world.

This is safe for the capture fixture too: capture runs at Elder standing
(`src/game/capture.rs:28-29`), so its projected view contains the full region
and artifact rosters.

**Acceptance:**
- `cargo test -p mytherra` passes; `cargo clippy --all-targets --all-features -- -D warnings` clean.
- Grep check: no `self.world` references remain in `selected_region_id`,
  `next_region_for_artifact`, or the `SelectRegion` arm.
- Add a unit test (or extend an existing one) that gives `Game`-level logic a
  `view` with an artifact + ≥2 regions and asserts `next_region_for_artifact`
  resolves round-robin from the **view**. If `Game` is hard to construct in a
  test, extract the two resolvers into free functions taking `&WorldView` and
  test those.

---

## Issue 2 — `/events` bypasses Standing visibility filtering (P0)

**Symptom:** any session — including a fresh Watcher — can call
`GET /events?since=0` and receive the entire shared chronicle, reconstructing
history about regions, wars, pantheon, etc. that its tier has not unlocked.
This defeats the visibility-tier design that `/view` carefully enforces.

**Cause:** `mytherra-server/src/main.rs:249-261` (`events` handler) returns
`authority.world.chronicle.since(query.since)` raw. Contrast with the view
projection, which gates the full chronicle behind `V::FullChronicle` and caps
everyone else at 32 recent events (`mytherra-protocol/src/view.rs:235-239`,
const `RECENT_EVENTS`).

**Fix:** filter the delta by the requesting player's Standing before returning
it. Put the filtering logic in `mytherra-protocol` (next to `project`, e.g.
`pub fn project_events(...)`) so it is a tested, shared rule rather than
server-inline code. Two layers:

1. **Volume rule (minimum required):** without `V::FullChronicle`, a player's
   delta must never let them accumulate deep history. Simplest correct rule:
   truncate the returned slice to the newest `RECENT_EVENTS` events (they
   already get the same 32 via `/view`, so this leaks nothing new).
2. **Kind rule (do this if `WorldEvent` carries a kind/category):** check what
   `WorldEvent` exposes (see `mytherra-core/src/world.rs` — the UI has an
   `EventKind::ALL` filter, so a kind exists). Map event kinds to the
   `VisibilityScope` that reveals them (e.g. region/war/weather events →
   `V::Regions`, pantheon events → `V::Pantheon`, hero events → `V::Heroes`)
   and drop events whose scope the player lacks. Keep the mapping conservative
   and data-local (a `match` in the protocol crate is fine). If a kind doesn't
   map cleanly, treat it as always-visible rather than inventing new scopes.

The handler already resolves the player index (`authority.index_of(...)`); it
just discards it. Use it to fetch the player and derive `standing_for(...)`
like the `view` handler does, then pass the standing into the filter.

**Important:** the cursor must still advance over *skipped* events (the client
stores `delta.cursor` and passes it back), so filter the events but return the
unfiltered `cursor` from `chronicle.since()`.

**Acceptance:**
- New protocol test: a Watcher standing receives a filtered/truncated delta; an
  Elder receives everything; the returned cursor equals the unfiltered cursor.
- Existing ignored integration test still passes against a live server:
  `cargo test -p mytherra -- --ignored net` (start the server first — see
  `RUNNING.md` / `run-server.ps1`).
- `cargo test -p mytherra-protocol -p mytherra-server` and clippy clean.

---

## Issue 3 — Server's pre-computed favor figures are discarded; dashboard recomputes from filtered data (P2)

**Symptom (online):** the dashboard's favor meter income figure under-reports
for any player below the Regions tier: it adds `faith_tithe()` over the
**view's** region list, which the server sends empty below that tier, while the
server actually tithes from the full world every tick.

**Cause chain:**
- `mytherra-protocol/src/view.rs:274-279` — `project()` pre-computes
  `PlayerView.max_favor` / `favor_recovery` "so the client needn't carry
  balance tables", but `favor_recovery` **omits the faith tithe**.
- `src/game/online.rs:337-344` — `adopt_view()` copies `player` and `standing`
  and **drops** `max_favor` / `favor_recovery` entirely (dead protocol fields).
- `src/ui/dashboard.rs:141-150` — recomputes both locally from
  `ctx.data.balance` and adds
  `crate::sim::faith_tithe(&ctx.world.regions, ...)` where `ctx.world` is the
  filtered `WorldView`.

The server applies the real tithe from the full world in
`mytherra-core/src/sim.rs:604`.

**Fix (three coordinated edits):**
1. In `project()`: compute `favor_recovery` as base recovery **plus**
   `faith_tithe(&world.regions, &data.balance.player)` (it has the full,
   unfiltered world). Update the doc comment on the field to say the tithe is
   included.
2. Thread the two figures to the UI: store them on `Game` when adopting a view
   (and when projecting locally for capture — same values come from the same
   `project()` call, so capture stays consistent), and expose them on
   `UiContext` (add `max_favor: i64`, `favor_income: i64` fields in
   `src/ui.rs` and populate in `Game::draw`, `src/game.rs:249-276`).
3. In `dashboard.rs`: delete the local recompute (lines 141-150) and render
   `ctx.max_favor` / `ctx.favor_income`. Remove the now-unused imports; the
   `crate::sim::faith_tithe` re-export may become unused in the client — if so,
   remove the client-side use, not the core function.

**Acceptance:**
- Protocol test: for a Watcher standing, `PlayerView.favor_recovery` equals
  base recovery + tithe of the full world (i.e. it does not depend on what the
  view reveals).
- No `ctx.data.balance.player` / `faith_tithe` reads remain in
  `src/ui/dashboard.rs` for the favor meter.
- Capture screenshot of the dashboard still renders sensible numbers:
  `.\scripts\capture_ui.ps1 -Scenes dashboard`, then view the PNG under
  `docs/verification/`.

---

## Issue 4 — Achievements never evaluate online (P1)

**Symptom (online):** players can never unlock achievements, never receive the
`achievement_experience` XP (which feeds deity level → tier progression per
`balance.player`), and the achievements panel shows an empty list — because the
server never populates definitions on the players it mints.

**Cause:**
- The predicate/unlock code lives in the **client** crate:
  `src/game/achievements.rs` (`earned`, `check`). The server cannot call it.
- `Game::check_achievements` runs only in the offline branch
  (`src/game.rs:184-193`); online, `self.player` is overwritten by the server's
  copy every poll anyway, so a client-side unlock would be clobbered —
  correctly so. The unlock must be server-side.
- Server-minted players start with `Achievements::new()` (empty; see
  `mytherra-core/src/world/player.rs:40`, minted at
  `mytherra-server/src/main.rs:111`) and nothing ever calls
  `sync_definitions` on them, so even the definitions list is empty in the
  `PlayerView` the client renders.

**Fix:**
1. **Move** `src/game/achievements.rs` into `mytherra-core` (e.g.
   `mytherra-core/src/sim/achievements.rs`, module `sim::achievements` — note
   the repo rule: `foo.rs` + `foo/` dir, never `mod.rs`). It only depends on
   `GameData`, `WorldState`, `PlayerState`, `bet_record`, `MagicState` — all
   already in core. Keep the XP grant with it: extract the "award
   `achievement_experience` per fresh unlock" logic that currently lives in
   `Game::check_achievements` (`src/game.rs:234-243`) into the core function so
   client and server can't drift; have it return the freshly-unlocked display
   names for notification.
2. **Server evaluates it**: in `tick_shared`
   (`mytherra-core/src/sim.rs:66...`), after the per-player favor/bet section,
   run the check for each player. Chronicle-or-drop the returned names — the
   client will see unlocks via its polled `PlayerView` either way; a chronicle
   event ("<deity> earned <achievement>") is optional, only add it if a
   fitting `EventKind` already exists and the string goes in
   `assets/data/strings.json` (data-driven rule).
3. **Definitions sync**: call `sync_definitions(data.achievements.clone())`
   when the server mints a guest (`Authority::new_guest`) and when loading
   players in `Authority::bootstrap` (definitions may have changed between
   server versions; `sync_definitions` is written to reconcile saved unlock
   state with current definitions — same call the client makes at
   `src/game.rs:227-231`).
4. **Client keeps its offline path working**: `Game::check_achievements`
   becomes a thin wrapper calling the core function and toasting the returned
   names (capture fixture only). The client-side unlock *notification* for
   online play can be driven by diffing unlocked state across adopted views if
   desired — optional; skip unless trivial.

**Acceptance:**
- Core test: construct a world+player meeting one predicate (e.g. set
  `player.nudges = 1` for `first_nudge`), run the check, assert it unlocks
  once, grants `achievement_experience` XP, and is idempotent on the second
  call.
- Server flow: after fix, a fresh guest's `/view` shows a non-empty
  `player.player.achievements` definitions list (extend the ignored
  integration test in `src/net.rs` if convenient).
- `cargo test` across the workspace (`cargo test -p mytherra -p mytherra-core -p mytherra-protocol`), clippy clean.
- Client crate no longer contains achievement predicates (file deleted, module
  removed from `src/game.rs`).

---

## Deferred / design notes — do NOT implement now, just recorded

- **Static-data skew:** client and server both embed `assets/data/*.json`; UI
  costs/thresholds and index-based wire fields (`stake_index`,
  `confidence_index`, `pattern_index`) assume both ends have identical tables.
  A future `SessionResponse` should carry a content hash/version and the client
  should warn on mismatch (GDD §7.6 territory).
- **Session security:** `X-Player-Id` values are sequential (`guest-0`, …) and
  trivially spoofable; CORS is permissive. Both are acknowledged M2-dev
  choices in the code — switch to random tokens + narrowed CORS before any
  non-localhost deployment. Do not fix as part of this batch.

---

## Constraints that apply to every fix (repo rules)

- `cargo fmt -- --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` must pass; run
  `cargo test` for every touched crate.
- 800-line hard cap per `.rs` file (soft 600). `sim.rs` is at ~1058 and already
  over — if Issue 4 would grow it, add the call site only and put the logic in
  the new `sim/achievements.rs` submodule; do not add bulk to `sim.rs` itself.
- Never create `mod.rs` files (`foo.rs` + `foo/` pattern).
- All player-facing text goes in `assets/data/strings.json`, all tuning in
  `assets/data/balance.json` — no hardcoded strings/numbers.
- No unused code left behind (delete, don't `_`-prefix).
- One focused commit per issue, message citing the GDD section it serves
  (match the existing commit style, e.g. "The herald acts on the world the god
  shows it, not the one it remembers (online target resolution)").
- Verify UI-visible changes with the capture harness:
  `.\scripts\capture_ui.ps1 -Scenes <scene>` renders PNGs into
  `docs/verification/` — read the image to confirm.
