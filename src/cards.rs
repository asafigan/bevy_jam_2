use bevy::{prelude::*, render::view::RenderLayers};
use iyes_loopless::prelude::*;

use crate::{
    player::Spell,
    prefab::{spawn, Prefab},
    utils::{default_font, go_to, square_mesh, white_color_material, WorldHover},
};

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(CardsState::None)
            .add_system(put_cards_in_pile)
            .add_enter_system(CardsState::Draw, draw.chain(put_cards_in_hand))
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(CardsState::Draw)
                    .with_system(go_to(CardsState::Select))
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(CardsState::Select)
                    .with_system(select_cards)
                    //.with_system(go_to(CardsState::Merge))
                    .into(),
            )
            .add_enter_system(CardsState::Merge, merge)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(CardsState::Merge)
                    .with_system(go_to(CardsState::Discard))
                    .into(),
            )
            .add_enter_system(CardsState::Discard, discard)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(CardsState::Discard)
                    .with_system(go_to(CardsState::End))
                    .into(),
            );
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

fn put_cards_in_hand(
    hands: Query<(&Hand, &Transform), Changed<Hand>>,
    mut cards: Query<&mut Transform, Without<Hand>>,
) {
    for (hand, hand_transform) in &hands {
        let mut iter = cards.iter_many_mut(&hand.cards);
        let mut i = 0.0;
        let space = 500.0;
        let offset = (hand.cards.len() / 2) as f32 * space;
        while let Some(mut transform) = iter.fetch_next() {
            *transform = *hand_transform * Transform::from_xyz(i * space - offset, 0.0, i);
            i += 1.0;
        }
    }
}

fn put_cards_in_pile(
    piles: Query<(&Pile, &Transform), Changed<Pile>>,
    mut cards: Query<(&mut Transform, &mut Visibility), Without<Pile>>,
) {
    for (pile, pile_transform) in &piles {
        let mut iter = cards.iter_many_mut(&pile.cards);
        while let Some((mut transform, mut visibility)) = iter.fetch_next() {
            *transform = pile_transform.with_rotation(Quat::from_rotation_y(180_f32.to_radians()));
        }
    }
}

fn draw(
    mut draw_piles: Query<&mut Pile, With<DrawPile>>,
    mut hands: Query<&mut Hand>,
    mut discard_piles: Query<&mut Pile, (With<DiscardPile>, Without<DrawPile>)>,
) {
    let mut draw_pile = draw_piles.single_mut();
    let mut hand = hands.single_mut();
    let mut discard_pile = discard_piles.single_mut();

    if draw_pile.cards.len() < 5 {
        fastrand::shuffle(&mut discard_pile.cards);
        draw_pile.cards.extend(discard_pile.cards.drain(..));
    }

    hand.cards.extend(draw_pile.cards.drain(..5));
}

fn select_cards(mut hands: Query<&mut Hand>, mut cards: Query<(&WorldHover, &mut Transform)>) {
    let hover_offset = Vec3::new(0.0, 100.0, 10.0);
    for mut hand in &mut hands {
        if let Some(card) = hand.hovered_card {
            let (hover, mut transform) = cards.get_mut(card).unwrap();

            if !hover.is_cursor_in {
                hand.hovered_card = None;
                transform.translation -= hover_offset;
            }
        }

        if hand.hovered_card.is_none() {
            let hand = hand.as_mut();
            for entity in &hand.cards {
                let (hover, mut transform) = cards.get_mut(*entity).unwrap();
                if hover.is_cursor_in {
                    hand.hovered_card = Some(*entity);
                    transform.translation += hover_offset;
                    break;
                }
            }
        }
    }
}

fn merge() {}

fn discard(mut discard_piles: Query<&mut Pile, With<DiscardPile>>, mut hands: Query<&mut Hand>) {
    let mut discard_pile = discard_piles.single_mut();
    let mut hand = hands.single_mut();

    discard_pile.cards.extend(hand.cards.drain(..));
}

#[derive(Component, Default)]
struct Hand {
    cards: Vec<Entity>,
    hovered_card: Option<Entity>,
}

#[derive(Component, Default)]
struct Pile {
    cards: Vec<Entity>,
}

#[derive(Component, Default)]
struct DrawPile;

#[derive(Component, Default)]
struct DiscardPile;

pub struct CardsPrefab {
    pub layer: RenderLayers,
    pub transform: Transform,
    pub spells: Vec<Spell>,
}

impl Prefab for CardsPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mut cards: Vec<Entity> = self
            .spells
            .iter()
            .cloned()
            .map(|spell| spawn(CardPrefab { spell }, commands))
            .collect();

        fastrand::shuffle(&mut cards);

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle {
                transform: self.transform,
                ..default()
            })
            .insert(self.layer)
            .push_children(&cards)
            .with_children(|c| {
                c.spawn_bundle(SpatialBundle {
                    transform: Transform::from_xyz(0.0, -1500.0, 0.0),
                    ..default()
                })
                .insert(Hand::default());

                c.spawn_bundle(SpatialBundle {
                    transform: Transform::from_xyz(1600.0, -2000.0, 0.0),
                    ..default()
                })
                .insert(Pile::default())
                .insert(DiscardPile);

                c.spawn_bundle(SpatialBundle {
                    transform: Transform::from_xyz(-1600.0, -2000.0, 0.0),
                    ..default()
                })
                .insert(Pile { cards })
                .insert(DrawPile);
            });
    }
}

pub struct CardPrefab {
    pub spell: Spell,
}

impl Prefab for CardPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        const SCALE: f32 = 4.0;
        let style = TextStyle {
            font: default_font(),
            font_size: 40.0 * SCALE,
            color: Color::BLACK,
        };

        let alignment = TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Center,
        };

        let width = 175.0 * SCALE;
        let height = 250.0 * SCALE;

        commands
            .entity(entity)
            .insert_bundle(SpatialBundle::default())
            .insert(WorldHover::new([width, height].into()).extend_bottom_bounds(1000.0))
            .with_children(|commands| {
                commands.spawn_bundle(ColorMesh2dBundle {
                    mesh: square_mesh().into(),
                    material: white_color_material(),
                    transform: Transform::from_scale([width, height, 1.0].into()),
                    ..default()
                });

                commands.spawn_bundle(ColorMesh2dBundle {
                    mesh: square_mesh().into(),
                    material: white_color_material(),
                    transform: Transform::from_scale([width, height, 1.0].into())
                        .with_rotation(Quat::from_rotation_y(180_f32.to_radians())),
                    ..default()
                });

                commands.spawn_bundle(Text2dBundle {
                    text: Text::from_section(self.spell.name.to_string(), style.clone())
                        .with_alignment(alignment),
                    transform: Transform::from_xyz(0.0, 100.0 * SCALE, 0.01),
                    ..default()
                });

                commands.spawn_bundle(Text2dBundle {
                    text: Text::from_section(self.spell.attack.to_string(), style)
                        .with_alignment(alignment),
                    transform: Transform::from_xyz(0.0, -70.0 * SCALE, 0.01),
                    ..default()
                });
            });
    }
}
