// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{prelude::*, tasks};
use wow_adt as adt;
use wow_mpq as mpq;

use crate::data::{
    ModelBundle,
    file::{self, FileInfoMap},
    model, world_model,
};

#[derive(Clone)]
pub struct WorldMapInfo {
    pub world_map: adt::Adt,
    pub model_paths: Vec<String>,
    pub world_model_paths: Vec<String>,
}

impl WorldMapInfo {
    pub fn new<P: AsRef<Path>>(file_path: &str, archive_path: P) -> Result<Self> {
        let mut archive = mpq::Archive::open(archive_path)?;
        let world_map = read_world_map(file_path, &mut archive)?;
        Ok(Self::from_adt(world_map))
    }

    fn from_adt(mut world_map: adt::Adt) -> Self {
        Self::fix_model_extensions(&mut world_map);
        let model_paths = Self::get_model_paths(&world_map);
        let world_model_paths = Self::get_world_model_paths(&world_map);
        Self {
            world_map,
            model_paths,
            world_model_paths,
        }
    }

    fn fix_model_extensions(world_map: &mut adt::Adt) {
        if let Some(mmdx) = &mut world_map.mmdx {
            for filename in &mut mmdx.filenames {
                if filename.ends_with(".mdx") {
                    *filename = filename.replace(".mdx", ".m2");
                }
            }
        }
    }

    fn get_model_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut models = Vec::new();
        if let Some(mmdx) = &world_map.mmdx {
            models.extend(
                mmdx.filenames
                    .iter()
                    .filter(|&f| f.ends_with(".m2"))
                    .cloned(),
            );
        }
        models
    }

    pub fn get_world_model_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut world_models = Vec::new();
        if let Some(modf) = &world_map.modf
            && let Some(mwmo) = &world_map.mwmo
        {
            let filenames = &mwmo.filenames;
            for model in &modf.models {
                if let Some(filename) = filenames.get(model.name_id as usize) {
                    world_models.push(filename.clone());
                }
            }
        }
        world_models
    }
}

pub fn read_world_map(path: &str, archive: &mut mpq::Archive) -> Result<adt::Adt> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(adt::Adt::from_reader(&mut reader)?)
}

pub fn is_world_map_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".adt")
}

pub fn loading_world_map_task(task: file::LoadFileTask) -> tasks::Task<file::LoadFileTask> {
    info!("Starting to load world map: {}", task.file.path);
    tasks::IoTaskPool::get().spawn(load_world_map(task))
}

async fn load_world_map(mut task: file::LoadFileTask) -> file::LoadFileTask {
    match WorldMapInfo::new(&task.file.path, &task.file.archive_path) {
        Ok(world_map_info) => {
            task.file.set_world_map(world_map_info);
            info!("Loaded world map: {}", task.file.path);
        }
        Err(e) => {
            error!("Failed to load world map {}: {}", task.file.path, e);
            task.file.state = file::FileInfoState::Error(e.to_string());
        }
    }
    task
}

// Actually used in tests
#[allow(unused)]
pub fn create_meshes_from_world_map_path(
    world_map_path: &str,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let world_map_info = file_info_map.get_world_map_info(world_map_path)?;
    create_meshes_from_world_map_info(world_map_info, file_info_map, images, materials, meshes)
}

pub fn create_meshes_from_world_map_info(
    world_map_info: &WorldMapInfo,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let mut bundles = Vec::new();

    let mut model_bundles = Vec::new();
    for model_path in &world_map_info.model_paths {
        let bundles = model::create_meshes_from_model_path(
            model_path,
            file_info_map,
            images,
            materials,
            meshes,
        )?;
        model_bundles.push(bundles);
    }

    if let Some(mddf) = &world_map_info.world_map.mddf {
        for placement in &mddf.doodads {
            let mut instantiated_bundles = model_bundles[placement.name_id as usize].clone();
            for bundle in &mut instantiated_bundles {
                bundle.transform.translation = Vec3::new(
                    placement.position[0],
                    placement.position[1],
                    placement.position[2],
                );
                bundle.transform.rotation =
                    Quat::from_axis_angle(Vec3::X, placement.rotation[0].to_radians())
                        * Quat::from_axis_angle(Vec3::Y, placement.rotation[1].to_radians())
                        * Quat::from_axis_angle(Vec3::Z, placement.rotation[2].to_radians());
                bundle.transform.scale = Vec3::splat(placement.scale);
            }
            bundles.extend(instantiated_bundles);
        }
    }

    let mut world_model_bundles = Vec::new();
    for world_model_path in &world_map_info.world_model_paths {
        let bundles = world_model::create_meshes_from_world_model_path(
            world_model_path,
            file_info_map,
            images,
            materials,
            meshes,
        )?;
        world_model_bundles.push(bundles);
    }

    if let Some(modf) = &world_map_info.world_map.modf {
        for placement in &modf.models {
            let mut instantiated_bundles = world_model_bundles[placement.name_id as usize].clone();
            for bundle in &mut instantiated_bundles {
                bundle.transform.translation = Vec3::new(
                    placement.position[0],
                    placement.position[1],
                    placement.position[2],
                );
                bundle.transform.rotation =
                    Quat::from_axis_angle(Vec3::X, placement.rotation[0].to_radians())
                        * Quat::from_axis_angle(Vec3::Y, placement.rotation[1].to_radians())
                        * Quat::from_axis_angle(Vec3::Z, placement.rotation[2].to_radians());
            }
            bundles.extend(instantiated_bundles);
        }
    }

    Ok(bundles)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn load_world_map() -> Result<()> {
        let settings = settings::load_settings()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.world_map_path.file_path)?;

        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        create_meshes_from_world_map_path(
            &settings.world_map_path.file_path,
            &file_info_map,
            &mut images,
            &mut materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
