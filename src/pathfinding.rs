use crate::{
    TileCell, TileCoord, TileKind, TileLayerId, TileLayerState, Tilemap, TilemapOrientation,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};

#[derive(Debug, Clone)]
pub struct TilePathResult {
    pub path: Vec<TileCoord>,
    pub total_cost: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct TilePathOptions {
    pub max_cost: u32,
    pub diagonal: bool,
}

impl Default for TilePathOptions {
    fn default() -> Self {
        Self {
            max_cost: u32::MAX,
            diagonal: false,
        }
    }
}

impl TilePathOptions {
    #[must_use]
    pub fn with_max_cost(mut self, max_cost: u32) -> Self {
        self.max_cost = max_cost;
        self
    }

    #[must_use]
    pub fn with_diagonal(mut self, diagonal: bool) -> Self {
        self.diagonal = diagonal;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TilePathStep<'a> {
    pub map: &'a Tilemap,
    pub layer_id: TileLayerId,
    pub layer: &'a TileLayerState,
    pub from: TileCoord,
    pub to: TileCoord,
    pub from_tile: Option<&'a TileCell>,
    pub from_kind: Option<&'a TileKind>,
    pub to_tile: Option<&'a TileCell>,
    pub to_kind: Option<&'a TileKind>,
}

pub trait TilePathPolicy {
    fn is_passable(&self, step: &TilePathStep<'_>) -> bool;

    fn movement_cost(&self, step: &TilePathStep<'_>) -> u32;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TileKindPathPolicy;

impl TilePathPolicy for TileKindPathPolicy {
    fn is_passable(&self, step: &TilePathStep<'_>) -> bool {
        matches!(step.to_kind, Some(kind) if kind.collision.is_none())
    }

    fn movement_cost(&self, step: &TilePathStep<'_>) -> u32 {
        step.to_kind.map_or(1, |kind| kind.movement_cost as u32)
    }
}

pub struct TilePathCallbacks<Passability, Cost> {
    passability: Passability,
    movement_cost: Cost,
}

impl<Passability, Cost> TilePathCallbacks<Passability, Cost> {
    #[must_use]
    pub const fn new(passability: Passability, movement_cost: Cost) -> Self {
        Self {
            passability,
            movement_cost,
        }
    }
}

impl<Passability, Cost> TilePathPolicy for TilePathCallbacks<Passability, Cost>
where
    Passability: for<'a> Fn(&TilePathStep<'a>) -> bool,
    Cost: for<'a> Fn(&TilePathStep<'a>) -> u32,
{
    fn is_passable(&self, step: &TilePathStep<'_>) -> bool {
        (self.passability)(step)
    }

    fn movement_cost(&self, step: &TilePathStep<'_>) -> u32 {
        (self.movement_cost)(step)
    }
}

pub fn find_path(
    map: &Tilemap,
    layer_id: TileLayerId,
    start: TileCoord,
    goal: TileCoord,
    options: &TilePathOptions,
) -> Option<TilePathResult> {
    find_path_with_policy(map, layer_id, start, goal, options, &TileKindPathPolicy)
}

pub fn find_path_with_policy<P>(
    map: &Tilemap,
    layer_id: TileLayerId,
    start: TileCoord,
    goal: TileCoord,
    options: &TilePathOptions,
    policy: &P,
) -> Option<TilePathResult>
where
    P: TilePathPolicy + ?Sized,
{
    if start == goal {
        return Some(TilePathResult {
            path: vec![start],
            total_cost: 0,
        });
    }

    let layer = map.layers.get(&layer_id)?;

    let mut open = BinaryHeap::new();
    let mut g_score: BTreeMap<TileCoord, u32> = BTreeMap::new();
    let mut came_from: BTreeMap<TileCoord, TileCoord> = BTreeMap::new();

    g_score.insert(start, 0);
    open.push(AStarNode {
        coord: start,
        f_score: heuristic(start, goal, options.diagonal),
    });

    while let Some(current) = open.pop() {
        if current.coord == goal {
            return Some(reconstruct_path(&came_from, current.coord, &g_score));
        }

        let current_g = g_score.get(&current.coord).copied().unwrap_or(u32::MAX);

        let neighbors = tile_neighbors(map, current.coord, options.diagonal);
        for neighbor in neighbors {
            let Some(move_cost) = step_cost(map, layer_id, layer, current.coord, neighbor, policy)
            else {
                continue;
            };

            let tentative_g = current_g.saturating_add(move_cost);
            if tentative_g > options.max_cost {
                continue;
            }

            let existing_g = g_score.get(&neighbor).copied().unwrap_or(u32::MAX);
            if tentative_g < existing_g {
                came_from.insert(neighbor, current.coord);
                g_score.insert(neighbor, tentative_g);
                open.push(AStarNode {
                    coord: neighbor,
                    f_score: tentative_g + heuristic(neighbor, goal, options.diagonal),
                });
            }
        }
    }

    None
}

pub fn reachable_tiles(
    map: &Tilemap,
    layer_id: TileLayerId,
    start: TileCoord,
    max_cost: u32,
    diagonal: bool,
) -> BTreeMap<TileCoord, u32> {
    reachable_tiles_with_policy(
        map,
        layer_id,
        start,
        max_cost,
        diagonal,
        &TileKindPathPolicy,
    )
}

pub fn reachable_tiles_with_policy<P>(
    map: &Tilemap,
    layer_id: TileLayerId,
    start: TileCoord,
    max_cost: u32,
    diagonal: bool,
    policy: &P,
) -> BTreeMap<TileCoord, u32>
where
    P: TilePathPolicy + ?Sized,
{
    let Some(layer) = map.layers.get(&layer_id) else {
        return BTreeMap::new();
    };

    let mut costs: BTreeMap<TileCoord, u32> = BTreeMap::new();
    let mut open = BinaryHeap::new();

    costs.insert(start, 0);
    open.push(DijkstraNode {
        coord: start,
        cost: 0,
    });

    while let Some(current) = open.pop() {
        if current.cost > costs.get(&current.coord).copied().unwrap_or(u32::MAX) {
            continue;
        }

        let neighbors = tile_neighbors(map, current.coord, diagonal);
        for neighbor in neighbors {
            let Some(move_cost) = step_cost(map, layer_id, layer, current.coord, neighbor, policy)
            else {
                continue;
            };

            let tentative = current.cost.saturating_add(move_cost);
            if tentative > max_cost {
                continue;
            }

            let existing = costs.get(&neighbor).copied().unwrap_or(u32::MAX);
            if tentative < existing {
                costs.insert(neighbor, tentative);
                open.push(DijkstraNode {
                    coord: neighbor,
                    cost: tentative,
                });
            }
        }
    }

    costs
}

fn step_cost<P>(
    map: &Tilemap,
    layer_id: TileLayerId,
    layer: &TileLayerState,
    from: TileCoord,
    to: TileCoord,
    policy: &P,
) -> Option<u32>
where
    P: TilePathPolicy + ?Sized,
{
    let from_tile = layer.get_tile(map.chunk_size, from);
    let from_kind = from_tile.and_then(|cell| layer.catalog.kind(cell.kind));
    let to_tile = layer.get_tile(map.chunk_size, to);
    let to_kind = to_tile.and_then(|cell| layer.catalog.kind(cell.kind));
    let step = TilePathStep {
        map,
        layer_id,
        layer,
        from,
        to,
        from_tile,
        from_kind,
        to_tile,
        to_kind,
    };

    policy
        .is_passable(&step)
        .then(|| policy.movement_cost(&step))
}

fn tile_neighbors(map: &Tilemap, coord: TileCoord, diagonal: bool) -> Vec<TileCoord> {
    match map.geometry.orientation {
        TilemapOrientation::Square => {
            if diagonal {
                coord.eight_neighbors().to_vec()
            } else {
                coord.cardinal_neighbors().to_vec()
            }
        }
        TilemapOrientation::IsometricDiamond => {
            if diagonal {
                coord.eight_neighbors().to_vec()
            } else {
                coord.cardinal_neighbors().to_vec()
            }
        }
        TilemapOrientation::HexPointyColumns(parity) => coord.hex_neighbors_pointy(parity).to_vec(),
        TilemapOrientation::HexFlatRows(parity) => coord.hex_neighbors_flat(parity).to_vec(),
    }
}

fn heuristic(a: TileCoord, b: TileCoord, diagonal: bool) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    if diagonal { dx.max(dy) } else { dx + dy }
}

fn reconstruct_path(
    came_from: &BTreeMap<TileCoord, TileCoord>,
    end: TileCoord,
    g_score: &BTreeMap<TileCoord, u32>,
) -> TilePathResult {
    let mut path = vec![end];
    let mut current = end;
    while let Some(&parent) = came_from.get(&current) {
        path.push(parent);
        current = parent;
    }
    path.reverse();
    TilePathResult {
        total_cost: g_score.get(&end).copied().unwrap_or(0),
        path,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct AStarNode {
    coord: TileCoord,
    f_score: u32,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DijkstraNode {
    coord: TileCoord,
    cost: u32,
}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
#[path = "pathfinding_tests.rs"]
mod tests;
