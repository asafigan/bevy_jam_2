use bevy::{asset::HandleId, gltf::Gltf, prelude::*, render::view::RenderLayers};
use iyes_loopless::prelude::*;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{Display, EnumCount, EnumIter, EnumVariantNames};

use crate::{
    board::BoardState,
    prefab::{spawn, Prefab},
};

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(BattleState::PlayerTurn)
            .add_startup_system(load_enemy_models)
            .add_system(play_idle_animation)
            .add_system(find_enemy_animations)
            .add_system(build_enemy_animators)
            .add_enter_system(BattleState::PlayerTurn, start_player_turn)
            .add_enter_system(
                BoardState::Waiting,
                player_attack.run_in_state(BattleState::PlayerTurn),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(BattleState::EnemyTurn)
                    .with_system(end_enemy_turn)
                    .into(),
            );
    }
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
enum BattleState {
    Intro,
    PlayerTurn,
    EnemyTurn,
    Outro,
}

struct EnemyModels {
    models: Vec<HandleUntyped>,
}

fn load_enemy_models(asset_server: Res<AssetServer>, mut commands: Commands) {
    let models = asset_server.load_folder("models/enemies").unwrap();

    commands.insert_resource(EnemyModels { models });
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
            && animator.current_animation.as_ref() != Some(&enemy_animations.idle)
        {
            animator.current_animation = Some(enemy_animations.idle.clone());
            animation_player
                .play(enemy_animations.idle.clone())
                .repeat();
        }
    }
}

fn start_player_turn(mut commands: Commands) {
    commands.insert_resource(NextState(BoardState::Ready));
}

fn player_attack(
    mut enemies: Query<(&mut EnemyAnimator, &EnemyAnimations)>,
    mut animation_players: Query<&mut AnimationPlayer>,
    mut commands: Commands,
) {
    commands.insert_resource(NextState(BattleState::EnemyTurn));

    for (mut animator, animations) in &mut enemies {
        let mut animation_player = animation_players
            .get_mut(animator.animation_player)
            .unwrap();

        animation_player.play(animations.hurt.clone());
        animator.current_animation = Some(animations.hurt.clone());
    }
}
fn end_enemy_turn(mut commands: Commands) {
    commands.insert_resource(NextState(BattleState::PlayerTurn));
}

pub struct BattlePrefab {
    pub layers: RenderLayers,
    pub enemy: EnemyPrefab,
    pub environment: Handle<Scene>,
}

impl Prefab for BattlePrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let enemy = spawn(self.enemy.clone(), commands);

        commands
            .entity(entity)
            .insert_bundle(SceneBundle {
                scene: self.environment.clone(),
                ..default()
            })
            .insert(self.layers)
            .add_child(enemy);
    }
}

#[derive(Clone)]
pub struct EnemyPrefab {
    pub kind: EnemyKind,
}

impl Prefab for EnemyPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .insert_bundle(SceneBundle {
                scene: self.kind.scene_handle(),
                ..default()
            })
            .insert(Enemy { kind: self.kind });
    }
}

#[derive(Component)]
pub struct Enemy {
    kind: EnemyKind,
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

    pub fn scene_handle(&self) -> Handle<Scene> {
        let path = format!("models/enemies/{self}.glb#Scene0");

        Handle::weak(HandleId::AssetPathId(path.as_str().into()))
    }

    pub fn gltf_handle(&self) -> Handle<Gltf> {
        let path = format!("models/enemies/{self}.glb");

        Handle::weak(HandleId::AssetPathId(path.as_str().into()))
    }
}
