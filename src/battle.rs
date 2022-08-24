use bevy::{asset::HandleId, prelude::*, render::view::RenderLayers};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{Display, EnumCount, EnumIter, EnumVariantNames};

use crate::prefab::{spawn, Prefab};

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(load_enemy_models);
    }
}

struct EnemyModels {
    models: Vec<HandleUntyped>,
}

fn load_enemy_models(asset_server: Res<AssetServer>, mut commands: Commands) {
    let models = asset_server.load_folder("models/enemies").unwrap();

    commands.insert_resource(EnemyModels { models });
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
        commands.entity(entity).insert_bundle(SceneBundle {
            scene: self.kind.scene_handle(),
            ..default()
        });
    }
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
}
