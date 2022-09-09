use std::marker::PhantomData;

use crate::prefab::*;
use bevy::prelude::*;

pub struct FullScreen<T> {
    pub color: Color,
    pub child: T,
}

impl<T: Prefab> Prefab for FullScreen<T> {
    fn construct(self, entity: &mut EntityCommands) {
        entity
            .insert_bundle(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    align_content: AlignContent::Center,
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                color: self.color.into(),
                ..Default::default()
            })
            .with_children(|p| {
                p.spawn_prefab(self.child);
            });
    }
}

#[derive(Clone)]
pub struct TextPrefab {
    pub text: String,
    pub size: f32,
    pub color: Color,
    pub font: Handle<Font>,
}

impl Prefab for TextPrefab {
    fn construct(self, entity: &mut EntityCommands) {
        entity.insert_bundle(TextBundle {
            style: Style {
                size: Size::new(Val::Undefined, Val::Px(self.size)),
                margin: UiRect {
                    left: Val::Auto,
                    right: Val::Auto,
                    ..Default::default()
                },
                ..Default::default()
            },
            text: Text::from_section(
                self.text.clone(),
                TextStyle {
                    font: self.font.clone(),
                    font_size: self.size,
                    color: self.color,
                },
            ),
            ..Default::default()
        });
    }
}

pub struct VBox {
    pub gap: f32,
    pub children: Vec<Child>,
}

impl Prefab for VBox {
    fn construct(self, entity: &mut EntityCommands) {
        entity
            .insert_bundle(NodeBundle {
                style: Style {
                    size: Size {
                        width: Val::Percent(100.0),
                        height: Val::Auto,
                    },
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                color: Color::NONE.into(),
                ..Default::default()
            })
            .with_children(|p| {
                let mut children = self.children.into_iter();
                if let Some(child) = children.next() {
                    child.construct_inner(&mut p.spawn());
                }
                for child in children {
                    p.spawn_bundle(NodeBundle {
                        style: Style {
                            padding: UiRect {
                                top: Val::Px(self.gap),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        color: Color::NONE.into(),
                        ..Default::default()
                    })
                    .with_children(|p| {
                        child.construct_inner(&mut p.spawn());
                    });
                }
            });
    }
}

pub struct ButtonPrefab<T, C> {
    pub child: T,
    pub on_click: C,
}

impl<T, C> Prefab for ButtonPrefab<T, C>
where
    C: Clone + Send + Sync + 'static,
    T: Prefab,
{
    fn construct(self, entity: &mut EntityCommands) {
        entity
            .insert_bundle(ButtonBundle {
                color: Color::WHITE.into(),
                ..Default::default()
            })
            .insert(OnClick(self.on_click.clone()))
            .with_children(|p| {
                p.spawn_prefab(self.child);
            });
    }
}

#[derive(Clone)]
pub struct ElementEvent<T> {
    pub element: Entity,
    pub inner: T,
}

#[derive(Component)]
pub(crate) struct OnClick<T>(pub T);

impl<T: Clone + Send + Sync + 'static> OnClick<T> {
    pub fn system(
        mut events: EventWriter<T>,
        changes: Query<(&Self, &Interaction), Changed<Interaction>>,
    ) {
        for (on_click, interaction) in changes.iter() {
            if interaction == &Interaction::Clicked {
                events.send(on_click.0.clone());
            }
        }
    }
}
pub struct OnClickPlugin<T>(PhantomData<T>);

impl<T: Clone + Send + Sync + 'static> OnClickPlugin<T> {
    pub fn new() -> Self {
        OnClickPlugin(default())
    }
}

impl<T: Clone + Send + Sync + 'static> Plugin for OnClickPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<T>().add_system(OnClick::<T>::system);
    }
}
