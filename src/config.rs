use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileAtlasLayout {
    pub image: Handle<Image>,
    pub texture_size: UVec2,
    pub tile_size: UVec2,
    pub columns: u32,
    pub rows: u32,
    pub padding: UVec2,
    pub margin: UVec2,
}

impl TileAtlasLayout {
    #[must_use]
    pub fn from_grid(
        image: Handle<Image>,
        texture_size: UVec2,
        tile_size: UVec2,
        columns: u32,
        rows: u32,
    ) -> Self {
        Self {
            image,
            texture_size,
            tile_size,
            columns,
            rows,
            padding: UVec2::ZERO,
            margin: UVec2::ZERO,
        }
    }

    #[must_use]
    pub fn with_padding(mut self, padding: UVec2) -> Self {
        self.padding = padding;
        self
    }

    #[must_use]
    pub fn with_margin(mut self, margin: UVec2) -> Self {
        self.margin = margin;
        self
    }

    #[must_use]
    pub fn tile_count(&self) -> u32 {
        self.columns * self.rows
    }

    #[must_use]
    pub fn uv_rect(&self, atlas_index: u32) -> [Vec2; 4] {
        let column = atlas_index % self.columns;
        let row = atlas_index / self.columns;

        let step_x = self.tile_size.x + self.padding.x;
        let step_y = self.tile_size.y + self.padding.y;

        let min_px = UVec2::new(
            self.margin.x + column * step_x,
            self.margin.y + row * step_y,
        );
        let max_px = min_px + self.tile_size;

        let min = min_px.as_vec2() / self.texture_size.as_vec2();
        let max = max_px.as_vec2() / self.texture_size.as_vec2();

        [
            Vec2::new(min.x, min.y),
            Vec2::new(max.x, min.y),
            Vec2::new(max.x, max.y),
            Vec2::new(min.x, max.y),
        ]
    }
}
