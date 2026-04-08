#[cfg(feature = "e2e")]
mod e2e;

use saddle_world_tilemap_example_support as support;

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_world_tilemap::{
    TileCell, TileCoord, TileKindId, TilemapCommand, TilemapDebugOverlay, TilemapDebugSettings,
    TilemapPlugin,
};
use support::{DemoPalette, GROUND_LAYER, HIGHLIGHT_LAYER, OverlayText};

const CANVAS_SIZE: UVec2 = UVec2::new(32, 24);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrushMode {
    Pencil,
    Line,
    Circle,
    Fill,
    Eraser,
}

impl std::fmt::Display for BrushMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pencil => write!(f, "Pencil"),
            Self::Line => write!(f, "Line"),
            Self::Circle => write!(f, "Circle"),
            Self::Fill => write!(f, "Flood Fill"),
            Self::Eraser => write!(f, "Eraser"),
        }
    }
}

#[derive(Resource)]
struct PainterDemo {
    map: Entity,
    palette: DemoPalette,
    brush_mode: BrushMode,
    brush_tile_index: usize,
    brush_radius: u32,
    line_start: Option<TileCoord>,
    hovered: Option<TileCoord>,
    last_paint: Option<TileCoord>,
}

#[derive(Resource, Default)]
struct PainterAutomation {
    hovered_override: Option<TileCoord>,
    pending_click: Option<TileCoord>,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TilePainterSystems {
    Drive,
}

const PALETTE_ORDER: &[fn(&support::DemoTileIds) -> TileKindId] = &[
    |t| t.grass,
    |t| t.soil,
    |t| t.sand,
    |t| t.rock,
    |t| t.wall,
    |t| t.flower,
    |t| t.crop,
    |t| t.water,
    |t| t.road,
];

const PALETTE_NAMES: &[&str] = &[
    "Grass", "Soil", "Sand", "Rock", "Wall", "Flower", "Crop", "Water", "Road",
];

fn main() {
    let mut app = App::new();

    app.insert_resource(support::TilemapExamplePane {
            debug_enabled: true,
            draw_chunk_bounds: true,
            ..default()
        })
        .insert_resource(PainterAutomation::default())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "tilemap tile painter — draw with mouse".into(),
                        resolution: (1360, 920).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(support::pane_plugins())
        .add_plugins(
            TilemapPlugin::default().with_debug_settings(TilemapDebugSettings {
                enabled: true,
                draw_dirty_chunks: true,
                ..default()
            }),
        )
        .register_pane::<support::TilemapExamplePane>()
        .configure_sets(Update, TilePainterSystems::Drive)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                support::sync_example_pane,
                painter_input,
                update_overlay,
            )
                .chain()
                .in_set(TilePainterSystems::Drive),
        );
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::TilePainterExampleE2EPlugin);

    app.run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let palette = DemoPalette::new(&mut images);
    let map = build_canvas(&palette);
    let center = support::map_local_center(&map, CANVAS_SIZE);

    support::spawn_camera(
        &mut commands,
        "Painter Camera",
        Vec3::new(center.x, center.y, 999.0),
    );
    support::spawn_overlay(
        &mut commands,
        "Tile Painter — Click to paint. The canvas starts grass-filled and the default brush is Soil so the first stroke is visible.\n1-9: select tile, Q/W/E/R/T: brush mode, +/-: brush radius",
    );

    let map_entity = support::spawn_map(
        &mut commands,
        "Painter Canvas",
        map,
        Vec3::ZERO,
        TilemapDebugOverlay {
            draw_chunk_bounds: true,
            draw_dirty_chunks: true,
        },
    );

    commands.insert_resource(PainterDemo {
        map: map_entity,
        palette,
        brush_mode: BrushMode::Pencil,
        brush_tile_index: 1,
        brush_radius: 1,
        line_start: None,
        hovered: None,
        last_paint: None,
    });
}

fn painter_input(
    windows: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut automation: ResMut<PainterAutomation>,
    mut demo: ResMut<PainterDemo>,
    map_query: Query<(&saddle_world_tilemap::Tilemap, &GlobalTransform)>,
    mut commands_out: MessageWriter<TilemapCommand>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        demo.brush_mode = BrushMode::Pencil;
    }
    if keys.just_pressed(KeyCode::KeyW) {
        demo.brush_mode = BrushMode::Line;
        demo.line_start = None;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        demo.brush_mode = BrushMode::Circle;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        demo.brush_mode = BrushMode::Fill;
    }
    if keys.just_pressed(KeyCode::KeyT) {
        demo.brush_mode = BrushMode::Eraser;
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        demo.brush_radius = (demo.brush_radius + 1).min(8);
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        demo.brush_radius = demo.brush_radius.saturating_sub(1).max(1);
    }

    for (i, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*key) && i < PALETTE_ORDER.len() {
            demo.brush_tile_index = i;
        }
    }

    let Ok((map, map_transform)) = map_query.get(demo.map) else {
        return;
    };
    let (camera, camera_transform) = *camera;

    let hovered = automation.hovered_override.or_else(|| {
        support::cursor_world(windows.into_inner(), camera, camera_transform)
            .and_then(|world| map.geometry.world_to_tile(map_transform, world))
            .filter(in_canvas_bounds)
    });

    if let Some(prev) = demo.hovered {
        if Some(prev) != hovered {
            commands_out.write(TilemapCommand::ClearTile {
                map: demo.map,
                layer: HIGHLIGHT_LAYER,
                coord: prev,
            });
        }
    }
    if let Some(next) = hovered {
        commands_out.write(TilemapCommand::SetTile {
            map: demo.map,
            layer: HIGHLIGHT_LAYER,
            coord: next,
            tile: TileCell::new(demo.palette.tiles.square_highlight),
        });
    }
    demo.hovered = hovered;

    let Some(coord) = hovered else {
        return;
    };
    let automation_clicked_here = automation.pending_click.take() == Some(coord);
    if automation_clicked_here {
        demo.last_paint = None;
    }

    let current_tile_kind = PALETTE_ORDER[demo.brush_tile_index](&demo.palette.tiles);

    match demo.brush_mode {
        BrushMode::Pencil => {
            if (buttons.pressed(MouseButton::Left) || automation_clicked_here)
                && demo.last_paint != Some(coord)
            {
                commands_out.write(TilemapCommand::SetTile {
                    map: demo.map,
                    layer: GROUND_LAYER,
                    coord,
                    tile: TileCell::new(current_tile_kind),
                });
                demo.last_paint = Some(coord);
            }
            if !buttons.pressed(MouseButton::Left) && !automation_clicked_here {
                demo.last_paint = None;
            }
        }
        BrushMode::Line => {
            if buttons.just_pressed(MouseButton::Left) || automation_clicked_here {
                if let Some(start) = demo.line_start {
                    commands_out.write(TilemapCommand::FillLine {
                        map: demo.map,
                        layer: GROUND_LAYER,
                        from: start,
                        to: coord,
                        tile: TileCell::new(current_tile_kind),
                    });
                    demo.line_start = None;
                } else {
                    demo.line_start = Some(coord);
                }
            }
        }
        BrushMode::Circle => {
            if buttons.just_pressed(MouseButton::Left) || automation_clicked_here {
                commands_out.write(TilemapCommand::FillCircle {
                    map: demo.map,
                    layer: GROUND_LAYER,
                    center: coord,
                    radius: demo.brush_radius,
                    tile: TileCell::new(current_tile_kind),
                });
            }
        }
        BrushMode::Fill => {
            if buttons.just_pressed(MouseButton::Left) || automation_clicked_here {
                commands_out.write(TilemapCommand::FloodFill {
                    map: demo.map,
                    layer: GROUND_LAYER,
                    start: coord,
                    tile: TileCell::new(current_tile_kind),
                    max_tiles: 2048,
                });
            }
        }
        BrushMode::Eraser => {
            if (buttons.pressed(MouseButton::Left) || automation_clicked_here)
                && demo.last_paint != Some(coord)
            {
                commands_out.write(TilemapCommand::ClearTile {
                    map: demo.map,
                    layer: GROUND_LAYER,
                    coord,
                });
                demo.last_paint = Some(coord);
            }
            if !buttons.pressed(MouseButton::Left) && !automation_clicked_here {
                demo.last_paint = None;
            }
        }
    }
}

fn update_overlay(
    demo: Res<PainterDemo>,
    map_query: Query<&saddle_world_tilemap::TilemapDiagnostics>,
    mut overlay: Single<&mut Text, With<OverlayText>>,
) {
    let diagnostics = map_query.get(demo.map).ok();
    let tile_name = PALETTE_NAMES[demo.brush_tile_index];
    let hover_str = demo
        .hovered
        .map(|c| format!("({}, {})", c.x, c.y))
        .unwrap_or_else(|| "outside".to_string());
    let line_str = demo
        .line_start
        .map(|c| format!("from ({}, {})", c.x, c.y))
        .unwrap_or_default();
    let edits = diagnostics.map_or(0, |d| d.tile_edits_this_frame);
    let chunks = diagnostics.map_or(0, |d| d.logical_chunks_total);

    overlay.0 = format!(
        "Tile Painter — Click to paint\n\
        1-9: tile  Q: pencil  W: line  E: circle  R: flood  T: eraser  +/-: radius\n\
        Brush: {} [{}]  Radius: {}  Hover: {} {}\n\
        Edits/frame: {}  Chunks: {}",
        demo.brush_mode, tile_name, demo.brush_radius, hover_str, line_str, edits, chunks,
    );
}

fn in_canvas_bounds(coord: &TileCoord) -> bool {
    coord.x >= 0 && coord.y >= 0 && coord.x < CANVAS_SIZE.x as i32 && coord.y < CANVAS_SIZE.y as i32
}

fn build_canvas(palette: &DemoPalette) -> saddle_world_tilemap::Tilemap {
    use saddle_world_tilemap::*;

    let geometry = TilemapGeometry::square(Vec2::splat(30.0));
    let mut map = Tilemap::new(geometry, UVec2::splat(8));
    let catalog = palette.catalog();

    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            GROUND_LAYER,
            "Canvas",
            TileLayerRenderConfig::new(palette.atlas.clone()).with_z_index(0.0),
        ),
        catalog.clone(),
    ));
    map.insert_layer(TileLayerState::new(
        TileLayerConfig::visual(
            HIGHLIGHT_LAYER,
            "Cursor",
            TileLayerRenderConfig::new(palette.atlas.clone())
                .with_z_index(4.0)
                .with_tint(Color::srgba(1.0, 1.0, 1.0, 0.75)),
        ),
        catalog,
    ));

    for y in 0..CANVAS_SIZE.y as i32 {
        for x in 0..CANVAS_SIZE.x as i32 {
            map.set_tile(
                GROUND_LAYER,
                TileCoord::new(x, y),
                TileCell::new(palette.tiles.grass),
            );
        }
    }

    map
}
