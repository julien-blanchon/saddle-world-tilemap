use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    TileAtlasLayout, TileCatalog, TileCell, TileCoord, TileKindId, TileLayerConfig, TileLayerId,
    TileLayerRenderConfig, TileLayerState, TileOrientation, TileRowDirection, Tilemap,
    TilemapGeometry, TilemapHexParity,
};

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub enum TilePropertyValue {
    Bool(bool),
    Integer(i64),
    Float(f32),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct TileObjectSpawn {
    pub id: u32,
    pub name: String,
    pub class_name: Option<String>,
    pub layer_name: String,
    pub coord: Option<TileCoord>,
    pub world_position: Vec2,
    pub kind: Option<TileKindId>,
    pub rotation_degrees: f32,
    pub properties: BTreeMap<String, TilePropertyValue>,
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Debug, Clone)]
pub struct ImportedTilemapScene {
    pub map: Tilemap,
    pub object_spawns: Vec<TileObjectSpawn>,
}

#[derive(Debug, Clone)]
pub struct TiledImportOptions {
    pub atlas: TileAtlasLayout,
    pub catalog: TileCatalog,
    pub gid_to_kind: BTreeMap<u32, TileKindId>,
    pub chunk_size: UVec2,
    pub origin: Vec2,
    pub row_direction: TileRowDirection,
    pub layer_z_step: f32,
    pub logic_only_layers: BTreeSet<String>,
}

impl TiledImportOptions {
    #[must_use]
    pub fn new(
        atlas: TileAtlasLayout,
        catalog: TileCatalog,
        gid_to_kind: BTreeMap<u32, TileKindId>,
    ) -> Self {
        Self {
            atlas,
            catalog,
            gid_to_kind,
            chunk_size: UVec2::splat(16),
            origin: Vec2::ZERO,
            row_direction: TileRowDirection::Down,
            layer_z_step: 1.0,
            logic_only_layers: BTreeSet::new(),
        }
    }
}

#[derive(Debug)]
pub enum TiledImportError {
    Parse(serde_json::Error),
    UnsupportedOrientation(String),
    UnsupportedHexStagger {
        axis: Option<String>,
        index: Option<String>,
    },
    MissingLayerData(String),
    InvalidLayerDimensions(String),
    UnknownTileGid(u32),
}

impl std::fmt::Display for TiledImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "failed to parse Tiled JSON: {error}"),
            Self::UnsupportedOrientation(orientation) => {
                write!(f, "unsupported Tiled orientation '{orientation}'")
            }
            Self::UnsupportedHexStagger { axis, index } => write!(
                f,
                "unsupported hex stagger combination axis={:?}, index={:?}",
                axis, index
            ),
            Self::MissingLayerData(layer_name) => {
                write!(f, "tile layer '{layer_name}' has neither data nor chunks")
            }
            Self::InvalidLayerDimensions(layer_name) => {
                write!(
                    f,
                    "tile layer '{layer_name}' is missing width or height information"
                )
            }
            Self::UnknownTileGid(gid) => {
                write!(f, "no TileKindId mapping was provided for gid {gid}")
            }
        }
    }
}

impl std::error::Error for TiledImportError {}

pub fn import_tiled_json_str(
    json: &str,
    options: &TiledImportOptions,
) -> Result<ImportedTilemapScene, TiledImportError> {
    let map_data: TiledMap = serde_json::from_str(json).map_err(TiledImportError::Parse)?;
    import_tiled_map(&map_data, options)
}

fn import_tiled_map(
    map_data: &TiledMap,
    options: &TiledImportOptions,
) -> Result<ImportedTilemapScene, TiledImportError> {
    let geometry = tiled_geometry(map_data, options)?;
    let mut map = Tilemap::new(geometry, options.chunk_size);
    let mut object_spawns = Vec::new();
    let mut next_layer_id = 1_u16;

    for (layer_index, layer) in map_data.layers.iter().enumerate() {
        match layer.kind.as_str() {
            "tilelayer" => {
                let layer_id = TileLayerId::new(next_layer_id);
                next_layer_id += 1;
                let offset = Vec2::new(
                    layer.offset_x.unwrap_or(0.0),
                    layer.offset_y.unwrap_or(0.0) * options.row_direction.sign(),
                );
                let mut config = if options.logic_only_layers.contains(&layer.name) {
                    TileLayerConfig::logic_only(layer_id, layer.name.clone())
                } else {
                    TileLayerConfig::visual(
                        layer_id,
                        layer.name.clone(),
                        TileLayerRenderConfig::new(options.atlas.clone())
                            .with_z_index(layer_index as f32 * options.layer_z_step)
                            .with_tint(Color::srgba(1.0, 1.0, 1.0, layer.opacity.unwrap_or(1.0))),
                    )
                };
                config.visible = layer.visible.unwrap_or(true);
                config.offset = offset;

                let mut state = TileLayerState::new(config, options.catalog.clone());
                import_tile_layer(layer, options, &mut state)?;
                map.insert_layer(state);
            }
            "objectgroup" => {
                import_object_layer(layer, map_data, &geometry, options, &mut object_spawns)?;
            }
            _ => {}
        }
    }

    Ok(ImportedTilemapScene { map, object_spawns })
}

fn import_tile_layer(
    layer: &TiledLayer,
    options: &TiledImportOptions,
    state: &mut TileLayerState,
) -> Result<(), TiledImportError> {
    let layer_id = state.config.id;
    let mut map =
        Tilemap::new(TilemapGeometry::default(), options.chunk_size).with_layer(state.clone());

    if let Some(data) = &layer.data {
        let width = layer.width.unwrap_or(0);
        if width == 0 || layer.height.unwrap_or(0) == 0 {
            return Err(TiledImportError::InvalidLayerDimensions(layer.name.clone()));
        }
        for (index, gid) in data.iter().copied().enumerate() {
            let x = index as i32 % width as i32 + layer.x.unwrap_or(0);
            let y = index as i32 / width as i32 + layer.y.unwrap_or(0);
            insert_gid(options, &mut map, layer_id, TileCoord::new(x, y), gid)?;
        }
        *state = map.layers.remove(&layer_id).unwrap();
        return Ok(());
    }

    if let Some(chunks) = &layer.chunks {
        for chunk in chunks {
            for (index, gid) in chunk.data.iter().copied().enumerate() {
                let x = index as i32 % chunk.width as i32 + chunk.x;
                let y = index as i32 / chunk.width as i32 + chunk.y;
                insert_gid(options, &mut map, layer_id, TileCoord::new(x, y), gid)?;
            }
        }
        *state = map.layers.remove(&layer_id).unwrap();
        return Ok(());
    }

    Err(TiledImportError::MissingLayerData(layer.name.clone()))
}

fn insert_gid(
    options: &TiledImportOptions,
    map: &mut Tilemap,
    layer_id: TileLayerId,
    coord: TileCoord,
    encoded_gid: u32,
) -> Result<(), TiledImportError> {
    let Some((gid, orientation)) = decode_gid(encoded_gid) else {
        return Ok(());
    };
    let kind = options
        .gid_to_kind
        .get(&gid)
        .copied()
        .ok_or(TiledImportError::UnknownTileGid(gid))?;
    map.set_tile(
        layer_id,
        coord,
        TileCell::new(kind).with_orientation(orientation),
    );
    Ok(())
}

fn import_object_layer(
    layer: &TiledLayer,
    map_data: &TiledMap,
    geometry: &TilemapGeometry,
    options: &TiledImportOptions,
    object_spawns: &mut Vec<TileObjectSpawn>,
) -> Result<(), TiledImportError> {
    let layer_offset = Vec2::new(
        layer.offset_x.unwrap_or(0.0),
        layer.offset_y.unwrap_or(0.0) * options.row_direction.sign(),
    );
    for object in layer.objects.iter().flatten() {
        let raw_position = Vec2::new(
            object.x + layer_offset.x,
            object.y * options.row_direction.sign() + layer_offset.y,
        );
        let tile_anchor = object.gid.map(|_| {
            Vec2::new(
                raw_position.x,
                raw_position.y - map_data.tile_height as f32 * options.row_direction.sign(),
            )
        });
        let coord = tile_anchor
            .or(Some(raw_position))
            .map(|local| geometry.local_to_tile(local + options.origin));
        let kind = object
            .gid
            .and_then(|encoded_gid| decode_gid(encoded_gid).map(|(gid, _)| gid))
            .map(|gid| {
                options
                    .gid_to_kind
                    .get(&gid)
                    .copied()
                    .ok_or(TiledImportError::UnknownTileGid(gid))
            })
            .transpose()?;

        object_spawns.push(TileObjectSpawn {
            id: object.id,
            name: object.name.clone().unwrap_or_default(),
            class_name: object
                .class_name
                .clone()
                .or_else(|| object.object_type.clone()),
            layer_name: layer.name.clone(),
            coord,
            world_position: raw_position + options.origin,
            kind,
            rotation_degrees: object.rotation.unwrap_or(0.0),
            properties: object
                .properties
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|property| (property.name, property.value.into()))
                .collect(),
        });
    }
    Ok(())
}

fn tiled_geometry(
    map_data: &TiledMap,
    options: &TiledImportOptions,
) -> Result<TilemapGeometry, TiledImportError> {
    let tile_size = Vec2::new(map_data.tile_width as f32, map_data.tile_height as f32);
    let geometry = match map_data.orientation.as_str() {
        "orthogonal" => TilemapGeometry::square(tile_size),
        "isometric" => TilemapGeometry::isometric_diamond(tile_size),
        "hexagonal" => match (
            map_data.stagger_axis.as_deref(),
            map_data.stagger_index.as_deref(),
        ) {
            (Some("x"), Some("odd")) => {
                TilemapGeometry::hex_pointy_columns(tile_size, TilemapHexParity::Odd)
            }
            (Some("x"), Some("even")) => {
                TilemapGeometry::hex_pointy_columns(tile_size, TilemapHexParity::Even)
            }
            (Some("y"), Some("odd")) => {
                TilemapGeometry::hex_flat_rows(tile_size, TilemapHexParity::Odd)
            }
            (Some("y"), Some("even")) => {
                TilemapGeometry::hex_flat_rows(tile_size, TilemapHexParity::Even)
            }
            (axis, index) => {
                return Err(TiledImportError::UnsupportedHexStagger {
                    axis: axis.map(str::to_string),
                    index: index.map(str::to_string),
                });
            }
        },
        orientation => {
            return Err(TiledImportError::UnsupportedOrientation(
                orientation.to_string(),
            ));
        }
    };

    Ok(geometry
        .with_origin(options.origin)
        .with_row_direction(options.row_direction))
}

fn decode_gid(encoded_gid: u32) -> Option<(u32, TileOrientation)> {
    const HORIZONTAL_FLIP: u32 = 0x8000_0000;
    const VERTICAL_FLIP: u32 = 0x4000_0000;
    const DIAGONAL_FLIP: u32 = 0x2000_0000;
    const FLAG_MASK: u32 = HORIZONTAL_FLIP | VERTICAL_FLIP | DIAGONAL_FLIP;

    let gid = encoded_gid & !FLAG_MASK;
    if gid == 0 {
        return None;
    }

    let flags = ((encoded_gid & HORIZONTAL_FLIP != 0) as u8) << 2
        | ((encoded_gid & VERTICAL_FLIP != 0) as u8) << 1
        | ((encoded_gid & DIAGONAL_FLIP != 0) as u8);

    let orientation = match flags {
        0b000 => TileOrientation::Default,
        0b001 => TileOrientation::MirrorHRotate90,
        0b010 => TileOrientation::MirrorHRotate180,
        0b011 => TileOrientation::Rotate90,
        0b100 => TileOrientation::MirrorH,
        0b101 => TileOrientation::Rotate270,
        0b110 => TileOrientation::Rotate180,
        _ => TileOrientation::MirrorHRotate270,
    };

    Some((gid, orientation))
}

#[derive(Clone, Deserialize)]
struct TiledMap {
    orientation: String,
    #[serde(rename = "tilewidth")]
    tile_width: u32,
    #[serde(rename = "tileheight")]
    tile_height: u32,
    #[serde(rename = "staggeraxis")]
    stagger_axis: Option<String>,
    #[serde(rename = "staggerindex")]
    stagger_index: Option<String>,
    layers: Vec<TiledLayer>,
}

#[derive(Clone, Deserialize)]
struct TiledLayer {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    visible: Option<bool>,
    opacity: Option<f32>,
    #[serde(rename = "offsetx")]
    offset_x: Option<f32>,
    #[serde(rename = "offsety")]
    offset_y: Option<f32>,
    width: Option<u32>,
    height: Option<u32>,
    x: Option<i32>,
    y: Option<i32>,
    data: Option<Vec<u32>>,
    chunks: Option<Vec<TiledChunk>>,
    objects: Option<Vec<TiledObject>>,
}

#[derive(Clone, Deserialize)]
struct TiledChunk {
    x: i32,
    y: i32,
    width: u32,
    data: Vec<u32>,
}

#[derive(Clone, Deserialize)]
struct TiledObject {
    id: u32,
    name: Option<String>,
    #[serde(rename = "class")]
    class_name: Option<String>,
    #[serde(rename = "type")]
    object_type: Option<String>,
    x: f32,
    y: f32,
    rotation: Option<f32>,
    gid: Option<u32>,
    properties: Option<Vec<TiledProperty>>,
}

#[derive(Clone, Deserialize)]
struct TiledProperty {
    name: String,
    value: serde_json::Value,
}

impl From<serde_json::Value> for TilePropertyValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => value
                .as_i64()
                .map(Self::Integer)
                .or_else(|| value.as_f64().map(|value| Self::Float(value as f32)))
                .unwrap_or_else(|| Self::Text(value.to_string())),
            serde_json::Value::String(value) => Self::Text(value),
            other => Self::Text(other.to_string()),
        }
    }
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
