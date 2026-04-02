use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use saddle_world_tilemap::{
    AutotileBinding, AutotileGroupId, AutotileNeighborhood, AutotileRuleSet, AutotileRuleSetId,
    TileAnimation, TileAtlasLayout, TileCatalog, TileCell, TileCollisionDescriptor, TileCoord,
    TileKind, TileKindId, TileLayerConfig, TileLayerId, TileLayerRenderConfig, TileLayerState,
    Tilemap, TilemapBundle, TilemapDebugOverlay, TilemapGeometry,
};

pub const GROUND_LAYER: TileLayerId = TileLayerId::new(1);
pub const DETAIL_LAYER: TileLayerId = TileLayerId::new(2);
pub const COLLISION_LAYER: TileLayerId = TileLayerId::new(3);
pub const HIGHLIGHT_LAYER: TileLayerId = TileLayerId::new(4);

pub const SQUARE_SIZE: UVec2 = UVec2::new(24, 18);
pub const ISOMETRIC_SIZE: UVec2 = UVec2::new(12, 10);

#[derive(Component)]
pub struct OverlayText;

#[derive(Clone, Debug)]
pub struct DemoTileIds {
    pub grass: TileKindId,
    pub soil: TileKindId,
    pub crop: TileKindId,
    pub flower: TileKindId,
    pub rock: TileKindId,
    pub sand: TileKindId,
    pub wall: TileKindId,
    pub square_highlight: TileKindId,
    pub road: TileKindId,
    pub water: TileKindId,
    pub iso_grass: TileKindId,
    pub iso_stone: TileKindId,
    pub iso_accent: TileKindId,
    pub iso_highlight: TileKindId,
}

#[derive(Clone, Debug)]
pub struct DemoPalette {
    pub atlas: TileAtlasLayout,
    pub tiles: DemoTileIds,
}

impl DemoPalette {
    pub fn new(images: &mut Assets<Image>) -> Self {
        const TILE_SIZE: u32 = 24;
        const COLUMNS: u32 = 8;
        const ROWS: u32 = 4;
        const TEXTURE_WIDTH: u32 = TILE_SIZE * COLUMNS;
        const TEXTURE_HEIGHT: u32 = TILE_SIZE * ROWS;

        let mut image = Image::new_fill(
            Extent3d {
                width: TEXTURE_WIDTH,
                height: TEXTURE_HEIGHT,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );

        {
            let data = image.data.as_mut().expect("rgba atlas image");
            for atlas_index in 0..(COLUMNS * ROWS) {
                paint_tile(
                    data,
                    UVec2::new(TEXTURE_WIDTH, TEXTURE_HEIGHT),
                    TILE_SIZE,
                    atlas_index,
                );
            }
        }

        let atlas_handle = images.add(image);
        let atlas = TileAtlasLayout::from_grid(
            atlas_handle,
            UVec2::new(TEXTURE_WIDTH, TEXTURE_HEIGHT),
            UVec2::splat(TILE_SIZE),
            COLUMNS,
            ROWS,
        );

        Self {
            atlas,
            tiles: DemoTileIds {
                grass: TileKindId::new(1),
                soil: TileKindId::new(2),
                crop: TileKindId::new(3),
                flower: TileKindId::new(4),
                rock: TileKindId::new(5),
                sand: TileKindId::new(6),
                wall: TileKindId::new(7),
                square_highlight: TileKindId::new(8),
                road: TileKindId::new(9),
                water: TileKindId::new(10),
                iso_grass: TileKindId::new(11),
                iso_stone: TileKindId::new(12),
                iso_accent: TileKindId::new(13),
                iso_highlight: TileKindId::new(14),
            },
        }
    }

    pub fn catalog(&self) -> TileCatalog {
        let mut catalog = TileCatalog::default();

        catalog.insert_kind(self.tiles.grass, TileKind::static_tile("grass", 0));
        catalog.insert_kind(self.tiles.soil, TileKind::static_tile("soil", 1));
        catalog.insert_kind(self.tiles.crop, TileKind::static_tile("crop", 2));
        catalog.insert_kind(self.tiles.flower, TileKind::static_tile("flower", 3));
        catalog.insert_kind(
            self.tiles.rock,
            TileKind::static_tile("rock", 4)
                .with_collision(TileCollisionDescriptor::solid())
                .with_movement_cost(4),
        );
        catalog.insert_kind(
            self.tiles.sand,
            TileKind::static_tile("sand", 5).with_movement_cost(2),
        );
        catalog.insert_kind(
            self.tiles.wall,
            TileKind::static_tile("wall", 6)
                .with_collision(TileCollisionDescriptor::solid())
                .with_flags(0b0001),
        );
        catalog.insert_kind(
            self.tiles.square_highlight,
            TileKind::static_tile("square_highlight", 7),
        );
        catalog.insert_autotile_rule(AutotileRuleSetId::new(1), full_cardinal_rule_set(8));
        catalog.insert_kind(
            self.tiles.road,
            TileKind::autotile(
                "road",
                AutotileBinding {
                    group: AutotileGroupId::new(1),
                    rule_set: AutotileRuleSetId::new(1),
                    fallback_atlas_index: 8,
                },
            ),
        );
        catalog.insert_kind(
            self.tiles.water,
            TileKind::animated_tile("water", TileAnimation::uniform(24..=27, 0.18))
                .with_movement_cost(3),
        );
        catalog.insert_kind(self.tiles.iso_grass, TileKind::static_tile("iso_grass", 28));
        catalog.insert_kind(
            self.tiles.iso_stone,
            TileKind::static_tile("iso_stone", 29).with_movement_cost(3),
        );
        catalog.insert_kind(
            self.tiles.iso_accent,
            TileKind::static_tile("iso_accent", 30),
        );
        catalog.insert_kind(
            self.tiles.iso_highlight,
            TileKind::static_tile("iso_highlight", 31),
        );

        catalog
    }
}

pub fn build_square_showcase_map(palette: &DemoPalette) -> Tilemap {
    let geometry = TilemapGeometry::square(Vec2::splat(30.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(8));
    let catalog = palette.catalog();

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            GROUND_LAYER,
            "Ground",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(0.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            DETAIL_LAYER,
            "Detail",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(2.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::logic_only(COLLISION_LAYER, "Collision"),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            HIGHLIGHT_LAYER,
            "Highlight",
            TileLayerRenderConfig::new(palette.atlas.clone())
                .with_z_index(4.0)
                .with_tint(Color::srgba(1.0, 1.0, 1.0, 0.92)),
        ),
        catalog,
    ));

    for y in 0..SQUARE_SIZE.y as i32 {
        for x in 0..SQUARE_SIZE.x as i32 {
            let coord = TileCoord::new(x, y);
            let ground = if (4..8).contains(&x) && (11..=15).contains(&y) {
                palette.tiles.water
            } else if (14..19).contains(&x) && (11..=15).contains(&y) {
                palette.tiles.sand
            } else if (4..10).contains(&x) && (6..10).contains(&y) {
                palette.tiles.soil
            } else if ((x * 5 + y * 3) % 19) == 0 {
                palette.tiles.sand
            } else {
                palette.tiles.grass
            };
            map.set_tile(GROUND_LAYER, coord, TileCell::new(ground));

            if (4..10).contains(&x) && (6..10).contains(&y) && ((x + y) % 2 == 0) {
                map.set_tile(
                    DETAIL_LAYER,
                    coord,
                    TileCell::new(palette.tiles.crop).with_tint(Color::srgb(0.94, 1.0, 0.92)),
                );
            }
            if ((x * 11 + y * 7) % 29) == 0 {
                map.set_tile(
                    DETAIL_LAYER,
                    coord,
                    TileCell::new(palette.tiles.flower).with_tint(Color::srgb(1.0, 0.92, 0.95)),
                );
            }
        }
    }

    for coord in square_road_coords() {
        map.set_tile(DETAIL_LAYER, coord, TileCell::new(palette.tiles.road));
    }

    for coord in [
        TileCoord::new(16, 6),
        TileCoord::new(17, 6),
        TileCoord::new(18, 6),
        TileCoord::new(16, 7),
        TileCoord::new(18, 7),
        TileCoord::new(16, 8),
        TileCoord::new(17, 8),
        TileCoord::new(18, 8),
    ] {
        map.set_tile(DETAIL_LAYER, coord, TileCell::new(palette.tiles.rock));
        map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
    }

    map
}

pub fn build_isometric_battlefield_map(palette: &DemoPalette) -> Tilemap {
    let geometry = TilemapGeometry::isometric_diamond(Vec2::new(60.0, 32.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(6));
    let catalog = palette.catalog();

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            GROUND_LAYER,
            "Ground",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(0.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            DETAIL_LAYER,
            "Detail",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(2.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::logic_only(COLLISION_LAYER, "Collision"),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            HIGHLIGHT_LAYER,
            "Highlight",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(4.0),
        ),
        catalog,
    ));

    for y in 0..ISOMETRIC_SIZE.y as i32 {
        for x in 0..ISOMETRIC_SIZE.x as i32 {
            let coord = TileCoord::new(x, y);
            let on_ridge = (x >= 6 && y <= 3) || (x <= 2 && y >= 6) || ((x + y) % 7 == 0);
            let ground = if on_ridge {
                palette.tiles.iso_stone
            } else {
                palette.tiles.iso_grass
            };
            map.set_tile(GROUND_LAYER, coord, TileCell::new(ground));

            if (x + y) % 5 == 0 && !on_ridge {
                map.set_tile(
                    DETAIL_LAYER,
                    coord,
                    TileCell::new(palette.tiles.iso_accent).with_tint(Color::srgb(1.0, 0.95, 0.86)),
                );
            }

            if (x == 7 && y <= 3) || (x == 2 && y >= 6) {
                map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
            }
        }
    }

    map
}

pub fn build_large_map(palette: &DemoPalette, size: UVec2) -> Tilemap {
    let geometry = TilemapGeometry::square(Vec2::splat(20.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(12));
    let catalog = palette.catalog();

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            GROUND_LAYER,
            "Ground",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(0.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            DETAIL_LAYER,
            "Detail",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(1.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::logic_only(COLLISION_LAYER, "Collision"),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            HIGHLIGHT_LAYER,
            "Highlight",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(3.0),
        ),
        catalog,
    ));

    for y in 0..size.y as i32 {
        for x in 0..size.x as i32 {
            let coord = TileCoord::new(x, y);
            let hash = deterministic_hash(x, y);
            let base = if (x - y).abs() < 2 || (x + y) % 23 == 0 {
                palette.tiles.soil
            } else if hash % 11 == 0 {
                palette.tiles.sand
            } else {
                palette.tiles.grass
            };
            map.set_tile(GROUND_LAYER, coord, TileCell::new(base));

            if y % 16 == 7 || x % 19 == 9 {
                map.set_tile(DETAIL_LAYER, coord, TileCell::new(palette.tiles.road));
            }
            if hash % 41 == 0 {
                map.set_tile(
                    DETAIL_LAYER,
                    coord,
                    TileCell::new(palette.tiles.flower).with_tint(Color::srgb(0.95, 1.0, 0.92)),
                );
            }
        }
    }

    map
}

pub fn square_runtime_edit_coords() -> Vec<TileCoord> {
    vec![
        TileCoord::new(5, 12),
        TileCoord::new(6, 12),
        TileCoord::new(7, 12),
        TileCoord::new(8, 12),
        TileCoord::new(9, 12),
        TileCoord::new(10, 12),
        TileCoord::new(11, 12),
        TileCoord::new(12, 12),
        TileCoord::new(12, 11),
        TileCoord::new(12, 10),
    ]
}

pub fn map_local_center(map: &Tilemap, size: UVec2) -> Vec2 {
    let min = map.geometry.tile_to_local(TileCoord::ZERO);
    let max = map
        .geometry
        .tile_to_local(TileCoord::new(size.x as i32 - 1, size.y as i32 - 1));
    (min + max) * 0.5
}

pub fn spawn_camera(commands: &mut Commands, title: &str, translation: Vec3) {
    commands.spawn((
        Name::new(title.to_string()),
        Camera2d,
        Transform::from_translation(translation),
    ));
}

pub fn spawn_overlay(commands: &mut Commands, title: impl Into<String>) -> Entity {
    commands
        .spawn((
            Name::new("Overlay"),
            OverlayText,
            Text::new(title.into()),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: px(14.0),
                width: px(440.0),
                padding: UiRect::axes(px(12.0), px(10.0)),
                border_radius: BorderRadius::all(px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.90)),
        ))
        .id()
}

pub fn spawn_label(commands: &mut Commands, label: impl Into<String>, position: Vec3) -> Entity {
    commands
        .spawn((
            Name::new("Map Label"),
            Text2d::new(label.into()),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.96, 0.98)),
            Transform::from_translation(position),
        ))
        .id()
}

pub fn spawn_map(
    commands: &mut Commands,
    name: &str,
    map: Tilemap,
    translation: Vec3,
    debug_overlay: TilemapDebugOverlay,
) -> Entity {
    let entity = commands
        .spawn((TilemapBundle::new(name, map), debug_overlay))
        .id();
    commands
        .entity(entity)
        .insert(Transform::from_translation(translation));
    entity
}

pub fn cursor_world(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Vec2> {
    window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
}

fn full_cardinal_rule_set(base_index: u32) -> AutotileRuleSet {
    let mut rules = AutotileRuleSet::new(AutotileNeighborhood::Cardinal4, base_index);
    for mask in 0..16 {
        rules = rules.with_variant(mask, base_index + u32::from(mask));
    }
    rules
}

fn square_road_coords() -> Vec<TileCoord> {
    let mut coords = Vec::new();
    coords.extend((2..=20).map(|x| TileCoord::new(x, 4)));
    coords.extend((4..=13).map(|y| TileCoord::new(10, y)));
    coords.extend((10..=17).map(|x| TileCoord::new(x, 9)));
    coords
}

fn deterministic_hash(x: i32, y: i32) -> u64 {
    let x = x as i64;
    let y = y as i64;
    ((x * 73_856_093) ^ (y * 19_349_663)).unsigned_abs()
}

fn paint_tile(data: &mut [u8], texture_size: UVec2, tile_size: u32, atlas_index: u32) {
    let column = atlas_index % 8;
    let row = atlas_index / 8;
    let origin = UVec2::new(column * tile_size, row * tile_size);

    match atlas_index {
        0 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.19, 0.55, 0.27, 1.0),
            );
            scatter_dots(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.27, 0.71, 0.33, 1.0),
                3,
            );
        }
        1 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.42, 0.27, 0.16, 1.0),
            );
            for stripe in (3..tile_size - 2).step_by(5) {
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(0, stripe),
                    UVec2::new(tile_size, 2),
                    rgba(0.32, 0.19, 0.11, 1.0),
                );
            }
        }
        2 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.42, 0.27, 0.16, 1.0),
            );
            for x in [5, 11, 17] {
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(x, 9),
                    UVec2::new(2, 8),
                    rgba(0.26, 0.82, 0.36, 1.0),
                );
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(x.saturating_sub(2), 7),
                    UVec2::new(6, 3),
                    rgba(0.52, 0.96, 0.48, 1.0),
                );
            }
        }
        3 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.18, 0.53, 0.26, 1.0),
            );
            for (x, y) in [(6, 6), (14, 9), (10, 15), (17, 5)] {
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(x, y),
                    UVec2::splat(3),
                    rgba(0.98, 0.74, 0.88, 1.0),
                );
            }
        }
        4 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.20, 0.22, 0.24, 1.0),
            );
            fill_rect(
                data,
                texture_size,
                origin + UVec2::new(5, 5),
                UVec2::new(14, 14),
                rgba(0.46, 0.48, 0.52, 1.0),
            );
            outline_rect(
                data,
                texture_size,
                origin + UVec2::new(5, 5),
                UVec2::new(14, 14),
                rgba(0.67, 0.69, 0.73, 1.0),
            );
        }
        5 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.79, 0.69, 0.44, 1.0),
            );
            for stripe in (2..tile_size - 1).step_by(4) {
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(0, stripe),
                    UVec2::new(tile_size, 1),
                    rgba(0.90, 0.81, 0.58, 1.0),
                );
            }
        }
        6 => {
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.18, 0.20, 0.22, 1.0),
            );
            for row in [4, 10, 16] {
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(0, row),
                    UVec2::new(tile_size, 2),
                    rgba(0.32, 0.35, 0.38, 1.0),
                );
            }
            for column in [7, 15] {
                fill_rect(
                    data,
                    texture_size,
                    origin + UVec2::new(column, 0),
                    UVec2::new(2, tile_size),
                    rgba(0.29, 0.32, 0.35, 1.0),
                );
            }
        }
        7 => {
            outline_rect(
                data,
                texture_size,
                origin + UVec2::new(2, 2),
                UVec2::new(tile_size - 4, tile_size - 4),
                rgba(0.99, 0.82, 0.28, 0.96),
            );
            fill_rect(
                data,
                texture_size,
                origin + UVec2::new(tile_size / 2 - 1, tile_size / 2 - 1),
                UVec2::splat(3),
                rgba(1.0, 0.95, 0.52, 1.0),
            );
        }
        8..=23 => {
            let mask = atlas_index - 8;
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.19, 0.55, 0.27, 1.0),
            );
            draw_road_mask(data, texture_size, origin, tile_size, mask as u16);
        }
        24..=27 => {
            let phase = atlas_index - 24;
            fill_tile(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.13, 0.39, 0.79, 1.0),
            );
            for y in 0..tile_size {
                for x in 0..tile_size {
                    if (x + y / 2 + phase * 3) % 9 < 3 {
                        set_pixel(
                            data,
                            texture_size,
                            origin + UVec2::new(x, y),
                            rgba(0.70, 0.90, 1.0, 0.86),
                        );
                    }
                }
            }
        }
        28 => {
            fill_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.16, 0.58, 0.30, 1.0),
            );
            outline_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.30, 0.85, 0.42, 1.0),
            );
        }
        29 => {
            fill_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.43, 0.46, 0.51, 1.0),
            );
            outline_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.70, 0.74, 0.80, 1.0),
            );
        }
        30 => {
            fill_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.17, 0.54, 0.28, 1.0),
            );
            fill_rect(
                data,
                texture_size,
                origin + UVec2::new(tile_size / 2 - 2, tile_size / 2 - 8),
                UVec2::new(4, 10),
                rgba(0.98, 0.75, 0.24, 1.0),
            );
            fill_rect(
                data,
                texture_size,
                origin + UVec2::new(tile_size / 2 - 6, tile_size / 2 - 2),
                UVec2::new(12, 4),
                rgba(0.98, 0.75, 0.24, 1.0),
            );
        }
        31 => {
            outline_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.99, 0.82, 0.28, 1.0),
            );
            fill_diamond(
                data,
                texture_size,
                origin,
                tile_size,
                rgba(0.99, 0.82, 0.28, 0.18),
            );
        }
        _ => {}
    }
}

fn draw_road_mask(data: &mut [u8], texture_size: UVec2, origin: UVec2, tile_size: u32, mask: u16) {
    let road = rgba(0.47, 0.36, 0.19, 1.0);
    let edge = rgba(0.69, 0.57, 0.33, 1.0);
    let center = tile_size / 2;
    let road_width = 6;
    let half = road_width / 2;

    fill_rect(
        data,
        texture_size,
        origin + UVec2::new(center - half, center - half),
        UVec2::splat(road_width),
        road,
    );

    if mask & 0b0001 != 0 {
        fill_rect(
            data,
            texture_size,
            origin + UVec2::new(center - half, 0),
            UVec2::new(road_width, center),
            road,
        );
    }
    if mask & 0b0010 != 0 {
        fill_rect(
            data,
            texture_size,
            origin + UVec2::new(center, center - half),
            UVec2::new(center, road_width),
            road,
        );
    }
    if mask & 0b0100 != 0 {
        fill_rect(
            data,
            texture_size,
            origin + UVec2::new(center - half, center),
            UVec2::new(road_width, center),
            road,
        );
    }
    if mask & 0b1000 != 0 {
        fill_rect(
            data,
            texture_size,
            origin + UVec2::new(0, center - half),
            UVec2::new(center, road_width),
            road,
        );
    }

    outline_rect(
        data,
        texture_size,
        origin + UVec2::new(center - half, center - half),
        UVec2::splat(road_width),
        edge,
    );
}

fn fill_tile(data: &mut [u8], texture_size: UVec2, origin: UVec2, tile_size: u32, color: [u8; 4]) {
    fill_rect(data, texture_size, origin, UVec2::splat(tile_size), color);
}

fn fill_rect(data: &mut [u8], texture_size: UVec2, origin: UVec2, size: UVec2, color: [u8; 4]) {
    for y in origin.y..origin.y + size.y {
        for x in origin.x..origin.x + size.x {
            set_pixel(data, texture_size, UVec2::new(x, y), color);
        }
    }
}

fn outline_rect(data: &mut [u8], texture_size: UVec2, origin: UVec2, size: UVec2, color: [u8; 4]) {
    if size.x == 0 || size.y == 0 {
        return;
    }
    for x in origin.x..origin.x + size.x {
        set_pixel(data, texture_size, UVec2::new(x, origin.y), color);
        set_pixel(
            data,
            texture_size,
            UVec2::new(x, origin.y + size.y - 1),
            color,
        );
    }
    for y in origin.y..origin.y + size.y {
        set_pixel(data, texture_size, UVec2::new(origin.x, y), color);
        set_pixel(
            data,
            texture_size,
            UVec2::new(origin.x + size.x - 1, y),
            color,
        );
    }
}

fn scatter_dots(
    data: &mut [u8],
    texture_size: UVec2,
    origin: UVec2,
    tile_size: u32,
    color: [u8; 4],
    step: u32,
) {
    for y in (3..tile_size - 2).step_by(step as usize) {
        for x in (2..tile_size - 2).step_by((step + 2) as usize) {
            if ((x + y) % 5) == 0 {
                set_pixel(data, texture_size, origin + UVec2::new(x, y), color);
            }
        }
    }
}

fn fill_diamond(
    data: &mut [u8],
    texture_size: UVec2,
    origin: UVec2,
    tile_size: u32,
    color: [u8; 4],
) {
    let center = tile_size as i32 / 2;
    for y in 0..tile_size as i32 {
        for x in 0..tile_size as i32 {
            if (x - center).abs() + (y - center).abs() <= center - 1 {
                set_pixel(
                    data,
                    texture_size,
                    origin + UVec2::new(x as u32, y as u32),
                    color,
                );
            }
        }
    }
}

fn outline_diamond(
    data: &mut [u8],
    texture_size: UVec2,
    origin: UVec2,
    tile_size: u32,
    color: [u8; 4],
) {
    let center = tile_size as i32 / 2;
    for y in 0..tile_size as i32 {
        for x in 0..tile_size as i32 {
            let manhattan = (x - center).abs() + (y - center).abs();
            if (center - 2..=center - 1).contains(&manhattan) {
                set_pixel(
                    data,
                    texture_size,
                    origin + UVec2::new(x as u32, y as u32),
                    color,
                );
            }
        }
    }
}

fn set_pixel(data: &mut [u8], texture_size: UVec2, position: UVec2, color: [u8; 4]) {
    if position.x >= texture_size.x || position.y >= texture_size.y {
        return;
    }
    let index = ((position.y * texture_size.x + position.x) * 4) as usize;
    data[index..index + 4].copy_from_slice(&color);
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
