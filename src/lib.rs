use battle::{BattlePlugin, BattlePrefab, EnemyKind, EnemyPrefab};
use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use board::BoardPlugin;
use prefab::spawn;
use utils::UtilsPlugin;

mod battle;
mod board;
mod prefab;
mod tween_untils;
mod utils;

pub fn build_app() -> App {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        present_mode: bevy::window::PresentMode::Immediate,
        ..Default::default()
    })
    .insert_resource(AmbientLight {
        brightness: 2.0,
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(TweeningPlugin)
    .add_plugin(BoardPlugin)
    .add_plugin(UtilsPlugin)
    .add_plugin(BattlePlugin)
    .add_startup_system(setup);

    app
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn(
        BattlePrefab {
            environment: asset_server.load("scenes/battles/super_basic.glb#Scene0"),
            enemy: EnemyPrefab {
                kind: EnemyKind::random(),
            },
        },
        &mut commands,
    );
}
