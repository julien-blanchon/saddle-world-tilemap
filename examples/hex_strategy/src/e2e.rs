use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_hex_grid::AxialHex;
use saddle_world_tilemap::Tilemap;
use saddle_world_tilemap_example_support as support;

use crate::{HexStrategyAutomation, HexStrategyDemo, HexStrategySystems, current_path};

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
struct HighlightSnapshot {
    highlighted: Vec<saddle_world_tilemap::TileCoord>,
}

pub struct HexStrategyExampleE2EPlugin;

impl Plugin for HexStrategyExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(HexStrategySystems::Drive));

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
                    "[tilemap_hex_strategy:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "hex_strategy_path_preview"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "hex_strategy_path_preview" => Some(path_preview()),
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
            "Launch the hex strategy example, verify the board and overlay appear, and capture the authored tactical frontier baseline.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Hex Strategy Map"));
            assert!(support::render_chunk_count(world) > 0);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Hex strategy board")
            }));
        })))
        .then(Action::Screenshot("hex_strategy_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn path_preview() -> Scenario {
    const GOAL: AxialHex = AxialHex::new(3, -2);

    Scenario::builder("hex_strategy_path_preview")
        .description(
            "Pin the hover target to a reachable frontier hex, verify the path preview stabilizes instead of churning, and capture the resulting route.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<HexStrategyAutomation>().hovered_override = Some(GOAL);
        })))
        .then(wait_until("path preview ready", |world| {
            let demo = world.resource::<HexStrategyDemo>();
            demo.hovered == Some(GOAL) && !demo.highlighted.is_empty()
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let demo = world.resource::<HexStrategyDemo>();
            let map = world
                .get::<Tilemap>(demo.map)
                .expect("hex strategy map should exist");
            let path = current_path(&demo, map, GOAL).expect("goal should stay reachable");
            assert_eq!(demo.highlighted.len(), path.cells.len());
            world.insert_resource(HighlightSnapshot {
                highlighted: demo.highlighted.clone(),
            });
        })))
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let demo = world.resource::<HexStrategyDemo>();
            let snapshot = world.resource::<HighlightSnapshot>();
            assert_eq!(demo.highlighted, snapshot.highlighted);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Goal hex: (3, -2)") && text.contains("Path length:")
            }));
        })))
        .then(Action::Screenshot("hex_strategy_path_preview".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<HexStrategyAutomation>().hovered_override = None;
        })))
        .build()
}
