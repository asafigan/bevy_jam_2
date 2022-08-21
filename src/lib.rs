use bevy::prelude::*;
use board::BoardPrefab;
use prefab::spawn;

mod board;
mod prefab;

pub fn build_app() -> App {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugin(board::BoardPlugin)
        .add_startup_system(setup);

    app
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_translation(Vec3::Z * 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

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
            transform: Transform::from_scale(Vec3::splat(0.4)),
        },
        &mut commands,
    );
}
