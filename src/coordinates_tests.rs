use super::*;

#[test]
fn chunk_math_uses_euclidean_division() {
    let chunk_size = UVec2::splat(16);
    let coord = TileCoord::new(-1, -17);

    assert_eq!(coord.chunk(chunk_size), ChunkCoord::new(-1, -2));
    assert_eq!(coord.local_in_chunk(chunk_size), UVec2::new(15, 15));
}

#[test]
fn square_geometry_round_trips() {
    let geometry = TilemapGeometry::square(Vec2::new(32.0, 24.0));
    let coord = TileCoord::new(4, 3);
    let local = geometry.tile_to_local(coord);

    assert_eq!(geometry.local_to_tile(local), coord);
}

#[test]
fn isometric_geometry_round_trips() {
    let geometry = TilemapGeometry::isometric_diamond(Vec2::new(64.0, 32.0));
    let coord = TileCoord::new(3, 2);
    let local = geometry.tile_to_local(coord);

    assert_eq!(geometry.local_to_tile(local), coord);
}

#[test]
fn hex_pointy_geometry_round_trips() {
    let geometry =
        TilemapGeometry::hex_pointy_columns(Vec2::new(72.0, 80.0), TilemapHexParity::Odd);

    for coord in [
        TileCoord::new(0, 0),
        TileCoord::new(1, 0),
        TileCoord::new(2, 3),
        TileCoord::new(-1, 2),
    ] {
        let local = geometry.tile_to_local(coord);
        assert_eq!(geometry.local_to_tile(local), coord);
    }
}

#[test]
fn hex_flat_geometry_round_trips() {
    let geometry = TilemapGeometry::hex_flat_rows(Vec2::new(80.0, 68.0), TilemapHexParity::Even);

    for coord in [
        TileCoord::new(0, 0),
        TileCoord::new(1, 1),
        TileCoord::new(3, 2),
        TileCoord::new(-2, 1),
    ] {
        let local = geometry.tile_to_local(coord);
        assert_eq!(geometry.local_to_tile(local), coord);
    }
}

#[test]
fn chunk_bounds_include_render_size() {
    let geometry =
        TilemapGeometry::square(Vec2::splat(16.0)).with_tile_render_size(Vec2::splat(20.0));
    let rect = geometry.chunk_bounds_local(UVec2::splat(2), ChunkCoord::ZERO);

    assert!(rect.width() > 16.0);
    assert!(rect.height() > 16.0);
}
