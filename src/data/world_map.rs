// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::io;

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
}

impl WorldMapInfo {
    pub fn new(mut world_map: adt::Adt) -> Self {
        Self::fix_model_extensions(&mut world_map);
        let model_paths = Self::get_model_paths(&world_map);
        Self {
            world_map,
            model_paths,
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

    pub fn has_stuff(&self) -> bool {
        self.world_map
            .mmdx
            .as_ref()
            .is_some_and(|mmdx| !mmdx.filenames.is_empty())
            || self
                .world_map
                .modf
                .as_ref()
                .is_some_and(|modf| !modf.models.is_empty())
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

    pub fn get_world_model_paths(&self) -> Vec<&String> {
        let mut world_models = Vec::new();
        if let Some(modf) = &self.world_map.modf
            && let Some(mwmo) = &self.world_map.mwmo
        {
            let filenames = &mwmo.filenames;
            for model in &modf.models {
                if let Some(filename) = filenames.get(model.name_id as usize) {
                    world_models.push(filename);
                }
            }
        }
        world_models
    }
}

pub fn read_world_maps(archive: &mut mpq::Archive) -> Result<Vec<WorldMapInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        let lowercase_name = entry.name.to_lowercase();
        if !lowercase_name.ends_with(".adt") {
            continue;
        }
        if let Ok(world_map) = read_world_map(&entry.name, archive) {
            infos.push(WorldMapInfo::new(world_map));
        }
    }

    Ok(infos)
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

pub fn start_loading_world_map(tasks: &mut file::LoadFileTask, file_info: &file::FileInfo) {
    info!("Starting to load world map: {}", file_info.path);
    let task = tasks::IoTaskPool::get().spawn(load_world_map(file_info.shallow_clone()));
    tasks.tasks.push(task);
}

async fn load_world_map(mut file_info: file::FileInfo) -> Result<file::FileInfo> {
    match load_world_map_impl(&file_info) {
        Ok(world_map_info) => {
            file_info.set_world_map(world_map_info);
            info!("Loaded world map: {}", file_info.path);
            Ok(file_info)
        }
        Err(e) => {
            error!("Failed to load world map {}: {}", file_info.path, e);
            file_info.state = file::FileInfoState::Error(e.to_string());
            Ok(file_info)
        }
    }
}

fn load_world_map_impl(file_info: &file::FileInfo) -> Result<WorldMapInfo> {
    let mut archive = mpq::Archive::open(&file_info.archive_path)?;
    let world_map = read_world_map(&file_info.path, &mut archive)?;
    Ok(WorldMapInfo::new(world_map))
}

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
    for world_model_path in &world_map_info.get_world_model_paths() {
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
    use crate::{
        data::{archive, texture},
        *,
    };

    #[test]
    fn read_adt() -> Result<()> {
        let settings = settings::load_settings()?;
        let mut file_info_map = texture::test::default_file_info_map(&settings)?;
        let mut model_archive_info = archive::ArchiveInfo::new(&settings.model_archive_path)?;
        file_info_map.fill(&mut model_archive_info)?;
        let mut world_map_archive_info =
            archive::ArchiveInfo::new(&settings.world_map_path.archive_path)?;
        file_info_map.fill(&mut world_map_archive_info)?;
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
