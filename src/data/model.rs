// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{
    prelude::*,
    tasks::{self, Task},
};
use wow_m2 as m2;

use crate::data::{archive, file};

#[derive(Clone)]
pub struct ModelInfo {
    pub model: m2::M2Model,
    pub data: Vec<u8>,
    pub texture_paths: Vec<String>,
}

impl ModelInfo {
    pub fn new<P: AsRef<Path>>(file_path: &str, archive_path: P) -> Result<Self> {
        let mut archive = archive::get_archive!(archive_path)?;
        let data = archive.read_file(file_path)?;
        let mut reader = io::Cursor::new(&data);
        let model = m2::M2Model::parse(&mut reader)?;
        let texture_paths = Self::create_texture_paths(&model);
        Ok(Self {
            model,
            data,
            texture_paths,
        })
    }

    fn create_texture_paths(model: &m2::M2Model) -> Vec<String> {
        model
            .textures
            .iter()
            .filter(|t| t.texture_type == m2::chunks::M2TextureType::Hardcoded)
            .map(|t| t.filename.string.to_string_lossy())
            .collect()
    }

    pub fn get_texture_paths(&self) -> Vec<String> {
        self.texture_paths.clone()
    }
}

pub fn is_model_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".m2")
        || lower_filename.ends_with(".mdx")
        || lower_filename.ends_with(".mdl")
}
