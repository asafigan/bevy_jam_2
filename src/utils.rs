use std::time::Duration;

use bevy::{
    asset::HandleId,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{shape::Quad, *},
    reflect::TypeUuid,
    render::view::RenderLayers,
    transform::TransformSystem,
};

use crate::prefab::*;

pub struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnEvent>()
            .add_startup_system(add_meshes)
            .add_startup_system(add_materials)
            .add_startup_system(load_fonts)
            .add_system(delayed_despawn)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_progress.before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(CoreStage::PostUpdate, propagate_render_layers);
    }
}

fn add_meshes(mut meshes: ResMut<Assets<Mesh>>) {
    let square = Quad {
        size: Vec2::splat(1.0),
        ..default()
    };

    meshes.set_untracked(square_mesh(), square.into());
}

fn add_materials(
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    standard_materials.set_untracked(
        white_standard_material(),
        StandardMaterial {
            unlit: true,
            ..default()
        },
    );

    color_materials.set_untracked(
        white_color_material(),
        ColorMaterial {
            color: Color::WHITE,
            ..default()
        },
    );
}

fn load_fonts(mut font: Local<Option<Handle<Font>>>, asset_server: Res<AssetServer>) {
    if font.is_none() {
        *font = Some(asset_server.load(DEFAULT_FONT_PATH));
    }
}

#[derive(Component, Default)]
pub struct DelayedDespawn {
    timer: Timer,
    reason: Option<DespawnReason>,
}

impl DelayedDespawn {
    pub fn new(duration: Duration) -> Self {
        DelayedDespawn {
            timer: Timer::new(duration, false),
            ..default()
        }
    }

    pub fn from_seconds(duration: f32) -> Self {
        DelayedDespawn::new(Duration::from_secs_f32(duration))
    }

    pub fn with_reason(mut self, reason: DespawnReason) -> Self {
        self.reason = Some(reason);

        self
    }

    pub fn reason(&self) -> Option<DespawnReason> {
        self.reason
    }
}

pub struct DespawnEvent {
    pub entity: Entity,
    pub reason: Option<DespawnReason>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DespawnReason {
    DestroyGem,
    DestroyEnemy,
}

fn delayed_despawn(
    mut events: EventWriter<DespawnEvent>,
    mut delays: Query<(Entity, &mut DelayedDespawn)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut delay) in &mut delays {
        if delay.timer.tick(time.delta()).finished() {
            commands.entity(entity).despawn_recursive();

            events.send(DespawnEvent {
                entity,
                reason: delay.reason,
            });
        }
    }
}

fn propagate_render_layers(
    roots: Query<Entity, (With<RenderLayers>, Without<Parent>)>,
    mut layers: Query<&mut RenderLayers>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    for root in &roots {
        let layer = layers.get(root).unwrap().clone();

        let mut children: Vec<Entity> = children_query
            .get(root)
            .into_iter()
            .flatten()
            .cloned()
            .collect();

        while !children.is_empty() {
            for child in std::mem::take(&mut children) {
                if let Ok(mut child_layer) = layers.get_mut(child) {
                    if *child_layer != layer {
                        *child_layer = layer;
                    }
                } else {
                    commands.entity(child).insert(layer);
                }

                children.extend(children_query.get(child).into_iter().flatten().cloned());
            }
        }
    }
}

#[derive(Component)]
pub struct ProgressBar {
    pub percentage: f32,
    progress: Entity,
}

#[derive(Default)]
pub struct ProgressBarPrefab {
    pub starting_percentage: f32,
    pub transform: Transform,
}

impl Prefab for ProgressBarPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mesh = commands
            .spawn_bundle(PbrBundle {
                mesh: square_mesh(),
                material: white_standard_material(),
                ..default()
            })
            .insert(NotShadowCaster)
            .insert(NotShadowReceiver)
            .id();

        let progress = commands
            .spawn_bundle(SpatialBundle::default())
            .add_child(mesh)
            .id();

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .insert(ProgressBar {
                percentage: self.starting_percentage,
                progress,
            })
            .add_child(progress);
    }
}

fn update_progress(
    progress_bars: Query<&ProgressBar, Changed<ProgressBar>>,
    mut transforms: Query<&mut Transform>,
) {
    for progress_bar in &progress_bars {
        let mut transform = transforms.get_mut(progress_bar.progress).unwrap();

        transform.scale.x = progress_bar.percentage;
    }
}

const SQUARE_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000 - 2);
const WHITE_STANDARD_MATERIAL_ID: HandleId = HandleId::new(StandardMaterial::TYPE_UUID, 10_000 - 2);
const WHITE_COLOR_MATERIAL_ID: HandleId = HandleId::new(ColorMaterial::TYPE_UUID, 10_000 - 2);
const DEFAULT_FONT_PATH: &str = "fonts/FiraMono-Medium.ttf";
pub fn default_font() -> Handle<Font> {
    Handle::weak(DEFAULT_FONT_PATH.into())
}

pub fn square_mesh() -> Handle<Mesh> {
    Handle::weak(SQUARE_MESH_ID)
}

pub fn white_color_material() -> Handle<ColorMaterial> {
    Handle::weak(WHITE_COLOR_MATERIAL_ID)
}

pub fn white_standard_material() -> Handle<StandardMaterial> {
    Handle::weak(WHITE_STANDARD_MATERIAL_ID)
}
