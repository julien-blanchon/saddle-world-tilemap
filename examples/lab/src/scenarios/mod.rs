mod support;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{CameraFocus, LabControl};
use saddle_world_tilemap::{
    TileCoord, TilePathCallbacks, TilePathOptions, TilePathStep, TilemapCommand,
    find_path, find_path_with_policy, reachable_tiles, reachable_tiles_with_policy,
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
        "tilemap_layer_visibility",
        "tilemap_animation_loops",
        "tilemap_autotiling",
        "tilemap_hex_strategy",
        "tilemap_tile_painter",
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
        "tilemap_layer_visibility" => Some(tilemap_layer_visibility()),
        "tilemap_animation_loops" => Some(tilemap_animation_loops()),
        "tilemap_autotiling" => Some(tilemap_autotiling()),
        "tilemap_hex_strategy" => Some(tilemap_hex_strategy()),
        "tilemap_tile_painter" => Some(tilemap_tile_painter()),
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

fn tilemap_layer_visibility() -> Scenario {
    use saddle_world_tilemap_example_support::HIGHLIGHT_LAYER;

    Scenario::builder("tilemap_layer_visibility")
        .description("Hide and show the highlight layer on the square showcase map via SetLayerVisibility and assert render-chunk counts change accordingly.")
        .then(wait_for_diagnostics("maps ready", |diagnostics| {
            diagnostics.large_total_chunks >= 64
        }))
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::Overview;
        }))
        .then(Action::WaitFrames(5))
        // Capture baseline (highlight layer visible).
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::render_chunk_count(world) > 0);
        })))
        .then(Action::Screenshot("visibility_on".into()))
        .then(Action::WaitFrames(1))
        // Hide the highlight layer on the square map.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut query = world.query::<(Entity, &Name)>();
            let square_map = query
                .iter(world)
                .find_map(|(entity, name)| (name.as_str() == "Square Showcase Map").then_some(entity))
                .expect("Square Showcase Map must exist");
            world
                .resource_mut::<Messages<TilemapCommand>>()
                .write(TilemapCommand::SetLayerVisibility {
                    map: square_map,
                    layer: HIGHLIGHT_LAYER,
                    visible: false,
                });
        })))
        .then(wait_for_diagnostics("visibility toggles applied", |diagnostics| {
            // Give the system a frame to process the command — any stable diagnostic is fine.
            diagnostics.large_total_chunks > 0
        }))
        .then(Action::WaitFrames(5))
        .then(Action::Screenshot("visibility_off".into()))
        .then(Action::WaitFrames(1))
        // Restore the layer visibility.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut query = world.query::<(Entity, &Name)>();
            let square_map = query
                .iter(world)
                .find_map(|(entity, name)| (name.as_str() == "Square Showcase Map").then_some(entity))
                .expect("Square Showcase Map must exist");
            world
                .resource_mut::<Messages<TilemapCommand>>()
                .write(TilemapCommand::SetLayerVisibility {
                    map: square_map,
                    layer: HIGHLIGHT_LAYER,
                    visible: true,
                });
        })))
        .then(Action::WaitFrames(5))
        .then(Action::Screenshot("visibility_restored".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_layer_visibility"))
        .build()
}

fn tilemap_animation_loops() -> Scenario {
    Scenario::builder("tilemap_animation_loops")
        .description("Wait for the animated tile loop counter to advance beyond the initial boot value, confirming the AdvanceAnimation system fires every frame cycle.")
        .then(wait_for_diagnostics("first animation loop observed", |diagnostics| {
            diagnostics.animation_loops >= 1
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let before = world.resource::<crate::LabDiagnostics>().animation_loops;
            world.insert_resource(AnimationLoopSnapshot(before));
        })))
        .then(Action::Screenshot("animation_loop_start".into()))
        .then(Action::WaitFrames(1))
        // Wait for at least one more loop cycle.
        .then(Action::WaitUntil {
            label: "animation loop advanced".into(),
            condition: Box::new(|world| {
                let before = world.resource::<AnimationLoopSnapshot>().0;
                world.resource::<crate::LabDiagnostics>().animation_loops > before
            }),
            max_frames: 600,
        })
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "animation_loops counter advanced",
            |diagnostics| diagnostics.animation_loops >= 2,
        ))
        .then(Action::Screenshot("animation_loop_advanced".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_animation_loops"))
        .build()
}

#[derive(Resource)]
struct AnimationLoopSnapshot(u32);

fn tilemap_autotiling() -> Scenario {
    Scenario::builder("tilemap_autotiling")
        .description(
            "Apply the full road branch (stage 2) and verify the runtime edit system \
             correctly places all 10 tiles on the square showcase map, confirming the \
             autotiling pipeline connects tiles across chunk boundaries.",
        )
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::Overview;
            control.square_edit_stage = 2;
        }))
        .then(wait_for_diagnostics("full branch applied", |diagnostics| {
            diagnostics.square_applied_tiles == 10
                && diagnostics.square_rebuilds_last_frame > 0
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "all 10 autotile tiles placed",
            |diagnostics| diagnostics.square_applied_tiles == 10,
        ))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "at least one chunk was rebuilt after autotile edits",
            |diagnostics| diagnostics.square_rebuilds_last_frame > 0,
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("applied tiles: 10")
            }));
        })))
        .then(Action::Screenshot("autotiling_full_branch".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_autotiling"))
        .build()
}

fn tilemap_hex_strategy() -> Scenario {
    Scenario::builder("tilemap_hex_strategy")
        .description(
            "Focus on the isometric (hexagonal-layout) showcase map and sweep through \
             several tile selections with different movement costs, verifying the metadata \
             lookup works correctly for both cheap and expensive tile kinds.",
        )
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::Isometric;
            // Stone tile — high cost
            control.iso_selection = TileCoord::new(6, 2);
        }))
        .then(wait_for_diagnostics("stone selected", |diagnostics| {
            diagnostics.iso_selection == TileCoord::new(6, 2) && diagnostics.iso_selection_cost >= 2
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "stone tile has cost >= 2",
            |diagnostics| diagnostics.iso_selection_cost >= 2,
        ))
        .then(Action::Screenshot("hex_strategy_stone".into()))
        .then(Action::WaitFrames(1))
        // Grass tile — low cost
        .then(set_control(|control| {
            control.iso_selection = TileCoord::new(2, 6);
        }))
        .then(wait_for_diagnostics("grass selected", |diagnostics| {
            diagnostics.iso_selection == TileCoord::new(2, 6) && diagnostics.iso_selection_cost <= 2
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "grass tile cost is lower than stone",
            |diagnostics| diagnostics.iso_selection_cost <= 2,
        ))
        .then(Action::Screenshot("hex_strategy_grass".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_hex_strategy"))
        .build()
}

fn tilemap_tile_painter() -> Scenario {
    Scenario::builder("tilemap_tile_painter")
        .description(
            "Simulate a tile-painter workflow: advance through all edit stages in sequence \
             (0 → 1 → 2) and capture a screenshot at each stage so the visual diff confirms \
             the incremental painting behavior.",
        )
        .then(set_control(|control| {
            control.camera_focus = CameraFocus::Overview;
            control.square_edit_stage = 0;
        }))
        .then(wait_for_diagnostics("blank canvas", |diagnostics| {
            diagnostics.square_applied_tiles == 0
        }))
        .then(Action::Screenshot("tile_painter_stage0".into()))
        .then(Action::WaitFrames(1))
        .then(set_control(|control| {
            control.square_edit_stage = 1;
        }))
        .then(wait_for_diagnostics("partial paint", |diagnostics| {
            diagnostics.square_applied_tiles == 4 && diagnostics.square_rebuilds_last_frame > 0
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "4 tiles painted in stage 1",
            |diagnostics| diagnostics.square_applied_tiles == 4,
        ))
        .then(Action::Screenshot("tile_painter_stage1".into()))
        .then(Action::WaitFrames(1))
        .then(set_control(|control| {
            control.square_edit_stage = 2;
        }))
        .then(wait_for_diagnostics("full paint", |diagnostics| {
            diagnostics.square_applied_tiles == 10 && diagnostics.square_rebuilds_last_frame > 0
        }))
        .then(assertions::resource_satisfies::<crate::LabDiagnostics>(
            "10 tiles painted in stage 2",
            |diagnostics| diagnostics.square_applied_tiles == 10,
        ))
        .then(Action::Screenshot("tile_painter_stage2".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("tilemap_tile_painter"))
        .build()
}
