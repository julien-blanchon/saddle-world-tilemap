use crate::{
    AutotileBinding, AutotileRuleSet, AutotileRuleSetId, ChunkCoord, TileAnimation,
    TileAtlasLayout, TileCollisionDescriptor, TileCoord, TileRect, TileRowDirection,
    TilemapGeometry, chunk::TileChunk,
};
use bevy::{prelude::*, sprite_render::AlphaMode2d};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct TileLayerId(pub u16);

impl TileLayerId {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct TileKindId(pub u16);

impl TileKindId {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, Default, PartialEq, Hash)]
#[repr(u8)]
pub enum TileOrientation {
    #[default]
    Default = 0b000,
    Rotate90 = 0b011,
    Rotate180 = 0b110,
    Rotate270 = 0b101,
    MirrorH = 0b100,
    MirrorHRotate90 = 0b001,
    MirrorHRotate180 = 0b010,
    MirrorHRotate270 = 0b111,
}

impl TileOrientation {
    const MIRROR_H_BIT: u8 = 0b100;
    const MIRROR_V_BIT: u8 = 0b010;
    const MIRROR_D_BIT: u8 = 0b001;

    #[must_use]
    pub fn mirror_h(self) -> bool {
        (self as u8) & Self::MIRROR_H_BIT != 0
    }

    #[must_use]
    pub fn mirror_v(self) -> bool {
        (self as u8) & Self::MIRROR_V_BIT != 0
    }

    #[must_use]
    pub fn mirror_d(self) -> bool {
        (self as u8) & Self::MIRROR_D_BIT != 0
    }

    #[must_use]
    pub fn inverse(self) -> Self {
        match self {
            Self::Default => Self::Default,
            Self::Rotate90 => Self::Rotate270,
            Self::Rotate180 => Self::Rotate180,
            Self::Rotate270 => Self::Rotate90,
            Self::MirrorH => Self::MirrorH,
            Self::MirrorHRotate90 => Self::MirrorHRotate90,
            Self::MirrorHRotate180 => Self::MirrorHRotate180,
            Self::MirrorHRotate270 => Self::MirrorHRotate270,
        }
    }

    #[must_use]
    pub fn apply_to_ivec2(self, position: &IVec2) -> IVec2 {
        let mut x = position.x;
        let mut y = -position.y;

        if self.mirror_d() {
            (x, y) = (y, x);
        }
        if self.mirror_h() {
            x = -x;
        }
        if self.mirror_v() {
            y = -y;
        }

        IVec2::new(x, -y)
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileVisual {
    pub atlas_index: u32,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum TileRenderRule {
    Static(TileVisual),
    Animated(TileAnimation),
    Autotile(AutotileBinding),
}

impl TileRenderRule {
    #[must_use]
    pub fn autotile_binding(&self) -> Option<&AutotileBinding> {
        match self {
            Self::Autotile(binding) => Some(binding),
            _ => None,
        }
    }

    #[must_use]
    pub fn animation(&self) -> Option<&TileAnimation> {
        match self {
            Self::Animated(animation) => Some(animation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileKind {
    pub name: String,
    pub render: TileRenderRule,
    pub collision: Option<TileCollisionDescriptor>,
    pub flags: u32,
    pub movement_cost: u16,
}

impl TileKind {
    #[must_use]
    pub fn static_tile(name: impl Into<String>, atlas_index: u32) -> Self {
        Self {
            name: name.into(),
            render: TileRenderRule::Static(TileVisual { atlas_index }),
            collision: None,
            flags: 0,
            movement_cost: 1,
        }
    }

    #[must_use]
    pub fn animated_tile(name: impl Into<String>, animation: TileAnimation) -> Self {
        Self {
            name: name.into(),
            render: TileRenderRule::Animated(animation),
            collision: None,
            flags: 0,
            movement_cost: 1,
        }
    }

    #[must_use]
    pub fn autotile(name: impl Into<String>, binding: AutotileBinding) -> Self {
        Self {
            name: name.into(),
            render: TileRenderRule::Autotile(binding),
            collision: None,
            flags: 0,
            movement_cost: 1,
        }
    }

    #[must_use]
    pub fn with_collision(mut self, collision: TileCollisionDescriptor) -> Self {
        self.collision = Some(collision);
        self
    }

    #[must_use]
    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }

    #[must_use]
    pub fn with_movement_cost(mut self, movement_cost: u16) -> Self {
        self.movement_cost = movement_cost;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileCatalog {
    pub kinds: BTreeMap<TileKindId, TileKind>,
    pub autotile_rules: BTreeMap<AutotileRuleSetId, AutotileRuleSet>,
}

impl TileCatalog {
    pub fn insert_kind(&mut self, id: TileKindId, kind: TileKind) {
        self.kinds.insert(id, kind);
    }

    pub fn insert_autotile_rule(&mut self, id: AutotileRuleSetId, rule: AutotileRuleSet) {
        self.autotile_rules.insert(id, rule);
    }

    #[must_use]
    pub fn kind(&self, id: TileKindId) -> Option<&TileKind> {
        self.kinds.get(&id)
    }

    #[must_use]
    pub fn autotile_rule(&self, id: AutotileRuleSetId) -> Option<&AutotileRuleSet> {
        self.autotile_rules.get(&id)
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileLayerRenderConfig {
    pub atlas: TileAtlasLayout,
    pub z_index: f32,
    pub tint: Color,
    pub alpha_mode: AlphaMode2d,
    pub chunk_sort_step: f32,
}

impl TileLayerRenderConfig {
    #[must_use]
    pub fn new(atlas: TileAtlasLayout) -> Self {
        Self {
            atlas,
            z_index: 0.0,
            tint: Color::WHITE,
            alpha_mode: AlphaMode2d::Blend,
            chunk_sort_step: 0.0001,
        }
    }

    #[must_use]
    pub fn with_z_index(mut self, z_index: f32) -> Self {
        self.z_index = z_index;
        self
    }

    #[must_use]
    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    #[must_use]
    pub fn with_alpha_mode(mut self, alpha_mode: AlphaMode2d) -> Self {
        self.alpha_mode = alpha_mode;
        self
    }

    #[must_use]
    pub fn with_chunk_sort_step(mut self, chunk_sort_step: f32) -> Self {
        self.chunk_sort_step = chunk_sort_step;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileLayerConfig {
    pub id: TileLayerId,
    pub name: String,
    pub visible: bool,
    pub offset: Vec2,
    pub render: Option<TileLayerRenderConfig>,
}

impl TileLayerConfig {
    #[must_use]
    pub fn visual(id: TileLayerId, name: impl Into<String>, render: TileLayerRenderConfig) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            offset: Vec2::ZERO,
            render: Some(render),
        }
    }

    #[must_use]
    pub fn logic_only(id: TileLayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            offset: Vec2::ZERO,
            render: None,
        }
    }

    #[must_use]
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileCell {
    pub kind: TileKindId,
    pub tint: Color,
    pub orientation: TileOrientation,
}

impl TileCell {
    #[must_use]
    pub fn new(kind: TileKindId) -> Self {
        Self {
            kind,
            tint: Color::WHITE,
            orientation: TileOrientation::Default,
        }
    }

    #[must_use]
    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    #[must_use]
    pub fn with_orientation(mut self, orientation: TileOrientation) -> Self {
        self.orientation = orientation;
        self
    }
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Debug, Clone)]
pub struct TileLayerState {
    pub config: TileLayerConfig,
    pub catalog: TileCatalog,
    pub chunks: BTreeMap<ChunkCoord, TileChunk>,
    #[reflect(ignore)]
    pub(crate) dirty_resolve: BTreeSet<ChunkCoord>,
    #[reflect(ignore)]
    pub(crate) dirty_render: BTreeSet<ChunkCoord>,
    #[reflect(ignore)]
    pub(crate) dirty_collision: BTreeSet<ChunkCoord>,
}

impl TileLayerState {
    #[must_use]
    pub fn new(config: TileLayerConfig, catalog: TileCatalog) -> Self {
        Self {
            config,
            catalog,
            chunks: BTreeMap::new(),
            dirty_resolve: BTreeSet::new(),
            dirty_render: BTreeSet::new(),
            dirty_collision: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn get_tile(&self, chunk_size: UVec2, coord: TileCoord) -> Option<&TileCell> {
        let chunk_coord = coord.chunk(chunk_size);
        let local = coord.local_in_chunk(chunk_size);
        self.chunks.get(&chunk_coord)?.get(chunk_size, local)
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct Tilemap {
    pub geometry: TilemapGeometry,
    pub chunk_size: UVec2,
    pub layers: BTreeMap<TileLayerId, TileLayerState>,
}

impl Tilemap {
    #[must_use]
    pub fn new(geometry: TilemapGeometry, chunk_size: UVec2) -> Self {
        Self {
            geometry,
            chunk_size,
            layers: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_layer(mut self, layer: TileLayerState) -> Self {
        self.insert_layer(layer);
        self
    }

    pub fn insert_layer(&mut self, mut layer: TileLayerState) {
        layer.mark_all_dirty_existing();
        self.layers.insert(layer.config.id, layer);
    }

    #[must_use]
    pub fn layer(&self, id: TileLayerId) -> Option<&TileLayerState> {
        self.layers.get(&id)
    }

    #[must_use]
    pub fn layer_mut(&mut self, id: TileLayerId) -> Option<&mut TileLayerState> {
        self.layers.get_mut(&id)
    }

    #[must_use]
    pub fn get_tile(&self, layer_id: TileLayerId, coord: TileCoord) -> Option<&TileCell> {
        let layer = self.layers.get(&layer_id)?;
        layer.get_tile(self.chunk_size, coord)
    }

    pub fn set_tile(&mut self, layer_id: TileLayerId, coord: TileCoord, tile: TileCell) {
        self.set_tile_internal(layer_id, coord, Some(tile));
    }

    pub fn clear_tile(&mut self, layer_id: TileLayerId, coord: TileCoord) {
        self.set_tile_internal(layer_id, coord, None);
    }

    pub fn swap_tiles(&mut self, layer_id: TileLayerId, a: TileCoord, b: TileCoord) {
        if a == b {
            return;
        }

        let tile_a = self.get_tile(layer_id, a).cloned();
        let tile_b = self.get_tile(layer_id, b).cloned();
        if tile_a == tile_b {
            return;
        }

        self.set_tile_internal(layer_id, a, tile_b);
        self.set_tile_internal(layer_id, b, tile_a);
    }

    pub fn fill_rect(&mut self, layer_id: TileLayerId, rect: TileRect, tile: TileCell) {
        for coord in rect.iter() {
            self.set_tile(layer_id, coord, tile.clone());
        }
    }

    pub fn set_layer_visibility(&mut self, layer_id: TileLayerId, visible: bool) {
        if let Some(layer) = self.layers.get_mut(&layer_id) {
            layer.config.visible = visible;
        }
    }

    pub fn mark_all_dirty(&mut self) {
        for layer in self.layers.values_mut() {
            layer.mark_all_dirty_existing();
        }
    }

    fn set_tile_internal(
        &mut self,
        layer_id: TileLayerId,
        coord: TileCoord,
        tile: Option<TileCell>,
    ) {
        let Some(layer) = self.layers.get_mut(&layer_id) else {
            return;
        };

        let next_tile = tile.clone();
        let chunk_coord = coord.chunk(self.chunk_size);
        let local = coord.local_in_chunk(self.chunk_size);
        let previous_tile = layer
            .chunks
            .get(&chunk_coord)
            .and_then(|chunk| chunk.get(self.chunk_size, local))
            .cloned();

        if previous_tile == tile {
            return;
        }

        if let Some(tile) = tile {
            let chunk = layer
                .chunks
                .entry(chunk_coord)
                .or_insert_with(|| TileChunk::new(self.chunk_size));
            chunk.set(self.chunk_size, local, Some(tile));
            layer.dirty_resolve.insert(chunk_coord);
        } else if let Some(chunk) = layer.chunks.get_mut(&chunk_coord) {
            chunk.set(self.chunk_size, local, None);
            if chunk.is_empty() {
                layer.chunks.remove(&chunk_coord);
                layer.dirty_render.insert(chunk_coord);
                layer.dirty_collision.insert(chunk_coord);
            } else {
                layer.dirty_resolve.insert(chunk_coord);
            }
        }

        let autotile_changed = previous_tile
            .as_ref()
            .and_then(|tile| layer.catalog.kind(tile.kind))
            .and_then(|kind| kind.render.autotile_binding())
            .is_some()
            || next_tile
                .as_ref()
                .and_then(|tile| layer.catalog.kind(tile.kind))
                .and_then(|kind| kind.render.autotile_binding())
                .is_some();

        if autotile_changed {
            for neighbor in coord.eight_neighbors() {
                let neighbor_chunk = neighbor.chunk(self.chunk_size);
                if layer.chunks.contains_key(&neighbor_chunk) {
                    layer.dirty_resolve.insert(neighbor_chunk);
                }
            }
        }
    }
}

impl TileLayerState {
    fn mark_all_dirty_existing(&mut self) {
        let chunk_coords: Vec<ChunkCoord> = self.chunks.keys().copied().collect();
        self.dirty_resolve.extend(chunk_coords);
    }
}

impl Default for Tilemap {
    fn default() -> Self {
        Self::new(
            TilemapGeometry::square(Vec2::splat(16.0)).with_row_direction(TileRowDirection::Down),
            UVec2::splat(16),
        )
    }
}

#[cfg(test)]
#[path = "layer_tests.rs"]
mod tests;
