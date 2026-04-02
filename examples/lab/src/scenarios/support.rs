use bevy::prelude::*;

pub fn named_entity_exists(world: &mut World, name: &str) -> bool {
    let mut query = world.query::<&Name>();
    query
        .iter(world)
        .any(|entity_name| entity_name.as_str() == name)
}

pub fn overlay_text(world: &mut World) -> Option<String> {
    let mut query = world.query::<(&Name, &Text)>();
    query
        .iter(world)
        .find_map(|(name, text)| (name.as_str() == "Overlay").then(|| text.0.clone()))
}

pub fn render_chunk_count(world: &mut World) -> usize {
    let mut query = world.query::<&saddle_world_tilemap::TilemapRenderChunk>();
    query.iter(world).count()
}

pub fn collision_chunk_count(world: &mut World) -> usize {
    let mut query = world.query::<&saddle_world_tilemap::TilemapCollisionChunk>();
    query.iter(world).count()
}
