//! The screen state for the main gameplay.

use bevy::{
    input::common_conditions::input_just_pressed, math::primitives::Circle, prelude::*,
    sprite::MaterialMesh2dBundle, window::PrimaryWindow,
};
use rand::{seq::IteratorRandom, Rng};
use std::{collections::VecDeque, f32::consts::PI};

use crate::{
    asset_tracking::LoadResource,
    audio::{Music, SoundEffect},
    screens::Screen,
};

const PLAYER_COLORS: [Color; 4] = [
    Color::srgb(0.95, 0.25, 0.25),
    Color::srgb(0.2, 0.8, 0.35),
    Color::srgb(0.98, 0.85, 0.2),
    Color::srgb(0.2, 0.45, 0.95),
];
const BOARD_WORLD_SIZE: f32 = 860.0;
const CELL_SIZE: f32 = 36.0;
const START_INDICES: [u8; 4] = [0, 13, 26, 39];
const SAFE_OFFSET: u8 = 8;
const TOKEN_PICK_RADIUS: f32 = 30.0;
const PLAYER_NAMES: [&str; 12] = [
    "Alex", "Sam", "Jordan", "Taylor", "Morgan", "Riley", "Casey", "Sky", "Avery", "Kai", "Nova",
    "Jules",
];

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MatchSetup>();
    app.init_resource::<LastMatchResult>();
    app.init_resource::<DiceAnimation>();
    app.add_systems(OnEnter(Screen::Gameplay), (spawn_board, setup_match));

    app.load_resource::<GameplayMusic>();
    app.load_resource::<GameplaySfx>();
    app.add_systems(OnEnter(Screen::Gameplay), play_gameplay_music);
    app.add_systems(OnExit(Screen::Gameplay), stop_music);

    app.add_systems(
        Update,
        (
            handle_roll_input,
            handle_token_selection,
            handle_keyboard_token_cursor,
            handle_pointer_input,
            run_bot_turn,
            sync_token_targets,
            update_token_visual_state,
            animate_token_transforms,
            fit_gameplay_board_to_window,
            update_status_text,
            update_dice_text,
            animate_confetti,
            move_to_win_screen,
            return_to_title_screen
                .run_if(input_just_pressed(KeyCode::Escape))
                .run_if(in_state(Screen::Gameplay)),
        )
            .run_if(in_state(Screen::Gameplay)),
    );
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StageTheme {
    Midnight,
    Ocean,
    Festival,
}

impl StageTheme {
    pub const ALL: [StageTheme; 3] = [
        StageTheme::Midnight,
        StageTheme::Ocean,
        StageTheme::Festival,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StageTheme::Midnight => "Midnight",
            StageTheme::Ocean => "Ocean",
            StageTheme::Festival => "Festival",
        }
    }
}

struct StagePalette {
    board_bg: Color,
    cross_bg: Color,
    track_cell: Color,
    safe_cell: Color,
}

fn stage_palette(theme: StageTheme) -> StagePalette {
    match theme {
        StageTheme::Midnight => StagePalette {
            board_bg: Color::srgb(0.08, 0.08, 0.12),
            cross_bg: Color::srgb(0.06, 0.06, 0.11),
            track_cell: Color::srgb(0.2, 0.2, 0.24),
            safe_cell: Color::srgb(0.95, 0.95, 0.58),
        },
        StageTheme::Ocean => StagePalette {
            board_bg: Color::srgb(0.06, 0.17, 0.26),
            cross_bg: Color::srgb(0.07, 0.21, 0.31),
            track_cell: Color::srgb(0.32, 0.57, 0.65),
            safe_cell: Color::srgb(0.8, 0.95, 1.0),
        },
        StageTheme::Festival => StagePalette {
            board_bg: Color::srgb(0.16, 0.07, 0.19),
            cross_bg: Color::srgb(0.2, 0.09, 0.24),
            track_cell: Color::srgb(0.45, 0.23, 0.49),
            safe_cell: Color::srgb(0.98, 0.82, 0.38),
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotStrategy {
    Random,
    Aggressive,
    Timid,
    Grudge,
    Racer,
    Train,
    SoloRunner,
}

impl BotStrategy {
    pub const ALL: [BotStrategy; 7] = [
        BotStrategy::Random,
        BotStrategy::Aggressive,
        BotStrategy::Timid,
        BotStrategy::Grudge,
        BotStrategy::Racer,
        BotStrategy::Train,
        BotStrategy::SoloRunner,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BotStrategy::Random => "Random",
            BotStrategy::Aggressive => "Aggressive",
            BotStrategy::Timid => "Timid",
            BotStrategy::Grudge => "Grudge",
            BotStrategy::Racer => "Win-First",
            BotStrategy::Train => "Train",
            BotStrategy::SoloRunner => "Solo Runner",
        }
    }

    pub fn resolved(self) -> Self {
        if self != BotStrategy::Random {
            return self;
        }
        *BotStrategy::ALL
            .iter()
            .skip(1)
            .choose(&mut rand::thread_rng())
            .unwrap_or(&BotStrategy::Racer)
    }
}

#[derive(Resource, Clone)]
pub struct MatchSetup {
    pub seats: [SeatSetup; 4],
    pub stage_theme: StageTheme,
}

#[derive(Clone)]
pub struct SeatSetup {
    pub name: String,
    pub human: bool,
    pub bot_strategy: BotStrategy,
}

impl Default for MatchSetup {
    fn default() -> Self {
        Self {
            seats: [
                SeatSetup {
                    name: "Player 1".into(),
                    human: true,
                    bot_strategy: BotStrategy::Random,
                },
                SeatSetup {
                    name: "Bot 2".into(),
                    human: false,
                    bot_strategy: BotStrategy::Random,
                },
                SeatSetup {
                    name: "Bot 3".into(),
                    human: false,
                    bot_strategy: BotStrategy::Random,
                },
                SeatSetup {
                    name: "Bot 4".into(),
                    human: false,
                    bot_strategy: BotStrategy::Random,
                },
            ],
            stage_theme: StageTheme::Midnight,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct LastMatchResult {
    pub ranking: Vec<String>,
}

#[derive(Component)]
struct TokenVisual {
    player: usize,
    token: usize,
    target: Vec3,
    logical_state: TokenState,
    segment_start: Vec3,
    segment_progress: f32,
    waypoints: VecDeque<Vec3>,
}

#[derive(Component)]
struct GameplayBoard;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct DiceValueText;

#[derive(Component)]
struct DiceSubText;

#[derive(Component)]
struct ConfettiPiece {
    velocity: Vec2,
    spin: f32,
    timer: Timer,
}

#[derive(Resource)]
struct DiceAnimation {
    timer: Timer,
    rolling: bool,
    last_face: Option<u8>,
    spin_face: u8,
    spin_tick: Timer,
}

impl Default for DiceAnimation {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.6, TimerMode::Once),
            rolling: false,
            last_face: None,
            spin_face: 1,
            spin_tick: Timer::from_seconds(0.06, TimerMode::Repeating),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayerKind {
    Human,
    Bot,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenState {
    Yard,
    Path(u8), // 0..=56 where 56 is finished
}

struct PlayerState {
    kind: PlayerKind,
    name: String,
    strategy: BotStrategy,
    grudge: [i32; 4],
    tokens: [TokenState; 4],
}

#[derive(Resource)]
struct LudoGame {
    players: [PlayerState; 4],
    current: usize,
    last_roll: Option<u8>,
    consecutive_sixes: u8,
    selectable_tokens: Vec<usize>,
    keyboard_selected_token: Option<usize>,
    winner_order: Vec<usize>,
    message: String,
    bot_timer: Timer,
    finished: bool,
}

#[derive(Resource, Asset, Reflect, Clone)]
pub struct GameplayMusic {
    #[dependency]
    handle: Handle<AudioSource>,
    entity: Option<Entity>,
}

#[derive(Resource, Asset, Reflect, Clone)]
struct GameplaySfx {
    #[dependency]
    roll: Handle<AudioSource>,
    #[dependency]
    capture: Handle<AudioSource>,
    #[dependency]
    win: Handle<AudioSource>,
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

impl FromWorld for GameplaySfx {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            roll: assets.load("audio/sound_effects/step1.ogg"),
            capture: assets.load("audio/sound_effects/button_press.ogg"),
            win: assets.load("audio/sound_effects/step4.ogg"),
        }
    }
}

fn setup_match(mut commands: Commands, setup: Res<MatchSetup>) {
    let players = std::array::from_fn(|i| {
        let seat = &setup.seats[i];
        PlayerState {
            kind: if seat.human {
                PlayerKind::Human
            } else {
                PlayerKind::Bot
            },
            name: seat.name.clone(),
            strategy: seat.bot_strategy.resolved(),
            grudge: [0; 4],
            tokens: [TokenState::Yard; 4],
        }
    });

    commands.insert_resource(LudoGame {
        players,
        current: 0,
        last_roll: None,
        consecutive_sixes: 0,
        selectable_tokens: Vec::new(),
        keyboard_selected_token: None,
        winner_order: Vec::new(),
        message:
            "Roll the die (Space or tap DICE). Then choose a token with 1-4, arrows+Enter, or tap."
                .into(),
        bot_timer: Timer::from_seconds(0.12, TimerMode::Repeating),
        finished: false,
    });
}

fn spawn_board(
    mut commands: Commands,
    setup: Res<MatchSetup>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let token_mesh = meshes.add(Circle::new(1.0));
    let palette = stage_palette(setup.stage_theme);
    let dice_anchor = Vec3::new(0.0, 265.0, 40.0);
    commands
        .spawn((
            Name::new("GameplayRoot"),
            StateScoped(Screen::Gameplay),
            GameplayBoard,
            Transform::default(),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: palette.board_bg,
                    custom_size: Some(Vec2::splat(BOARD_WORLD_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -20.0),
                ..default()
            });

            for (player, c) in PLAYER_COLORS.iter().enumerate() {
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: c.with_alpha(0.17),
                        custom_size: Some(Vec2::new(300.0, 300.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(yard_base(player).extend(-5.0)),
                    ..default()
                });
            }

            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: palette.cross_bg,
                    custom_size: Some(Vec2::new(220.0, 560.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -8.0),
                ..default()
            });
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: palette.cross_bg,
                    custom_size: Some(Vec2::new(560.0, 220.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -8.0),
                ..default()
            });

            let points = track_points();
            for p in &points {
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: palette.track_cell,
                        custom_size: Some(Vec2::splat(26.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(*p),
                    ..default()
                });
            }

            for safe_idx in safe_track_indices() {
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: palette.safe_cell,
                        custom_size: Some(Vec2::splat(14.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(
                        points[safe_idx as usize] + Vec3::new(0.0, 0.0, 1.0),
                    ),
                    ..default()
                });
            }

            for (player, player_color) in PLAYER_COLORS.iter().enumerate() {
                for step in 0..6 {
                    parent.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: player_color.with_alpha(0.34),
                            custom_size: Some(Vec2::splat(24.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(
                            home_position(player, 50 + step) - Vec3::new(0.0, 0.0, 1.0),
                        ),
                        ..default()
                    });
                }
            }

            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font_size: 18.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    transform: Transform::from_xyz(0.0, 370.0, 12.0),
                    ..default()
                },
                StatusText,
            ));

            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "-",
                        TextStyle {
                            font_size: 64.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    transform: Transform::from_translation(dice_anchor),
                    ..default()
                },
                DiceValueText,
            ));

            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font_size: 28.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    transform: Transform::from_translation(
                        dice_anchor + Vec3::new(0.0, -56.0, 0.0),
                    ),
                    ..default()
                },
                DiceSubText,
            ));

            for (player, color) in PLAYER_COLORS.into_iter().enumerate() {
                for token in 0..4 {
                    let start = yard_position(player, token);
                    parent
                        .spawn((
                            Transform::from_translation(start),
                            GlobalTransform::default(),
                            TokenVisual {
                                player,
                                token,
                                target: start,
                                logical_state: TokenState::Yard,
                                segment_start: start,
                                segment_progress: 0.0,
                                waypoints: VecDeque::new(),
                            },
                        ))
                        .with_children(|token_parent| {
                            token_parent.spawn(MaterialMesh2dBundle {
                                mesh: token_mesh.clone().into(),
                                material: materials.add(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                                transform: Transform::from_xyz(1.0, -2.5, -0.3)
                                    .with_scale(Vec3::splat(13.0)),
                                ..default()
                            });
                            token_parent.spawn(MaterialMesh2dBundle {
                                mesh: token_mesh.clone().into(),
                                material: materials.add(Color::srgb(0.97, 0.97, 0.97)),
                                transform: Transform::from_scale(Vec3::splat(11.5)),
                                ..default()
                            });
                            token_parent.spawn(MaterialMesh2dBundle {
                                mesh: token_mesh.clone().into(),
                                material: materials.add(color),
                                transform: Transform::from_scale(Vec3::splat(8.4)),
                                ..default()
                            });
                            token_parent.spawn(MaterialMesh2dBundle {
                                mesh: token_mesh.clone().into(),
                                material: materials.add(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                                transform: Transform::from_xyz(-2.2, 2.1, 0.2)
                                    .with_scale(Vec3::splat(3.0)),
                                ..default()
                            });
                            token_parent.spawn(SpriteBundle {
                                sprite: Sprite {
                                    color: Color::srgb(0.96, 0.96, 0.96),
                                    custom_size: Some(Vec2::new(6.8, 6.8)),
                                    ..default()
                                },
                                transform: Transform::from_xyz(0.0, -8.8, 0.15)
                                    .with_rotation(Quat::from_rotation_z(45_f32.to_radians())),
                                ..default()
                            });
                            token_parent.spawn(SpriteBundle {
                                sprite: Sprite {
                                    color,
                                    custom_size: Some(Vec2::new(3.8, 3.8)),
                                    ..default()
                                },
                                transform: Transform::from_xyz(0.0, -8.2, 0.18)
                                    .with_rotation(Quat::from_rotation_z(45_f32.to_radians())),
                                ..default()
                            });
                        });
                }
            }
        });
}

fn fit_gameplay_board_to_window(
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut board_q: Query<&mut Transform, With<GameplayBoard>>,
) {
    let Ok(window) = window_q.get_single() else {
        return;
    };
    let scale_from_width = window.width() * 0.98 / BOARD_WORLD_SIZE;
    let scale_from_height = window.height() * 0.86 / BOARD_WORLD_SIZE;
    let target = scale_from_width.min(scale_from_height).clamp(0.55, 2.2);
    let portrait = window.height() > window.width() * 1.2;
    let y_offset = if portrait {
        (window.height() * 0.12) / target
    } else {
        0.0
    };
    for mut transform in &mut board_q {
        transform.scale = Vec3::splat(target);
        transform.translation = Vec3::new(0.0, y_offset, 0.0);
    }
}

fn handle_roll_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
    mut dice_animation: ResMut<DiceAnimation>,
    sfx: Res<GameplaySfx>,
) {
    if game_over(&game)
        || game.players[game.current].kind != PlayerKind::Human
        || game.last_roll.is_some()
    {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        play_sfx(&mut commands, sfx.roll.clone());
        let roll = roll_for_current_player(&mut game);
        start_dice_roll_animation(&mut dice_animation, roll);
    }
}

fn handle_token_selection(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
    sfx: Res<GameplaySfx>,
) {
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
                game.keyboard_selected_token = Some(idx);
                apply_move(&mut game, idx, &mut commands, &sfx);
            }
            break;
        }
    }
}

fn handle_keyboard_token_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
    sfx: Res<GameplaySfx>,
) {
    if game_over(&game)
        || game.players[game.current].kind != PlayerKind::Human
        || game.last_roll.is_none()
        || game.selectable_tokens.is_empty()
    {
        return;
    }

    let mut moved_cursor = false;

    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::ArrowUp) {
        game.keyboard_selected_token = Some(step_selected_token(&game, -1));
        moved_cursor = true;
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::ArrowDown) {
        game.keyboard_selected_token = Some(step_selected_token(&game, 1));
        moved_cursor = true;
    }

    if moved_cursor {
        return;
    }

    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        let token = game
            .keyboard_selected_token
            .filter(|t| game.selectable_tokens.contains(t))
            .unwrap_or(game.selectable_tokens[0]);
        game.keyboard_selected_token = Some(token);
        apply_move(&mut game, token, &mut commands, &sfx);
    }
}

fn handle_pointer_input(
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    token_q: Query<(&TokenVisual, &GlobalTransform)>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
    mut dice_animation: ResMut<DiceAnimation>,
    sfx: Res<GameplaySfx>,
) {
    if game_over(&game) || game.players[game.current].kind != PlayerKind::Human {
        return;
    }

    let pressed = mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed();
    if !pressed {
        return;
    }

    let Some(pointer_world) = pointer_world_position(&window_q, &camera_q, &touches) else {
        return;
    };

    if game.last_roll.is_none() {
        let _ = pointer_world;
        play_sfx(&mut commands, sfx.roll.clone());
        let roll = roll_for_current_player(&mut game);
        start_dice_roll_animation(&mut dice_animation, roll);
        return;
    }

    if game.selectable_tokens.is_empty() {
        return;
    }

    if let Some(token) = nearest_selectable_token(&game, pointer_world, &token_q) {
        game.keyboard_selected_token = Some(token);
        apply_move(&mut game, token, &mut commands, &sfx);
    }
}

fn run_bot_turn(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
    mut dice_animation: ResMut<DiceAnimation>,
    sfx: Res<GameplaySfx>,
) {
    if game_over(&game) || game.players[game.current].kind != PlayerKind::Bot {
        return;
    }

    if dice_animation.rolling {
        game.bot_timer.reset();
        return;
    }

    game.bot_timer.tick(time.delta());
    if !game.bot_timer.finished() {
        return;
    }

    if game.last_roll.is_none() {
        play_sfx(&mut commands, sfx.roll.clone());
        let roll = roll_for_current_player(&mut game);
        start_dice_roll_animation(&mut dice_animation, roll);
    } else if let Some(token) =
        pick_bot_move(&game, game.current, game.selectable_tokens.as_slice())
    {
        apply_move(&mut game, token, &mut commands, &sfx);
    } else {
        end_turn(&mut game, false);
    }
}

fn pick_bot_move(game: &LudoGame, player: usize, options: &[usize]) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    let roll = game.last_roll?;
    let strat = game.players[player].strategy;

    options
        .iter()
        .copied()
        .max_by_key(|token| score_move(game, player, *token, roll, strat))
}

fn score_move(
    game: &LudoGame,
    player: usize,
    token: usize,
    roll: u8,
    strategy: BotStrategy,
) -> i32 {
    let state = game.players[player].tokens[token];
    let next_path = match state {
        TokenState::Yard => 0,
        TokenState::Path(path) => path + roll,
    };
    let progress_bonus = i32::from(next_path) * 5;
    let in_home_lane = if next_path > 50 { 18 } else { 0 };
    let capture = capture_count(game, player, next_path) as i32;
    let danger = danger_count(game, player, next_path) as i32;
    let block = own_count_on_target(game, player, next_path) as i32;
    let is_furthest = furthest_token(game, player) == token;
    let is_back = rearmost_token(game, player) == token;
    let grudge_target = grudge_target(game, player);
    let grudge_hit = if hits_player(game, player, next_path, grudge_target) {
        1
    } else {
        0
    };

    match strategy {
        BotStrategy::Aggressive => progress_bonus + capture * 120 - danger * 15 + block * 8,
        BotStrategy::Timid => {
            progress_bonus + in_home_lane * 3 + block * 40 - danger * 70 + capture * 20
        }
        BotStrategy::Grudge => progress_bonus + capture * 80 + grudge_hit * 130 - danger * 20,
        BotStrategy::Racer => progress_bonus * 2 + in_home_lane * 40 - capture * 10 - danger * 10,
        BotStrategy::Train => {
            progress_bonus + block * 90 + capture * 15 - danger * 15 + if is_back { 20 } else { 0 }
        }
        BotStrategy::SoloRunner => {
            progress_bonus + if is_furthest { 120 } else { -50 } + in_home_lane * 35
        }
        BotStrategy::Random => rand::thread_rng().gen_range(0..100),
    }
}

fn roll_for_current_player(game: &mut LudoGame) -> u8 {
    let roll = rand::thread_rng().gen_range(1..=6);
    game.last_roll = Some(roll);

    if roll == 6 {
        game.consecutive_sixes += 1;
        if game.consecutive_sixes == 3 {
            game.message = format!(
                "{} rolled three sixes. Turn skipped!",
                game.players[game.current].name
            );
            game.last_roll = None;
            end_turn(game, false);
            return roll;
        }
    }

    game.selectable_tokens = legal_moves(game, game.current, roll);
    game.keyboard_selected_token = game.selectable_tokens.first().copied();
    if game.selectable_tokens.is_empty() {
        game.message = format!(
            "{} rolled {roll} and has no legal moves.",
            game.players[game.current].name
        );
        game.last_roll = None;
        game.keyboard_selected_token = None;
        end_turn(game, roll == 6);
        return roll;
    }

    game.message = if game.players[game.current].kind == PlayerKind::Human {
        format!(
            "{} rolled {roll}. Choose token {}",
            game.players[game.current].name,
            game.selectable_tokens
                .iter()
                .map(|t| (t + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "{} ({}) rolled {roll}.",
            game.players[game.current].name,
            game.players[game.current].strategy.label()
        )
    };

    roll
}

fn start_dice_roll_animation(dice_animation: &mut DiceAnimation, roll: u8) {
    dice_animation.rolling = true;
    dice_animation.last_face = Some(roll);
    dice_animation.spin_face = rand::thread_rng().gen_range(1..=6);
    dice_animation.timer.reset();
    dice_animation.spin_tick.reset();
}

fn apply_move(game: &mut LudoGame, token_index: usize, commands: &mut Commands, sfx: &GameplaySfx) {
    let roll = game.last_roll.unwrap_or_default();
    game.last_roll = None;

    let player = game.current;
    let token = game.players[player].tokens[token_index];

    let landed_path = match token {
        TokenState::Yard => {
            game.players[player].tokens[token_index] = TokenState::Path(0);
            Some(0)
        }
        TokenState::Path(path) => {
            let next = path + roll;
            game.players[player].tokens[token_index] = TokenState::Path(next);
            Some(next)
        }
    };

    let mut capture_happened = false;
    if let Some(path) = landed_path.filter(|p| *p <= 50) {
        let track = absolute_track_index(player, path);
        if !is_safe_track(track) {
            let same_enemy: Vec<(usize, usize)> = (0..4)
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
                game.message = format!(
                    "{} crashed into a block and got sent home!",
                    game.players[player].name
                );
            } else {
                for (other, idx) in same_enemy {
                    game.players[other].tokens[idx] = TokenState::Yard;
                    game.players[other].grudge[player] += 1;
                    capture_happened = true;
                }
            }
        }
    }

    if capture_happened {
        play_sfx(commands, sfx.capture.clone());
        game.message = format!(
            "{} captured a token and earned another roll!",
            game.players[player].name
        );
    }

    let reached_home = landed_path == Some(56);
    if reached_home {
        game.message = format!(
            "{} reached home and earned another roll!",
            game.players[player].name
        );
    }

    if player_finished(&game.players[player]) && !game.winner_order.contains(&player) {
        game.winner_order.push(player);
        play_sfx(commands, sfx.win.clone());
        spawn_confetti(commands);
        game.message = format!(
            "{} finished at place {}!",
            game.players[player].name,
            game.winner_order.len()
        );
    }

    end_turn(game, roll == 6 || capture_happened || reached_home);
}

fn end_turn(game: &mut LudoGame, mut extra_turn: bool) {
    game.selectable_tokens.clear();
    game.keyboard_selected_token = None;
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

fn sync_token_targets(game: Res<LudoGame>, mut query: Query<&mut TokenVisual>) {
    if !game.is_changed() {
        return;
    }

    let points = track_points();
    for mut token_visual in &mut query {
        let player = token_visual.player;
        let token = token_visual.token;
        let new_state = game.players[player].tokens[token];
        token_visual.target = token_position_for_state(player, token, new_state, &points);

        if token_visual.logical_state != new_state {
            token_visual.waypoints = VecDeque::from(movement_waypoints(
                player,
                token,
                token_visual.logical_state,
                new_state,
                &points,
            ));
            token_visual.segment_progress = 0.0;
            token_visual.segment_start = token_visual.target;
            token_visual.logical_state = new_state;
        }
    }
}

fn update_token_visual_state(
    game: Res<LudoGame>,
    time: Res<Time>,
    mut token_query: Query<(&TokenVisual, &mut Transform)>,
) {
    for (token_visual, mut transform) in &mut token_query {
        let is_current = token_visual.player == game.current;
        let is_selectable = game.selectable_tokens.contains(&token_visual.token) && is_current;
        let is_selected = game.keyboard_selected_token == Some(token_visual.token) && is_selectable;

        let scale = if is_selected {
            1.6
        } else if is_selectable {
            1.35
        } else {
            1.0
        };
        let pulse = if is_selectable {
            1.0 + (time.elapsed_seconds() * 8.0).sin().abs() * 0.1
        } else {
            1.0
        };
        transform.scale = Vec3::splat(scale * pulse);
    }
}

fn step_selected_token(game: &LudoGame, direction: i32) -> usize {
    let options = &game.selectable_tokens;
    if options.is_empty() {
        return 0;
    }

    let current_idx = game
        .keyboard_selected_token
        .and_then(|selected| options.iter().position(|token| *token == selected))
        .unwrap_or(0) as i32;

    let len = options.len() as i32;
    let next_idx = (current_idx + direction).rem_euclid(len) as usize;
    options[next_idx]
}

fn pointer_world_position(
    window_q: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    touches: &Touches,
) -> Option<Vec2> {
    let window = window_q.get_single().ok()?;
    let screen_position = if let Some(touch) = touches.iter_just_pressed().next() {
        touch.position()
    } else {
        window.cursor_position()?
    };

    let (camera, camera_transform) = camera_q.get_single().ok()?;
    camera.viewport_to_world_2d(camera_transform, screen_position)
}

fn nearest_selectable_token(
    game: &LudoGame,
    pointer_world: Vec2,
    token_q: &Query<(&TokenVisual, &GlobalTransform)>,
) -> Option<usize> {
    token_q
        .iter()
        .filter(|(token_visual, _)| {
            token_visual.player == game.current
                && game.selectable_tokens.contains(&token_visual.token)
        })
        .filter_map(|(token_visual, transform)| {
            let token_pos = transform.translation().truncate();
            let distance = token_pos.distance(pointer_world);
            (distance <= TOKEN_PICK_RADIUS).then_some((token_visual.token, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(token, _)| token)
}
fn animate_token_transforms(time: Res<Time>, mut query: Query<(&mut TokenVisual, &mut Transform)>) {
    for (mut token, mut transform) in &mut query {
        if let Some(next_stop) = token.waypoints.front().copied() {
            if token.segment_start == token.target {
                token.segment_start = transform.translation;
            }

            let duration = (token.segment_start.distance(next_stop) / 210.0).max(0.08);
            token.segment_progress =
                (token.segment_progress + time.delta_seconds() / duration).clamp(0.0, 1.0);

            let mut position = token.segment_start.lerp(next_stop, token.segment_progress);
            position.y += (PI * token.segment_progress).sin() * 14.0;
            transform.translation = position;

            if token.segment_progress >= 1.0 {
                transform.translation = next_stop;
                token.segment_start = next_stop;
                token.segment_progress = 0.0;
                token.waypoints.pop_front();
            }
        } else {
            let speed = (time.delta_seconds() * 9.0).clamp(0.0, 1.0);
            transform.translation = transform.translation.lerp(token.target, speed);
            token.segment_start = transform.translation;
        }
    }
}

fn animate_confetti(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut ConfettiPiece)>,
) {
    for (entity, mut transform, mut sprite, mut confetti) in &mut query {
        confetti.timer.tick(time.delta());
        confetti.velocity.y -= 520.0 * time.delta_seconds();
        transform.translation += (confetti.velocity * time.delta_seconds()).extend(0.0);
        transform.rotate_z(confetti.spin * time.delta_seconds());

        let life = 1.0 - confetti.timer.fraction();
        sprite.color.set_alpha(life.clamp(0.0, 1.0));

        if confetti.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn move_to_win_screen(
    mut next_screen: ResMut<NextState<Screen>>,
    mut game: ResMut<LudoGame>,
    mut last: ResMut<LastMatchResult>,
) {
    if !game_over(&game) || game.finished {
        return;
    }

    game.finished = true;
    last.ranking = game
        .winner_order
        .iter()
        .map(|idx| game.players[*idx].name.clone())
        .collect();
    next_screen.set(Screen::Win);
}

fn update_status_text(game: Res<LudoGame>, mut text_query: Query<&mut Text, With<StatusText>>) {
    let mut text = text_query.single_mut();
    let ranking = if game.winner_order.is_empty() {
        "None".to_string()
    } else {
        game.winner_order
            .iter()
            .map(|p| game.players[*p].name.clone())
            .collect::<Vec<_>>()
            .join(" > ")
    };

    text.sections[0].value = format!(
        "Now: {}\nTurn: {} ({})   Last roll: {}\nWinners: {}\nControls: Space/Tap DICE roll | 1-4 or Arrows pick | Enter/Tap move | Esc title",
        game.message,
        game.players[game.current].name,
        if game.players[game.current].kind == PlayerKind::Human {
            "Human"
        } else {
            game.players[game.current].strategy.label()
        },
        game.last_roll.map(|x| x.to_string()).unwrap_or_else(|| "-".into()),
        ranking
    );
}

fn update_dice_text(
    time: Res<Time>,
    game: Res<LudoGame>,
    mut dice_animation: ResMut<DiceAnimation>,
    mut value_query: Query<(&mut Text, &mut Transform), With<DiceValueText>>,
    mut sub_query: Query<&mut Text, With<DiceSubText>>,
) {
    let Ok((mut value_text, mut value_transform)) = value_query.get_single_mut() else {
        return;
    };
    let Ok(mut sub_text) = sub_query.get_single_mut() else {
        return;
    };

    let face = if dice_animation.rolling {
        dice_animation.timer.tick(time.delta());
        dice_animation.spin_tick.tick(time.delta());
        if dice_animation.spin_tick.just_finished() {
            let mut rng = rand::thread_rng();
            let mut next = rng.gen_range(1..=6);
            if next == dice_animation.spin_face {
                next = (next % 6) + 1;
            }
            dice_animation.spin_face = next;
        }
        value_transform.rotation = Quat::from_rotation_z(time.elapsed_seconds() * 11.0);
        value_transform.scale =
            Vec3::splat(1.0 + (time.elapsed_seconds() * 12.0).sin().abs() * 0.16);

        if dice_animation.timer.finished() {
            dice_animation.rolling = false;
            value_transform.rotation = Quat::IDENTITY;
            value_transform.scale = Vec3::ONE;
            dice_animation.last_face.unwrap_or(dice_animation.spin_face)
        } else {
            dice_animation.spin_face
        }
    } else {
        value_transform.rotation = Quat::IDENTITY;
        value_transform.scale = Vec3::ONE;
        game.last_roll
            .or(dice_animation.last_face)
            .unwrap_or_default()
    };

    let sub_header = if face == 6 && game.last_roll == Some(6) {
        "Bonus roll!".to_string()
    } else if game.last_roll.is_some() {
        "Choose a token".to_string()
    } else {
        format!("{}'s turn", game.players[game.current].name)
    };

    value_text.sections[0].value = if face == 0 {
        "-".to_string()
    } else {
        face.to_string()
    };
    sub_text.sections[0].value = sub_header;
}

fn token_position_for_state(
    player: usize,
    token: usize,
    state: TokenState,
    points: &[Vec3],
) -> Vec3 {
    match state {
        TokenState::Yard => yard_position(player, token),
        TokenState::Path(path) if path <= 50 => {
            let track = absolute_track_index(player, path);
            points[track as usize] + Vec3::new((token as f32 - 1.5) * 4.0, 0.0, 2.0)
        }
        TokenState::Path(path) => home_position(player, path),
    }
}

fn movement_waypoints(
    player: usize,
    token: usize,
    from: TokenState,
    to: TokenState,
    points: &[Vec3],
) -> Vec<Vec3> {
    match (from, to) {
        (TokenState::Yard, TokenState::Path(0)) => {
            vec![token_position_for_state(player, token, to, points)]
        }
        (TokenState::Path(from_path), TokenState::Path(to_path)) if to_path > from_path => {
            let mut path_points = Vec::new();
            for step in (from_path + 1)..=to_path {
                path_points.push(token_position_for_state(
                    player,
                    token,
                    TokenState::Path(step),
                    points,
                ));
            }
            path_points
        }
        (_, TokenState::Yard) => vec![yard_position(player, token)],
        _ => vec![token_position_for_state(player, token, to, points)],
    }
}

fn spawn_confetti(commands: &mut Commands) {
    let mut rng = rand::thread_rng();
    let confetti_colors = [
        Color::srgb(0.98, 0.2, 0.2),
        Color::srgb(0.2, 0.9, 0.3),
        Color::srgb(0.95, 0.82, 0.2),
        Color::srgb(0.2, 0.55, 0.98),
        Color::srgb(1.0, 0.58, 0.18),
        Color::srgb(0.93, 0.35, 0.96),
    ];

    for _ in 0..70 {
        let color = confetti_colors[rng.gen_range(0..confetti_colors.len())];
        let velocity = Vec2::new(rng.gen_range(-260.0..260.0), rng.gen_range(130.0..420.0));
        let size = rng.gen_range(5.0..10.0);
        commands.spawn((
            StateScoped(Screen::Gameplay),
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 60.0),
                ..default()
            },
            ConfettiPiece {
                velocity,
                spin: rng.gen_range(-8.0..8.0),
                timer: Timer::from_seconds(rng.gen_range(0.8..1.6), TimerMode::Once),
            },
        ));
    }
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

fn safe_track_indices() -> [u8; 8] {
    [
        START_INDICES[0],
        START_INDICES[1],
        START_INDICES[2],
        START_INDICES[3],
        (START_INDICES[0] + SAFE_OFFSET) % 52,
        (START_INDICES[1] + SAFE_OFFSET) % 52,
        (START_INDICES[2] + SAFE_OFFSET) % 52,
        (START_INDICES[3] + SAFE_OFFSET) % 52,
    ]
}

fn is_safe_track(track: u8) -> bool {
    safe_track_indices().contains(&track)
}

fn own_count_on_target(game: &LudoGame, player: usize, path: u8) -> usize {
    game.players[player]
        .tokens
        .iter()
        .filter(|t| matches!(t, TokenState::Path(p) if *p<=50 && absolute_track_index(player,*p)==absolute_track_index(player, path)))
        .count()
}

fn capture_count(game: &LudoGame, player: usize, path: u8) -> usize {
    if path > 50 {
        return 0;
    }
    let target = absolute_track_index(player, path);
    if is_safe_track(target) {
        return 0;
    }
    (0..4)
        .filter(|other| *other != player)
        .map(|other| {
            game.players[other]
                .tokens
                .iter()
                .filter(|t| matches!(t, TokenState::Path(p) if *p<=50 && absolute_track_index(other,*p)==target))
                .count()
        })
        .sum()
}

fn danger_count(game: &LudoGame, player: usize, path: u8) -> usize {
    if path > 50 {
        return 0;
    }
    let target = absolute_track_index(player, path);
    if is_safe_track(target) {
        return 0;
    }
    (0..4)
        .filter(|other| *other != player)
        .map(|other| {
            game.players[other]
                .tokens
                .iter()
                .filter(|t| match t {
                    TokenState::Path(p) if *p <= 50 => {
                        let other_track = absolute_track_index(other, *p);
                        let dist = (target + 52 - other_track) % 52;
                        (1..=6).contains(&dist)
                    }
                    _ => false,
                })
                .count()
        })
        .sum()
}

fn furthest_token(game: &LudoGame, player: usize) -> usize {
    game.players[player]
        .tokens
        .iter()
        .enumerate()
        .max_by_key(|(_, t)| match t {
            TokenState::Yard => 0,
            TokenState::Path(p) => *p,
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn rearmost_token(game: &LudoGame, player: usize) -> usize {
    game.players[player]
        .tokens
        .iter()
        .enumerate()
        .min_by_key(|(_, t)| match t {
            TokenState::Yard => 0,
            TokenState::Path(p) => *p,
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn grudge_target(game: &LudoGame, player: usize) -> usize {
    game.players[player]
        .grudge
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != player)
        .max_by_key(|(_, score)| **score)
        .map(|(idx, _)| idx)
        .unwrap_or((player + 1) % 4)
}

fn hits_player(game: &LudoGame, player: usize, path: u8, target_player: usize) -> bool {
    if path > 50 {
        return false;
    }
    let target = absolute_track_index(player, path);
    if is_safe_track(target) {
        return false;
    }
    game.players[target_player]
        .tokens
        .iter()
        .any(|t| matches!(t, TokenState::Path(p) if *p<=50 && absolute_track_index(target_player,*p)==target))
}

fn board_coord(col: i32, row: i32) -> Vec3 {
    let x = (col as f32 - 7.0) * CELL_SIZE;
    let y = (7.0 - row as f32) * CELL_SIZE;
    Vec3::new(x, y, 0.0)
}

fn track_points() -> Vec<Vec3> {
    [
        (1, 6),
        (2, 6),
        (3, 6),
        (4, 6),
        (5, 6),
        (6, 5),
        (6, 4),
        (6, 3),
        (6, 2),
        (6, 1),
        (6, 0),
        (7, 0),
        (8, 0),
        (8, 1),
        (8, 2),
        (8, 3),
        (8, 4),
        (8, 5),
        (9, 6),
        (10, 6),
        (11, 6),
        (12, 6),
        (13, 6),
        (14, 6),
        (14, 7),
        (14, 8),
        (13, 8),
        (12, 8),
        (11, 8),
        (10, 8),
        (9, 8),
        (8, 9),
        (8, 10),
        (8, 11),
        (8, 12),
        (8, 13),
        (8, 14),
        (7, 14),
        (6, 14),
        (6, 13),
        (6, 12),
        (6, 11),
        (6, 10),
        (6, 9),
        (5, 8),
        (4, 8),
        (3, 8),
        (2, 8),
        (1, 8),
        (0, 8),
        (0, 7),
        (0, 6),
    ]
    .into_iter()
    .map(|(col, row)| board_coord(col, row))
    .collect()
}

fn yard_base(player: usize) -> Vec2 {
    [
        Vec2::new(-220.0, 220.0),
        Vec2::new(220.0, 220.0),
        Vec2::new(220.0, -220.0),
        Vec2::new(-220.0, -220.0),
    ][player]
}

fn yard_position(player: usize, token: usize) -> Vec3 {
    let offsets = [
        Vec2::new(-42.0, -42.0),
        Vec2::new(42.0, -42.0),
        Vec2::new(-42.0, 42.0),
        Vec2::new(42.0, 42.0),
    ];
    (yard_base(player) + offsets[token]).extend(3.0)
}

fn home_position(player: usize, path: u8) -> Vec3 {
    if path >= 56 {
        return Vec3::new(0.0, 0.0, 2.0);
    }

    let step = (path - 50) as i32;
    match player {
        0 => board_coord(1 + step, 7) + Vec3::new(0.0, 0.0, 2.0),
        1 => board_coord(7, 1 + step) + Vec3::new(0.0, 0.0, 2.0),
        2 => board_coord(13 - step, 7) + Vec3::new(0.0, 0.0, 2.0),
        _ => board_coord(7, 13 - step) + Vec3::new(0.0, 0.0, 2.0),
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

fn play_sfx(commands: &mut Commands, handle: Handle<AudioSource>) {
    commands.spawn((
        AudioBundle {
            source: handle,
            settings: PlaybackSettings::DESPAWN,
        },
        SoundEffect,
    ));
}

fn return_to_title_screen(mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Title);
}

pub fn random_name() -> String {
    PLAYER_NAMES
        .iter()
        .choose(&mut rand::thread_rng())
        .unwrap_or(&"Player")
        .to_string()
}
