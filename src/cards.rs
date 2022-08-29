use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    render::view::RenderLayers,
    utils::HashSet,
};
use iyes_loopless::prelude::*;

use crate::{
    player::{Player, Spell},
    prefab::{spawn, Prefab},
    utils::{blue_color_material, go_to, square_mesh, white_color_material, WorldHover},
};

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(CardsState::None)
            .add_system(put_cards_in_pile)
            .add_system(put_cards_in_hand)
            .add_system(hover_active_card.run_not_in_state(CardsState::None))
            .add_enter_system(CardsState::Draw, draw)
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(CardsState::Draw)
                    .with_system(go_to(CardsState::Select))
                    .into(),
            )
            .add_system_set(
                ConditionSet::new()
                    .run_in_state(CardsState::Select)
                    .with_system(hover_cards.chain(select_cards).chain(start_merge))
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

#[allow(clippy::type_complexity)]
fn put_cards_in_hand(
    hands: Query<(&Hand, &Transform, &WorldHover), (Changed<Hand>, Changed<WorldHover>)>,
    mut cards: Query<&mut Transform, Without<Hand>>,
) {
    let space = 500.0;
    let hover_offset = Vec3::new(0.0, 100.0, 10.0);
    let selected_offset = Vec3::new(0.0, 200.0, 0.0);

    for (hand, hand_transform, hover) in &hands {
        let offset = (hand.cards.len() / 2) as f32 * space;
        let mut iter = cards.iter_many_mut(&hand.cards);

        let root = *hand_transform
            * if hover.is_cursor_in {
                Transform::from_xyz(0.0, 400.0, 0.0)
            } else {
                default()
            };

        let mut i = 0.0;
        while let Some(mut transform) = iter.fetch_next() {
            *transform = root * Transform::from_xyz(i * space - offset, 0.0, i + 10.0);
            i += 1.0;
        }

        if let Some(mut transform) = hand.hovered_card.and_then(|x| cards.get_mut(x).ok()) {
            transform.translation += hover_offset;
        }

        for entity in &hand.selected_cards {
            if let Ok(mut transform) = cards.get_mut(*entity) {
                transform.translation += selected_offset;
            }
        }
    }
}

fn put_cards_in_pile(
    piles: Query<(&Pile, &Transform), Changed<Pile>>,
    mut cards: Query<&mut Transform, Without<Pile>>,
) {
    for (pile, pile_transform) in &piles {
        let mut iter = cards.iter_many_mut(&pile.cards);
        while let Some(mut transform) = iter.fetch_next() {
            *transform = pile_transform.with_rotation(Quat::from_rotation_y(180_f32.to_radians()));
        }
    }
}

fn draw(
    mut draw_piles: Query<&mut Pile, With<DrawPile>>,
    mut hands: Query<(Entity, &mut Hand)>,
    mut discard_piles: Query<&mut Pile, (With<DiscardPile>, Without<DrawPile>)>,
    mut commands: Commands,
) {
    let mut draw_pile = draw_piles.single_mut();
    let (entity, mut hand) = hands.single_mut();
    let mut discard_pile = discard_piles.single_mut();

    if draw_pile.cards.len() < 5 {
        fastrand::shuffle(&mut discard_pile.cards);
        draw_pile.cards.append(&mut discard_pile.cards);
    }

    hand.cards.extend(draw_pile.cards.drain(..5));

    commands.entity(entity).despawn_descendants();
}

fn hover_cards(mut hands: Query<&mut Hand>, cards: Query<&WorldHover>) {
    for mut hand in &mut hands {
        if let Some(card) = hand.hovered_card {
            let hover = cards.get(card).unwrap();

            if !hover.is_cursor_in {
                hand.hovered_card = None;
            }
        }

        if hand.hovered_card.is_none() {
            let hand = hand.as_mut();
            for entity in &hand.cards {
                let hover = cards.get(*entity).unwrap();
                if hover.is_cursor_in {
                    hand.hovered_card = Some(*entity);
                    break;
                }
            }
        }
    }
}

fn select_cards(mut hands: Query<&mut Hand>, mut events: EventReader<MouseButtonInput>) {
    let clicked = events
        .iter()
        .any(|e| e.state == ButtonState::Pressed && e.button == MouseButton::Left);

    if clicked {
        for mut hand in &mut hands {
            if let Some(card) = hand.hovered_card {
                if !hand.selected_cards.insert(card) {
                    hand.selected_cards.remove(&card);
                }
            }
        }
    }
}

fn start_merge(hands: Query<&Hand>, mut commands: Commands) {
    for hand in &hands {
        if hand.selected_cards.len() == 2 {
            commands.insert_resource(NextState(CardsState::Merge));
            break;
        }
    }
}

fn merge(
    mut hands: Query<(Entity, &mut Hand)>,
    mut player: ResMut<Player>,
    cards: Query<&Spell>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let (entity, mut hand) = hands.single_mut();

    let mut new_spell = Spell::empty();

    for entity in hand.selected_cards.drain() {
        let spell = cards.get(entity).unwrap();

        new_spell.attack += spell.attack;
        new_spell.name = (new_spell.name.to_string() + " " + spell.name.as_ref()).into();
        match &mut new_spell.elements {
            std::borrow::Cow::Borrowed(_) => todo!(),
            std::borrow::Cow::Owned(vec) => vec.extend_from_slice(&spell.elements),
        }
    }

    let card = spawn(
        CardPrefab {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            spell: new_spell.clone(),
        },
        &mut commands,
    );

    commands.entity(card).insert(ActiveCard);

    player.active_spell = Some(new_spell);
    commands.entity(entity).add_child(card);
}

#[derive(Component)]
struct ActiveCard;

fn hover_active_card(mut cards: Query<(&mut Transform, &WorldHover), With<ActiveCard>>) {
    for (mut transform, hover) in &mut cards {
        *transform = if hover.is_cursor_in {
            Transform::from_xyz(0.0, 500.0, 0.0)
        } else {
            default()
        };
    }
}

fn discard(mut discard_piles: Query<&mut Pile, With<DiscardPile>>, mut hands: Query<&mut Hand>) {
    let mut discard_pile = discard_piles.single_mut();
    let mut hand = hands.single_mut();

    discard_pile.cards.append(&mut hand.cards);

    hand.hovered_card = None;
}

#[derive(Component, Default)]
struct Hand {
    cards: Vec<Entity>,
    hovered_card: Option<Entity>,
    selected_cards: HashSet<Entity>,
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
    pub font: Handle<Font>,
}

impl Prefab for CardsPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        let mut cards: Vec<Entity> = self
            .spells
            .iter()
            .cloned()
            .map(|spell| {
                spawn(
                    CardPrefab {
                        spell,
                        font: self.font.clone(),
                    },
                    commands,
                )
            })
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
                    transform: Transform::from_xyz(0.0, -1900.0, 20.0),
                    ..default()
                })
                .insert(WorldHover::new([4000.0, 2500.0].into()))
                .insert(Hand::default());

                c.spawn_bundle(SpatialBundle {
                    transform: Transform::from_xyz(1600.0, -2000.0, 15.0),
                    ..default()
                })
                .insert(Pile::default())
                .insert(DiscardPile);

                c.spawn_bundle(SpatialBundle {
                    transform: Transform::from_xyz(-1600.0, -2000.0, 15.0),
                    ..default()
                })
                .insert(Pile { cards })
                .insert(DrawPile);
            });
    }
}

pub struct CardPrefab {
    pub font: Handle<Font>,
    pub spell: Spell,
}

impl Prefab for CardPrefab {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        const SCALE: f32 = 4.0;
        let style = TextStyle {
            font: self.font.clone(),
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
            .insert(self.spell.clone())
            .with_children(|commands| {
                commands.spawn_bundle(ColorMesh2dBundle {
                    mesh: square_mesh().into(),
                    material: white_color_material(),
                    transform: Transform::from_scale([width, height, 1.0].into()),
                    ..default()
                });

                commands.spawn_bundle(ColorMesh2dBundle {
                    mesh: square_mesh().into(),
                    material: blue_color_material(),
                    transform: Transform::from_xyz(0.0, 0.0, -1.0)
                        .with_scale([width, height, 1.0].into())
                        .with_rotation(Quat::from_rotation_y(180_f32.to_radians())),
                    ..default()
                });

                for element in self.spell.elements.as_ref() {
                    commands.spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            color: element.color() * Vec4::new(1.0, 1.0, 1.0, 0.5),
                            custom_size: Some([400.0, 400.0].into()),
                            ..default()
                        },
                        texture: element.icon_handle(),
                        transform: Transform::from_xyz(0.0, 0.0, 1.0),
                        ..default()
                    });
                }

                commands.spawn_bundle(Text2dBundle {
                    text: Text::from_section(self.spell.name.to_string(), style.clone())
                        .with_alignment(alignment),
                    transform: Transform::from_xyz(0.0, 100.0 * SCALE, 2.0),
                    ..default()
                });

                commands.spawn_bundle(Text2dBundle {
                    text: Text::from_section(self.spell.attack.to_string(), style)
                        .with_alignment(alignment),
                    transform: Transform::from_xyz(0.0, -70.0 * SCALE, 2.0),
                    ..default()
                });
            });
    }
}
