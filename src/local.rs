use std::time::Duration;

use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::{
    pong::{
        ball_collision, ball_movement, ball_scoring, paddle_movement, scoreboard, serve, Ball,
        CourtSide, GameloopStage, LeftPlayer, Paddle, PaddleInputs, PongConfig, PongState,
        RightPlayer, Side,
    },
    types::GameState,
    GameType,
};

#[derive(Component)]
struct PlayerController;

#[derive(Component)]
struct AiController;

#[derive(Component, Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum LocalGameType {
    SinglePlayer,
    MultiPlayer,
}

struct AiHandicap {
    view_percentage: f32,
}

#[derive(Resource)]
struct LocalConfig {
    ai_handicap: AiHandicap,
}

fn ai_input(
    mut paddle_query: Query<(&mut Paddle, &Transform, &Sprite), With<AiController>>,
    ball_query: Query<(&Ball, &Transform), Without<Paddle>>,
    mut inputs: ResMut<PaddleInputs>,
    pong_config: Res<PongConfig>,
    local_config: Res<LocalConfig>,
) {
    let court_width = pong_config.court_size[0];
    for (mut paddle, paddle_transform, paddle_sprite) in &mut paddle_query {
        let paddle_size = paddle_sprite
            .custom_size
            .expect("Paddle should have custom size");
        let view_distance_px = court_width * local_config.ai_handicap.view_percentage;
        let mut direction = 0.0;
        let (ball, ball_transform) = ball_query.single();
        if ball.velocity.x < 0.0
            || (ball_transform.translation.x - paddle_transform.translation.x).abs()
                > view_distance_px
        {
            return;
        }

        if (ball_transform.translation.y - paddle_transform.translation.y).abs()
            > paddle_size.y / 2.
        {
            let input = &mut inputs[1];
            input.move_down = false;
            input.move_up = false;
            input.serve = true;
            if ball_transform.translation.y > paddle_transform.translation.y {
                input.move_up = true;
            }
            if ball_transform.translation.y < paddle_transform.translation.y {
                input.move_down = true;
            }
        }

        paddle.direction.y = direction;
    }
}

fn keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut inputs: ResMut<PaddleInputs>,
    local_game_type: Res<CurrentState<LocalGameType>>,
) {
    inputs[0].move_down = false;
    inputs[0].move_up = false;
    inputs[0].serve = false;

    if keyboard_input.pressed(KeyCode::W) {
        inputs[0].move_up = true;
    }
    if keyboard_input.pressed(KeyCode::S) {
        inputs[0].move_down = true;
    }
    if keyboard_input.pressed(KeyCode::Space) {
        inputs[0].serve = true;
    }

    if local_game_type.0 == LocalGameType::SinglePlayer {
        if keyboard_input.pressed(KeyCode::Up) {
            inputs[0].move_up = true;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            inputs[0].move_down = true;
        }
    }

    if local_game_type.0 == LocalGameType::MultiPlayer {
        inputs[1].move_down = false;
        inputs[1].move_up = false;
        inputs[1].serve = false;

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

fn setup_local_player_controllers(
    mut commands: Commands,
    gametype: Res<CurrentState<LocalGameType>>,
    mut paddle_set: ParamSet<(
        Query<Entity, (Added<Paddle>, With<LeftPlayer>)>,
        Query<Entity, (Added<Paddle>, With<RightPlayer>)>,
    )>,
) {
    if let Ok(left_paddle) = paddle_set.p0().get_single() {
        commands.entity(left_paddle).insert(PlayerController);
    }

    if let Ok(right_paddle) = paddle_set.p1().get_single() {
        match gametype.0 {
            LocalGameType::SinglePlayer => {
                commands.entity(right_paddle).insert(AiController);
            }
            LocalGameType::MultiPlayer => {
                commands.entity(right_paddle).insert(PlayerController);
            }
            _ => (),
        };
    }
}

pub struct LocalPlugin;

impl Plugin for LocalPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LocalConfig {
            ai_handicap: AiHandicap {
                view_percentage: 0.5,
            },
        })
        .add_loopless_state(LocalGameType::SinglePlayer);

        app.add_system(
            setup_local_player_controllers
                .run_in_state(GameType::Local)
                .run_in_state(GameState::Ingame),
        );

        app.add_fixed_timestep(Duration::from_millis(30), "fixed_timestep")
            .add_fixed_timestep_system_set(
                "fixed_timestep",
                0,
                ConditionSet::new()
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .label(GameloopStage::Input)
                    .with_system(keyboard_input)
                    .with_system(ai_input.run_in_state(LocalGameType::SinglePlayer))
                    .into(),
            )
            .add_fixed_timestep_system_set(
                "fixed_timestep",
                0,
                ConditionSet::new()
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .run_in_state(PongState::Serve(Side::Left))
                    .after(GameloopStage::Input)
                    .with_system(serve::<LeftPlayer>)
                    .into(),
            )
            .add_fixed_timestep_system_set(
                "fixed_timestep",
                0,
                ConditionSet::new()
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .run_in_state(PongState::Serve(Side::Right))
                    .after(GameloopStage::Input)
                    .with_system(serve::<RightPlayer>)
                    .into(),
            )
            .add_fixed_timestep_system(
                "fixed_timestep",
                0,
                paddle_movement
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .after(GameloopStage::Input)
                    .label(GameloopStage::Movement),
            )
            .add_fixed_timestep_system(
                "fixed_timestep",
                0,
                ball_movement
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .run_in_state(PongState::Playing)
                    .after(GameloopStage::Input)
                    .label(GameloopStage::Movement),
            )
            .add_fixed_timestep_system(
                "fixed_timestep",
                0,
                ball_collision
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .run_in_state(PongState::Playing)
                    .after(GameloopStage::Movement)
                    .label(GameloopStage::Collision),
            )
            .add_fixed_timestep_system(
                "fixed_timestep",
                0,
                ball_scoring
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .run_in_state(PongState::Playing)
                    .after(GameloopStage::Collision)
                    .label(GameloopStage::Scoring),
            )
            .add_fixed_timestep_system(
                "fixed_timestep",
                0,
                scoreboard
                    .run_in_state(GameType::Local)
                    .run_in_state(GameState::Ingame)
                    .after(GameloopStage::Scoring),
            );
    }
}
