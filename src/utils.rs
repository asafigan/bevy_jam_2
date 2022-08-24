use std::time::Duration;

use bevy::{prelude::*, render::view::RenderLayers};

pub struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnEvent>()
            .add_system(delayed_despawn)
            .add_system_to_stage(CoreStage::PostUpdate, propagate_render_layers);
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
}

pub struct DespawnEvent {
    pub entity: Entity,
    pub reason: Option<DespawnReason>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DespawnReason {
    DestroyGem,
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
