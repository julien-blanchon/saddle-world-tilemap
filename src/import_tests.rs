use super::*;
use crate::{TileKind, TileKindId, TilemapOrientation};
use std::collections::BTreeMap;

fn test_options() -> TiledImportOptions {
    let atlas = TileAtlasLayout::from_grid(
        Handle::<Image>::default(),
        UVec2::new(64, 64),
        UVec2::splat(16),
        4,
        4,
    );
    let mut catalog = TileCatalog::default();
    catalog.insert_kind(TileKindId::new(1), TileKind::static_tile("grass", 0));
    catalog.insert_kind(TileKindId::new(2), TileKind::static_tile("torch", 1));

    TiledImportOptions::new(
        atlas,
        catalog,
        BTreeMap::from([(1, TileKindId::new(1)), (2, TileKindId::new(2))]),
    )
}

#[test]
fn imports_basic_tiled_layers_and_objects() {
    let json = r#"
    {
      "orientation": "orthogonal",
      "tilewidth": 16,
      "tileheight": 16,
      "layers": [
        {
          "type": "tilelayer",
          "name": "Ground",
          "visible": true,
          "width": 2,
          "height": 2,
          "data": [1, 2147483650, 0, 1]
        },
        {
          "type": "objectgroup",
          "name": "Objects",
          "objects": [
            {
              "id": 7,
              "name": "Torch",
              "class": "LightSource",
              "x": 16,
              "y": 16,
              "gid": 2,
              "properties": [
                { "name": "faction", "value": "player" },
                { "name": "lit", "value": true }
              ]
            }
          ]
        }
      ]
    }
    "#;

    let imported = import_tiled_json_str(json, &test_options()).expect("imports");
    let layer = imported
        .map
        .layer(TileLayerId::new(1))
        .expect("ground layer");

    assert_eq!(
        imported
            .map
            .get_tile(TileLayerId::new(1), TileCoord::new(0, 0))
            .map(|tile| tile.kind),
        Some(TileKindId::new(1))
    );
    assert_eq!(
        imported
            .map
            .get_tile(TileLayerId::new(1), TileCoord::new(1, 0))
            .map(|tile| tile.orientation),
        Some(TileOrientation::MirrorH)
    );
    assert!(layer.config.visible);
    assert_eq!(imported.object_spawns.len(), 1);
    assert_eq!(imported.object_spawns[0].kind, Some(TileKindId::new(2)));
    assert_eq!(imported.object_spawns[0].coord, Some(TileCoord::new(1, 0)));
    assert_eq!(
        imported.object_spawns[0].properties.get("faction"),
        Some(&TilePropertyValue::Text("player".into()))
    );
}

#[test]
fn imports_hexagonal_orientation_from_tiled_stagger_metadata() {
    let json = r#"
    {
      "orientation": "hexagonal",
      "tilewidth": 72,
      "tileheight": 64,
      "staggeraxis": "y",
      "staggerindex": "odd",
      "layers": []
    }
    "#;

    let imported = import_tiled_json_str(json, &test_options()).expect("imports");
    assert!(matches!(
        imported.map.geometry.orientation,
        TilemapOrientation::HexFlatRows(TilemapHexParity::Odd)
    ));
}
