use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_tilemap::{TileCoord, Tilemap};
use saddle_world_tilemap_example_support as support;

use crate::{BasicAutomation, BasicDemo, BasicSystems};

pub struct BasicExampleE2EPlugin;

impl Plugin for BasicExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(BasicSystems::Drive));

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
                    "[tilemap_basic:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "basic_hover_pick"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "basic_hover_pick" => Some(hover_pick()),
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
        max_frames: 90,
    }
}

fn smoke() -> Scenario {
    Scenario::builder("smoke_launch")
        .description(
            "Launch the basic tilemap example, verify the map and overlay appear, and capture the authored square-map baseline.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Basic Map"));
            assert!(support::render_chunk_count(world) > 0);
            assert!(support::collision_chunk_count(world) > 0);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("cursor picking") && text.contains("Hovered tile")
            }));
        })))
        .then(Action::Screenshot("basic_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn hover_pick() -> Scenario {
    const TARGET: TileCoord = TileCoord::new(10, 4);

    Scenario::builder("basic_hover_pick")
        .description(
            "Override the hover target to a valid square tile, verify the highlight layer and overlay update, then clear the hover again.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<BasicAutomation>().hovered_override = Some(TARGET);
        })))
        .then(wait_until("hover applied", |world| {
            world.resource::<BasicDemo>().hovered == Some(TARGET)
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let map_entity = world.resource::<BasicDemo>().map;
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("basic map component should exist");
            assert!(map.get_tile(support::HIGHLIGHT_LAYER, TARGET).is_some());
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Hovered tile: (10, 4)") && text.contains("chunk: (1, 0)")
            }));
        })))
        .then(Action::Screenshot("basic_hover".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<BasicAutomation>().hovered_override = None;
        })))
        .then(wait_until("hover cleared", |world| {
            world.resource::<BasicDemo>().hovered.is_none()
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("outside the authored bounds")
            }));
        })))
        .then(Action::Screenshot("basic_hover_cleared".into()))
        .then(Action::WaitFrames(1))
        .build()
}
