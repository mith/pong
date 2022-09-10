use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};

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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum PongStage {
    Serve(Controller),
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

struct Scoreboard {
    player: usize,
    ai: usize,
}

fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
) {
    let paddle_speed = config.paddle_speed;

    let window = windows.primary();

    let height_ratio = window.height() / config.court_size[0];
    if !window.is_focused() {
        info!("Window is not focused!");
    } else {
        info!("Window is focused.")
    }

    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scale: height_ratio,
            ..default()
        },
        ..default()
    });

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
                .insert(PlayerController);

            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(player_distance, 0.0, 1.0)),
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
                .insert(AiController);

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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            color: UiColor(Color::NONE),
            ..default()
        })
        .with_children(|root| {
            root.spawn_bundle(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(50.0), Val::Percent(100.0)),
                    ..default()
                },
                color: UiColor(Color::NONE),
                ..default()
            })
            .with_children(|parent| {
                parent
                    .spawn_bundle(
                        TextBundle::from_sections([TextSection::new("0", text_style.clone())])
                            .with_style(Style {
                                position_type: PositionType::Absolute,
                                position: UiRect {
                                    top: Val::Px(16.0),
                                    left: Val::Px(4.0),
                                    ..default()
                                },
                                ..default()
                            }),
                    )
                    .insert(PlayerController);
                parent
                    .spawn_bundle(
                        TextBundle::from_sections([TextSection::new("0", text_style.clone())])
                            .with_style(Style {
                                align_content: AlignContent::FlexEnd,
                                position_type: PositionType::Absolute,
                                position: UiRect {
                                    top: Val::Px(16.0),
                                    right: Val::Px(4.0),
                                    ..default()
                                },
                                ..default()
                            }),
                    )
                    .insert(AiController);
            });
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

fn player_serve(
    config: Res<Config>,
    mut state: ResMut<State<PongStage>>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform), With<PlayerController>>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<PlayerController>>,
) {
    let (_paddle, paddle_transform) = query.single_mut();
    let (mut ball, mut ball_transform) = ball_query.single_mut();

    ball_transform.translation.x = -(config.court_size[0] / 2.
        * config.players_distance_percentage
        + config.court_size[0] / 2. * 0.2);
    ball_transform.translation.y = paddle_transform.translation.y;

    if keyboard_input.pressed(KeyCode::Space) {
        state.set(PongStage::Playing).unwrap();
        if keyboard_input.pressed(KeyCode::Up) {
            ball.velocity = config.ball_speed * Vec3::new(1., 1., 0.).normalize();
        } else if keyboard_input.pressed(KeyCode::Down) {
            ball.velocity = config.ball_speed * Vec3::new(1., -1., 0.).normalize();
        } else {
            ball.velocity = config.ball_speed * Vec3::new(1., 0., 0.).normalize();
        }
    }
}

fn ai_serve(
    config: Res<Config>,
    mut state: ResMut<State<PongStage>>,
    mut query: Query<(&mut Paddle, &Transform), With<AiController>>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<AiController>>,
) {
    let (mut paddle, paddle_transform) = query.single_mut();
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
        state.set(PongStage::Playing).unwrap();
        ball.velocity = config.ball_speed * Vec3::new(-1., 0., 0.).normalize();
    }
}

fn keyboard_movement_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Paddle, With<PlayerController>>,
) {
    let mut paddle = query.single_mut();
    let mut direction = 0.0;
    if keyboard_input.pressed(KeyCode::Up) {
        direction += 1.0;
    }
    if keyboard_input.pressed(KeyCode::Down) {
        direction -= 1.0;
    }

    paddle.direction.y = direction;
}

fn ai_movement_input(
    mut paddle_query: Query<(&mut Paddle, &Transform), With<AiController>>,
    ball_query: Query<(&Ball, &Transform), Without<AiController>>,
    config: Res<Config>,
) {
    let court_width = config.court_size[0];
    let view_distance_px = court_width * config.ai_handicap.view_percentage;
    let (mut paddle, paddle_transform) = paddle_query.single_mut();
    let mut direction = 0.0;
    let (ball, ball_transform) = ball_query.single();
    if ball.velocity.x < 0.0
        || (ball_transform.translation.x - paddle_transform.translation.x).abs() > view_distance_px
    {
        return;
    }

    if (ball_transform.translation.y - paddle_transform.translation.y).abs() > 10.0 {
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
                    let angle = ball_transform
                        .translation
                        .angle_between(paddle_transform.translation);
                    let mut force = angle * 3000.0;
                    if paddle_transform.translation.y > ball_transform.translation.y {
                        force = -force;
                    }
                    match collision {
                        Collision::Left => {
                            velocity.x = -velocity.x.abs();
                            velocity.y = force;
                        }
                        Collision::Right => {
                            velocity.x = velocity.x.abs();
                            velocity.y = force;
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
    mut state: ResMut<State<PongStage>>,
    ball_query: Query<(&Ball, &Transform, &Sprite), Without<Court>>,
    mut scoreboard: ResMut<Scoreboard>,
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
                state.set(PongStage::Serve(Controller::Player)).unwrap();
            }
            Collision::Right => {
                scoreboard.player += 1;
                state.set(PongStage::Serve(Controller::AI)).unwrap();
            }
            _ => (),
        }
    }
}

fn scoreboard(
    scoreboard: ResMut<Scoreboard>,
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
            canvas: Some("canvas".into()),
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Scoreboard { player: 0, ai: 0 })
        .insert_resource(Config {
            paddle_speed: 1000.,
            ball_speed: 1000.,
            court_size: [1600., 1000.],
            players_distance_percentage: 0.4,
            ai_handicap: AiHandicap {
                view_percentage: 0.5,
            },
        })
        .add_state(PongStage::Serve(Controller::Player))
        .add_startup_system(setup)
        .add_system(adjust_scale)
        .add_system_set(
            SystemSet::on_update(PongStage::Serve(Controller::Player))
                .with_system(player_serve.before(paddle_movement))
                .with_system(keyboard_movement_input.before(paddle_movement))
                .with_system(paddle_movement),
        )
        .add_system_set(
            SystemSet::on_update(PongStage::Serve(Controller::AI))
                .with_system(ai_serve.before(paddle_movement))
                .with_system(keyboard_movement_input.before(paddle_movement))
                .with_system(paddle_movement),
        )
        .add_system_set(
            SystemSet::on_update(PongStage::Playing)
                .label(GameloopStage::Input)
                .with_system(keyboard_movement_input)
                .with_system(ai_movement_input),
        )
        .add_system_set(
            SystemSet::on_update(PongStage::Playing)
                .label(GameloopStage::Physics)
                .after(GameloopStage::Input)
                .with_system(ball_movement.before(ball_collision))
                .with_system(paddle_movement.before(ball_collision))
                .with_system(ball_collision),
        )
        .add_system_set(
            SystemSet::on_update(PongStage::Playing)
                .label(GameloopStage::Scoring)
                .after(GameloopStage::Physics)
                .with_system(ball_scoring)
                .with_system(scoreboard.after(ball_scoring)),
        )
        .run();
}
