#[cfg(feature = "e2e")]
mod e2e;

use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{
    TileCoord, TilemapCommand, TilemapDebugOverlay, TilemapDebugSettings, TilemapPlugin,
};
use support::{DemoPalette, HIGHLIGHT_LAYER, OverlayText, SQUARE_SIZE};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum BasicSystems {
    Drive,
}

#[derive(Resource)]
struct BasicDemo {
    map: Entity,
    hovered: Option<TileCoord>,
    highlight_kind: saddle_world_tilemap::TileKindId,
}

#[derive(Resource, Default)]
struct BasicAutomation {
    hovered_override: Option<TileCoord>,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(support::TilemapExamplePane {
        debug_enabled: true,
        draw_chunk_bounds: true,
        ..default()
    })
    .insert_resource(BasicAutomation::default())
    .add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "tilemap basic".into(),
                    resolution: (1360, 920).into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(support::pane_plugins())
    .add_plugins(
        TilemapPlugin::default().with_debug_settings(TilemapDebugSettings {
            enabled: true,
            draw_dirty_chunks: false,
            ..default()
        }),
    )
    .register_pane::<support::TilemapExamplePane>()
    .configure_sets(Update, BasicSystems::Drive)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (support::sync_example_pane, update_hover).in_set(BasicSystems::Drive),
    );
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::BasicExampleE2EPlugin);
    app.run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = support::build_square_showcase_map(&palette);
    let center = support::map_local_center(&map, SQUARE_SIZE);

    support::spawn_camera(
        &mut commands,
        "Basic Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Square map authored from logic data with layered rendering, collision metadata, chunk gizmos, and cursor picking.",
    );
    support::spawn_label(
        &mut commands,
        "Top-down authored level",
        Vec3::new(center.x, center.y + 330.0, 5.0),
    );

    let map = support::spawn_map(
        &mut commands,
        "Basic Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(BasicDemo {
        map,
        hovered: None,
        highlight_kind: palette.tiles.square_highlight,
    });
}

fn update_hover(
    windows: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut demo: ResMut<BasicDemo>,
    map_query: Query<(
        &saddle_world_tilemap::Tilemap,
        &GlobalTransform,
        &saddle_world_tilemap::TilemapDiagnostics,
    )>,
    automation: Res<BasicAutomation>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    let Ok((map, map_transform, diagnostics)) = map_query.get(demo.map) else {
        return;
    };
    let (camera, camera_transform) = *camera;

    let hovered = automation.hovered_override.or_else(|| {
        support::cursor_world(windows.into_inner(), camera, camera_transform)
            .and_then(|world| map.geometry.world_to_tile(map_transform, world))
            .filter(in_square_bounds)
    });

    if hovered != demo.hovered {
        if let Some(previous) = demo.hovered {
            commands_out.write(TilemapCommand::ClearTile {
                map: demo.map,
                layer: HIGHLIGHT_LAYER,
                coord: previous,
            });
        }
        if let Some(next) = hovered {
            commands_out.write(TilemapCommand::SetTile {
                map: demo.map,
                layer: HIGHLIGHT_LAYER,
                coord: next,
                tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
            });
        }
        demo.hovered = hovered;
    }

    overlay.0 = if let Some(coord) = hovered {
        format!(
            "Square map authored from logic data with layered rendering, collision metadata, chunk gizmos, and cursor picking.\nHovered tile: ({}, {})  chunk: ({}, {})\nChunks: {}  collision chunks: {}  last-frame rebuilds: {}",
            coord.x,
            coord.y,
            coord.chunk(map.chunk_size).x,
            coord.chunk(map.chunk_size).y,
            diagnostics.logical_chunks_total,
            diagnostics.collision_chunks_total,
            diagnostics.chunks_rebuilt_this_frame,
        )
    } else {
        format!(
            "Square map authored from logic data with layered rendering, collision metadata, chunk gizmos, and cursor picking.\nHovered tile: outside the authored bounds\nChunks: {}  collision chunks: {}  last-frame rebuilds: {}",
            diagnostics.logical_chunks_total,
            diagnostics.collision_chunks_total,
            diagnostics.chunks_rebuilt_this_frame,
        )
    };
}

fn in_square_bounds(coord: &TileCoord) -> bool {
    coord.x >= 0 && coord.y >= 0 && coord.x < SQUARE_SIZE.x as i32 && coord.y < SQUARE_SIZE.y as i32
}
