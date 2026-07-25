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

## Publish the client, keep the server at home (via the gateway)

To let anyone play the **published** client (webhatchery.au) against a server on
your desktop — without exposing your IP, and without the browser mixed-content
block — route everything through the `local_gateway` reverse proxy
(`apps/local_gateway`). Clients only ever talk to `https://webhatchery.au`, and
only the gateway talks to your box.

```
client ──https──▶ webhatchery.au/local_gateway/api/p/mytherra/* ──http──▶ your desktop:8791
```

The client already knows to use the gateway: `game_config.json` carries
`gateway_url`, and a **published** build (WebGL, or a release native binary)
targets it automatically, while a local `cargo run` (debug) still talks straight
to `server_url`. Override on native anytime with `MYTHERRA_SERVER_URL`.

One-time setup:

1. **Deploy the gateway.** Set a strong `GATEWAY_ADMIN_TOKEN` in
   `apps/local_gateway/backend/.env.production`, then publish it to production.
2. **Port-forward** `8791` on your router to this desktop (ideally firewalled to
   accept only webhatchery.au), and bind the server wide: set
   `server_listen_addr` to `0.0.0.0:8791` in `game_config.json`.
3. **Publish the mytherra client** to production (`.\publish.ps1 -Production`).

Each session:

1. Start the server: `.\run-server.ps1`.
2. Advertise it to the gateway on a heartbeat:
   ```powershell
   $env:GATEWAY_ADMIN_TOKEN = '<the token>'
   .\register-server.ps1            # registers http://<your public IP>:8791
   ```
   Leave it running; it re-registers every 60 s so the gateway record never goes
   stale. Then open the published client — it connects through the gateway.

> **Browser over a tunnel instead of port-forwarding.** If you'd rather not open a
> port, run an https tunnel (e.g. `cloudflared tunnel --url http://localhost:8791`)
> and register its URL: `.\register-server.ps1 -Target https://<id>.trycloudflare.com`.
> That also gives the browser client the https origin it needs.

## Notes

- **The DB is the save.** Restarting the server resumes the same world; there is
  no local save file. To start the world over, drop the `mytherra_rust` database
  and restart the server (it recreates it fresh).
- **LAN-only, no gateway:** to reach the server from other devices on your LAN
  without the gateway, set `server_listen_addr` to `0.0.0.0:8791`, point a native
  build at it with `MYTHERRA_SERVER_URL=http://<desktop LAN IP>:8791`, and allow
  the port through the firewall.
- **Offline local browser dev:** a local WebGL build still targets `gateway_url`
  (wasm can't read env). For fully offline browser testing, temporarily blank
  `gateway_url` in `game_config.json`, or just use the native `cargo run` client.
