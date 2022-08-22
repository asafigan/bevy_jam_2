use bevy::prelude::*;

pub trait Prefab: Send + Sync + 'static {
    fn construct(&self, entity: Entity, commands: &mut Commands);
}

pub fn spawn<T: Prefab + Sized>(prefab: T, commands: &mut Commands) -> Entity {
    let entity = commands.spawn().id();
    prefab.construct(entity, commands);

    entity
}
pub struct Child(Box<dyn Prefab>);

impl<T> From<T> for Child
where
    T: Prefab,
{
    fn from(element: T) -> Self {
        Child(Box::new(element))
    }
}

impl Child {
    pub fn new<T: Prefab>(prefab: T) -> Self {
        prefab.into()
    }

    pub fn construct(&self, entity: Entity, commands: &mut Commands) -> Entity {
        let child = commands.spawn().id();

        self.0.construct(child, commands);

        commands.entity(entity).add_child(child);

        child
    }
}
