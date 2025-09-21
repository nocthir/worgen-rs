// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{
    prelude::*,
    tasks::{self, Task},
};
use wow_m2 as m2;
use wow_mpq as mpq;

use crate::data::file;

#[derive(Clone)]
pub struct ModelInfo {
    pub model: m2::M2Model,
    pub data: Vec<u8>,
    pub texture_paths: Vec<String>,
}

impl ModelInfo {
    pub fn new<P: AsRef<Path>>(file_path: &str, archive_path: P) -> Result<Self> {
        let mut archive = mpq::Archive::open(archive_path)?;
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

pub fn loading_model_task(task: file::LoadFileTask) -> Task<file::LoadFileTask> {
    info!("Starting to load model: {}", task.file.path);
    tasks::IoTaskPool::get().spawn(load_model(task))
}

async fn load_model(mut task: file::LoadFileTask) -> file::LoadFileTask {
    match ModelInfo::new(&task.file.path, &task.file.archive_path) {
        Ok(model_info) => {
            task.file.set_model(model_info);
            info!("Loaded model: {}", task.file.path);
        }
        Err(e) => {
            error!("Failed to load model {}: {}", task.file.path, e);
            task.file.state = file::FileInfoState::Error(e.to_string());
        }
    }
    task
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{data::bundle, *};

    #[test]
    fn load_main_menu() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.default_model.file_path)?;
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();

        bundle::create_mesh_from_file_path(
            &settings.default_model.file_path,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn load_city() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.city_model.file_path)?;
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        bundle::create_mesh_from_file_path(
            &settings.city_model.file_path,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn load_dwarf() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.test_model.file_path)?;
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        bundle::create_mesh_from_file_path(
            &settings.test_model.file_path,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
