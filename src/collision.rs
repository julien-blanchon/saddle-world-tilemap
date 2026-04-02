use crate::{ChunkCoord, TileCoord, TileLayerId};
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum TileCollisionShape {
    Solid,
    Rect { offset: Vec2, size: Vec2 },
    SlopeNorthEast,
    SlopeNorthWest,
    SlopeSouthEast,
    SlopeSouthWest,
    SensorRect { offset: Vec2, size: Vec2 },
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileCollisionDescriptor {
    pub shape: TileCollisionShape,
    pub flags: u32,
}

impl TileCollisionDescriptor {
    #[must_use]
    pub fn solid() -> Self {
        Self {
            shape: TileCollisionShape::Solid,
            flags: 0,
        }
    }

    #[must_use]
    pub fn sensor_rect(offset: Vec2, size: Vec2) -> Self {
        Self {
            shape: TileCollisionShape::SensorRect { offset, size },
            flags: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileCollisionCell {
    pub coord: TileCoord,
    pub map_local_center: Vec2,
    pub descriptor: TileCollisionDescriptor,
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct TilemapCollisionChunk {
    pub map: Entity,
    pub layer: TileLayerId,
    pub chunk: ChunkCoord,
    pub revision: u64,
    pub cells: Vec<TileCollisionCell>,
}
