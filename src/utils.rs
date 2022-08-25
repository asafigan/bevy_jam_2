use std::time::Duration;

use bevy::{
    asset::HandleId,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{shape::RegularPolygon, *},
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
            .add_system(delayed_despawn)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_progress.before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(CoreStage::PostUpdate, propagate_render_layers);
    }
}

fn add_meshes(mut meshes: ResMut<Assets<Mesh>>) {
    let square = RegularPolygon {
        radius: f32::sqrt(0.5),
        sides: 4,
    };

    meshes.set_untracked(ProgressBarPrefab::mesh_handle(), square.into());
}

fn add_materials(mut materials: ResMut<Assets<StandardMaterial>>) {
    materials.set_untracked(
        ProgressBarPrefab::material_handle(),
        StandardMaterial {
            unlit: true,
            ..default()
        },
    );
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
                mesh: Self::mesh_handle(),
                material: Self::material_handle(),
                transform: Transform::from_rotation(Quat::from_rotation_z(45_f32.to_radians())),
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

const SQUARE_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000 - 2);
const SQUARE_MATERIAL_ID: HandleId = HandleId::new(StandardMaterial::TYPE_UUID, 10_000 - 2);

impl ProgressBarPrefab {
    fn mesh_handle() -> Handle<Mesh> {
        Handle::weak(SQUARE_MESH_ID)
    }

    fn material_handle() -> Handle<StandardMaterial> {
        Handle::weak(SQUARE_MATERIAL_ID)
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
