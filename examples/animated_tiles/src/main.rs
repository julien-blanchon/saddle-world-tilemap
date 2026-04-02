use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_world_tilemap::{TileAnimationLooped, TilemapDebugOverlay, TilemapPlugin};
use support::{DemoPalette, OverlayText, SQUARE_SIZE};

#[derive(Resource, Default)]
struct AnimatedDemo {
    loop_count: u32,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap animated tiles".into(),
                        resolution: (1360, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(TilemapPlugin::default())
        .init_resource::<AnimatedDemo>()
        .add_systems(Startup, setup)
        .add_systems(Update, (count_loops, update_overlay))
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = support::build_square_showcase_map(&palette);
    let center = support::map_local_center(&map, SQUARE_SIZE);

    support::spawn_camera(
        &mut commands,
        "Animated Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Water tiles share a definition-driven animation. Only chunks containing animated kinds rebuild when the frame changes.",
    );
    support::spawn_label(
        &mut commands,
        "Animated water strip",
        Vec3::new(center.x, center.y + 330.0, 5.0),
    );
    support::spawn_map(
        &mut commands,
        "Animated Map",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay::default(),
    );
}

fn count_loops(mut events: MessageReader<TileAnimationLooped>, mut demo: ResMut<AnimatedDemo>) {
    for _ in events.read() {
        demo.loop_count += 1;
    }
}

fn update_overlay(
    demo: Res<AnimatedDemo>,
    diagnostics: Single<
        &saddle_world_tilemap::TilemapDiagnostics,
        With<saddle_world_tilemap::TilemapRoot>,
    >,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    overlay.0 = format!(
        "Water tiles share a definition-driven animation. Only chunks containing animated kinds rebuild when the frame changes.\nAnimation loops observed: {}\nAnimated chunks: {}  last-frame rebuilds: {}",
        demo.loop_count, diagnostics.animated_chunks_total, diagnostics.chunks_rebuilt_this_frame,
    );
}
