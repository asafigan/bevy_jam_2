use bevy::{asset::HandleId, gltf::Gltf, prelude::*, render::view::RenderLayers};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{Display, EnumCount, EnumIter, EnumVariantNames};

use crate::prefab::{spawn, Prefab};

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_enemy_models)
            .add_system(play_idle_animation)
            .add_system(find_enemy_animations);
    }
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

fn play_idle_animation(
    enemies: Query<(Entity, &EnemyAnimations)>,
    children: Query<&Children>,
    mut animations: Query<&mut AnimationPlayer>,
) {
    fn find_animation_player(
        entity: Entity,
        children: &Query<&Children>,
        animations: &Query<&mut AnimationPlayer>,
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

    for (entity, enemy_animations) in &enemies {
        if let Some(entity) = find_animation_player(entity, &children, &animations) {
            let mut animation_player = animations.get_mut(entity).unwrap();
            if animation_player.elapsed() == 0.0 {
                animation_player
                    .play(enemy_animations.idle.clone())
                    .repeat();
            }
        }
    }
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
