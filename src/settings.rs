// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{fs, io};

use anyhow::Result;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{assets::material::ExtTerrainMaterial, state::WorgenState};

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_common_assets::json::JsonAssetPlugin::<Settings>::new(
            &["settings.json"],
        ))
        .insert_resource(TerrainSettings::default())
        .add_systems(Startup, load_settings_asset)
        .add_systems(
            Update,
            check_settings_loaded.run_if(in_state(WorgenState::Loading)),
        )
        .add_systems(Update, apply_terrain_settings);
    }
}

fn check_settings_loaded(
    settings_handle: Res<SettingsHandle>,
    settings_assets: Res<Assets<Settings>>,
    mut state: ResMut<NextState<WorgenState>>,
) {
    if settings_assets.get(&settings_handle.0).is_some() {
        info!("Settings loaded");
        state.set(WorgenState::Ready);
    }
}

fn apply_terrain_settings(
    terrain_settings: Res<TerrainSettings>,
    mut materials: ResMut<Assets<ExtTerrainMaterial>>,
) {
    for (_idx, material) in materials.iter_mut() {
        let level_mask = (if terrain_settings.level0 { 1 } else { 0 })
            | (if terrain_settings.level1 { 2 } else { 0 })
            | (if terrain_settings.level2 { 4 } else { 0 })
            | (if terrain_settings.level3 { 8 } else { 0 });
        material.extension.level_mask = level_mask;
    }
}

#[derive(Asset, TypePath, Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
    pub test_model_path: Option<String>,
}

impl Settings {
    // Keep helper for tests/tools to parse settings directly from file if needed
    #[allow(dead_code)]
    pub fn load_direct() -> Result<Self> {
        let file = fs::read("assets/settings.json")?;
        let reader = io::Cursor::new(file);
        let settings: Settings = serde_json::from_reader(reader)?;
        Ok(settings)
    }
}

// Resource holding the handle to the loaded Settings asset
#[derive(Resource, Deref, DerefMut)]
pub struct SettingsHandle(pub Handle<Settings>);

fn load_settings_asset(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle: Handle<Settings> = asset_server.load("settings.json");
    commands.insert_resource(SettingsHandle(handle));
}

#[derive(Serialize, Deserialize, Default)]
pub struct FileSettings {
    pub archive_path: String,
    pub file_path: String,
}

#[derive(Resource, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TestSettings {
    pub game_path: String,
    pub default_model: FileSettings,
    pub test_model_path: String,
    pub test_world_model: FileSettings,
    pub city_model: FileSettings,
    pub world_map_path: FileSettings,
    pub texture_archive_path: String,
    pub interface_archive_path: String,
    pub model_archive_path: String,
    pub world_model_archive_path: String,
    pub terrain_archive_path: String,
    pub test_texture: FileSettings,
    pub test_terrain_path: String,
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

#[derive(Reflect, Resource, Debug, Clone, Copy)]
pub struct TerrainSettings {
    pub level0: bool,
    pub level1: bool,
    pub level2: bool,
    pub level3: bool,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        Self {
            level0: true,
            level1: true,
            level2: true,
            level3: true,
        }
    }
}
