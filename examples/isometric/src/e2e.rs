use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_tilemap::{TileCoord, Tilemap};
use saddle_world_tilemap_example_support as support;

use crate::{IsoAutomation, IsoDemo, IsometricSystems};

pub struct IsometricExampleE2EPlugin;

impl Plugin for IsometricExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(IsometricSystems::Drive));

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
                    "[tilemap_isometric:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "isometric_pick_costs"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "isometric_pick_costs" => Some(pick_costs()),
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
            "Launch the isometric example, verify the battlefield and overlay text appear, and capture the baseline board state.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Isometric Map"));
            assert!(support::render_chunk_count(world) > 0);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("isometric battlefield")
            }));
        })))
        .then(Action::Screenshot("isometric_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn pick_costs() -> Scenario {
    const STONE: TileCoord = TileCoord::new(6, 2);
    const GRASS: TileCoord = TileCoord::new(4, 4);

    Scenario::builder("isometric_pick_costs")
        .description(
            "Drive the isometric hover target across a costly stone tile and a cheaper grass tile, verifying highlight placement and movement-cost feedback.",
        )
        .then(Action::WaitFrames(20))
        .then(set_hover(Some(STONE)))
        .then(wait_until("stone hover applied", |world| {
            world.resource::<IsoDemo>().hovered == Some(STONE)
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let map_entity = world.resource::<IsoDemo>().map;
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("isometric map should exist");
            assert!(map.get_tile(support::HIGHLIGHT_LAYER, STONE).is_some());
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Hovered tile: (6, 2)") && text.contains("Movement cost: 3")
            }));
        })))
        .then(Action::Screenshot("isometric_stone".into()))
        .then(Action::WaitFrames(1))
        .then(set_hover(Some(GRASS)))
        .then(wait_until("grass hover applied", |world| {
            world.resource::<IsoDemo>().hovered == Some(GRASS)
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let map_entity = world.resource::<IsoDemo>().map;
            let map = world
                .get::<Tilemap>(map_entity)
                .expect("isometric map should exist");
            assert!(map.get_tile(support::HIGHLIGHT_LAYER, GRASS).is_some());
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Hovered tile: (4, 4)") && text.contains("Movement cost: 1")
            }));
        })))
        .then(Action::Screenshot("isometric_grass".into()))
        .then(Action::WaitFrames(1))
        .then(set_hover(None))
        .then(wait_until("hover cleared", |world| {
            world.resource::<IsoDemo>().hovered.is_none()
        }))
        .build()
}

fn set_hover(coord: Option<TileCoord>) -> Action {
    Action::Custom(Box::new(move |world: &mut World| {
        world.resource_mut::<IsoAutomation>().hovered_override = coord;
    }))
}
