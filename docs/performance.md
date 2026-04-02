# `saddle-world-tilemap` Performance

## Current runtime model

The `0.1.0` runtime is dense and chunk-based:

- each logical layer stores dense `TileChunk` buffers
- each dirty visual chunk becomes one mesh entity
- each dirty collision chunk becomes one descriptor entity

This is a good fit for:

- authored levels
- procedural maps with predictable density
- runtime edits localized to a few chunks
- large scrolling maps that stay fully resident

It is not yet a sparse or streamed backend.

## Chunk size guidance

Chunk size is the main tuning knob.

### Smaller chunks

Good when:

- single-tile edits are frequent
- runtime brushes are local
- animated tiles only affect small pockets of the world

Tradeoffs:

- more chunk entities
- more mesh assets
- more bookkeeping

### Larger chunks

Good when:

- the world is mostly static
- the map is dense
- entity count matters more than isolated edit cost

Tradeoffs:

- one small edit rebuilds more geometry
- chunk-local autotile and collision sync work touches more tiles

## Practical starting points

- `8x8` or `12x12`: edit-heavy maps, builders, sandboxes
- `16x16`: balanced general-purpose default
- `24x24` or `32x32`: denser, more static worlds

## Runtime edit costs

### `SetTile` and `ClearTile`

The logical mutation is cheap. The cost comes from the follow-up projection work:

1. mark the owning chunk dirty
2. resolve the chunk
3. rebuild the visual mesh and collision descriptors for that chunk

### Autotile edits

Autotile edits mark the edited chunk and any neighboring chunks that contain affected cells. This keeps the work local, but roads, rivers, and pipe networks near chunk seams will rebuild more than one chunk.

### Batch edits

Grouped edits to the same chunk are cheap relative to scattering one edit across many chunks. If you are applying a brush or procedural patch, prefer batching by chunk when possible.

## Animated tile costs

Animated tiles do not create one timer per tile. The runtime tracks playback per animated kind.

When the visible frame changes:

- only chunks containing that animated kind are marked dirty
- only those chunks re-resolve and rebuild

This keeps the cost proportional to affected chunks, not total tile count.

## Large map behavior

The crate-local lab includes a dense `96x96` map as a practical smoke case. In the current implementation:

- all chunks stay resident once built
- camera motion does not stream chunks in or out
- camera sweeps are mostly a renderer and transform test, not a streaming test

If you need true streaming or sparse residency, that should be a later backend extension rather than a hidden side effect of the current API.

## Dense vs sparse tradeoff

The current crate is dense-only. Mostly-empty infinite worlds will waste memory compared with a sparse backend.

This is an intentional `0.1.0` limit:

- dense chunks keep access predictable and simple
- runtime edits stay straightforward
- editor import paths remain easy to reason about

Sparse storage can be added later behind the same logical surface if it proves necessary.

## Debugging locality

Enable `TilemapDebugSettings` in labs or tools to validate:

- chunk bounds
- dirty chunk scopes
- how many chunks each runtime edit touches
- whether animation rebuilds stay localized

`TilemapDiagnostics` is meant for the same workflow. It exposes enough counters for BRP and E2E tests to catch accidental broad rebuilds.

`dirty_chunks` is reported as a distinct chunk count across resolve, render, and collision queues so one hot chunk does not look three times dirtier than it really is.
