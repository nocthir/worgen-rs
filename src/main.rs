// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_egui::*;

use crate::loading::LoadingPlugin;

mod camera;
mod loading;
mod settings;
mod state;
mod ui;
mod worgen;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .init_state::<state::GameState>()
        .add_plugins(EguiPlugin::default())
        .add_plugins(settings::SettingsPlugin)
        .add_plugins(LoadingPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(worgen::WorgenPlugin)
        .add_plugins(camera::PanOrbitCameraPlugin)
        .run();
}
