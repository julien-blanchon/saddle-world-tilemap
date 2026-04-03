mod animation;
mod autotile;
mod chunk;
mod collision;
mod commands;
mod components;
mod config;
mod coordinates;
mod debug;
mod import;
mod layer;
mod rendering;
mod systems;

pub use animation::{TileAnimation, TileAnimationFrame};
pub use autotile::{
    AutotileBinding, AutotileGroupId, AutotileNeighborhood, AutotileRuleSet, AutotileRuleSetId,
    compute_autotile_mask,
};
use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};
pub use chunk::TileChunk;
pub use collision::{
    TileCollisionCell, TileCollisionDescriptor, TileCollisionShape, TilemapCollisionChunk,
};
pub use commands::{
    ChunkRebuilt, LayerVisibilityChanged, TileAnimationLooped, TileChanged, TilemapCommand,
};
pub use components::{
    TilemapBundle, TilemapDiagnostics, TilemapLayerNode, TilemapRenderChunk, TilemapRoot,
};
pub use config::TileAtlasLayout;
pub use coordinates::{
    ChunkCoord, TileCoord, TileRect, TileRowDirection, TilemapGeometry, TilemapHexParity,
    TilemapOrientation,
};
pub use debug::{TilemapDebugOverlay, TilemapDebugSettings};
pub use import::{
    ImportedTilemapScene, TileObjectSpawn, TilePropertyValue, TiledImportError, TiledImportOptions,
    import_tiled_json_str,
};
pub use layer::{
    TileCatalog, TileCell, TileKind, TileKindId, TileLayerConfig, TileLayerId,
    TileLayerRenderConfig, TileLayerState, TileOrientation, TileRenderRule, TileVisual, Tilemap,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TilemapSystems {
    Prepare,
    ApplyCommands,
    AdvanceAnimation,
    ResolveAutotiling,
    SyncCollision,
    SyncRender,
    Debug,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct TilemapPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
    pub debug_settings: TilemapDebugSettings,
}

impl TilemapPlugin {
    #[must_use]
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
            debug_settings: TilemapDebugSettings::default(),
        }
    }

    #[must_use]
    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }

    #[must_use]
    pub fn with_debug_settings(mut self, debug_settings: TilemapDebugSettings) -> Self {
        self.debug_settings = debug_settings;
        self
    }
}

impl Default for TilemapPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        if !app.world().contains_resource::<TilemapDebugSettings>() {
            app.insert_resource(self.debug_settings);
        }

        app.init_resource::<systems::TilemapRuntimeControl>()
            .add_message::<TilemapCommand>()
            .add_message::<TileChanged>()
            .add_message::<ChunkRebuilt>()
            .add_message::<LayerVisibilityChanged>()
            .add_message::<TileAnimationLooped>()
            .register_type::<AutotileBinding>()
            .register_type::<AutotileGroupId>()
            .register_type::<AutotileNeighborhood>()
            .register_type::<AutotileRuleSet>()
            .register_type::<AutotileRuleSetId>()
            .register_type::<ChunkCoord>()
            .register_type::<TileAnimation>()
            .register_type::<TileAnimationFrame>()
            .register_type::<TileAtlasLayout>()
            .register_type::<TileCatalog>()
            .register_type::<TileCell>()
            .register_type::<TileCollisionCell>()
            .register_type::<TileCollisionDescriptor>()
            .register_type::<TileCollisionShape>()
            .register_type::<TileChunk>()
            .register_type::<TileCoord>()
            .register_type::<TileKind>()
            .register_type::<TileKindId>()
            .register_type::<TileLayerConfig>()
            .register_type::<TileLayerId>()
            .register_type::<TilemapLayerNode>()
            .register_type::<TileLayerRenderConfig>()
            .register_type::<TileLayerState>()
            .register_type::<TileObjectSpawn>()
            .register_type::<TilePropertyValue>()
            .register_type::<TileOrientation>()
            .register_type::<TileRect>()
            .register_type::<TileRenderRule>()
            .register_type::<TileVisual>()
            .register_type::<Tilemap>()
            .register_type::<TilemapCollisionChunk>()
            .register_type::<TilemapDebugOverlay>()
            .register_type::<TilemapDebugSettings>()
            .register_type::<TilemapDiagnostics>()
            .register_type::<TilemapGeometry>()
            .register_type::<TilemapHexParity>()
            .register_type::<TilemapOrientation>()
            .register_type::<TilemapRenderChunk>()
            .register_type::<TilemapRoot>()
            .register_type::<TileRowDirection>()
            .configure_sets(
                self.update_schedule,
                (
                    TilemapSystems::Prepare,
                    TilemapSystems::ApplyCommands,
                    TilemapSystems::AdvanceAnimation,
                    TilemapSystems::ResolveAutotiling,
                    TilemapSystems::SyncCollision,
                    TilemapSystems::SyncRender,
                    TilemapSystems::Debug,
                )
                    .chain(),
            )
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .add_systems(
                self.update_schedule,
                (
                    systems::prepare_maps.in_set(TilemapSystems::Prepare),
                    systems::apply_commands.in_set(TilemapSystems::ApplyCommands),
                    systems::advance_animation.in_set(TilemapSystems::AdvanceAnimation),
                    systems::resolve_dirty_chunks.in_set(TilemapSystems::ResolveAutotiling),
                    systems::sync_collision_chunks.in_set(TilemapSystems::SyncCollision),
                    systems::sync_render_chunks.in_set(TilemapSystems::SyncRender),
                    systems::sync_layer_visibility.in_set(TilemapSystems::SyncRender),
                )
                    .run_if(systems::runtime_is_active),
            );

        if app.is_plugin_added::<bevy::gizmos::GizmoPlugin>() {
            app.add_systems(
                self.update_schedule,
                systems::draw_debug
                    .in_set(TilemapSystems::Debug)
                    .run_if(systems::runtime_is_active),
            );
        }
    }
}
