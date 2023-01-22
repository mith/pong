use bevy::prelude::*;

use iyes_loopless::prelude::*;

use local::{LocalGameType, LocalPlugin};
use online::OnlinePlugin;
use pong::{
    adjust_scoreboard_scale, setup_camera, setup_court, Court, PaddleInput, PaddleInputs,
    PongConfig, PongGameSetup, PongPlugin, PongState, Score, Scoreboard, Side,
};

use types::{GameType, MainMenu};
use util::despawn_with;

use crate::types::GameState;

mod local;
mod online;
mod pong;
mod types;
mod util;

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PublicPixel-z84yD.ttf");
    let text_style = TextStyle {
        font,
        font_size: 32.,
        ..default()
    };

    commands
        .spawn((
            MainMenu,
            NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::ColumnReverse,
                    ..default()
                },
                background_color: Color::NONE.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            let menu_spacing = 40.;
            parent
                .spawn((
                    GameType::Local,
                    LocalGameType::SinglePlayer,
                    ButtonBundle {
                        style: Style {
                            // size: Size::new(Val::Px(150.), Val::Px(50.)),
                            margin: UiRect::all(Val::Px(menu_spacing)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::BLACK.into(),
                        ..default()
                    },
                ))
                .with_children(|parent_button| {
                    parent_button.spawn(TextBundle::from_section("1 Player", text_style.clone()));
                });

            parent
                .spawn((
                    GameType::Local,
                    LocalGameType::MultiPlayer,
                    ButtonBundle {
                        style: Style {
                            margin: UiRect::all(Val::Px(menu_spacing)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::BLACK.into(),
                        ..default()
                    },
                ))
                .with_children(|parent_button| {
                    parent_button.spawn(TextBundle::from_section("2 Players", text_style.clone()));
                });
            parent
                .spawn((
                    GameType::Online,
                    ButtonBundle {
                        style: Style {
                            margin: UiRect::all(Val::Px(menu_spacing)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: Color::BLACK.into(),
                        ..default()
                    },
                ))
                .with_children(|parent_button| {
                    parent_button.spawn(TextBundle::from_section("Online", text_style.clone()));
                });
        });
}

fn gametype_button(
    mut commands: Commands,
    interaction_query: Query<
        (Entity, &Interaction, &GameType),
        (Changed<Interaction>, With<Button>),
    >,
    local_game_type_query: Query<&LocalGameType>,
) {
    for (button_entity, interaction, gametype) in &interaction_query {
        if Interaction::Clicked == *interaction {
            info!("Gametype picked: {:?}", gametype);
            commands.insert_resource(NextState(gametype.clone()));
            match gametype {
                GameType::Local => {
                    if let Ok(local_game_type) = local_game_type_query.get(button_entity) {
                        commands.insert_resource(NextState(GameState::Ingame));
                        commands.insert_resource(NextState(local_game_type.clone()));
                    }
                }
                GameType::Online => {
                    commands.insert_resource(NextState(GameState::Lobby));
                }
            };
        }
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        window: WindowDescriptor {
            fit_canvas_to_parent: true,
            ..default()
        },
        ..default()
    }))
    .insert_resource(ClearColor(Color::BLACK))
    .add_loopless_state(GameState::MainMenu)
    .add_loopless_state(GameType::Local)
    .add_startup_system(setup_camera)
    .add_enter_system(GameState::MainMenu, setup_menu)
    .add_exit_system(GameState::MainMenu, despawn_with::<MainMenu>)
    .add_system(gametype_button.run_in_state(GameState::MainMenu))
    .add_plugin(PongPlugin)
    .add_plugin(LocalPlugin)
    .add_plugin(OnlinePlugin)
    .run();
}
