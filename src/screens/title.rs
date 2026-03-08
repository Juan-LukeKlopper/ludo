//! The title screen that appears when the game starts.

use bevy::prelude::*;

use crate::{
    screens::{
        gameplay::{random_name, BotStrategy, MatchSetup},
        Screen,
    },
    theme::prelude::*,
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
            children.header("🎲 Ludo King V1");
            children.label("Customize players, names, bot personalities and start the match.");

            for seat in 0..4 {
                children.spawn((
                    TextBundle::from_section(
                        "",
                        TextStyle {
                            font_size: 22.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    SeatSummary(seat),
                ));
                action_btn(
                    children,
                    format!("Seat {}: Human/Bot", seat + 1),
                    SeatAction::ToggleHuman(seat),
                );
                action_btn(
                    children,
                    format!("Seat {}: Change Name", seat + 1),
                    SeatAction::CycleName(seat),
                );
                action_btn(
                    children,
                    format!("Seat {}: Bot Strategy", seat + 1),
                    SeatAction::CycleBot(seat),
                );
            }

            action_btn(
                children,
                "Randomize bot names/strategies",
                SeatAction::RandomizeBots,
            );
            action_btn(children, "Start Game", SeatAction::Start);
            action_btn(children, "Credits", SeatAction::Credits);

            #[cfg(not(target_family = "wasm"))]
            children.button("Exit").observe(exit_app);
        });
}

fn action_btn(parent: &mut ChildBuilder, text: impl Into<String>, action: SeatAction) {
    parent.button(text).insert(ActionButton(action));
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
