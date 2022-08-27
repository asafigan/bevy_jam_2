use std::marker::PhantomData;

use crate::prefab::*;
use bevy::prelude::*;

pub struct FullScreen {
    pub color: Color,
    pub child: Child,
}

impl Prefab for FullScreen {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        commands.entity(entity).insert_bundle(NodeBundle {
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
        });

        self.child.construct(entity, commands);
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
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        commands.entity(entity).insert_bundle(TextBundle {
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
    pub children: Vec<Child>,
}

impl Prefab for VBox {
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        commands.entity(entity).insert_bundle(NodeBundle {
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
        });

        for child in &self.children {
            child.construct(entity, commands);
        }
    }
}

pub struct ButtonPrefab<T> {
    pub child: Child,
    pub on_click: T,
}

impl<T> Prefab for ButtonPrefab<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn construct(&self, entity: Entity, commands: &mut Commands) {
        commands
            .entity(entity)
            .insert_bundle(ButtonBundle {
                color: Color::WHITE.into(),
                ..Default::default()
            })
            .insert(OnClick(self.on_click.clone()));

        self.child.construct(entity, commands);
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
