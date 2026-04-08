use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_tilemap::Tilemap;
use saddle_world_tilemap_example_support as support;

use crate::{AutotileAutomation, AutotileDemo, AutotilingSystems};

pub struct AutotilingExampleE2EPlugin;

impl Plugin for AutotilingExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(AutotilingSystems::Drive));

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
                    "[tilemap_autotiling:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "autotiling_growth"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "autotiling_growth" => Some(growth()),
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
            "Launch the autotiling example, pause timer-driven growth, and capture the initial road layout before the extra branch expands.",
        )
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<AutotileAutomation>().pause_timer = true;
        })))
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Autotile Map"));
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Waiting for the first growth tick")
            }));
        })))
        .then(Action::Screenshot("autotiling_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn growth() -> Scenario {
    let growth_coords = support::square_runtime_edit_coords();
    let fifth = growth_coords[4];
    let last = *growth_coords.last().expect("growth coords should not be empty");
    let total_steps = growth_coords.len();

    Scenario::builder("autotiling_growth")
        .description(
            "Advance the road-growth sequence in controlled steps, verifying the latest highlighted coordinate and final branch completion.",
        )
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut automation = world.resource_mut::<AutotileAutomation>();
            automation.pause_timer = true;
            automation.pending_steps = 0;
        })))
        .then(Action::WaitFrames(10))
        .then(schedule_steps(1))
        .then(wait_until("first growth step", |world| {
            let demo = world.resource::<AutotileDemo>();
            demo.next_index == 1 && demo.latest_coord == Some(support::square_runtime_edit_coords()[0])
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let (map_entity, road_kind) = {
                let demo = world.resource::<AutotileDemo>();
                (demo.map, demo.road_kind)
            };
            let first = support::square_runtime_edit_coords()[0];
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("autotile map should exist");
            assert_eq!(
                map.get_tile(support::DETAIL_LAYER, first).map(|tile| tile.kind),
                Some(road_kind)
            );
        })))
        .then(Action::Screenshot("autotiling_first_step".into()))
        .then(Action::WaitFrames(1))
        .then(schedule_steps(4))
        .then(wait_until("five growth steps", move |world| {
            let demo = world.resource::<AutotileDemo>();
            demo.next_index == 5 && demo.latest_coord == Some(fifth)
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("step: 5/10")
            }));
        })))
        .then(Action::Screenshot("autotiling_mid_growth".into()))
        .then(Action::WaitFrames(1))
        .then(schedule_steps(total_steps - 5))
        .then(wait_until("full branch grown", move |world| {
            let demo = world.resource::<AutotileDemo>();
            demo.next_index == total_steps && demo.latest_coord == Some(last)
        }))
        .then(Action::Custom(Box::new(move |world: &mut World| {
            let (map_entity, road_kind) = {
                let demo = world.resource::<AutotileDemo>();
                (demo.map, demo.road_kind)
            };
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("autotile map should exist");
            assert_eq!(
                map.get_tile(support::DETAIL_LAYER, last).map(|tile| tile.kind),
                Some(road_kind)
            );
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("step: 10/10")
            }));
        })))
        .then(Action::Screenshot("autotiling_complete".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn schedule_steps(steps: usize) -> Action {
    Action::Custom(Box::new(move |world: &mut World| {
        world.resource_mut::<AutotileAutomation>().pending_steps = steps;
    }))
}
