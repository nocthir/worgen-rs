// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{diagnostic, prelude::*};

mod camera;
mod data;
mod material;
mod settings;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        .add_plugins(material::CustomMaterialPlugin)
        .add_plugins(settings::SettingsPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(data::DataPlugin)
        .add_plugins(camera::PanOrbitCameraPlugin)
        .run();
}
