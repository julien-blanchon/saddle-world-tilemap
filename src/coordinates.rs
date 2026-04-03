use bevy::{camera::Camera, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

impl TileCoord {
    pub const ZERO: Self = Self::new(0, 0);

    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }

    #[must_use]
    pub fn chunk(self, chunk_size: UVec2) -> ChunkCoord {
        ChunkCoord::new(
            self.x.div_euclid(chunk_size.x as i32),
            self.y.div_euclid(chunk_size.y as i32),
        )
    }

    #[must_use]
    pub fn local_in_chunk(self, chunk_size: UVec2) -> UVec2 {
        UVec2::new(
            self.x.rem_euclid(chunk_size.x as i32) as u32,
            self.y.rem_euclid(chunk_size.y as i32) as u32,
        )
    }

    #[must_use]
    pub const fn cardinal_neighbors(self) -> [Self; 4] {
        [
            self.offset(0, -1),
            self.offset(1, 0),
            self.offset(0, 1),
            self.offset(-1, 0),
        ]
    }

    #[must_use]
    pub const fn eight_neighbors(self) -> [Self; 8] {
        [
            self.offset(0, -1),
            self.offset(1, -1),
            self.offset(1, 0),
            self.offset(1, 1),
            self.offset(0, 1),
            self.offset(-1, 1),
            self.offset(-1, 0),
            self.offset(-1, -1),
        ]
    }
}

impl Default for TileCoord {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub const ZERO: Self = Self::new(0, 0);

    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub fn tile_origin(self, chunk_size: UVec2) -> TileCoord {
        TileCoord::new(self.x * chunk_size.x as i32, self.y * chunk_size.y as i32)
    }
}

impl Default for ChunkCoord {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct TileRect {
    pub min: TileCoord,
    pub size: UVec2,
}

impl TileRect {
    #[must_use]
    pub const fn new(min: TileCoord, size: UVec2) -> Self {
        Self { min, size }
    }

    pub fn iter(self) -> impl Iterator<Item = TileCoord> {
        (0..self.size.y as i32).flat_map(move |row| {
            (0..self.size.x as i32)
                .map(move |col| TileCoord::new(self.min.x + col, self.min.y + row))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub enum TileRowDirection {
    Up,
    Down,
}

impl TileRowDirection {
    #[must_use]
    pub const fn sign(self) -> f32 {
        match self {
            Self::Up => 1.0,
            Self::Down => -1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub enum TilemapHexParity {
    Odd,
    Even,
}

impl TilemapHexParity {
    #[must_use]
    pub const fn is_shifted(self, index: i32) -> bool {
        match self {
            Self::Odd => index & 1 != 0,
            Self::Even => index & 1 == 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub enum TilemapOrientation {
    Square,
    IsometricDiamond,
    HexPointyColumns(TilemapHexParity),
    HexFlatRows(TilemapHexParity),
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TilemapGeometry {
    pub orientation: TilemapOrientation,
    pub grid_size: Vec2,
    pub tile_render_size: Vec2,
    pub origin: Vec2,
    pub row_direction: TileRowDirection,
}

impl TilemapGeometry {
    #[must_use]
    pub fn square(tile_size: Vec2) -> Self {
        Self {
            orientation: TilemapOrientation::Square,
            grid_size: tile_size,
            tile_render_size: tile_size,
            origin: Vec2::ZERO,
            row_direction: TileRowDirection::Down,
        }
    }

    #[must_use]
    pub fn isometric_diamond(tile_render_size: Vec2) -> Self {
        Self {
            orientation: TilemapOrientation::IsometricDiamond,
            grid_size: tile_render_size * 0.5,
            tile_render_size,
            origin: Vec2::ZERO,
            row_direction: TileRowDirection::Down,
        }
    }

    #[must_use]
    pub fn hex_pointy_columns(
        tile_render_size: Vec2,
        parity: TilemapHexParity,
    ) -> Self {
        Self {
            orientation: TilemapOrientation::HexPointyColumns(parity),
            grid_size: Vec2::new(tile_render_size.x * 0.75, tile_render_size.y),
            tile_render_size,
            origin: Vec2::ZERO,
            row_direction: TileRowDirection::Down,
        }
    }

    #[must_use]
    pub fn hex_flat_rows(tile_render_size: Vec2, parity: TilemapHexParity) -> Self {
        Self {
            orientation: TilemapOrientation::HexFlatRows(parity),
            grid_size: Vec2::new(tile_render_size.x, tile_render_size.y * 0.75),
            tile_render_size,
            origin: Vec2::ZERO,
            row_direction: TileRowDirection::Down,
        }
    }

    #[must_use]
    pub fn with_origin(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    #[must_use]
    pub fn with_row_direction(mut self, row_direction: TileRowDirection) -> Self {
        self.row_direction = row_direction;
        self
    }

    #[must_use]
    pub fn with_tile_render_size(mut self, tile_render_size: Vec2) -> Self {
        self.tile_render_size = tile_render_size;
        self
    }

    #[must_use]
    pub fn tile_to_local(self, tile: TileCoord) -> Vec2 {
        match self.orientation {
            TilemapOrientation::Square => Vec2::new(
                self.origin.x + tile.x as f32 * self.grid_size.x,
                self.origin.y + tile.y as f32 * self.grid_size.y * self.row_direction.sign(),
            ),
            TilemapOrientation::IsometricDiamond => Vec2::new(
                self.origin.x + (tile.x - tile.y) as f32 * self.grid_size.x,
                self.origin.y
                    + (tile.x + tile.y) as f32 * self.grid_size.y * self.row_direction.sign(),
            ),
            TilemapOrientation::HexPointyColumns(parity) => Vec2::new(
                self.origin.x + tile.x as f32 * self.grid_size.x,
                self.origin.y
                    + (tile.y as f32 * self.grid_size.y
                        + if parity.is_shifted(tile.x) {
                            self.tile_render_size.y * 0.5
                        } else {
                            0.0
                        })
                        * self.row_direction.sign(),
            ),
            TilemapOrientation::HexFlatRows(parity) => Vec2::new(
                self.origin.x
                    + tile.x as f32 * self.grid_size.x
                    + if parity.is_shifted(tile.y) {
                        self.tile_render_size.x * 0.5
                    } else {
                        0.0
                    },
                self.origin.y + tile.y as f32 * self.grid_size.y * self.row_direction.sign(),
            ),
        }
    }

    #[must_use]
    pub fn tile_bounds_local(self, tile: TileCoord) -> Rect {
        Rect::from_center_size(self.tile_to_local(tile), self.tile_render_size)
    }

    #[must_use]
    pub fn local_to_tile(self, local: Vec2) -> TileCoord {
        let local = local - self.origin;
        let row_sign = self.row_direction.sign();

        match self.orientation {
            TilemapOrientation::Square => TileCoord::new(
                (local.x / self.grid_size.x).round() as i32,
                (local.y / (self.grid_size.y * row_sign)).round() as i32,
            ),
            TilemapOrientation::IsometricDiamond => {
                let fx = local.x / self.grid_size.x;
                let fy = local.y / (self.grid_size.y * row_sign);
                TileCoord::new(
                    ((fx + fy) * 0.5).round() as i32,
                    ((fy - fx) * 0.5).round() as i32,
                )
            }
            TilemapOrientation::HexPointyColumns(parity) => {
                let guess_x = (local.x / self.grid_size.x).round() as i32;
                let guess_y = ((local.y / row_sign - hex_column_vertical_offset(
                    parity,
                    guess_x,
                    self.tile_render_size.y,
                )) / self.grid_size.y)
                    .round() as i32;
                self.closest_hex_coord(local, TileCoord::new(guess_x, guess_y))
            }
            TilemapOrientation::HexFlatRows(parity) => {
                let guess_y = (local.y / (self.grid_size.y * row_sign)).round() as i32;
                let guess_x = ((local.x
                    - hex_row_horizontal_offset(parity, guess_y, self.tile_render_size.x))
                    / self.grid_size.x)
                    .round() as i32;
                self.closest_hex_coord(local, TileCoord::new(guess_x, guess_y))
            }
        }
    }

    #[must_use]
    pub fn tile_to_world(self, transform: &GlobalTransform, tile: TileCoord) -> Vec2 {
        transform
            .affine()
            .transform_point3(self.tile_to_local(tile).extend(0.0))
            .truncate()
    }

    pub fn world_to_tile(
        self,
        transform: &GlobalTransform,
        world_position: Vec2,
    ) -> Option<TileCoord> {
        let inverse = transform.affine().inverse();
        let local = inverse
            .transform_point3(world_position.extend(0.0))
            .truncate();
        Some(self.local_to_tile(local))
    }

    #[must_use]
    pub fn cursor_to_tile(
        self,
        camera: &Camera,
        camera_transform: &GlobalTransform,
        cursor_position: Vec2,
        map_transform: &GlobalTransform,
    ) -> Option<TileCoord> {
        let world = camera
            .viewport_to_world_2d(camera_transform, cursor_position)
            .ok()?;
        self.world_to_tile(map_transform, world)
    }

    #[must_use]
    pub fn chunk_bounds_local(self, chunk_size: UVec2, chunk: ChunkCoord) -> Rect {
        let origin = chunk.tile_origin(chunk_size);
        let corners = [
            origin,
            origin.offset(chunk_size.x as i32 - 1, 0),
            origin.offset(0, chunk_size.y as i32 - 1),
            origin.offset(chunk_size.x as i32 - 1, chunk_size.y as i32 - 1),
        ];

        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for corner in corners {
            let bounds = self.tile_bounds_local(corner);
            min = min.min(bounds.min);
            max = max.max(bounds.max);
        }

        Rect::from_corners(min, max)
    }

    fn closest_hex_coord(self, local: Vec2, guess: TileCoord) -> TileCoord {
        let geometry = self.with_origin(Vec2::ZERO);
        let mut best = guess;
        let mut best_distance = geometry.tile_to_local(guess).distance_squared(local);

        for dy in -2..=2 {
            for dx in -2..=2 {
                let candidate = guess.offset(dx, dy);
                let distance = geometry.tile_to_local(candidate).distance_squared(local);
                if distance < best_distance {
                    best = candidate;
                    best_distance = distance;
                }
            }
        }

        best
    }
}

fn hex_column_vertical_offset(parity: TilemapHexParity, column: i32, tile_height: f32) -> f32 {
    if parity.is_shifted(column) {
        tile_height * 0.5
    } else {
        0.0
    }
}

fn hex_row_horizontal_offset(parity: TilemapHexParity, row: i32, tile_width: f32) -> f32 {
    if parity.is_shifted(row) {
        tile_width * 0.5
    } else {
        0.0
    }
}

impl Default for TilemapGeometry {
    fn default() -> Self {
        Self::square(Vec2::splat(16.0))
    }
}

#[cfg(test)]
#[path = "coordinates_tests.rs"]
mod tests;
