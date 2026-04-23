//! A loading screen during which game assets are loaded.
//! This reduces stuttering, especially for audio on WASM.

use bevy::prelude::*;

use crate::{
    demo::player::PlayerAssets,
    screens::{credits::CreditsMusic, gameplay::GameplayMusic, Screen},
    theme::{interaction::InteractionAssets, prelude::*},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Loading), spawn_loading_screen);

    app.add_systems(
        Update,
        continue_to_title_screen
            .run_if(in_state(Screen::Loading))
            .run_if(all_assets_loaded),
    );
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .ui_root()
        .insert(DespawnOnExit(Screen::Loading))
        .with_children(|children| {
            children.label("Loading...").insert(Node {
                justify_content: JustifyContent::Center,
                ..default()
            });
        });
}

fn continue_to_title_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.as_mut().set_if_neq(Screen::Title);
}

fn all_assets_loaded(
    player_assets: Option<Res<PlayerAssets>>,
    interaction_assets: Option<Res<InteractionAssets>>,
    credits_music: Option<Res<CreditsMusic>>,
    gameplay_music: Option<Res<GameplayMusic>>,
) -> bool {
    player_assets.is_some()
        && interaction_assets.is_some()
        && credits_music.is_some()
        && gameplay_music.is_some()
}
