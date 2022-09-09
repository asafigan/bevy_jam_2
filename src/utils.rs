use std::{hash::Hash, time::Duration};

use bevy::{
    asset::HandleId,
    ecs::{query::QueryEntityError, system::AsSystemLabel},
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{shape::Quad, *},
    reflect::TypeUuid,
    render::{
        camera::RenderTarget,
        view::{RenderLayers, VisibleEntities},
    },
    transform::TransformSystem,
};
use iyes_loopless::state::NextState;

use crate::prefab::*;

pub struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnEvent>()
            .add_event::<WorldCursorEvent>()
            .init_resource::<Loading>()
            .add_startup_system(add_meshes)
            .add_startup_system(add_materials)
            .add_stage_before(
                CoreStage::PostUpdate,
                "delayed_despawn",
                SystemStage::parallel(),
            )
            .add_system_to_stage("delayed_despawn", delayed_despawn)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_progress.before(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                "delayed_despawn",
                propagate_render_layers.before(delayed_despawn),
            )
            .add_system_to_stage(CoreStage::PreUpdate, update_world_cursors)
            .add_system_to_stage(
                CoreStage::PreUpdate,
                track_world_hover.after(update_world_cursors.as_system_label()),
            );
    }
}

fn add_meshes(mut meshes: ResMut<Assets<Mesh>>) {
    let square: Mesh = Quad {
        size: Vec2::splat(1.0),
        ..default()
    }
    .into();

    meshes.set_untracked(square_mesh(), square);
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

    color_materials.set_untracked(
        blue_color_material(),
        ColorMaterial {
            color: Color::BLUE,
            ..default()
        },
    );
}

#[derive(Default)]
pub struct Loading {
    pub assets: Vec<HandleUntyped>,
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
        let layer = *layers.get(root).unwrap();

        let mut children: Vec<Entity> = children_query
            .get(root)
            .into_iter()
            .flatten()
            .cloned()
            .collect();

        while !children.is_empty() {
            for child in std::mem::take(&mut children) {
                match layers.get_mut(child) {
                    Ok(mut child_layer) => {
                        if *child_layer != layer {
                            *child_layer = layer;
                        }
                    }
                    Err(QueryEntityError::QueryDoesNotMatch(entity)) => {
                        commands.entity(entity).insert(layer);
                    }
                    _ => {}
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

#[derive(Default, Clone, Copy)]
pub enum ProgressBarPosition {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Default, Clone)]
pub struct ProgressBarPrefab {
    pub starting_percentage: f32,
    pub size: Vec2,
    pub border: f32,
    pub color: Color,
    pub border_color: Color,
    pub background_color: Color,
    pub position: ProgressBarPosition,
    pub transform: Transform,
}

impl Prefab for ProgressBarPrefab {
    fn construct(self, entity: &mut EntityCommands) {
        let id = entity.id();
        entity.commands().add(move |world: &mut World| {
            let (progress_color, background_color, border_color) =
                world.resource_scope(|_, mut materials: Mut<Assets<StandardMaterial>>| {
                    (
                        materials.add(StandardMaterial {
                            base_color: self.color,
                            unlit: true,
                            alpha_mode: if self.color.a() < 1.0 {
                                AlphaMode::Blend
                            } else {
                                default()
                            },
                            ..default()
                        }),
                        materials.add(StandardMaterial {
                            base_color: self.background_color,
                            unlit: true,
                            alpha_mode: if self.background_color.a() < 1.0 {
                                AlphaMode::Blend
                            } else {
                                default()
                            },
                            ..default()
                        }),
                        materials.add(StandardMaterial {
                            base_color: self.border_color,
                            unlit: true,
                            alpha_mode: if self.border_color.a() < 1.0 {
                                AlphaMode::Blend
                            } else {
                                default()
                            },
                            ..default()
                        }),
                    )
                });

            let mesh = world
                .spawn()
                .insert_bundle(PbrBundle {
                    mesh: square_mesh(),
                    material: progress_color,
                    transform: Transform::from_translation(match self.position {
                        ProgressBarPosition::Left => Vec3::X / 2.0,
                        ProgressBarPosition::Center => default(),
                        ProgressBarPosition::Right => -Vec3::X / 2.0,
                    }),
                    ..default()
                })
                .insert(NotShadowCaster)
                .insert(NotShadowReceiver)
                .id();

            let progress = world
                .spawn()
                .insert_bundle(SpatialBundle {
                    transform: Transform::from_translation(match self.position {
                        ProgressBarPosition::Left => -Vec3::X / 2.0,
                        ProgressBarPosition::Center => default(),
                        ProgressBarPosition::Right => Vec3::X / 2.0,
                    }),
                    ..default()
                })
                .push_children(&[mesh])
                .id();

            let background = world
                .spawn()
                .insert_bundle(PbrBundle {
                    mesh: square_mesh(),
                    material: background_color,
                    transform: Transform::from_translation(-Vec3::Z * 0.001),
                    ..default()
                })
                .insert(NotShadowCaster)
                .insert(NotShadowReceiver)
                .id();

            let inner = world
                .spawn()
                .insert_bundle(SpatialBundle {
                    transform: Transform::from_scale(
                        ((self.size - self.border).max(Vec2::ZERO)).extend(1.0),
                    ),
                    ..default()
                })
                .push_children(&[progress, background])
                .id();

            let border = world
                .spawn()
                .insert_bundle(PbrBundle {
                    mesh: square_mesh(),
                    material: border_color,
                    transform: Transform::from_scale(self.size.extend(1.0))
                        .with_translation(-Vec3::Z * 0.002),
                    ..default()
                })
                .insert(NotShadowCaster)
                .insert(NotShadowReceiver)
                .id();

            world
                .entity_mut(id)
                .insert_bundle(SpatialBundle {
                    transform: self.transform,
                    ..default()
                })
                .insert(ProgressBar {
                    percentage: self.starting_percentage,
                    progress,
                })
                .push_children(&[inner, border]);
        });
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
const BLUE_COLOR_MATERIAL_ID: HandleId = HandleId::new(ColorMaterial::TYPE_UUID, 10_000 - 30);

pub fn square_mesh() -> Handle<Mesh> {
    Handle::weak(SQUARE_MESH_ID)
}

pub fn white_color_material() -> Handle<ColorMaterial> {
    Handle::weak(WHITE_COLOR_MATERIAL_ID)
}

pub fn blue_color_material() -> Handle<ColorMaterial> {
    Handle::weak(BLUE_COLOR_MATERIAL_ID)
}

pub fn white_standard_material() -> Handle<StandardMaterial> {
    Handle::weak(WHITE_STANDARD_MATERIAL_ID)
}

pub fn go_to<T: Clone + Eq + Hash + Send + Sync + 'static>(state: T) -> impl Fn(Commands) {
    move |mut commands| {
        commands.insert_resource(NextState(state.clone()));
    }
}

#[derive(Component, Default)]
pub struct WorldCursor {
    pub position: Option<Vec2>,
}

fn update_world_cursors(
    windows: Res<Windows>,
    mut cameras: Query<(&Camera, &GlobalTransform, &mut WorldCursor)>,
) {
    for (camera, camera_transform, mut cursor) in &mut cameras {
        cursor.position = if let RenderTarget::Window(id) = camera.target {
            windows.get(id).and_then(|window| {
                let window_size = Vec2::new(window.width(), window.height());
                let cursor_position = window.cursor_position()?;

                // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
                let ndc = (cursor_position / window_size) * 2.0 - Vec2::ONE;

                // matrix for undoing the projection and camera transform
                let ndc_to_world =
                    camera_transform.compute_matrix() * camera.projection_matrix().inverse();

                // use it to convert ndc to world-space coordinates
                let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

                // reduce it to a 2D value
                Some(world_pos.truncate())
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldCursorEvent {
    pub entity: Entity,
    pub info: WorldCursorEventInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldCursorEventInfo {
    Entered,
    Exited,
}

#[derive(Component)]
pub struct WorldHover {
    pub bounds: Vec2,
    pub offset: Vec2,
    pub is_cursor_in: bool,
    pub cursors_in_bounds: Vec<Entity>,
    pub check_visibility_of: Option<Entity>,
}

impl WorldHover {
    pub fn new(bounds: Vec2) -> Self {
        Self {
            bounds,
            offset: -bounds / 2.0,
            is_cursor_in: false,
            cursors_in_bounds: default(),
            check_visibility_of: None,
        }
    }

    pub fn extend_bottom_bounds(mut self, by: f32) -> Self {
        self.bounds.y += by;
        self.offset.y -= by;
        self
    }

    pub fn with_check_visibility_of(self, entity: Entity) -> Self {
        Self {
            check_visibility_of: Some(entity),
            ..self
        }
    }
}

fn track_world_hover(
    mut hoverable: Query<(Entity, &mut WorldHover, &GlobalTransform)>,
    mut events: EventWriter<WorldCursorEvent>,
    cursors: Query<(Entity, &WorldCursor, &VisibleEntities)>,
) {
    for (entity, mut hoverable, transform) in &mut hoverable {
        let check_visibility_of = hoverable.check_visibility_of.unwrap_or(entity);

        hoverable.cursors_in_bounds = cursors
            .iter()
            .filter(|(_, _, entities)| entities.entities.contains(&check_visibility_of))
            .filter_map(|(entity, cursor, _)| cursor.position.map(|x| (entity, x)))
            .filter(|(_, position)| {
                let matrix = transform.compute_matrix().inverse();
                let position = matrix.transform_point3(position.extend(0.0)).truncate();

                let [max_x, max_y] = (hoverable.bounds + hoverable.offset).to_array();
                let [min_x, min_y] = hoverable.offset.to_array();

                position.x < max_x && position.x > min_x && position.y < max_y && position.y > min_y
            })
            .map(|(x, _)| x)
            .collect();

        let is_cursor_in = !hoverable.cursors_in_bounds.is_empty();

        if hoverable.is_cursor_in != is_cursor_in {
            hoverable.is_cursor_in = is_cursor_in;
            events.send(WorldCursorEvent {
                entity,
                info: if is_cursor_in {
                    WorldCursorEventInfo::Entered
                } else {
                    WorldCursorEventInfo::Exited
                },
            });
        }
    }
}
