use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_tilemap::{TileCoord, Tilemap};
use saddle_world_tilemap_example_support as support;

use crate::{RuntimeEditingAutomation, RuntimeEditingDemo, RuntimeEditingSystems};

pub struct RuntimeEditingExampleE2EPlugin;

impl Plugin for RuntimeEditingExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(RuntimeEditingSystems::Drive));

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = support::parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                init_scenario(app, scenario);
            } else {
                error!(
                    "[tilemap_runtime_editing:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "runtime_editing_cycle"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "runtime_editing_cycle" => Some(editing_cycle()),
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
        max_frames: 120,
    }
}

fn smoke() -> Scenario {
    Scenario::builder("smoke_launch")
        .description(
            "Launch the runtime-editing example, freeze the timer-driven loop, and capture the initial authored map before any edits apply.",
        )
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<RuntimeEditingAutomation>().pause_timer = true;
        })))
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Runtime Editing Map"));
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("message-driven") && text.contains("Current phase")
            }));
        })))
        .then(Action::Screenshot("runtime_editing_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn editing_cycle() -> Scenario {
    Scenario::builder("runtime_editing_cycle")
        .description(
            "Step the runtime-editing loop through fill, set, swap, and reset phases, verifying the underlying tilemap state after each message-driven operation.",
        )
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut automation = world.resource_mut::<RuntimeEditingAutomation>();
            automation.pause_timer = true;
            automation.advance_once = false;
        })))
        .then(Action::WaitFrames(10))
        .then(advance_phase())
        .then(wait_until("phase 1 applied", |world| {
            world.resource::<RuntimeEditingDemo>().phase == 1
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (map_entity, soil_kind) = {
                let demo = world.resource::<RuntimeEditingDemo>();
                (demo.map, demo.soil_kind)
            };
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("runtime-editing map should exist");
            assert_eq!(
                map.get_tile(support::GROUND_LAYER, TileCoord::new(13, 11))
                    .map(|tile| tile.kind),
                Some(soil_kind)
            );
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("fill ground patch")
            }));
        })))
        .then(Action::Screenshot("runtime_editing_phase1".into()))
        .then(Action::WaitFrames(1))
        .then(advance_phase())
        .then(wait_until("phase 2 applied", |world| {
            world.resource::<RuntimeEditingDemo>().phase == 2
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (map_entity, crop_kind) = {
                let demo = world.resource::<RuntimeEditingDemo>();
                (demo.map, demo.crop_kind)
            };
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("runtime-editing map should exist");
            assert_eq!(
                map.get_tile(support::DETAIL_LAYER, TileCoord::new(13, 11))
                    .map(|tile| tile.kind),
                Some(crop_kind)
            );
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("plant crop accents")
            }));
        })))
        .then(Action::Screenshot("runtime_editing_phase2".into()))
        .then(Action::WaitFrames(1))
        .then(advance_phase())
        .then(wait_until("phase 3 applied", |world| {
            world.resource::<RuntimeEditingDemo>().phase == 3
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (map_entity, crop_kind, road_kind) = {
                let demo = world.resource::<RuntimeEditingDemo>();
                (demo.map, demo.crop_kind, demo.road_kind)
            };
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("runtime-editing map should exist");
            assert_eq!(
                map.get_tile(support::DETAIL_LAYER, TileCoord::new(13, 11))
                    .map(|tile| tile.kind),
                Some(road_kind)
            );
            assert_eq!(
                map.get_tile(support::DETAIL_LAYER, TileCoord::new(10, 4))
                    .map(|tile| tile.kind),
                Some(crop_kind)
            );
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("swap crops into a road branch")
            }));
        })))
        .then(Action::Screenshot("runtime_editing_phase3".into()))
        .then(Action::WaitFrames(1))
        .then(advance_phase())
        .then(wait_until("phase 4 applied", |world| {
            world.resource::<RuntimeEditingDemo>().phase == 4
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (map_entity, grass_kind, wall_kind) = {
                let demo = world.resource::<RuntimeEditingDemo>();
                (demo.map, demo.grass_kind, demo.wall_kind)
            };
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("runtime-editing map should exist");
            assert_eq!(
                map.get_tile(support::GROUND_LAYER, TileCoord::new(13, 11))
                    .map(|tile| tile.kind),
                Some(grass_kind)
            );
            assert!(
                map.get_tile(support::DETAIL_LAYER, TileCoord::new(13, 11))
                    .is_none()
            );
            assert_eq!(
                map.get_tile(support::COLLISION_LAYER, TileCoord::new(16, 6))
                    .map(|tile| tile.kind),
                Some(wall_kind)
            );
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("reset visuals and touch collision-only cells")
            }));
        })))
        .then(Action::Screenshot("runtime_editing_phase4".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn advance_phase() -> Action {
    Action::Custom(Box::new(|world: &mut World| {
        world.resource_mut::<RuntimeEditingAutomation>().advance_once = true;
    }))
}
