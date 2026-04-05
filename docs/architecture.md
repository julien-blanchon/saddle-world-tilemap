# `saddle-world-tilemap` Architecture

## Design goals

`saddle-world-tilemap` is built around four rules:

1. logical tile storage is the source of truth
2. render chunks and collision chunks are projections of that logical state
3. runtime edits stay localized through dirty-chunk tracking
4. coordinate conversion remains explicit and testable

## Main runtime split

### Logical model

The public logical model is:

- `Tilemap`
- `TileLayerState`
- `TileChunk`
- `TileCatalog`
- `TileKind`
- `TileCell`

`Tilemap` owns:

- map geometry
- chunk size
- stable layer IDs
- the per-layer chunk store

`TileLayerState` owns:

- the layer config
- the catalog for that layer
- dense chunk-local tile storage
- dirty sets for resolve, render, and collision

The logical model never stores render entities directly.

### Runtime projection state

Internal runtime state lives in `TilemapRuntimeComponent` and tracks:

- spawned layer nodes
- spawned render chunk entities
- spawned collision chunk entities
- animation playback state per animated tile kind

This runtime cache is rebuildable. Deactivation clears it and leaves the logical tile state intact.

## System pipeline

`TilemapSystems` describes the intended frame order:

1. `Prepare`
2. `ApplyCommands`
3. `AdvanceAnimation`
4. `ResolveAutotiling`
5. `SyncCollision`
6. `SyncRender`
7. `Debug`

### `Prepare`

- ensures every map root has runtime state
- spawns layer nodes lazily
- initializes diagnostics
- marks the full map dirty on first activation

### `ApplyCommands`

- consumes `TilemapCommand`
- mutates logical tiles only
- emits `TileChanged` and `LayerVisibilityChanged`
- marks the edited chunk, and neighboring chunks when autotile groups are involved

### `AdvanceAnimation`

- advances per-kind animation clocks
- marks only chunks containing the animated kind dirty when the visible frame changes
- emits `TileAnimationLooped` when a clip wraps

Animation is definition-driven. The system never creates one timer per tile entity.

### `ResolveAutotiling`

- walks dirty logical chunks
- resolves each tile into:
  - final atlas index
  - tint/orientation
  - collision descriptor
  - animated-kind membership
- increments the chunk revision
- forwards the chunk to render and collision sync stages

Autotiling uses reusable rule sets keyed by `AutotileRuleSetId`. The core implementation currently ships bitmask-style matching for cardinal or Moore neighborhoods.

### `SyncCollision`

- rebuilds `TilemapCollisionChunk` components from resolved descriptors
- despawns empty collision chunks
- keeps collision output generic and backend-agnostic

The crate deliberately stops at descriptors. A physics integration layer can translate them into colliders later.

### `SyncRender`

- rebuilds one mesh per dirty chunk per visual layer
- keeps layer ordering and visibility on the layer node
- uses chunk-local mesh projection based on `TilemapGeometry`

The current backend is a simple internal chunk mesh builder using `Mesh2d` and `ColorMaterial`. The public crate surface does not expose the renderer internals so this can evolve later.

### `Debug`

- draws chunk bounds and dirty chunks through gizmos when `GizmoPlugin` is present
- stays optional and data-driven through `TilemapDebugSettings` and `TilemapDebugOverlay`

## Chunk lifecycle

1. logical edits create or mutate `TileChunk`
2. the layer marks the chunk dirty
3. resolution computes final visual and collision snapshots
4. sync stages spawn or update chunk entities
5. empty chunks despawn their render and collision projections

Chunk coordinates are stable through `ChunkCoord`. Tile-to-chunk math uses Euclidean division so negative coordinates remain coherent.

## Coordinate architecture

`TilemapGeometry` is the only place that knows how a logical tile maps into local or world space.

Current modes:

- `Square`
- `IsometricDiamond`
- `HexPointyColumns`
- `HexFlatRows`

The same public helpers are used for:

- tile placement
- cursor picking
- world-to-tile conversion
- chunk debug rectangles

This keeps authored content, procedural generation, and runtime picking on the same math path.

## Pathfinding

The crate includes two standalone pathfinding functions that operate directly on the logical tile model:

- `find_path` — A* search on a named tile layer, returning the optimal path as a `Vec<TileCoord>` with total accumulated cost
- `reachable_tiles` — Dijkstra flood from a starting tile, returning all tiles reachable within a cost budget

Both functions:

- read tile data from `TileLayerState` without spawning entities
- treat tiles with a `TileCollisionDescriptor` as impassable
- use `TileKind.movement_cost` as the per-tile cost
- support all orientation modes through `tile_neighbors()`, which dispatches to cardinal, 8-directional, or hex-6 neighborhoods based on `TilemapOrientation`

Empty tiles (no data) are passable with an implicit cost of 1.

### Design choice

Pathfinding operates on the logical model, not on render or collision projections. This means it stays usable even when rendering is disabled or collision chunks have not been synced yet.

## Fill helpers

The `Tilemap` struct provides three fill methods for common editing operations:

- `fill_circle` — fills all tiles within a Euclidean radius
- `fill_line` — Bresenham line rasterization between two tile coordinates
- `flood_fill` — BFS flood fill from a starting tile, replacing matching tiles up to a configurable safety limit

These are also exposed as `TilemapCommand` variants (`FillCircle`, `FillLine`, `FloodFill`) for message-driven usage through `MessageWriter<TilemapCommand>`.

## What is intentionally not in the core runtime

- sparse storage
- streaming activation or chunk culling
- physics backend bindings
- navmesh or visibility baking
- save/load formats

Tiled JSON translation now ships directly in the crate as a normalized import boundary. LDtk parsing and editor-specific scene/entity instantiation still stay outside the core runtime.

Those are useful adapter layers, but they are not part of the durable core runtime contract in `0.1.0`.
