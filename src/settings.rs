// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{fs, io};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::GameState;

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct Settings {
    pub default_model: ModelSettings,
    pub test_model: ModelSettings,
    pub test_world_model: ModelSettings,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ModelSettings {
    pub archive_path: String,
    pub model_path: String,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_resource);
    }
}

fn load_resource(mut commands: Commands, mut next: ResMut<NextState<GameState>>) -> Result {
    commands.insert_resource(load_settings()?);
    next.set(GameState::Main);

    Ok(())
}

pub fn load_settings() -> Result<Settings> {
    let file = fs::read("assets/settings.json")?;
    let reader = io::Cursor::new(file);
    let settings: Settings = serde_json::from_reader(reader)?;
    Ok(settings)
}
