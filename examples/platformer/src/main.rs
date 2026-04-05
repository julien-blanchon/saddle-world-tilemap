use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{
    TileCell, TileCoord, TilemapCommand, TilemapDebugOverlay, TilemapDebugSettings, TilemapPlugin,
};
use support::{COLLISION_LAYER, DemoPalette, GROUND_LAYER, HIGHLIGHT_LAYER, OverlayText};

const LEVEL_WIDTH: i32 = 40;
const LEVEL_HEIGHT: i32 = 22;

#[derive(Resource)]
struct PlatformerDemo {
    map: Entity,
    player_coord: TileCoord,
    palette: DemoPalette,
    velocity_y: f32,
    grounded: bool,
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
                        title: "tilemap platformer — WASD/arrows + Space to jump".into(),
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
        .add_systems(
            Update,
            (support::sync_example_pane, player_movement, update_overlay),
        )
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = build_platformer_level(&palette);

    support::spawn_camera(
        &mut commands,
        "Platformer Camera",
        Vec3::new(600.0, -280.0, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Side-scrolling platformer — Arrow keys or WASD to move, Space to jump. Collision layer blocks movement.",
    );

    let map_entity = support::spawn_map(
        &mut commands,
        "Platformer Level",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    let start = TileCoord::new(3, 17);

    commands.insert_resource(PlatformerDemo {
        map: map_entity,
        player_coord: start,
        palette,
        velocity_y: 0.0,
        grounded: false,
    });
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut demo: ResMut<PlatformerDemo>,
    map_query: Query<&saddle_world_tilemap::Tilemap>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    let Ok(map) = map_query.get(demo.map) else {
        return;
    };

    let gravity = 15.0;
    let jump_speed = -6.0;

    let mut dx = 0;
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        dx = 1;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        dx = -1;
    }

    let below = demo.player_coord.offset(0, 1);
    let is_solid_below = is_solid_tile(map, below);
    demo.grounded = is_solid_below;

    if keys.just_pressed(KeyCode::Space) && demo.grounded {
        demo.velocity_y = jump_speed;
    }

    demo.velocity_y += gravity * time.delta_secs();
    let dy = if demo.velocity_y > 1.0 {
        demo.velocity_y = 0.0;
        1
    } else if demo.velocity_y < -1.0 {
        demo.velocity_y = 0.0;
        -1
    } else {
        0
    };

    let old = demo.player_coord;
    let mut new_coord = old;

    if dx != 0 {
        let candidate = old.offset(dx, 0);
        if !is_solid_tile(map, candidate) && in_level_bounds(&candidate) {
            new_coord.x = candidate.x;
        }
    }

    if dy != 0 {
        let candidate = new_coord.offset(0, dy);
        if !is_solid_tile(map, candidate) && in_level_bounds(&candidate) {
            new_coord.y = candidate.y;
        } else {
            demo.velocity_y = 0.0;
        }
    }

    if new_coord != old {
        commands_out.write(TilemapCommand::ClearTile {
            map: demo.map,
            layer: HIGHLIGHT_LAYER,
            coord: old,
        });
        commands_out.write(TilemapCommand::SetTile {
            map: demo.map,
            layer: HIGHLIGHT_LAYER,
            coord: new_coord,
            tile: TileCell::new(demo.palette.tiles.square_highlight),
        });
        demo.player_coord = new_coord;
    }
}

fn update_overlay(
    demo: Res<PlatformerDemo>,
    map_query: Query<&saddle_world_tilemap::TilemapDiagnostics>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    let chunks = map_query
        .get(demo.map)
        .map(|d| d.logical_chunks_total)
        .unwrap_or(0);

    overlay.0 = format!(
        "Platformer — Arrows/WASD to move, Space to jump\n\
        Player: ({}, {})  Grounded: {}  Chunks: {}",
        demo.player_coord.x, demo.player_coord.y, demo.grounded, chunks,
    );
}

fn is_solid_tile(map: &saddle_world_tilemap::Tilemap, coord: TileCoord) -> bool {
    map.get_tile(COLLISION_LAYER, coord).is_some()
}

fn in_level_bounds(coord: &TileCoord) -> bool {
    coord.x >= 0 && coord.y >= 0 && coord.x < LEVEL_WIDTH && coord.y < LEVEL_HEIGHT
}

fn build_platformer_level(palette: &DemoPalette) -> saddle_world_tilemap::Tilemap {
    use saddle_world_tilemap::*;

    let geometry = TilemapGeometry::square(Vec2::splat(30.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(8));
    let catalog = palette.catalog();

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            GROUND_LAYER,
            "Background",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(0.0),
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
            "Player",
            TileLayerRenderConfig::new(palette.atlas.clone())
                .with_z_index(4.0)
                .with_tint(Color::srgba(1.0, 0.85, 0.2, 0.95)),
        ),
        catalog,
    ));

    for y in 0..LEVEL_HEIGHT {
        for x in 0..LEVEL_WIDTH {
            let coord = TileCoord::new(x, y);
            let sky = palette.tiles.grass;
            map.set_tile(
                GROUND_LAYER,
                coord,
                TileCell::new(sky).with_tint(Color::srgb(0.55, 0.75, 0.95)),
            );
        }
    }

    let floor_y = LEVEL_HEIGHT - 2;
    for x in 0..LEVEL_WIDTH {
        let coord = TileCoord::new(x, floor_y);
        map.set_tile(GROUND_LAYER, coord, TileCell::new(palette.tiles.soil));
        map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
        let under = TileCoord::new(x, floor_y + 1);
        map.set_tile(GROUND_LAYER, under, TileCell::new(palette.tiles.rock));
        map.set_tile(COLLISION_LAYER, under, TileCell::new(palette.tiles.wall));
    }

    for x in 5..=8 {
        map.clear_tile(GROUND_LAYER, TileCoord::new(x, floor_y));
        map.clear_tile(COLLISION_LAYER, TileCoord::new(x, floor_y));
        map.set_tile(
            GROUND_LAYER,
            TileCoord::new(x, floor_y),
            TileCell::new(palette.tiles.water),
        );
    }

    let platforms = [
        (8, 15, 5),
        (16, 12, 4),
        (23, 14, 6),
        (32, 10, 4),
        (28, 7, 3),
        (20, 8, 4),
        (12, 6, 5),
    ];

    for (px, py, pw) in platforms {
        for dx in 0..pw {
            let coord = TileCoord::new(px + dx, py);
            map.set_tile(GROUND_LAYER, coord, TileCell::new(palette.tiles.soil));
            map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
        }
    }

    for y in 0..LEVEL_HEIGHT {
        let left = TileCoord::new(0, y);
        let right = TileCoord::new(LEVEL_WIDTH - 1, y);
        map.set_tile(GROUND_LAYER, left, TileCell::new(palette.tiles.wall));
        map.set_tile(COLLISION_LAYER, left, TileCell::new(palette.tiles.wall));
        map.set_tile(GROUND_LAYER, right, TileCell::new(palette.tiles.wall));
        map.set_tile(COLLISION_LAYER, right, TileCell::new(palette.tiles.wall));
    }
    for x in 0..LEVEL_WIDTH {
        let top = TileCoord::new(x, 0);
        map.set_tile(GROUND_LAYER, top, TileCell::new(palette.tiles.wall));
        map.set_tile(COLLISION_LAYER, top, TileCell::new(palette.tiles.wall));
    }

    let start = TileCoord::new(3, floor_y - 1);
    map.set_tile(
        HIGHLIGHT_LAYER,
        start,
        TileCell::new(palette.tiles.square_highlight),
    );

    map
}
