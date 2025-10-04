// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{fs, io, ptr::addr_of, sync::Once};

use anyhow::Result;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::assets::material::ExtTerrainMaterial;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TerrainSettings::default())
            .add_systems(Update, apply_terrain_settings);
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

pub static mut SETTINGS: Settings = Settings::new();
static SETTINGS_ONCE: Once = Once::new();

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
    pub test_image_path: String,
    pub test_model_path: Option<String>,
}

impl Settings {
    pub const fn new() -> Self {
        Self {
            game_path: String::new(),
            test_image_path: String::new(),
            test_model_path: None,
        }
    }

    pub fn get() -> &'static Self {
        debug_assert!(SETTINGS_ONCE.is_completed());
        // SAFETY: no mut references exist at this point
        unsafe { &*addr_of!(SETTINGS) }
    }

    pub fn load(&mut self) -> Result<()> {
        let file = fs::read("assets/settings.json")?;
        let reader = io::Cursor::new(file);
        *self = serde_json::from_reader(reader)?;
        Ok(())
    }

    pub fn init() {
        // SAFETY: no concurrent static mut access due to std::Once
        #[allow(static_mut_refs)]
        SETTINGS_ONCE.call_once(|| unsafe { SETTINGS.load().expect("Failed to load settings") });
    }
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
