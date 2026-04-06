mod support;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{CameraFocus, LabControl};
use saddle_world_tilemap::{
    TileCoord, TilePathCallbacks, TilePathOptions, TilePathStep, find_path,
    find_path_with_policy, reachable_tiles, reachable_tiles_with_policy,
};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "tilemap_smoke",
        "tilemap_runtime_edit",
        "tilemap_isometric_pick",
        "tilemap_large_map",
        "tilemap_pathfinding",
        "tilemap_custom_path_policy",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(tilemap_smoke("smoke_launch")),
        "tilemap_smoke" => Some(tilemap_smoke("tilemap_smoke")),
        "tilemap_runtime_edit" => Some(tilemap_runtime_edit()),
        "tilemap_isometric_pick" => Some(tilemap_isometric_pick()),
        "tilemap_large_map" => Some(tilemap_large_map()),
        "tilemap_pathfinding" => Some(tilemap_pathfinding()),
        "tilemap_custom_path_policy" => Some(tilemap_custom_path_policy()),
        _ => None,
    }
}

fn set_control(mutator: impl Fn(&mut LabControl) + Send + Sync + 'static) -> Action {
    Action::Custom(Box::new(move |world| {
        let mut control = world.resource_mut::<LabControl>();
        mutator(&mut control);
    }))
}

fn wait_for_diagnostics(
    label: impl Into<String>,
    condition: impl Fn(&crate::LabDiagnostics) -> bool + Send + Sync + 'static,
) -> Action {
    Action::WaitUntil {
        label: label.into(),
        condition: Box::new(move |world| condition(world.resource::<crate::LabDiagnostics>())),
        max_frames: 60,
    }
}

fn tilemap_smoke(name: &'static str) -> Scenario {
    Scenario::builder(name)
        .description("Boot the tilemap lab, wait for the three showcase maps to initialize, then capture the overview once animation and chunk diagnostics are live.")
        .then(wait_for_diagnostics("overview ready", |diagnostics| {
            diagnostics.large_total_chunks >= 64 && diagnostics.animation_loops >= 1
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Square Showcase Map"));
            assert!(support::named_entity_exists(world, "Isometric Showcase Map"));
            assert!(support::named_entity_exists(world, "Large Showcase Map"));
        })))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "large map initialized enough chunks",
            |diagnostics| diagnostics.large_total_chunks >= 64,
        ))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "animation loop observed",
            |diagnostics| diagnostics.animation_loops >= 1,
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Controls:")
                    && text.contains("Square stage:")
                    && text.contains("Iso selection:")
            }));
        })))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::render_chunk_count(world) > 0);
            assert!(support::collision_chunk_count(world) > 0);
        })))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary(name))
        .build()
}

fn tilemap_runtime_edit() -> Scenario {
    Scenario::builder("tilemap_runtime_edit")
        .description("Capture the overview before any staged edits, then apply a partial road branch and finally the full extension across chunk boundaries.")
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::Overview;
            control.square_edit_stage = 0;
        }))
        .then(wait_for_diagnostics("stage 0", |diagnostics| {
            diagnostics.square_applied_tiles == 0
        }))
        .then(Action::Screenshot("runtime_before".into()))
        .then(Action::WaitFrames(1))
        .then(set_control(|control| {
            control.square_edit_stage = 1;
        }))
        .then(wait_for_diagnostics("stage 1", |diagnostics| {
            diagnostics.square_applied_tiles == 4
                && diagnostics.square_latest_edit == TileCoord::new(8, 12)
                && diagnostics.square_rebuilds_last_frame > 0
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "stage 1 applied four tiles",
            |diagnostics| diagnostics.square_applied_tiles == 4,
        ))
        .then(Action::Screenshot("runtime_partial".into()))
        .then(Action::WaitFrames(1))
        .then(set_control(|control| {
            control.square_edit_stage = 2;
        }))
        .then(wait_for_diagnostics("stage 2", |diagnostics| {
            diagnostics.square_applied_tiles == 10
                && diagnostics.square_latest_edit == TileCoord::new(12, 10)
                && diagnostics.square_rebuilds_last_frame > 0
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "full branch applied",
            |diagnostics| diagnostics.square_applied_tiles == 10,
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Square stage: 2") && text.contains("applied tiles: 10")
            }));
        })))
        .then(Action::Screenshot("runtime_full".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_runtime_edit"))
        .build()
}

fn tilemap_isometric_pick() -> Scenario {
    Scenario::builder("tilemap_isometric_pick")
        .description("Move the camera to the isometric board, pick a costly stone tile, then move to a cheaper grass tile and confirm the diagnostics change with the screenshots.")
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::Isometric;
            control.iso_selection = TileCoord::new(6, 2);
        }))
        .then(wait_for_diagnostics("stone selection", |diagnostics| {
            diagnostics.iso_selection == TileCoord::new(6, 2) && diagnostics.iso_selection_cost == 3
        }))
        .then(Action::Screenshot("iso_stone".into()))
        .then(Action::WaitFrames(1))
        .then(set_control(|control| {
            control.iso_selection = TileCoord::new(4, 4);
        }))
        .then(wait_for_diagnostics("grass selection", |diagnostics| {
            diagnostics.iso_selection == TileCoord::new(4, 4) && diagnostics.iso_selection_cost == 1
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "grass tile is cheaper than the stone selection",
            |diagnostics| diagnostics.iso_selection_cost == 1,
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Iso selection: (4, 4)") && text.contains("movement cost: 1")
            }));
        })))
        .then(Action::Screenshot("iso_grass".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_isometric_pick"))
        .build()
}

fn tilemap_large_map() -> Scenario {
    Scenario::builder("tilemap_large_map")
        .description("Sweep the camera from the left side of the large dense map to the right side and assert that the center tile and chunk indices cross chunk boundaries as expected.")
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::LargeLeft;
        }))
        .then(wait_for_diagnostics("left focus", |diagnostics| {
            diagnostics.large_total_chunks >= 64 && diagnostics.large_center_chunk.x <= 2
        }))
        .then(Action::Screenshot("large_left".into()))
        .then(Action::WaitFrames(1))
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::LargeRight;
        }))
        .then(wait_for_diagnostics("right focus", |diagnostics| {
            diagnostics.large_center_chunk.x >= 5
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "camera crossed multiple chunk columns",
            |diagnostics| diagnostics.large_center_chunk.x >= 5,
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Camera focus: LargeRight") && text.contains("large chunks:")
            }));
        })))
        .then(Action::Screenshot("large_right".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_large_map"))
        .build()
}

fn tilemap_pathfinding() -> Scenario {
    use saddle_world_tilemap_example_support::GROUND_LAYER;

    Scenario::builder("tilemap_pathfinding")
        .description("Run A* pathfinding and reachable-tiles flood on the square showcase map to verify the pathfinding API produces valid results at runtime.")
        .then(wait_for_diagnostics("maps ready", |diagnostics| {
            diagnostics.large_total_chunks >= 64
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            // Find the square showcase map entity and its Tilemap component
            let mut query = world.query::<(&Name, &saddle_world_tilemap::Tilemap)>();
            let (_, map) = query
                .iter(world)
                .find(|(name, _)| name.as_str() == "Square Showcase Map")
                .expect("Square Showcase Map entity must exist");

            // A* pathfinding: find a path from (1,1) to (10,10)
            let start = TileCoord::new(1, 1);
            let goal = TileCoord::new(10, 10);
            let options = TilePathOptions::default().with_diagonal(false);
            let result = find_path(map, GROUND_LAYER, start, goal, &options);
            assert!(result.is_some(), "A* must find a path from (1,1) to (10,10)");
            let path = result.unwrap();
            assert!(path.path.len() >= 2, "Path must have at least start and end");
            assert_eq!(path.path.first().copied(), Some(start), "Path must start at start");
            assert_eq!(path.path.last().copied(), Some(goal), "Path must end at goal");
            assert!(path.total_cost > 0, "Path cost must be positive");

            // Verify path continuity: each step is a cardinal neighbor of the previous
            for window in path.path.windows(2) {
                let dx = (window[0].x - window[1].x).abs();
                let dy = (window[0].y - window[1].y).abs();
                assert!(
                    (dx == 1 && dy == 0) || (dx == 0 && dy == 1),
                    "Each path step must be a cardinal neighbor"
                );
            }

            // Reachable tiles: flood from (5,5) with max cost 5
            let center = TileCoord::new(5, 5);
            let reachable = reachable_tiles(map, GROUND_LAYER, center, 5, false);
            assert!(reachable.len() > 1, "Reachable tiles must include more than just the start");
            assert!(reachable.contains_key(&center), "Reachable must include the start tile");
            assert_eq!(reachable[&center], 0, "Start tile cost must be zero");

            // All reachable tiles must have cost <= 5
            for (_, cost) in &reachable {
                assert!(*cost <= 5, "No reachable tile should exceed max_cost");
            }

            info!("Pathfinding scenario passed: path len={}, cost={}, reachable={}", path.path.len(), path.total_cost, reachable.len());
        })))
        .then(Action::Screenshot("pathfinding".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_pathfinding"))
        .build()
}

fn tilemap_custom_path_policy() -> Scenario {
    use saddle_world_tilemap_example_support::{COLLISION_LAYER, GROUND_LAYER};

    Scenario::builder("tilemap_custom_path_policy")
        .description("Run pathfinding with an injected traversal policy that reads the collision layer, verifying the generic callbacks can block tiles independently from the queried ground layer.")
        .then(wait_for_diagnostics("maps ready", |diagnostics| {
            diagnostics.large_total_chunks >= 64
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut query = world.query::<(&Name, &saddle_world_tilemap::Tilemap)>();
            let (_, map) = query
                .iter(world)
                .find(|(name, _)| name.as_str() == "Square Showcase Map")
                .expect("Square Showcase Map entity must exist");

            let start = TileCoord::new(15, 7);
            let goal = TileCoord::new(19, 7);
            let options = TilePathOptions::default().with_diagonal(false);

            let default = find_path(map, GROUND_LAYER, start, goal, &options)
                .expect("default path should exist on the ground layer");
            assert!(
                default
                    .path
                    .iter()
                    .any(|coord| (16..=18).contains(&coord.x) && (6..=8).contains(&coord.y)),
                "default policy should ignore the separate collision layer"
            );

            let policy = TilePathCallbacks::new(
                |step: &TilePathStep<'_>| step.map.get_tile(COLLISION_LAYER, step.to).is_none(),
                |step: &TilePathStep<'_>| step.to_kind.map_or(1, |kind| kind.movement_cost as u32),
            );
            let custom = find_path_with_policy(map, GROUND_LAYER, start, goal, &options, &policy)
                .expect("custom policy should route around the rock enclosure");
            assert!(
                !custom
                    .path
                    .iter()
                    .any(|coord| (16..=18).contains(&coord.x) && (6..=8).contains(&coord.y)),
                "custom policy should avoid the collision-layer blockers"
            );

            let reachable =
                reachable_tiles_with_policy(map, GROUND_LAYER, TileCoord::new(15, 7), 8, false, &policy);
            assert!(
                !reachable.contains_key(&TileCoord::new(16, 7)),
                "reachable tiles should also respect the injected blocker policy"
            );

            info!(
                "Custom path policy scenario passed: default_len={}, custom_len={}, reachable={}",
                default.path.len(),
                custom.path.len(),
                reachable.len()
            );
        })))
        .then(Action::Screenshot("custom_path_policy".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_custom_path_policy"))
        .build()
}
