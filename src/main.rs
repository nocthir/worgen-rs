use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorgenPlugin)
        .run();
}

pub struct WorgenPlugin;

impl Plugin for WorgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_info);
    }
}

fn start_info() {
    info!("Hello, Worgen!");
}
