// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{fs, io};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
    pub default_model: FileSettings,
    pub test_model: FileSettings,
    pub test_world_model: FileSettings,
    pub city_model: FileSettings,
    pub world_map_path: FileSettings,
    pub texture_archive_path: String,
    pub interface_archive_path: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FileSettings {
    pub archive_path: String,
    pub file_path: String,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_resource);
    }
}

fn load_resource(mut commands: Commands) -> Result {
    commands.insert_resource(load_settings()?);
    Ok(())
}

pub fn load_settings() -> Result<Settings> {
    let file = fs::read("assets/settings.json")?;
    let reader = io::Cursor::new(file);
    let settings: Settings = serde_json::from_reader(reader)?;
    Ok(settings)
}
