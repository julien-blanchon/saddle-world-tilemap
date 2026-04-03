use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{TilemapCommand, TilemapDebugOverlay, TilemapPlugin};
use support::{DETAIL_LAYER, DemoPalette, OverlayText, SQUARE_SIZE};

#[derive(Resource)]
struct LayeredDemo {
    map: Entity,
    timer: Timer,
    detail_visible: bool,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap layered map".into(),
                        resolution: (1360, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(support::pane_plugins())
        .add_plugins(TilemapPlugin::default())
        .register_pane::<support::TilemapExamplePane>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                support::sync_example_pane,
                toggle_detail_layer,
                update_overlay,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = support::build_square_showcase_map(&palette);
    let center = support::map_local_center(&map, SQUARE_SIZE);

    support::spawn_camera(
        &mut commands,
        "Layered Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Ground stays fixed while the detail layer blinks on and off. The collision-only layer remains present regardless of render visibility.",
    );
    support::spawn_label(
        &mut commands,
        "Layer visibility toggles",
        Vec3::new(center.x, center.y + 330.0, 5.0),
    );

    let map = support::spawn_map(
        &mut commands,
        "Layered Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );

    commands.insert_resource(LayeredDemo {
        map,
        timer: Timer::from_seconds(1.1, TimerMode::Repeating),
        detail_visible: true,
    });
}

fn toggle_detail_layer(
    time: Res<Time>,
    mut demo: ResMut<LayeredDemo>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    if !demo.timer.tick(time.delta()).just_finished() {
        return;
    }

    demo.detail_visible = !demo.detail_visible;
    commands_out.write(TilemapCommand::SetLayerVisibility {
        map: demo.map,
        layer: DETAIL_LAYER,
        visible: demo.detail_visible,
    });
}

fn update_overlay(
    demo: Res<LayeredDemo>,
    diagnostics: Single<
        &saddle_world_tilemap::TilemapDiagnostics,
        With<saddle_world_tilemap::TilemapRoot>,
    >,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    overlay.0 = format!(
        "Ground stays fixed while the detail layer blinks on and off. The collision-only layer remains present regardless of render visibility.\nDetail layer visible: {}\nVisible chunk rebuilds last frame: {}  collision chunks: {}",
        demo.detail_visible,
        diagnostics.chunks_rebuilt_this_frame,
        diagnostics.collision_chunks_total,
    );
}
