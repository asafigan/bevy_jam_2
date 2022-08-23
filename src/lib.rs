use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_tweening::TweeningPlugin;
use board::{BoardPlugin, BoardPrefab, WorldCursor};
use prefab::spawn;

mod board;
mod prefab;

pub fn build_app() -> App {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        present_mode: bevy::window::PresentMode::Immediate,
        ..Default::default()
    })
    .insert_resource(AmbientLight {
        brightness: 0.5,
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(TweeningPlugin)
    .add_plugin(BoardPlugin)
    .add_startup_system(setup);

    app
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(Camera3dBundle {
            projection: OrthographicProjection {
                scale: 3.0,
                scaling_mode: ScalingMode::FixedVertical(2.0),
                ..default()
            }
            .into(),
            transform: Transform::from_translation(Vec3::Z * 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(WorldCursor::default());

    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_rotation(
            Quat::from_rotation_x(-45_f32.to_radians())
                * Quat::from_rotation_y(-45_f32.to_radians()),
        ),
        ..default()
    });

    spawn(
        BoardPrefab {
            gems: BoardPrefab::random_gems(),
            transform: Transform::from_scale(Vec3::splat(0.5)),
        },
        &mut commands,
    );
}
