# `saddle-world-tilemap` Editor Integration

`saddle-world-tilemap` now ships a normalized Tiled JSON importer and still keeps LDtk as an adapter concern. The core data model remains the durable runtime boundary either way.

## Integration rule

Treat editor data as an import boundary, not as the runtime model.

The adapter's job is to translate editor concepts into:

- `Tilemap`
- `TileLayerState`
- `TileCatalog`
- `TileKind`
- `TileCell`
- `TileLayerId`
- `TileCollisionDescriptor`

Once that translation is done, gameplay code should only talk to the `saddle-world-tilemap` runtime types.

## Built-in Tiled path

Use `import_tiled_json_str(...)` when you want to translate a Tiled JSON asset directly into:

- `Tilemap`
- `TileLayerState`
- `TileObjectSpawn`

The caller still owns:

- tileset asset loading
- `gid -> TileKindId` mapping
- which imported layers are visual vs logic-only
- how object-layer spawns become actual entities or scenes

## Mapping Tiled concepts

### Infinite maps

Tiled infinite maps already think in chunks. A Tiled adapter can map each incoming chunk to:

- `ChunkCoord`
- dense tile data inside `TileChunk`

The runtime crate does not need to know whether a chunk came from Tiled, procedural generation, or a save file.

### Global tile IDs

Tiled GIDs should be resolved by the adapter into:

- a `TileKindId`
- optional `TileOrientation` derived from flip flags

The adapter should strip flip bits and own any tileset lookup tables. The runtime crate should only receive normalized tile IDs and orientations.

### Tile properties

Tiled custom properties can map into:

- `flags`
- `movement_cost`
- `TileCollisionDescriptor`
- adapter-owned side tables if the data is too editor-specific for the shared crate

Avoid pushing arbitrary editor schemas into the core runtime.

### Layer ordering

Tiled layer order should become stable `TileLayerId` values plus `z_index` values in `TileLayerRenderConfig`.

Use `logic_only(...)` for collision or metadata layers that should not render.

## Mapping LDtk concepts

### Auto-layers

LDtk auto-layers are a good mental model for `AutotileRuleSet`, but the runtime crate does not execute LDtk rules directly.

An LDtk adapter should translate:

- terrain or auto-layer output into final `TileCell` values
- reusable auto-layer semantics into `AutotileBinding` and `AutotileRuleSet` where that preserves value

### IntGrid and metadata layers

LDtk IntGrid layers are a good fit for:

- `flags`
- movement or traversal cost side tables
- logic-only layers

If the data is more expressive than a `u32` flag set, keep the richer structure in an adapter-owned asset and use `saddle-world-tilemap` only for the cross-cutting runtime pieces.

## Recommended workflow

1. Load or parse the external editor format into an adapter-owned asset.
2. Build a `TileCatalog` from the referenced tilesets.
3. Allocate stable `TileLayerId` values for each runtime layer.
4. Convert the editor coordinates into `TileCoord`.
5. Populate `TileLayerState` and `Tilemap`, or call `import_tiled_json_str(...)` for Tiled JSON.
6. Spawn the result through `TilemapBundle`.

This keeps editor concerns isolated from the runtime crate and makes it easier to support multiple import sources later.

## What should stay outside the core crate

- LDtk parsing
- tileset asset resolution policies
- entity or scene spawning policy
- editor-specific property schemas
- one-off content rules

Those are adapter concerns. The shared crate should stay focused on durable tile runtime behavior.
