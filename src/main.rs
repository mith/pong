use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};

#[derive(Component)]
struct PlayerController;

#[derive(Component)]
struct AiController;

#[derive(Debug, Component)]
struct Paddle {
    speed: f32,
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
    ai_handicap: AiHandicap,
}

struct Scoreboard {
    player: usize,
    ai: usize,
}

#[derive(SystemLabel)]
enum GameloopStages {
    Input,
    Physics,
    Scoring,
}

#[derive(Component)]
struct Court;

fn setup(mut commands: Commands, config: Res<Config>, asset_server: Res<AssetServer>) {
    let paddle_speed = config.paddle_speed;

    commands.spawn_bundle(Camera2dBundle::default());

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.3, 0.3, 0.3),
                custom_size: Some(Vec2::from_array(config.court_size)),
                ..default()
            },
            ..default()
        })
        .insert(Court)
        .with_children(|parent| {
            let player_distance = config.court_size[0] * config.players_distance_percentage;
            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(-player_distance, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::rgb(0.5, 0.5, 1.0),
                        custom_size: Some(Vec2::new(20.0, 130.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(Paddle {
                    speed: paddle_speed,
                })
                .insert(PlayerController);

            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(player_distance, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::rgb(0.5, 0.5, 1.0),
                        custom_size: Some(Vec2::new(20.0, 130.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(Paddle {
                    speed: paddle_speed,
                })
                .insert(AiController);

            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::rgb(1.0, 0.5, 0.5),
                        custom_size: Some(Vec2::new(30.0, 30.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(Ball {
                    velocity: 800.0 * Vec3::new(0.5, -0.5, 0.0).normalize(),
                });
        });

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font,
        font_size: 35.0,
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
                    size: Size::new(Val::Px(config.court_size[0]), Val::Px(config.court_size[1] + 100.0)),
                    ..default()
                },
                color: UiColor(Color::NONE),
                ..default()
            })
            .with_children(|parent| {
                parent
                    .spawn_bundle(
                        TextBundle::from_sections([
                            TextSection::new("Player: ", text_style.clone()),
                            TextSection::from_style(text_style.clone()),
                        ])
                        .with_style(Style {
                            position_type: PositionType::Absolute,
                            position: UiRect {
                                top: Val::Px(4.0),
                                left: Val::Px(4.0),
                                ..default()
                            },
                            ..default()
                        }),
                    )
                    .insert(PlayerController);
                parent
                    .spawn_bundle(
                        TextBundle::from_sections([
                            TextSection::new("AI: ", text_style.clone()),
                            TextSection::from_style(text_style.clone()),
                        ])
                        .with_style(Style {
                            align_content: AlignContent::FlexEnd,
                            position_type: PositionType::Absolute,
                            position: UiRect {
                                top: Val::Px(4.0),
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

fn keyboard_input(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform, &Sprite), With<PlayerController>>,
    config: Res<Config>,
) {
    let half_court_height = config.court_size[1] / 2.0;
    for (paddle, mut paddle_transform, paddle_sprite) in query.iter_mut() {
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Up) {
            direction += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            direction -= 1.0;
        }

        let paddle_half_height = paddle_sprite
            .custom_size
            .expect("Sprite should have custom height")
            .y
            / 2.0;
        let translation = &mut paddle_transform.translation;
        translation.y += time.delta_seconds() * direction * paddle.speed;
        translation.y = translation
            .y
            .min(half_court_height - paddle_half_height)
            .max(-half_court_height + paddle_half_height)
    }
}

fn ai_input(
    time: Res<Time>,
    mut paddle_query: Query<(&Paddle, &mut Transform, &Sprite), With<AiController>>,
    ball_query: Query<(&Ball, &Transform), Without<AiController>>,
    config: Res<Config>,
) {
    let half_court_height = config.court_size[1] / 2.0;
    let court_width = config.court_size[0];
    let view_distance_px = court_width * config.ai_handicap.view_percentage;
    for (paddle, mut paddle_transform, paddle_sprite) in paddle_query.iter_mut() {
        let mut direction = 0.0;
        for (ball, ball_transform) in ball_query.iter() {
            if ball.velocity.x < 0.0
                || f32::abs(ball_transform.translation.x - paddle_transform.translation.x)
                    > view_distance_px
            {
                continue;
            }

            if ball_transform.translation.y > paddle_transform.translation.y {
                direction += 1.0;
            }
            if ball_transform.translation.y < paddle_transform.translation.y {
                direction -= 1.0;
            }

            let paddle_half_height = paddle_sprite
                .custom_size
                .expect("Sprite should have custom height")
                .y
                / 2.0;
            let translation = &mut paddle_transform.translation;
            translation.y += time.delta_seconds() * direction * paddle.speed;
            translation.y = translation
                .y
                .min(half_court_height - paddle_half_height)
                .max(-half_court_height + paddle_half_height)
        }
    }
}

fn ball_movement_system(time: Res<Time>, mut ball_query: Query<(&Ball, &mut Transform)>) {
    // clamp the timestep to stop the ball from escaping when the game starts
    let delta_seconds = f32::min(0.2, time.delta_seconds());

    for (ball, mut transform) in ball_query.iter_mut() {
        transform.translation += ball.velocity * delta_seconds;
    }
}

fn ball_collision_system(
    mut ball_query: Query<(&mut Ball, &Transform, &Sprite)>,
    court_collider_query: Query<(&Court, &Transform, &Sprite)>,
    paddle_collider_query: Query<(&Paddle, &Transform, &Sprite)>,
) {
    for (mut ball, ball_transform, sprite) in ball_query.iter_mut() {
        let ball_size = sprite.custom_size.expect("Ball should have custom size");
        let velocity = &mut ball.velocity;

        // check collision with court top and bottom
        for (_court, court_transform, sprite) in court_collider_query.iter() {
            let other_size = sprite
                .custom_size
                .expect("Collider should have custom size");
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
        }

        for (_paddle, transform, sprite) in &paddle_collider_query {
            let paddle_size = sprite.custom_size.expect("Paddle should have custom size");
            let collision = collide(
                ball_transform.translation,
                ball_size,
                transform.translation,
                paddle_size,
            );
            if let Some(collision) = collision {
                match collision {
                    Collision::Right => velocity.x = velocity.x.abs(),
                    Collision::Left => velocity.x = -velocity.x.abs(),
                    _ => (),
                }
            }
        }
    }
}

fn ball_scoring_system(
    mut ball_query: Query<(&Ball, &mut Transform, &Sprite), Without<Court>>,
    mut scoreboard: ResMut<Scoreboard>,
    court_collider_query: Query<(&Court, &Transform, &Sprite), Without<Ball>>,
) {
    for (_ball, mut ball_transform, ball_sprite) in ball_query.iter_mut() {
        let ball_size = ball_sprite
            .custom_size
            .expect("Collider should have custom size");
        for (_court, court_transform, court_sprite) in &court_collider_query {
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
                        ball_transform.translation.x = 0.0;
                        ball_transform.translation.y = 0.0;
                        scoreboard.ai += 1;
                    }
                    Collision::Right => {
                        ball_transform.translation.x = 0.0;
                        ball_transform.translation.y = 0.0;
                        scoreboard.player += 1;
                    }
                    _ => (),
                }
            }
        }
    }
}

fn scoreboardsystem(
    scoreboard: ResMut<Scoreboard>,
    mut player_scoreboard: Query<&mut Text, (With<PlayerController>, Without<AiController>)>,
    mut ai_scoreboard: Query<&mut Text, (With<AiController>, Without<PlayerController>)>,
) {
    for mut text in player_scoreboard.iter_mut() {
        text.sections[1].value = format!("{}", scoreboard.player);
    }
    for mut text in ai_scoreboard.iter_mut() {
        text.sections[1].value = format!("{}", scoreboard.ai);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Scoreboard { player: 0, ai: 0 })
        .insert_resource(Config {
            paddle_speed: 1000.0,
            court_size: [1000.0, 1000.0],
            players_distance_percentage: 0.3,
            ai_handicap: AiHandicap {
                view_percentage: 0.5,
            },
        })
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .label(GameloopStages::Input)
                .before(GameloopStages::Physics)
                .with_system(keyboard_input)
                .with_system(ai_input),
        )
        .add_system_set(
            SystemSet::new()
                .label(GameloopStages::Physics)
                .with_system(ball_movement_system)
                .with_system(ball_collision_system.after(ball_movement_system)),
        )
        .add_system_set(
            SystemSet::new()
                .label(GameloopStages::Scoring)
                .after(GameloopStages::Physics)
                .with_system(ball_scoring_system)
                .with_system(scoreboardsystem.after(ball_scoring_system)),
        )
        .run();
}
