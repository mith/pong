use bevy::{prelude::*, tasks::IoTaskPool};
use bevy_ggrs::{GGRSPlugin, Rollback, RollbackIdProvider, Session};
use bytemuck::{Pod, Zeroable};
use ggrs::{Config, InputStatus, PlayerHandle, SessionBuilder};
use iyes_loopless::prelude::*;
use matchbox_socket::WebRtcSocket;

use crate::{
    pong::{
        ball_collision, ball_movement, ball_scoring, paddle_movement, scoreboard, serve, Ball,
        GameloopStage, LeftPlayer, Paddle, PaddleInput, PaddleInputs, PongState, RightPlayer, Side,
    },
    types::GameType,
    GameState,
};

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

    commands.insert_resource(Socket(Some(socket)));
}

#[derive(Component)]
struct LobbyText;
#[derive(Component)]
struct LobbyUI;

fn setup_lobby(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
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
                .spawn(TextBundle {
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

#[derive(Resource, Deref, DerefMut)]
struct Socket(Option<WebRtcSocket>);

fn lobby(
    mut commands: Commands,
    mut socket: ResMut<Socket>,
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

    commands.insert_resource(Session::P2PSession(sess));

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
#[derive(Resource, Deref, DerefMut)]
struct BoxInputs(Vec<(BoxInput, InputStatus)>);

fn box_input_to_paddle_input(box_inputs: Res<BoxInputs>, mut paddle_inputs: ResMut<PaddleInputs>) {
    for i in 0..2 {
        let input = box_inputs[i].0.inp;
        paddle_inputs[i] = PaddleInput {
            move_up: input & INPUT_UP != 0,
            move_down: input & INPUT_DOWN != 0,
            serve: input & INPUT_SERVE != 0,
        }
    }
}

fn log_ggrs_events(mut session: ResMut<Session<GGRSConfig>>) {
    match session.as_mut() {
        Session::P2PSession(s) => {
            for event in s.events() {
                info!("GGRS Event: {:?}", event);
            }
        }
        _ => {}
    }
}

fn setup_online_player_controllers(
    mut commands: Commands,
    mut rip: ResMut<RollbackIdProvider>,
    mut paddle_set: ParamSet<(
        Query<Entity, (Added<Paddle>, With<LeftPlayer>)>,
        Query<Entity, (Added<Paddle>, With<RightPlayer>)>,
    )>,
) {
    if let Ok(left_paddle) = paddle_set.p0().get_single() {
        commands
            .entity(left_paddle)
            .insert(Rollback::new(rip.next_id()));
    }

    if let Ok(right_paddle) = paddle_set.p1().get_single() {
        commands
            .entity(right_paddle)
            .insert(Rollback::new(rip.next_id()));
    }
}

const ROLLBACK_DEFAULT: &str = "rollback_default";
pub(crate) struct OnlinePlugin;

impl Plugin for OnlinePlugin {
    fn build(&self, app: &mut App) {
        GGRSPlugin::<GGRSConfig>::new()
            .with_update_frequency(60)
            .with_input_system(input)
            .register_rollback_component::<Transform>()
            .register_rollback_component::<Ball>()
            .register_rollback_component::<Paddle>()
            .with_rollback_schedule(
                Schedule::default().with_stage(
                    ROLLBACK_DEFAULT,
                    SystemStage::parallel()
                        .with_system(box_input_to_paddle_input.before(GameloopStage::Input))
                        .with_system(
                            serve::<LeftPlayer>
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .run_in_state(PongState::Serve(Side::Left))
                                .label(GameloopStage::Input),
                        )
                        .with_system(
                            serve::<RightPlayer>
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .run_in_state(PongState::Serve(Side::Right))
                                .label(GameloopStage::Input),
                        )
                        .with_system(
                            paddle_movement
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .after(GameloopStage::Input)
                                .label(GameloopStage::Movement),
                        )
                        .with_system(
                            ball_movement
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .run_in_state(PongState::Playing)
                                .after(GameloopStage::Input)
                                .label(GameloopStage::Movement),
                        )
                        .with_system(
                            ball_collision
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .run_in_state(PongState::Playing)
                                .after(GameloopStage::Movement)
                                .label(GameloopStage::Collision),
                        )
                        .with_system(
                            ball_scoring
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .run_in_state(PongState::Playing)
                                .after(GameloopStage::Collision)
                                .label(GameloopStage::Scoring),
                        )
                        .with_system(
                            scoreboard
                                .run_in_state(GameType::Online)
                                .run_in_state(GameState::Ingame)
                                .after(GameloopStage::Scoring),
                        ),
                ),
            )
            .build(app);

        app.add_enter_system_set(
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
        .add_system(
            setup_online_player_controllers
                .run_in_state(GameType::Online)
                .run_in_state(GameState::Ingame),
        );
    }
}
