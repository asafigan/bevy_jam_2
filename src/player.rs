use crate::board::Element;
use bevy::prelude::*;
use std::borrow::Cow;

pub struct Player {
    pub max_health: u32,
    pub current_health: u32,
    pub spells: Vec<Spell>,
    pub active_spell: Option<Spell>,
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
            active_spell: None,
        }
    }
}

#[derive(Clone, Component)]
pub struct Spell {
    pub name: Cow<'static, str>,
    pub elements: Cow<'static, [Element]>,
    pub attack: u32,
}

impl Spell {
    const FIRE: Self = Spell {
        name: Cow::Borrowed("Fire"),
        elements: Cow::Borrowed(&[Element::Fire]),
        attack: 2,
    };

    const SPALSH: Self = Spell {
        name: Cow::Borrowed("Splash"),
        elements: Cow::Borrowed(&[Element::Water]),
        attack: 2,
    };

    const WHIP: Self = Spell {
        name: Cow::Borrowed("Whip"),
        elements: Cow::Borrowed(&[Element::Nature]),
        attack: 2,
    };

    const BOLT: Self = Spell {
        name: Cow::Borrowed("Bolt"),
        elements: Cow::Borrowed(&[Element::Electric]),
        attack: 3,
    };

    const CURSE: Self = Spell {
        name: Cow::Borrowed("Curse"),
        elements: Cow::Borrowed(&[Element::Death]),
        attack: 3,
    };

    pub fn empty() -> Spell {
        Spell {
            name: Cow::Borrowed(""),
            elements: default(),
            attack: 0,
        }
    }
}
