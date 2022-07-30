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

#[derive(Component)]
enum Collider {
    Solid,
    Paddle,
}

struct Scoreboard {
    player: usize,
    ai: usize,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let paddle_speed = 600.0;

    commands.spawn_bundle(Camera2dBundle::default());

    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(-500.0, 0.0, 0.0)),
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
        .insert(PlayerController)
        .insert(Collider::Paddle);

    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(500.0, 0.0, 0.0)),
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
        .insert(AiController)
        .insert(Collider::Paddle);

    commands
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

    // walls
    let wall_thickness = 40.0;
    let court_half_width = 340.0;

    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, court_half_width, 0.0)),
            sprite: Sprite {
                color: Color::rgb(0.8, 0.8, 0.8),
                custom_size: Some(Vec2::new(1300.0, wall_thickness)),
                ..default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid);

    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0.0, -court_half_width, 0.0)),
            sprite: Sprite {
                color: Color::rgb(0.8, 0.8, 0.8),
                custom_size: Some(Vec2::new(1300.0, wall_thickness)),
                ..default()
            },
            ..default()
        })
        .insert(Collider::Solid);

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font,
        font_size: 35.0,
        ..default()
    };

    commands
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
    commands
        .spawn_bundle(
            TextBundle::from_sections([
                TextSection::new("AI: ", text_style.clone()),
                TextSection::from_style(text_style.clone()),
            ])
            .with_style(Style {
                align_self: AlignSelf::FlexEnd,
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
}

fn keyboard_input(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Paddle, &mut Transform), With<PlayerController>>,
) {
    for (paddle, mut transform) in query.iter_mut() {
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Up) {
            direction += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            direction -= 1.0;
        }

        let translation = &mut transform.translation;
        translation.y += time.delta_seconds() * direction * paddle.speed;
        translation.y = translation.y.min(300.0).max(-300.0);
    }
}

fn ai_input(
    time: Res<Time>,
    mut paddle_query: Query<(&Paddle, &mut Transform), With<AiController>>,
    ball_query: Query<(&Ball, &Transform), Without<AiController>>,
) {
    for (paddle, mut paddle_transform) in paddle_query.iter_mut() {
        let mut direction = 0.0;
        for (ball, ball_transform) in ball_query.iter() {
            if ball.velocity.x < 0.0 || ball_transform.translation.x < -400.0 {
                continue;
            }
            
            if ball_transform.translation.y > paddle_transform.translation.y {
                direction += 1.0;
            }
            if ball_transform.translation.y < paddle_transform.translation.y {
                direction -= 1.0;
            }
            let translation = &mut paddle_transform.translation;
            translation.y += time.delta_seconds() * direction * paddle.speed;
            translation.y = translation.y.min(300.0).max(-300.0);
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
    collider_query: Query<(Entity, &Collider, &Transform, &Sprite)>,
) {
    for (mut ball, ball_transform, sprite) in ball_query.iter_mut() {
        let ball_size = sprite.custom_size.expect("Ball should have custom size");
        let velocity = &mut ball.velocity;

        // check collision with walls
        for (_collider_entity, collider, transform, sprite) in collider_query.iter() {
            let other_size = sprite
                .custom_size
                .expect("Collider should have custom size");
            let collision = collide(
                ball_transform.translation,
                ball_size,
                transform.translation,
                other_size,
            );

            if let Some(collision) = collision {
                // reflect the ball when it collides
                let mut reflect_x = false;
                let mut reflect_y = false;

                // only reflect if the ball's velocity is going in the opposite direction of the collision
                match collision {
                    Collision::Left => reflect_x = velocity.x > 0.0,
                    Collision::Right => reflect_x = velocity.x < 0.0,
                    Collision::Top => reflect_y = velocity.y < 0.0,
                    Collision::Bottom => reflect_y = velocity.y > 0.0,
                    Collision::Inside => (),
                }

                // reflect velocity on the x-axis if we hit something on the x-axis
                if reflect_x {
                    velocity.x = -velocity.x;
                }

                // reflect velocity on the y-axis if we hit something on the y-axis
                if reflect_y {
                    velocity.y = -velocity.y;
                }

                // break if this collide is on a solid, otherwise continue check whether a solid is also in collision
                if let Collider::Solid = *collider {
                    break;
                }
            }
        }
    }
}

fn ball_scoring_system(
    mut ball_query: Query<(&Ball, &mut Transform)>,
    mut scoreboard: ResMut<Scoreboard>,
) {
    let bounds = 600.0;
    for (_ball, mut transform) in ball_query.iter_mut() {
        if transform.translation.x < -bounds {
            transform.translation.x = 0.0;
            scoreboard.ai += 1;
        }

        if transform.translation.x > bounds {
            transform.translation.x = 0.0;
            scoreboard.player += 1;
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
        .add_startup_system(setup)
        .add_system(keyboard_input)
        .add_system(ball_movement_system)
        .add_system(ball_collision_system)
        .add_system(ai_input)
        .add_system(ball_scoring_system)
        .add_system(scoreboardsystem)
        .run();
}
