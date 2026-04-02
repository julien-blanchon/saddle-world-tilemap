use crate::{TileCell, TileKindId, layer::TileOrientation};
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(crate) struct ResolvedTileVisual {
    pub atlas_index: u32,
    pub tint: Color,
    pub orientation: TileOrientation,
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Debug, Clone)]
pub struct TileChunk {
    pub tiles: Vec<Option<TileCell>>,
    #[reflect(ignore)]
    pub(crate) resolved_visuals: Vec<Option<ResolvedTileVisual>>,
    #[reflect(ignore)]
    pub(crate) resolved_collisions: Vec<Option<crate::TileCollisionDescriptor>>,
    #[reflect(ignore)]
    pub(crate) animated_kinds: BTreeSet<TileKindId>,
    pub revision: u64,
}

impl TileChunk {
    #[must_use]
    pub fn new(chunk_size: UVec2) -> Self {
        let len = (chunk_size.x * chunk_size.y) as usize;
        Self {
            tiles: vec![None; len],
            resolved_visuals: vec![None; len],
            resolved_collisions: vec![None; len],
            animated_kinds: BTreeSet::new(),
            revision: 0,
        }
    }

    #[must_use]
    pub fn index(chunk_size: UVec2, local: UVec2) -> usize {
        (local.y * chunk_size.x + local.x) as usize
    }

    #[must_use]
    pub fn get(&self, chunk_size: UVec2, local: UVec2) -> Option<&TileCell> {
        self.tiles.get(Self::index(chunk_size, local))?.as_ref()
    }

    pub fn set(
        &mut self,
        chunk_size: UVec2,
        local: UVec2,
        tile: Option<TileCell>,
    ) -> Option<TileCell> {
        let index = Self::index(chunk_size, local);
        let previous = self.tiles[index].take();
        self.tiles[index] = tile;
        previous
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.iter().all(Option::is_none)
    }
}
