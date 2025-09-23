// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{fs, io, ptr::addr_of, sync::Once};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub static mut SETTINGS: Settings = Settings::new();
static SETTINGS_ONCE: Once = Once::new();

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
}

impl Settings {
    pub const fn new() -> Self {
        Self {
            game_path: String::new(),
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
}

#[derive(Serialize, Deserialize, Default)]
pub struct FileSettings {
    pub archive_path: String,
    pub file_path: String,
}

pub fn load_settings() {
    // SAFETY: no concurrent static mut access due to std::Once
    #[allow(static_mut_refs)]
    SETTINGS_ONCE.call_once(|| unsafe { SETTINGS.load().expect("Failed to load settings") });
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
    pub test_texture: FileSettings,
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
