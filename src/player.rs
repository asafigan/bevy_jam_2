use bevy::prelude::*;

use crate::board::Element;

pub struct Player {
    pub max_health: u32,
    pub current_health: u32,
    pub spells: Vec<Spell>,
}

impl Default for Player {
    fn default() -> Self {
        let max_health = 100;

        Self {
            max_health,
            current_health: max_health,
            spells: vec![
                Spell::FIRE,
                Spell::FIRE,
                Spell::SPALSH,
                Spell::SPALSH,
                Spell::WHIP,
                Spell::WHIP,
                Spell::BOLT,
                Spell::CURSE,
            ],
        }
    }
}

#[derive(Clone)]
pub struct Spell {
    pub name: &'static str,
    pub element: Element,
    pub attack: u32,
}

impl Spell {
    const FIRE: Self = Spell {
        name: "Fire",
        element: Element::Fire,
        attack: 2,
    };

    const SPALSH: Self = Spell {
        name: "Splash",
        element: Element::Water,
        attack: 2,
    };

    const WHIP: Self = Spell {
        name: "Whip",
        element: Element::Nature,
        attack: 2,
    };

    const BOLT: Self = Spell {
        name: "Bolt",
        element: Element::Electric,
        attack: 3,
    };

    const CURSE: Self = Spell {
        name: "Curse",
        element: Element::Death,
        attack: 2,
    };
}
