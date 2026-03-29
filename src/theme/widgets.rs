//! Helper traits for creating common widgets.

use bevy::{ecs::system::EntityCommands, prelude::*, ui::Val::*};

use crate::theme::{interaction::InteractionPalette, palette::*};

/// An extension trait for spawning UI widgets.
pub trait Widgets {
    /// Spawn a simple button with text.
    fn button(&mut self, text: impl Into<String>) -> EntityCommands<'_>;

    /// Spawn a simple header label. Bigger than [`Widgets::label`].
    fn header(&mut self, text: impl Into<String>) -> EntityCommands<'_>;

    /// Spawn a simple text label.
    fn label(&mut self, text: impl Into<String>) -> EntityCommands<'_>;
}

impl Widgets for Commands<'_, '_> {
    fn button(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        let mut entity = self.spawn((
            Name::new("Button"),
            ButtonBundle {
                node: Node {
                    width: Px(200.0),
                    height: Px(65.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(NODE_BACKGROUND),
                ..default()
            },
            InteractionPalette {
                none: NODE_BACKGROUND,
                hovered: BUTTON_HOVERED_BACKGROUND,
                pressed: BUTTON_PRESSED_BACKGROUND,
            },
        ));
        entity.with_children(|children| {
            children.spawn((
                Name::new("Button Text"),
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(BUTTON_TEXT),
            ));
        });

        entity
    }

    fn header(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        let mut entity = self.spawn((
            Name::new("Header"),
            NodeBundle {
                node: Node {
                    width: Px(500.0),
                    height: Px(65.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(NODE_BACKGROUND),
                ..default()
            },
        ));
        entity.with_children(|children| {
            children.spawn((
                Name::new("Header Text"),
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(HEADER_TEXT),
            ));
        });
        entity
    }

    fn label(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        let entity = self.spawn((
            Name::new("Label"),
            Text::new(text),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(LABEL_TEXT),
            Node {
                width: Px(500.0),
                ..default()
            },
        ));
        entity
    }
}

impl Widgets for ChildBuilder<'_> {
    fn button(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        let mut entity = self.spawn((
            Name::new("Button"),
            ButtonBundle {
                node: Node {
                    width: Px(200.0),
                    height: Px(65.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(NODE_BACKGROUND),
                ..default()
            },
            InteractionPalette {
                none: NODE_BACKGROUND,
                hovered: BUTTON_HOVERED_BACKGROUND,
                pressed: BUTTON_PRESSED_BACKGROUND,
            },
        ));
        entity.with_children(|children| {
            children.spawn((
                Name::new("Button Text"),
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(BUTTON_TEXT),
            ));
        });

        entity
    }

    fn header(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        let mut entity = self.spawn((
            Name::new("Header"),
            NodeBundle {
                node: Node {
                    width: Px(500.0),
                    height: Px(65.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(NODE_BACKGROUND),
                ..default()
            },
        ));
        entity.with_children(|children| {
            children.spawn((
                Name::new("Header Text"),
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(HEADER_TEXT),
            ));
        });
        entity
    }

    fn label(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        let entity = self.spawn((
            Name::new("Label"),
            Text::new(text),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(LABEL_TEXT),
            Node {
                width: Px(500.0),
                ..default()
            },
        ));
        entity
    }
}

/// An extension trait for spawning UI containers.
pub trait Containers {
    /// Spawns a root node that covers the full screen
    /// and centers its content horizontally and vertically.
    fn ui_root(&mut self) -> EntityCommands<'_>;
}

impl Containers for Commands<'_, '_> {
    fn ui_root(&mut self) -> EntityCommands<'_> {
        self.spawn((
            Name::new("UI Root"),
            NodeBundle {
                node: Node {
                    width: Percent(100.0),
                    height: Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Px(10.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
        ))
    }
}
