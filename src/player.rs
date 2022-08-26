use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub max_health: u32,
    pub current_health: u32,
}

impl Default for Player {
    fn default() -> Self {
        let max_health = 100;

        Self {
            max_health,
            current_health: max_health,
        }
    }
}
