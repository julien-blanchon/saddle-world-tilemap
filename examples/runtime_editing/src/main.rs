use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use support::{
    COLLISION_LAYER, DETAIL_LAYER, DemoPalette, GROUND_LAYER, HIGHLIGHT_LAYER, OverlayText,
    SQUARE_SIZE,
};
use saddle_world_tilemap::{TileCoord, TileRect, TilemapCommand, TilemapDebugOverlay, TilemapPlugin};

#[derive(Resource)]
struct RuntimeEditingDemo {
    map: Entity,
    timer: Timer,
    phase: usize,
    highlighted: Vec<TileCoord>,
    swap_pairs: Vec<(TileCoord, TileCoord)>,
    crop_kind: saddle_world_tilemap::TileKindId,
    grass_kind: saddle_world_tilemap::TileKindId,
    soil_kind: saddle_world_tilemap::TileKindId,
    road_kind: saddle_world_tilemap::TileKindId,
    wall_kind: saddle_world_tilemap::TileKindId,
    highlight_kind: saddle_world_tilemap::TileKindId,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap runtime editing".into(),
                        resolution: (1360, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(TilemapPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (run_edit_cycle, update_overlay))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = support::build_square_showcase_map(&palette);
    let center = support::map_local_center(&map, SQUARE_SIZE);

    support::spawn_camera(
        &mut commands,
        "Runtime Editing Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "This loop alternates between fill, set, clear, and collision-only edits. The public API stays message-driven the whole time.",
    );
    support::spawn_label(
        &mut commands,
        "Runtime editing cycle",
        Vec3::new(center.x, center.y + 330.0, 5.0),
    );

    let map = support::spawn_map(
        &mut commands,
        "Runtime Editing Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(RuntimeEditingDemo {
        map,
        timer: Timer::from_seconds(0.95, TimerMode::Repeating),
        phase: 0,
        highlighted: Vec::new(),
        swap_pairs: vec![
            (TileCoord::new(13, 11), TileCoord::new(10, 4)),
            (TileCoord::new(14, 11), TileCoord::new(11, 4)),
            (TileCoord::new(14, 12), TileCoord::new(12, 4)),
            (TileCoord::new(15, 12), TileCoord::new(13, 4)),
        ],
        crop_kind: palette.tiles.crop,
        grass_kind: palette.tiles.grass,
        soil_kind: palette.tiles.soil,
        road_kind: palette.tiles.road,
        wall_kind: palette.tiles.wall,
        highlight_kind: palette.tiles.square_highlight,
    });
}

fn run_edit_cycle(
    time: Res<Time>,
    mut demo: ResMut<RuntimeEditingDemo>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    let map = demo.map;
    for coord in demo.highlighted.drain(..) {
        commands_out.write(TilemapCommand::ClearTile {
            map,
            layer: HIGHLIGHT_LAYER,
            coord,
        });
    }

    match demo.phase % 4 {
        0 => {
            let rect = TileRect::new(TileCoord::new(13, 11), UVec2::new(4, 3));
            commands_out.write(TilemapCommand::FillRect {
                map: demo.map,
                layer: GROUND_LAYER,
                rect,
                tile: saddle_world_tilemap::TileCell::new(demo.soil_kind),
            });
            for coord in rect.iter() {
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: HIGHLIGHT_LAYER,
                    coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
                });
                demo.highlighted.push(coord);
            }
        }
        1 => {
            for coord in [
                TileCoord::new(13, 11),
                TileCoord::new(14, 11),
                TileCoord::new(14, 12),
                TileCoord::new(15, 12),
            ] {
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: DETAIL_LAYER,
                    coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.crop_kind),
                });
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: HIGHLIGHT_LAYER,
                    coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
                });
                demo.highlighted.push(coord);
            }
        }
        2 => {
            for (patch_coord, partner_coord) in demo.swap_pairs.clone() {
                commands_out.write(TilemapCommand::SwapTiles {
                    map,
                    layer: DETAIL_LAYER,
                    a: patch_coord,
                    b: partner_coord,
                });
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: HIGHLIGHT_LAYER,
                    coord: patch_coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
                });
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: HIGHLIGHT_LAYER,
                    coord: partner_coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
                });
                demo.highlighted.push(patch_coord);
                demo.highlighted.push(partner_coord);
            }
        }
        _ => {
            let rect = TileRect::new(TileCoord::new(13, 11), UVec2::new(4, 3));
            commands_out.write(TilemapCommand::FillRect {
                map: demo.map,
                layer: GROUND_LAYER,
                rect,
                tile: saddle_world_tilemap::TileCell::new(demo.grass_kind),
            });
            for coord in rect.iter() {
                commands_out.write(TilemapCommand::ClearTile {
                    map,
                    layer: DETAIL_LAYER,
                    coord,
                });
            }
            for (_, partner_coord) in &demo.swap_pairs {
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: DETAIL_LAYER,
                    coord: *partner_coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.road_kind),
                });
            }
            for coord in [
                TileCoord::new(16, 6),
                TileCoord::new(17, 6),
                TileCoord::new(18, 6),
            ] {
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: COLLISION_LAYER,
                    coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.wall_kind),
                });
                commands_out.write(TilemapCommand::SetTile {
                    map,
                    layer: HIGHLIGHT_LAYER,
                    coord,
                    tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
                });
                demo.highlighted.push(coord);
            }
        }
    }

    demo.phase += 1;
}

fn update_overlay(
    demo: Res<RuntimeEditingDemo>,
    diagnostics: Single<&saddle_world_tilemap::TilemapDiagnostics, With<saddle_world_tilemap::TilemapRoot>>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    let phase_label = match demo.phase % 4 {
        1 => "fill ground patch",
        2 => "plant crop accents",
        3 => "swap crops into a road branch",
        _ => "reset visuals and touch collision-only cells",
    };

    overlay.0 = format!(
        "This loop alternates between fill, set, clear, and collision-only edits. The public API stays message-driven the whole time.\nCurrent phase: {}\nTile edits this frame: {}  dirty chunks: {}  collision chunks: {}",
        phase_label,
        diagnostics.tile_edits_this_frame,
        diagnostics.dirty_chunks,
        diagnostics.collision_chunks_total,
    );
}
