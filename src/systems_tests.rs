use crate::{
    AutotileBinding, AutotileGroupId, AutotileNeighborhood, AutotileRuleSet, AutotileRuleSetId,
    TileAnimation, TileAtlasLayout, TileCatalog, TileCell, TileChanged, TileCollisionDescriptor,
    TileKind, TileKindId, TileLayerConfig, TileLayerId, TileLayerRenderConfig, TileLayerState,
    Tilemap, TilemapBundle, TilemapCollisionChunk, TilemapCommand, TilemapGeometry,
    TilemapLayerNode, TilemapPlugin, TilemapRenderChunk, TilemapRoot, TilemapSystems,
    rendering::TilemapRuntimeComponent,
};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

fn build_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(TilemapPlugin::always_on(Update));
    app.init_resource::<Assets<ColorMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.world_mut()
        .resource_mut::<crate::systems::TilemapRuntimeControl>()
        .active = true;
    app
}

fn spawn_test_map(app: &mut App) -> Entity {
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let atlas = TileAtlasLayout::from_grid(image, UVec2::new(64, 16), UVec2::splat(16), 4, 1);
    let render = TileLayerRenderConfig::new(atlas);

    let mut catalog = TileCatalog::default();
    catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("grass", 0));
    catalog.insert_kind(
        TileKindId::new(4),
        TileKind::static_tile("wall", 0).with_collision(TileCollisionDescriptor::solid()),
    );
    catalog.insert_kind(
        TileKindId::new(2),
        TileKind::animated_tile("water", TileAnimation::uniform([1, 2], 0.05)),
    );
    catalog.insert_autotile_rule(
        AutotileRuleSetId::new(1),
        AutotileRuleSet::new(AutotileNeighborhood::Cardinal4, 0).with_variant(0b1111, 3),
    );
    catalog.insert_kind(
        TileKindId::new(3),
        TileKind::autotile(
            "road",
            AutotileBinding {
                group: AutotileGroupId::new(1),
                rule_set: AutotileRuleSetId::new(1),
                fallback_atlas_index: 0,
            },
        ),
    );

    let mut map = Tilemap::new(TilemapGeometry::square(Vec2::splat(16.0)), UVec2::splat(4))
        .with_layer(TileLayerState::new(
            TileLayerConfig::visual(TileLayerId::new(1), "ground", render),
            catalog.clone(),
        ))
        .with_layer(TileLayerState::new(
            TileLayerConfig::logic_only(TileLayerId::new(2), "collision"),
            catalog,
        ));
    map.set_tile(
        TileLayerId::new(1),
        crate::TileCoord::new(0, 0),
        TileCell::new(TileKindId::new(1)),
    );

    app.world_mut()
        .spawn(TilemapBundle::new("Test Map", map))
        .id()
}

fn spawn_logic_only_map(app: &mut App) -> Entity {
    let mut catalog = TileCatalog::default();
    catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("grass", 0));

    let mut map = Tilemap::new(TilemapGeometry::square(Vec2::splat(16.0)), UVec2::splat(4))
        .with_layer(TileLayerState::new(
            TileLayerConfig::logic_only(TileLayerId::new(1), "logic"),
            catalog,
        ));
    map.set_tile(
        TileLayerId::new(1),
        crate::TileCoord::new(0, 0),
        TileCell::new(TileKindId::new(1)),
    );

    app.world_mut()
        .spawn(TilemapBundle::new("Logic Only Map", map))
        .id()
}

#[derive(Resource, Default)]
struct ObservedTileChanges(Vec<TileChanged>);

fn collect_tile_changes(
    mut reader: MessageReader<TileChanged>,
    mut observed: ResMut<ObservedTileChanges>,
) {
    observed.0.extend(reader.read().cloned());
}

#[test]
fn runtime_tile_edit_marks_and_builds_render_chunk() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);

    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SetTile {
            map,
            layer: TileLayerId::new(1),
            coord: crate::TileCoord::new(5, 5),
            tile: TileCell::new(TileKindId::new(1)),
        });

    app.update();

    let world = app.world_mut();
    let mut query = world.query::<&TilemapRenderChunk>();
    let chunk_count = query.iter(world).count();
    assert!(chunk_count >= 1);
}

#[test]
fn only_affected_chunk_is_rebuilt_on_single_tile_edit() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SetTile {
            map,
            layer: TileLayerId::new(1),
            coord: crate::TileCoord::new(1, 1),
            tile: TileCell::new(TileKindId::new(1)),
        });
    app.update();

    let world = app.world_mut();
    let mut query = world.query_filtered::<&crate::TilemapDiagnostics, With<TilemapRoot>>();
    let diagnostics = query.single(world).expect("tilemap diagnostics");
    assert_eq!(diagnostics.chunks_rebuilt_this_frame, 1);
}

#[test]
fn animation_advances_and_rebuilds_affected_chunk() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);
    app.update();
    {
        let mut query = app.world_mut().query::<&mut Tilemap>();
        let mut tilemap = query.single_mut(app.world_mut()).expect("tilemap");
        tilemap.set_tile(
            TileLayerId::new(1),
            crate::TileCoord::new(2, 0),
            TileCell::new(TileKindId::new(2)),
        );
    }
    app.update();

    let initial_revision = {
        let world = app.world_mut();
        let mut query = world.query::<&TilemapRenderChunk>();
        query.single(world).expect("render chunk").revision
    };

    *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
        TimeUpdateStrategy::ManualDuration(Duration::from_millis(60));
    app.update();

    let world = app.world_mut();
    let mut query = world.query::<&TilemapRuntimeComponent>();
    let runtime = query.single(world).expect("runtime");
    assert!(
        runtime
            .0
            .animation_states
            .contains_key(&(TileLayerId::new(1), TileKindId::new(2)))
    );
    let mut render_chunks = world.query::<&TilemapRenderChunk>();
    let updated_revision = render_chunks.single(world).expect("render chunk").revision;
    assert!(updated_revision > initial_revision);
    assert!(app.world().entities().contains(map));
}

#[test]
fn collision_only_edits_sync_collision_chunks() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SetTile {
            map,
            layer: TileLayerId::new(2),
            coord: crate::TileCoord::new(6, 1),
            tile: TileCell::new(TileKindId::new(4)),
        });
    app.update();

    let world = app.world_mut();
    let mut query = world.query::<&TilemapCollisionChunk>();
    let collision_chunk = query.single(world).expect("collision chunk");
    assert_eq!(collision_chunk.layer, TileLayerId::new(2));
    assert_eq!(collision_chunk.cells.len(), 1);
    assert_eq!(collision_chunk.cells[0].coord, crate::TileCoord::new(6, 1));
}

#[test]
fn swap_command_updates_both_tiles() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);
    {
        let mut query = app.world_mut().query::<&mut Tilemap>();
        let mut tilemap = query.single_mut(app.world_mut()).expect("tilemap");
        tilemap.set_tile(
            TileLayerId::new(1),
            crate::TileCoord::new(5, 0),
            TileCell::new(TileKindId::new(2)),
        );
    }
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SwapTiles {
            map,
            layer: TileLayerId::new(1),
            a: crate::TileCoord::new(0, 0),
            b: crate::TileCoord::new(5, 0),
        });
    app.update();

    let mut query = app.world_mut().query::<&Tilemap>();
    let tilemap = query.single(app.world_mut()).expect("tilemap");
    assert_eq!(
        tilemap
            .get_tile(TileLayerId::new(1), crate::TileCoord::new(0, 0))
            .map(|tile| tile.kind),
        Some(TileKindId::new(2))
    );
    assert_eq!(
        tilemap
            .get_tile(TileLayerId::new(1), crate::TileCoord::new(5, 0))
            .map(|tile| tile.kind),
        Some(TileKindId::new(1))
    );
}

#[test]
fn visibility_commands_hide_layer_nodes() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SetLayerVisibility {
            map,
            layer: TileLayerId::new(1),
            visible: false,
        });
    app.update();

    let world = app.world_mut();
    let mut query = world.query::<(&TilemapLayerNode, &Visibility)>();
    let (_, visibility) = query
        .iter(world)
        .find(|(node, _)| node.layer == TileLayerId::new(1))
        .expect("layer node");
    assert_eq!(*visibility, Visibility::Hidden);
}

#[test]
fn logic_only_maps_do_not_require_render_assets() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(TilemapPlugin::always_on(Update));
    app.world_mut()
        .resource_mut::<crate::systems::TilemapRuntimeControl>()
        .active = true;

    let map = spawn_logic_only_map(&mut app);
    app.update();
    app.update();

    assert!(app.world().entities().contains(map));
    let world = app.world_mut();
    let mut query = world.query::<&TilemapRenderChunk>();
    assert_eq!(query.iter(world).count(), 0);
}

#[test]
fn collision_chunk_total_persists_after_the_rebuild_frame() {
    let mut app = build_test_app();
    let map = spawn_test_map(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SetTile {
            map,
            layer: TileLayerId::new(2),
            coord: crate::TileCoord::new(6, 1),
            tile: TileCell::new(TileKindId::new(4)),
        });
    app.update();
    app.update();

    let world = app.world_mut();
    let mut query = world.query_filtered::<&crate::TilemapDiagnostics, With<TilemapRoot>>();
    let diagnostics = query.single(world).expect("tilemap diagnostics");
    assert_eq!(diagnostics.collision_chunks_total, 1);
}

#[test]
fn fill_circle_command_reports_every_tile_change() {
    let mut app = build_test_app();
    app.insert_resource(ObservedTileChanges::default());
    app.add_systems(
        Update,
        collect_tile_changes.after(TilemapSystems::ApplyCommands),
    );

    let map = spawn_test_map(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::FillCircle {
            map,
            layer: TileLayerId::new(1),
            center: crate::TileCoord::new(6, 6),
            radius: 1,
            tile: TileCell::new(TileKindId::new(1)),
        });
    app.update();

    let observed = &app.world().resource::<ObservedTileChanges>().0;
    assert_eq!(observed.len(), 5);

    let world = app.world_mut();
    let mut query = world.query_filtered::<&crate::TilemapDiagnostics, With<TilemapRoot>>();
    let diagnostics = query.single(world).expect("tilemap diagnostics");
    assert_eq!(diagnostics.tile_edits_this_frame, 5);
}

#[test]
fn flood_fill_command_reports_every_tile_change() {
    let mut app = build_test_app();
    app.insert_resource(ObservedTileChanges::default());
    app.add_systems(
        Update,
        collect_tile_changes.after(TilemapSystems::ApplyCommands),
    );

    let map = spawn_test_map(&mut app);
    app.update();

    app.world_mut()
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::FloodFill {
            map,
            layer: TileLayerId::new(1),
            start: crate::TileCoord::new(8, 8),
            tile: TileCell::new(TileKindId::new(1)),
            max_tiles: 4,
        });
    app.update();

    let observed = &app.world().resource::<ObservedTileChanges>().0;
    assert_eq!(observed.len(), 4);

    let world = app.world_mut();
    let mut query = world.query_filtered::<&crate::TilemapDiagnostics, With<TilemapRoot>>();
    let diagnostics = query.single(world).expect("tilemap diagnostics");
    assert_eq!(diagnostics.tile_edits_this_frame, 4);
}
