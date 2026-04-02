use saddle_world_tilemap::{
    ChunkCoord, TileAnimation, TileCell, TileChunk, TileCoord, TileKind, TileKindId, TileLayerId,
    TileOrientation, TileRect, TilemapGeometry, TilemapPlugin, TilemapRoot,
};

#[test]
fn public_api_is_constructible() {
    let _plugin = TilemapPlugin::default();
    let _geometry = TilemapGeometry::square(bevy::prelude::Vec2::splat(16.0));
    let _chunk_data = TileChunk::new(bevy::prelude::UVec2::splat(4));
    let _chunk = ChunkCoord::new(1, -1);
    let _rect = TileRect::new(TileCoord::new(0, 0), bevy::prelude::UVec2::splat(4));
    let _tile = TileCell::new(TileKindId::new(1));
    let _orientation = TileOrientation::Rotate90;
    let _kind = TileKind::animated_tile("water", TileAnimation::uniform([0, 1, 2], 0.1));
    let _layer = TileLayerId::new(2);
}

#[test]
fn plugin_registers_brp_facing_reflect_types() {
    use std::any::TypeId;

    use bevy::prelude::AppTypeRegistry;

    let mut app = bevy::prelude::App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(TilemapPlugin::always_on(bevy::prelude::Update));

    let registry = app.world().resource::<AppTypeRegistry>().read();
    assert!(registry.get(TypeId::of::<TilemapRoot>()).is_some());
    assert!(registry.get(TypeId::of::<TileChunk>()).is_some());
    assert!(registry.get(TypeId::of::<TileOrientation>()).is_some());
}
