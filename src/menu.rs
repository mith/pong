use bevy::prelude::*;

use iyes_loopless::prelude::*;

use crate::local::{LocalGameType, LocalPlugin};

use crate::types::{GameType, MainMenu};
use crate::util::despawn_with;

use crate::types::GameState;

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/PublicPixel-z84yD.ttf");
    let text_style = TextStyle {
        font,
        font_size: 32.,
        ..default()
    };
    let menu_spacing = 40.;
    let button_bundle = ButtonBundle {
        style: Style {
            margin: UiRect::all(Val::Px(menu_spacing)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        background_color: Color::BLACK.into(),
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
            parent
                .spawn((
                    GameType::Local,
                    LocalGameType::SinglePlayer,
                    button_bundle.clone(),
                ))
                .with_children(|parent_button| {
                    parent_button.spawn(TextBundle::from_section("1 Player", text_style.clone()));
                });

            parent
                .spawn((
                    GameType::Local,
                    LocalGameType::MultiPlayer,
                    button_bundle.clone(),
                ))
                .with_children(|parent_button| {
                    parent_button.spawn(TextBundle::from_section("2 Players", text_style.clone()));
                });

            #[cfg(feature = "online")]
            parent
                .spawn((GameType::Online, button_bundle))
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
                #[cfg(feature = "online")]
                GameType::Online => {
                    commands.insert_resource(NextState(GameState::Lobby));
                }
            };
        }
    }
}

pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::MainMenu, setup_menu)
            .add_exit_system(GameState::MainMenu, despawn_with::<MainMenu>)
            .add_system(gametype_button.run_in_state(GameState::MainMenu));
    }
}
