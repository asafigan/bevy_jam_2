use std::time::Duration;

use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::{
    battle::{BattleCleanedUp, BattlePrefab, BattleResources, BattleState, EnemyKind, EnemyPrefab},
    player::Player,
    prefab::spawn,
    transitions::{FadeScreenPrefab, Transition, TransitionDirection, TransitionEnd},
    ui::*,
};

pub struct MainStatePlugin;

impl Plugin for MainStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(OnClickPlugin::<Restart>::new())
            .add_loopless_state(MainState::Map)
            .insert_resource(Player::default())
            .insert_resource(Difficulty::default())
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Map)
                    .with_system(start_battle)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Battle)
                    .with_system(die)
                    .with_system(go_to_map.run_on_event::<BattleCleanedUp>())
                    .into(),
            )
            .add_enter_system(BattleState::End, BattleResources::clean_up_system)
            .add_enter_system(MainState::Death, show_death_screen)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Death)
                    .with_system(go_to_restart.run_on_event::<Restart>())
                    .into(),
            )
            .add_enter_system(MainState::Restart, fade_screen)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Restart)
                    .with_system(BattleResources::clean_up_system.run_on_event::<TransitionEnd>())
                    .with_system(reset_player.run_on_event::<TransitionEnd>())
                    .with_system(reset_difficulty.run_on_event::<TransitionEnd>())
                    .with_system(clean_up_death_screen.run_on_event::<TransitionEnd>())
                    .with_system(Transition::clean_up_system.run_on_event::<BattleCleanedUp>())
                    .with_system(go_to_map.run_on_event::<BattleCleanedUp>())
                    .into(),
            );
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MainState {
    MainMenu,
    Map,
    Battle,
    Death,
    Restart,
}

#[derive(Clone, Copy)]
struct Restart;

struct Difficulty {
    enemy_health: u32,
    enemy_attack: u32,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self {
            enemy_health: 20,
            enemy_attack: 100,
        }
    }
}

fn start_battle(
    mut difficulty: ResMut<Difficulty>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    spawn(
        BattlePrefab {
            environment: asset_server.load("scenes/battles/super_basic.glb#Scene0"),
            enemy: EnemyPrefab {
                kind: EnemyKind::random(),
                max_health: difficulty.enemy_health,
                attack: difficulty.enemy_attack,
                transform: default(),
            },
        },
        &mut commands,
    );

    difficulty.enemy_health = (difficulty.enemy_health as f32 * 1.2) as u32;
    difficulty.enemy_attack += 2;

    commands.insert_resource(NextState(MainState::Battle));
    commands.insert_resource(NextState(BattleState::Intro));
}

fn die(player: Res<Player>, mut commands: Commands) {
    if player.current_health == 0 {
        commands.insert_resource(NextState(MainState::Death));
        commands.insert_resource(NextState(BattleState::None));
    }
}

fn go_to_map(mut commands: Commands) {
    commands.insert_resource(NextState(MainState::Map))
}

#[derive(Component)]
struct DeathScreen;

fn show_death_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let death_screen = spawn(
        FullScreen {
            color: Color::Rgba {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.5,
            },
            child: ButtonPrefab {
                on_click: Restart,
                child: TextPrefab {
                    text: "Restart".into(),
                    size: 40.0,
                    color: Color::BLACK,
                    font,
                },
            },
        },
        &mut commands,
    );

    commands.entity(death_screen).insert(DeathScreen);
}

fn clean_up_death_screen(screens: Query<Entity, With<DeathScreen>>, mut commands: Commands) {
    for entity in &screens {
        commands.entity(entity).despawn_recursive();
    }
}

fn go_to_restart(mut commands: Commands) {
    commands.insert_resource(NextState(MainState::Restart));
}

fn fade_screen(mut commands: Commands) {
    spawn(
        FadeScreenPrefab {
            direction: TransitionDirection::Out,
            color: Color::BLACK,
            delay: default(),
            duration: Duration::from_secs(1),
        },
        &mut commands,
    );
}

fn reset_player(mut player: ResMut<Player>) {
    *player = default();
}

fn reset_difficulty(mut difficulty: ResMut<Difficulty>) {
    *difficulty = default();
}
