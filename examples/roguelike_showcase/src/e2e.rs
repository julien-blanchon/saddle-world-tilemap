use bevy::prelude::*;
use saddle_ai_fov::{FovSystems, GridFovState};
use saddle_bevy_e2e::{
    E2EPlugin, E2ESet, action::Action, actions::assertions, init_scenario, scenario::Scenario,
};
use saddle_world_tilemap::TileCoord;

use crate::{
    DemoInputState, FogOverlayMarker, PlayerGridPosition, RoguelikePane, RoguelikeScene,
};

#[derive(Resource, Clone, Copy)]
struct MoveSnapshot {
    start: IVec2,
    min_distance: i32,
    start_explored: usize,
}

pub struct RoguelikeShowcaseE2EPlugin;

impl Plugin for RoguelikeShowcaseE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(FovSystems::MarkDirty));

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                init_scenario(app, scenario);
            } else {
                error!(
                    "[roguelike_showcase:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}

fn list_scenarios() -> Vec<&'static str> {
    vec![
        "roguelike_showcase_smoke",
        "roguelike_showcase_move_and_regenerate",
        "roguelike_showcase_explore_fog",
    ]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "roguelike_showcase_smoke" => Some(build_smoke()),
        "roguelike_showcase_move_and_regenerate" => Some(build_move_and_regenerate()),
        "roguelike_showcase_explore_fog" => Some(build_explore_fog()),
        _ => None,
    }
}

fn wait_until(
    label: impl Into<String>,
    condition: impl Fn(&World) -> bool + Send + Sync + 'static,
) -> Action {
    Action::WaitUntil {
        label: label.into(),
        condition: Box::new(condition),
        max_frames: 180,
    }
}

fn build_smoke() -> Scenario {
    Scenario::builder("roguelike_showcase_smoke")
        .description(
            "Launch the roguelike showcase, wait for the dungeon, fog overlay, and FOV state to initialize, then capture a settled baseline frame.",
        )
        .then(wait_until("roguelike map ready", |world| {
            let Some(scene) = world.get_resource::<RoguelikeScene>() else {
                return false;
            };
            let Some(map_entity) = scene.map_entity else {
                return false;
            };
            let Some(state) = world.get::<GridFovState>(scene.player_entity) else {
                return false;
            };
            world.get_entity(map_entity).is_ok()
                && !scene.marker_entities.is_empty()
                && !state.visible_now.is_empty()
        }))
        .then(Action::WaitFrames(24))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (overlay_entity, player_entity) = {
                let scene = world.resource::<RoguelikeScene>();
                (scene.overlay_entity, scene.player_entity)
            };
            let text = world
                .get::<Text>(overlay_entity)
                .expect("overlay text should exist");
            assert!(text.0.contains("tilemap + FOV + fog of war"));
            assert!(text.0.contains("Seed"));

            let state = world
                .get::<GridFovState>(player_entity)
                .expect("player FOV state should exist");
            assert!(!state.visible_now.is_empty());
            assert!(!state.explored.is_empty());
            assert_player_aligned(world);

            let mut overlays = world.query_filtered::<Entity, With<FogOverlayMarker>>();
            assert!(overlays.single(world).is_ok());
        })))
        .then(Action::Screenshot("roguelike_showcase_smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("roguelike_showcase_smoke summary"))
        .build()
}

fn build_move_and_regenerate() -> Scenario {
    Scenario::builder("roguelike_showcase_move_and_regenerate")
        .description(
            "Move the scout through the generated dungeon, verify fog/FOV stay aligned with the tilemap, then retune the seed and confirm the dungeon rebuild completes cleanly.",
        )
        .then(wait_until("roguelike map ready", |world| {
            let Some(scene) = world.get_resource::<RoguelikeScene>() else {
                return false;
            };
            scene.map_entity.is_some() && world.get::<GridFovState>(scene.player_entity).is_some()
        }))
        .then(Action::WaitFrames(24))
        .then(begin_walk())
        .then(wait_until("player moved", movement_reached_target))
        .then(end_walk())
        .then(Action::WaitFrames(24))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert_player_aligned(world);
            assert_player_fog_tracking(world);
        })))
        .then(Action::Screenshot("roguelike_showcase_moved".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let current_seed = world.resource::<RoguelikePane>().seed;
            let target_seed = if current_seed == 19 {
                137
            } else {
                current_seed + 1
            };
            world.resource_mut::<RoguelikePane>().seed = target_seed;
        })))
        .then(wait_until("dungeon rebuilt", |world| {
            let Some(scene) = world.get_resource::<RoguelikeScene>() else {
                return false;
            };
            let pane = world.resource::<RoguelikePane>();
            scene.snapshot.seed == pane.seed && scene.map_entity.is_some()
        }))
        .then(Action::WaitFrames(24))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (overlay_entity, seed) = {
                let scene = world.resource::<RoguelikeScene>();
                (scene.overlay_entity, world.resource::<RoguelikePane>().seed)
            };
            let text = world
                .get::<Text>(overlay_entity)
                .expect("overlay text should exist");
            assert!(text.0.contains(&format!("Seed {seed}")));
            assert_player_aligned(world);
        })))
        .then(Action::Screenshot("roguelike_showcase_regenerated".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary(
            "roguelike_showcase_move_and_regenerate summary",
        ))
        .build()
}

fn build_explore_fog() -> Scenario {
    Scenario::builder("roguelike_showcase_explore_fog")
        .description(
            "Advance the scout far enough to reveal new space, then verify the fog/FOV state and tilemap alignment remain coherent at the new location.",
        )
        .then(wait_until("roguelike map ready", |world| {
            let Some(scene) = world.get_resource::<RoguelikeScene>() else {
                return false;
            };
            scene.map_entity.is_some() && world.get::<GridFovState>(scene.player_entity).is_some()
        }))
        .then(Action::WaitFrames(24))
        .then(begin_walk())
        .then(wait_until("player advanced into fog", movement_reached_target))
        .then(end_walk())
        .then(Action::WaitFrames(24))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert_player_aligned(world);
            assert_player_fog_tracking(world);

            let scene = world.resource::<RoguelikeScene>();
            let snapshot = world.resource::<MoveSnapshot>();
            let player = world
                .get::<PlayerGridPosition>(scene.player_entity)
                .expect("player position should exist");
            assert!(player.cell != snapshot.start);
        })))
        .then(Action::Screenshot("roguelike_showcase_explore_fog".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary(
            "roguelike_showcase_explore_fog summary",
        ))
        .build()
}

fn begin_walk() -> Action {
    Action::Custom(Box::new(|world: &mut World| {
        let (player_entity, start, axis, min_distance, start_explored) = {
            let scene = world.resource::<RoguelikeScene>();
            let player = world
                .get::<PlayerGridPosition>(scene.player_entity)
                .expect("player position should exist");
            let fov_state = world
                .get::<GridFovState>(scene.player_entity)
                .expect("player FOV state should exist");
            let directions = [
                (IVec2::X, Vec2::X),
                (-IVec2::X, -Vec2::X),
                (IVec2::Y, Vec2::Y),
                (-IVec2::Y, -Vec2::Y),
            ];
            let (axis, min_distance) = directions
                .into_iter()
                .find_map(|(grid, axis)| {
                    scene
                        .dungeon
                        .tile(player.cell + grid)
                        .filter(|tile| !tile.blocks_movement())?;
                    let min_distance = scene
                        .dungeon
                        .tile(player.cell + grid * 2)
                        .filter(|tile| !tile.blocks_movement())
                        .map_or(1, |_| 2);
                    Some((axis, min_distance))
                })
                .expect("generated dungeon should have at least one walkable neighbor");
            (
                scene.player_entity,
                player.cell,
                axis,
                min_distance,
                fov_state.explored.len(),
            )
        };

        world.insert_resource(MoveSnapshot {
            start,
            min_distance,
            start_explored,
        });
        world
            .get_mut::<DemoInputState>(player_entity)
            .expect("input state should exist")
            .move_axis = axis;
    }))
}

fn end_walk() -> Action {
    Action::Custom(Box::new(|world: &mut World| {
        let player_entity = world.resource::<RoguelikeScene>().player_entity;
        world
            .get_mut::<DemoInputState>(player_entity)
            .expect("input state should exist")
            .move_axis = Vec2::ZERO;
    }))
}

fn movement_reached_target(world: &World) -> bool {
    let Some(scene) = world.get_resource::<RoguelikeScene>() else {
        return false;
    };
    let Some(snapshot) = world.get_resource::<MoveSnapshot>() else {
        return false;
    };
    let Some(player) = world.get::<PlayerGridPosition>(scene.player_entity) else {
        return false;
    };
    (player.cell.x - snapshot.start.x).abs() + (player.cell.y - snapshot.start.y).abs()
        >= snapshot.min_distance
}

fn assert_player_fog_tracking(world: &mut World) {
    let scene = world.resource::<RoguelikeScene>();
    let snapshot = world.resource::<MoveSnapshot>();
    let state = world
        .get::<GridFovState>(scene.player_entity)
        .expect("player FOV state should exist");
    let player = world
        .get::<PlayerGridPosition>(scene.player_entity)
        .expect("player position should exist");
    assert!(state.visible_now.contains(&player.cell));
    assert!(state.explored.len() >= snapshot.start_explored);
}

fn assert_player_aligned(world: &mut World) {
    let scene = world.resource::<RoguelikeScene>();
    let map_entity = scene.map_entity.expect("roguelike map should exist");
    let player = world
        .get::<PlayerGridPosition>(scene.player_entity)
        .expect("player position should exist");
    let player_transform = world
        .get::<Transform>(scene.player_entity)
        .expect("player transform should exist");
    let map = world
        .get::<saddle_world_tilemap::Tilemap>(map_entity)
        .expect("roguelike map should exist");
    let map_transform = world
        .get::<GlobalTransform>(map_entity)
        .expect("roguelike map transform should exist");
    let expected = map.geometry.tile_to_world(
        map_transform,
        TileCoord::new(player.cell.x, player.cell.y),
    );
    let actual = player_transform.translation.truncate();
    assert!(
        actual.distance(expected) <= 0.1,
        "player is misaligned with the tilemap: actual {actual:?}, expected {expected:?}",
    );
}
