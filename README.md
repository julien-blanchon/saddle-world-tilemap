# Saddle World Tilemap

Chunk-based square, isometric, and hex tilemap foundation for Bevy.

The crate keeps logical tile storage separate from render chunks and collision output. It is meant to stay generic enough for authored levels, procedural maps, runtime edits, multiple layers, and large scrolling worlds without tying the API to a specific editor or physics backend.

## Why this crate exists

`saddle-world-tilemap` is meant to cover the durable shared-crate layer for:

- authored 2D levels
- procedural world generation
- runtime tile edits and patching
- multi-layer maps with logic-only collision layers
- square, isometric, and hex coordinate conversion
- autotiled terrain or road networks
- animated tiles driven by shared tile definitions
- Tiled JSON import with object-layer extraction

The implementation studies ideas from Bevy's built-in tilemap chunk examples, `bevy_ecs_tilemap`, Tiled infinite maps, and LDtk auto-layers, but owns the runtime model directly.

## Quick start

```toml
[dependencies]
saddle-world-tilemap = { git = "https://github.com/julien-blanchon/saddle-world-tilemap" }
```

```rust
use bevy::prelude::*;
use saddle_world_tilemap::{
    TileAtlasLayout, TileCatalog, TileCell, TileKind, TileKindId, TileLayerConfig,
    TileLayerId, TileLayerRenderConfig, TileLayerState, Tilemap, TilemapBundle,
    TilemapGeometry, TilemapPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TilemapPlugin::always_on(Update))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load("tiles/demo_atlas.png");
    let atlas = TileAtlasLayout::from_grid(
        image,
        UVec2::new(256, 256),
        UVec2::splat(16),
        16,
        16,
    );

    let mut catalog = TileCatalog::default();
    catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("grass", 0));

    let mut map = Tilemap::new(TilemapGeometry::square(Vec2::splat(16.0)), UVec2::splat(8));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            TileLayerId::new(1),
            "Ground",
            TileLayerRenderConfig::new(atlas),
        ),
        catalog,
    ));
    map.set_tile(TileLayerId::new(1), saddle_world_tilemap::TileCoord::new(0, 0), TileCell::new(TileKindId::new(1)));

    commands.spawn(TilemapBundle::new("Example Map", map));
}
```

## Public API

### Plugin and scheduling

- `TilemapPlugin`
- `TilemapSystems::{Prepare, ApplyCommands, AdvanceAnimation, ResolveAutotiling, SyncCollision, SyncRender, Debug}`
- injectable schedules through `TilemapPlugin::new(...)`
- convenience constructors: `TilemapPlugin::always_on(Update)` and `Default`

### Runtime-facing components and messages

- `TilemapBundle`
- `TilemapRoot`
- `TilemapLayerNode`
- `TilemapRenderChunk`
- `TilemapCollisionChunk`
- `TilemapDiagnostics`
- `TilemapCommand`
  - `SetTile`
  - `ClearTile`
  - `FillRect`
  - `SwapTiles`
  - `SetLayerVisibility`
- `TileChanged`
- `ChunkRebuilt`
- `LayerVisibilityChanged`
- `TileAnimationLooped`

### Coordinate and geometry helpers

- `TileCoord`
- `ChunkCoord`
- `TileRect`
- `TilemapGeometry`
- `TilemapOrientation::{Square, IsometricDiamond, HexPointyColumns, HexFlatRows}`
- `TilemapHexParity`
- `TileRowDirection`

`TilemapGeometry` exposes the main conversion helpers:

- `tile_to_local`
- `local_to_tile`
- `tile_to_world`
- `world_to_tile`
- `cursor_to_tile`
- `chunk_bounds_local`

### Tile and layer model

- `Tilemap`
- `TileChunk`
- `TileLayerState`
- `TileLayerConfig`
- `TileLayerRenderConfig`
- `TileCatalog`
- `TileKind`
- `TileKindId`
- `TileCell`
- `TileOrientation`
- `TileAtlasLayout`

### Autotiling, animation, and collision metadata

- `AutotileBinding`
- `AutotileGroupId`
- `AutotileRuleSetId`
- `AutotileNeighborhood`
- `AutotileRuleSet`
- `compute_autotile_mask`
- `TileAnimation`
- `TileAnimationFrame`
- `TileCollisionDescriptor`
- `TileCollisionShape`
- `TileCollisionCell`

### Tiled import

- `import_tiled_json_str`
- `TiledImportOptions`
- `ImportedTilemapScene`
- `TileObjectSpawn`
- `TilePropertyValue`
- `TiledImportError`

## Coordinate systems

Shipped in `0.1.0`:

- square grids
- isometric diamond grids
- pointy-top hex grids stored as staggered columns
- flat-top hex grids stored as staggered rows

Deferred on purpose:

- sparse storage backends
- streaming activation windows
- LDtk parsing
- direct scene/entity spawning from editor data

All geometry modes share the same logical map model. Only coordinate conversion, chunk mesh projection, and depth ordering change.

## Runtime model

The important separation is:

- `Tilemap` and `TileLayerState` own the logical tiles
- render chunks are generated projections of the logical state
- collision chunks are generated descriptors, not physics objects
- autotile resolution and animation happen against tile definitions, not ad-hoc entities

Tile edits mark dirty chunks. The runtime then resolves only the affected chunks and rebuilds only the render and collision surfaces that changed. `TilemapDiagnostics::dirty_chunks` reports distinct dirty chunk coordinates across resolve, render, and collision phases instead of double-counting the same chunk in multiple queues.

## Examples

| Example | Focus |
| --- | --- |
| `basic` | square authored map, hover picking, and chunk diagnostics |
| `autotiling` | incremental road growth with local autotile recomputation |
| `runtime_editing` | message-driven fill, set, clear, and collision-only edits |
| `animated_tiles` | definition-driven animated water and rebuild counters |
| `layered_map` | layer visibility toggles over ground, detail, and logic-only layers |
| `isometric` | isometric world-to-tile picking and movement-cost metadata |
| `hex_strategy` | hex board rendering through tilemap plus `saddle-world-hex-grid` pathfinding |
| `saddle-world-tilemap-lab` | crate-local BRP/E2E lab covering smoke, runtime edits, isometric picks, large-map sweeps, and manual debug controls |

Every shipped example now includes a live `saddle-pane` control surface for debug toggles and the most useful layout parameters.

Run them with:

```bash
cargo run -p saddle-world-tilemap-example-basic
cargo run -p saddle-world-tilemap-example-autotiling
cargo run -p saddle-world-tilemap-example-runtime-editing
cargo run -p saddle-world-tilemap-example-animated-tiles
cargo run -p saddle-world-tilemap-example-layered-map
cargo run -p saddle-world-tilemap-example-isometric
cargo run -p saddle-world-tilemap-example-hex-strategy
cargo run -p saddle-world-tilemap-lab
```

## Dependency philosophy

Runtime dependency surface stays minimal:

- `bevy = "0.18"`
- `serde`
- `serde_json`

The crate still does not depend on `game_core`, Avian, LDtk, or `bevy_ecs_tilemap`. Physics integration and editor-specific entity spawning stay adapter concerns outside the runtime crate, while Tiled JSON translation is now supported directly through a normalized import API.
