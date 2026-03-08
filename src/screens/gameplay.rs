//! The screen state for the main gameplay.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use rand::Rng;

use crate::{asset_tracking::LoadResource, audio::Music, screens::Screen};

const PLAYER_COLORS: [Color; 4] = [
    Color::srgb(0.95, 0.25, 0.25),
    Color::srgb(0.2, 0.8, 0.35),
    Color::srgb(0.2, 0.45, 0.95),
    Color::srgb(0.98, 0.85, 0.2),
];
const START_INDICES: [u8; 4] = [0, 13, 26, 39];

pub(super) fn plugin(app: &mut App) {
    app.insert_resource(MatchSetup { human_players: 1 });
    app.add_systems(OnEnter(Screen::Gameplay), (spawn_board, setup_match));

    app.load_resource::<GameplayMusic>();
    app.add_systems(OnEnter(Screen::Gameplay), play_gameplay_music);
    app.add_systems(OnExit(Screen::Gameplay), stop_music);

    app.add_systems(
        Update,
        (
            handle_roll_input,
            handle_token_selection,
            run_bot_turn,
            sync_token_visuals,
            update_status_text,
            return_to_title_screen
                .run_if(input_just_pressed(KeyCode::Escape))
                .run_if(in_state(Screen::Gameplay)),
        )
            .run_if(in_state(Screen::Gameplay)),
    );
}

#[derive(Resource, Clone, Copy)]
pub(super) struct MatchSetup {
    pub human_players: usize,
}

#[derive(Component)]
struct TokenVisual {
    player: usize,
    token: usize,
}

#[derive(Component)]
struct StatusText;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
    Human,
    Bot,
}

#[derive(Clone, Copy)]
enum TokenState {
    Yard,
    Path(u8), // 0..=56 where 56 is finished
}

struct PlayerState {
    kind: PlayerKind,
    tokens: [TokenState; 4],
}

#[derive(Resource)]
struct LudoGame {
    players: [PlayerState; 4],
    current: usize,
    last_roll: Option<u8>,
    consecutive_sixes: u8,
    selectable_tokens: Vec<usize>,
    winner_order: Vec<usize>,
    message: String,
    bot_timer: Timer,
}

fn setup_match(mut commands: Commands, setup: Res<MatchSetup>) {
    let players = std::array::from_fn(|i| PlayerState {
        kind: if i < setup.human_players {
            PlayerKind::Human
        } else {
            PlayerKind::Bot
        },
        tokens: [TokenState::Yard; 4],
    });

    commands.insert_resource(LudoGame {
        players,
        current: 0,
        last_roll: None,
        consecutive_sixes: 0,
        selectable_tokens: Vec::new(),
        winner_order: Vec::new(),
        message: "Roll the die (Space). Then pick token 1-4.".into(),
        bot_timer: Timer::from_seconds(0.8, TimerMode::Repeating),
    });
}

fn spawn_board(mut commands: Commands) {
    commands
        .spawn((
            Name::new("GameplayRoot"),
            StateScoped(Screen::Gameplay),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font_size: 26.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    transform: Transform::from_xyz(0.0, 360.0, 10.0),
                    ..default()
                },
                StatusText,
            ));

            for (track_i, p) in track_points().iter().enumerate() {
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: if [0, 13, 26, 39].contains(&track_i) {
                            Color::srgb(0.9, 0.9, 0.9)
                        } else {
                            Color::srgb(0.18, 0.18, 0.2)
                        },
                        custom_size: Some(Vec2::splat(28.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(*p),
                    ..default()
                });
            }

            for (player, color) in PLAYER_COLORS.into_iter().enumerate() {
                for token in 0..4 {
                    parent.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::splat(20.0)),
                                ..default()
                            },
                            transform: Transform::from_translation(yard_position(player, token)),
                            ..default()
                        },
                        TokenVisual { player, token },
                    ));
                }
            }
        });
}

fn handle_roll_input(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<LudoGame>) {
    if game_over(&game)
        || game.players[game.current].kind != PlayerKind::Human
        || game.last_roll.is_some()
    {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        roll_for_current_player(&mut game);
    }
}

fn handle_token_selection(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<LudoGame>) {
    if game_over(&game)
        || game.players[game.current].kind != PlayerKind::Human
        || game.last_roll.is_none()
    {
        return;
    }

    for (idx, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .into_iter()
    .enumerate()
    {
        if keys.just_pressed(key) {
            if game.selectable_tokens.contains(&idx) {
                apply_move(&mut game, idx);
            }
            break;
        }
    }
}

fn run_bot_turn(time: Res<Time>, mut game: ResMut<LudoGame>) {
    if game_over(&game) || game.players[game.current].kind != PlayerKind::Bot {
        return;
    }

    game.bot_timer.tick(time.delta());
    if !game.bot_timer.finished() {
        return;
    }

    if game.last_roll.is_none() {
        roll_for_current_player(&mut game);
    } else if let Some(token) = game.selectable_tokens.first().copied() {
        apply_move(&mut game, token);
    } else {
        end_turn(&mut game, false);
    }
}

fn roll_for_current_player(game: &mut LudoGame) {
    let roll = rand::thread_rng().gen_range(1..=6);
    game.last_roll = Some(roll);

    if roll == 6 {
        game.consecutive_sixes += 1;
        if game.consecutive_sixes == 3 {
            game.message = format!(
                "Player {} rolled three sixes. Turn skipped!",
                game.current + 1
            );
            game.last_roll = None;
            end_turn(game, false);
            return;
        }
    }

    game.selectable_tokens = legal_moves(game, game.current, roll);
    if game.selectable_tokens.is_empty() {
        game.message = format!(
            "Player {} rolled {roll} and has no legal moves.",
            game.current + 1
        );
        game.last_roll = None;
        end_turn(game, roll == 6);
        return;
    }

    game.message = if game.players[game.current].kind == PlayerKind::Human {
        format!(
            "Player {} rolled {roll}. Choose token {}",
            game.current + 1,
            game.selectable_tokens
                .iter()
                .map(|t| (t + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!("Bot {} rolled {roll}.", game.current + 1)
    };
}

fn apply_move(game: &mut LudoGame, token_index: usize) {
    let roll = game.last_roll.unwrap_or_default();
    game.last_roll = None;

    let player = game.current;
    let token = game.players[player].tokens[token_index];
    let mut landed_path: Option<u8> = None;

    game.players[player].tokens[token_index] = match token {
        TokenState::Yard => {
            landed_path = Some(0);
            TokenState::Path(0)
        }
        TokenState::Path(path) => {
            let next = path + roll;
            landed_path = Some(next);
            TokenState::Path(next)
        }
    };

    if let Some(path) = landed_path.filter(|p| *p <= 50) {
        let track = absolute_track_index(player, path);
        let same_enemy: Vec<(usize, usize)> =
            (0..4)
                .filter(|other| *other != player)
                .flat_map(|other| {
                    game.players[other].tokens.iter().enumerate().filter_map(
                        move |(i, t)| match t {
                            TokenState::Path(other_path)
                                if *other_path <= 50
                                    && absolute_track_index(other, *other_path) == track =>
                            {
                                Some((other, i))
                            }
                            _ => None,
                        },
                    )
                })
                .collect();

        if same_enemy.len() >= 2 {
            game.players[player].tokens[token_index] = TokenState::Yard;
            game.message = format!("Player {} hit a block and was sent home!", player + 1);
        } else {
            for (other, idx) in same_enemy {
                game.players[other].tokens[idx] = TokenState::Yard;
            }
        }
    }

    if player_finished(&game.players[player]) && !game.winner_order.contains(&player) {
        game.winner_order.push(player);
        game.message = format!(
            "Player {} finished at place {}!",
            player + 1,
            game.winner_order.len()
        );
    }

    end_turn(game, roll == 6);
}

fn end_turn(game: &mut LudoGame, mut extra_turn: bool) {
    game.selectable_tokens.clear();
    if game_over(game) {
        return;
    }

    if game.winner_order.contains(&game.current) {
        extra_turn = false;
    }

    if !extra_turn {
        game.consecutive_sixes = 0;
        for _ in 0..4 {
            game.current = (game.current + 1) % 4;
            if !game.winner_order.contains(&game.current) {
                break;
            }
        }
    }
}

fn legal_moves(game: &LudoGame, player: usize, roll: u8) -> Vec<usize> {
    game.players[player]
        .tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| match t {
            TokenState::Yard if roll == 6 => Some(i),
            TokenState::Path(path) if *path < 56 && *path + roll <= 56 => Some(i),
            _ => None,
        })
        .collect()
}

fn sync_token_visuals(game: Res<LudoGame>, mut query: Query<(&TokenVisual, &mut Transform)>) {
    if !game.is_changed() {
        return;
    }

    for (token_visual, mut transform) in &mut query {
        let state = game.players[token_visual.player].tokens[token_visual.token];
        transform.translation = match state {
            TokenState::Yard => yard_position(token_visual.player, token_visual.token),
            TokenState::Path(path) if path <= 50 => {
                let track = absolute_track_index(token_visual.player, path);
                track_points()[track as usize]
                    + Vec3::new((token_visual.token as f32 - 1.5) * 4.0, 0.0, 2.0)
            }
            TokenState::Path(path) => home_position(token_visual.player, path),
        };
    }
}

fn update_status_text(game: Res<LudoGame>, mut text_query: Query<&mut Text, With<StatusText>>) {
    let mut text = text_query.single_mut();
    let ranking = if game.winner_order.is_empty() {
        "None".to_string()
    } else {
        game.winner_order
            .iter()
            .map(|p| format!("P{}", p + 1))
            .collect::<Vec<_>>()
            .join(" > ")
    };

    text.sections[0].value = format!(
        "{}\nTurn: Player {} ({}) | Last roll: {} | Winners: {}\nControls: Space=Roll, 1-4=Move token, Esc=Title",
        game.message,
        game.current + 1,
        if game.players[game.current].kind == PlayerKind::Human {
            "Human"
        } else {
            "Bot"
        },
        game.last_roll
            .map(|x| x.to_string())
            .unwrap_or_else(|| "-".into()),
        ranking
    );
}

fn game_over(game: &LudoGame) -> bool {
    game.winner_order.len() >= 4
}

fn player_finished(player: &PlayerState) -> bool {
    player
        .tokens
        .iter()
        .all(|t| matches!(t, TokenState::Path(56)))
}

fn absolute_track_index(player: usize, path: u8) -> u8 {
    (START_INDICES[player] + path) % 52
}

fn track_points() -> Vec<Vec3> {
    (0..52)
        .map(|i| {
            let angle = (i as f32 / 52.0) * std::f32::consts::TAU;
            Vec3::new(angle.cos() * 280.0, angle.sin() * 280.0, 0.0)
        })
        .collect()
}

fn yard_position(player: usize, token: usize) -> Vec3 {
    let base = [
        Vec2::new(-240.0, -240.0),
        Vec2::new(240.0, -240.0),
        Vec2::new(240.0, 240.0),
        Vec2::new(-240.0, 240.0),
    ][player];
    let offsets = [
        Vec2::new(-30.0, -30.0),
        Vec2::new(30.0, -30.0),
        Vec2::new(-30.0, 30.0),
        Vec2::new(30.0, 30.0),
    ];
    (base + offsets[token]).extend(3.0)
}

fn home_position(player: usize, path: u8) -> Vec3 {
    if path >= 56 {
        return Vec3::new(0.0, 0.0, 2.0);
    }
    let step = (path - 50) as f32;
    match player {
        0 => Vec3::new(-200.0 + step * 40.0, 0.0, 2.0),
        1 => Vec3::new(0.0, -200.0 + step * 40.0, 2.0),
        2 => Vec3::new(200.0 - step * 40.0, 0.0, 2.0),
        _ => Vec3::new(0.0, 200.0 - step * 40.0, 2.0),
    }
}

#[derive(Resource, Asset, Reflect, Clone)]
pub struct GameplayMusic {
    #[dependency]
    handle: Handle<AudioSource>,
    entity: Option<Entity>,
}

impl FromWorld for GameplayMusic {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            handle: assets.load("audio/music/Fluffing A Duck.ogg"),
            entity: None,
        }
    }
}

fn play_gameplay_music(mut commands: Commands, mut music: ResMut<GameplayMusic>) {
    music.entity = Some(
        commands
            .spawn((
                AudioBundle {
                    source: music.handle.clone(),
                    settings: PlaybackSettings::LOOP,
                },
                Music,
            ))
            .id(),
    );
}

fn stop_music(mut commands: Commands, mut music: ResMut<GameplayMusic>) {
    if let Some(entity) = music.entity.take() {
        commands.entity(entity).despawn_recursive();
    }
}

fn return_to_title_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Title);
}
