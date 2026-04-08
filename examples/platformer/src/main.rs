#[cfg(feature = "e2e")]
mod e2e;

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
    horizontal_remainder: f32,
    velocity_y: f32,
    vertical_remainder: f32,
    grounded: bool,
}

#[derive(Resource, Default)]
struct PlatformerAutomation {
    horizontal_axis: i32,
    jump_requested: bool,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PlatformerSystems {
    Drive,
}

fn main() {
    let mut app = App::new();

    app.insert_resource(support::TilemapExamplePane {
            debug_enabled: false,
            draw_chunk_bounds: false,
            ..default()
        })
        .insert_resource(PlatformerAutomation::default())
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
        .configure_sets(Update, PlatformerSystems::Drive)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                support::sync_example_pane,
                player_movement,
                update_overlay,
            )
                .chain()
                .in_set(PlatformerSystems::Drive),
        );
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::PlatformerExampleE2EPlugin);

    app.run();
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

    let start = TileCoord::new(3, LEVEL_HEIGHT - 3);

    commands.insert_resource(PlatformerDemo {
        map: map_entity,
        player_coord: start,
        palette,
        horizontal_remainder: 0.0,
        velocity_y: 0.0,
        vertical_remainder: 0.0,
        grounded: false,
    });
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut automation: ResMut<PlatformerAutomation>,
    mut demo: ResMut<PlatformerDemo>,
    map_query: Query<&saddle_world_tilemap::Tilemap>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    let Ok(map) = map_query.get(demo.map) else {
        return;
    };

    let gravity = 28.0;
    let horizontal_speed = 8.0;
    let jump_speed = -18.0;

    let keyboard_axis = (keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD)) as i32
        - (keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA)) as i32;
    let horizontal_axis = if automation.horizontal_axis != 0 {
        automation.horizontal_axis.clamp(-1, 1)
    } else {
        keyboard_axis
    };
    let jump_requested = automation.jump_requested || keys.just_pressed(KeyCode::Space);
    automation.jump_requested = false;

    let below = demo.player_coord.offset(0, 1);
    let is_solid_below = is_solid_tile(map, below);
    let mut grounded = is_solid_below;
    let mut velocity_y = demo.velocity_y;
    let mut vertical_remainder = demo.vertical_remainder;

    if grounded && velocity_y > 0.0 {
        velocity_y = 0.0;
        vertical_remainder = 0.0;
    }

    if jump_requested && grounded {
        velocity_y = jump_speed;
        vertical_remainder = 0.0;
        grounded = false;
    }

    demo.horizontal_remainder += horizontal_axis as f32 * horizontal_speed * time.delta_secs();
    velocity_y += gravity * time.delta_secs();
    vertical_remainder += velocity_y * time.delta_secs();

    let old = demo.player_coord;
    let mut new_coord = old;

    apply_axis_steps(
        &mut new_coord,
        take_whole_steps(&mut demo.horizontal_remainder),
        IVec2::X,
        map,
        &mut demo.horizontal_remainder,
        None,
    );
    apply_axis_steps(
        &mut new_coord,
        take_whole_steps(&mut vertical_remainder),
        IVec2::Y,
        map,
        &mut vertical_remainder,
        Some((&mut velocity_y, &mut grounded)),
    );
    demo.player_coord = new_coord;
    demo.grounded = is_solid_tile(map, demo.player_coord.offset(0, 1));
    demo.velocity_y = velocity_y;
    demo.vertical_remainder = vertical_remainder;

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

fn take_whole_steps(remainder: &mut f32) -> i32 {
    let mut steps = 0;

    while *remainder >= 1.0 {
        *remainder -= 1.0;
        steps += 1;
    }
    while *remainder <= -1.0 {
        *remainder += 1.0;
        steps -= 1;
    }

    steps
}

fn apply_axis_steps(
    coord: &mut TileCoord,
    steps: i32,
    axis: IVec2,
    map: &saddle_world_tilemap::Tilemap,
    remainder: &mut f32,
    vertical_state: Option<(&mut f32, &mut bool)>,
) {
    if steps == 0 {
        return;
    }

    let direction = steps.signum();
    let mut vertical_state = vertical_state;
    for _ in 0..steps.unsigned_abs() {
        let candidate = coord.offset(axis.x * direction, axis.y * direction);
        if !in_level_bounds(&candidate) || is_solid_tile(map, candidate) {
            *remainder = 0.0;
            if let Some((velocity_y, grounded)) = &mut vertical_state {
                **velocity_y = 0.0;
                if direction > 0 {
                    **grounded = true;
                }
            }
            break;
        }

        *coord = candidate;
    }
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
