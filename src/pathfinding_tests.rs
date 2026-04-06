use super::*;
use crate::{
    TileCatalog, TileCell, TileCollisionDescriptor, TileKind, TileKindId, TileLayerConfig,
    TileLayerId, TileLayerState, Tilemap, TilemapGeometry,
};
use bevy::prelude::*;

fn test_map() -> (Tilemap, TileLayerId) {
    let layer_id = TileLayerId::new(1);
    let geometry = TilemapGeometry::square(Vec2::splat(16.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(8));
    let mut catalog = TileCatalog::default();
    catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("floor", 0));
    catalog.insert_kind(
        TileKindId::new(2),
        TileKind::static_tile("wall", 1).with_collision(TileCollisionDescriptor::solid()),
    );
    catalog.insert_kind(
        TileKindId::new(3),
        TileKind::static_tile("mud", 2).with_movement_cost(3),
    );

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::logic_only(layer_id, "Ground"),
        catalog,
    ));

    for y in 0..10 {
        for x in 0..10 {
            map.set_tile(
                layer_id,
                TileCoord::new(x, y),
                TileCell::new(TileKindId::new(1)),
            );
        }
    }

    (map, layer_id)
}

fn layered_test_map() -> (Tilemap, TileLayerId, TileLayerId) {
    let ground_layer = TileLayerId::new(1);
    let collision_layer = TileLayerId::new(2);
    let geometry = TilemapGeometry::square(Vec2::splat(16.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(8));
    let mut ground_catalog = TileCatalog::default();
    ground_catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("floor", 0));
    ground_catalog.insert_kind(
        TileKindId::new(2),
        TileKind::static_tile("mud", 1).with_movement_cost(3),
    );
    let mut collision_catalog = TileCatalog::default();
    collision_catalog.insert_kind(
        TileKindId::new(10),
        TileKind::static_tile("wall", 2).with_collision(TileCollisionDescriptor::solid()),
    );

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::logic_only(ground_layer, "Ground"),
        ground_catalog,
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::logic_only(collision_layer, "Collision"),
        collision_catalog,
    ));

    for y in 0..10 {
        for x in 0..10 {
            map.set_tile(
                ground_layer,
                TileCoord::new(x, y),
                TileCell::new(TileKindId::new(1)),
            );
        }
    }

    (map, ground_layer, collision_layer)
}

#[test]
fn path_to_self() {
    let (map, layer) = test_map();
    let result = find_path(
        &map,
        layer,
        TileCoord::new(0, 0),
        TileCoord::new(0, 0),
        &TilePathOptions::default(),
    );
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.path.len(), 1);
    assert_eq!(result.total_cost, 0);
}

#[test]
fn straight_line_path() {
    let (map, layer) = test_map();
    let result = find_path(
        &map,
        layer,
        TileCoord::new(0, 0),
        TileCoord::new(5, 0),
        &TilePathOptions::default(),
    );
    assert!(result.is_some());
    let result = result.unwrap();
    assert_eq!(result.total_cost, 5);
    assert_eq!(*result.path.first().unwrap(), TileCoord::new(0, 0));
    assert_eq!(*result.path.last().unwrap(), TileCoord::new(5, 0));
}

#[test]
fn path_around_wall() {
    let (mut map, layer) = test_map();
    for y in 0..5 {
        map.set_tile(
            layer,
            TileCoord::new(3, y),
            TileCell::new(TileKindId::new(2)),
        );
    }
    let result = find_path(
        &map,
        layer,
        TileCoord::new(0, 0),
        TileCoord::new(5, 0),
        &TilePathOptions::default(),
    );
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(!result.path.iter().any(|c| c.x == 3 && c.y < 5));
}

#[test]
fn no_path_fully_blocked() {
    let (mut map, layer) = test_map();
    for y in 0..10 {
        map.set_tile(
            layer,
            TileCoord::new(5, y),
            TileCell::new(TileKindId::new(2)),
        );
    }
    let result = find_path(
        &map,
        layer,
        TileCoord::new(0, 0),
        TileCoord::new(9, 0),
        &TilePathOptions::default(),
    );
    assert!(result.is_none());
}

#[test]
fn prefers_low_cost() {
    let (mut map, layer) = test_map();
    for x in 2..8 {
        map.set_tile(
            layer,
            TileCoord::new(x, 3),
            TileCell::new(TileKindId::new(3)),
        );
    }
    let result = find_path(
        &map,
        layer,
        TileCoord::new(0, 3),
        TileCoord::new(9, 3),
        &TilePathOptions::default(),
    );
    assert!(result.is_some());
}

#[test]
fn max_cost_limits_path() {
    let (map, layer) = test_map();
    let result = find_path(
        &map,
        layer,
        TileCoord::new(0, 0),
        TileCoord::new(9, 9),
        &TilePathOptions::default().with_max_cost(5),
    );
    assert!(result.is_none());
}

#[test]
fn reachable_basic() {
    let (map, layer) = test_map();
    let reach = reachable_tiles(&map, layer, TileCoord::new(5, 5), 2, false);
    assert!(reach.contains_key(&TileCoord::new(5, 5)));
    assert!(reach.contains_key(&TileCoord::new(5, 3)));
    assert!(!reach.contains_key(&TileCoord::new(5, 2)));
}

#[test]
fn custom_policy_can_use_a_separate_collision_layer() {
    let (mut map, ground_layer, collision_layer) = layered_test_map();
    for y in 0..5 {
        map.set_tile(
            collision_layer,
            TileCoord::new(5, y),
            TileCell::new(TileKindId::new(10)),
        );
    }

    let default = find_path(
        &map,
        ground_layer,
        TileCoord::new(0, 0),
        TileCoord::new(9, 0),
        &TilePathOptions::default(),
    )
    .expect("default path should ignore separate collision layer");
    assert!(
        default
            .path
            .iter()
            .any(|coord| coord.x == 5 && (0..5).contains(&coord.y))
    );

    let policy = TilePathCallbacks::new(
        |step: &TilePathStep<'_>| step.map.get_tile(collision_layer, step.to).is_none(),
        |step: &TilePathStep<'_>| step.to_kind.map_or(1, |kind| kind.movement_cost as u32),
    );
    let custom = find_path_with_policy(
        &map,
        ground_layer,
        TileCoord::new(0, 0),
        TileCoord::new(9, 0),
        &TilePathOptions::default(),
        &policy,
    )
    .expect("custom policy should find a detour");

    assert!(
        !custom
            .path
            .iter()
            .any(|coord| coord.x == 5 && (0..5).contains(&coord.y)),
        "custom path crossed blocked tiles: {:?}",
        custom.path
    );
    assert!(custom.total_cost > default.total_cost);
}

#[test]
fn custom_policy_can_override_movement_costs() {
    let (mut map, layer) = test_map();
    for x in 2..8 {
        map.set_tile(
            layer,
            TileCoord::new(x, 3),
            TileCell::new(TileKindId::new(3)),
        );
    }

    let default = find_path(
        &map,
        layer,
        TileCoord::new(0, 3),
        TileCoord::new(9, 3),
        &TilePathOptions::default(),
    )
    .expect("default path should exist");

    let policy = TilePathCallbacks::new(
        |_step: &TilePathStep<'_>| true,
        |step: &TilePathStep<'_>| match step.to_kind.map(|kind| kind.name.as_str()) {
            Some("mud") => 1,
            _ => step.to_kind.map_or(1, |kind| kind.movement_cost as u32),
        },
    );
    let custom = find_path_with_policy(
        &map,
        layer,
        TileCoord::new(0, 3),
        TileCoord::new(9, 3),
        &TilePathOptions::default(),
        &policy,
    )
    .expect("custom-cost path should exist");

    assert!(custom.total_cost < default.total_cost);
    assert!(
        custom
            .path
            .iter()
            .any(|coord| coord.y == 3 && (2..8).contains(&coord.x))
    );
}

#[test]
fn reachable_tiles_with_policy_respects_custom_blockers() {
    let (mut map, ground_layer, collision_layer) = layered_test_map();
    for x in 4..7 {
        map.set_tile(
            collision_layer,
            TileCoord::new(x, 5),
            TileCell::new(TileKindId::new(10)),
        );
    }

    let policy = TilePathCallbacks::new(
        |step: &TilePathStep<'_>| step.map.get_tile(collision_layer, step.to).is_none(),
        |step: &TilePathStep<'_>| step.to_kind.map_or(1, |kind| kind.movement_cost as u32),
    );
    let reach =
        reachable_tiles_with_policy(&map, ground_layer, TileCoord::new(5, 3), 4, false, &policy);

    assert!(!reach.contains_key(&TileCoord::new(5, 5)));
    assert!(reach.contains_key(&TileCoord::new(5, 4)));
}
