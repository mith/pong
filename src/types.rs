use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct MainMenu;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Component)]
pub(crate) enum GameType {
    Local,
    #[cfg(feature = "online")]
    Online,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum GameState {
    MainMenu,
    #[cfg(feature = "online")]
    Lobby,
    Ingame,
}
