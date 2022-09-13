use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};
use iyes_loopless::prelude::*;

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
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    MainMenu,
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
    Scoring,
    Physics,
}

#[derive(Debug, Component)]
struct Paddle {
    speed: f32,
    direction: Vec2,
}

#[derive(Component)]
struct Ball {
    velocity: Vec3,
}

struct AiHandicap {
    view_percentage: f32,
}

struct Config {
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

/// Despawn all entities with a given component type
fn despawn_with<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn setup(mut commands: Commands, config: Res<Config>, windows: Res<Windows>) {
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
            let menu_spacing = 20.;
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
            commands.insert_resource(NextState(GameState::Ingame));
        }
    }
}

fn setup_game(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
    gametype: Res<NextState<GameType>>,
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
                    speed: paddle_speed,
                    direction: Vec2::new(0., 0.),
                })
                .insert(PlayerController)
                .insert(LeftPlayer);

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
                        speed: paddle_speed,
                        direction: Vec2::new(0., 0.),
                    })
                    .insert(PlayerController)
                    .insert(RightPlayer);
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
    config: Res<Config>,
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
    config: Res<Config>,
    keyboard_input: Res<Input<KeyCode>>,
    mut paddle_query: Query<(&Paddle, &mut Transform), (With<T>, With<PlayerController>)>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<PlayerController>>,
) {
    if paddle_query.is_empty() {
        return;
    }

    let (_paddle, paddle_transform) = paddle_query.single_mut();
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

    if keyboard_input.pressed(KeyCode::Space) {
        commands.insert_resource(NextState(PongState::Playing));
        if keyboard_input.pressed(KeyCode::Up) {
            ball.velocity = config.ball_speed * Vec3::new(bounce_direction, 1., 0.).normalize();
        } else if keyboard_input.pressed(KeyCode::Down) {
            ball.velocity = config.ball_speed * Vec3::new(bounce_direction, -1., 0.).normalize();
        } else {
            ball.velocity = config.ball_speed * Vec3::new(bounce_direction, 0., 0.).normalize();
        }
    }
}

fn ai_serve<T: CourtSide + Component>(
    mut commands: Commands,
    config: Res<Config>,
    mut paddle_query: Query<(&mut Paddle, &Transform), (With<T>, With<AiController>)>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<AiController>>,
) {
    if paddle_query.is_empty() {
        return;
    }

    let (mut paddle, paddle_transform) = paddle_query.single_mut();
    if rand::random() {
        paddle.direction.y = 1.0;
    } else {
        paddle.direction.y = -1.0;
    }
    let (mut ball, mut ball_transform) = ball_query.single_mut();
    ball_transform.translation.x = config.court_size[0] / 2. * config.players_distance_percentage
        + config.court_size[0] / 2. * 0.2;
    ball_transform.translation.y = paddle_transform.translation.y;
    if rand::random() {
        commands.insert_resource(NextState(PongState::Playing));
        ball.velocity = config.ball_speed * Vec3::new(-1., 0., 0.).normalize();
    }
}

fn keyboard_movement_input(
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
) {
    if !left_paddle_query.is_empty() {
        let mut left_paddle = left_paddle_query.single_mut();
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Comma) {
            direction += 1.0;
        }
        if keyboard_input.pressed(KeyCode::O) {
            direction -= 1.0;
        }

        left_paddle.direction.y = direction;
    }

    if !right_paddle_query.is_empty() {
        let mut right_paddle = right_paddle_query.single_mut();
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Up) {
            direction += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            direction -= 1.0;
        }

        right_paddle.direction.y = direction;
    }
}

fn ai_movement_input(
    mut paddle_query: Query<(&mut Paddle, &Transform, &Sprite), With<AiController>>,
    ball_query: Query<(&Ball, &Transform), Without<AiController>>,
    config: Res<Config>,
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
        if ball_transform.translation.y > paddle_transform.translation.y {
            direction += 1.0;
        }
        if ball_transform.translation.y < paddle_transform.translation.y {
            direction -= 1.0;
        }
    }

    paddle.direction.y = direction;
}

fn paddle_movement(
    time: Res<Time>,
    mut paddle_query: Query<(&mut Paddle, &mut Transform, &Sprite)>,
    config: Res<Config>,
) {
    // clamp the timestep to stop the ball from escaping when the game starts
    let delta_seconds = f32::min(0.2, time.delta_seconds());
    let half_court_height = config.court_size[1] / 2.0;

    for (mut paddle, mut transform, sprite) in &mut paddle_query {
        let paddle_half_height = sprite
            .custom_size
            .expect("Sprite should have custom height")
            .y
            / 2.0;

        let translation = &mut transform.translation;
        translation.y += delta_seconds * paddle.direction.y * paddle.speed;
        translation.y = translation.y.clamp(
            -half_court_height + paddle_half_height,
            half_court_height - paddle_half_height,
        );

        paddle.direction = Vec2::new(0., 0.);
    }
}

fn ball_movement(time: Res<Time>, mut ball_query: Query<(&Ball, &mut Transform)>) {
    // clamp the timestep to stop the ball from escaping when the game starts
    let delta_seconds = f32::min(0.2, time.delta_seconds());

    let (ball, mut transform) = ball_query.single_mut();
    transform.translation += ball.velocity * delta_seconds;
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

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            fit_canvas_to_parent: true,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Score { player: 0, ai: 0 })
        .insert_resource(Config {
            paddle_speed: 1000.,
            ball_speed: 1000.,
            court_size: [1600., 1000.],
            players_distance_percentage: 0.4,
            ai_handicap: AiHandicap {
                view_percentage: 0.5,
            },
        })
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
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Serve(Side::Left))
                .label(GameloopStage::Input)
                .with_system(player_serve::<LeftPlayer>)
                .with_system(ai_serve::<LeftPlayer>.run_in_state(GameType::PlayerVsAi))
                .with_system(keyboard_movement_input)
                .into(),
        )
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Serve(Side::Right))
                .label(GameloopStage::Input)
                .with_system(player_serve::<RightPlayer>)
                .with_system(ai_serve::<RightPlayer>.run_in_state(GameType::PlayerVsAi))
                .with_system(keyboard_movement_input)
                .into(),
        )
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Playing)
                .label(GameloopStage::Input)
                .with_system(keyboard_movement_input)
                .with_system(ai_movement_input.run_in_state(GameType::PlayerVsAi))
                .into(),
        )
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Ingame)
                .label(GameloopStage::Physics)
                .after(GameloopStage::Input)
                .with_system(ball_movement)
                .with_system(paddle_movement)
                .into(),
        )
        .add_system(
            ball_collision
                .run_in_state(GameState::Ingame)
                .after(GameloopStage::Physics),
        )
        .add_system_set(
            ConditionSet::new()
                .run_in_state(GameState::Ingame)
                .run_in_state(PongState::Playing)
                .label(GameloopStage::Scoring)
                .after(GameloopStage::Physics)
                .with_system(ball_scoring)
                .into(),
        )
        .add_system(
            scoreboard
                .run_in_state(GameState::Ingame)
                .after(GameloopStage::Scoring),
        )
        .run();
}
