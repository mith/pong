use bevy::{
    ecs::schedule::ShouldRun,
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    tasks::IoTaskPool,
};
use bevy_ggrs::{GGRSPlugin, Rollback, RollbackIdProvider, SessionType};
use bytemuck::{Pod, Zeroable};
use ggrs::{Config, InputStatus, P2PSession, PlayerHandle, SessionBuilder};
use iyes_loopless::prelude::*;
use matchbox_socket::WebRtcSocket;
use std::time::Duration;

#[derive(Component)]
struct Court;

#[derive(Component)]
struct PlayerController;

#[derive(Component)]
struct AiController;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Controller {
    Player,
    AI,
}

trait CourtSide {}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum Side {
    Left,
    Right,
}

#[derive(Component)]
struct LeftPlayer;

impl CourtSide for LeftPlayer {}

#[derive(Component)]
struct RightPlayer;

impl CourtSide for RightPlayer {}

#[derive(Component)]
struct MainMenu;

#[derive(Component)]
struct Scoreboard;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Component)]
enum GameType {
    PlayerVsAi,
    PlayerVsPlayer,
    Online,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    MainMenu,
    Lobby,
    Ingame,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum PongState {
    Serve(Side),
    Playing,
}

#[derive(SystemLabel)]
enum GameloopStage {
    Input,
    Physics,
    Scoring,
    Movement,
    Collision,
}

#[derive(Debug, Component)]
struct Paddle {
    handle: usize,
    speed: f32,
    direction: Vec2,
}

#[derive(Component, Reflect, Default)]
struct Ball {
    velocity: Vec3,
}

struct AiHandicap {
    view_percentage: f32,
}

struct PongConfig {
    court_size: [f32; 2],
    players_distance_percentage: f32,
    paddle_speed: f32,
    ball_speed: f32,
    ai_handicap: AiHandicap,
}

struct Score {
    player: usize,
    ai: usize,
}

const ROLLBACK_DEFAULT: &str = "rollback_default";

/// Despawn all entities with a given component type
fn despawn_with<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn setup(mut commands: Commands, config: Res<PongConfig>, windows: Res<Windows>) {
    let window = windows.primary();

    let height_ratio = window.height() / config.court_size[0];
    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scale: height_ratio,
            ..default()
        },
        ..default()
    });
}

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PublicPixel-z84yD.ttf");
    let text_style = TextStyle {
        font,
        font_size: 32.,
        ..default()
    };

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::ColumnReverse,
                ..default()
            },
            color: UiColor(Color::NONE),
            ..default()
        })
        .insert(MainMenu)
        .with_children(|parent| {
            let menu_spacing = 40.;
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        // size: Size::new(Val::Px(150.), Val::Px(50.)),
                        margin: UiRect::all(Val::Px(menu_spacing)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::BLACK.into(),
                    ..default()
                })
                .insert(GameType::PlayerVsAi)
                .with_children(|button| {
                    button.spawn_bundle(TextBundle::from_section("1 Player", text_style.clone()));
                });

            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        margin: UiRect::all(Val::Px(menu_spacing)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::BLACK.into(),
                    ..default()
                })
                .insert(GameType::PlayerVsPlayer)
                .with_children(|button| {
                    button.spawn_bundle(TextBundle::from_section("2 Players", text_style.clone()));
                });
            parent
                .spawn_bundle(ButtonBundle {
                    style: Style {
                        margin: UiRect::all(Val::Px(menu_spacing)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::BLACK.into(),
                    ..default()
                })
                .insert(GameType::Online)
                .with_children(|button| {
                    button.spawn_bundle(TextBundle::from_section("Online", text_style.clone()));
                });
        });
}

fn gametype_button(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, &GameType), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, gametype) in &interaction_query {
        if Interaction::Clicked == *interaction {
            info!("Gametype picked: {:?}", gametype);
            commands.insert_resource(NextState(gametype.clone()));
            if *gametype == GameType::Online {
                commands.insert_resource(NextState(GameState::Lobby));
            } else {
                commands.insert_resource(NextState(GameState::Ingame));
            }
        }
    }
}

fn setup_game(
    mut commands: Commands,
    config: Res<PongConfig>,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
    gametype: Res<CurrentState<GameType>>,
    mut rip: ResMut<RollbackIdProvider>,
) {
    let paddle_speed = config.paddle_speed;

    let window = windows.primary();

    if !window.is_focused() {
        info!("Window is not focused!");
    } else {
        info!("Window is focused.")
    }

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::BLACK,
                custom_size: Some(Vec2::from_array(config.court_size)),
                ..default()
            },
            ..default()
        })
        .insert(Court)
        .with_children(|parent| {
            let num_dashes = (config.court_size[1] / 30.0) as i32;
            for y in 0..num_dashes {
                parent.spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(
                        0.0,
                        y as f32 * 30.0 - config.court_size[1] / 2.0 + 20.0,
                        1.0,
                    )),
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(5.0, 12.0)),
                        ..default()
                    },
                    ..default()
                });
            }

            let paddle_size = Vec2::new(17.0, 80.0);
            let player_distance = config.court_size[0] * config.players_distance_percentage;
            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(-player_distance, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(paddle_size),
                        ..default()
                    },
                    ..default()
                })
                .insert(Paddle {
                    handle: 0,
                    speed: paddle_speed,
                    direction: Vec2::new(0., 0.),
                })
                .insert(PlayerController)
                .insert(LeftPlayer)
                .insert(Rollback::new(rip.next_id()));

            dbg!(gametype.clone());
            if gametype.0 == GameType::PlayerVsAi {
                info!("Spawning AI controlled right paddle");
                parent
                    .spawn_bundle(SpriteBundle {
                        transform: Transform::from_translation(Vec3::new(
                            player_distance,
                            0.0,
                            1.0,
                        )),
                        sprite: Sprite {
                            color: Color::rgb(1.0, 1.0, 1.0),
                            custom_size: Some(paddle_size),
                            ..default()
                        },
                        ..default()
                    })
                    .insert(Paddle {
                        handle: 1,
                        speed: paddle_speed,
                        direction: Vec2::new(0., 0.),
                    })
                    .insert(AiController)
                    .insert(RightPlayer);
            } else {
                // the borrow checker freaks out if I put the common part here in a variable, so
                // just repeat this for now
                info!("Spawning player controlled right paddle");
                parent
                    .spawn_bundle(SpriteBundle {
                        transform: Transform::from_translation(Vec3::new(
                            player_distance,
                            0.0,
                            1.0,
                        )),
                        sprite: Sprite {
                            color: Color::rgb(1.0, 1.0, 1.0),
                            custom_size: Some(paddle_size),
                            ..default()
                        },
                        ..default()
                    })
                    .insert(Paddle {
                        handle: 1,
                        speed: paddle_speed,
                        direction: Vec2::new(0., 0.),
                    })
                    .insert(PlayerController)
                    .insert(RightPlayer)
                    .insert(Rollback::new(rip.next_id()));
            }

            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(20.0, 20.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(Ball {
                    velocity: Vec3::new(0.0, 0.0, 0.0),
                });
        });

    let font = asset_server.load("fonts/PublicPixel-z84yD.ttf");
    let text_style = TextStyle {
        font,
        font_size: window.height() * 0.15,
        ..default()
    };

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            color: UiColor(Color::NONE),
            ..default()
        })
        .insert(Scoreboard)
        .with_children(|parent| {
            parent
                .spawn_bundle(
                    TextBundle::from_section("0", text_style.clone()).with_style(Style {
                        // position_type: PositionType::Absolute,
                        align_content: AlignContent::FlexStart,
                        margin: UiRect {
                            right: Val::Percent(10.),
                            ..default()
                        },
                        ..default()
                    }),
                )
                .insert(PlayerController);
            parent
                .spawn_bundle(
                    TextBundle::from_section("0", text_style.clone()).with_style(Style {
                        align_content: AlignContent::FlexStart,
                        margin: UiRect {
                            left: Val::Percent(10.),
                            ..default()
                        },
                        ..default()
                    }),
                )
                .insert(AiController);
        });
}

fn adjust_scale(
    config: Res<PongConfig>,
    windows: Res<Windows>,
    mut projections: Query<&mut OrthographicProjection, With<Camera>>,
    mut player_score: Query<&mut Text, (With<PlayerController>, Without<AiController>)>,
    mut ai_score: Query<&mut Text, (With<AiController>, Without<PlayerController>)>,
) {
    let window = windows.primary();
    let height_ratio = config.court_size[0] / window.height();

    for mut ortho in projections.iter_mut() {
        ortho.scale = height_ratio * 0.8;
    }

    for mut score in &mut ai_score {
        score.sections[0].style.font_size = window.height() * 0.07;
    }

    for mut score in &mut player_score {
        score.sections[0].style.font_size = window.height() * 0.07;
    }
}

fn player_serve<T: CourtSide + Component>(
    mut commands: Commands,
    config: Res<PongConfig>,
    inputs: Res<Vec<PaddleInput>>,
    mut paddle_query: Query<(&Paddle, &mut Transform), (With<T>, With<PlayerController>)>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<PlayerController>>,
) {
    if paddle_query.is_empty() {
        return;
    }

    let (paddle, paddle_transform) = paddle_query.single_mut();
    let (mut ball, mut ball_transform) = ball_query.single_mut();

    ball_transform.translation.x = paddle_transform.translation.x * 0.8;
    ball_transform.translation.y = paddle_transform.translation.y;

    let bounce_direction = {
        if paddle_transform.translation.x.is_sign_positive() {
            -1.
        } else {
            1.
        }
    };

    let input = inputs[paddle.handle as usize];
    if input.serve {
        commands.insert_resource(NextState(PongState::Playing));
        if input.move_up && !input.move_down {
            ball.velocity = config.ball_speed * Vec3::new(bounce_direction, 1., 0.).normalize();
        } else if !input.move_up && input.move_down {
            ball.velocity = config.ball_speed * Vec3::new(bounce_direction, -1., 0.).normalize();
        } else {
            ball.velocity = config.ball_speed * Vec3::new(bounce_direction, 0., 0.).normalize();
        }
    }
}

fn ai_serve<T: CourtSide + Component>(
    mut commands: Commands,
    config: Res<PongConfig>,
    mut paddle_query: Query<(&mut Paddle, &Transform), (With<T>, With<AiController>)>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<AiController>>,
) {
    if paddle_query.is_empty() {
        return;
    }

    let (mut paddle, paddle_transform) = paddle_query.single_mut();
    let (mut ball, mut ball_transform) = ball_query.single_mut();
    ball_transform.translation.x = config.court_size[0] / 2. * config.players_distance_percentage
        + config.court_size[0] / 2. * 0.2;
    ball_transform.translation.y = paddle_transform.translation.y;

    commands.insert_resource(NextState(PongState::Playing));
    ball.velocity = config.ball_speed * Vec3::new(-1., 0., 0.).normalize();
}

fn keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut left_paddle_query: Query<
        &mut Paddle,
        (
            With<LeftPlayer>,
            Without<RightPlayer>,
            With<PlayerController>,
        ),
    >,
    mut right_paddle_query: Query<
        &mut Paddle,
        (
            With<RightPlayer>,
            Without<LeftPlayer>,
            With<PlayerController>,
        ),
    >,
    mut inputs: ResMut<Vec<PaddleInput>>,
    gametype: Res<CurrentState<GameType>>,
) {
    if !left_paddle_query.is_empty() {
        inputs[0].move_down = false;
        inputs[0].move_up = false;
        inputs[0].serve = false;

        let mut left_paddle = left_paddle_query.single_mut();
        if keyboard_input.pressed(KeyCode::W) {
            inputs[0].move_up = true;
        }
        if keyboard_input.pressed(KeyCode::S) {
            inputs[0].move_down = true;
        }
        if keyboard_input.pressed(KeyCode::Space) {
            inputs[0].serve = true;
        }

        if gametype.0 == GameType::PlayerVsAi {
            if keyboard_input.pressed(KeyCode::Up) {
                inputs[0].move_up = true;
            }
            if keyboard_input.pressed(KeyCode::Down) {
                inputs[0].move_down = true;
            }
        }
    }

    if !right_paddle_query.is_empty() {
        inputs[1].move_down = false;
        inputs[1].move_up = false;
        inputs[1].serve = false;

        let mut right_paddle = right_paddle_query.single_mut();
        if keyboard_input.pressed(KeyCode::Up) {
            inputs[1].move_up = true;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            inputs[1].move_down = true;
        }
        if keyboard_input.pressed(KeyCode::Space) {
            inputs[1].serve = true;
        }
    }
}

fn ai_input(
    mut paddle_query: Query<(&mut Paddle, &Transform, &Sprite), With<AiController>>,
    ball_query: Query<(&Ball, &Transform), Without<AiController>>,
    mut inputs: ResMut<Vec<PaddleInput>>,
    config: Res<PongConfig>,
) {
    let court_width = config.court_size[0];
    let (mut paddle, paddle_transform, paddle_sprite) = paddle_query.single_mut();
    let paddle_size = paddle_sprite
        .custom_size
        .expect("Paddle should have custom size");
    let view_distance_px = court_width * config.ai_handicap.view_percentage;
    let mut direction = 0.0;
    let (ball, ball_transform) = ball_query.single();
    if ball.velocity.x < 0.0
        || (ball_transform.translation.x - paddle_transform.translation.x).abs() > view_distance_px
    {
        return;
    }

    if (ball_transform.translation.y - paddle_transform.translation.y).abs() > paddle_size.y / 2. {
        let input = &mut inputs[1];
        input.move_down = false;
        input.move_up = false;
        input.serve = false;
        if ball_transform.translation.y > paddle_transform.translation.y {
            input.move_up = true;
        }
        if ball_transform.translation.y < paddle_transform.translation.y {
            input.move_down = true;
        }
    }

    paddle.direction.y = direction;
}

fn paddle_movement(
    mut paddle_query: Query<(&mut Paddle, &mut Transform, &Sprite)>,
    inputs: Res<Vec<PaddleInput>>,
    config: Res<PongConfig>,
) {
    let half_court_height = config.court_size[1] / 2.0;

    for (mut paddle, mut transform, sprite) in &mut paddle_query {
        let input = inputs[paddle.handle as usize];

        if input.move_up && !input.move_down {
            paddle.direction.y += 1.;
        }
        if !input.move_up && input.move_down {
            paddle.direction.y -= 1.;
        }

        let paddle_half_height = sprite
            .custom_size
            .expect("Sprite should have custom height")
            .y
            / 2.0;

        let translation = &mut transform.translation;

        translation.y += paddle.direction.y * paddle.speed;
        translation.y = translation.y.clamp(
            -half_court_height + paddle_half_height,
            half_court_height - paddle_half_height,
        );

        paddle.direction = Vec2::new(0., 0.);
    }
}

fn ball_movement(mut ball_query: Query<(&Ball, &mut Transform)>) {
    let (ball, mut transform) = ball_query.single_mut();
    transform.translation += ball.velocity;
}

fn ball_collision(
    mut ball_query: Query<(&mut Ball, &mut Transform, &Sprite), (Without<Court>, Without<Paddle>)>,
    court_collider_query: Query<(&Court, &Transform, &Sprite)>,
    paddle_collider_query: Query<(&Paddle, &Transform, &Sprite)>,
) {
    let (mut ball, mut ball_transform, sprite) = ball_query.single_mut();
    let ball_size = sprite.custom_size.expect("Ball should have custom size");
    let velocity = &mut ball.velocity;
    let (_court, court_transform, court_sprite) = court_collider_query.single();
    let other_size = court_sprite
        .custom_size
        .expect("Collider should have custom size");

    // Sometimes the ball clips through a wall, so we clamp the position to within the
    // court bounds
    let half_size = other_size / 2.0;
    ball_transform.translation.x = ball_transform
        .translation
        .x
        .clamp(-half_size.x, half_size.x);
    ball_transform.translation.y = ball_transform
        .translation
        .y
        .clamp(-half_size.y, half_size.y);

    // check collision with court top and bottom
    let collision = collide(
        ball_transform.translation,
        ball_size,
        court_transform.translation,
        other_size,
    );

    if let Some(collision) = collision {
        match collision {
            Collision::Top => velocity.y = -velocity.y.abs(),
            Collision::Bottom => velocity.y = velocity.y.abs(),
            _ => (),
        }
    }

    // check collision with both paddles
    for (_paddle, paddle_transform, paddle_sprite) in &paddle_collider_query {
        let paddle_size = paddle_sprite
            .custom_size
            .expect("Paddle should have custom size");
        let collision = collide(
            ball_transform.translation,
            ball_size,
            paddle_transform.translation,
            paddle_size,
        );
        if let Some(collision) = collision {
            match collision {
                Collision::Left | Collision::Right => {
                    let paddle_ball_distance =
                        paddle_transform.translation.y - ball_transform.translation.y;
                    if paddle_ball_distance > (0.25 * paddle_size.y) {
                        *velocity = Vec3::new(1., -1., 0.).normalize() * velocity.length();
                    } else if paddle_ball_distance < -(0.25 * paddle_size.y) {
                        *velocity = Vec3::new(1., 1., 0.).normalize() * velocity.length();
                    } else {
                        *velocity = Vec3::new(1., 0., 0.).normalize() * velocity.length();
                    }
                    match collision {
                        Collision::Left => {
                            velocity.x = -velocity.x.abs();
                        }
                        Collision::Right => {
                            velocity.x = velocity.x.abs();
                        }
                        _ => (),
                    };
                }
                Collision::Top => velocity.y = velocity.y.abs(),
                Collision::Bottom => velocity.y = -velocity.y.abs(),
                _ => (),
            }
        }
    }
}

fn ball_scoring(
    mut commands: Commands,
    ball_query: Query<(&Ball, &Transform, &Sprite), Without<Court>>,
    mut scoreboard: ResMut<Score>,
    court_collider_query: Query<(&Court, &Transform, &Sprite), Without<Ball>>,
) {
    let (_ball, ball_transform, ball_sprite) = ball_query.single();
    let ball_size = ball_sprite
        .custom_size
        .expect("Collider should have custom size");
    let (_court, court_transform, court_sprite) = &court_collider_query.single();
    let court_size = court_sprite
        .custom_size
        .expect("Court should have custom size");
    let collision = collide(
        ball_transform.translation,
        ball_size,
        court_transform.translation,
        court_size,
    );

    if let Some(collision) = collision {
        match collision {
            Collision::Left => {
                scoreboard.ai += 1;
                commands.insert_resource(NextState(PongState::Serve(Side::Left)));
            }
            Collision::Right => {
                scoreboard.player += 1;
                commands.insert_resource(NextState(PongState::Serve(Side::Right)));
            }
            _ => (),
        }
    }
}

fn scoreboard(
    scoreboard: ResMut<Score>,
    mut player_scoreboard: Query<&mut Text, (With<PlayerController>, Without<AiController>)>,
    mut ai_scoreboard: Query<&mut Text, (With<AiController>, Without<PlayerController>)>,
) {
    let mut text = player_scoreboard.single_mut();
    text.sections[0].value = format!("{}", scoreboard.player);

    let mut text = ai_scoreboard.single_mut();
    text.sections[0].value = format!("{}", scoreboard.ai);
}

#[derive(Debug)]
pub struct GGRSConfig;
impl Config for GGRSConfig {
    type Input = BoxInput;
    type State = u8;
    type Address = String;
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Pod, Zeroable)]
pub struct BoxInput {
    pub inp: u8,
}

fn start_matchbox_socket(mut commands: Commands) {
    let room_url = "wss://pong-signalling-server.herokuapp.com/pong?next=2";
    info!("connecting to matchbox server: {:?}", room_url);
    let (socket, message_loop) = WebRtcSocket::new(room_url);

    // The message loop needs to be awaited, or nothing will happen.
    // We do this here using bevy's task system.
    let task_pool = IoTaskPool::get();
    task_pool.spawn(message_loop).detach();

    commands.insert_resource(Some(socket));
}

#[derive(Component)]
struct LobbyText;
#[derive(Component)]
struct LobbyUI;

fn setup_lobby(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.), Val::Percent(100.)),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(TextBundle {
                    style: Style {
                        align_self: AlignSelf::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    text: Text::from_section(
                        "Entering lobby...",
                        TextStyle {
                            font: asset_server.load("fonts/PublicPixel-z84yD.ttf"),
                            font_size: 40.,
                            color: Color::BLACK,
                        },
                    ),
                    ..default()
                })
                .insert(LobbyText);
        })
        .insert(LobbyUI);
}

fn lobby_cleanup(query: Query<Entity, With<LobbyUI>>, mut commands: Commands) {
    for e in query.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn lobby(
    mut commands: Commands,
    mut socket: ResMut<Option<WebRtcSocket>>,
    mut text_query: Query<&mut Text, With<LobbyText>>,
) {
    let socket = socket.as_mut();

    socket.as_mut().unwrap().accept_new_connections().len();

    text_query.single_mut().sections[0].value = format!("Waiting for another player...");

    let num_connected = socket.as_ref().unwrap().connected_peers().len();
    if num_connected == 0 {
        return;
    }

    info!("Enough players in lobby, starting game.");

    let socket = socket.take().unwrap();

    let players = socket.players();

    let max_prediction = 12;

    // create a GGRS P2P session
    let mut sess_build = SessionBuilder::<GGRSConfig>::new()
        .with_num_players(2)
        .with_max_prediction_window(max_prediction)
        .with_input_delay(2)
        .with_fps(60)
        .expect("Invalid fps.");

    for (i, player) in players.into_iter().enumerate() {
        sess_build = sess_build
            .add_player(player, i)
            .expect("Failed to add player.");
    }

    // start the GGRS session
    let sess = sess_build
        .start_p2p_session(socket)
        .expect("Failed to add player.");

    commands.insert_resource(sess);
    commands.insert_resource(SessionType::P2PSession);

    commands.insert_resource(NextState(GameState::Ingame));
}

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_SERVE: u8 = 1 << 2;

fn input(_handle: In<PlayerHandle>, keyboard_input: Res<Input<KeyCode>>) -> BoxInput {
    let mut input: u8 = 0;

    if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
        input |= INPUT_UP;
    }
    if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
        input |= INPUT_DOWN;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        input |= INPUT_SERVE;
    }

    BoxInput { inp: input }
}

#[derive(Default, Copy, Clone)]
struct PaddleInput {
    move_up: bool,
    move_down: bool,
    serve: bool,
}

fn box_input_to_paddle_input(
    box_inputs: Res<Vec<(BoxInput, InputStatus)>>,
    mut paddle_inputs: ResMut<Vec<PaddleInput>>,
) {
    for i in 0..2 {
        let input = box_inputs[i].0.inp;
        paddle_inputs[i] = PaddleInput {
            move_up: input & INPUT_UP != 0,
            move_down: input & INPUT_DOWN != 0,
            serve: input & INPUT_SERVE != 0,
        }
    }
}

fn log_ggrs_events(mut session: ResMut<P2PSession<GGRSConfig>>) {
    for event in session.events() {
        info!("GGRS Event: {:?}", event);
    }
}

fn main() {
    let mut app = App::new();

    GGRSPlugin::<GGRSConfig>::new()
        .with_update_frequency(60)
        .with_input_system(input)
        .register_rollback_type::<Transform>()
        .register_rollback_type::<Ball>()
        .with_rollback_schedule(
            Schedule::default().with_stage(
                ROLLBACK_DEFAULT,
                SystemStage::parallel()
                    .with_system(box_input_to_paddle_input.before(GameloopStage::Input))
                    .with_system(
                        player_serve::<LeftPlayer>
                            .run_in_state(GameState::Ingame)
                            .run_in_state(PongState::Serve(Side::Left))
                            .label(GameloopStage::Input),
                    )
                    .with_system(
                        player_serve::<RightPlayer>
                            .run_in_state(GameState::Ingame)
                            .run_in_state(PongState::Serve(Side::Right))
                            .label(GameloopStage::Input),
                    )
                    .with_system(
                        paddle_movement
                            .run_in_state(GameState::Ingame)
                            .after(GameloopStage::Input)
                            .label(GameloopStage::Movement),
                    )
                    .with_system(
                        ball_movement
                            .run_in_state(GameState::Ingame)
                            .run_in_state(PongState::Playing)
                            .after(GameloopStage::Input)
                            .label(GameloopStage::Movement),
                    )
                    .with_system(
                        ball_collision
                            .run_in_state(GameState::Ingame)
                            .run_in_state(PongState::Playing)
                            .after(GameloopStage::Movement)
                            .label(GameloopStage::Collision),
                    )
                    .with_system(
                        ball_scoring
                            .run_in_state(GameState::Ingame)
                            .run_in_state(PongState::Playing)
                            .after(GameloopStage::Collision)
                            .label(GameloopStage::Scoring),
                    )
                    .with_system(
                        scoreboard
                            .run_in_state(GameState::Ingame)
                            .after(GameloopStage::Scoring),
                    ),
            ),
        )
        .build(&mut app);

    let mut fixed_timestep = SystemStage::parallel()
        .with_system_set(
            ConditionSet::new()
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .label(GameloopStage::Input)
                .with_system(keyboard_input)
                .with_system(ai_input.run_in_state(GameType::PlayerVsAi))
                .into(),
        )
        .with_system_set(
            ConditionSet::new()
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Serve(Side::Left))
                .after(GameloopStage::Input)
                .with_system(player_serve::<LeftPlayer>)
                .with_system(ai_serve::<LeftPlayer>.run_in_state(GameType::PlayerVsAi))
                .into(),
        )
        .with_system_set(
            ConditionSet::new()
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Serve(Side::Right))
                .after(GameloopStage::Input)
                .with_system(player_serve::<RightPlayer>)
                .with_system(ai_serve::<RightPlayer>.run_in_state(GameType::PlayerVsAi))
                .into(),
        )
        .with_system(
            paddle_movement
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .after(GameloopStage::Input)
                .label(GameloopStage::Movement),
        )
        .with_system(
            ball_movement
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Playing)
                .after(GameloopStage::Input)
                .label(GameloopStage::Movement),
        )
        .with_system(
            ball_collision
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Playing)
                .after(GameloopStage::Movement)
                .label(GameloopStage::Collision),
        )
        .with_system(
            ball_scoring
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Playing)
                .after(GameloopStage::Collision)
                .label(GameloopStage::Scoring),
        )
        .with_system(
            scoreboard
                .run_not_in_state(GameType::Online)
                .run_in_state(GameState::Ingame)
                .after(GameloopStage::Scoring),
        );

    app.insert_resource(WindowDescriptor {
        fit_canvas_to_parent: true,
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(Score { player: 0, ai: 0 })
    .insert_resource(PongConfig {
        paddle_speed: 10.,
        ball_speed: 20.,
        court_size: [1600., 1000.],
        players_distance_percentage: 0.4,
        ai_handicap: AiHandicap {
            view_percentage: 0.5,
        },
    })
    .insert_resource(vec![
        PaddleInput { ..default() },
        PaddleInput { ..default() },
    ])
    .add_loopless_state(GameState::MainMenu)
    .add_loopless_state(GameType::PlayerVsAi)
    .add_loopless_state(PongState::Serve(Side::Left))
    .add_startup_system(setup)
    .add_enter_system(GameState::MainMenu, setup_menu)
    .add_enter_system(GameState::Ingame, setup_game)
    .add_exit_system(GameState::MainMenu, despawn_with::<MainMenu>)
    .add_exit_system(GameState::Ingame, despawn_with::<Court>)
    .add_exit_system(GameState::Ingame, despawn_with::<Scoreboard>)
    .add_system(gametype_button.run_in_state(GameState::MainMenu))
    .add_system(adjust_scale.run_in_state(GameState::Ingame))
    .add_stage_before(
        CoreStage::Update,
        "fixed_timestep",
        FixedTimestepStage::new(Duration::from_millis(30)).with_stage(fixed_timestep),
    )
    .add_enter_system_set(
        GameState::Lobby,
        ConditionSet::new()
            .with_system(start_matchbox_socket)
            .with_system(setup_lobby)
            .into(),
    )
    .add_exit_system(GameState::Lobby, lobby_cleanup)
    .add_system(lobby.run_in_state(GameState::Lobby))
    .add_system_set(
        ConditionSet::new()
            .run_in_state(GameType::Online)
            .run_in_state(GameState::Ingame)
            .with_system(log_ggrs_events)
            .into(),
    )
    .run();
}
