use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};
use iyes_loopless::prelude::*;

use crate::{types::GameState, util::despawn_with};

#[derive(Default, Copy, Clone)]
pub(crate) struct PaddleInput {
    pub(crate) move_up: bool,
    pub(crate) move_down: bool,
    pub(crate) serve: bool,
}

#[derive(Component)]
pub(crate) struct Court;

pub trait CourtSide {}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum Side {
    Left,
    Right,
}

#[derive(Component)]
pub(crate) struct LeftPlayer;

impl CourtSide for LeftPlayer {}

#[derive(Component)]
pub(crate) struct RightPlayer;

impl CourtSide for RightPlayer {}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum PongState {
    Serve(Side),
    Playing,
}

#[derive(SystemLabel)]
pub(crate) enum GameloopStage {
    Input,
    Movement,
    Collision,
    Scoring,
}

#[derive(Debug, Component, Reflect, Default)]
pub(crate) struct Paddle {
    pub(crate) handle: usize,
    pub(crate) speed: f32,
    pub(crate) direction: Vec2,
}

#[derive(Component, Reflect, Default)]
pub(crate) struct Ball {
    pub(crate) velocity: Vec3,
}

#[derive(Resource)]
pub(crate) struct PongConfig {
    pub(crate) court_size: [f32; 2],
    pub(crate) players_distance_percentage: f32,
    pub(crate) paddle_speed: f32,
    pub(crate) ball_speed: f32,
}

#[derive(Resource, Debug)]
pub(crate) struct Score {
    pub(crate) left: usize,
    pub(crate) right: usize,
}

#[derive(Component)]
pub(crate) struct Scoreboard;

pub(crate) fn paddle_movement(
    mut paddle_query: Query<(&mut Paddle, &mut Transform, &Sprite)>,
    inputs: Res<PaddleInputs>,
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

pub(crate) fn ball_movement(mut ball_query: Query<(&Ball, &mut Transform)>) {
    let (ball, mut transform) = ball_query.single_mut();
    transform.translation += ball.velocity;
}

pub(crate) fn ball_collision(
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

pub(crate) fn ball_scoring(
    mut commands: Commands,
    ball_query: Query<(&Ball, &Transform, &Sprite), Without<Court>>,
    mut score: ResMut<Score>,
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
                score.right += 1;
                commands.insert_resource(NextState(PongState::Serve(Side::Left)));
            }
            Collision::Right => {
                score.left += 1;
                commands.insert_resource(NextState(PongState::Serve(Side::Right)));
            }
            _ => (),
        }
    }
}

pub(crate) fn scoreboard(
    scoreboard: ResMut<Score>,

    mut score_text_set: ParamSet<(
        Query<&mut Text, With<LeftPlayer>>,
        Query<&mut Text, With<RightPlayer>>,
    )>,
) {
    {
        let mut left_query = score_text_set.p0();
        let mut left_score_text = left_query.single_mut();
        left_score_text.sections[0].value = format!("{}", scoreboard.left);
    }

    {
        let mut right_query = score_text_set.p1();
        let mut right_score_text = right_query.single_mut();
        right_score_text.sections[0].value = format!("{}", scoreboard.right);
    }
}

pub(crate) fn adjust_scoreboard_scale(
    config: Res<PongConfig>,
    windows: Res<Windows>,
    mut projections: Query<&mut OrthographicProjection, With<Camera>>,
    mut scoreboard_query: Query<&mut Text, With<Scoreboard>>,
) {
    let window = windows.primary();
    let height_ratio = config.court_size[0] / window.height();

    for mut ortho in projections.iter_mut() {
        ortho.scale = height_ratio * 0.8;
    }

    for mut score in &mut scoreboard_query {
        score.sections[0].style.font_size = window.height() * 0.07;
    }
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct PaddleInputs(pub(crate) Vec<PaddleInput>);

pub(crate) fn serve<T: CourtSide + Component>(
    mut commands: Commands,
    config: Res<PongConfig>,
    inputs: Res<PaddleInputs>,
    mut paddle_query: Query<(&Paddle, &mut Transform), With<T>>,
    mut ball_query: Query<(&mut Ball, &mut Transform), Without<Paddle>>,
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

pub(crate) fn setup_camera(mut commands: Commands, config: Res<PongConfig>, windows: Res<Windows>) {
    let window = windows.primary();

    let height_ratio = window.height() / config.court_size[0];
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: height_ratio,
            ..default()
        },
        ..default()
    });
}

#[derive(SystemLabel)]
pub(crate) struct PongGameSetup;

pub(crate) fn setup_court(mut commands: Commands, config: Res<PongConfig>) {
    let paddle_speed = config.paddle_speed;
    commands
        .spawn((
            Court,
            SpriteBundle {
                sprite: Sprite {
                    color: Color::BLACK,
                    custom_size: Some(Vec2::from_array(config.court_size)),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|parent| {
            let num_dashes = (config.court_size[1] / 30.0) as i32;
            for y in 0..num_dashes {
                parent.spawn(SpriteBundle {
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
            parent.spawn((
                LeftPlayer,
                Paddle {
                    handle: 0,
                    speed: paddle_speed,
                    direction: Vec2::new(0., 0.),
                },
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(-player_distance, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(paddle_size),
                        ..default()
                    },
                    ..default()
                },
            ));

            parent.spawn((
                RightPlayer,
                Paddle {
                    handle: 1,
                    speed: paddle_speed,
                    direction: Vec2::new(0., 0.),
                },
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(player_distance, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::rgb(1.0, 1.0, 1.0),
                        custom_size: Some(paddle_size),
                        ..default()
                    },
                    ..default()
                },
            ));

            parent.spawn((
                Ball {
                    velocity: Vec3::new(0.0, 0.0, 0.0),
                },
                SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(20.0, 20.0)),
                        ..default()
                    },
                    ..default()
                },
            ));
        });
}

pub(crate) fn setup_scoreboard(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    windows: Res<Windows>,
) {
    let window = windows.primary();
    let font = asset_server.load("fonts/PublicPixel-z84yD.ttf");
    let text_style = TextStyle {
        font,
        font_size: window.height() * 0.15,
        ..default()
    };

    commands
        .spawn((
            Scoreboard,
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    align_items: AlignItems::FlexEnd,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                background_color: Color::NONE.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                LeftPlayer,
                TextBundle::from_section("0", text_style.clone()).with_style(Style {
                    // position_type: PositionType::Absolute,
                    align_content: AlignContent::FlexStart,
                    margin: UiRect {
                        right: Val::Percent(10.),
                        ..default()
                    },
                    ..default()
                }),
            ));

            parent.spawn((
                RightPlayer,
                TextBundle::from_section("0", text_style.clone()).with_style(Style {
                    align_content: AlignContent::FlexStart,
                    margin: UiRect {
                        left: Val::Percent(10.),
                        ..default()
                    },
                    ..default()
                }),
            ));
        });
}

pub(crate) struct PongPlugin;

impl Plugin for PongPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Score { left: 0, right: 0 })
            .insert_resource(PongConfig {
                paddle_speed: 10.,
                ball_speed: 20.,
                court_size: [1600., 1000.],
                players_distance_percentage: 0.4,
            })
            .insert_resource(PaddleInputs(vec![
                PaddleInput { ..default() },
                PaddleInput { ..default() },
            ]))
            .add_loopless_state(PongState::Serve(Side::Left))
            .add_enter_system(GameState::Ingame, setup_court)
            .add_enter_system(GameState::Ingame, setup_scoreboard)
            .add_exit_system(GameState::Ingame, despawn_with::<Court>)
            .add_exit_system(GameState::Ingame, despawn_with::<Scoreboard>)
            .add_system(adjust_scoreboard_scale.run_in_state(GameState::Ingame));
    }
}
