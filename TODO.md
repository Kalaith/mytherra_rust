# TODO — Mytherra

## Protocol & content integrity

- Carry a content hash/version on `SessionResponse` and warn the client on mismatch. Both ends embed `assets/data/*.json`, and the index-based wire fields (`stake_index`, `confidence_index`, `pattern_index`) silently assume identical tables (GDD §7.6 territory).

## Server security

- Replace sequential `X-Player-Id` session ids (`guest-0`, `guest-1`, …) with random tokens — they are trivially spoofable today.
- Narrow `CorsLayer::permissive()` to the published client origins. Both this and the session ids are acknowledged M2-development choices and must change before any non-localhost deployment.

## Code health

- Five files sit over the 800-line hard cap and want a responsibility extracted into sibling modules: `mytherra-core/src/sim.rs` (1074), `sim/era.rs` (941), `command.rs` (940), `sim/myth.rs` (918), `sim/settlement.rs` (828).
