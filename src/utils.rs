use std::time::Duration;

use bevy::prelude::*;

pub struct UtilsPlugin;

impl Plugin for UtilsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnEvent>().add_system(delayed_despawn);
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
