use super::*;

#[test]
fn layout_snapshot_quantizes_loop_density() {
    let pane = RoguelikePane {
        seed: 42,
        room_attempts: 224,
        loop_density: 0.2314,
        secret_rooms: false,
        ..default()
    };

    let snapshot = LayoutSnapshot::from_pane(&pane);
    assert_eq!(snapshot.seed, 42);
    assert_eq!(snapshot.room_attempts, 224);
    assert_eq!(snapshot.loop_density_milli, 231);
    assert!(!snapshot.secret_rooms);
}

#[test]
fn dungeon_config_tracks_pane_values() {
    let pane = RoguelikePane {
        seed: 73,
        room_attempts: 160,
        loop_density: 0.18,
        secret_rooms: true,
        ..default()
    };

    let config = dungeon_config(&pane);
    assert_eq!(config.width, GRID_DIMENSIONS.x);
    assert_eq!(config.height, GRID_DIMENSIONS.y);
    assert_eq!(config.seed.0, 73);
    assert_eq!(config.rooms_corridors.room_attempts, 160);
    assert_eq!(config.loop_density, 0.18);
    assert!(config.lock_key.enabled);
    assert!(config.secret_rooms.enabled);
}

#[test]
fn cardinal_direction_uses_dominant_axis_and_dead_zone() {
    assert_eq!(cardinal_direction(Vec2::new(0.0, 0.0)), IVec2::ZERO);
    assert_eq!(cardinal_direction(Vec2::new(0.2, 0.2)), IVec2::ZERO);
    assert_eq!(cardinal_direction(Vec2::new(0.9, 0.4)), IVec2::X);
    assert_eq!(cardinal_direction(Vec2::new(-0.7, 0.2)), -IVec2::X);
    assert_eq!(cardinal_direction(Vec2::new(0.3, -0.8)), -IVec2::Y);
}

#[test]
fn cell_center_places_the_origin_cell_in_the_first_tile() {
    assert_eq!(cell_center(IVec2::ZERO), Vec2::splat(TILE_SIZE * 0.5));
}
