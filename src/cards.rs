use bevy::{prelude::*, render::view::RenderLayers};
use iyes_loopless::prelude::*;

use crate::{
    player::Spell,
    prefab::{spawn, Prefab},
    utils::{default_font, square_mesh, white_color_material},
};

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(CardsState::None)
            .add_system(debug_text);
    }
}

fn debug_text(text2d: Query<(&Text, &ComputedVisibility)>, fonts: Res<Assets<Font>>) {
    for text in &text2d {
        dbg!(text);
    }

    for x in fonts.iter() {
        dbg!(x);
    }
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum CardsState {
    None,
    Draw,
    Select,
    Merge,
    Discard,
    End,
}

#[derive(Component, Default)]
struct Hand {
    cards: Vec<Entity>,
}

#[derive(Component, Default)]
struct DrawPile {
    cards: Vec<Entity>,
}

#[derive(Component, Default)]
struct DiscardPile {
    cards: Vec<Entity>,
}

pub struct CardsPrefab {
    pub layer: RenderLayers,
    pub transform: Transform,
    pub spells: Vec<Spell>,
}

impl Prefab for CardsPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let cards: Vec<Entity> = self
            .spells
            .iter()
            .cloned()
            .map(|spell| spawn(CardPrefab { spell }, commands))
            .collect();

        for card in &cards {
            commands
                .entity(*card)
                .insert(Visibility { is_visible: false });
        }

        commands.entity(cards[0]).insert(Visibility::default());

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .insert(self.layer)
            .push_children(&cards)
            .with_children(|c| {
                c.spawn_bundle(SpatialBundle::default())
                    .insert(Hand::default());
                c.spawn_bundle(SpatialBundle::default())
                    .insert(DrawPile { cards });
                c.spawn_bundle(SpatialBundle::default())
                    .insert(DiscardPile::default());
            });
    }
}

pub struct CardPrefab {
    pub spell: Spell,
}

impl Prefab for CardPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let style = TextStyle {
            font: default_font(),
            font_size: 40.0,
            color: Color::BLACK,
        };

        let alignment = TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Center,
        };

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle::default())
            .with_children(|commands| {
                commands.spawn_bundle(ColorMesh2dBundle {
                    mesh: square_mesh().into(),
                    material: white_color_material(),
                    transform: Transform::from_scale([175.0, 250.0, 1.0].into()),
                    ..default()
                });

                commands.spawn_bundle(Text2dBundle {
                    text: Text::from_section(self.spell.name.to_string(), style.clone())
                        .with_alignment(alignment),
                    transform: Transform::from_xyz(0.0, 100.0, 0.01),
                    ..default()
                });

                commands.spawn_bundle(Text2dBundle {
                    text: Text::from_section(self.spell.attack.to_string(), style)
                        .with_alignment(alignment),
                    transform: Transform::from_xyz(0.0, -70.0, 0.01),
                    ..default()
                });
            });
    }
}
