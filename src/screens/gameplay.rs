//! The screen state for the main gameplay.

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use rand::{seq::IteratorRandom, Rng};

use crate::{
    asset_tracking::LoadResource,
    audio::{Music, SoundEffect},
    screens::Screen,
};

const PLAYER_COLORS: [Color; 4] = [
    Color::srgb(0.95, 0.25, 0.25),
    Color::srgb(0.2, 0.8, 0.35),
    Color::srgb(0.2, 0.45, 0.95),
    Color::srgb(0.98, 0.85, 0.2),
];
const START_INDICES: [u8; 4] = [0, 13, 26, 39];
const PLAYER_NAMES: [&str; 12] = [
    "Alex", "Sam", "Jordan", "Taylor", "Morgan", "Riley", "Casey", "Sky", "Avery", "Kai", "Nova",
    "Jules",
];

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MatchSetup>();
    app.init_resource::<LastMatchResult>();
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
            run_bot_turn,
            sync_token_targets,
            animate_token_transforms,
            update_status_text,
            move_to_win_screen,
            return_to_title_screen
                .run_if(input_just_pressed(KeyCode::Escape))
                .run_if(in_state(Screen::Gameplay)),
        )
            .run_if(in_state(Screen::Gameplay)),
    );
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
        winner_order: Vec::new(),
        message: "Roll the die (Space). Then pick token 1-4.".into(),
        bot_timer: Timer::from_seconds(0.65, TimerMode::Repeating),
        finished: false,
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
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgb(0.08, 0.08, 0.12),
                    custom_size: Some(Vec2::new(860.0, 860.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, -20.0),
                ..default()
            });

            parent.spawn((
                Text2dBundle {
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font_size: 24.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    transform: Transform::from_xyz(0.0, 350.0, 12.0),
                    ..default()
                },
                StatusText,
            ));

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

            for p in track_points() {
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgb(0.2, 0.2, 0.24),
                        custom_size: Some(Vec2::splat(27.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(p),
                    ..default()
                });
            }

            for (player, color) in PLAYER_COLORS.into_iter().enumerate() {
                for token in 0..4 {
                    let start = yard_position(player, token);
                    parent.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::splat(20.0)),
                                ..default()
                            },
                            transform: Transform::from_translation(start),
                            ..default()
                        },
                        TokenVisual {
                            player,
                            token,
                            target: start,
                        },
                    ));
                }
            }
        });
}

fn handle_roll_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
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
        roll_for_current_player(&mut game);
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
                apply_move(&mut game, idx, &mut commands, &sfx);
            }
            break;
        }
    }
}

fn run_bot_turn(
    time: Res<Time>,
    mut commands: Commands,
    mut game: ResMut<LudoGame>,
    sfx: Res<GameplaySfx>,
) {
    if game_over(&game) || game.players[game.current].kind != PlayerKind::Bot {
        return;
    }

    game.bot_timer.tick(time.delta());
    if !game.bot_timer.finished() {
        return;
    }

    if game.last_roll.is_none() {
        play_sfx(&mut commands, sfx.roll.clone());
        roll_for_current_player(&mut game);
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

fn roll_for_current_player(game: &mut LudoGame) {
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
            return;
        }
    }

    game.selectable_tokens = legal_moves(game, game.current, roll);
    if game.selectable_tokens.is_empty() {
        game.message = format!(
            "{} rolled {roll} and has no legal moves.",
            game.players[game.current].name
        );
        game.last_roll = None;
        end_turn(game, roll == 6);
        return;
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

    if capture_happened {
        play_sfx(commands, sfx.capture.clone());
    }

    if player_finished(&game.players[player]) && !game.winner_order.contains(&player) {
        game.winner_order.push(player);
        play_sfx(commands, sfx.win.clone());
        game.message = format!(
            "{} finished at place {}!",
            game.players[player].name,
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

fn sync_token_targets(game: Res<LudoGame>, mut query: Query<&mut TokenVisual>) {
    if !game.is_changed() {
        return;
    }

    for mut token_visual in &mut query {
        let player = token_visual.player;
        let token = token_visual.token;
        let state = game.players[player].tokens[token];
        token_visual.target = match state {
            TokenState::Yard => yard_position(player, token),
            TokenState::Path(path) if path <= 50 => {
                let track = absolute_track_index(player, path);
                track_points()[track as usize] + Vec3::new((token as f32 - 1.5) * 4.0, 0.0, 2.0)
            }
            TokenState::Path(path) => home_position(player, path),
        };
    }
}

fn animate_token_transforms(time: Res<Time>, mut query: Query<(&TokenVisual, &mut Transform)>) {
    let speed = (time.delta_seconds() * 9.0).clamp(0.0, 1.0);
    for (token, mut transform) in &mut query {
        transform.translation = transform.translation.lerp(token.target, speed);
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
        "{}\nTurn: {} ({}) | Last roll: {} | Winners: {}\nControls: Space=Roll, 1-4=Move token, Esc=Title",
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
    game.players[target_player]
        .tokens
        .iter()
        .any(|t| matches!(t, TokenState::Path(p) if *p<=50 && absolute_track_index(target_player,*p)==target))
}

fn track_points() -> Vec<Vec3> {
    (0..52)
        .map(|i| {
            let angle = (i as f32 / 52.0) * std::f32::consts::TAU;
            Vec3::new(angle.cos() * 280.0, angle.sin() * 280.0, 0.0)
        })
        .collect()
}

fn yard_base(player: usize) -> Vec2 {
    [
        Vec2::new(-260.0, -220.0),
        Vec2::new(260.0, -220.0),
        Vec2::new(260.0, 220.0),
        Vec2::new(-260.0, 220.0),
    ][player]
}

fn yard_position(player: usize, token: usize) -> Vec3 {
    let offsets = [
        Vec2::new(-30.0, -30.0),
        Vec2::new(30.0, -30.0),
        Vec2::new(-30.0, 30.0),
        Vec2::new(30.0, 30.0),
    ];
    (yard_base(player) + offsets[token]).extend(3.0)
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
