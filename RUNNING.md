# Running Mytherra locally

Mytherra is online-only: the client renders and submits actions, but the **one
shared world lives in the authority server** (`mytherra-server`), which persists
it to MySQL (GDD 6/8). To play or test you run the server, then point one or
more clients at it. This guide covers the minimal single-desktop setup — server,
database, and every client on one machine.

## Prerequisites

- **XAMPP MySQL running.** The server creates the `mytherra_rust` database and
  its schema automatically on first start. Credentials live in
  `mytherra-server/.env` (copy `mytherra-server/.env.example` if it is missing).
- **XAMPP Apache running** only if you want to play in a browser (it serves the
  WebGL client). Native clients don't need it.
- A Rust toolchain (the `wasm32-unknown-unknown` target too, for the WebGL client).

## 1. Start the server

```powershell
.\run-server.ps1
```

It builds `mytherra-server` (release), connects to MySQL, and starts ticking the
shared world on `http://127.0.0.1:8791` (from `assets/data/game_config.json`).
Leave it running. `Ctrl+C` stops it; the world is saved and resumes on the next
start.

## 2. Connect clients

Every client points at the server's address (baked into the client from
`game_config.json` → `server_url`). Any mix of these works at once, and each is
its own guest deity sharing the one world:

- **Browser (WebGL):** deploy the client once with
  ```powershell
  .\publish.ps1 -WebGLOnly
  ```
  then open **http://127.0.0.1/games/mytherra/** and click *Enter the World*.
  Open several tabs/windows for several concurrent deities.
- **Native window:**
  ```powershell
  cargo run -p mytherra
  ```

> The browser client is served from `http://127.0.0.1` (port 80) but calls the
> server on port 8791 — a different origin — so the server sends permissive CORS
> to allow it. That's a dev default; a hosted deployment would narrow it (§7.6).

## Notes

- **The DB is the save.** Restarting the server resumes the same world; there is
  no local save file. To start the world over, drop the `mytherra_rust` database
  and restart the server (it recreates it fresh).
- **Beyond one desktop (later):** to reach the server from other devices on your
  LAN, change `server_listen_addr` to `0.0.0.0:8791` and `server_url` to the
  desktop's LAN IP in `game_config.json`, republish the client, and allow the
  port through the firewall. Not needed for single-desktop testing.
