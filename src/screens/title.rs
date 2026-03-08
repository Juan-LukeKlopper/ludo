//! The title screen that appears when the game starts.

use bevy::prelude::*;

use crate::{
    screens::{gameplay::MatchSetup, Screen},
    theme::prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Title), spawn_title_screen);
}

fn spawn_title_screen(mut commands: Commands) {
    commands
        .ui_root()
        .insert(StateScoped(Screen::Title))
        .with_children(|children| {
            children.button("Play Solo (3 Bots)").observe(start_solo);
            children
                .button("Play 2 Players (2 Bots)")
                .observe(start_duo);
            children
                .button("Play 3 Players (1 Bot)")
                .observe(start_trio);
            children.button("Play 4 Players").observe(start_quad);
            children.button("Credits").observe(enter_credits_screen);

            #[cfg(not(target_family = "wasm"))]
            children.button("Exit").observe(exit_app);
        });
}

fn start_solo(
    _trigger: Trigger<OnPress>,
    mut setup: ResMut<MatchSetup>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    setup.human_players = 1;
    next_screen.set(Screen::Gameplay);
}

fn start_duo(
    _trigger: Trigger<OnPress>,
    mut setup: ResMut<MatchSetup>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    setup.human_players = 2;
    next_screen.set(Screen::Gameplay);
}

fn start_trio(
    _trigger: Trigger<OnPress>,
    mut setup: ResMut<MatchSetup>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    setup.human_players = 3;
    next_screen.set(Screen::Gameplay);
}

fn start_quad(
    _trigger: Trigger<OnPress>,
    mut setup: ResMut<MatchSetup>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    setup.human_players = 4;
    next_screen.set(Screen::Gameplay);
}

fn enter_credits_screen(_trigger: Trigger<OnPress>, mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Credits);
}

#[cfg(not(target_family = "wasm"))]
fn exit_app(_trigger: Trigger<OnPress>, mut app_exit: EventWriter<AppExit>) {
    app_exit.send(AppExit::Success);
}
