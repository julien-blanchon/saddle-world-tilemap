use super::*;
use crate::{
    TileCatalog, TileCell, TileKind, TileKindId, TileLayerConfig, TileLayerId,
    TileLayerRenderConfig, TileLayerState, Tilemap, TilemapGeometry,
};

fn test_map() -> Tilemap {
    let image = Handle::<Image>::default();
    let atlas =
        crate::TileAtlasLayout::from_grid(image, UVec2::new(64, 16), UVec2::splat(16), 4, 1);
    let render = TileLayerRenderConfig::new(atlas);
    let mut catalog = TileCatalog::default();
    catalog.insert_autotile_rule(
        AutotileRuleSetId::new(1),
        AutotileRuleSet::new(AutotileNeighborhood::Cardinal4, 0)
            .with_variant(0b1111, 3)
            .with_variant(0b0101, 2)
            .with_variant(0b1010, 1),
    );
    catalog.insert_kind(
        TileKindId::new(1),
        TileKind::autotile(
            "road",
            AutotileBinding {
                group: AutotileGroupId::new(7),
                rule_set: AutotileRuleSetId::new(1),
                fallback_atlas_index: 0,
            },
        ),
    );

    Tilemap::new(TilemapGeometry::square(Vec2::splat(16.0)), UVec2::splat(8)).with_layer(
        TileLayerState::new(
            TileLayerConfig::visual(TileLayerId::new(1), "ground", render),
            catalog,
        ),
    )
}

#[test]
fn cardinal_mask_is_computed_in_nesw_order() {
    let mut map = test_map();
    let layer = TileLayerId::new(1);
    let road = TileCell::new(TileKindId::new(1));
    let center = TileCoord::new(10, 10);

    map.set_tile(layer, center, road.clone());
    map.set_tile(layer, center.offset(0, -1), road.clone());
    map.set_tile(layer, center.offset(1, 0), road.clone());
    map.set_tile(layer, center.offset(0, 1), road.clone());

    assert_eq!(
        compute_autotile_mask(&map, layer, center, AutotileGroupId::new(7)),
        0b0111
    );
}

#[test]
fn rule_set_uses_fallback_when_mask_is_missing() {
    let rule = AutotileRuleSet::new(AutotileNeighborhood::Cardinal4, 9).with_variant(1, 3);
    assert_eq!(rule.resolve(7), 9);
}
