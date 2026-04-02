use crate::{TileCoord, TileLayerId, Tilemap};
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct AutotileGroupId(pub u16);

impl AutotileGroupId {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub struct AutotileRuleSetId(pub u16);

impl AutotileRuleSetId {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Clone, PartialEq, Hash)]
pub enum AutotileNeighborhood {
    Cardinal4,
    Moore8,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct AutotileBinding {
    pub group: AutotileGroupId,
    pub rule_set: AutotileRuleSetId,
    pub fallback_atlas_index: u32,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct AutotileRuleSet {
    pub neighborhood: AutotileNeighborhood,
    pub variants: BTreeMap<u16, u32>,
    pub fallback_atlas_index: u32,
}

impl AutotileRuleSet {
    #[must_use]
    pub fn new(neighborhood: AutotileNeighborhood, fallback_atlas_index: u32) -> Self {
        Self {
            neighborhood,
            variants: BTreeMap::new(),
            fallback_atlas_index,
        }
    }

    #[must_use]
    pub fn with_variant(mut self, mask: u16, atlas_index: u32) -> Self {
        self.variants.insert(mask, atlas_index);
        self
    }

    #[must_use]
    pub fn resolve(&self, mask: u16) -> u32 {
        self.variants
            .get(&mask)
            .copied()
            .unwrap_or(self.fallback_atlas_index)
    }
}

#[must_use]
pub fn compute_autotile_mask(
    map: &Tilemap,
    layer_id: TileLayerId,
    coord: TileCoord,
    group: AutotileGroupId,
) -> u16 {
    let Some(layer) = map.layers.get(&layer_id) else {
        return 0;
    };
    let Some(kind) = map
        .get_tile(layer_id, coord)
        .and_then(|tile| layer.catalog.kind(tile.kind))
    else {
        return 0;
    };
    let Some(binding) = kind.render.autotile_binding() else {
        return 0;
    };
    let Some(rule_set) = layer.catalog.autotile_rule(binding.rule_set) else {
        return 0;
    };

    let offsets: &[(i32, i32)] = match rule_set.neighborhood {
        AutotileNeighborhood::Cardinal4 => &[(0, -1), (1, 0), (0, 1), (-1, 0)],
        AutotileNeighborhood::Moore8 => &[
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ],
    };

    offsets
        .iter()
        .enumerate()
        .fold(0, |mask, (index, (dx, dy))| {
            let neighbor_coord = coord.offset(*dx, *dy);
            let matches_group = map
                .get_tile(layer_id, neighbor_coord)
                .and_then(|tile| layer.catalog.kind(tile.kind))
                .and_then(|neighbor_kind| neighbor_kind.render.autotile_binding())
                .is_some_and(|neighbor_binding| neighbor_binding.group == group);

            if matches_group {
                mask | (1 << index)
            } else {
                mask
            }
        })
}
#[cfg(test)]
#[path = "autotile_tests.rs"]
mod tests;
