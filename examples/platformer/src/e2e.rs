use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_tilemap::Tilemap;
use saddle_world_tilemap_example_support as support;

use crate::{PlatformerAutomation, PlatformerDemo, PlatformerSystems};

pub struct PlatformerExampleE2EPlugin;

impl Plugin for PlatformerExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(PlatformerSystems::Drive));

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
                    "[tilemap_platformer:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "platformer_first_platform"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "platformer_first_platform" => Some(first_platform()),
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
        max_frames: 240,
    }
}

fn smoke() -> Scenario {
    Scenario::builder("smoke_launch")
        .description(
            "Launch the platformer example, verify the authored level appears, and capture the baseline player spawn on the ground.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Platformer Level"));
            assert!(support::render_chunk_count(world) > 0);
            assert!(support::collision_chunk_count(world) > 0);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Platformer") && text.contains("Grounded")
            }));

            let demo = world.resource::<PlatformerDemo>();
            let map = world
                .get::<Tilemap>(demo.map)
                .expect("platformer map should exist");
            assert!(map
                .get_tile(support::HIGHLIGHT_LAYER, demo.player_coord)
                .is_some());
        })))
        .then(Action::Screenshot("platformer_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn first_platform() -> Scenario {
    Scenario::builder("platformer_first_platform")
        .description(
            "Hold right and trigger a jump through the automation hook, then verify the player can travel sideways in the air and land on the first raised platform.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut automation = world.resource_mut::<PlatformerAutomation>();
            automation.horizontal_axis = 1;
            automation.jump_requested = true;
        })))
        .then(wait_until("player reaches first platform", |world| {
            let demo = world.resource::<PlatformerDemo>();
            demo.grounded
                && demo.player_coord.y == 14
                && (8..=12).contains(&demo.player_coord.x)
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<PlatformerAutomation>().horizontal_axis = 0;

            let demo = world.resource::<PlatformerDemo>();
            assert!(demo.player_coord.x >= 8);
            assert_eq!(demo.player_coord.y, 14);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Player: (") && text.contains("Grounded: true")
            }));
        })))
        .then(Action::Screenshot("platformer_first_platform".into()))
        .then(Action::WaitFrames(1))
        .build()
}
