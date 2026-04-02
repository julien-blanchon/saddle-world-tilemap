use super::*;
use crate::{
    AutotileBinding, AutotileGroupId, AutotileNeighborhood, AutotileRuleSet, AutotileRuleSetId,
    TileCatalog, TileKind, TileKindId,
};

fn test_layer_catalog() -> TileCatalog {
    let mut catalog = TileCatalog::default();
    catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("grass", 0));
    catalog.insert_kind(TileKindId::new(2), TileKind::static_tile("soil", 1));
    catalog.insert_autotile_rule(
        AutotileRuleSetId::new(1),
        AutotileRuleSet::new(AutotileNeighborhood::Cardinal4, 0)
            .with_variant(0b1111, 3)
            .with_variant(0b0101, 2),
    );
    catalog.insert_kind(
        TileKindId::new(3),
        TileKind::autotile(
            "road",
            AutotileBinding {
                group: AutotileGroupId::new(5),
                rule_set: AutotileRuleSetId::new(1),
                fallback_atlas_index: 0,
            },
        ),
    );
    catalog
}

fn test_layer() -> Tilemap {
    Tilemap::new(TilemapGeometry::square(Vec2::splat(16.0)), UVec2::splat(4)).with_layer(
        TileLayerState::new(
            TileLayerConfig::logic_only(TileLayerId::new(1), "ground"),
            test_layer_catalog(),
        ),
    )
}

#[test]
fn swap_tiles_moves_values_between_coordinates() {
    let mut map = test_layer();
    let layer = TileLayerId::new(1);
    let first = TileCoord::new(0, 0);
    let second = TileCoord::new(5, 0);
    map.set_tile(layer, first, TileCell::new(TileKindId::new(1)));
    map.set_tile(layer, second, TileCell::new(TileKindId::new(2)));

    {
        let layer = map.layer_mut(layer).expect("layer");
        layer.dirty_resolve.clear();
        layer.dirty_render.clear();
        layer.dirty_collision.clear();
    }

    map.swap_tiles(layer, first, second);

    assert_eq!(
        map.get_tile(layer, first).map(|tile| tile.kind),
        Some(TileKindId::new(2))
    );
    assert_eq!(
        map.get_tile(layer, second).map(|tile| tile.kind),
        Some(TileKindId::new(1))
    );

    let layer = map.layer(layer).expect("layer");
    assert!(layer.dirty_resolve.contains(&ChunkCoord::new(0, 0)));
    assert!(layer.dirty_resolve.contains(&ChunkCoord::new(1, 0)));
}

#[test]
fn autotile_boundary_edit_marks_neighbor_chunk_dirty() {
    let mut map = test_layer();
    let layer = TileLayerId::new(1);
    let road = TileCell::new(TileKindId::new(3));

    map.set_tile(layer, TileCoord::new(3, 1), road.clone());
    map.set_tile(layer, TileCoord::new(3, 2), road.clone());
    map.set_tile(layer, TileCoord::new(4, 1), road);

    {
        let layer = map.layer_mut(layer).expect("layer");
        layer.dirty_resolve.clear();
        layer.dirty_render.clear();
        layer.dirty_collision.clear();
    }

    map.clear_tile(layer, TileCoord::new(3, 1));

    let layer = map.layer(layer).expect("layer");
    assert!(layer.dirty_resolve.contains(&ChunkCoord::new(0, 0)));
    assert!(layer.dirty_resolve.contains(&ChunkCoord::new(1, 0)));
}

#[test]
fn layer_visibility_toggles_in_logical_state() {
    let mut map = test_layer();
    let layer = TileLayerId::new(1);

    assert!(map.layer(layer).expect("layer").config.visible);
    map.set_layer_visibility(layer, false);
    assert!(!map.layer(layer).expect("layer").config.visible);
}
