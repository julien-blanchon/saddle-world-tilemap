# `saddle-world-tilemap` Configuration

## `TilemapPlugin`

Runtime integration surface for logical maps, chunk sync, animation, and debug drawing.

### Fields

- `activate_schedule: Interned<dyn ScheduleLabel>`
- `deactivate_schedule: Interned<dyn ScheduleLabel>`
- `update_schedule: Interned<dyn ScheduleLabel>`
- `debug_settings: TilemapDebugSettings`

### Constructors

- `TilemapPlugin::new(activate, deactivate, update)`
- `TilemapPlugin::always_on(update)`
- `TilemapPlugin::default()` which is equivalent to `always_on(Update)`
- `with_debug_settings(TilemapDebugSettings)`

### Guidance

- use `new(...)` for state-scoped gameplay or tool modes
- use `always_on(Update)` for examples, editors, or always-live world tooling
- if `GizmoPlugin` is not present, the debug draw system is skipped automatically

## `TilemapGeometry`

Controls tile placement, world conversion, and chunk bounds.

### Fields

- `orientation: TilemapOrientation`
- `grid_size: Vec2`
- `tile_render_size: Vec2`
- `origin: Vec2`
- `row_direction: TileRowDirection`

### Constructors

- `TilemapGeometry::square(tile_size)`
- `TilemapGeometry::isometric_diamond(tile_render_size)`
- `TilemapGeometry::hex_pointy_columns(tile_render_size, TilemapHexParity)`
- `TilemapGeometry::hex_flat_rows(tile_render_size, TilemapHexParity)`

### Orientation variants

- `TilemapOrientation::Square`
- `TilemapOrientation::IsometricDiamond`
- `TilemapOrientation::HexPointyColumns(TilemapHexParity)`
- `TilemapOrientation::HexFlatRows(TilemapHexParity)`

### `TilemapHexParity`

- `TilemapHexParity::Odd`
- `TilemapHexParity::Even`

### Builders

- `with_origin(Vec2)`
- `with_row_direction(TileRowDirection)`
- `with_tile_render_size(Vec2)`

### Guidance

- keep `grid_size.x > 0.0` and `grid_size.y > 0.0`
- for square maps, `grid_size` and `tile_render_size` usually match
- for isometric maps, `grid_size` should stay half-width and half-height of the rendered diamond unless you are intentionally stretching or overlapping tiles
- for pointy-column hex maps, `tile_render_size` should match the full rendered hex footprint and parity should match the source data layout
- for flat-row hex maps, `tile_render_size` should match the full rendered hex footprint and parity should match the source data layout
- `row_direction = Down` matches common 2D screen-space conventions

## `Tilemap`

Top-level logical map component.

### Fields

- `geometry: TilemapGeometry`
- `chunk_size: UVec2`
- `layers: BTreeMap<TileLayerId, TileLayerState>`

### Guidance

- chunk sizes between `8x8` and `32x32` are the practical starting range
- smaller chunks reduce the cost of isolated edits
- larger chunks reduce entity and mesh counts for mostly static maps

## `TileLayerConfig`

Per-layer authored configuration.

### Fields

- `id: TileLayerId`
- `name: String`
- `visible: bool`
- `offset: Vec2`
- `render: Option<TileLayerRenderConfig>`

### Constructors

- `TileLayerConfig::visual(id, name, render)`
- `TileLayerConfig::logic_only(id, name)`
- `with_offset(Vec2)`

### Guidance

- use `logic_only(...)` for collision, navigation, or metadata layers that should not render
- prefer stable `TileLayerId` values for persistence or editor adapters
- use `offset` sparingly; it is best for parallax or overlay alignment, not as a replacement for a coherent map transform

## `TileLayerRenderConfig`

Visual settings for a renderable layer.

### Fields

- `atlas: TileAtlasLayout`
- `z_index: f32`
- `tint: Color`
- `alpha_mode: AlphaMode2d`
- `chunk_sort_step: f32`

### Defaults

- `z_index = 0.0`
- `tint = Color::WHITE`
- `alpha_mode = AlphaMode2d::Blend`
- `chunk_sort_step = 0.0001`

### Builders

- `with_z_index(f32)`
- `with_tint(Color)`
- `with_alpha_mode(AlphaMode2d)`
- `with_chunk_sort_step(f32)`

### Guidance

- use larger `z_index` values for overlays, highlights, or decals
- `chunk_sort_step` matters for isometric and hex chunk ordering
- transparent oversized art should live on a dedicated layer when you need explicit ordering control

## `TileAtlasLayout`

Describes how atlas indices map into UVs.

### Fields

- `image: Handle<Image>`
- `texture_size: UVec2`
- `tile_size: UVec2`
- `columns: u32`
- `rows: u32`
- `padding: UVec2`
- `margin: UVec2`

### Constructors and builders

- `TileAtlasLayout::from_grid(image, texture_size, tile_size, columns, rows)`
- `with_padding(UVec2)`
- `with_margin(UVec2)`

### Guidance

- `columns * rows` defines the valid atlas index range
- set `padding` and `margin` when importing atlas sheets that were packed with gutters
- prefer nearest sampling in examples or pixel-art workflows

## `TileKind`

Reusable tile definition.

### Fields

- `name: String`
- `render: TileRenderRule`
- `collision: Option<TileCollisionDescriptor>`
- `flags: u32`
- `movement_cost: u16`

### Constructors

- `TileKind::static_tile(name, atlas_index)`
- `TileKind::animated_tile(name, TileAnimation)`
- `TileKind::autotile(name, AutotileBinding)`

### Builders

- `with_collision(TileCollisionDescriptor)`
- `with_flags(u32)`
- `with_movement_cost(u16)`

### Guidance

- keep tile definitions reusable and data-oriented
- use `flags` for small shared bitfields, not game-specific object graphs
- `movement_cost = 1` is the intended baseline

## `AutotileRuleSet`

Rule table for reusable autotile matching.

### Fields

- `neighborhood: AutotileNeighborhood`
- `variants: BTreeMap<u16, u32>`
- `fallback_atlas_index: u32`

### Guidance

- `Cardinal4` fits roads, rivers, walls, and pipe-style connectors
- `Moore8` is more appropriate for full blob or terrain masks
- missing masks fall back to `fallback_atlas_index`

## `TileAnimation`

Definition-driven animated tile clip.

### Fields

- `frames: Vec<TileAnimationFrame>`

### Guidance

- frame durations should stay positive
- animation cost scales with the number of chunks containing the animated kind, not the number of tile entities

## `TilemapDebugSettings`

Runtime debug resource.

### Fields

- `enabled: bool`
- `draw_chunk_bounds: bool`
- `draw_dirty_chunks: bool`
- `chunk_color: Color`
- `dirty_color: Color`
- `collision_color: Color`

### Defaults

- `enabled = false`
- chunk bounds enabled
- dirty chunk bounds enabled

### Guidance

- enable this in labs and sandboxes, not by default in shipping game scenes
- dirty chunk drawing is useful for validating edit locality and animation rebuild scopes

## `TilePathOptions`

Options for the A* pathfinding algorithm.

### Fields

- `max_cost: u32` — maximum total path cost before giving up. Default: `u32::MAX`
- `diagonal: bool` — whether to allow diagonal movement on square/iso grids. Default: `false`

### Builders

- `with_max_cost(u32)`
- `with_diagonal(bool)`

### Guidance

- set `max_cost` to limit search radius for performance or gameplay reasons
- `diagonal = true` allows 8-directional movement; path steps include diagonals
- hex grids ignore the `diagonal` flag and always use 6 neighbors

## `TilePathResult`

Result of a successful pathfinding query.

### Fields

- `path: Vec<TileCoord>` — ordered tile coordinates from start to goal (inclusive)
- `total_cost: u32` — accumulated movement cost along the path

## `TileKindPathPolicy`

Built-in pathfinding policy used by `find_path(...)` and `reachable_tiles(...)`.

### Behavior

- destination tiles must exist on the queried layer
- same-layer tiles with `TileCollisionDescriptor` are impassable
- same-layer `TileKind.movement_cost` is used as the per-step cost

### Guidance

- use this when the queried layer already contains the locomotion metadata you want
- switch to `find_path_with_policy(...)` or `reachable_tiles_with_policy(...)` when movement depends on other layers or agent-specific rules

## `TilePathStep<'a>`

Context passed into custom `TilePathPolicy` implementations and `TilePathCallbacks`.

### Fields

- `map: &Tilemap`
- `layer_id: TileLayerId`
- `layer: &TileLayerState`
- `from: TileCoord`
- `to: TileCoord`
- `from_tile: Option<&TileCell>`
- `from_kind: Option<&TileKind>`
- `to_tile: Option<&TileCell>`
- `to_kind: Option<&TileKind>`

### Guidance

- inspect `to_kind` / `to_tile` for same-layer terrain decisions
- inspect `map` when passability or cost depends on other layers
- prefer positive movement costs for passable tiles

## `TilePathCallbacks`

Closure adapter for custom pathfinding rules.

### Constructor

- `TilePathCallbacks::new(passability, movement_cost)`

### Guidance

- use this for one-off closures in examples, tools, or gameplay systems
- implement `TilePathPolicy` directly for reusable agent types or locomotion presets

## `TiledImportOptions`

Runtime import surface for normalized Tiled JSON ingestion.

### Fields

- `atlas: TileAtlasLayout`
- `catalog: TileCatalog`
- `gid_to_kind: BTreeMap<u32, TileKindId>`
- `chunk_size: UVec2`
- `origin: Vec2`
- `row_direction: TileRowDirection`
- `layer_z_step: f32`
- `logic_only_layers: BTreeSet<String>`

### Constructor

- `TiledImportOptions::new(atlas, catalog, gid_to_kind)`

### Guidance

- `gid_to_kind` should map stripped Tiled global tile IDs, not flip-flagged values
- `logic_only_layers` is useful for collision, nav, and metadata layers that should import but not render
- `layer_z_step` becomes the per-layer `z_index` increment for imported visual layers
- `origin` and `row_direction` should match the world-space convention used by the rest of your app

## `ImportedTilemapScene`

### Fields

- `map: Tilemap`
- `object_spawns: Vec<TileObjectSpawn>`

The importer intentionally returns normalized object-layer spawn descriptors instead of spawning entities directly. That keeps scene/entity instantiation policy outside the durable tile runtime.
