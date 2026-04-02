use crate::{
    TileAtlasLayout, TileLayerId, TileLayerState, Tilemap, TilemapOrientation,
    animation::TileAnimationRuntimeState,
    chunk::{ResolvedTileVisual, TileChunk},
    layer::TileOrientation,
};
use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct TilemapRuntimeState {
    pub initialized: bool,
    pub layer_nodes: BTreeMap<TileLayerId, Entity>,
    pub render_chunks: BTreeMap<(TileLayerId, crate::ChunkCoord), Entity>,
    pub collision_chunks: BTreeMap<(TileLayerId, crate::ChunkCoord), Entity>,
    pub animation_states: BTreeMap<(TileLayerId, crate::TileKindId), TileAnimationRuntimeState>,
}

#[derive(Component, Default)]
pub(crate) struct TilemapRuntimeComponent(pub TilemapRuntimeState);

pub(crate) fn build_chunk_mesh(
    map: &Tilemap,
    layer: &TileLayerState,
    chunk: crate::ChunkCoord,
) -> Option<Mesh> {
    let render = layer.config.render.as_ref()?;
    let chunk_ref = layer.chunks.get(&chunk)?;
    let tile_origin = chunk.tile_origin(map.chunk_size);

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut draw_order: Vec<(usize, crate::TileCoord, &ResolvedTileVisual)> = Vec::new();
    for local_y in 0..map.chunk_size.y {
        for local_x in 0..map.chunk_size.x {
            let index = TileChunk::index(map.chunk_size, UVec2::new(local_x, local_y));
            let Some(visual) = chunk_ref.resolved_visuals[index].as_ref() else {
                continue;
            };
            let global_coord = tile_origin.offset(local_x as i32, local_y as i32);
            draw_order.push((index, global_coord, visual));
        }
    }

    if draw_order.is_empty() {
        return None;
    }

    if matches!(
        map.geometry.orientation,
        TilemapOrientation::IsometricDiamond
    ) {
        draw_order.sort_by_key(|(_, coord, _)| (coord.x + coord.y, coord.x));
    }

    for (_, coord, visual) in draw_order {
        let center = map.geometry.tile_to_local(coord) - map.geometry.tile_to_local(tile_origin);
        let half = map.geometry.tile_render_size * 0.5;
        let base_index = positions.len() as u32;

        positions.extend([
            [center.x - half.x, center.y + half.y, 0.0],
            [center.x + half.x, center.y + half.y, 0.0],
            [center.x + half.x, center.y - half.y, 0.0],
            [center.x - half.x, center.y - half.y, 0.0],
        ]);

        let base_uvs = render.atlas.uv_rect(visual.atlas_index);
        let mapped_uvs = oriented_uvs(base_uvs, visual.orientation);
        uvs.extend(mapped_uvs.map(|uv| [uv.x, uv.y]));

        let color = visual.tint.to_srgba().to_f32_array();
        colors.extend([color; 4]);

        indices.extend_from_slice(&[
            base_index,
            base_index + 2,
            base_index + 1,
            base_index,
            base_index + 3,
            base_index + 2,
        ]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

pub(crate) fn chunk_local_translation(
    map: &Tilemap,
    layer: &TileLayerState,
    chunk: crate::ChunkCoord,
) -> Vec3 {
    let render = layer.config.render.as_ref();
    let origin = chunk.tile_origin(map.chunk_size);
    let local = map.geometry.tile_to_local(origin) + layer.config.offset;
    let z = render.map_or(0.0, |render| {
        let chunk_depth = if matches!(
            map.geometry.orientation,
            TilemapOrientation::IsometricDiamond
        ) {
            (chunk.x + chunk.y) as f32 * render.chunk_sort_step
        } else {
            0.0
        };
        render.z_index + chunk_depth
    });
    Vec3::new(local.x, local.y, z)
}

pub(crate) fn build_color_material(render: &crate::TileLayerRenderConfig) -> ColorMaterial {
    ColorMaterial {
        color: render.tint,
        alpha_mode: render.alpha_mode,
        texture: Some(render.atlas.image.clone()),
        ..default()
    }
}

fn oriented_uvs(corners: [Vec2; 4], orientation: TileOrientation) -> [Vec2; 4] {
    let source_positions = [
        IVec2::new(-1, 1),
        IVec2::new(1, 1),
        IVec2::new(1, -1),
        IVec2::new(-1, -1),
    ];

    let mut mapped = corners;
    for (dest_index, position) in source_positions.iter().enumerate() {
        let source = orientation.inverse().apply_to_ivec2(position);
        let source_index = source_positions
            .iter()
            .position(|candidate| *candidate == source)
            .unwrap_or(dest_index);
        mapped[dest_index] = corners[source_index];
    }
    mapped
}

pub(crate) fn multiply_colors(a: Color, b: Color) -> Color {
    let a = a.to_srgba();
    let b = b.to_srgba();
    Color::srgba(
        a.red * b.red,
        a.green * b.green,
        a.blue * b.blue,
        a.alpha * b.alpha,
    )
}

pub(crate) fn resolve_static_visual(
    atlas: &TileAtlasLayout,
    atlas_index: u32,
    tint: Color,
    orientation: TileOrientation,
) -> Option<ResolvedTileVisual> {
    (atlas_index < atlas.tile_count()).then_some(ResolvedTileVisual {
        atlas_index,
        tint,
        orientation,
    })
}
