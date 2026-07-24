# Mytherra

You are a minor god among many, watching one living, shared world that keeps
turning whether or not you are present. Nudge mortals toward glory or ruin, raise
legendary champions, wager on the fates in the Divine Observatory — and read what
changed since you last looked in. Unusually for this catalog, Mytherra is a
**persistent, shared-world multiplayer game**: the world exists on an authority
server, not in any one player's save file.

See **[`gdd.md`](gdd.md)** for the full design and **[`RUNNING.md`](RUNNING.md)**
for how to stand it up and play locally.

## What it is (and isn't)

- **Online-only.** The client renders a server-sent, Standing-filtered view of the
  world and submits actions; it runs no simulation of its own. Real play requires
  a running `mytherra-server`. There is no offline/local-world mode — the one
  local-world use left is the headless screenshot-capture fixture (`src/game/capture.rs`),
  which is not a play mode.
- **The database is the save.** The server owns the world and persists it to MySQL
  continuously (GDD §6/§8); a server restart resumes the same world. The client
  keeps only an auth token and UI preferences locally, never game state.
- **No rendered map or art** — a legible `VirtualUi` screen set (dashboard, regions,
  heroes, divine tools, the betting Observatory, eras), same presentation style as
  the rest of the catalog.

## Workspace layout

Mytherra is one game in a five-crate Cargo workspace (the root `Cargo.toml`):

| Crate | Kind | Role |
| --- | --- | --- |
| `mytherra` (root) | bin | the macroquad **client** — `src/` (`game.rs`, `net.rs`, `ui/`, …); online-only |
| `mytherra-core` | lib | the pure, deterministic **simulation** — `world/`, `sim/`, `data/`, serialization. No macroquad, no I/O. Compiles to wasm (the client embeds it for the capture fixture) |
| `mytherra-protocol` | lib | the shared **wire types** both ends compile against — `WorldView`/`PlayerView`, `PlayerAction`, the §5.9 Standing/tier model, event-delta types |
| `mytherra-persistence` | lib | all **storage** — sqlx/MySQL, migrations. Depends on core but sits outside it, so core never links a database. Two dissociated stores: the shared world and the per-deity player domain |
| `mytherra-server` | bin | the **authority** — `axum`/`tokio`; owns the world, runs the tick loop, authorizes and projects per player (§7.7), and write-throughs to the store |

## Run

The full local setup (server + MySQL + one or more clients) is in
**[`RUNNING.md`](RUNNING.md)**. In short:

```powershell
.\run-server.ps1          # build + run the authority server against local MySQL
.\publish.ps1 -WebGLOnly  # deploy the WebGL client → http://127.0.0.1/games/mytherra/
cargo run -p mytherra     # or run a native client window
```

Each connected client is its own guest deity sharing the one persistent world.

## Build & test

```powershell
cargo build                        # whole workspace (client + libs + server)
cargo test -p mytherra-core        # simulation determinism + logic (the bulk of the suite)
cargo build --release --target wasm32-unknown-unknown   # the WebGL client
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt -- --check
```

The live client↔server round-trip tests need a running server and are `#[ignore]`d:

```powershell
cargo test -p mytherra -- --ignored net
```

## Other docs

- `gdd.md` — design document (pillars, systems, multiplayer architecture, milestones).
- `RUNNING.md` — local hosting / how to play.
- `AGENTS.md`, `CODE_STANDARDS.md`, `GAME_DEVELOPMENT_GUIDE.md`, `MACROQUAD_TOOLKIT.md`
  — shared RustGames standards (synced from `rust_management/docs`; do not hand-edit).
