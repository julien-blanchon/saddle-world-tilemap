use std::collections::HashSet;

use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_hex_grid::{AxialHex, HexPath, a_star};
use saddle_world_tilemap::{TileCoord, TilemapCommand, TilemapDebugOverlay, TilemapPlugin};
use support::{COLLISION_LAYER, DETAIL_LAYER, GROUND_LAYER, HIGHLIGHT_LAYER, OverlayText};

#[derive(Resource)]
struct HexStrategyDemo {
    map: Entity,
    board: HashSet<AxialHex>,
    start: AxialHex,
    hovered: Option<AxialHex>,
    highlighted: Vec<TileCoord>,
    blocked: HashSet<AxialHex>,
    highlight_kind: saddle_world_tilemap::TileKindId,
}

fn main() {
    App::new()
        .insert_resource(support::TilemapExamplePane {
            detail_layer_visible: true,
            highlight_alpha: 0.8,
            ..default()
        })
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap hex strategy".into(),
                        resolution: (1440, 980).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(support::pane_plugins())
        .add_plugins(TilemapPlugin::default())
        .register_pane::<support::TilemapExamplePane>()
        .add_systems(Startup, setup)
        .add_systems(Update, (support::sync_example_pane, update_strategy))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = support::DemoPalette::new(&mut images);
    let (mut map, coords) = support::build_hex_strategy_map(&palette);
    let blocked = HashSet::from([
        AxialHex::new(1, 0),
        AxialHex::new(1, -1),
        AxialHex::new(0, 2),
        AxialHex::new(-1, 2),
    ]);
    for hex in &blocked {
        let coord = support::hex_axial_to_tile(*hex);
        map.set_tile(
            COLLISION_LAYER,
            coord,
            saddle_world_tilemap::TileCell::new(palette.tiles.wall),
        );
        map.set_tile(
            DETAIL_LAYER,
            coord,
            saddle_world_tilemap::TileCell::new(palette.tiles.rock),
        );
    }

    support::spawn_camera(
        &mut commands,
        "Hex Strategy Camera",
        Vec3::new(0.0, 0.0, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Hex strategy board rendered through saddle-world-tilemap and pathfound through saddle-world-hex-grid.\nMove the cursor across the frontier to preview the cheapest route around the canyon blockers.",
    );
    support::spawn_label(
        &mut commands,
        "Hex tactics frontier",
        Vec3::new(0.0, 305.0, 5.0),
    );

    let map = support::spawn_map(
        &mut commands,
        "Hex Strategy Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(HexStrategyDemo {
        map,
        board: coords.into_iter().collect(),
        start: AxialHex::ZERO,
        hovered: None,
        highlighted: Vec::new(),
        blocked,
        highlight_kind: palette.tiles.square_highlight,
    });
}

fn update_strategy(
    windows: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut demo: ResMut<HexStrategyDemo>,
    map_query: Query<(&saddle_world_tilemap::Tilemap, &GlobalTransform)>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    let Ok((map, map_transform)) = map_query.get(demo.map) else {
        return;
    };
    let (camera, camera_transform) = *camera;
    let hovered = support::cursor_world(windows.into_inner(), camera, camera_transform)
        .and_then(|world| map.geometry.world_to_tile(map_transform, world))
        .map(support::hex_tile_to_axial)
        .filter(|hex| demo.board.contains(hex));

    let path = hovered.and_then(|goal| current_path(&demo, map, goal));
    let next_highlighted: Vec<_> = path
        .as_ref()
        .map(|path| {
            path.cells
                .iter()
                .copied()
                .map(support::hex_axial_to_tile)
                .collect()
        })
        .unwrap_or_default();

    if demo.hovered != hovered || demo.highlighted != next_highlighted {
        let map_entity = demo.map;
        for coord in demo.highlighted.drain(..) {
            commands_out.write(TilemapCommand::ClearTile {
                map: map_entity,
                layer: HIGHLIGHT_LAYER,
                coord,
            });
        }
        for coord in &next_highlighted {
            commands_out.write(TilemapCommand::SetTile {
                map: map_entity,
                layer: HIGHLIGHT_LAYER,
                coord: *coord,
                tile: saddle_world_tilemap::TileCell::new(demo.highlight_kind),
            });
        }
        demo.highlighted = next_highlighted;
        demo.hovered = hovered;
    }

    overlay.0 = if let Some(goal) = hovered {
        if let Some(path) = path {
            format!(
                "Hex strategy board rendered through saddle-world-tilemap and pathfound through saddle-world-hex-grid.\nGoal hex: ({}, {})  Path length: {}  Total cost: {}\nBlocked cells: {}",
                goal.q,
                goal.r,
                path.cells.len(),
                path.total_cost,
                demo.blocked.len(),
            )
        } else {
            format!(
                "Hex strategy board rendered through saddle-world-tilemap and pathfound through saddle-world-hex-grid.\nGoal hex: ({}, {}) is unreachable with the current canyon blockers.",
                goal.q, goal.r,
            )
        }
    } else {
        "Hex strategy board rendered through saddle-world-tilemap and pathfound through saddle-world-hex-grid.\nMove the cursor across the frontier to preview the cheapest route.".to_string()
    };
}

fn current_path(
    demo: &HexStrategyDemo,
    map: &saddle_world_tilemap::Tilemap,
    goal: AxialHex,
) -> Option<HexPath> {
    a_star(demo.start, goal, |_, to| {
        if !demo.board.contains(&to) || demo.blocked.contains(&to) {
            return None;
        }

        let coord = support::hex_axial_to_tile(to);
        let movement_cost = map
            .get_tile(GROUND_LAYER, coord)
            .and_then(|tile| map.layer(GROUND_LAYER)?.catalog.kind(tile.kind))
            .map_or(1, |kind| kind.movement_cost.max(1));
        Some(movement_cost as u32)
    })
}
