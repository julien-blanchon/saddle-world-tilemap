use bevy::prelude::*;
use saddle_world_tilemap::{TileCoord, TilemapCommand};

use crate::{CameraFocus, LabControl};

pub fn entity_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .find_map(|(entity, entity_name)| (entity_name.as_str() == name).then_some(entity))
}

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

pub fn set_camera_focus(world: &mut World, focus: CameraFocus) {
    world.resource_mut::<LabControl>().camera_focus = focus;
}

pub fn set_square_edit_stage(world: &mut World, stage: u8) {
    world.resource_mut::<LabControl>().square_edit_stage = stage;
}

pub fn set_iso_selection(world: &mut World, selection: TileCoord) {
    world.resource_mut::<LabControl>().iso_selection = selection;
}

pub fn set_square_showcase_layer_visible(world: &mut World, visible: bool) {
    let square_map = entity_by_name(world, "Square Showcase Map")
        .expect("Square Showcase Map entity should exist");
    world
        .resource_mut::<Messages<TilemapCommand>>()
        .write(TilemapCommand::SetLayerVisibility {
            map: square_map,
            layer: saddle_world_tilemap_example_support::HIGHLIGHT_LAYER,
            visible,
        });
}
