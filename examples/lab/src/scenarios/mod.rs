mod support;

use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};

use crate::{CameraFocus, LabControl};
use saddle_world_tilemap::TileCoord;

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "tilemap_smoke",
        "tilemap_runtime_edit",
        "tilemap_isometric_pick",
        "tilemap_large_map",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(tilemap_smoke("smoke_launch")),
        "tilemap_smoke" => Some(tilemap_smoke("tilemap_smoke")),
        "tilemap_runtime_edit" => Some(tilemap_runtime_edit()),
        "tilemap_isometric_pick" => Some(tilemap_isometric_pick()),
        "tilemap_large_map" => Some(tilemap_large_map()),
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
