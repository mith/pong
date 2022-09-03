use bevy::{
    prelude::*,
    sprite::{
        collide_aabb::{collide, Collision},
        MaterialMesh2dBundle,
    },
    utils::Duration,
};
use iyes_loopless::prelude::*;

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

fn setup(
    mut commands: Commands,
    config: Res<Config>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    windows: Res<Windows>,
) {
    let paddle_speed = config.paddle_speed;

    let window = windows.primary();

    let height_ratio = window.height() / config.court_size[0];

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
                color: Color::rgb(0.0, 0.0, 0.0),
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
                        color: Color::rgb(1.0, 1.0, 1.0),
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
                        color: Color::rgb(1.0, 1.0, 1.0),
                        custom_size: Some(paddle_size),
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
                        color: Color::rgb(1.0, 1.0, 1.0),
                        custom_size: Some(paddle_size),
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
                        color: Color::rgb(1.0, 1.0, 1.0),
                        custom_size: Some(Vec2::new(20.0, 20.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(Ball {
                    velocity: 800.0 * Vec3::new(0.5, -0.5, 0.0).normalize(),
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
                        TextBundle::from_sections([
                            TextSection::from_style(text_style.clone()),
                        ])
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
                        TextBundle::from_sections([
                            TextSection::from_style(text_style.clone()),
                        ])
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

            if (ball_transform.translation.y - paddle_transform.translation.y).abs() > 10.0 {
                if ball_transform.translation.y > paddle_transform.translation.y {
                    direction += 1.0;
                }
                if ball_transform.translation.y < paddle_transform.translation.y {
                    direction -= 1.0;
                }
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

fn ball_movement_system(
    time: Res<FixedTimestepInfo>,
    mut ball_query: Query<(&Ball, &mut Transform)>,
) {
    // clamp the timestep to stop the ball from escaping when the game starts
    let delta_seconds = f32::min(0.2, time.timestep().as_secs_f32());

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

        for (_paddle, paddle_transform, sprite) in &paddle_collider_query {
            let paddle_size = sprite.custom_size.expect("Paddle should have custom size");
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
        text.sections[0].value = format!("{}", scoreboard.player);
    }
    for mut text in ai_scoreboard.iter_mut() {
        text.sections[0].value = format!("{}", scoreboard.ai);
    }
}

fn main() {
    let mut fixedupdate = SystemStage::parallel();
    fixedupdate.add_system(ball_movement_system);
    fixedupdate.add_system(ball_collision_system);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            fit_canvas_to_parent: true,
            ..default()
        })
        .insert_resource(Scoreboard { player: 0, ai: 0 })
        .insert_resource(Config {
            paddle_speed: 1000.0,
            court_size: [1600.0, 1000.0],
            players_distance_percentage: 0.3,
            ai_handicap: AiHandicap {
                view_percentage: 0.5,
            },
        })
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .label(GameloopStages::Input)
                .before(GameloopStages::Scoring)
                .with_system(keyboard_input)
                .with_system(ai_input)
                .with_system(adjust_scale),
        )
        .add_stage_before(
            CoreStage::Update,
            "my_fixed_update",
            FixedTimestepStage::from_stage(Duration::from_millis(4), fixedupdate),
        )
        .add_system_set(
            SystemSet::new()
                .label(GameloopStages::Scoring)
                .with_system(ball_scoring_system)
                .with_system(scoreboardsystem.after(ball_scoring_system)),
        )
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angled_collide_from_above() {
        let mut velocity = Vec3 {
            x: -1.0,
            y: 1.0,
            z: 0.0,
        };
        let angle = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
        .angle_between(Vec3 {
            x: 1.0,
            y: -1.0,
            z: 0.0,
        });
        angled_collide(Collision::Left, &mut velocity, angle, Vec2::new(20., 200.));
    }
}
