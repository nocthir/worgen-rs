// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;
use bevy_egui::*;

mod camera;
mod ui;
mod worgen;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(ui::UiPlugin)
        .add_plugins(worgen::WorgenPlugin)
        .add_systems(Startup, camera::setup_camera)
        .run();
}
