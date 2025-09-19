// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{fs, io};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let file = fs::read("assets/settings.json")?;
        let reader = io::Cursor::new(file);
        let settings: Settings = serde_json::from_reader(reader)?;
        Ok(settings)
    }
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
    commands.insert_resource(Settings::load()?);
    Ok(())
}

#[derive(Resource, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TestSettings {
    pub game_path: String,
    pub default_model: FileSettings,
    pub test_model: FileSettings,
    pub test_world_model: FileSettings,
    pub city_model: FileSettings,
    pub world_map_path: FileSettings,
    pub texture_archive_path: String,
    pub interface_archive_path: String,
    pub model_archive_path: String,
    pub world_model_archive_path: String,
    pub terrain_archive_path: String,
}

impl TestSettings {
    #[allow(dead_code)]
    pub fn load() -> Result<Self> {
        let file = fs::read("assets/settings.test.json")?;
        let reader = io::Cursor::new(file);
        let settings: TestSettings = serde_json::from_reader(reader)?;
        Ok(settings)
    }
}
