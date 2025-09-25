// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::AssetMetaCheck, diagnostic, prelude::*};

use worgen_rs::*;

fn main() {
    App::new()
        .add_plugins((
            assets::archive::ArchiveAssetReaderPlugin,
            DefaultPlugins.set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
        ))
        .add_plugins(assets::ExperimentalModelAssetPlugin)
        .add_plugins(diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        .add_plugins(material::TerrainMaterialPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(data::DataPlugin)
        .add_plugins(camera::PanOrbitCameraPlugin)
        .run();
}
