#[cfg(feature = "e2e")]
mod e2e;
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

use bevy::prelude::*;
use bevy_enhanced_input::context::InputContextAppExt;
use bevy_enhanced_input::prelude::{
    Action, Bindings, Cancel as InputCancel, Cardinal, Complete, EnhancedInputPlugin, Fire,
    InputAction, Start, actions, bindings,
};
use saddle_ai_fov::{FovPlugin, FovSystems, GridFov, GridFovState, GridMapSpec, GridOpacityMap};
use saddle_pane::prelude::*;
use saddle_procgen_dungeon_gen::{
    DungeonConfig, DungeonMap, DungeonSeed, LockKeyConfig, SecretRoomConfig, TileType,
    generate_dungeon,
};
use saddle_world_fog_of_war::{
    FogLayerId, FogLayerMask, FogOfWarConfig, FogOfWarMap, FogOfWarPlugin, FogOfWarSystems,
    FogOverlay2d, FogWorldAxes, VisionCellSource,
};
use saddle_world_tilemap::{
    TileCell, TileCoord, TileLayerConfig, TileLayerId, TileLayerRenderConfig, TileLayerState,
    TileRowDirection, Tilemap, TilemapDebugOverlay, TilemapDebugSettings, TilemapGeometry,
    TilemapPlugin,
};
use saddle_world_tilemap_example_support as support;

const GRID_DIMENSIONS: UVec2 = UVec2::new(72, 48);
const TILE_SIZE: f32 = 32.0;
const PLAYER_Z: f32 = 6.0;
const MARKER_Z: f32 = 5.0;
const MAP_BACKGROUND_Z: f32 = -1.0;
const HIGHLIGHT_LAYER: TileLayerId = support::HIGHLIGHT_LAYER;
const COLLISION_LAYER: TileLayerId = support::COLLISION_LAYER;
const GROUND_LAYER: TileLayerId = support::GROUND_LAYER;
const DETAIL_LAYER: TileLayerId = support::DETAIL_LAYER;

#[derive(Component)]
struct DemoPlayer;

#[derive(Component, Default)]
struct DemoInputState {
    move_axis: Vec2,
    rebuild_requested: bool,
}

#[derive(Component)]
struct PlayerGridPosition {
    cell: IVec2,
    cooldown_remaining: f32,
}

#[derive(Component)]
struct FollowCamera;

#[derive(Component)]
struct FogOverlayMarker;

#[derive(Component)]
struct PulseMarker {
    base_y: f32,
    amplitude: f32,
    speed: f32,
}

#[derive(Debug, InputAction)]
#[action_output(Vec2)]
struct MoveAction;

#[derive(Debug, InputAction)]
#[action_output(bool)]
struct RegenerateAction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayoutSnapshot {
    seed: u32,
    room_attempts: u32,
    loop_density_milli: u32,
    secret_rooms: bool,
}

impl LayoutSnapshot {
    fn from_pane(pane: &RoguelikePane) -> Self {
        Self {
            seed: pane.seed,
            room_attempts: pane.room_attempts,
            loop_density_milli: (pane.loop_density * 1_000.0).round() as u32,
            secret_rooms: pane.secret_rooms,
        }
    }
}

#[derive(Resource, Clone, Debug, Pane)]
#[pane(title = "Roguelike Showcase", position = "top-right")]
struct RoguelikePane {
    #[pane(tab = "Generation", slider, min = 1, max = 4096, step = 1)]
    seed: u32,
    #[pane(tab = "Generation", slider, min = 96, max = 320, step = 8)]
    room_attempts: u32,
    #[pane(tab = "Generation", slider, min = 0.0, max = 0.45, step = 0.01)]
    loop_density: f32,
    #[pane(tab = "Generation", toggle)]
    secret_rooms: bool,
    #[pane(tab = "Visibility", slider, min = 3, max = 10, step = 1)]
    fov_radius: i32,
    #[pane(tab = "Visibility", slider, min = 0.0, max = 0.6, step = 0.01)]
    fog_edge_softness: f32,
    #[pane(tab = "Traversal", slider, min = 0.06, max = 0.32, step = 0.01)]
    move_cadence: f32,
    #[pane(tab = "Traversal", slider, min = 2.0, max = 18.0, step = 0.25)]
    camera_follow_lerp: f32,
    #[pane(tab = "Runtime", monitor)]
    visible_cells: usize,
    #[pane(tab = "Runtime", monitor)]
    explored_cells: usize,
}

impl Default for RoguelikePane {
    fn default() -> Self {
        Self {
            seed: 19,
            room_attempts: 192,
            loop_density: 0.22,
            secret_rooms: true,
            fov_radius: 6,
            fog_edge_softness: 0.24,
            move_cadence: 0.12,
            camera_follow_lerp: 10.0,
            visible_cells: 0,
            explored_cells: 0,
        }
    }
}

#[derive(Resource)]
struct RoguelikeAssets {
    palette: support::DemoPalette,
}

#[derive(Resource)]
struct RoguelikeScene {
    map_entity: Option<Entity>,
    marker_entities: Vec<Entity>,
    player_entity: Entity,
    overlay_entity: Entity,
    snapshot: LayoutSnapshot,
    dungeon: DungeonMap,
    map_spec: GridMapSpec,
}

fn main() {
    let mut app = App::new();
    let fog_config = fog_config();

    app.insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.04)))
        .insert_resource(support::TilemapExamplePane {
            rows_face_up: true,
            highlight_alpha: 0.82,
            ..default()
        })
        .insert_resource(RoguelikePane::default())
        .insert_resource(GridOpacityMap::new(grid_spec()))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap roguelike_showcase".into(),
                        resolution: (1520, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins((
            support::pane_plugins(),
            TilemapPlugin::default().with_debug_settings(TilemapDebugSettings {
                enabled: false,
                draw_dirty_chunks: false,
                ..default()
            }),
            EnhancedInputPlugin,
            FovPlugin::default(),
            FogOfWarPlugin::default().with_config(fog_config),
        ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::RoguelikeShowcaseE2EPlugin);
    app.register_pane::<support::TilemapExamplePane>()
        .register_pane::<RoguelikePane>()
        .add_input_context::<DemoPlayer>()
        .add_observer(cache_move_axis)
        .add_observer(clear_move_axis_on_cancel)
        .add_observer(clear_move_axis_on_complete)
        .add_observer(request_rebuild)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                support::sync_example_pane,
                rebuild_level_if_needed,
                move_player.before(FovSystems::MarkDirty),
                sync_player_fov_and_fog.before(FovSystems::MarkDirty),
                sync_fov_to_fog
                    .after(FovSystems::Recompute)
                    .before(FogOfWarSystems::CollectVisionSources),
                sync_fog_overlay.after(FogOfWarSystems::UpdateExplorationMemory),
                follow_camera,
                animate_markers,
                update_overlay.after(FogOfWarSystems::UpdateExplorationMemory),
                update_monitors.after(FogOfWarSystems::UpdateExplorationMemory),
            ),
        );

    app.run();
}

fn grid_spec() -> GridMapSpec {
    GridMapSpec {
        origin: Vec2::ZERO,
        dimensions: GRID_DIMENSIONS,
        cell_size: Vec2::splat(TILE_SIZE),
    }
}

fn fog_config() -> FogOfWarConfig {
    let mut config = FogOfWarConfig::default();
    config.grid.origin = Vec2::ZERO;
    config.grid.dimensions = GRID_DIMENSIONS;
    config.grid.cell_size = Vec2::splat(TILE_SIZE);
    config.grid.chunk_size = UVec2::splat(12);
    config.world_axes = FogWorldAxes::XY;
    config
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = support::DemoPalette::new(&mut images);
    let config = dungeon_config(&RoguelikePane::default());
    let dungeon = generate_dungeon(&config).expect("roguelike showcase config should stay valid");
    let map_spec = grid_spec();
    let center = map_spec.origin + map_spec.world_size() * 0.5;

    commands.spawn((
        Name::new("Dungeon Backdrop"),
        Sprite::from_color(
            Color::srgb(0.04, 0.05, 0.07),
            map_spec.world_size() + Vec2::splat(96.0),
        ),
        Transform::from_xyz(center.x, center.y, MAP_BACKGROUND_Z),
    ));
    commands.spawn((
        Name::new("Roguelike Camera"),
        FollowCamera,
        Camera2d,
        Transform::from_xyz(center.x, center.y, 1000.0),
    ));

    let overlay_entity = support::spawn_overlay(
        &mut commands,
        "Procedural dungeon + tilemap + FOV + fog of war.\nWASD / arrows explore, R regenerates, and the pane retunes generation plus visibility live.",
    );
    support::spawn_label(
        &mut commands,
        "Roguelike integration showcase",
        Vec3::new(center.x, map_spec.world_size().y + 56.0, 3.0),
    );

    let player_entity = commands
        .spawn((
            Name::new("Scout"),
            DemoPlayer,
            DemoInputState::default(),
            PlayerGridPosition {
                cell: dungeon.start,
                cooldown_remaining: 0.0,
            },
            GridFov::new(RoguelikePane::default().fov_radius),
            VisionCellSource::new(FogLayerMask::bit(FogLayerId(0))),
            Sprite::from_color(Color::srgb(0.42, 0.96, 0.66), Vec2::splat(TILE_SIZE * 0.44)),
            Transform::from_translation(cell_center(dungeon.start).extend(PLAYER_Z)),
            actions!(DemoPlayer[
                (
                    Action::<MoveAction>::new(),
                    Bindings::spawn((Cardinal::wasd_keys(), Cardinal::arrows()))
                ),
                (
                    Action::<RegenerateAction>::new(),
                    bindings![KeyCode::KeyR]
                ),
            ]),
        ))
        .id();

    commands.insert_resource(RoguelikeAssets { palette });
    commands.insert_resource(RoguelikeScene {
        map_entity: None,
        marker_entities: Vec::new(),
        player_entity,
        overlay_entity,
        snapshot: LayoutSnapshot::from_pane(&RoguelikePane::default()),
        dungeon: DungeonMap::default(),
        map_spec,
    });
}

fn rebuild_level_if_needed(
    pane: Res<RoguelikePane>,
    assets: Res<RoguelikeAssets>,
    mut scene: ResMut<RoguelikeScene>,
    mut player: Query<
        (
            &mut DemoInputState,
            &mut PlayerGridPosition,
            &mut Transform,
            &mut GridFov,
            &mut VisionCellSource,
        ),
        With<DemoPlayer>,
    >,
    mut fog_map: ResMut<FogOfWarMap>,
    mut commands: Commands,
) {
    let Ok((mut input, mut grid_position, mut transform, mut grid_fov, mut vision)) =
        player.get_mut(scene.player_entity)
    else {
        return;
    };

    let snapshot = LayoutSnapshot::from_pane(&pane);
    let should_rebuild =
        scene.map_entity.is_none() || input.rebuild_requested || snapshot != scene.snapshot;
    if !should_rebuild {
        return;
    }

    input.rebuild_requested = false;
    scene.snapshot = snapshot;

    if let Some(map_entity) = scene.map_entity.take() {
        commands.entity(map_entity).despawn();
    }
    for marker in scene.marker_entities.drain(..) {
        commands.entity(marker).despawn();
    }

    let dungeon = generate_dungeon(&dungeon_config(&pane))
        .expect("roguelike showcase config should stay valid");
    let map = build_dungeon_tilemap(&assets.palette, &dungeon);
    let map_entity = support::spawn_map(
        &mut commands,
        "Roguelike Dungeon",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );
    let markers = spawn_dungeon_markers(&mut commands, &dungeon);

    grid_position.cell = dungeon.start;
    grid_position.cooldown_remaining = 0.0;
    transform.translation = cell_center(dungeon.start).extend(PLAYER_Z);
    grid_fov.config.radius = pane.fov_radius;
    vision.cells = Vec::new();
    vision.enabled = true;

    let new_grid = opacity_map_from_dungeon(&dungeon);
    *fog_map = FogOfWarMap::new(fog_config());
    commands.insert_resource(new_grid);

    scene.map_entity = Some(map_entity);
    scene.marker_entities = markers;
    scene.dungeon = dungeon;
}

fn build_dungeon_tilemap(palette: &support::DemoPalette, dungeon: &DungeonMap) -> Tilemap {
    let geometry = TilemapGeometry::square(Vec2::splat(TILE_SIZE))
        .with_origin(Vec2::splat(TILE_SIZE * 0.5))
        .with_row_direction(TileRowDirection::Up);
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
                .with_tint(Color::srgba(0.90, 0.95, 1.0, 0.82)),
        ),
        catalog,
    ));

    for y in 0..dungeon.height_i32() {
        for x in 0..dungeon.width_i32() {
            let cell = IVec2::new(x, y);
            let coord = TileCoord::new(x, y);
            let tile = dungeon.tile(cell).unwrap_or(TileType::Void);
            let room_role = dungeon.room_at(cell).and_then(|room| room.role);

            let ground_kind = match tile {
                TileType::Void => palette.tiles.rock,
                TileType::Wall | TileType::SecretWall => palette.tiles.rock,
                TileType::Floor => palette.tiles.soil,
                TileType::Corridor => palette.tiles.grass,
                TileType::Door => palette.tiles.sand,
                TileType::StairsUp | TileType::StairsDown => palette.tiles.soil,
            };
            let mut ground = TileCell::new(ground_kind);
            if matches!(tile, TileType::Void) {
                ground = ground.with_tint(Color::srgb(0.08, 0.09, 0.12));
            } else if matches!(tile, TileType::Wall | TileType::SecretWall) {
                ground = ground.with_tint(Color::srgb(0.20, 0.22, 0.26));
            } else if matches!(room_role, Some(saddle_procgen_dungeon_gen::RoomRole::Boss)) {
                ground = ground.with_tint(Color::srgb(0.47, 0.25, 0.22));
            }
            map.set_tile(GROUND_LAYER, coord, ground);

            match tile {
                TileType::Corridor => {
                    map.set_tile(
                        DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.road).with_tint(Color::srgb(0.76, 0.68, 0.44)),
                    );
                }
                TileType::Door => {
                    map.set_tile(
                        DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.square_highlight)
                            .with_tint(Color::srgb(0.82, 0.58, 0.22)),
                    );
                }
                TileType::StairsUp => {
                    map.set_tile(
                        DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.flower)
                            .with_tint(Color::srgb(0.46, 0.96, 0.64)),
                    );
                }
                TileType::StairsDown => {
                    map.set_tile(
                        DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.square_highlight)
                            .with_tint(Color::srgb(1.0, 0.84, 0.32)),
                    );
                }
                TileType::Floor
                    if matches!(
                        room_role,
                        Some(saddle_procgen_dungeon_gen::RoomRole::Treasure)
                    ) =>
                {
                    map.set_tile(
                        DETAIL_LAYER,
                        coord,
                        TileCell::new(palette.tiles.flower).with_tint(Color::srgb(1.0, 0.88, 0.42)),
                    );
                }
                _ => {}
            }

            if tile.blocks_movement() {
                map.set_tile(COLLISION_LAYER, coord, TileCell::new(palette.tiles.wall));
            }
        }
    }

    map
}

fn spawn_dungeon_markers(commands: &mut Commands, dungeon: &DungeonMap) -> Vec<Entity> {
    let mut markers = Vec::new();
    markers.push(spawn_marker(
        commands,
        "Entry Beacon",
        dungeon.start,
        Color::srgb(0.34, 0.98, 0.70),
        0.0,
    ));
    markers.push(spawn_marker(
        commands,
        "Exit Beacon",
        dungeon.exit,
        Color::srgb(1.0, 0.82, 0.28),
        0.8,
    ));

    for room in dungeon.rooms.iter().take(8) {
        let Some(role) = room.role else {
            continue;
        };
        let color = match role {
            saddle_procgen_dungeon_gen::RoomRole::Boss => Color::srgb(0.92, 0.28, 0.28),
            saddle_procgen_dungeon_gen::RoomRole::Treasure => Color::srgb(0.98, 0.86, 0.32),
            saddle_procgen_dungeon_gen::RoomRole::Shop => Color::srgb(0.42, 0.82, 1.0),
            saddle_procgen_dungeon_gen::RoomRole::Safe => Color::srgb(0.54, 0.92, 0.52),
            _ => continue,
        };
        markers.push(spawn_marker(
            commands,
            format!("Room Marker {}", room.id),
            room.center,
            color,
            room.id as f32 * 0.37,
        ));
    }

    markers
}

fn spawn_marker(
    commands: &mut Commands,
    name: impl Into<String>,
    cell: IVec2,
    color: Color,
    phase: f32,
) -> Entity {
    let center = cell_center(cell);
    commands
        .spawn((
            Name::new(name.into()),
            PulseMarker {
                base_y: center.y,
                amplitude: 4.0,
                speed: 1.4 + phase * 0.15,
            },
            Sprite::from_color(color, Vec2::splat(TILE_SIZE * 0.28)),
            Transform::from_xyz(center.x, center.y, MARKER_Z),
        ))
        .id()
}

fn opacity_map_from_dungeon(dungeon: &DungeonMap) -> GridOpacityMap {
    GridOpacityMap::from_fn(grid_spec(), |cell| {
        dungeon
            .tile(cell)
            .map(|tile| tile.blocks_movement())
            .unwrap_or(true)
    })
}

fn dungeon_config(pane: &RoguelikePane) -> DungeonConfig {
    let mut config = DungeonConfig {
        width: GRID_DIMENSIONS.x,
        height: GRID_DIMENSIONS.y,
        seed: DungeonSeed(pane.seed as u64),
        loop_density: pane.loop_density,
        lock_key: LockKeyConfig {
            enabled: true,
            lock_count: 1,
        },
        secret_rooms: SecretRoomConfig {
            enabled: pane.secret_rooms,
            ..default()
        },
        ..Default::default()
    };
    config.rooms_corridors.room_attempts = pane.room_attempts;
    config
}

fn cell_center(cell: IVec2) -> Vec2 {
    grid_spec()
        .cell_to_world_center(cell)
        .expect("dungeon cells should stay in bounds")
}

fn cache_move_axis(
    trigger: On<Fire<MoveAction>>,
    mut players: Query<&mut DemoInputState, With<DemoPlayer>>,
) {
    if let Ok(mut input) = players.get_mut(trigger.context) {
        input.move_axis = trigger.value;
    }
}

fn clear_move_axis_on_cancel(
    trigger: On<InputCancel<MoveAction>>,
    mut players: Query<&mut DemoInputState, With<DemoPlayer>>,
) {
    if let Ok(mut input) = players.get_mut(trigger.context) {
        input.move_axis = Vec2::ZERO;
    }
}

fn clear_move_axis_on_complete(
    trigger: On<Complete<MoveAction>>,
    mut players: Query<&mut DemoInputState, With<DemoPlayer>>,
) {
    if let Ok(mut input) = players.get_mut(trigger.context) {
        input.move_axis = Vec2::ZERO;
    }
}

fn request_rebuild(
    trigger: On<Start<RegenerateAction>>,
    mut players: Query<&mut DemoInputState, With<DemoPlayer>>,
) {
    if let Ok(mut input) = players.get_mut(trigger.context) {
        let _ = trigger;
        input.rebuild_requested = true;
    }
}

fn move_player(
    time: Res<Time>,
    pane: Res<RoguelikePane>,
    scene: Res<RoguelikeScene>,
    mut player: Query<(&DemoInputState, &mut PlayerGridPosition, &mut Transform), With<DemoPlayer>>,
) {
    let Ok((input, mut grid_position, mut transform)) = player.get_mut(scene.player_entity) else {
        return;
    };

    grid_position.cooldown_remaining =
        (grid_position.cooldown_remaining - time.delta_secs()).max(0.0);
    if grid_position.cooldown_remaining > 0.0 {
        return;
    }

    let direction = cardinal_direction(input.move_axis);
    if direction == IVec2::ZERO {
        return;
    }

    let next = grid_position.cell + direction;
    let Some(tile) = scene.dungeon.tile(next) else {
        return;
    };
    if tile.blocks_movement() {
        return;
    }

    grid_position.cell = next;
    grid_position.cooldown_remaining = pane.move_cadence.max(0.02);
    transform.translation = cell_center(next).extend(PLAYER_Z);
}

fn cardinal_direction(axis: Vec2) -> IVec2 {
    if axis.length_squared() <= 0.25 {
        return IVec2::ZERO;
    }
    if axis.x.abs() >= axis.y.abs() {
        IVec2::new(axis.x.signum() as i32, 0)
    } else {
        IVec2::new(0, axis.y.signum() as i32)
    }
}

fn sync_player_fov_and_fog(
    pane: Res<RoguelikePane>,
    scene: Res<RoguelikeScene>,
    mut players: Query<(&PlayerGridPosition, &mut GridFov), With<DemoPlayer>>,
) {
    let Ok((grid_position, mut fov)) = players.get_mut(scene.player_entity) else {
        return;
    };

    if fov.config.radius != pane.fov_radius {
        fov.config.radius = pane.fov_radius;
    }
    let _ = grid_position;
}

fn sync_fov_to_fog(
    scene: Res<RoguelikeScene>,
    players: Query<&GridFovState, With<DemoPlayer>>,
    mut vision: Query<&mut VisionCellSource, With<DemoPlayer>>,
) {
    let Ok(state) = players.get(scene.player_entity) else {
        return;
    };
    let Ok(mut source) = vision.get_mut(scene.player_entity) else {
        return;
    };
    source.cells.clear();
    source.cells.extend(state.visible_now.iter().copied());
}

fn sync_fog_overlay(
    pane: Res<RoguelikePane>,
    mut overlays: Query<&mut FogOverlay2d, With<FogOverlayMarker>>,
) {
    for mut overlay in &mut overlays {
        if (overlay.edge_softness - pane.fog_edge_softness).abs() > 0.001 {
            overlay.edge_softness = pane.fog_edge_softness;
        }
    }
}

fn follow_camera(
    time: Res<Time>,
    pane: Res<RoguelikePane>,
    scene: Res<RoguelikeScene>,
    player: Query<&Transform, With<DemoPlayer>>,
    mut camera: Query<&mut Transform, (With<FollowCamera>, Without<DemoPlayer>)>,
) {
    let Ok(player_transform) = player.get(scene.player_entity) else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };

    let blend = 1.0 - (-pane.camera_follow_lerp.max(0.0) * time.delta_secs()).exp();
    camera_transform.translation.x = camera_transform
        .translation
        .x
        .lerp(player_transform.translation.x, blend);
    camera_transform.translation.y = camera_transform
        .translation
        .y
        .lerp(player_transform.translation.y, blend);
}

fn animate_markers(time: Res<Time>, mut markers: Query<(&PulseMarker, &mut Transform)>) {
    for (pulse, mut transform) in &mut markers {
        transform.translation.y =
            pulse.base_y + (time.elapsed_secs() * pulse.speed).sin() * pulse.amplitude;
    }
}

fn update_overlay(
    pane: Res<RoguelikePane>,
    scene: Res<RoguelikeScene>,
    player: Query<&PlayerGridPosition, With<DemoPlayer>>,
    state: Query<&GridFovState, With<DemoPlayer>>,
    mut overlay: Query<&mut Text, With<support::OverlayText>>,
) {
    let Ok(player_position) = player.get(scene.player_entity) else {
        return;
    };
    let Ok(fov_state) = state.get(scene.player_entity) else {
        return;
    };
    let Ok(mut text) = overlay.get_mut(scene.overlay_entity) else {
        return;
    };

    text.0 = format!(
        "Procedural dungeon + tilemap + FOV + fog of war.\nWASD / arrows explore, R regenerates, and the pane retunes generation plus visibility live.\nSeed {}  rooms {}  loops {:.2}  secret rooms {}\nPlayer ({}, {})  visible {}  explored {}\nStart {:?}  exit {:?}  floor coverage {:>4.0}%  authored rooms {}",
        pane.seed,
        pane.room_attempts,
        pane.loop_density,
        if pane.secret_rooms { "on" } else { "off" },
        player_position.cell.x,
        player_position.cell.y,
        fov_state.visible_now.len(),
        fov_state.explored.len(),
        scene.dungeon.start,
        scene.dungeon.exit,
        scene.dungeon.floor_coverage_ratio() * 100.0,
        scene.dungeon.rooms.len(),
    );
}

fn update_monitors(
    scene: Res<RoguelikeScene>,
    states: Query<&GridFovState, With<DemoPlayer>>,
    mut pane: ResMut<RoguelikePane>,
    overlay: Query<&mut FogOverlay2d, With<FogOverlayMarker>>,
    mut commands: Commands,
) {
    if overlay.is_empty() {
        let mut fog_overlay = FogOverlay2d::new(
            FogLayerId(0),
            scene.map_spec.origin,
            scene.map_spec.world_size(),
        );
        fog_overlay.opacity = 1.0;
        fog_overlay.edge_softness = pane.fog_edge_softness;
        fog_overlay.z = 9.0;
        commands.spawn((Name::new("Fog Overlay"), FogOverlayMarker, fog_overlay));
    }

    let Ok(state) = states.get(scene.player_entity) else {
        return;
    };
    pane.visible_cells = state.visible_now.len();
    pane.explored_cells = state.explored.len();
}
