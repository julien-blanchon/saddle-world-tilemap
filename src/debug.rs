use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Resource, Debug, Clone, PartialEq)]
pub struct TilemapDebugSettings {
    pub enabled: bool,
    pub draw_chunk_bounds: bool,
    pub draw_dirty_chunks: bool,
    pub chunk_color: Color,
    pub dirty_color: Color,
    pub collision_color: Color,
}

impl Default for TilemapDebugSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_chunk_bounds: true,
            draw_dirty_chunks: true,
            chunk_color: Color::srgba(0.18, 0.82, 0.98, 0.75),
            dirty_color: Color::srgba(0.95, 0.72, 0.16, 0.95),
            collision_color: Color::srgba(0.91, 0.26, 0.31, 0.95),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct TilemapDebugOverlay {
    pub draw_chunk_bounds: bool,
    pub draw_dirty_chunks: bool,
}

impl Default for TilemapDebugOverlay {
    fn default() -> Self {
        Self {
            draw_chunk_bounds: true,
            draw_dirty_chunks: true,
        }
    }
}
