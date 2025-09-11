// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use bevy_egui::*;

use crate::material::CustomMaterialPlugin;

mod camera;
mod data;
mod material;
mod settings;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(CustomMaterialPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(settings::SettingsPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(data::DataPlugin)
        .add_plugins(camera::PanOrbitCameraPlugin)
        .run();
}
