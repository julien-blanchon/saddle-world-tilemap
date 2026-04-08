#[cfg(feature = "e2e")]
mod e2e;

use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{TileCoord, TilemapCommand, TilemapDebugOverlay, TilemapPlugin};
use support::{DemoPalette, GROUND_LAYER, HIGHLIGHT_LAYER, ISOMETRIC_SIZE, OverlayText};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum IsometricSystems {
    Drive,
}

#[derive(Resource)]
struct IsoDemo {
    map: Entity,
    hovered: Option<TileCoord>,
    highlight_kind: saddle_world_tilemap::TileKindId,
}

#[derive(Resource, Default)]
struct IsoAutomation {
    hovered_override: Option<TileCoord>,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(IsoAutomation::default())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap isometric".into(),
                        resolution: (1380, 940).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(support::pane_plugins())
        .add_plugins(TilemapPlugin::default())
        .register_pane::<support::TilemapExamplePane>()
        .configure_sets(Update, IsometricSystems::Drive)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (support::sync_example_pane, update_pick).in_set(IsometricSystems::Drive),
        );
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::IsometricExampleE2EPlugin);
    app.run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = support::build_isometric_battlefield_map(&palette);
    let center = support::map_local_center(&map, ISOMETRIC_SIZE);

    support::spawn_camera(
        &mut commands,
        "Isometric Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Cursor picking runs through tile <-> world helpers on an isometric battlefield. Stone tiles report a higher movement cost than grass.",
    );
    support::spawn_label(
        &mut commands,
        "Isometric tactics board",
        Vec3::new(center.x, center.y + 240.0, 5.0),
    );

    let map = support::spawn_map(
        &mut commands,
        "Isometric Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(IsoDemo {
        map,
        hovered: None,
        highlight_kind: palette.tiles.iso_highlight,
    });
}

fn update_pick(
    windows: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut demo: ResMut<IsoDemo>,
    map_query: Query<(&saddle_world_tilemap::Tilemap, &GlobalTransform)>,
    automation: Res<IsoAutomation>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    let Ok((map, map_transform)) = map_query.get(demo.map) else {
        return;
    };
    let (camera, camera_transform) = *camera;

    let hovered = automation.hovered_override.or_else(|| {
        support::cursor_world(windows.into_inner(), camera, camera_transform)
            .and_then(|world| map.geometry.world_to_tile(map_transform, world))
            .filter(in_isometric_bounds)
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

    if let Some(coord) = hovered {
        let movement_cost = map
            .get_tile(GROUND_LAYER, coord)
            .and_then(|tile| map.layer(GROUND_LAYER)?.catalog.kind(tile.kind))
            .map_or(0, |kind| kind.movement_cost);

        overlay.0 = format!(
            "Cursor picking runs through tile <-> world helpers on an isometric battlefield. Stone tiles report a higher movement cost than grass.\nHovered tile: ({}, {})  chunk: ({}, {})\nMovement cost: {}",
            coord.x,
            coord.y,
            coord.chunk(map.chunk_size).x,
            coord.chunk(map.chunk_size).y,
            movement_cost,
        );
    } else {
        overlay.0 = "Cursor picking runs through tile <-> world helpers on an isometric battlefield. Stone tiles report a higher movement cost than grass.\nHovered tile: outside the battlefield".to_string();
    }
}

fn in_isometric_bounds(coord: &TileCoord) -> bool {
    coord.x >= 0
        && coord.y >= 0
        && coord.x < ISOMETRIC_SIZE.x as i32
        && coord.y < ISOMETRIC_SIZE.y as i32
}
