#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy::remote::RemotePlugin;
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_world_tilemap::{
    ChunkCoord, TileCoord, TilemapCommand, TilemapDebugOverlay, TilemapDebugSettings,
    TilemapPlugin, TilemapSystems,
};
use support::{DemoPalette, GROUND_LAYER, HIGHLIGHT_LAYER, ISOMETRIC_SIZE, OverlayText};

const LARGE_MAP_SIZE: UVec2 = UVec2::new(96, 96);
const MAX_SQUARE_STAGE: u8 = 2;

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum LabSystems {
    Drive,
    Diagnostics,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum CameraFocus {
    Overview,
    Isometric,
    LargeLeft,
    LargeRight,
}

#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct LabControl {
    pub camera_focus: CameraFocus,
    pub square_edit_stage: u8,
    pub iso_selection: TileCoord,
}

impl Default for LabControl {
    fn default() -> Self {
        Self {
            camera_focus: CameraFocus::Overview,
            square_edit_stage: 0,
            iso_selection: TileCoord::new(6, 2),
        }
    }
}

#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub square_applied_tiles: usize,
    pub square_latest_edit: TileCoord,
    pub square_rebuilds_last_frame: usize,
    pub iso_selection: TileCoord,
    pub iso_selection_cost: u16,
    pub large_center_tile: TileCoord,
    pub large_center_chunk: ChunkCoord,
    pub large_total_chunks: usize,
    pub animation_loops: u32,
}

impl Default for LabDiagnostics {
    fn default() -> Self {
        Self {
            square_applied_tiles: 0,
            square_latest_edit: TileCoord::ZERO,
            square_rebuilds_last_frame: 0,
            iso_selection: TileCoord::new(6, 2),
            iso_selection_cost: 0,
            large_center_tile: TileCoord::ZERO,
            large_center_chunk: ChunkCoord::ZERO,
            large_total_chunks: 0,
            animation_loops: 0,
        }
    }
}

#[derive(Resource)]
struct LabScene {
    square_map: Entity,
    iso_map: Entity,
    large_map: Entity,
    square_road_kind: saddle_world_tilemap::TileKindId,
    square_highlight_kind: saddle_world_tilemap::TileKindId,
    iso_highlight_kind: saddle_world_tilemap::TileKindId,
    overview_focus: Vec3,
    isometric_focus: Vec3,
    large_left_focus: Vec3,
    large_right_focus: Vec3,
}

#[derive(Resource, Default)]
struct LabRuntime {
    applied_square_stage: Option<u8>,
    square_highlighted: Vec<TileCoord>,
    applied_iso_selection: Option<TileCoord>,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));
    app.insert_resource(LabControl::default());
    app.insert_resource(LabDiagnostics::default());
    app.insert_resource(LabRuntime::default());
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "tilemap crate-local lab".into(),
                    resolution: (1820, 1040).into(),
                    ..default()
                }),
                ..default()
            }),
    );
    #[cfg(feature = "dev")]
    app.add_plugins(RemotePlugin::default());
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_port(15702));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::TilemapLabE2EPlugin);
    app.add_plugins(
        TilemapPlugin::default().with_debug_settings(TilemapDebugSettings {
            enabled: true,
            draw_chunk_bounds: true,
            draw_dirty_chunks: true,
            ..default()
        }),
    );
    app.register_type::<CameraFocus>()
        .register_type::<LabControl>()
        .register_type::<LabDiagnostics>()
        .configure_sets(
            Update,
            LabSystems::Drive.before(TilemapSystems::ApplyCommands),
        )
        .configure_sets(
            Update,
            LabSystems::Diagnostics.after(TilemapSystems::SyncRender),
        )
        .configure_sets(Update, LabSystems::Overlay.after(LabSystems::Diagnostics))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_debug_input,
                apply_camera_focus,
                apply_square_stage,
                apply_iso_selection,
            )
                .in_set(LabSystems::Drive),
        )
        .add_systems(
            Update,
            (record_animation_loops, update_diagnostics).in_set(LabSystems::Diagnostics),
        )
        .add_systems(Update, update_overlay.in_set(LabSystems::Overlay));
    app.run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);

    let square_map = support::build_square_showcase_map(&palette);
    let square_center = support::map_local_center(&square_map, support::SQUARE_SIZE);
    let square_focus = Vec2::new(-370.0, 60.0);
    let square_translation = (square_focus - square_center).extend(0.0);

    let iso_map = support::build_isometric_battlefield_map(&palette);
    let iso_center = support::map_local_center(&iso_map, support::ISOMETRIC_SIZE);
    let iso_focus = Vec2::new(430.0, 50.0);
    let iso_translation = (iso_focus - iso_center).extend(0.0);

    let large_map = support::build_large_map(&palette, LARGE_MAP_SIZE);
    let large_center = support::map_local_center(&large_map, LARGE_MAP_SIZE);
    let large_focus = Vec2::new(0.0, -1040.0);
    let large_translation = (large_focus - large_center).extend(0.0);

    support::spawn_camera(
        &mut commands,
        "Tilemap Lab Camera",
        Vec3::new(20.0, 15.0, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Controls: 1-4 camera focus, Q/E square edit stage, WASD move iso selection, R reset.\nLeft: square runtime edits and animated water. Right: isometric picking and movement-cost metadata. Below: a large dense map for camera sweeps across chunk boundaries.",
    );

    let square_entity = support::spawn_map(
        &mut commands,
        "Square Showcase Map",
        square_map,
        square_translation,
        TilemapDebugOverlay::default(),
    );
    support::spawn_label(
        &mut commands,
        "Square runtime-edit showcase",
        Vec3::new(square_focus.x, square_focus.y + 340.0, 5.0),
    );

    let iso_entity = support::spawn_map(
        &mut commands,
        "Isometric Showcase Map",
        iso_map,
        iso_translation,
        TilemapDebugOverlay::default(),
    );
    support::spawn_label(
        &mut commands,
        "Isometric picking showcase",
        Vec3::new(iso_focus.x, iso_focus.y + 250.0, 5.0),
    );

    let large_entity = support::spawn_map(
        &mut commands,
        "Large Showcase Map",
        large_map,
        large_translation,
        TilemapDebugOverlay::default(),
    );
    support::spawn_label(
        &mut commands,
        "Large map camera sweep",
        Vec3::new(large_focus.x, large_focus.y + 1140.0, 5.0),
    );

    commands.insert_resource(LabScene {
        square_map: square_entity,
        iso_map: iso_entity,
        large_map: large_entity,
        square_road_kind: palette.tiles.road,
        square_highlight_kind: palette.tiles.square_highlight,
        iso_highlight_kind: palette.tiles.iso_highlight,
        overview_focus: Vec3::new(20.0, 15.0, 999.0),
        isometric_focus: Vec3::new(iso_focus.x, iso_focus.y - 20.0, 999.0),
        large_left_focus: Vec3::new(large_focus.x - 640.0, large_focus.y, 999.0),
        large_right_focus: Vec3::new(large_focus.x + 640.0, large_focus.y, 999.0),
    });
}

fn handle_debug_input(input: Res<ButtonInput<KeyCode>>, mut control: ResMut<LabControl>) {
    if input.just_pressed(KeyCode::Digit1) {
        control.camera_focus = CameraFocus::Overview;
    }
    if input.just_pressed(KeyCode::Digit2) {
        control.camera_focus = CameraFocus::Isometric;
    }
    if input.just_pressed(KeyCode::Digit3) {
        control.camera_focus = CameraFocus::LargeLeft;
    }
    if input.just_pressed(KeyCode::Digit4) {
        control.camera_focus = CameraFocus::LargeRight;
    }

    if input.just_pressed(KeyCode::KeyQ) {
        control.square_edit_stage = control.square_edit_stage.saturating_sub(1);
    }
    if input.just_pressed(KeyCode::KeyE) {
        control.square_edit_stage = (control.square_edit_stage + 1).min(MAX_SQUARE_STAGE);
    }

    let mut iso_delta = IVec2::ZERO;
    if input.just_pressed(KeyCode::KeyA) {
        iso_delta.x -= 1;
    }
    if input.just_pressed(KeyCode::KeyD) {
        iso_delta.x += 1;
    }
    if input.just_pressed(KeyCode::KeyW) {
        iso_delta.y -= 1;
    }
    if input.just_pressed(KeyCode::KeyS) {
        iso_delta.y += 1;
    }
    if iso_delta != IVec2::ZERO {
        control.iso_selection = TileCoord::new(
            (control.iso_selection.x + iso_delta.x).clamp(0, ISOMETRIC_SIZE.x as i32 - 1),
            (control.iso_selection.y + iso_delta.y).clamp(0, ISOMETRIC_SIZE.y as i32 - 1),
        );
        control.camera_focus = CameraFocus::Isometric;
    }

    if input.just_pressed(KeyCode::KeyR) {
        *control = LabControl::default();
    }
}

fn apply_camera_focus(
    control: Res<LabControl>,
    scene: Res<LabScene>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
) {
    camera.translation = match control.camera_focus {
        CameraFocus::Overview => scene.overview_focus,
        CameraFocus::Isometric => scene.isometric_focus,
        CameraFocus::LargeLeft => scene.large_left_focus,
        CameraFocus::LargeRight => scene.large_right_focus,
    };
}

fn apply_square_stage(
    control: Res<LabControl>,
    scene: Res<LabScene>,
    mut runtime: ResMut<LabRuntime>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    if runtime.applied_square_stage == Some(control.square_edit_stage) {
        return;
    }

    for coord in runtime.square_highlighted.drain(..) {
        commands_out.write(TilemapCommand::ClearTile {
            map: scene.square_map,
            layer: HIGHLIGHT_LAYER,
            coord,
        });
    }

    let branch = support::square_runtime_edit_coords();
    for coord in &branch {
        commands_out.write(TilemapCommand::ClearTile {
            map: scene.square_map,
            layer: support::DETAIL_LAYER,
            coord: *coord,
        });
    }

    let active_count = match control.square_edit_stage {
        0 => 0,
        1 => 4,
        _ => branch.len(),
    };

    for coord in branch.iter().take(active_count) {
        commands_out.write(TilemapCommand::SetTile {
            map: scene.square_map,
            layer: support::DETAIL_LAYER,
            coord: *coord,
            tile: saddle_world_tilemap::TileCell::new(scene.square_road_kind),
        });
        commands_out.write(TilemapCommand::SetTile {
            map: scene.square_map,
            layer: HIGHLIGHT_LAYER,
            coord: *coord,
            tile: saddle_world_tilemap::TileCell::new(scene.square_highlight_kind),
        });
        runtime.square_highlighted.push(*coord);
    }

    runtime.applied_square_stage = Some(control.square_edit_stage);
}

fn apply_iso_selection(
    control: Res<LabControl>,
    scene: Res<LabScene>,
    mut runtime: ResMut<LabRuntime>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    if runtime.applied_iso_selection == Some(control.iso_selection) {
        return;
    }

    if let Some(previous) = runtime.applied_iso_selection {
        commands_out.write(TilemapCommand::ClearTile {
            map: scene.iso_map,
            layer: HIGHLIGHT_LAYER,
            coord: previous,
        });
    }
    commands_out.write(TilemapCommand::SetTile {
        map: scene.iso_map,
        layer: HIGHLIGHT_LAYER,
        coord: control.iso_selection,
        tile: saddle_world_tilemap::TileCell::new(scene.iso_highlight_kind),
    });
    runtime.applied_iso_selection = Some(control.iso_selection);
}

fn record_animation_loops(
    mut events: MessageReader<saddle_world_tilemap::TileAnimationLooped>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    diagnostics.animation_loops += events.read().count() as u32;
}

fn update_diagnostics(
    control: Res<LabControl>,
    scene: Res<LabScene>,
    mut diagnostics: ResMut<LabDiagnostics>,
    maps: Query<(
        &saddle_world_tilemap::Tilemap,
        &saddle_world_tilemap::TilemapDiagnostics,
        &GlobalTransform,
    )>,
    camera: Single<&Transform, With<Camera2d>>,
) {
    if let Ok((_, square_diagnostics, _)) = maps.get(scene.square_map) {
        diagnostics.square_rebuilds_last_frame = square_diagnostics.chunks_rebuilt_this_frame;
    }

    let branch = support::square_runtime_edit_coords();
    diagnostics.square_applied_tiles = match control.square_edit_stage {
        0 => 0,
        1 => 4,
        _ => branch.len(),
    };
    diagnostics.square_latest_edit = if diagnostics.square_applied_tiles == 0 {
        TileCoord::ZERO
    } else {
        branch[diagnostics.square_applied_tiles - 1]
    };

    if let Ok((iso_map, _, _)) = maps.get(scene.iso_map) {
        diagnostics.iso_selection = control.iso_selection;
        diagnostics.iso_selection_cost = iso_map
            .get_tile(GROUND_LAYER, control.iso_selection)
            .and_then(|tile| {
                iso_map
                    .layer(GROUND_LAYER)
                    .and_then(|layer| layer.catalog.kind(tile.kind))
            })
            .map_or(0, |kind| kind.movement_cost);
    }

    if let Ok((large_map, _, large_transform)) = maps.get(scene.large_map) {
        let world_center = camera.translation.truncate();
        diagnostics.large_center_tile = large_map
            .geometry
            .world_to_tile(large_transform, world_center)
            .unwrap_or(TileCoord::ZERO);
        diagnostics.large_center_chunk = diagnostics.large_center_tile.chunk(large_map.chunk_size);
        diagnostics.large_total_chunks = large_map
            .layer(GROUND_LAYER)
            .map_or(0, |layer| layer.chunks.len());
    }
}

fn update_overlay(
    control: Res<LabControl>,
    diagnostics: Res<LabDiagnostics>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    overlay.0 = format!(
        "Controls: 1-4 camera focus, Q/E square edit stage, WASD move iso selection, R reset.\nLeft: square runtime edits and animated water. Right: isometric picking and movement-cost metadata. Below: a large dense map for camera sweeps across chunk boundaries.\nSquare stage: {}  applied tiles: {}  latest edit: ({}, {})  rebuilds: {}\nIso selection: ({}, {})  movement cost: {}\nCamera focus: {:?}  large center tile: ({}, {})  chunk: ({}, {})  large chunks: {}  animation loops: {}",
        control.square_edit_stage,
        diagnostics.square_applied_tiles,
        diagnostics.square_latest_edit.x,
        diagnostics.square_latest_edit.y,
        diagnostics.square_rebuilds_last_frame,
        diagnostics.iso_selection.x,
        diagnostics.iso_selection.y,
        diagnostics.iso_selection_cost,
        control.camera_focus,
        diagnostics.large_center_tile.x,
        diagnostics.large_center_tile.y,
        diagnostics.large_center_chunk.x,
        diagnostics.large_center_chunk.y,
        diagnostics.large_total_chunks,
        diagnostics.animation_loops,
    );
}
