use battle::{BattlePlugin, BattlePrefab, BattleState, EnemyKind, EnemyPrefab};
use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use board::{BoardPlugin, BoardState};
use iyes_loopless::state::CurrentState;
use prefab::spawn;
use std::{fmt::Debug, hash::Hash};
use transitions::TransitionPlugin;
use utils::UtilsPlugin;

mod battle;
mod board;
mod prefab;
mod transitions;
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
    .add_plugin(TransitionPlugin)
    .add_startup_system(setup)
    .add_system(log_states::<BoardState>)
    .add_system(log_states::<BattleState>);

    app
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    spawn(
        BattlePrefab {
            environment: asset_server.load("scenes/battles/super_basic.glb#Scene0"),
            enemy: EnemyPrefab {
                kind: EnemyKind::random(),
                max_health: 0,
                transform: default(),
            },
        },
        &mut commands,
    );
}

fn log_states<T: Hash + Eq + Clone + Sync + Send + 'static + Debug>(state: Res<CurrentState<T>>) {
    if state.is_changed() {
        println!(
            "State {} changed to {:?}",
            std::any::type_name::<T>(),
            &state.0
        );
    }
}
