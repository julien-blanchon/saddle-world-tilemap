use bevy::prelude::*;
use saddle_bevy_e2e::{E2EPlugin, E2ESet, action::Action, init_scenario, scenario::Scenario};
use saddle_world_tilemap::Tilemap;
use saddle_world_tilemap_example_support as support;

use crate::{
    BrushMode, PainterAutomation, PainterDemo, TilePainterSystems, GROUND_LAYER, TileCoord,
};

pub struct TilePainterExampleE2EPlugin;

impl Plugin for TilePainterExampleE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(Update, E2ESet.before(TilePainterSystems::Drive));

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
                    "[tilemap_tile_painter:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "tile_painter_paint_and_erase"]
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke()),
        "tile_painter_paint_and_erase" => Some(paint_and_erase()),
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
            "Launch the tile painter, verify the canvas and overlay appear, and capture the grass-filled baseline with the contrasting soil brush selected.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::named_entity_exists(world, "Painter Canvas"));
            assert!(support::render_chunk_count(world) > 0);
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Tile Painter") && text.contains("Brush: Pencil [Soil]")
            }));
        })))
        .then(Action::Screenshot("tile_painter_smoke".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn paint_and_erase() -> Scenario {
    const TARGET: TileCoord = TileCoord::new(10, 8);

    Scenario::builder("tile_painter_paint_and_erase")
        .description(
            "Paint a visible soil stroke onto the grass canvas, then switch to the eraser and clear it again to verify both directions of editing.",
        )
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let mut demo = world.resource_mut::<PainterDemo>();
            demo.brush_mode = BrushMode::Pencil;
            demo.brush_tile_index = 1;
            let mut automation = world.resource_mut::<PainterAutomation>();
            automation.hovered_override = Some(TARGET);
            automation.pending_click = Some(TARGET);
        })))
        .then(wait_until("soil stroke applied", |world| {
            let demo = world.resource::<PainterDemo>();
            let Some(map) = world.get::<Tilemap>(demo.map) else {
                return false;
            };
            map.get_tile(GROUND_LAYER, TARGET)
                .is_some_and(|tile| tile.kind == demo.palette.tiles.soil)
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            assert!(support::overlay_text(world).is_some_and(|text| {
                text.contains("Hover: (10, 8)") && text.contains("Brush: Pencil [Soil]")
            }));
        })))
        .then(Action::Screenshot("tile_painter_painted".into()))
        .then(Action::WaitFrames(1))
        .then(Action::Custom(Box::new(|world: &mut World| {
            world.resource_mut::<PainterDemo>().brush_mode = BrushMode::Eraser;
            world.resource_mut::<PainterAutomation>().pending_click = Some(TARGET);
        })))
        .then(wait_until("stroke erased", |world| {
            let demo = world.resource::<PainterDemo>();
            let Some(map) = world.get::<Tilemap>(demo.map) else {
                return false;
            };
            map.get_tile(GROUND_LAYER, TARGET).is_none()
        }))
        .then(Action::Screenshot("tile_painter_erased".into()))
        .then(Action::WaitFrames(1))
        .build()
}
