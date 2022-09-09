use bevy::{
    asset::HandleId,
    core_pipeline::clear_color::ClearColorConfig,
    prelude::{shape::Quad, *},
    reflect::TypeUuid,
    render::view::RenderLayers,
};
use bevy_tweening::{lens::ColorMaterialColorLens, *};
use std::time::Duration;

use crate::prefab::*;

pub struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TransitionEnd>()
            .add_startup_system(add_meshes)
            .add_startup_system(add_materials)
            .add_system(update_transitions);
    }
}

fn add_meshes(mut meshes: ResMut<Assets<Mesh>>) {
    let square = Quad {
        size: Vec2::splat(1.0),
        ..Default::default()
    };

    meshes.set_untracked(FadeScreenPrefab::mesh_handle(), square.into());
}

fn add_materials(mut materials: ResMut<Assets<ColorMaterial>>) {
    materials.set_untracked(
        FadeScreenPrefab::material_handle(),
        ColorMaterial {
            color: Color::NONE,
            ..default()
        },
    );
}

pub struct TransitionEnd {
    pub transition: Entity,
}

#[derive(Component)]
pub struct Transition {
    timer: Timer,
}

impl Transition {
    pub fn clean_up_system(transitions: Query<Entity, With<Transition>>, mut commands: Commands) {
        for entity in &transitions {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TransitionDirection {
    In,
    Out,
}

fn update_transitions(
    mut transitions: Query<(Entity, &mut Transition)>,
    mut events: EventWriter<TransitionEnd>,
    time: Res<Time>,
) {
    for (entity, mut transition) in &mut transitions {
        if transition.timer.tick(time.delta()).just_finished() {
            events.send(TransitionEnd { transition: entity });
        }
    }
}

pub struct FadeScreenPrefab {
    pub direction: TransitionDirection,
    pub color: Color,
    pub delay: Duration,
    pub duration: Duration,
}

const TRANSITION_LAYER: RenderLayers = RenderLayers::layer(RenderLayers::TOTAL_LAYERS as u8 - 1);

impl Prefab for FadeScreenPrefab {
    fn construct(self, entity: &mut EntityCommands) {
        let id = entity.id();

        entity
            .insert_bundle(SpatialBundle::default())
            .insert(Transition {
                timer: Timer::new(self.duration + self.delay, false),
            })
            .insert(TRANSITION_LAYER)
            .with_children(|p| {
                p.spawn_bundle(Camera2dBundle {
                    camera: Camera {
                        priority: isize::MAX,
                        ..default()
                    },
                    camera_2d: Camera2d {
                        clear_color: ClearColorConfig::None,
                    },
                    ..default()
                })
                .insert(UiCameraConfig { show_ui: false });
            });

        entity.commands().add(move |world: &mut World| {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

            let (start, end) = match self.direction {
                TransitionDirection::In => (self.color, Color::NONE),
                TransitionDirection::Out => (Color::NONE, self.color),
            };

            let material_handle = materials.add(ColorMaterial {
                color: start,
                ..default()
            });

            let overlay = world
                .spawn()
                .insert_bundle(ColorMesh2dBundle {
                    mesh: FadeScreenPrefab::mesh_handle().into(),
                    material: material_handle.clone(),
                    transform: Transform::from_scale(Vec3::splat(100000.0)),
                    ..default()
                })
                .insert(AssetAnimator::new(
                    material_handle,
                    Delay::new(self.delay).then(Tween::new(
                        EaseFunction::QuarticOut,
                        TweeningType::Once,
                        self.duration,
                        ColorMaterialColorLens { start, end },
                    )),
                ))
                .id();

            world.entity_mut(id).push_children(&[overlay]);
        });
    }
}

const FADE_TRANSITION_MESH_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 10_000 - 3);
const FADE_TRANSITION_MATERIAL_ID: HandleId = HandleId::new(ColorMaterial::TYPE_UUID, 10_000 - 3);

impl FadeScreenPrefab {
    fn mesh_handle() -> Handle<Mesh> {
        Handle::weak(FADE_TRANSITION_MESH_ID)
    }

    fn material_handle() -> Handle<ColorMaterial> {
        Handle::weak(FADE_TRANSITION_MATERIAL_ID)
    }
}
