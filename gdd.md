# Mytherra — Game Design Document

*Draft v0.1 — living document.*

> A minor deity, watching one living world that many other deities also watch. You don't
> command mortals — you nudge regions, cultivate champions, forge artifacts, and bet on
> what the world will do next. Everyone sees the same world. Everyone's bets are their
> own.

Sources: `game_apps/mytherra/` (React/PHP original), `RustGames/migration_candidates.md`,
`RustGames/standing.md`, `RustGames/docs/GAME_DEVELOPMENT_GUIDE.md`,
`RustGames/docs/CODE_STANDARDS.md`, `RustGames/docs/MACROQUAD_TOOLKIT.md`,
`RustGames/kaiju_sim/kaiju_server/` (the catalog's one precedent for a Rust-side server
component).

---

## 0. Migration Snapshot

- **Old game:** `game_apps/mytherra/` — a React 19 frontend (mostly React Context and
  bespoke polling hooks, not Zustand, despite that being this catalog's usual convention)
  over a PHP/Slim-style backend with a genuinely deep simulation: region/settlement/
  resource ecology, hero and champion lifecycles, a fixed-odds betting system, seven
  "divine tool" subsystems (Artifacts, Weather, Omens, Magic, Myths, Civilization,
  Pantheon), and a 100-year era/legacy system. This is the largest and most mechanically
  complete of the three web games this catalog has scoped for a Rust port so far.
- **Why it was picked:** `migration_candidates.md`'s explicit top pick — "the cleanest
  genre gap, the fantasy (watch a world, nudge it, no direct control) is inherently about
  simulation depth over spectacle, and it plays straight to what an AI-driven,
  artist-light dev process is actually good at."
- **The multiplayer decision.** Unlike `stellar_legacy` and `dragons_den` (both scoped as
  standalone local-save single-player ports), this project is being scoped **as a live,
  shared-world multiplayer game from the start** — one persistent world, all players
  observing and nudging it, each player's bets and Divine Favor their own. This is a
  deliberate departure from every other game in this catalog and the single biggest
  architectural decision in this document (§7).
  - **Why this genuinely works here and wouldn't for most of the catalog's games:** the
    original design already assumes a world nobody fully controls — "no direct command"
    is the pitch, not a limitation. Turning "one deity nudging an indifferent
    simulation" into "many deities nudging the same world, in view of each other" is a
    small conceptual step, not a genre change, and it's the one system in this doc that
    genuinely cannot be replicated by a single-player toolkit save file — betting against
    *other people's* reads of the world, and having your nudges visible to them, is new
    gameplay texture, not just an infrastructure upgrade.
  - **Confirmed by the source itself:** the backend's `game_states` table has a literal
    `singleton_id` primary key, and `regions`/`heroes`/`settlements`/`landmarks`/
    `game_events` carry **no player-scoping columns at all** — this was already one
    shared world server-side. What it lacked was per-player economy: a deep-dive into
    `Player::getSinglePlayer()` found the entire game currently spends Divine Favor from
    **one hardcoded `SINGLE_PLAYER` row shared by every account, guest or logged-in** —
    i.e. the original is accidentally single-player at the economy layer despite being
    genuinely shared-world at the simulation layer. The port makes this real and
    intentional instead of an artifact of how the prototype was built (§5.1, §7).

- **Art-liability audit.** Same finding as every other web game surveyed in this catalog
  so far: zero image assets. A full grep of `frontend/src` for
  `.png/.jpg/.svg/.webp/.gif` returns nothing; there isn't even a `frontend/public`
  directory. All "iconography" is inline emoji in JSX; the only visual library in use is
  Chart.js for the Dashboard's stat graphs. Nothing to cut for art reasons — every call
  below is systems/content/architecture, not art avoidance.

  | Old asset (web) | Art cost | Rust replacement |
  | --- | --- | --- |
  | Inline emoji, Tailwind panels | None | `ui`/`SurfaceStyle`/`TextStyle`, same as every other port |
  | Chart.js bar/pie stat graphs (Dashboard) | Low (a JS charting lib, no art) | Simple bar/line rendering via `ui`/`math`, or a small project-local chart widget if the toolkit lacks one |

- **Mechanic carry-over table.** Grouped by subsystem given the scale of this game.

  **Region / world simulation**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Region stats (prosperity/chaos/danger/magic_affinity/population/climate/cultural_influence/divine_resonance) | Keep as-is | Real, working tick formulas (§5.2). |
  | Bless / Corrupt / Guide Research actions, resonance-scaled cost/effect | Keep as-is | Clean, already-tuned formula (§5.2). |
  | `DivineInfluenceService` (a second, parallel influence system) | Cut | Confirmed broken: treats an enum as numeric (silent no-op via PHP loose typing), has a literal `// TODO: Implement history-based reduction` stub, and duplicates `Region`'s real influence path. Not a design choice to preserve — an abandoned parallel prototype. |
  | `corruption_level` column references (`RegionRepository::updateCorruptionLevel`, a betting filter) | Cut | The column doesn't exist in the schema — dead code referencing a field that was never added. |
  | Region status set / crisis-detection (`isInCrisis`/`isThriving`) | Keep concept, fix values | The original checks status strings (`'warring'`, `'peaceful'`) that don't match the real seeded status list (`war_torn`, etc.) — a genuine bug, not intentional design; port aligns the enum. |
  | Settlement growth, resource-node status state machine | Keep as-is | Real formulas (§5.3). |

  **Heroes / champions**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Hero level-up/aging/death/region-move formulas (`Hero.php`/`GameLoopService`) | Keep as-is | Real, tuned (§5.4). |
  | `HeroLifecycleService` (a rival implementation) | Cut | Confirmed dead: calls a nonexistent method, makes a static call to an instance method — never invoked by the tick, never worked. |
  | Champion designation/cultivation/rivalry resolution | Keep as-is | Deterministic strength-vs-threat rivalry resolution (not a dice roll) is genuinely good design (§5.4). |
  | Several `InfluenceActions` methods (`empowerHero`, `guideHero`, `guideRegionResearch`) | Cut or implement for real | Currently canned "success" responses bound to live routes with no actual state mutation — placeholder endpoints, not working features. |

  **Betting**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Fixed-odds formula (`base_odds × confidence × timeframe × target_modifier`) | Keep as the "fair value" layer | Real and reasonable (§5.5). |
  | Payout house-edge curve by confidence tier | Keep as-is | Well-designed risk/reward shaping (§5.5). |
  | `war_outcome` bet type | Cut | Declared with a base odds value but has no resolution case anywhere — can never pay out. Dead content. |
  | Betting config tables (`bet_types`, `confidence_levels`, `timeframe_modifiers`, `bet_target_modifiers`) | Keep concept, **actually seed them** | These ship completely empty in the original — the whole system runs on hardcoded PHP fallback constants because its own seeder path (`GameDataSeeder`) references `InitData` classes that don't exist anywhere in the repo and would fatal-error if run. Port ships real seed data from day one. |
  | `target_modifier` reading a `target_statistics` table | Fix | That table doesn't exist; the original silently falls back to fixed placeholder stats (settlement prosperity always "50", hero level always "5", etc.) for most bet types, so odds for those types never actually react to the real target. Port reads live target state directly — no fallback needed once real tables exist. |
  | Pari-mutuel / crowd-aware odds | **New, multiplayer-only addition** | The original is a pure fixed "house odds" model — verified nothing anywhere reads aggregate player stakes. This didn't matter in a single-viewer game; it's the single biggest missed opportunity once the world is genuinely shared. Port adds a crowd-lean payout adjustment on top of the kept house-odds formula (§5.5) — this is the concrete answer to "what does multiplayer actually add to the gameplay," not just to the tech stack. |
  | `DivineBettingService` (a second, incompatible betting model) | Cut | Wired via DI but never called — leftover prototype with the wrong enum values and payout formula. |
  | `ComboBetService` (parlays) | Cut for v1 | Never persisted anywhere in the original (its own code comment admits "can be persisted to a combo_bets table" — no such table exists), so combo bets are already stateless and unresolvable as shipped. Revisit only once single-leg betting is solid. |
  | Two duplicate `BettingBaseRepository` classes referencing a nonexistent `bets` table | Cut | Unreachable dead code. |

  **The seven "divine tools" (Artifacts, Weather, Omens, Magic, Myths, Civilization, Pantheon)**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | All seven systems' actual mechanics (see §5.6 for a summary of each) | Keep as-is | Unlike almost everything else surveyed in this catalog so far, these are **genuinely complete, non-trivial simulations** — no stubs, no TODO markers found across any of the seven services. This is content to preserve carefully, not redesign. |
  | Storage as a single JSON blob per system inside a generic `game_configs(category,key,value)` table | Redesign — mandatory, not optional | This pattern is fine for one concurrent writer (the original's accidental single-player-at-the-economy-layer reality). It is **not safe** for real concurrent multiplayer writes — two players acting on the Pantheon or Magic system in the same moment would race on a read-modify-write of one JSON column. The port gives each system real relational tables (§6). |
  | Omens (read-only forecasting, never mutates world state) | Keep as a deliberate design choice | Confirmed intentional, not a stub — a nice contrast to the mutating tools. |

  **Era system**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Era pressure (5 weighted triggers), 100-year era length, transition carryover/reset rules (settlement decay tiers, hero reincarnation/death, bet carryover) | Keep as-is | Genuinely well-differentiated, not a blanket wipe — good design, and a natural fit for a live-service world's long-term pacing (§5.7). |
  | Era-generation name banks (small fixed 4-word prefix/title cycles) | Expand | Mechanically fine, but the name variety is thin — a content-volume gap to close (§9), not a mechanic to fix. |

  **Auth / player identity**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Guest-session creation + WebHatchery account linking | Keep as-is | Already fully built and working — reused directly rather than inventing new auth (§7). |
  | Single shared `SINGLE_PLAYER` Divine Favor row for the whole game | Redesign — this is the core economy fix | Every account currently spends from one global favor balance regardless of who's logged in. The port gives each player their own persisted Favor balance; the world (regions/heroes/divine-tool state) stays global and shared, but the resource you spend to act on it is now genuinely yours (§5.1, §7). |
  | `guild_id`/`joinGuild()` on the `User` model | Revive as a real feature (open question, §13) | Aspirational dead code in the original — no `guilds` table exists. Given this port is explicitly multiplayer, a real "pantheon/faction" grouping (players banding together, pooling favor or coordinating bets/nudges) is a natural, low-art feature to consider, not a random addition. |

  **Frontend architecture**

  | Old mechanic | Disposition | Notes |
  | --- | --- | --- |
  | Pure blind-refetch-after-mutation, no conflict handling, opt-in polling per page (10-30s intervals) | Redesign | This was tolerable when the game was accidentally single-player at the economy layer. A real shared world with concurrent actors needs an actual "what changed since I was last here" model, not silent refetches (§7). |

- **Explicitly out of scope for the port:** real-time PvP combat, live chat/voice, a
  fully pooled (non-house) pari-mutuel betting engine for v1 (§5.5 ships a hybrid, not a
  full liquidity market), any WebSocket/real-time push transport for v1 (polling is
  sufficient at this game's pace — see §7), and the parallel/dead systems listed above.

---

## 1. High Concept

- **Pitch:** One persistent world. Many quiet gods. You read what changed since you last
  looked, nudge what you can afford to nudge, bet on what you think happens next — and so
  does everyone else watching the same world.
- **Genre:** God-game / prediction-market hybrid, and — new for this catalog — a
  **persistent shared-world multiplayer game**. No other game in `standing.md` has a
  world that exists independently of any one player's save file.
- **Perspective & presentation:** UI-only, text/data-forward — event timelines, region
  cards, a world map that's a grid/list rather than a rendered terrain, stat meters,
  odds/payout panels. Same lean `ui`-module-only footprint as `stellar_legacy`, just at
  much greater breadth (many more screens/systems).
- **Tone:** Detached and archival — a chronicle being written in real time, occasionally
  interrupted by very personal stakes (your bet, your champion, your favor).
- **Comparables:** *NationStates* (a persistent shared text-driven world checked in on
  asynchronously), prediction-market platforms like *Polymarket*/*Metaculus* (wagering on
  real-world-shaped uncertain outcomes as the core loop), *EVE Online*'s slow-burn "the
  server keeps running while you're gone, and other people's politics matter" texture,
  scaled down to a scope a small team can actually ship.
- **Audience:** Players who like systems-deep god-games and prediction markets more than
  spectacle, and who specifically want a world that keeps being true whether or not
  they're logged in — the asynchronous, "the world doesn't wait for you" crowd.
- **Scope:** Full game — the largest of the three ports scoped so far, both mechanically
  and architecturally (§7 is new work this catalog hasn't done before).
- **Platforms:** Native Windows + WebGL client, talking to a persistent server (§7) —
  this is the one game in the catalog that cannot ship as a fully offline itch.io/Steam
  binary; it requires an always-on backend, which is a real ongoing commitment distinct
  from every other port here (flagged again in §12).

---

## 2. Design Pillars

1. **The world belongs to everyone, not to you.** No player has a private instance.
   Every nudge, bet, and champion action happens in a world other people are also
   watching and shaping. If a system only makes sense from one player's point of view,
   it doesn't belong in this design.
2. **Betting must react to the crowd, not just the sim.** This is the pillar that
   justifies going multiplayer at all (§0). If odds only ever reflect world-state, a bet
   is just "did you read the simulation correctly" — solvable in single-player, and
   already true of the original. Odds reacting to what *other people* have staked is the
   one thing only a shared world can give this game (§5.5).
3. **Your favor is yours; the world is not.** Fixing the original's accidental
   single-shared-favor-pool bug is not just a bug fix — it's the load-bearing design
   principle that makes concurrent play fair. Every player's economy is private; every
   player's *target* (regions, heroes, the pantheon) is shared.
4. **Influence is public, and manipulation is visible, not hidden.** Because nudges and
   bets happen in a shared world, someone blessing a region right before their own bet
   resolves is a legitimate, interesting, and *visible* strategic act — not a bug to
   prevent, but a piece of "divine politics" the event log should surface plainly so
   other players can react to it (counter-nudge, bet against the manipulator, call it
   out). See §7's rate-limiting for the guardrail that keeps this from being simply
   "whoever has the most favor always wins."
5. **The world doesn't wait for you.** Ticks run on a server schedule, not a per-player
   button. Part of the fantasy is genuinely being a minor deity checking in on a world
   that has kept moving without you — this must be true architecturally, not simulated.

---

## 3. Core Loop

**Per-visit session rhythm** (kept from the original's own framing, now explicitly
multiplayer):

1. Log in (guest session or linked account) and see what changed in the shared world
   since your last visit — events, other players' visible nudges/bets, tick results.
2. Inspect regions, heroes, champions, the seven divine tools, and active speculation
   events for opportunities.
3. Spend your own Divine Favor — bless/corrupt/guide a region, cultivate a champion,
   forge or empower an artifact, nudge weather, research magic, promote a myth, advance
   a civic agenda, appease or challenge a pantheon deity.
4. Place bets where the visible odds — now shaped by both world-state *and* other
   players' aggregate stakes (§5.5) — look favorable to your read.
5. Wait for the next server tick (or the next time you check in); the world advances on
   its own schedule regardless of whether you're present (§7).
6. Review outcomes: resolved bets, champion quest/rivalry results, era pressure changes,
   and what other deities did while you were away.

**World-level loop** (spans the whole persistent world, not any one player):

1. The server ticks on its own schedule, advancing every region/settlement/resource/
   hero/champion/divine-tool/pantheon system and resolving due bets, independent of any
   single player being online.
2. Era pressure accumulates from world-state (danger, chaos, prosperity collapse, active
   divine-war stakes); when it crosses a threshold or the era's calendar length elapses,
   an era transition reshapes the world — settlements decay or endure, heroes reincarnate
   or pass on, some bets carry across the boundary.
3. The chronicle of what happened persists and accumulates, visible to every player, for
   as long as the world runs.

---

## 4. Player Role & Verbs

- **The player is:** one of potentially many minor deities quietly watching and nudging
  the same world. Never an avatar with a body in the world.
- **The player directly controls:** their own Divine Favor spending — region actions
  (Bless/Corrupt/Guide Research), champion designation/cultivation, artifact creation/
  empowerment/transfer/stabilization, weather nudges, omen requests, magic research
  direction, myth promotion, civilization agenda nudges, pantheon appease/challenge, and
  bet placement.
- **The player does NOT control:** any other player's actions, the tick schedule (server-
  driven, §7), individual hero/settlement/resource-node behavior beyond the nudges
  above (all of it simulated), or the outcome of a bet once placed.
- **Core verb list:** *Read* (events/dashboard), *Bless/Corrupt/Guide* (regions),
  *Cultivate* (champions), *Forge/Empower/Transfer/Stabilize* (artifacts), *Shape*
  (weather), *Request* (omens), *Research* (magic), *Promote* (myths), *Advance*
  (civilization agendas), *Appease/Challenge* (pantheon), *Bet* (speculation events).
- **Views and verbs are progressively revealed** by the player's Standing (§5.9): a
  freshly-woken deity sees and acts on only a narrow slice of the world (Heroes, a hero-
  scoped betting market), and unlocks broader visibility and higher-altitude verbs — up to
  world-shaping weather and region-collapse wagers — as it grows. The list above is the
  *fully-revealed* verb set, not what a new player starts with.

---

## 5. Systems & Mechanics

### 5.1 Divine Favor Economy (redesigned)

| Stat | Meaning | Scope |
| --- | --- | --- |
| Divine Favor | Spendable resource for every action in §4 | **Per-player** (fixes the original's shared `SINGLE_PLAYER` row, §0) |
| Favor recovery | Passive gain per tick | Kept concept: `+10`/tick, credited to every active player individually |

Every cost/effect formula below (region actions, champion cultivation, artifacts,
weather, bets) is unchanged from the original in *shape* — only the ledger it draws from
changes, from one shared row to one row per player.

### 5.2 Region System

Regions carry `prosperity/chaos/danger/magic_affinity/population/climate_type/
cultural_influence/divine_resonance` (kept as-is, real schema).

```text
Bless Region:   cost 15 -> prosperity +8, chaos -4, danger -3
Corrupt Region: cost 15 -> chaos +8, danger +5, prosperity -3
Guide Research: cost 12 -> magic_affinity +7, prosperity +2

cost_multiplier   = clamp(0.7, 1.3,  1 - (divine_resonance-50)*0.006)
effect_multiplier = clamp(0.75,1.35, 1 + (divine_resonance-50)*0.007)
final_cost   = max(1, round(base_cost * cost_multiplier))
final_effect = max(1, round(|delta| * effect_multiplier))   // sign preserved
```

Per-tick drift (kept as-is):

```text
prosperity_delta = (chaos>70 ? -3 : chaos<30 ? 2 : 1) + resource_pressure + settlement_pressure + magic_culture_pressure
chaos_delta, danger_delta similarly, all clamped 0-100
```

A region's dominant culture (scholarly/martial/mystical/mercantile/pastoral) is scored
from heroes/landmarks/resources/settlements/trade-routes each tick, but only flips with
an inertia guard (`top_score >= current_score + 3`) — kept as-is, it's good design.

### 5.3 Settlement & Resource Simulation

```text
growth_rate = 0.02 + (prosperity-50)/2500 + (region_prosperity-50)/3000
              - region_chaos/5000 + resource/defense/civic pressure
              clamped [-0.03, 0.08]
population *= (1 + growth_rate)
```

Resource nodes cycle through a status state machine (active/blessed/flourishing/
overworked/contested/corrupted/unstable/depleted) with roll-based transitions (e.g.
contested→corrupted at `0.14 + danger_pressure*0.03` when regional chaos≥65) and status-
based output multipliers (depleted ×0 up to flourishing ×1.5). Kept as-is.

### 5.4 Heroes & Champions

```text
level_up_chance(level) =
  level<=15:  0.3 * 4.0 * 0.95^(level-1)
  level 16-49: 0.3 * 1.5 * 0.95^(level-1)
  level>=50:  0.3 * 0.3 * 0.95^(level-1)

life_expectancy = 70 + level*2
death: if age > life_expectancy -> 20% roll; else danger_chance = max(0.005, danger/1000 - level/3000), rolled every tick
region_move_chance = 12% flat, per tick
```

Champions (a small player-cultivated roster, max 3): cultivation cost
`15 + rank*5 + focus_cost_modifier`; rank `= min(10, max(current, 1+bond/25, 1+quests/3))`;
quest progress per tick `= max(8, min(35, 7 + rank*3 + bond/12 + level/8 + focus_bonus))`.
Rivalry resolution is **deterministic, not a dice roll**:

```text
strength = bond + rank*8 + level*2
threat   = pressure + danger/2 + chaos/3
resolved if strength >= threat (region danger -5, chaos -3, prosperity +1)
else escalated (danger +4, chaos +3, pressure +8)
```

All kept as-is — genuinely good design.

### 5.5 Betting — the Divine Observatory

**Fixed "house" odds** (kept from the original as the fair-value baseline):

```text
odds = base_odds(bet_type) * confidence_modifier * timeframe_modifier * target_modifier
final_odds = max(1.1, round(odds, 2))
```

| Confidence | Odds modifier | Stake multiplier |
| --- | --- | --- |
| Long shot | 2.0 | 0.5 |
| Possible | 1.0 | 1.0 |
| Likely | 0.7 | 1.5 |
| Near-certain | 0.4 | 2.0 |

Payout (kept as-is — a real house-edge curve, not a flat `stake*odds`):

```text
raw_multiplier  = odds * stake_multiplier(confidence)
house_edge      = long_shot 0.92 / possible 0.88 / likely 0.86 / near_certain 0.80
gross_multiplier = clamp(min_mult, max_mult, 1 + (raw_multiplier-1)*house_edge)
gross_payout    = max(stake+1, floor(stake * gross_multiplier))
```

**New: crowd-lean payout adjustment** (the multiplayer-only addition justified in §0/§2):

```text
crowd_lean = total_stake_on_this_outcome / total_stake_on_all_outcomes_for_this_event
payout_odds = house_odds * clamp(0.6, 1.5, 1 / (0.5 + crowd_lean))
```

Heavily-backed outcomes pay out less than the pure house odds would suggest; thin/
contrarian positions pay more — a real pari-mutuel-flavored adjustment layered on top of
the world-state-derived fair value, so betting well now means reading *both* the
simulation and the other deities watching it. Bounds are placeholders for a first
balance pass, not final tuning.

Bet types: kept, minus `war_outcome` (§0 — never resolves in the original, dead
content). Resolution stays a real per-tick world-state check (e.g. `hero_death`:
`!hero.is_alive`; `prosperity_threshold`: `settlement.prosperity>=80`).

### 5.6 The Seven Divine Tools

All seven are genuinely complete simulations in the original (no stubs found) — kept
as-is mechanically; only their storage changes (§6). Brief summary of each:

- **Artifacts** — up to 9 active; 4 starter relics; create/empower/transfer/stabilize
  costs (40 / `20 + power*10 + instability/5` / 8 / 15); 4 focus types (protection/
  prosperity/war/knowledge); multi-step delayed consequence chains (2-3 steps) that
  mutate real Region/Settlement/Hero/Landmark state.
- **Weather** — 3 intensities × 5 patterns; cost scaled by divine resonance; delayed
  consequences within 5 years, decaying 0.08/step.
- **Omens** — 3 horizons (near/generation/era-dynamic); region pressure
  `= chaos*0.38 + danger*0.42 + (100-prosperity)*0.2`; **deliberately never mutates
  world state** — a read-only forecasting tool by design.
- **Magic** — 5 research paths; `known` at progress≥100 & evidence≥82, `emerging` at
  progress≥35 & evidence≥55; genuinely mutates Region/Hero/Landmark/Settlement/
  ResourceNode state — the deepest of the seven.
- **Myths** — promotion cost 22, myth cap 24, echo cooldown 4yr/threshold 58; candidates
  scored from real event history.
- **Civilization** — 6 competing regional agenda scores (expansion/defense/trade/
  rivalry/research/recovery), recomputed live from weighted-linear formulas; only
  agendas scoring ≥35 apply per tick; 5-year diplomacy cooldown.
- **Pantheon** — 4 AI deities in a fixed ally/rival diamond; per-domain pressure tiers at
  25/35/55/75; appease 12 / challenge 18 favor; 3-year relationship-arc cooldowns.

### 5.7 Era System

```text
era_length = 100 years (calendar trigger) OR era_pressure >= 85 ("breaking", forces early transition)
era_pressure = highest-scoring of 5 weighted triggers:
  cataclysm       = danger*0.42 + chaos*0.22 + hazard_ratios
  collapse        = (100-prosperity)*0.38 + distressed/depleted ratios
  conquest, magical_rupture, divine_war (= active bet stakes*0.24 + fallen-hero ratio + low-favor*0.14)
```

Transition carryover is genuinely differentiated, not a blanket reset: settlements decay
by legacy/scarred multipliers (×0.34 scarred / ×0.55 default / ×0.78 legacy-anchored);
heroes either reincarnate (age reset 18-34, scaled stats) or age/die (35% random death or
age≥75); bets spanning the boundary are explicitly carried or force-expired; 1-4 new
descendant heroes are created. Kept as-is — a good long-run pacing mechanic for a live
shared world specifically, since it gives the persistent world a natural "chapter break"
without ever needing a player-triggered reset.

### 5.8 Randomness & Determinism

The original uses a mix of real dice rolls (`random_int`-based, e.g. hero level-up/death)
and deterministic pseudo-randomness (`crc32(seed)%100` for artifact/weather risk
resolution). For the multiplayer port, **all resolution must be server-side and
auditable** — more so than any single-player port in this catalog, since players cannot
be trusted to verify each other's outcomes. The server owns every roll; the client never
computes an outcome, only requests actions and displays results (§7).

### 5.9 Deific Standing & Progressive Revelation

A new deity wakes to a small, legible world and grows into the full pantheon-god's view
over many sessions. What a player can *see* and *do* is gated by their **Standing** — a
per-player set of unlocked capabilities (§6) — so onboarding is narrow and the endgame is
systemic. This layer gates *access only*; it changes none of the cost/effect formulas in
§5.1–5.6.

Standing moves along two orthogonal axes:

- **Visibility scope** — which entities/screens are revealed: Heroes → Regions →
  Settlements/Resources → Divine Tools → Pantheon/Eras → the full Chronicle.
- **Influence altitude** — which verbs (§4) are available, from *direct/local* (Bless a
  region, cultivate a champion) to *systemic/indirect* (shape weather, steer a magic path,
  tilt a civilization agenda) — where the deity perturbs an input and the simulation
  propagates the outcome rather than dictating it.

Capabilities are three data-driven flag groups on the player, unlocked in named bundles
("tiers") defined in `tiers.json` (copy in `strings.json`, thresholds in `balance.json`):

- `VisibilityScope` — Heroes, Regions, Settlements, Resources, DivineTool(kind),
  Observatory, Pantheon, Eras, FullChronicle
- `ActionVerb` — RegionAction, Champion, Weather, Artifact, Magic, Myth, Agenda, Pantheon
- `BettingMarket` — which `BetPredicate` families may be wagered on (hero-scoped →
  region-scoped → world/era-scoped)

Tiers are **purely additive**: ascending grants new scopes/verbs/markets and never removes
an earlier one. A reference progression:

| Tier | Newly revealed | New verbs | New betting markets |
| --- | --- | --- | --- |
| 0 · Watcher | Heroes, Chronicle | — (or one cheap hero-adjacent nudge) | hero reaches legend, hero death |
| 1 · Patron | Regions | Bless/Corrupt/Guide, champion cultivation | region prosperity, settlement growth |
| 2 · Shaper | Settlements, Resources, Artifacts/Magic/Myths | forge/empower artifacts, research magic, promote myths | famine, plague, war |
| 3 · Elder | Weather, Civilization, Pantheon, Eras | shape weather, advance agendas, appease/challenge deities | region collapse, age ends |

High Standing is defined by *reach*, not by giving anything up — but the highest-altitude
playstyle is deliberately **emergent and optional**. A tier-3 Elder who has unlocked
Weather and a region-collapse market can win a "this region will be destroyed" wager
**without ever acting on the region directly**: the simulation already chains
`weather::tick_weather` (drought suppresses resource output) → `famine::tick_famine` (the
harvest fails and the region starves) → `refugee::tick_refugees` (its people flee) →
`genesis::tick_genesis` (the collapsed, undefended region is conquered and removed). The
Elder shapes the skies; the world does the rest. Every such nudge and bet is attributed in
the public event log (§7.5) — "visible manipulation is a feature" — so rival deities can
counter (bless the region, shape counter-weather, bet the other side), which keeps a
high-favor whale from acting unopposed. This market needs a `region_collapse` predicate
added alongside the existing `AgeEnds`.

Unlock hooks (all already present in the code or trivially derived):

- **Standing/level** — the existing favor-driven `PlayerState.level` (§5.1) maps to tier
  thresholds.
- **Achievements** — `macroquad_toolkit::achievements` is already wired into
  `PlayerState`; a milestone reveals a scope.
- **Witnessed events** — seeing a first Legend reveals Regions; surviving a first famine
  opens the famine/weather markets — tying revelation to the living world.
- **Favor-purchased ascension** — an explicit favor sink that buys the next tier.

Enforcement is server-authoritative (§7.7): the server sends each player only the
projection their scopes reveal and rejects any verb they have not unlocked. Client-side
gating is presentation only.

---

## 6. Data Model

Two categories of state, kept explicitly distinct per Pillar 3:

**Shared/global tables** (one row set for the whole persistent world):
`regions`, `settlements`, `resource_nodes`, `buildings`, `landmarks`, `heroes`,
`game_events`, `game_state` (singleton tick/year counter), and — new relational tables
replacing the original's `game_configs` JSON-blob pattern — `artifacts`,
`weather_state`, `omens`, `magic_paths`, `myths`, `civilization_agendas`,
`pantheon_deities`, `pantheon_relations`, `era_history`.

**Per-player tables** (one row set per account):
`players` (favor balance, level/experience), `player_bets` (formerly `divine_bets`,
already correctly player-scoped), `player_champions` (a player's cultivated roster),
`player_achievements`, `player_standing` (the unlocked `VisibilityScope`/`ActionVerb`/
`BettingMarket` flag sets that drive §5.9's per-player projection and authorization), and
— if the guild/pantheon-faction idea (§0, §13) is pursued — `player_guilds`/
`guild_members`.

```json
// bet_types.json — seed data the original never actually shipped (§0)
{ "id": "settlement_growth", "base_odds": 2.2, "requires_target": "settlement" }
```

```json
// confidence_levels.json
{ "id": "likely", "odds_modifier": 0.7, "stake_multiplier": 1.5 }
```

The server owns a real database (MySQL, matching the existing backend, or SQLite/
Postgres if the backend is eventually rewritten in Rust, §7) — this is **not** a
`macroquad-toolkit::persistence` local-save situation; there is no local save file for
world state at all, only a cached view and an auth token (§7, §12).

---

## 7. Multiplayer & Shared-World Architecture

*This section exists because this game, uniquely in this catalog, cannot ship as a
locally-simulated, offline binary. Every other port in `RustGames` uses
`macroquad-toolkit::persistence` for a local save file; this game's "save" is the live
server's database, shared by every player.*

> **Status update (supersedes the earlier "keep PHP" recommendation, §7.2).** This
> architecture is now **built** (through M2). The simulation was ported to Rust as a clean,
> deterministic, `serde`-serializable core, and the workspace was realized as a **five-crate
> Rust workspace** — `mytherra-core` (sim) / `mytherra-protocol` (shared wire types) /
> `mytherra-persistence` (MySQL storage) / `mytherra-server` (axum authority) / `mytherra`
> (the macroquad client, the root package) — described in §7.2 and §12. A dedicated
> persistence crate was added beyond the original four so the pure core never links a
> database (§6/§8). The remaining PHP/MySQL "keep the backend" plan is fully superseded.

### 7.1 Server authority model

- **The server is the sole simulation authority.** All economy math, tick advancement,
  bet resolution, and RNG happen server-side (§5.8). The Rust/macroquad client is a thin
  renderer + input layer: it displays server-reported state and submits action requests;
  it never computes an outcome locally, mirroring (and going further than) `dragons_den`'s
  "server determines gold, not the client" principle.
- **Ticks run on a server schedule**, not a per-player button (Pillar 5) — matching the
  original's cron/queue-worker model (`GameTickJob` self-requeuing every 60s, gated on a
  `simulation.enabled` flag), but the **real-world-to-game-year pace is an open question**
  (§13), not something to copy uncritically — the original's roughly 1-real-minute-per-
  year cadence reads like a development/testing artifact more than a deliberate "check in
  a few times a day" pace for a live shared world.

### 7.2 The simulation is already a Rust core — build a Rust-native server around it

An earlier draft of this section recommended keeping the original PHP/MySQL backend as the
authority and *not* rewriting the simulation in Rust for v1. **That recommendation is
superseded.** The simulation has since been fully ported to Rust, and the port is exactly
the clean, headless authority this architecture needs:

- `sim::tick_world(&mut world, &mut player, &data)` is a **pure, deterministic** function.
  The world's `SeededRng` lives in state and is serialized, so a save — or a server row —
  resumes the exact same sequence (§5.8; guarded by a byte-for-byte two-run determinism
  test).
- `WorldState` (shared) and `PlayerState` (per-account) are already the serialization
  contract, split along the §6 shared/per-player line.
- `world/`, `sim/`, and `data/` carry **no rendering dependency** — they touch only pure
  toolkit utilities (`rng`, `math`, `data_loader`, `achievements`), never macroquad.

So the "stretch/alternative" Rust path (`axum` + `tokio` + `sqlx`, following the
`kaiju_sim/kaiju_server` precedent already in this workspace) is now the **recommended**
path, and the work is *extraction + interface design*, not reimplementation from scratch.
The realized layout is a **five-crate Cargo workspace** (one repo — this is still one game):

- **`mytherra-core`** (lib) — `world/`, `sim/`, `data/`, serialization; the authority's
  brain. No macroquad, no I/O. Compiles to wasm (the client embeds it for the capture
  fixture).
- **`mytherra-protocol`** (lib) — the shared wire types both ends compile against so they
  can never drift: the per-player `WorldView`/`PlayerView` projections, the `PlayerAction`
  command enum (the mutating subset of today's `UiAction`), the §5.9 capability/tier
  definitions, error types, and the since-cursor event-delta types (§7.4).
- **`mytherra-persistence`** (lib) — all MySQL storage (`sqlx`, migrations), depending on
  core but sitting *outside* it so the pure core (and the wasm client) never link a
  database. Two dissociated stores — the shared world and the per-deity player domain
  (§6/§8) — that share a pool but never a row.
- **`mytherra-server`** (bin) — `axum`/`tokio`; owns the world + one `PlayerState`
  per account, runs the tick loop (§7.1), authorizes actions and projects per-player views
  (§7.7), and write-throughs to the persistence store (the DB *is* the save, §6/§8).
- **`mytherra`** (bin, the root package) — today's `ui/` + loop, rendering a received `WorldView` and
  emitting `PlayerAction`s; no local `tick_world`. It is **online-only**: real play requires
  a running `mytherra-server` to connect to — there is no playable local-world/offline mode.
  It still embeds `mytherra-core`, but only so the headless **screenshot capture fixture**
  (`game/capture.rs`) can drive a throwaway local world to render each screen for CI/visual
  verification; that fixture never touches the network and is not a play mode. (Earlier
  drafts planned an offline single-player mode to ship the §5.9 revelation feature before
  networking — that was delivered in M0.5 and then removed once the server landed.)

**Workspace prerequisite:** `mytherra-core`/`-server` must not link a GL/windowing engine,
but the pure toolkit modules they need (`rng`, `math`, `data_loader`, `achievements`) live
in `macroquad-toolkit`, which depends on macroquad. Either feature-gate those modules to
build window-free, or extract them into a headless `macroquad-toolkit-core` crate that both
the toolkit and the server depend on — a win for every game in the catalog, not just this
one.

### 7.3 Auth

Reuse the original's existing, already-working auth wholesale: guest-session creation
(no account needed to start playing — important for a standalone client) plus optional
WebHatchery account linking for cross-device continuity. No new auth system needed.

### 7.4 Networking model

- **Polling over HTTP**, not WebSockets, for v1 — this game's pace (check in, read what
  changed, act, wait for a tick) does not need real-time push, and polling avoids an
  entire class of connection-management complexity a small team doesn't need to take on
  yet.
- **Fix the original's real gap**: the web frontend does blind refetch-after-mutation
  with no "what changed since I was last here" model (confirmed: `RegionProvider` fetches
  once on mount with no refresh; most divine-tool pages never poll at all). The Rust
  client needs a proper delta/since-cursor endpoint (event log already supports
  pagination/filtering server-side — extend it to "since my last acknowledged event id")
  so a player returning after being away sees exactly what changed, including other
  players' visible actions (Pillar 1, Pillar 4).
- **Platform caveat to research before committing:** `macroquad-toolkit` has no
  networking module at all, and neither does anything else in this workspace except
  `kaiju_server`'s server-side `reqwest`. A native build can use `reqwest` directly; the
  WebGL/WASM build cannot use a normal socket-based HTTP client and needs a
  browser-`fetch`-backed crate (e.g. `quad-net`, `ehttp`, or a `wasm-bindgen`/`web-sys`
  fetch wrapper) — this needs a spike before M1 (§14), since both native and WebGL are
  required targets for every game in this catalog and this is the first one where that
  actually matters for gameplay function, not just rendering.

### 7.5 Concurrency & fairness

- Per-player Favor spending only needs a per-player row lock (a much smaller problem than
  general shared-mutable-world concurrency), since regions/heroes/divine-tools are
  advanced only by the server's own tick process, not by concurrent player writes racing
  each other directly.
- **Anti-abuse, framed as gameplay (Pillar 4), not just moderation:** a per-region,
  per-tick cap on total nudge magnitude (from any single player or the sum of several)
  prevents "whoever has the most favor always wins" while still allowing legitimate
  large pushes to matter. Every nudge and bet is attributed in the public event log —
  visible manipulation is a feature (other deities can see it coming and counter-nudge or
  bet against it), not a hidden exploit.

### 7.6 Hosting & operations (flagged, not resolved)

Unlike every other game in this catalog, this one requires an always-on, maintained
server for as long as it's playable — a genuine ongoing operational commitment (hosting
cost, uptime, moderation of a live shared world, database backups) rather than "ship a
binary to itch.io/Steam and you're done." Who owns this is an open question (§13), not
something this document can resolve on its own.

### 7.7 Visibility & authorization are server-authoritative

The Standing system (§5.9) must be enforced on the server, never by the client merely
hiding screens:

- **Per-player projection** — the server computes each player's `WorldView` from their
  unlocked `VisibilityScope`s, so an un-revealed entity is *absent from the payload*, not
  just hidden. A low-tier Watcher receives a small view; the full world is never shipped to
  a client that has not earned it (a pleasant performance side-effect). This is the `view`
  DTO of §12, now filtered per player.
- **Action authorization** — every mutating command is checked against the player's
  `ActionVerb`/`BettingMarket` flags *and* the §7.5 per-region/per-tick nudge cap before it
  touches the world. An unauthorized `ShapeWeather` is a rejection, not a silent no-op.

The client mirrors enabled/disabled affordances for UX, but the server is the sole
authority (§7.1, §5.8).

---

## 8. World & Progression Structure

- **World layout:** no rendered map/tilemap — a region list/grid, same presentation
  style as every screen in this game (§10).
- **Session/world length:** the world is genuinely persistent — it does not reset per
  player, only at era transitions (§5.7), which are a built-in long-run pacing mechanic
  rather than something the design needs to invent. A player's own "progression" is their
  Favor balance, champion roster, bet history, and standing — not the world's state,
  which they never fully own.
- **Save/persistence model:** there is no local save file for world state — the server's
  database *is* the save, continuously. The Rust client persists only an auth token and a
  small local UI-preference cache, not game state — a fundamentally different use than
  every other port in this catalog.

> **Status update — persistence is built (`mytherra-persistence`, MySQL `mytherra_rust`).**
> The store bootstraps the world on startup and write-throughs after every guest mint,
> action, and tick, so a server restart resumes the same world (guest sessions included —
> the guest-id counter persists). Two dissociated domains mirror §6's shared/per-player
> split: the **world** is decomposed by entity (each `WorldState` collection — `regions`,
> `heroes`, `settlements`, … — is its own table, one row per entity, with the scalars, era,
> chronicle, and RNG in a single `world_core` row), and the **player** domain is relational
> (economy columns on `players`, `player_champions`/`player_bets` child tables, and a
> `player_registry` holding the guest-id counter). Writes are tracked per entity, so an
> unchanged row is not rewritten. This is the pragmatic realization of §6's relational
> target: entity-per-row with a JSON document per entity, rather than a full column-per-field
> schema. Credentials come from `mytherra-server/.env`; the DB is created and migrated on
> first run.

---

## 9. Content Inventory

| Content type | Original seed count | Prototype target | Full target |
| --- | ---: | ---: | ---: |
| Regions | 3 | 3 | 8–12 |
| Settlements | 3 | 5 | 20+ |
| Heroes | 3 | 6 | 30+ |
| Landmarks | 4 | 6 | 20+ |
| Resource nodes | 7 | 10 | 40+ |
| Bet types (minus `war_outcome`) | 14 (defined, unseeded) | 14, real seed data | 18–20 |
| Confidence levels / timeframe modifiers | 4 / 5 (defined, unseeded) | real seed data, as original | as original |
| Hero roles | 4 | 4 | 6–8 |
| Settlement/building/landmark/resource-node type tables | 7 / 2 / 5 / 7 | as original | roughly doubled — building_types (2) is especially thin |
| Starter artifacts | 4 | 4 | 6–8 |
| Weather patterns / magic paths / pantheon deities | 5 / 5 / 4 | as original | as original (pantheon deity count is a fixed cast by design) |
| Era-generation name banks | small fixed 4-word cycles | expanded pool | large pool — this is the clearest content-thinness gap flagged in research (§0) |

The core gap to close isn't "the mechanics are shallow" (they mostly aren't, unusually
for this catalog) — it's that **the betting-config tables ship completely empty** in the
original and the world bootstrap content (3 regions/settlements/heroes) is sized for a
demo, not an ongoing shared world meant to host many concurrent players.

---

## 10. UI/UX & Screen Flow

The original's 17 web pages consolidate into fewer top-level screens for a legible
macroquad UI, folding the seven divine tools into one tabbed screen rather than seven
separate top-level destinations:

| Screen | Purpose | Toolkit pieces |
| --- | --- | --- |
| Login/Guest Entry | Guest session or account link | `VirtualUi`, buttons |
| Dashboard | Status, last-tick summary, era panels, stat charts | `GridLayout`, meters, simple bar/line rendering |
| Event Log | Timeline since last visit, filters, other players' visible actions | `ScrollTabs`, `TextStyle` |
| World Map / Regions | Region list, drill-down detail, Bless/Corrupt/Guide actions | `GridLayout`, meters, badges |
| Heroes & Champions | Roster, influence actions, champion cultivation/rivalry | `ScrollTabs`, meters |
| Divine Tools | Tabbed: Artifacts / Weather / Omens / Magic / Myths / Civilization / Pantheon | `ScrollTabs`, `GridLayout`, `TextStyle` |
| Divine Observatory (Betting) | Speculation events, odds (house + crowd-lean), stakes, active/resolved bets | `GridLayout`, meters, tooltips |
| Eras | Era pressure, legacy/chronicle, transition history | `ScrollTabs`, `TextStyle` |
| Settings | Autosave of local prefs, audio, account linking | `VirtualUi` |

Interaction flow mirrors the original's own "session rhythm" (§3), now with an explicit
step for surfacing other players' actions rather than treating the world as a solo view.

---

## 11. Toolkit Mapping

| Need | Toolkit module | Using it? | Notes |
| --- | --- | --- | --- |
| Input handling | `input` | Yes | Buttons/panels only |
| Widgets/layout/text | `ui` (`VirtualUi`, `GridLayout`, `SurfaceStyle`, `TextStyle`, meters, badges, tabs, scroll) | Yes | Carries the whole presentation layer, same as the other two ports |
| Textures/manifest | `assets` (`AssetManager`) | No | No art (§0) |
| Camera/pan/zoom | `camera` | No | No rendered world |
| Cross-system messaging | `events` (`EventBus<UiAction>`) | Yes | Local UI intents only — world events come from the server, not this bus |
| Palette | `colors` | Yes | Region status/divine-tool color coding |
| Frame timing | `timing` | Yes | Frame loop + client-side polling interval timing |
| User settings | `settings` | Yes | Polling interval, audio, account link state |
| Dev overlay | `debug` | Yes | Standard |
| Deterministic randomness | `rng` | No (server-side only) | The client never rolls outcomes (§5.8, §7.1) — this is the one port where `rng` belongs entirely on the server side of the architecture, not the client |
| Save/load | `persistence` | Yes, but narrow | Only for local auth token + UI prefs (§8) — not game state, unlike every other port |
| Networking (HTTP client, cross-platform native+WASM) | *(none — project gap)* | Yes, project-local (`src/net.rs`) | **Resolved** with `quad-net 0.1.2`: one poll-based (`Pending<T>`) API for both native (background thread) and wasm (macroquad's own `sapp-jsutils` JS interop, so it coexists with the WebGL build). Requests carry a timeout, and the client tracks link state (Connecting/Live/Reconnecting) so a dropped server is visible and reconnected (§7.4). A candidate future toolkit addition now the pattern is proven. |
| Sprite/raster/FlatGrid/pathing | — | No | No spatial world (§8) |

This game's toolkit footprint is the leanest on rendering of any port so far, but the
**first to need something the toolkit doesn't have at all** — networking — which is the
one genuinely new engineering investment this project asks of the catalog.

---

## 12. Architecture Skeleton

A **five-crate Cargo workspace** (§7.2): `mytherra-core` (sim) → `mytherra-protocol`
(shared wire types) → `mytherra-persistence` (MySQL storage) → `mytherra-server` (axum
authority) + `mytherra` (the root package: the macroquad client/renderer). The crates are
flat directories at the repo root (`mytherra-core/`, `mytherra-protocol/`, …), not nested
under a `crates/` folder; the client is the root package's `src/`.

**`mytherra`** (the root package — the macroquad client binary). Realized `src/` layout:

```
src/
├── main.rs
├── game.rs               # Game struct, update()/draw() loop, GameState
├── game/
│   ├── online.rs         # the online session: /session handshake, /view + /events polling,
│   │                     #   link state (Connecting/Live/Reconnecting) + reconnect (§7.4)
│   ├── command.rs        # the apply_action split: UiAction → PlayerAction, submit()
│   ├── capture.rs        # headless screenshot fixture (embeds core; not a play mode)
│   └── achievements.rs
├── net.rs                # cross-platform HTTP over quad-net; Pending<T> + poll_timed (§7.4)
├── ui.rs
└── ui/                   # dashboard, regions, heroes, divine_tools, betting, eras,
                          #   chronicle, settings, title, shell (header/nav), widgets
```

The client renders from a received (or, for capture, locally-projected) `WorldView`; there
is no `view.rs` DTO layer or `local_prefs.rs` in the realized tree, and no `Menu`/save-slot
concept (there is no local world save, §8). Auth-token/UI-pref persistence is a later slice.

**`mytherra-core`** (`mytherra-core/` — lib): `world/`, `sim/`, `data/` and serialization.
Exposes `tick_world`/`tick_shared`, `WorldState`, `PlayerState`, `GameData`, the `command`
(apply/authorize) and `capability` (Standing/tier) modules. No macroquad, no I/O; compiles
to wasm. The server and persistence crates depend on it, as does the client's capture fixture.

**`mytherra-protocol`** (`mytherra-protocol/` — lib): `WorldView`/`PlayerView` (the
per-player projections of §7.7) and `project()`, `SessionResponse`, `ClientView`/`EventsDelta`
wire types. Re-exports core's `PlayerAction`/`Standing`. No I/O — pure types depended on by
both server and client.

**`mytherra-persistence`** (`mytherra-persistence/` — lib): all MySQL storage via `sqlx`
plus its own `migrations/`. `Store::connect(&DbConfig)` builds two dissociated stores — a
`WorldStore` (the shared world, decomposed into per-entity tables with per-row change
tracking) and a `PlayerStore` (the relational per-deity domain: economy columns, champion/bet
child tables, the guest-id registry). Depends on core but sits outside it, so core never
links a database (§6/§8). Configuration is the caller's concern — it takes a `DbConfig`, not
env-var names.

**`mytherra-server`** (`mytherra-server/` — bin): `axum`/`tokio`. Holds the
authoritative `WorldState` + one `PlayerState` per connected guest, runs the tick loop (§7.1),
authorizes/projects per player (§7.7), and bootstraps-from / write-throughs to the
persistence store (§6/§8). Reads its `DbConfig` from a local `.env`. Endpoints:
`POST /session`, `GET /view` (filtered), `POST /action`, `GET /events?since=` (§7.4), `GET /health`.

- **Client `GameState` variants:** `Login`, `Gameplay` (internal screen enum per §10) — no
  `Menu`-owned save-slot concept the way single-player ports have one, since there's no
  local world save to select (§8).
- **The client owns no simulation:** everything in `net.rs` + the online session is
  fetch-state / render / submit-action against the server (§7.1). Real play is online-only —
  the client requires a running server. The one remaining local-world use is the headless
  **screenshot capture fixture**, which embeds `mytherra-core` and projects a throwaway world
  through the *same* projection code the server uses — so the UI renders identically whether
  the `WorldView` is fetched (play) or projected locally (capture); it is not a play mode.
- **The `apply_action` split:** today's single `Game::apply_action` match mixes
  client-local UI-state changes (`Select*`, `Set*Page/Filter`, `Cycle*`, `TogglePause`)
  with authoritative world/player mutations. Splitting it along the "touches world/player"
  line yields the client↔server command boundary almost mechanically — the local variants
  stay in the client and never round-trip; the rest become `PlayerAction`s.

---

## 13. Non-Goals / Open Questions

- **Explicitly not building (v1):** real-time PvP combat, live chat/voice, a full
  liquidity-pool pari-mutuel betting engine (v1 ships the hybrid crowd-lean adjustment in
  §5.5, not a real order book), WebSocket/real-time push transport. *(The Rust server that
  earlier drafts listed here as a non-goal is now the recommended path — see §7.2.)*
- **Open questions**, in priority order:

  1. **Tick cadence.** The original's ~1-real-minute-per-year pace looks like a dev
     artifact, not a deliberate design choice for a live shared world. What real-world
     cadence (once an hour? a few times a day? once a day, "play by mail" style?) best
     fits "check in periodically, the world kept moving without you"? This needs
     playtesting, not a guess baked into the doc.
  2. **Guild/pantheon-faction system.** The original's dead `guild_id` column hints at an
     unbuilt feature that fits this port's multiplayer premise unusually well — should
     players be able to form factions that pool favor or coordinate nudges/bets? Worth a
     dedicated design pass once the core loop (§3) is proven, not a v1 requirement.
  3. **Hosting/ops ownership** (§7.6) — who runs and maintains the always-on server long
     term, and what happens to the persistent world if it needs to go down for
     maintenance or eventually be retired? This is a product decision, not a design one,
     but it gates whether this game can actually ship as scoped.
  4. **Crowd-lean tuning** (§5.5) — the `clamp(0.6, 1.5, ...)` bounds are placeholders;
     real tuning needs actual concurrent-player betting data, which doesn't exist until
     multiplayer is live — a genuine chicken-and-egg to solve with a small closed
     playtest before wide balance claims are made.
  5. **Standing/tier curve** (§5.9) — the four reference tiers and their unlock triggers
     (level vs. achievement vs. witnessed-event vs. favor-purchase) are a starting point,
     not a tuned progression. How fast should a new deity ascend, and which capabilities
     belong at which tier? Best settled by playtesting M0.5 in single-player, where the
     whole system runs without the server.

---

## 14. Milestones

> **Status (2026-07): M0, M0.5, and M1 complete; M2 substantially delivered.** Built:
> the five-crate workspace with `mytherra-persistence` (DB-is-save, world/player
> decomposition, per-entity change tracking); concurrent guest sessions with independent
> favor; heroes/champions and betting with real seeded config tables + crowd-lean, now
> including hero lifespan and region-defection wagers; `/events` since-cursor deltas; a
> forward-compatible wire protocol; and client connection resilience (offline detection +
> reconnect). A `run-server.ps1` + `RUNNING.md` stand the whole thing up on one desktop.
> **M2's remaining piece is a real playtest** — a hosted/reachable server and in-browser
> confirmation of the deployed fetch/reconnect flow — plus crowd-lean tuning (§13.4), which
> is chicken-and-egg with that playtest. Everything else labeled below for M3 (relational
> tool tables, account linking, rate-limiting/anti-grief) is still ahead.

| Milestone | Proves | Target content |
| --- | --- | --- |
| M0 — Crate extraction | `mytherra-core` split out as a headless lib (determinism/save tests green inside it); `mytherra-protocol` defines `WorldView`/`PlayerView`/`PlayerAction`; the client renders from a locally-built `WorldView` and the `apply_action` split (§12) is done. Pure refactor — the game still runs exactly as today | existing content |
| M0.5 — Tiered revelation (single-player) | The §5.9 Standing system live in offline mode: capability flags on the player, per-player projection + action-gating enforced through the projection code, the four reference tiers shipped as `tiers.json`, unlocks firing off level/achievements/witnessed-events. Delivers the visibility gameplay *before* any networking | existing content + `tiers.json` |
| M1 — Mechanical proof | Networking spike resolved (native + WASM); `mytherra-server` (axum) hosts the shared world and runs the tick loop; client logs in (guest session), fetches its filtered `WorldView`, renders regions/events, and submits a Bless/Corrupt/Guide action end-to-end with per-player favor and server-side authorization | 3 regions (as original), guest auth only |
| M2 — Playable prototype | Full core loop: heroes/champions, betting with real seeded config tables and the crowd-lean adjustment live against at least a small pool of concurrent testers, event log delta/since-cursor working | Prototype content targets from §9 |
| M3 — Content-complete | All seven divine tools on real relational tables, era system, expanded world content, account linking, rate-limiting/anti-grief in place | Full targets from §9 |

Because this game requires a live server, "verify at the shared preview root" (this
catalog's usual `.\publish.ps1` validation path) covers the **client** build only. The
server is Rust (`mytherra-server` + `mytherra-persistence`), so its validation is the same
workspace loop — `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo test` (determinism/logic in `mytherra-core`; live client↔server round-trips under
`cargo test -p mytherra -- --ignored net` against a running server) — plus a
`run-server.ps1` smoke run confirming the DB migrates and the world resumes.
