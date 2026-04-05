use crate::{ChunkCoord, TileCell, TileCoord, TileKindId, TileLayerId, TileRect};
use bevy::prelude::*;

#[derive(Message, Debug, Clone)]
pub enum TilemapCommand {
    SetTile {
        map: Entity,
        layer: TileLayerId,
        coord: TileCoord,
        tile: TileCell,
    },
    ClearTile {
        map: Entity,
        layer: TileLayerId,
        coord: TileCoord,
    },
    FillRect {
        map: Entity,
        layer: TileLayerId,
        rect: TileRect,
        tile: TileCell,
    },
    SwapTiles {
        map: Entity,
        layer: TileLayerId,
        a: TileCoord,
        b: TileCoord,
    },
    SetLayerVisibility {
        map: Entity,
        layer: TileLayerId,
        visible: bool,
    },
    FillCircle {
        map: Entity,
        layer: TileLayerId,
        center: TileCoord,
        radius: u32,
        tile: TileCell,
    },
    FillLine {
        map: Entity,
        layer: TileLayerId,
        from: TileCoord,
        to: TileCoord,
        tile: TileCell,
    },
    FloodFill {
        map: Entity,
        layer: TileLayerId,
        start: TileCoord,
        tile: TileCell,
        max_tiles: usize,
    },
}

#[derive(Message, Debug, Clone)]
pub struct TileChanged {
    pub map: Entity,
    pub layer: TileLayerId,
    pub coord: TileCoord,
    pub previous_kind: Option<TileKindId>,
    pub next_kind: Option<TileKindId>,
}

#[derive(Message, Debug, Clone)]
pub struct ChunkRebuilt {
    pub map: Entity,
    pub layer: TileLayerId,
    pub chunk: ChunkCoord,
    pub render_updated: bool,
    pub collision_updated: bool,
}

#[derive(Message, Debug, Clone)]
pub struct LayerVisibilityChanged {
    pub map: Entity,
    pub layer: TileLayerId,
    pub visible: bool,
}

#[derive(Message, Debug, Clone)]
pub struct TileAnimationLooped {
    pub map: Entity,
    pub layer: TileLayerId,
    pub kind: TileKindId,
}
