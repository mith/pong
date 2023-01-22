use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct MainMenu;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Component)]
pub(crate) enum GameType {
    Local,
    Online,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum GameState {
    MainMenu,
    Lobby,
    Ingame,
}
