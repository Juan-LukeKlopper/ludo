//! Win / standings screen.

use bevy::prelude::*;

use crate::{
    screens::{gameplay::LastMatchResult, Screen},
    theme::prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Win), spawn_win_screen);
}

fn spawn_win_screen(mut commands: Commands, result: Res<LastMatchResult>) {
    commands
        .ui_root()
        .insert(StateScoped(Screen::Win))
        .with_children(|children| {
            children.header("🏆 Match Over");
            if result.ranking.is_empty() {
                children.label("No winner data.");
            } else {
                for (i, name) in result.ranking.iter().enumerate() {
                    children.label(format!("{}. {}", i + 1, name));
                }
            }
            children.button("Play Again").observe(back_to_title);
        });
}

fn back_to_title(_trigger: Trigger<OnPress>, mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Title);
}
