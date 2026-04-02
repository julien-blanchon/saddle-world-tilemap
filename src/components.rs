use crate::{ChunkCoord, TileLayerId, Tilemap};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Debug, Clone, Default)]
pub struct TilemapRoot;

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct TilemapLayerNode {
    pub map: Entity,
    pub layer: TileLayerId,
}

#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct TilemapRenderChunk {
    pub map: Entity,
    pub layer: TileLayerId,
    pub chunk: ChunkCoord,
    pub revision: u64,
}

#[derive(Component, Debug, Clone, PartialEq, Reflect, Default)]
#[reflect(Component, Debug, Clone, Default)]
pub struct TilemapDiagnostics {
    pub logical_chunks_total: usize,
    pub dirty_chunks: usize,
    pub chunks_rebuilt_this_frame: usize,
    pub collision_chunks_total: usize,
    pub animated_chunks_total: usize,
    pub tile_edits_this_frame: usize,
}

#[derive(Bundle)]
pub struct TilemapBundle {
    pub name: Name,
    pub root: TilemapRoot,
    pub tilemap: Tilemap,
    pub diagnostics: TilemapDiagnostics,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl TilemapBundle {
    #[must_use]
    pub fn new(name: impl Into<String>, tilemap: Tilemap) -> Self {
        Self {
            name: Name::new(name.into()),
            root: TilemapRoot,
            tilemap,
            diagnostics: TilemapDiagnostics::default(),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::Visible,
            inherited_visibility: InheritedVisibility::VISIBLE,
            view_visibility: ViewVisibility::default(),
        }
    }
}
