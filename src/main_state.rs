use std::time::Duration;

use bevy::{asset::LoadState, prelude::*};
use iyes_loopless::prelude::*;

use crate::{
    battle::{BattleCleanedUp, BattlePrefab, BattleResources, BattleState, EnemyKind, EnemyPrefab},
    cards::CardsState,
    player::Player,
    prefab::*,
    transitions::{FadeScreenPrefab, Transition, TransitionDirection, TransitionEnd},
    ui::*,
    utils::Loading,
};

pub struct MainStatePlugin;

impl Plugin for MainStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(OnClickPlugin::<Restart>::new())
            .add_loopless_state(MainState::Load)
            .insert_resource(Player::default())
            .insert_resource(Difficulty::default())
            .add_startup_system(load_assets)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Load)
                    .with_system(loaded)
                    .into(),
            )
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
                    .with_system(go_to_restart)
                    .into(),
            )
            .add_enter_system(MainState::Win, show_win_screen)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Win)
                    .with_system(go_to_restart)
                    .into(),
            )
            .add_enter_system(MainState::Restart, fade_screen)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(MainState::Restart)
                    .with_system(clean_up_battle)
                    .with_system(reset_player.run_on_event::<TransitionEnd>())
                    .with_system(reset_difficulty.run_on_event::<TransitionEnd>())
                    .with_system(clean_up_death_screen.run_on_event::<TransitionEnd>())
                    .with_system(clean_up_win_screen.run_on_event::<TransitionEnd>())
                    .with_system(Transition::clean_up_system.run_on_event::<BattleCleanedUp>())
                    .with_system(go_to_map.run_on_event::<BattleCleanedUp>())
                    .into(),
            );
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum MainState {
    Load,
    Map,
    Battle,
    Death,
    Win,
    Restart,
}

#[derive(Clone, Copy)]
struct Restart;

struct Difficulty {
    round: u32,
    enemy_health: u32,
    enemy_attack: u32,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self {
            round: 1,
            enemy_health: 40,
            enemy_attack: 10,
        }
    }
}

fn load_assets(asset_server: Res<AssetServer>, mut loading: ResMut<Loading>) {
    loading.assets.extend([
        asset_server.load_untyped("scenes/battles/super_basic.glb"),
        asset_server.load_untyped("fonts/FiraMono-Medium.ttf"),
    ]);
}

fn loaded(asset_server: Res<AssetServer>, loading: Res<Loading>, mut commands: Commands) {
    match asset_server.get_group_load_state(loading.assets.iter().map(|x| x.id)) {
        LoadState::NotLoaded | LoadState::Loading => {}
        _ => commands.insert_resource(NextState(MainState::Map)),
    }
}

fn start_battle(
    mut difficulty: ResMut<Difficulty>,
    mut commands: Commands,
    player: Res<Player>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_prefab(BattlePrefab {
        round: difficulty.round,
        num_rounds: 8,
        environment: asset_server.load("scenes/battles/super_basic.glb#Scene0"),
        enemy: EnemyPrefab {
            kind: EnemyKind::random(),
            max_health: difficulty.enemy_health,
            attack: difficulty.enemy_attack,
            transform: default(),
        },
        spells: player.spells.clone(),
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
    });

    difficulty.enemy_health = (difficulty.enemy_health as f32 * 1.2) as u32;
    difficulty.enemy_attack += 2;

    difficulty.round += 1;

    commands.insert_resource(NextState(MainState::Battle));
    commands.insert_resource(NextState(BattleState::Intro));
}

fn die(player: Res<Player>, mut commands: Commands) {
    if player.current_health == 0 {
        commands.insert_resource(NextState(MainState::Death));
        commands.insert_resource(NextState(BattleState::None));
        commands.insert_resource(NextState(CardsState::None));
    }
}

fn go_to_map(mut commands: Commands, difficulty: Res<Difficulty>) {
    if difficulty.round > 8 {
        commands.insert_resource(NextState(MainState::Win))
    } else {
        commands.insert_resource(NextState(MainState::Map))
    }
}

#[derive(Component)]
struct DeathScreen;

fn show_death_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn_prefab(FullScreen {
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
        })
        .insert(DeathScreen);
}

fn clean_up_death_screen(screens: Query<Entity, With<DeathScreen>>, mut commands: Commands) {
    for entity in &screens {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Component)]
struct WinScreen;

fn show_win_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    commands
        .spawn_prefab(FullScreen {
            color: Color::Rgba {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },
            child: VBox {
                gap: 30.0,
                children: vec![
                    TextPrefab {
                        text: "You Won!".into(),
                        size: 80.0,
                        color: Color::WHITE,
                        font: font.clone(),
                    }
                    .into(),
                    ButtonPrefab {
                        on_click: Restart,
                        child: TextPrefab {
                            text: "Restart".into(),
                            size: 40.0,
                            color: Color::BLACK,
                            font,
                        },
                    }
                    .into(),
                ],
            },
        })
        .insert(WinScreen);

    commands
        .spawn_bundle(Camera3dBundle::default())
        .insert(WinScreen);
}

fn clean_up_win_screen(screens: Query<Entity, With<WinScreen>>, mut commands: Commands) {
    for entity in &screens {
        commands.entity(entity).despawn_recursive();
    }
}

fn go_to_restart(mut commands: Commands, mut events: EventReader<Restart>) {
    if events.iter().count() > 0 {
        commands.insert_resource(NextState(MainState::Restart));
    }
}

fn clean_up_battle(
    commands: Commands,
    battle_resources: ResMut<BattleResources>,
    mut events: EventReader<TransitionEnd>,
) {
    if events.iter().count() > 0 {
        BattleResources::clean_up_system(battle_resources, commands);
    }
}

fn fade_screen(mut commands: Commands) {
    commands.spawn_prefab(FadeScreenPrefab {
        direction: TransitionDirection::Out,
        color: Color::BLACK,
        delay: default(),
        duration: Duration::from_secs(1),
    });
}

fn reset_player(mut player: ResMut<Player>) {
    *player = default();
}

fn reset_difficulty(mut difficulty: ResMut<Difficulty>) {
    *difficulty = default();
}
