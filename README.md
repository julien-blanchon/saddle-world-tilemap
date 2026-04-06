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
  - `FillCircle`
  - `FillLine`
  - `FloodFill`
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

### Pathfinding

- `find_path(map, layer_id, start, goal, options) -> Option<TilePathResult>` — A* search on a tile layer, respecting movement costs and collision
- `find_path_with_policy(map, layer_id, start, goal, options, policy) -> Option<TilePathResult>` — A* with injectable passability and cost policy
- `reachable_tiles(map, layer_id, start, max_cost, diagonal) -> BTreeMap<TileCoord, u32>` — Dijkstra flood returning all tiles reachable within a cost budget
- `reachable_tiles_with_policy(map, layer_id, start, max_cost, diagonal, policy) -> BTreeMap<TileCoord, u32>` — Dijkstra flood with injectable passability and cost policy
- `TilePathOptions` — configuration: `max_cost`, `diagonal`
- `TilePathResult` — result: `path: Vec<TileCoord>`, `total_cost: u32`
- `TilePathPolicy` — trait hook for custom traversal rules
- `TilePathStep` — per-edge query context passed into custom policies
- `TilePathCallbacks::new(passability, movement_cost)` — convenience adapter from closures
- `TileKindPathPolicy` — built-in policy used by `find_path` and `reachable_tiles`

Pathfinding supports all orientation modes (square, isometric, hex). `find_path` and `reachable_tiles` keep the existing built-in behavior through `TileKindPathPolicy`: the destination tile must exist on the queried layer, same-layer `TileCollisionDescriptor` blocks traversal, and `TileKind.movement_cost` supplies the per-step cost.

Use `*_with_policy` when locomotion depends on different rules, such as:

- reading a separate collision layer
- preferring roads from a detail layer
- agent-specific movement rules
- directional or contextual terrain costs

### Fill helpers

Direct methods on `Tilemap`:

- `fill_circle(layer_id, center, radius, tile)` — Euclidean radius fill
- `fill_line(layer_id, from, to, tile)` — Bresenham line rasterization
- `flood_fill(layer_id, start, tile, max_tiles) -> usize` — flood fill with safety limit

These are also available as `TilemapCommand` variants (`FillCircle`, `FillLine`, `FloodFill`) for message-driven usage.

### Coordinate helpers on `TileCoord`

- `cardinal_neighbors()` — 4 orthogonal neighbors
- `eight_neighbors()` — 8 including diagonals
- `hex_neighbors_pointy(parity)` — 6 hex neighbors (pointy-top stagger)
- `hex_neighbors_flat(parity)` — 6 hex neighbors (flat-top stagger)
- `manhattan_distance(other)` — L1 distance
- `chebyshev_distance(other)` — L∞ distance

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
| `rpg_village` | top-down RPG village with custom A* policy that avoids collision-layer walls and prefers detail-layer roads |
| `platformer` | side-scrolling platformer with gravity, collision layer, and platform jumping |
| `tile_painter` | runtime tile editor with pencil, line, circle, flood fill, and eraser brush modes |
| `roguelike_showcase` | P0 integration demo layering `saddle-procgen-dungeon-gen`, `saddle-ai-fov`, and `saddle-world-fog-of-war` onto a playable tilemap dungeon |
| `saddle-world-tilemap-lab` | crate-local BRP/E2E lab covering smoke, runtime edits, isometric picks, large-map sweeps, pathfinding, and manual debug controls |

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
cargo run -p saddle-world-tilemap-example-rpg-village
cargo run -p saddle-world-tilemap-example-platformer
cargo run -p saddle-world-tilemap-example-tile-painter
cargo run -p saddle-world-tilemap-example-roguelike-showcase
cargo run -p saddle-world-tilemap-lab
```

Smoke-check the integration showcase with:

```bash
cargo run -p saddle-world-tilemap-example-roguelike-showcase --features e2e -- roguelike_showcase_smoke
```

## Dependency philosophy

Runtime dependency surface stays minimal:

- `bevy = "0.18"`
- `serde`
- `serde_json`

The crate still does not depend on `game_core`, Avian, LDtk, or `bevy_ecs_tilemap`. Physics integration and editor-specific entity spawning stay adapter concerns outside the runtime crate, while Tiled JSON translation is now supported directly through a normalized import API.
