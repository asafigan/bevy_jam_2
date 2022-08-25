use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, render::view::RenderLayers};
use std::{marker::PhantomData, time::Duration};

use crate::prefab::Prefab;

pub struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TransitionEnd>()
            .add_system(update_transitions);
    }
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
            .insert(TRANSITION_LAYER)
            .id();

        commands
            .entity(entity)
            .add_child(camera)
            .insert(Transition {
                timer: Timer::new(self.duration, false),
            })
            .insert(TRANSITION_LAYER);
    }
}
