//! The title screen that appears when the game starts.

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        ButtonState,
    },
    prelude::*,
    ui::Val::*,
};

use crate::{
    screens::{
        gameplay::{random_name, BotStrategy, MatchSetup, StageTheme},
        Screen,
    },
    theme::{palette::*, prelude::*},
};

#[derive(Component)]
struct SeatSummary(usize);

#[derive(Component)]
struct SeatPanel(usize);

#[derive(Component)]
struct SeatPageLabel;

#[derive(Component)]
struct NameEditHint;

#[derive(Component)]
struct ThemeLabel;

#[derive(Component)]
struct ActionButton(SeatAction);

#[derive(Resource, Default)]
struct SeatPage {
    index: usize,
}

#[derive(Resource, Default)]
struct NameEditState {
    seat: usize,
}

#[derive(Clone, Copy)]
enum SeatAction {
    ToggleHuman(usize),
    EditName(usize),
    ClearName(usize),
    CycleBot(usize),
    PrevSeatPage,
    NextSeatPage,
    RandomizeBots,
    CycleTheme,
    Start,
    Credits,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<SeatPage>();
    app.init_resource::<NameEditState>();
    app.add_systems(
        OnEnter(Screen::Title),
        (reset_seat_page, reset_name_edit, spawn_title_screen),
    );
    app.add_systems(
        Update,
        (
            refresh_seat_summaries,
            refresh_name_edit_hint,
            refresh_theme_label,
            refresh_seat_page_ui,
            handle_action_buttons,
            handle_name_typing,
        )
            .run_if(in_state(Screen::Title)),
    );
}

fn reset_seat_page(mut seat_page: ResMut<SeatPage>) {
    seat_page.index = 0;
}

fn reset_name_edit(mut name_edit: ResMut<NameEditState>) {
    name_edit.seat = 0;
}

fn spawn_title_screen(mut commands: Commands) {
    commands
        .ui_root()
        .insert(StateScoped(Screen::Title))
        .with_children(|children| {
            children
                .spawn(NodeBundle {
                    node: Node {
                        width: Percent(100.0),
                        height: Percent(100.0),
                        padding: UiRect::all(Px(18.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Px(12.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.04, 0.06, 0.11, 0.92)),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("🎲 Ludo King V1"),
                        TextFont {
                            font_size: 34.0,
                            ..default()
                        },
                        TextColor(HEADER_TEXT),
                    ));
                    panel.spawn((
                        Text::new("Build your lobby, then hit Start Game."),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(LABEL_TEXT),
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(LABEL_TEXT),
                        SeatPageLabel,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.93, 1.0)),
                        NameEditHint,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.92, 0.7)),
                        ThemeLabel,
                    ));

                    panel
                        .spawn(NodeBundle {
                            node: Node {
                                width: Percent(100.0),
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Px(8.0),
                                row_gap: Px(8.0),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|row| {
                            action_btn(row, "Previous Seats", SeatAction::PrevSeatPage);
                            action_btn(row, "Next Seats", SeatAction::NextSeatPage);
                        });

                    panel
                        .spawn(NodeBundle {
                            node: Node {
                                width: Percent(100.0),
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Px(10.0),
                                row_gap: Px(10.0),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|seat_grid| {
                            for seat in 0..4 {
                                seat_grid
                                    .spawn((
                                        NodeBundle {
                                            node: Node {
                                                flex_grow: 1.0,
                                                width: Percent(48.0),
                                                min_width: Px(320.0),
                                                flex_direction: FlexDirection::Column,
                                                padding: UiRect::all(Px(10.0)),
                                                row_gap: Px(8.0),
                                                ..default()
                                            },
                                            background_color: BackgroundColor(Color::srgba(
                                                0.16, 0.2, 0.4, 0.75,
                                            )),
                                            ..default()
                                        },
                                        SeatPanel(seat),
                                    ))
                                    .with_children(|seat_panel| {
                                        seat_panel.spawn((
                                            Text::new(""),
                                            TextFont {
                                                font_size: 20.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                            SeatSummary(seat),
                                        ));

                                        seat_panel
                                            .spawn(NodeBundle {
                                                node: Node {
                                                    width: Percent(100.0),
                                                    flex_wrap: FlexWrap::Wrap,
                                                    row_gap: Px(8.0),
                                                    column_gap: Px(8.0),
                                                    ..default()
                                                },
                                                ..default()
                                            })
                                            .with_children(|row| {
                                                action_btn(
                                                    row,
                                                    format!("Seat {}: Human/Bot", seat + 1),
                                                    SeatAction::ToggleHuman(seat),
                                                );
                                                action_btn(
                                                    row,
                                                    format!("Seat {}: Edit Name", seat + 1),
                                                    SeatAction::EditName(seat),
                                                );
                                                action_btn(
                                                    row,
                                                    format!("Seat {}: Clear Name", seat + 1),
                                                    SeatAction::ClearName(seat),
                                                );
                                                action_btn(
                                                    row,
                                                    format!("Seat {}: Bot Strategy", seat + 1),
                                                    SeatAction::CycleBot(seat),
                                                );
                                            });
                                    });
                            }
                        });

                    panel
                        .spawn(NodeBundle {
                            node: Node {
                                width: Percent(100.0),
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Px(8.0),
                                row_gap: Px(8.0),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|row| {
                            action_btn(
                                row,
                                "Randomize bot names/strategies",
                                SeatAction::RandomizeBots,
                            );
                            action_btn(row, "Stage Theme", SeatAction::CycleTheme);
                            action_btn(row, "Start Game", SeatAction::Start);
                            action_btn(row, "Credits", SeatAction::Credits);

                            #[cfg(not(target_family = "wasm"))]
                            row.button("Exit").observe(exit_app);
                        });
                });
        });
}

fn action_btn(parent: &mut ChildBuilder, text: impl Into<String>, action: SeatAction) {
    parent
        .spawn((
            ButtonBundle {
                node: Node {
                    flex_grow: 1.0,
                    min_width: Px(190.0),
                    min_height: Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Px(8.0)),
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
            ActionButton(action),
        ))
        .with_children(|children| {
            children.spawn((
                Text::new(text),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(BUTTON_TEXT),
            ));
        });
}

fn refresh_seat_summaries(
    setup: Res<MatchSetup>,
    name_edit: Res<NameEditState>,
    mut labels: Query<(&SeatSummary, &mut Text)>,
) {
    for (seat, mut text) in &mut labels {
        let s = &setup.seats[seat.0];
        let editing = if name_edit.seat == seat.0 {
            " [EDITING]"
        } else {
            ""
        };
        text.0 = format!(
            "Seat {}{} | {} | Name: {} | Bot: {}",
            seat.0 + 1,
            editing,
            if s.human { "Human" } else { "Bot" },
            s.name,
            s.bot_strategy.label()
        );
    }
}

fn refresh_name_edit_hint(
    setup: Res<MatchSetup>,
    name_edit: Res<NameEditState>,
    mut labels: Query<&mut Text, With<NameEditHint>>,
) {
    for mut text in &mut labels {
        text.0 = format!(
            "Typing Seat {} name ({}). Keyboard input, Backspace=delete, Delete=clear.",
            name_edit.seat + 1,
            setup.seats[name_edit.seat].name
        );
    }
}

fn refresh_theme_label(setup: Res<MatchSetup>, mut labels: Query<&mut Text, With<ThemeLabel>>) {
    for mut text in &mut labels {
        text.0 = format!(
            "Current stage theme: {} (tap Stage Theme to cycle)",
            setup.stage_theme.label()
        );
    }
}

fn refresh_seat_page_ui(
    seat_page: Res<SeatPage>,
    mut page_label: Query<&mut Text, With<SeatPageLabel>>,
    mut seat_panels: Query<(&SeatPanel, &mut Visibility)>,
) {
    let start = seat_page.index * 2;
    let end = (start + 2).min(4);

    for mut text in &mut page_label {
        text.0 = format!("Showing seats {}-{}", start + 1, end);
    }

    for (seat_panel, mut visibility) in &mut seat_panels {
        *visibility = if (start..end).contains(&seat_panel.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn handle_action_buttons(
    mut interaction_q: Query<(&ActionButton, &Interaction), (With<Button>, Changed<Interaction>)>,
    mut setup: ResMut<MatchSetup>,
    mut seat_page: ResMut<SeatPage>,
    mut name_edit: ResMut<NameEditState>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    for (button, interaction) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            SeatAction::ToggleHuman(i) => {
                setup.seats[i].human = !setup.seats[i].human;
                if setup.seats[i].human && setup.seats[i].name.starts_with("Bot") {
                    setup.seats[i].name = format!("Player {}", i + 1);
                }
                if !setup.seats[i].human && setup.seats[i].name.starts_with("Player") {
                    setup.seats[i].name = format!("Bot {}", i + 1);
                }
            }
            SeatAction::EditName(i) => {
                name_edit.seat = i;
            }
            SeatAction::ClearName(i) => {
                setup.seats[i].name.clear();
            }
            SeatAction::CycleBot(i) => {
                let current = setup.seats[i].bot_strategy;
                let idx = BotStrategy::ALL
                    .iter()
                    .position(|s| *s == current)
                    .unwrap_or(0);
                setup.seats[i].bot_strategy = BotStrategy::ALL[(idx + 1) % BotStrategy::ALL.len()];
            }
            SeatAction::PrevSeatPage => {
                seat_page.index = seat_page.index.saturating_sub(1);
            }
            SeatAction::NextSeatPage => {
                seat_page.index = (seat_page.index + 1).min(1);
            }
            SeatAction::RandomizeBots => {
                for seat in &mut setup.seats {
                    if !seat.human {
                        seat.name = random_name();
                        seat.bot_strategy = BotStrategy::Random;
                    }
                }
            }
            SeatAction::CycleTheme => {
                let idx = StageTheme::ALL
                    .iter()
                    .position(|theme| *theme == setup.stage_theme)
                    .unwrap_or(0);
                setup.stage_theme = StageTheme::ALL[(idx + 1) % StageTheme::ALL.len()];
            }
            SeatAction::Start => {
                for (i, seat) in setup.seats.iter_mut().enumerate() {
                    if seat.name.trim().is_empty() {
                        seat.name = if seat.human {
                            format!("Player {}", i + 1)
                        } else {
                            format!("Bot {}", i + 1)
                        };
                    }
                }
                next_screen.set(Screen::Gameplay)
            }
            SeatAction::Credits => next_screen.set(Screen::Credits),
        }
    }
}

fn handle_name_typing(
    mut keyboard_events: EventReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut setup: ResMut<MatchSetup>,
    name_edit: Res<NameEditState>,
) {
    let seat = &mut setup.seats[name_edit.seat];

    if keys.just_pressed(KeyCode::Backspace) {
        seat.name.pop();
    }
    if keys.just_pressed(KeyCode::Delete) {
        seat.name.clear();
    }

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        let Key::Character(chars) = &event.logical_key else {
            continue;
        };

        for c in chars.chars() {
            if c.is_control() {
                continue;
            }
            if seat.name.chars().count() >= 18 {
                break;
            }
            if c.is_ascii_alphanumeric() || " -_.'".contains(c) {
                seat.name.push(c);
            }
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn exit_app(_trigger: Trigger<OnPress>, mut app_exit: EventWriter<AppExit>) {
    app_exit.send(AppExit::Success);
}
