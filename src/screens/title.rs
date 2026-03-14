//! The title screen that appears when the game starts.

use bevy::{prelude::*, ui::Val::*};

use crate::{
    screens::{
        gameplay::{random_name, BotStrategy, MatchSetup},
        Screen,
    },
    theme::{palette::*, prelude::*},
};

#[derive(Component)]
struct SeatSummary(usize);

#[derive(Component)]
struct ActionButton(SeatAction);

#[derive(Clone, Copy)]
enum SeatAction {
    ToggleHuman(usize),
    CycleName(usize),
    CycleBot(usize),
    RandomizeBots,
    Start,
    Credits,
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Title), spawn_title_screen);
    app.add_systems(
        Update,
        (refresh_seat_summaries, handle_action_buttons).run_if(in_state(Screen::Title)),
    );
}

fn spawn_title_screen(mut commands: Commands) {
    commands
        .ui_root()
        .insert(StateScoped(Screen::Title))
        .with_children(|children| {
            children
                .spawn(NodeBundle {
                    style: Style {
                        width: Percent(96.0),
                        max_width: Px(1100.0),
                        max_height: Percent(92.0),
                        padding: UiRect::all(Px(18.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Px(12.0),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.04, 0.06, 0.11, 0.92)),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn(TextBundle::from_section(
                        "🎲 Ludo King V1",
                        TextStyle {
                            font_size: 34.0,
                            color: HEADER_TEXT,
                            ..default()
                        },
                    ));
                    panel.spawn(TextBundle::from_section(
                        "Build your lobby, then hit Start Game.",
                        TextStyle {
                            font_size: 18.0,
                            color: LABEL_TEXT,
                            ..default()
                        },
                    ));

                    panel
                        .spawn(NodeBundle {
                            style: Style {
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
                                    .spawn(NodeBundle {
                                        style: Style {
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
                                    })
                                    .with_children(|seat_panel| {
                                        seat_panel.spawn((
                                            TextBundle::from_section(
                                                "",
                                                TextStyle {
                                                    font_size: 20.0,
                                                    color: Color::WHITE,
                                                    ..default()
                                                },
                                            ),
                                            SeatSummary(seat),
                                        ));

                                        seat_panel
                                            .spawn(NodeBundle {
                                                style: Style {
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
                                                    format!("Seat {}: Change Name", seat + 1),
                                                    SeatAction::CycleName(seat),
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
                            style: Style {
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
                style: Style {
                    flex_grow: 1.0,
                    min_width: Px(190.0),
                    min_height: Px(40.0),
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
            children.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font_size: 16.0,
                    color: BUTTON_TEXT,
                    ..default()
                },
            ));
        });
}

fn refresh_seat_summaries(setup: Res<MatchSetup>, mut labels: Query<(&SeatSummary, &mut Text)>) {
    for (seat, mut text) in &mut labels {
        let s = &setup.seats[seat.0];
        text.sections[0].value = format!(
            "Seat {} | {} | Name: {} | Bot: {}",
            seat.0 + 1,
            if s.human { "Human" } else { "Bot" },
            s.name,
            s.bot_strategy.label()
        );
    }
}

fn handle_action_buttons(
    mut interaction_q: Query<(&ActionButton, &Interaction), (With<Button>, Changed<Interaction>)>,
    mut setup: ResMut<MatchSetup>,
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
            SeatAction::CycleName(i) => {
                setup.seats[i].name = random_name();
            }
            SeatAction::CycleBot(i) => {
                let current = setup.seats[i].bot_strategy;
                let idx = BotStrategy::ALL
                    .iter()
                    .position(|s| *s == current)
                    .unwrap_or(0);
                setup.seats[i].bot_strategy = BotStrategy::ALL[(idx + 1) % BotStrategy::ALL.len()];
            }
            SeatAction::RandomizeBots => {
                for seat in &mut setup.seats {
                    if !seat.human {
                        seat.name = random_name();
                        seat.bot_strategy = BotStrategy::Random;
                    }
                }
            }
            SeatAction::Start => next_screen.set(Screen::Gameplay),
            SeatAction::Credits => next_screen.set(Screen::Credits),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn exit_app(_trigger: Trigger<OnPress>, mut app_exit: EventWriter<AppExit>) {
    app_exit.send(AppExit::Success);
}
