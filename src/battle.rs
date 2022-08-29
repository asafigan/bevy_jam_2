use std::time::Duration;

use bevy::{
    asset::HandleId,
    core_pipeline::clear_color::ClearColorConfig,
    gltf::Gltf,
    prelude::*,
    render::{camera::ScalingMode, view::RenderLayers},
};
use bevy_tweening::{
    lens::TransformPositionLens, Animator, Delay, EaseFunction, Tween, TweeningType,
};
use iyes_loopless::prelude::*;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{Display, EnumCount, EnumIter, EnumVariantNames};

use crate::{
    board::{
        BoardPrefab, BoardState, Element, Match, Tile, BETWEEN_MATCH_DELAY, MATCH_START_DELAY,
    },
    cards::{CardsPrefab, CardsState},
    particles::ParticleEmitter,
    player::{Player, Spell},
    prefab::{spawn, Prefab},
    transitions::{FadeScreenPrefab, TransitionDirection, TransitionEnd},
    utils::{
        go_to, DelayedDespawn, DespawnReason, Loading, ProgressBar, ProgressBarPrefab, WorldCursor,
    },
};

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(BattleState::None)
            .add_event::<BattleCleanedUp>()
            .insert_resource(BattleResources {
                root_entities: vec![],
            })
            .add_startup_system(load_enemy_models)
            .add_system(play_idle_animation)
            .add_system(find_enemy_animations)
            .add_system(build_enemy_animators)
            .add_system(remove_unlit_materials)
            .add_system(update_enemy_health_bar)
            .add_system(update_player_health_bar)
            .add_system(stop_board.run_not_in_state(BattleState::PlayerTurn))
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(BattleState::Intro)
                    .with_system(intro)
                    .into(),
            )
            .add_enter_system(BattleState::PlayerTurn, start_player_turn)
            .add_enter_system(BattleState::PlayerTurn, go_to(CardsState::Draw))
            .add_enter_system(
                CardsState::End,
                go_to(BoardState::Ready).run_in_state(BattleState::PlayerTurn),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(BattleState::PlayerTurn)
                    .with_system(track_matches)
                    .with_system(animate_matches)
                    .with_system(start_outtro)
                    .with_system(kill_enemies)
                    .with_system(end_player_turn.run_in_state(BoardState::End))
                    .into(),
            )
            .add_enter_system(
                BoardState::End,
                player_attack
                    .chain(animate_attack)
                    .run_in_state(BattleState::PlayerTurn),
            )
            .add_enter_system(BattleState::EnemyTurn, enemies_attack)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(BattleState::EnemyTurn)
                    .with_system(start_outtro)
                    .with_system(end_enemy_turn)
                    .with_system(kill_enemies)
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(BattleState::Outtro)
                    .with_system(fade_out)
                    .into(),
            )
            .add_enter_system(BattleState::CleanedUp, send_cleanup_event);
    }
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum BattleState {
    None,
    Intro,
    PlayerTurn,
    EnemyTurn,
    Outtro,
    End,
    CleanedUp,
}

pub struct BattleResources {
    pub root_entities: Vec<Entity>,
}

impl BattleResources {
    pub fn clean_up(&mut self, commands: &mut Commands) {
        for entity in self.root_entities.drain(..) {
            commands.entity(entity).despawn_recursive()
        }
    }

    pub fn clean_up_system(mut resources: ResMut<Self>, mut commands: Commands) {
        resources.clean_up(&mut commands);
        commands.insert_resource(NextState(BattleState::CleanedUp))
    }
}

pub struct BattleCleanedUp;

fn send_cleanup_event(mut events: EventWriter<BattleCleanedUp>) {
    events.send(BattleCleanedUp);
}

fn load_enemy_models(asset_server: Res<AssetServer>, mut loading: ResMut<Loading>) {
    let models: Vec<_> = EnemyKind::gltf_paths()
        .into_iter()
        .map(|path| asset_server.load_untyped(&path))
        .collect();

    loading.assets.extend(models);
}

fn stop_board(mut commands: Commands, state: Res<CurrentState<BoardState>>) {
    if state.0 != BoardState::None {
        commands.insert_resource(NextState(BoardState::None));
    }
}

#[derive(Component)]
struct EnemyAnimations {
    idle: Handle<AnimationClip>,
    hurt: Handle<AnimationClip>,
    death: Option<Handle<AnimationClip>>,
    attack: Handle<AnimationClip>,
}

#[derive(Component)]
struct EnemyAnimator {
    animation_player: Entity,
    current_animation: Option<Handle<AnimationClip>>,
}

fn find_enemy_animations(
    enemies: Query<(Entity, &Enemy), Without<EnemyAnimations>>,
    mut commands: Commands,
    gltfs: Res<Assets<Gltf>>,
) {
    for (entity, enemy) in &enemies {
        if let Some(gltf) = gltfs.get(&enemy.kind.gltf_handle()) {
            let idle = ["Idle", "Flying"]
                .iter()
                .find_map(|name| gltf.named_animations.get(*name));
            let hurt = ["HitRecieve"]
                .iter()
                .find_map(|name| gltf.named_animations.get(*name));
            let death = ["Death"]
                .iter()
                .find_map(|name| gltf.named_animations.get(*name));
            let attack = ["Bite_Front"]
                .iter()
                .find_map(|name| gltf.named_animations.get(*name));

            if let (Some(idle), Some(hurt), Some(attack)) = (idle, hurt, attack) {
                commands.entity(entity).insert(EnemyAnimations {
                    idle: idle.clone(),
                    hurt: hurt.clone(),
                    death: death.cloned(),
                    attack: attack.clone(),
                });
            }
        }
    }
}

fn build_enemy_animators(
    enemies: Query<Entity, (With<Enemy>, Without<EnemyAnimator>)>,
    children: Query<&Children>,
    animations: Query<&AnimationPlayer>,
    mut commands: Commands,
) {
    fn find_animation_player(
        entity: Entity,
        children: &Query<&Children>,
        animations: &Query<&AnimationPlayer>,
    ) -> Option<Entity> {
        if animations.contains(entity) {
            return Some(entity);
        }

        children
            .get(entity)
            .into_iter()
            .flatten()
            .cloned()
            .find_map(|e| find_animation_player(e, children, animations))
    }

    for entity in &enemies {
        if let Some(animation_player) = find_animation_player(entity, &children, &animations) {
            commands.entity(entity).insert(EnemyAnimator {
                animation_player,
                current_animation: None,
            });
        }
    }
}

fn play_idle_animation(
    mut enemies: Query<(&EnemyAnimations, &mut EnemyAnimator)>,
    mut animation_players: Query<&mut AnimationPlayer>,
    animations: Res<Assets<AnimationClip>>,
) {
    for (enemy_animations, mut animator) in &mut enemies {
        let mut animation_player = animation_players
            .get_mut(animator.animation_player)
            .unwrap();

        // The default animation player is playing by default and never stops even though there is no animation clip.
        // The animation's elapsed time is very unlikely to be a 0.0 unless there is no animation clip.
        // Therefore, it is assumed at if elapsed time in 0.0 there in no animation playing.
        // What is needed on bevy side is a getter to the animation player's animation clip handle
        // so we can see if it is the default handle (no animation clip).
        let no_animation = !animation_player.is_changed() && animation_player.elapsed() == 0.0;

        let current_animation = animator
            .current_animation
            .as_ref()
            .and_then(|x| animations.get(x));

        // There is no way to check if animation player is looping?
        let animation_ended = current_animation
            .map(|x| animation_player.elapsed() > x.duration())
            .unwrap_or_default();

        if (no_animation || animation_ended)
            && (animator.current_animation.as_ref() != Some(&enemy_animations.idle))
        {
            animator.current_animation = Some(enemy_animations.idle.clone());
            animation_player
                .play(enemy_animations.idle.clone())
                .repeat();
        }
    }
}

fn remove_unlit_materials(mut materials: ResMut<Assets<StandardMaterial>>) {
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        let mut material = materials.get_mut(&Handle::weak(id)).unwrap();

        material.unlit = false;
    }
}

fn intro(
    mut started: Local<bool>,
    mut events: EventReader<TransitionEnd>,
    mut cameras: Query<&mut Camera, With<BattleCamera>>,
    mut commands: Commands,
) {
    if *started {
        for event in events.iter() {
            commands.entity(event.transition).despawn_recursive();

            commands.insert_resource(NextState(BattleState::PlayerTurn));

            *started = false;
        }
    } else {
        spawn(
            FadeScreenPrefab {
                direction: TransitionDirection::In,
                color: Color::BLACK,
                delay: default(),
                duration: Duration::from_secs(1),
            },
            &mut commands,
        );

        for mut camera in &mut cameras {
            camera.is_active = true;
        }

        *started = true;
    }
}

#[derive(Default)]
struct Matches(Vec<Match>);

fn start_player_turn(mut commands: Commands) {
    commands.insert_resource(Matches::default());
}

fn track_matches(mut events: EventReader<Match>, mut matches: ResMut<Matches>) {
    matches.0.extend(events.iter().cloned());
}

fn animate_matches(
    mut events: EventReader<Match>,
    mut commands: Commands,
    tiles: Query<&GlobalTransform, With<Tile>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    player: Res<Player>,
) {
    if let Some(spell) = player.active_spell.as_ref() {
        let start_delay = Duration::from_secs_f32(MATCH_START_DELAY);
        let delay_between_matches = Duration::from_secs_f32(BETWEEN_MATCH_DELAY);

        let mut delay = start_delay;
        for event in events.iter() {
            if spell.elements.contains(&event.element) {
                let material = materials.add(StandardMaterial {
                    base_color: event.element.color(),
                    base_color_texture: Some(asset_server.load("particles/star_06.png")),
                    double_sided: true,
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });

                for tile in &event.tiles {
                    let transform = tiles.get(*tile).unwrap();

                    let transform =
                        transform.compute_transform() * Transform::from_xyz(0.0, 0.0, 1.0);
                    commands
                        .spawn_bundle(SpatialBundle {
                            transform,
                            ..default()
                        })
                        .insert(BOARD_LAYER)
                        .insert(DelayedDespawn::new(delay + Duration::from_secs_f32(0.7)))
                        .insert(Animator::new(Delay::new(delay).then(Tween::new(
                            EaseFunction::QuadraticInOut,
                            TweeningType::Once,
                            Duration::from_secs_f32(0.5),
                            TransformPositionLens {
                                start: transform.translation,
                                end: Vec3::new(0.0, -2.5, 1.0),
                            },
                        ))))
                        .with_children(|c| {
                            c.spawn_bundle(SpatialBundle::default())
                                .insert(ParticleEmitter {
                                    material: material.clone(),
                                    timer: Timer::from_seconds(1.0 / 200.0, true),
                                    size_range: 0.2..0.3,
                                    velocity_range: -0.01..0.01,
                                    lifetime_range: 0.5..1.0,
                                    particles_track: true,
                                });

                            c.spawn_bundle(SpatialBundle::default())
                                .insert(ParticleEmitter {
                                    material: material.clone(),
                                    timer: Timer::from_seconds(1.0 / 100.0, true),
                                    size_range: 0.2..0.3,
                                    velocity_range: -0.01..0.01,
                                    lifetime_range: 0.2..0.5,
                                    particles_track: false,
                                });
                        });
                }

                delay += delay_between_matches;
            }
        }
    }
}

fn animate_attack(
    matches: Res<Matches>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    player: Res<Player>,
) {
    if let Some(spell) = player.active_spell.as_ref() {
        for event in matches
            .0
            .iter()
            .filter(|x| spell.elements.contains(&x.element))
        {
            let material = materials.add(StandardMaterial {
                base_color: event.element.color(),
                base_color_texture: Some(asset_server.load("particles/star_06.png")),
                double_sided: true,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            for _ in &event.tiles {
                let transform = Transform::from_xyz(0.0, -2.5, 1.0);
                commands
                    .spawn_bundle(SpatialBundle {
                        transform,
                        ..default()
                    })
                    .insert(BOARD_LAYER)
                    .insert(DelayedDespawn::from_seconds(0.7))
                    .insert(Animator::new(Tween::new(
                        EaseFunction::QuadraticInOut,
                        TweeningType::Once,
                        Duration::from_secs_f32(0.5),
                        TransformPositionLens {
                            start: transform.translation,
                            end: Vec3::new(0.0, 2.1, 1.0),
                        },
                    )))
                    .with_children(|c| {
                        c.spawn_bundle(SpatialBundle::default())
                            .insert(ParticleEmitter {
                                material: material.clone(),
                                timer: Timer::from_seconds(1.0 / 200.0, true),
                                size_range: 0.2..0.3,
                                velocity_range: -0.01..0.01,
                                lifetime_range: 0.5..1.0,
                                particles_track: true,
                            });

                        c.spawn_bundle(SpatialBundle::default())
                            .insert(ParticleEmitter {
                                material: material.clone(),
                                timer: Timer::from_seconds(1.0 / 100.0, true),
                                size_range: 0.2..0.3,
                                velocity_range: -0.01..0.01,
                                lifetime_range: 0.2..0.5,
                                particles_track: false,
                            });
                    });
            }
        }
    }
}

fn player_attack(
    mut enemies: Query<(&mut Enemy, &mut EnemyAnimator, &EnemyAnimations)>,
    mut animation_players: Query<&mut AnimationPlayer>,
    matches: Res<Matches>,
    mut player: ResMut<Player>,
) {
    let spell = player.active_spell.as_ref().unwrap();
    let matches: Vec<_> = matches.0.iter().collect();
    let damage = matches
        .iter()
        .filter(|x| spell.elements.contains(&x.element))
        .map(|x| x.tiles.len() as u32)
        .sum::<u32>()
        * spell.attack;

    if damage != 0 {
        for (mut enemy, mut animator, animations) in &mut enemies {
            enemy.current_health = enemy.current_health.saturating_sub(damage);

            let mut animation_player = animation_players
                .get_mut(animator.animation_player)
                .unwrap();

            animation_player.play(animations.hurt.clone());
            animator.current_animation = Some(animations.hurt.clone());
        }
    }

    let heal: u32 = matches
        .iter()
        .filter(|x| x.element == Element::Heal)
        .map(|x| x.tiles.len() as u32)
        .sum();

    player.current_health = player.max_health.min(player.current_health + heal * 3);
}

fn end_player_turn(mut commands: Commands, enemies: Query<(&EnemyAnimator, &EnemyAnimations)>) {
    let enemy_animations_finished = enemies.iter().all(|(animator, animations)| {
        animator.current_animation.as_ref() == Some(&animations.idle)
    });

    if enemy_animations_finished {
        commands.insert_resource(NextState(BattleState::EnemyTurn));
    }
}

fn kill_enemies(
    enemies: Query<(Entity, &Enemy, &EnemyAnimations, &EnemyAnimator)>,
    mut animation_players: Query<&mut AnimationPlayer>,
    animations: Res<Assets<AnimationClip>>,
    mut commands: Commands,
) {
    for (entity, enemy, enemy_animations, animator) in &enemies {
        if enemy.current_health == 0 {
            let mut animation_player = animation_players
                .get_mut(animator.animation_player)
                .unwrap();

            let kill_time = if let Some(animation) = &enemy_animations.death {
                animation_player.play(animation.clone());
                animations.get(animation).unwrap().duration()
            } else {
                animation_player.pause();

                0.0
            };

            // prevents enemy from returning to idle at the end of the animation
            commands
                .entity(entity)
                .remove::<EnemyAnimator>()
                .remove::<Enemy>()
                .insert(
                    DelayedDespawn::from_seconds(kill_time + 1.0)
                        .with_reason(DespawnReason::DestroyEnemy),
                );
        }
    }
}

fn start_outtro(enemies: Query<&Enemy>, mut commands: Commands) {
    if enemies.iter().count() == 0 {
        commands.insert_resource(NextState(BattleState::Outtro));
    }
}

fn update_enemy_health_bar(
    enemies: Query<&Enemy, Changed<Enemy>>,
    mut progress_bars: Query<&mut ProgressBar>,
) {
    for enemy in &enemies {
        let mut progress_bar = progress_bars.get_mut(enemy.health_bar).unwrap();

        progress_bar.percentage = enemy.current_health as f32 / enemy.max_health as f32;
    }
}

fn update_player_health_bar(
    mut progress_bars: Query<&mut ProgressBar, With<PlayerHealthBar>>,
    player: Res<Player>,
) {
    for mut health_bar in &mut progress_bars {
        if player.is_changed() || health_bar.is_added() {
            health_bar.percentage = player.current_health as f32 / player.max_health as f32;
        }
    }
}

fn enemies_attack(
    mut enemies: Query<(&Enemy, &mut EnemyAnimator, &EnemyAnimations)>,
    mut animation_players: Query<&mut AnimationPlayer>,
    mut player: ResMut<Player>,
) {
    for (enemy, mut animator, animations) in &mut enemies {
        player.current_health = player.current_health.saturating_sub(enemy.attack);

        let mut animation_player = animation_players
            .get_mut(animator.animation_player)
            .unwrap();

        animation_player.play(animations.attack.clone());
        animator.current_animation = Some(animations.attack.clone());
    }
}

fn end_enemy_turn(mut commands: Commands, enemies: Query<(&EnemyAnimator, &EnemyAnimations)>) {
    let enemy_animations_finished = enemies.iter().all(|(animator, animations)| {
        animator.current_animation.as_ref() == Some(&animations.idle)
    });

    if enemy_animations_finished {
        if enemies.iter().count() == 0 {
            commands.insert_resource(NextState(BattleState::Outtro));
        } else {
            commands.insert_resource(NextState(BattleState::PlayerTurn));
        }
    }
}

fn fade_out(
    mut started: Local<bool>,
    delays: Query<&DelayedDespawn>,
    mut events: EventReader<TransitionEnd>,
    mut resources: ResMut<BattleResources>,
    mut commands: Commands,
) {
    let waiting_for_enemy_death = delays
        .iter()
        .any(|x| x.reason() == Some(DespawnReason::DestroyEnemy));
    if !*started && !waiting_for_enemy_death {
        resources.root_entities.push(spawn(
            FadeScreenPrefab {
                delay: Duration::from_secs_f32(0.5),
                duration: Duration::from_secs(1),
                direction: TransitionDirection::Out,
                color: Color::BLACK,
            },
            &mut commands,
        ));

        *started = true;
    }
    if events.iter().count() > 0 {
        commands.insert_resource(NextState(BattleState::End));
        *started = false;
    }
}

#[derive(Component)]
struct PlayerHealthBar;

#[derive(Component)]
struct BattleCamera;

pub struct BattlePrefab {
    pub round: u32,
    pub num_rounds: u32,
    pub enemy: EnemyPrefab,
    pub environment: Handle<Scene>,
    pub spells: Vec<Spell>,
    pub font: Handle<Font>,
}

const ENVIRONMENT_LAYER: RenderLayers = RenderLayers::layer(0);
const BOARD_LAYER: RenderLayers = RenderLayers::layer(1);
const CARDS_LAYER: RenderLayers = RenderLayers::layer(2);

impl Prefab for BattlePrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let board_camera = commands
            .spawn_bundle(Camera3dBundle {
                projection: OrthographicProjection {
                    scale: 3.0,
                    scaling_mode: ScalingMode::FixedVertical(2.0),
                    ..default()
                }
                .into(),
                transform: Transform::from_translation(Vec3::Z * 10.0)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                camera_3d: Camera3d {
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                camera: Camera {
                    priority: 1,
                    is_active: false,
                    ..default()
                },
                ..default()
            })
            .insert(WorldCursor::default())
            .insert(BOARD_LAYER)
            .insert(BattleCamera)
            .id();

        let board = spawn(
            BoardPrefab {
                layers: BOARD_LAYER,
                gems: BoardPrefab::random_gems(),
                transform: Transform::from_xyz(0.0, -0.5, 0.0).with_scale(Vec3::splat(0.5)),
            },
            commands,
        );

        let health_bar = spawn(
            ProgressBarPrefab {
                starting_percentage: 1.0,
                size: [6.0, 0.3].into(),
                border: 0.1,
                transform: Transform::from_xyz(0.0, -2.9, 1.0),
                color: Color::hex(HEALTH_COLOR_HEX).unwrap(),
                ..default()
            },
            commands,
        );

        commands.entity(board).add_child(health_bar);

        commands.entity(health_bar).insert(PlayerHealthBar);

        let cards_camera = commands
            .spawn_bundle(Camera2dBundle {
                camera_2d: Camera2d {
                    clear_color: ClearColorConfig::None,
                },
                camera: Camera {
                    priority: 2,
                    is_active: false,
                    ..default()
                },
                projection: OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical(4000.0),
                    ..Camera2dBundle::default().projection
                },
                ..default()
            })
            .insert(BattleCamera)
            .insert(WorldCursor::default())
            .insert(CARDS_LAYER)
            .id();

        let cards = spawn(
            CardsPrefab {
                font: self.font.clone(),
                layer: CARDS_LAYER,
                transform: default(),
                spells: self.spells.clone(),
            },
            commands,
        );

        commands.entity(cards).with_children(|c| {
            c.spawn_bundle(Text2dBundle {
                text: Text::from_section(
                    format!("Round {} / {}", self.round, self.num_rounds),
                    TextStyle {
                        font: self.font.clone(),
                        font_size: 200.0,
                        color: Color::WHITE,
                    },
                )
                .with_alignment(TextAlignment::TOP_LEFT),
                transform: Transform::from_xyz(-1500.0, 2000.0, 0.0),
                ..default()
            });
        });

        let environment_camera = commands
            .spawn_bundle(Camera3dBundle {
                camera: Camera {
                    is_active: false,
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 5.0, 13.0)
                    .looking_at([0.0, 0.0, 5.0].into(), Vec3::Y),
                ..default()
            })
            .insert(BattleCamera)
            .insert(ENVIRONMENT_LAYER)
            .id();

        let enemy = spawn(self.enemy.clone(), commands);

        let light = commands
            .spawn_bundle(PointLightBundle {
                point_light: PointLight {
                    shadows_enabled: true,
                    range: 50.0,
                    intensity: 100000.0,
                    shadow_depth_bias: 0.001,
                    ..default()
                },
                transform: Transform::from_xyz(10.0, 10.0, 10.0).with_rotation(
                    Quat::from_rotation_x(-45_f32.to_radians())
                        * Quat::from_rotation_y(-45_f32.to_radians()),
                ),
                ..default()
            })
            .insert(BattleCamera)
            .id();

        let root = commands
            .entity(entity)
            .insert_bundle(SceneBundle {
                scene: self.environment.clone(),
                ..default()
            })
            .insert(ENVIRONMENT_LAYER)
            .add_child(environment_camera)
            .add_child(light)
            .add_child(enemy)
            .id();

        commands.insert_resource(BattleResources {
            root_entities: vec![root, board, board_camera, cards_camera, cards],
        });
    }
}

#[derive(Clone)]
pub struct EnemyPrefab {
    pub transform: Transform,
    pub kind: EnemyKind,
    pub max_health: u32,
    pub attack: u32,
}

const HEALTH_COLOR_HEX: &str = "871e16";

impl Prefab for EnemyPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let health_bar = spawn(
            ProgressBarPrefab {
                starting_percentage: 1.0,
                border: 0.1,
                size: [1.0, 0.2].into(),
                transform: self.transform * Transform::from_xyz(0.0, 0.2, 1.2),
                color: Color::hex(HEALTH_COLOR_HEX).unwrap(),
                ..default()
            },
            commands,
        );

        commands
            .entity(entity)
            .insert_bundle(SceneBundle {
                scene: self.kind.scene_handle(),
                transform: self.transform,
                ..default()
            })
            .insert(Enemy {
                kind: self.kind,
                max_health: self.max_health,
                current_health: self.max_health,
                attack: self.attack,
                health_bar,
            })
            .add_child(health_bar);
    }
}

#[derive(Component)]
pub struct Enemy {
    kind: EnemyKind,
    max_health: u32,
    current_health: u32,
    attack: u32,
    health_bar: Entity,
}

#[derive(Clone, Copy, EnumVariantNames, EnumIter, EnumCount, Display)]
pub enum EnemyKind {
    Alien,
    Bat,
    Bee,
    Cactus,
    Chicken,
    Crab,
    Cthulhu,
    Cyclops,
    Deer,
    Demon,
    Ghost,
    GreenDemon,
    Mushroom,
    Panda,
    Penguin,
    Pig,
    Skull,
    Tree,
    YellowDragon,
    Yeti,
}

impl EnemyKind {
    pub fn random() -> Self {
        let rng = fastrand::Rng::new();

        let n = rng.usize(..Self::COUNT);
        Self::iter().nth(n).unwrap()
    }

    pub fn gltf_paths() -> Vec<String> {
        Self::iter().map(|x| x.gltf_path()).collect()
    }

    pub fn scene_handle(&self) -> Handle<Scene> {
        let path = format!("models/enemies/{self}.glb#Scene0");

        Handle::weak(HandleId::AssetPathId(path.as_str().into()))
    }

    pub fn gltf_path(&self) -> String {
        format!("models/enemies/{self}.glb")
    }

    pub fn gltf_handle(&self) -> Handle<Gltf> {
        let path = self.gltf_path();

        Handle::weak(HandleId::AssetPathId(path.as_str().into()))
    }
}
