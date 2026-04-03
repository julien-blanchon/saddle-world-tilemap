use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{TileCoord, TilemapCommand, TilemapDebugOverlay, TilemapPlugin};
use support::{DETAIL_LAYER, DemoPalette, HIGHLIGHT_LAYER, OverlayText, SQUARE_SIZE};

#[derive(Resource)]
struct AutotileDemo {
    map: Entity,
    timer: Timer,
    next_index: usize,
    latest_coord: Option<TileCoord>,
    expansion: Vec<TileCoord>,
    highlight_kind: saddle_world_tilemap::TileKindId,
    road_kind: saddle_world_tilemap::TileKindId,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap autotiling".into(),
                        resolution: (1360, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(support::pane_plugins())
        .add_plugins(TilemapPlugin::default())
        .register_pane::<support::TilemapExamplePane>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (support::sync_example_pane, grow_roads, update_overlay),
        )
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = support::build_square_showcase_map(&palette);
    let center = support::map_local_center(&map, SQUARE_SIZE);

    support::spawn_camera(
        &mut commands,
        "Autotiling Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "A timer grows a new road branch. Each added tile triggers local autotile resolution and rebuilds only the affected chunks.",
    );
    support::spawn_label(
        &mut commands,
        "Autotile growth",
        Vec3::new(center.x, center.y + 330.0, 5.0),
    );

    let map = support::spawn_map(
        &mut commands,
        "Autotile Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(AutotileDemo {
        map,
        timer: Timer::from_seconds(0.45, TimerMode::Repeating),
        next_index: 0,
        latest_coord: None,
        expansion: support::square_runtime_edit_coords(),
        highlight_kind: palette.tiles.square_highlight,
        road_kind: palette.tiles.road,
    });
}

fn grow_roads(
    time: Res<Time>,
    mut demo: ResMut<AutotileDemo>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    if !demo.timer.tick(time.delta()).just_finished() || demo.next_index >= demo.expansion.len() {
        return;
    }

    let coord = demo.expansion[demo.next_index];
    demo.next_index += 1;

    commands_out.write(TilemapCommand::SetTile {
        map: demo.map,
        layer: DETAIL_LAYER,
        coord,
        tile: saddle_world_tilemap::TileCell::new(demo.road_kind),
    });

    if let Some(previous) = demo.latest_coord {
        commands_out.write(TilemapCommand::ClearTile {
            map: demo.map,
            layer: HIGHLIGHT_LAYER,
            coord: previous,
        });
    }
    commands_out.write(TilemapCommand::SetTile {
        map: demo.map,
        layer: HIGHLIGHT_LAYER,
        coord,
        tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
    });
    demo.latest_coord = Some(coord);
}

fn update_overlay(
    demo: Res<AutotileDemo>,
    diagnostics: Single<
        &saddle_world_tilemap::TilemapDiagnostics,
        With<saddle_world_tilemap::TilemapRoot>,
    >,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    overlay.0 = if let Some(coord) = demo.latest_coord {
        format!(
            "A timer grows a new road branch. Each added tile triggers local autotile resolution and rebuilds only the affected chunks.\nLatest road tile: ({}, {})  step: {}/{}\nLast-frame rebuilds: {}  dirty chunks: {}",
            coord.x,
            coord.y,
            demo.next_index,
            demo.expansion.len(),
            diagnostics.chunks_rebuilt_this_frame,
            diagnostics.dirty_chunks,
        )
    } else {
        format!(
            "A timer grows a new road branch. Each added tile triggers local autotile resolution and rebuilds only the affected chunks.\nWaiting for the first growth tick\nLast-frame rebuilds: {}  dirty chunks: {}",
            diagnostics.chunks_rebuilt_this_frame, diagnostics.dirty_chunks,
        )
    };
}
