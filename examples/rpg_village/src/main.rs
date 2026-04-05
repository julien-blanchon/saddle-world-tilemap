use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{
    TileCell, TileCoord, TilePathOptions, TilemapCommand, TilemapDebugOverlay,
    TilemapDebugSettings, TilemapPlugin, find_path,
};
use support::{COLLISION_LAYER, DemoPalette, GROUND_LAYER, HIGHLIGHT_LAYER, OverlayText};

const VILLAGE_SIZE: UVec2 = UVec2::new(32, 24);

#[derive(Resource)]
struct VillageDemo {
    map: Entity,
    palette: DemoPalette,
    path_start: Option<TileCoord>,
    path_end: Option<TileCoord>,
    current_path: Vec<TileCoord>,
    npc_positions: Vec<TileCoord>,
}

fn main() {
    App::new()
        .insert_resource(support::TilemapExamplePane {
            debug_enabled: false,
            draw_chunk_bounds: false,
            ..default()
        })
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap RPG village — click to pathfind".into(),
                        resolution: (1360, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(support::pane_plugins())
        .add_plugins(
            TilemapPlugin::default().with_debug_settings(TilemapDebugSettings {
                enabled: false,
                ..default()
            }),
        )
        .register_pane::<support::TilemapExamplePane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (support::sync_example_pane, update_pathfinding))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = build_village_map(&palette);
    let center = support::map_local_center(&map, VILLAGE_SIZE);

    support::spawn_camera(
        &mut commands,
        "Village Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "RPG Village — Left-click to set path start, Right-click to set path end. A* pathfinding avoids walls and prefers roads.",
    );

    let npc_positions = vec![
        TileCoord::new(8, 6),
        TileCoord::new(20, 14),
        TileCoord::new(14, 10),
        TileCoord::new(25, 8),
    ];

    let map_entity = support::spawn_map(
        &mut commands,
        "RPG Village Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(VillageDemo {
        map: map_entity,
        palette,
        path_start: None,
        path_end: None,
        current_path: Vec::new(),
        npc_positions,
    });
}

fn update_pathfinding(
    windows: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut demo: ResMut<VillageDemo>,
    map_query: Query<(
        &saddle_world_tilemap::Tilemap,
        &GlobalTransform,
        &saddle_world_tilemap::TilemapDiagnostics,
    )>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    let Ok((map, map_transform, diagnostics)) = map_query.get(demo.map) else {
        return;
    };
    let (camera, camera_transform) = *camera;

    let hovered = support::cursor_world(windows.into_inner(), camera, camera_transform)
        .and_then(|world| map.geometry.world_to_tile(map_transform, world))
        .filter(in_village_bounds);

    if buttons.just_pressed(MouseButton::Left) {
        if let Some(coord) = hovered {
            demo.path_start = Some(coord);
        }
    }

    if buttons.just_pressed(MouseButton::Right) {
        if let Some(coord) = hovered {
            demo.path_end = Some(coord);
        }
    }

    let should_recompute =
        buttons.just_pressed(MouseButton::Left) || buttons.just_pressed(MouseButton::Right);

    if should_recompute {
        for coord in &demo.current_path {
            commands_out.write(TilemapCommand::ClearTile {
                map: demo.map,
                layer: HIGHLIGHT_LAYER,
                coord: *coord,
            });
        }
        demo.current_path.clear();

        if let (Some(start), Some(end)) = (demo.path_start, demo.path_end) {
            let options = TilePathOptions::default().with_diagonal(false);
            if let Some(result) = find_path(map, GROUND_LAYER, start, end, &options) {
                for coord in &result.path {
                    commands_out.write(TilemapCommand::SetTile {
                        map: demo.map,
                        layer: HIGHLIGHT_LAYER,
                        coord: *coord,
                        tile: TileCell::new(demo.palette.tiles.square_highlight),
                    });
                }
                demo.current_path = result.path;
            }
        }
    }

    let start_str = demo
        .path_start
        .map(|c| format!("({}, {})", c.x, c.y))
        .unwrap_or_else(|| "none".to_string());
    let end_str = demo
        .path_end
        .map(|c| format!("({}, {})", c.x, c.y))
        .unwrap_or_else(|| "none".to_string());
    let hover_str = hovered
        .map(|c| format!("({}, {})", c.x, c.y))
        .unwrap_or_else(|| "outside".to_string());

    overlay.0 = format!(
        "RPG Village — Left-click: path start, Right-click: path end\n\
        Hover: {}  Start: {}  End: {}\n\
        Path length: {} tiles  Chunks: {}\n\
        NPCs: {} wandering the village",
        hover_str,
        start_str,
        end_str,
        demo.current_path.len(),
        diagnostics.logical_chunks_total,
        demo.npc_positions.len(),
    );
}

fn in_village_bounds(coord: &TileCoord) -> bool {
    coord.x >= 0
        && coord.y >= 0
        && coord.x < VILLAGE_SIZE.x as i32
        && coord.y < VILLAGE_SIZE.y as i32
}

fn build_village_map(palette: &DemoPalette) -> saddle_world_tilemap::Tilemap {
    use saddle_world_tilemap::*;

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
            support::DETAIL_LAYER,
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
                .with_tint(Color::srgba(1.0, 0.9, 0.3, 0.85)),
        ),
        catalog,
    ));

    for y in 0..VILLAGE_SIZE.y as i32 {
        for x in 0..VILLAGE_SIZE.x as i32 {
            let coord = TileCoord::new(x, y);
            let hash = deterministic_hash(x, y);
            let ground = if is_water(x, y) {
                palette.tiles.water
            } else if is_sand_shore(x, y) {
                palette.tiles.sand
            } else if hash.is_multiple_of(17) {
                palette.tiles.soil
            } else {
                palette.tiles.grass
            };
            map.set_tile(GROUND_LAYER, coord, TileCell::new(ground));

            if hash.is_multiple_of(23) && !is_water(x, y) && !is_building(x, y) {
                map.set_tile(
                    support::DETAIL_LAYER,
                    coord,
                    TileCell::new(palette.tiles.flower).with_tint(Color::srgb(1.0, 0.92, 0.95)),
                );
            }
        }
    }

    for coord in village_road_coords() {
        map.set_tile(
            support::DETAIL_LAYER,
            coord,
            TileCell::new(palette.tiles.road),
        );
    }

    for (bx, by, bw, bh) in village_buildings() {
        for dy in 0..bh {
            for dx in 0..bw {
                let coord = TileCoord::new(bx + dx, by + dy);
                let is_edge = dx == 0 || dy == 0 || dx == bw - 1 || dy == bh - 1;
                if is_edge {
                    map.set_tile(
                        support::DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.wall),
                    );
                    map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
                } else {
                    map.set_tile(
                        support::DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.soil),
                    );
                }
            }
        }
        let door = TileCoord::new(bx + bw / 2, by + bh - 1);
        map.clear_tile(COLLISION_LAYER, door);
        map.set_tile(
            support::DETAIL_LAYER,
            door,
            TileCell::new(palette.tiles.sand),
        );
    }

    for coord in village_fence_coords() {
        map.set_tile(
            support::DETAIL_LAYER,
            coord,
            TileCell::new(palette.tiles.rock),
        );
        map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
    }

    map
}

fn village_buildings() -> Vec<(i32, i32, i32, i32)> {
    vec![
        (4, 3, 5, 4),
        (14, 2, 6, 5),
        (24, 4, 4, 4),
        (6, 14, 5, 4),
        (18, 12, 4, 5),
        (26, 15, 5, 4),
    ]
}

fn is_building(x: i32, y: i32) -> bool {
    village_buildings()
        .iter()
        .any(|(bx, by, bw, bh)| x >= *bx && x < bx + bw && y >= *by && y < by + bh)
}

fn is_water(x: i32, y: i32) -> bool {
    ((0..=2).contains(&x) && (18..=23).contains(&y))
        || ((0..=4).contains(&x) && (20..=23).contains(&y))
        || ((28..=31).contains(&x) && (0..=4).contains(&y))
}

fn is_sand_shore(x: i32, y: i32) -> bool {
    if is_water(x, y) {
        return false;
    }
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        if is_water(x + dx, y + dy) {
            return true;
        }
    }
    false
}

fn village_road_coords() -> Vec<TileCoord> {
    let mut coords = Vec::new();
    coords.extend((1..=30).map(|x| TileCoord::new(x, 9)));
    coords.extend((1..=22).map(|y| TileCoord::new(12, y)));
    coords.extend((1..=22).map(|y| TileCoord::new(22, y)));
    coords.extend((12..=22).map(|x| TileCoord::new(x, 19)));
    coords
}

fn village_fence_coords() -> Vec<TileCoord> {
    let mut coords = Vec::new();
    for x in 10..=16 {
        coords.push(TileCoord::new(x, 7));
        coords.push(TileCoord::new(x, 11));
    }
    for y in 7..=11 {
        coords.push(TileCoord::new(10, y));
        coords.push(TileCoord::new(16, y));
    }
    coords.push(TileCoord::new(13, 11));
    coords.retain(|c| c.x != 13 || c.y != 11);
    coords
}

fn deterministic_hash(x: i32, y: i32) -> u64 {
    let x = x as i64;
    let y = y as i64;
    ((x * 73_856_093) ^ (y * 19_349_663)).unsigned_abs()
}
