use battle::{BattlePlugin, BattleState};
use bevy::prelude::*;
use bevy_tweening::TweeningPlugin;
use board::{BoardPlugin, BoardState};
use cards::{CardPlugin, CardsState};
use iyes_loopless::prelude::*;
use main_state::{MainState, MainStatePlugin};
use particles::ParticlesPlugin;
use std::{fmt::Debug, hash::Hash};
use transitions::TransitionPlugin;
use utils::UtilsPlugin;

#[cfg(target_arch = "wasm32")]
mod wasm;

mod battle;
mod board;
mod cards;
mod main_state;
mod particles;
mod player;
mod prefab;
mod transitions;
mod tween_untils;
mod ui;
pub mod utils;

pub fn build_app() -> App {
    let mut app = App::new();

    #[cfg(target_os = "linux")]
    {
        app.insert_resource(WindowDescriptor {
            present_mode: bevy::window::PresentMode::Immediate,
            ..Default::default()
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        app.add_plugin(wasm::WasmPlugin);
    }

    app.insert_resource(AmbientLight {
        brightness: 2.0,
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(TweeningPlugin)
    .add_plugin(BoardPlugin)
    .add_plugin(UtilsPlugin)
    .add_plugin(CardPlugin)
    .add_plugin(BattlePlugin)
    .add_plugin(TransitionPlugin)
    .add_plugin(MainStatePlugin)
    .add_plugin(ParticlesPlugin)
    .add_system(log_states::<BoardState>)
    .add_system(log_states::<BattleState>)
    .add_system(log_states::<MainState>)
    .add_system(log_states::<CardsState>);

    app
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
