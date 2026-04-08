use crate::{
    ChunkRebuilt, LayerVisibilityChanged, TileAnimationLooped, TileCell, TileChanged,
    TileCollisionCell, TileLayerId, Tilemap, TilemapCollisionChunk, TilemapCommand,
    TilemapDiagnostics, TilemapLayerNode, TilemapRenderChunk,
    animation::TileAnimationRuntimeState,
    chunk::{ResolvedTileVisual, TileChunk},
    layer::{bresenham_line, fill_circle_coords, flood_fill_coords},
    rendering::{
        TilemapRuntimeComponent, build_chunk_mesh, build_color_material, chunk_local_translation,
        multiply_colors, resolve_static_visual,
    },
};
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct TilemapRuntimeControl {
    pub active: bool,
}

pub(crate) fn activate_runtime(mut runtime: ResMut<TilemapRuntimeControl>) {
    runtime.active = true;
}

pub(crate) fn deactivate_runtime(
    mut commands: Commands,
    mut runtime: ResMut<TilemapRuntimeControl>,
    mut maps: Query<(Entity, &mut Tilemap, Option<&mut TilemapRuntimeComponent>)>,
) {
    runtime.active = false;

    for (_, mut map, runtime_component) in &mut maps {
        map.mark_all_dirty();
        if let Some(mut runtime_component) = runtime_component {
            for entity in runtime_component
                .0
                .render_chunks
                .values()
                .chain(runtime_component.0.collision_chunks.values())
                .chain(runtime_component.0.layer_nodes.values())
            {
                commands.entity(*entity).despawn();
            }
            runtime_component.0 = Default::default();
        }
    }
}

pub(crate) fn runtime_is_active(runtime: Res<TilemapRuntimeControl>) -> bool {
    runtime.active
}

pub(crate) fn render_backend_is_available(
    meshes: Option<Res<Assets<Mesh>>>,
    materials: Option<Res<Assets<ColorMaterial>>>,
) -> bool {
    meshes.is_some() && materials.is_some()
}

pub(crate) fn prepare_maps(
    mut commands: Commands,
    mut maps: Query<
        (
            Entity,
            &mut Tilemap,
            &mut TilemapDiagnostics,
            Option<&mut TilemapRuntimeComponent>,
        ),
        With<crate::TilemapRoot>,
    >,
) {
    for (map_entity, mut map, mut diagnostics, runtime_component) in &mut maps {
        diagnostics.logical_chunks_total =
            map.layers.values().map(|layer| layer.chunks.len()).sum();
        let mut dirty_chunks = BTreeSet::new();
        for layer in map.layers.values() {
            dirty_chunks.extend(layer.dirty_resolve.iter().copied());
            dirty_chunks.extend(layer.dirty_render.iter().copied());
            dirty_chunks.extend(layer.dirty_collision.iter().copied());
        }
        diagnostics.dirty_chunks = dirty_chunks.len();
        diagnostics.chunks_rebuilt_this_frame = 0;
        diagnostics.collision_chunks_total = 0;
        diagnostics.animated_chunks_total = map
            .layers
            .values()
            .map(|layer| {
                layer
                    .chunks
                    .values()
                    .filter(|chunk| !chunk.animated_kinds.is_empty())
                    .count()
            })
            .sum();
        diagnostics.tile_edits_this_frame = 0;

        let mut runtime_state = if let Some(runtime_component) = runtime_component {
            runtime_component
        } else {
            commands
                .entity(map_entity)
                .insert(TilemapRuntimeComponent::default());
            continue;
        };

        if !runtime_state.0.initialized {
            map.mark_all_dirty();
            runtime_state.0.initialized = true;
        }

        for (&layer_id, layer) in &map.layers {
            runtime_state
                .0
                .layer_nodes
                .entry(layer_id)
                .or_insert_with(|| {
                    commands
                        .spawn((
                            Name::new(format!("Tilemap Layer {}", layer.config.name)),
                            TilemapLayerNode {
                                map: map_entity,
                                layer: layer_id,
                            },
                            Transform::from_translation(layer.config.offset.extend(0.0)),
                            GlobalTransform::default(),
                            if layer.config.visible {
                                Visibility::Visible
                            } else {
                                Visibility::Hidden
                            },
                            InheritedVisibility::VISIBLE,
                            ViewVisibility::default(),
                        ))
                        .set_parent_in_place(map_entity)
                        .id()
                });
        }
    }
}

pub(crate) fn apply_commands(
    mut commands_in: MessageReader<TilemapCommand>,
    mut changed: MessageWriter<TileChanged>,
    mut visibility_changed: MessageWriter<LayerVisibilityChanged>,
    mut maps: Query<(&mut Tilemap, &mut TilemapDiagnostics)>,
) {
    for command in commands_in.read() {
        match command {
            TilemapCommand::SetTile {
                map,
                layer,
                coord,
                tile,
            } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                let previous_tile = tilemap.get_tile(*layer, *coord).cloned();
                tilemap.set_tile(*layer, *coord, tile.clone());
                let next_tile = tilemap.get_tile(*layer, *coord).cloned();
                record_tile_change(
                    &mut changed,
                    &mut diagnostics,
                    *map,
                    *layer,
                    *coord,
                    previous_tile,
                    next_tile,
                );
            }
            TilemapCommand::ClearTile { map, layer, coord } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                let previous_tile = tilemap.get_tile(*layer, *coord).cloned();
                tilemap.clear_tile(*layer, *coord);
                let next_tile = tilemap.get_tile(*layer, *coord).cloned();
                record_tile_change(
                    &mut changed,
                    &mut diagnostics,
                    *map,
                    *layer,
                    *coord,
                    previous_tile,
                    next_tile,
                );
            }
            TilemapCommand::FillRect {
                map,
                layer,
                rect,
                tile,
            } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                apply_tile_batch(
                    &mut tilemap,
                    &mut diagnostics,
                    &mut changed,
                    *map,
                    *layer,
                    rect.iter(),
                    tile,
                );
            }
            TilemapCommand::SwapTiles { map, layer, a, b } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                let previous_a = tilemap.get_tile(*layer, *a).cloned();
                let previous_b = tilemap.get_tile(*layer, *b).cloned();
                tilemap.swap_tiles(*layer, *a, *b);
                let next_a = tilemap.get_tile(*layer, *a).cloned();
                let next_b = tilemap.get_tile(*layer, *b).cloned();

                record_tile_change(
                    &mut changed,
                    &mut diagnostics,
                    *map,
                    *layer,
                    *a,
                    previous_a,
                    next_a,
                );
                record_tile_change(
                    &mut changed,
                    &mut diagnostics,
                    *map,
                    *layer,
                    *b,
                    previous_b,
                    next_b,
                );
            }
            TilemapCommand::SetLayerVisibility {
                map,
                layer,
                visible,
            } => {
                let Ok((mut tilemap, _)) = maps.get_mut(*map) else {
                    continue;
                };
                tilemap.set_layer_visibility(*layer, *visible);
                visibility_changed.write(LayerVisibilityChanged {
                    map: *map,
                    layer: *layer,
                    visible: *visible,
                });
            }
            TilemapCommand::FillCircle {
                map,
                layer,
                center,
                radius,
                tile,
            } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                apply_tile_batch(
                    &mut tilemap,
                    &mut diagnostics,
                    &mut changed,
                    *map,
                    *layer,
                    fill_circle_coords(*center, *radius),
                    tile,
                );
            }
            TilemapCommand::FillLine {
                map,
                layer,
                from,
                to,
                tile,
            } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                apply_tile_batch(
                    &mut tilemap,
                    &mut diagnostics,
                    &mut changed,
                    *map,
                    *layer,
                    bresenham_line(*from, *to),
                    tile,
                );
            }
            TilemapCommand::FloodFill {
                map,
                layer,
                start,
                tile,
                max_tiles,
            } => {
                let Ok((mut tilemap, mut diagnostics)) = maps.get_mut(*map) else {
                    continue;
                };
                let coords = flood_fill_coords(&tilemap, *layer, *start, tile.kind, *max_tiles);
                apply_tile_batch(
                    &mut tilemap,
                    &mut diagnostics,
                    &mut changed,
                    *map,
                    *layer,
                    coords,
                    tile,
                );
            }
        }
    }
}

pub(crate) fn advance_animation(
    time: Res<Time>,
    mut loops: MessageWriter<TileAnimationLooped>,
    mut maps: Query<(Entity, &mut Tilemap, &mut TilemapRuntimeComponent)>,
) {
    for (map_entity, mut map, mut runtime_component) in &mut maps {
        let layer_ids: Vec<_> = map.layers.keys().copied().collect();
        for layer_id in layer_ids {
            let animated_kinds: Vec<_> = map
                .layers
                .get(&layer_id)
                .map(|layer| {
                    layer
                        .catalog
                        .kinds
                        .iter()
                        .filter_map(|(&kind_id, kind)| {
                            kind.render
                                .animation()
                                .map(|animation| (kind_id, animation.clone()))
                        })
                        .collect()
                })
                .unwrap_or_default();

            for (kind_id, animation) in animated_kinds {
                let state = runtime_component
                    .0
                    .animation_states
                    .entry((layer_id, kind_id))
                    .or_insert(TileAnimationRuntimeState {
                        elapsed_seconds: 0.0,
                        frame_index: animation.frame_index_at(0.0),
                    });

                let previous_frame = state.frame_index;
                state.elapsed_seconds += time.delta_secs();
                state.frame_index = animation.frame_index_at(state.elapsed_seconds);

                if state.frame_index != previous_frame {
                    if let Some(layer) = map.layers.get_mut(&layer_id) {
                        let dirty_chunks: Vec<_> = layer
                            .chunks
                            .iter()
                            .filter_map(|(&chunk_coord, chunk)| {
                                chunk
                                    .animated_kinds
                                    .contains(&kind_id)
                                    .then_some(chunk_coord)
                            })
                            .collect();
                        layer.dirty_resolve.extend(dirty_chunks);
                    }
                }

                if state.frame_index < previous_frame {
                    loops.write(TileAnimationLooped {
                        map: map_entity,
                        layer: layer_id,
                        kind: kind_id,
                    });
                }
            }
        }
    }
}

pub(crate) fn resolve_dirty_chunks(mut maps: Query<(&mut Tilemap, &mut TilemapRuntimeComponent)>) {
    for (mut map, runtime_component) in &mut maps {
        let layer_ids: Vec<TileLayerId> = map.layers.keys().copied().collect();
        for layer_id in layer_ids {
            let dirty: Vec<_> = map.layers[&layer_id]
                .dirty_resolve
                .iter()
                .copied()
                .collect();
            for chunk_coord in &dirty {
                let resolved =
                    resolve_chunk_snapshot(&map, &runtime_component.0, layer_id, *chunk_coord);
                let Some(layer) = map.layers.get_mut(&layer_id) else {
                    continue;
                };

                let Some(chunk) = layer.chunks.get_mut(chunk_coord) else {
                    layer.dirty_render.insert(*chunk_coord);
                    layer.dirty_collision.insert(*chunk_coord);
                    continue;
                };

                if let Some(resolved) = resolved {
                    chunk.resolved_visuals = resolved.visuals;
                    chunk.resolved_collisions = resolved.collisions;
                    chunk.animated_kinds = resolved.animated_kinds;
                    chunk.revision += 1;
                }

                layer.dirty_render.insert(*chunk_coord);
                layer.dirty_collision.insert(*chunk_coord);
            }

            if let Some(layer) = map.layers.get_mut(&layer_id) {
                layer.dirty_resolve.clear();
            }
        }
    }
}

pub(crate) fn sync_collision_chunks(
    mut commands: Commands,
    mut rebuilt: MessageWriter<ChunkRebuilt>,
    mut maps: Query<
        (
            Entity,
            &mut Tilemap,
            &mut TilemapDiagnostics,
            &mut TilemapRuntimeComponent,
        ),
        With<crate::TilemapRoot>,
    >,
) {
    for (map_entity, mut map, mut diagnostics, mut runtime_component) in &mut maps {
        let layer_ids: Vec<_> = map.layers.keys().copied().collect();
        for layer_id in layer_ids {
            let dirty: Vec<_> = map.layers[&layer_id]
                .dirty_collision
                .iter()
                .copied()
                .collect();
            for chunk_coord in &dirty {
                let entity_key = (layer_id, *chunk_coord);
                let Some(layer) = map.layers.get(&layer_id) else {
                    continue;
                };
                let cells = layer
                    .chunks
                    .get(chunk_coord)
                    .map(|chunk| {
                        chunk
                            .resolved_collisions
                            .iter()
                            .enumerate()
                            .filter_map(|(index, descriptor)| {
                                let descriptor = descriptor.clone()?;
                                let local_x = (index as u32) % map.chunk_size.x;
                                let local_y = (index as u32) / map.chunk_size.x;
                                let coord = chunk_coord
                                    .tile_origin(map.chunk_size)
                                    .offset(local_x as i32, local_y as i32);
                                Some(TileCollisionCell {
                                    coord,
                                    map_local_center: map.geometry.tile_to_local(coord)
                                        + layer.config.offset,
                                    descriptor,
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if cells.is_empty() {
                    if let Some(entity) = runtime_component.0.collision_chunks.remove(&entity_key) {
                        commands.entity(entity).despawn();
                    }
                    continue;
                }

                let revision = layer
                    .chunks
                    .get(chunk_coord)
                    .map_or(0, |chunk| chunk.revision);
                let collision_chunk = TilemapCollisionChunk {
                    map: map_entity,
                    layer: layer_id,
                    chunk: *chunk_coord,
                    revision,
                    cells,
                };

                let Some(&parent) = runtime_component.0.layer_nodes.get(&layer_id) else {
                    continue;
                };
                let entity = runtime_component
                    .0
                    .collision_chunks
                    .entry(entity_key)
                    .or_insert_with(|| {
                        commands
                            .spawn((
                                Name::new(format!(
                                    "Tilemap Collision Chunk {} ({}, {})",
                                    layer_id.0, chunk_coord.x, chunk_coord.y
                                )),
                                collision_chunk.clone(),
                            ))
                            .set_parent_in_place(parent)
                            .id()
                    });

                commands.entity(*entity).insert(collision_chunk);
                rebuilt.write(ChunkRebuilt {
                    map: map_entity,
                    layer: layer_id,
                    chunk: *chunk_coord,
                    render_updated: false,
                    collision_updated: true,
                });
            }

            if let Some(layer) = map.layers.get_mut(&layer_id) {
                layer.dirty_collision.clear();
            }
        }

        diagnostics.collision_chunks_total = runtime_component.0.collision_chunks.len();
    }
}

pub(crate) fn sync_render_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut rebuilt: MessageWriter<ChunkRebuilt>,
    mut maps: Query<
        (
            Entity,
            &mut Tilemap,
            &mut TilemapDiagnostics,
            &mut TilemapRuntimeComponent,
        ),
        With<crate::TilemapRoot>,
    >,
) {
    for (map_entity, mut map, mut diagnostics, mut runtime_component) in &mut maps {
        let layer_ids: Vec<_> = map.layers.keys().copied().collect();
        for layer_id in layer_ids {
            let dirty: Vec<_> = map.layers[&layer_id].dirty_render.iter().copied().collect();
            let visible = map.layers[&layer_id].config.visible;
            let render_config = map.layers[&layer_id].config.render.clone();
            let Some(render_config) = render_config else {
                for chunk_coord in &dirty {
                    if let Some(entity) = runtime_component
                        .0
                        .render_chunks
                        .remove(&(layer_id, *chunk_coord))
                    {
                        commands.entity(entity).despawn();
                    }
                }
                if let Some(layer) = map.layers.get_mut(&layer_id) {
                    layer.dirty_render.clear();
                }
                continue;
            };

            for chunk_coord in &dirty {
                let entity_key = (layer_id, *chunk_coord);
                let Some(mesh) = build_chunk_mesh(&map, &map.layers[&layer_id], *chunk_coord)
                else {
                    if let Some(entity) = runtime_component.0.render_chunks.remove(&entity_key) {
                        commands.entity(entity).despawn();
                    }
                    continue;
                };

                let mesh_handle = meshes.add(mesh);
                let material_handle = materials.add(build_color_material(&render_config));
                let translation =
                    chunk_local_translation(&map, &map.layers[&layer_id], *chunk_coord);
                let revision = map.layers[&layer_id]
                    .chunks
                    .get(chunk_coord)
                    .map_or(0, |chunk| chunk.revision);

                let render_chunk = TilemapRenderChunk {
                    map: map_entity,
                    layer: layer_id,
                    chunk: *chunk_coord,
                    revision,
                };

                let Some(&parent) = runtime_component.0.layer_nodes.get(&layer_id) else {
                    continue;
                };
                let entity = runtime_component
                    .0
                    .render_chunks
                    .entry(entity_key)
                    .or_insert_with(|| {
                        commands
                            .spawn((
                                Name::new(format!(
                                    "Tilemap Render Chunk {} ({}, {})",
                                    layer_id.0, chunk_coord.x, chunk_coord.y
                                )),
                                render_chunk.clone(),
                                Mesh2d(mesh_handle.clone()),
                                MeshMaterial2d(material_handle.clone()),
                                Transform::from_translation(translation),
                                GlobalTransform::default(),
                                if visible {
                                    Visibility::Visible
                                } else {
                                    Visibility::Hidden
                                },
                                InheritedVisibility::VISIBLE,
                                ViewVisibility::default(),
                            ))
                            .set_parent_in_place(parent)
                            .id()
                    });

                commands.entity(*entity).insert((
                    render_chunk,
                    Mesh2d(mesh_handle),
                    MeshMaterial2d(material_handle),
                    Transform::from_translation(translation),
                    if visible {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                ));

                diagnostics.chunks_rebuilt_this_frame += 1;
                rebuilt.write(ChunkRebuilt {
                    map: map_entity,
                    layer: layer_id,
                    chunk: *chunk_coord,
                    render_updated: true,
                    collision_updated: false,
                });
            }

            if let Some(layer) = map.layers.get_mut(&layer_id) {
                layer.dirty_render.clear();
            }
        }
    }
}

pub(crate) fn sync_layer_visibility(
    maps: Query<(&Tilemap, &TilemapRuntimeComponent), With<crate::TilemapRoot>>,
    mut layer_nodes: Query<(&TilemapLayerNode, &mut Visibility)>,
) {
    for (node, mut visibility) in &mut layer_nodes {
        let Ok((map, _)) = maps.get(node.map) else {
            continue;
        };
        if let Some(layer) = map.layers.get(&node.layer) {
            *visibility = if layer.config.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

pub(crate) fn draw_debug(
    settings: Res<crate::TilemapDebugSettings>,
    maps: Query<(
        &Tilemap,
        &GlobalTransform,
        Option<&crate::TilemapDebugOverlay>,
    )>,
    mut gizmos: Gizmos,
) {
    if !settings.enabled {
        return;
    }

    for (map, transform, overlay) in &maps {
        let overlay = overlay.copied().unwrap_or_default();
        for layer in map.layers.values() {
            if overlay.draw_chunk_bounds && settings.draw_chunk_bounds {
                for chunk in layer.chunks.keys() {
                    let rect = map.geometry.chunk_bounds_local(map.chunk_size, *chunk);
                    let min = transform
                        .affine()
                        .transform_point3(rect.min.extend(0.0))
                        .truncate();
                    let max = transform
                        .affine()
                        .transform_point3(rect.max.extend(0.0))
                        .truncate();
                    gizmos.rect_2d((min + max) * 0.5, max - min, settings.chunk_color);
                }
            }

            if overlay.draw_dirty_chunks && settings.draw_dirty_chunks {
                for chunk in layer.dirty_resolve.iter().chain(layer.dirty_render.iter()) {
                    let rect = map.geometry.chunk_bounds_local(map.chunk_size, *chunk);
                    let min = transform
                        .affine()
                        .transform_point3(rect.min.extend(0.0))
                        .truncate();
                    let max = transform
                        .affine()
                        .transform_point3(rect.max.extend(0.0))
                        .truncate();
                    gizmos.rect_2d((min + max) * 0.5, max - min, settings.dirty_color);
                }
            }
        }
    }
}

#[derive(Debug)]
struct ResolvedChunkSnapshot {
    visuals: Vec<Option<ResolvedTileVisual>>,
    collisions: Vec<Option<crate::TileCollisionDescriptor>>,
    animated_kinds: std::collections::BTreeSet<crate::TileKindId>,
}

fn resolve_chunk_snapshot(
    map: &Tilemap,
    runtime: &crate::rendering::TilemapRuntimeState,
    layer_id: TileLayerId,
    chunk_coord: crate::ChunkCoord,
) -> Option<ResolvedChunkSnapshot> {
    let layer = map.layers.get(&layer_id)?;
    let chunk = layer.chunks.get(&chunk_coord)?;
    let tile_origin = chunk_coord.tile_origin(map.chunk_size);

    let mut visuals = vec![None; chunk.tiles.len()];
    let mut collisions = vec![None; chunk.tiles.len()];
    let mut animated_kinds = std::collections::BTreeSet::new();

    for local_y in 0..map.chunk_size.y {
        for local_x in 0..map.chunk_size.x {
            let index = TileChunk::index(map.chunk_size, UVec2::new(local_x, local_y));
            let Some(cell) = chunk.tiles[index].as_ref() else {
                continue;
            };
            let Some(kind) = layer.catalog.kind(cell.kind) else {
                continue;
            };

            let atlas_index = match &kind.render {
                crate::TileRenderRule::Static(visual) => visual.atlas_index,
                crate::TileRenderRule::Animated(animation) => {
                    animated_kinds.insert(cell.kind);
                    let runtime_state = runtime
                        .animation_states
                        .get(&(layer_id, cell.kind))
                        .copied()
                        .unwrap_or_default();
                    animation.atlas_index_at(runtime_state.elapsed_seconds)
                }
                crate::TileRenderRule::Autotile(binding) => {
                    let mask = crate::compute_autotile_mask(
                        map,
                        layer_id,
                        tile_origin.offset(local_x as i32, local_y as i32),
                        binding.group,
                    );
                    layer
                        .catalog
                        .autotile_rule(binding.rule_set)
                        .map_or(binding.fallback_atlas_index, |rule| rule.resolve(mask))
                }
            };

            let Some(render) = layer.config.render.as_ref() else {
                collisions[index] = kind.collision.clone();
                continue;
            };

            visuals[index] = resolve_static_visual(
                &render.atlas,
                atlas_index,
                multiply_colors(render.tint, cell.tint),
                cell.orientation,
            );
            collisions[index] = kind.collision.clone();
        }
    }

    Some(ResolvedChunkSnapshot {
        visuals,
        collisions,
        animated_kinds,
    })
}

fn apply_tile_batch<I>(
    tilemap: &mut Tilemap,
    diagnostics: &mut TilemapDiagnostics,
    changed: &mut MessageWriter<TileChanged>,
    map: Entity,
    layer: TileLayerId,
    coords: I,
    tile: &TileCell,
) where
    I: IntoIterator<Item = crate::TileCoord>,
{
    for coord in coords {
        let previous_tile = tilemap.get_tile(layer, coord).cloned();
        tilemap.set_tile(layer, coord, tile.clone());
        let next_tile = tilemap.get_tile(layer, coord).cloned();
        record_tile_change(
            changed,
            diagnostics,
            map,
            layer,
            coord,
            previous_tile,
            next_tile,
        );
    }
}

fn record_tile_change(
    changed: &mut MessageWriter<TileChanged>,
    diagnostics: &mut TilemapDiagnostics,
    map: Entity,
    layer: TileLayerId,
    coord: crate::TileCoord,
    previous_tile: Option<TileCell>,
    next_tile: Option<TileCell>,
) {
    if previous_tile == next_tile {
        return;
    }

    diagnostics.tile_edits_this_frame += 1;
    changed.write(TileChanged {
        map,
        layer,
        coord,
        previous_kind: previous_tile.map(|tile| tile.kind),
        next_kind: next_tile.map(|tile| tile.kind),
    });
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
