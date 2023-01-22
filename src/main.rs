use bevy::prelude::*;

use iyes_loopless::prelude::*;

use local::LocalPlugin;
use menu::MenuPlugin;
#[cfg(feature = "online")]
use online::OnlinePlugin;
use pong::{setup_camera, PongPlugin};

use types::GameType;

use crate::types::GameState;

mod local;
mod menu;
#[cfg(feature = "online")]
mod online;
mod pong;
mod types;
mod util;

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
    .add_plugin(MenuPlugin)
    .add_plugin(PongPlugin)
    .add_plugin(LocalPlugin);

    #[cfg(feature = "online")]
    app.add_plugin(OnlinePlugin);

    app.run();
}
