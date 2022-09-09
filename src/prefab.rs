use bevy::prelude::*;

pub use bevy::ecs::system::EntityCommands;

pub trait Prefab: Send + Sync + 'static {
    fn construct(self, entity: &mut EntityCommands);
}

pub trait SpawnPrefabExt<'w, 's> {
    fn spawn_prefab<'a>(&'a mut self, prefab: impl Prefab) -> EntityCommands<'w, 's, 'a>;
}

impl<'w, 's> SpawnPrefabExt<'w, 's> for Commands<'w, 's> {
    fn spawn_prefab<'a>(&'a mut self, prefab: impl Prefab) -> EntityCommands<'w, 's, 'a> {
        let mut entity = self.spawn();
        prefab.construct(&mut entity);
        entity
    }
}

impl<'w, 's, 'b> SpawnPrefabExt<'w, 's> for ChildBuilder<'w, 's, 'b> {
    fn spawn_prefab<'a>(&'a mut self, prefab: impl Prefab) -> EntityCommands<'w, 's, 'a> {
        let mut entity = self.spawn();
        prefab.construct(&mut entity);
        entity
    }
}

pub trait ConstructPrefabExt<'w, 's, 'a> {
    fn construct_prefab(&mut self, prefab: impl Prefab) -> &mut EntityCommands<'w, 's, 'a>;
}

impl<'w, 's, 'a> ConstructPrefabExt<'w, 's, 'a> for EntityCommands<'w, 's, 'a> {
    fn construct_prefab(&mut self, prefab: impl Prefab) -> &mut EntityCommands<'w, 's, 'a> {
        prefab.construct(self);
        self
    }
}

pub struct Child(Box<dyn ChildPrefab>);

impl<T> From<T> for Child
where
    T: Prefab,
{
    fn from(element: T) -> Self {
        Child(Box::new(InnerChild(Some(element))))
    }
}

impl Child {
    pub fn construct_inner(mut self, entity: &mut EntityCommands) {
        // This will not panic because Child is moved so it can only be called once.
        // Also, it is always constructed with a valid inner child.
        self.0.construct_once(entity);
    }
}

trait ChildPrefab: Send + Sync + 'static {
    fn construct_once(&mut self, entity: &mut EntityCommands);
}

pub struct InnerChild<T>(Option<T>);

// This implementation can only be called once
impl<T> ChildPrefab for InnerChild<T>
where
    T: Prefab,
{
    fn construct_once(&mut self, entity: &mut EntityCommands) {
        self.0.take().unwrap().construct(entity)
    }
}
