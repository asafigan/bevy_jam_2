use std::ops::Range;

use bevy::{
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::view::RenderLayers,
};

use crate::utils::square_mesh;

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(emit_particles).add_system(move_particles);
    }
}

#[derive(Component)]
pub struct Particle {
    pub lifetime: Timer,
    pub velocity: Vec3,
}

#[derive(Component)]
pub struct ParticleEmitter {
    pub material: Handle<StandardMaterial>,
    pub timer: Timer,
    pub size_range: Range<f32>,
    pub velocity_range: Range<f32>,
    // in seconds
    pub lifetime_range: Range<f32>,
    pub particles_track: bool,
}

fn emit_particles(
    mut emitters: Query<(
        Entity,
        &mut ParticleEmitter,
        &GlobalTransform,
        Option<&RenderLayers>,
    )>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let mut rng = fastrand::Rng::new();
    for (entity, mut emitter, transform, render_layers) in &mut emitters {
        for _ in 0..(emitter.timer.tick(time.delta()).times_finished_this_tick()) {
            let lifetime = random_in_range(&emitter.lifetime_range, &mut rng);
            let size = random_in_range(&emitter.size_range, &mut rng);
            let velocity = Vec2::new(
                random_in_range(&emitter.velocity_range, &mut rng),
                random_in_range(&emitter.velocity_range, &mut rng),
            )
            .extend(0.0);

            let particle = commands
                .spawn_bundle(PbrBundle {
                    mesh: square_mesh(),
                    material: emitter.material.clone(),
                    transform: if emitter.particles_track {
                        default()
                    } else {
                        transform.compute_transform()
                    }
                    .with_scale(Vec3::splat(size)),
                    ..default()
                })
                .insert(Particle {
                    lifetime: Timer::from_seconds(lifetime, false),
                    velocity,
                })
                .insert(NotShadowCaster)
                .insert(NotShadowReceiver)
                .id();

            if emitter.particles_track {
                commands.entity(entity).add_child(particle);
            }

            if let Some(render_layers) = render_layers {
                commands.entity(particle).insert(*render_layers);
            }
        }
    }
}

fn random_in_range(range: &Range<f32>, rng: &mut fastrand::Rng) -> f32 {
    rng.f32() * (range.end - range.start) + range.start
}

fn move_particles(
    mut particles: Query<(Entity, &mut Particle, &mut Transform)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut particle, mut transform) in &mut particles {
        if particle.lifetime.tick(time.delta()).finished() {
            commands.entity(entity).despawn();
        } else {
            transform.translation += particle.velocity;
        }
    }
}
