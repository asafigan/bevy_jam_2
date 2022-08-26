use bevy::{
    asset::HandleId,
    core_pipeline::clear_color::ClearColorConfig,
    ecs::system::Command,
    prelude::{
        shape::{Cube, Quad, RegularPolygon},
        *,
    },
    reflect::TypeUuid,
    render::view::RenderLayers,
    sprite::Mesh2dHandle,
};
use bevy_tweening::{lens::ColorMaterialColorLens, *};
use std::time::Duration;

use crate::prefab::Prefab;

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
    transition: Entity,
}

#[derive(Component)]
struct Transition {
    timer: Timer,
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
    pub duration: Duration,
}

const TRANSITION_LAYER: RenderLayers = RenderLayers::layer(RenderLayers::TOTAL_LAYERS as u8 - 1);

impl Prefab for FadeScreenPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let camera = commands
            .spawn_bundle(Camera2dBundle {
                camera: Camera {
                    priority: isize::MAX,
                    ..default()
                },
                camera_2d: Camera2d {
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                ..default()
            })
            .id();

        let duration = self.duration;
        commands.add(move |world: &mut World| {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

            let material_handle = materials.add(ColorMaterial {
                color: Color::NONE,
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
                    Tween::new(
                        EaseFunction::QuarticOut,
                        TweeningType::Once,
                        duration,
                        ColorMaterialColorLens {
                            start: Color::RgbaLinear {
                                red: 0.0,
                                green: 0.0,
                                blue: 0.0,
                                alpha: 0.0,
                            },
                            end: Color::RgbaLinear {
                                red: 0.0,
                                green: 0.0,
                                blue: 0.0,
                                alpha: 1.0,
                            },
                        },
                    ),
                ))
                .id();
            println!("add");
            AddChild {
                parent: entity,
                child: overlay,
            }
            .write(world);
        });

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle::default())
            .insert(Transition {
                timer: Timer::new(self.duration, false),
            })
            .insert(TRANSITION_LAYER)
            .add_child(camera);
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
