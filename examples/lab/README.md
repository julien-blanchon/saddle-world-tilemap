# `tilemap_lab`

Crate-local verification app for the shared `tilemap` crate.

## Purpose

- keep square runtime edits, isometric picking, and a large dense chunk sweep in one scene
- expose map diagnostics, chunk counts, edit stages, and movement-cost feedback through an overlay
- support manual keyboard exploration as well as BRP and E2E automation
- keep tilemap-specific verification inside the crate instead of pushing it into project sandboxes

## Status

Working

## Run

```bash
cargo run -p saddle-world-tilemap-lab
```

Controls:

- `1` / `2` / `3` / `4`: switch camera focus between overview, isometric board, and large-map sweep anchors
- `Q` / `E`: step the square runtime-edit showcase backward or forward
- `W` / `A` / `S` / `D`: move the isometric highlight tile
- `R`: reset the lab control state

## E2E scenarios

```bash
cargo run -p saddle-world-tilemap-lab --features e2e -- tilemap_smoke
cargo run -p saddle-world-tilemap-lab --features e2e -- tilemap_runtime_edit
cargo run -p saddle-world-tilemap-lab --features e2e -- tilemap_isometric_pick
cargo run -p saddle-world-tilemap-lab --features e2e -- tilemap_large_map
cargo run -p saddle-world-tilemap-lab --features e2e -- tilemap_custom_path_policy
```

## BRP

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch tilemap_lab
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/tilemap_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```
