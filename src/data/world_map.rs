// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::prelude::*;
use wow_adt as adt;
use wow_mpq as mpq;

use crate::data::{ModelBundle, archive, model, world_model};

#[derive(Default, Clone)]
pub struct WorldMapInfo {
    pub path: String,
    pub models: Vec<String>,
    pub world_models: Vec<String>,
}

impl WorldMapInfo {
    pub fn has_stuff(&self) -> bool {
        !self.models.is_empty() || !self.world_models.is_empty()
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
            infos.push(get_world_map_info(&world_map, &entry.name));
        }
    }

    Ok(infos)
}

fn get_world_map_info(world_map: &adt::Adt, file_name: &str) -> WorldMapInfo {
    let mut models = Vec::new();
    if let Some(mmdx) = &world_map.mmdx {
        for filename in &mmdx.filenames {
            if filename.ends_with(".m2") {
                models.push(filename.clone());
            } else if filename.ends_with(".mdx") {
                let file_path = Path::new(filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| format!("{s}.m2"))
                    .unwrap();
                models.push(file_path);
            } else if filename.ends_with(".mdl") {
                models.push(filename.clone());
            }
        }
    }

    let mut world_models = Vec::new();
    if let Some(modf) = &world_map.modf {
        let filenames = if let Some(mwmo) = &world_map.mwmo {
            mwmo.filenames.clone()
        } else {
            Vec::new()
        };
        for model in &modf.models {
            if let Some(filename) = filenames.get(model.name_id as usize) {
                world_models.push(filename.clone());
            }
        }
    }

    WorldMapInfo {
        path: file_name.to_string(),
        models,
        world_models,
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

pub fn create_meshes_from_world_map_path(
    world_map_path: &str,
    file_archive_map: &archive::FileArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let mut bundles = Vec::new();

    let mut archive = file_archive_map.get_archive(world_map_path)?;
    let world_map = read_world_map(world_map_path, &mut archive)?;
    let world_map_info = get_world_map_info(&world_map, world_map_path);

    let mut model_bundles = Vec::new();
    for model_path in &world_map_info.models {
        let bundles = model::create_meshes_from_model_path(
            model_path,
            file_archive_map,
            images,
            materials,
            meshes,
        )?;
        model_bundles.push(bundles);
    }

    if let Some(mddf) = &world_map.mddf {
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
    for world_model_path in &world_map_info.world_models {
        let bundles = world_model::create_meshes_from_world_model_path(
            world_model_path,
            file_archive_map,
            images,
            materials,
            meshes,
        )?;
        world_model_bundles.push(bundles);
    }

    if let Some(modf) = &world_map.modf {
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
    use crate::{data::texture, *};

    #[test]
    fn read_adt() -> Result<()> {
        let settings = settings::load_settings()?;
        let mut file_archive_map = texture::test::default_file_archive_map(&settings)?;
        file_archive_map.fill_models(&settings.model_archive_path)?;
        file_archive_map.fill_world_map(&settings.world_map_path.archive_path)?;
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        create_meshes_from_world_map_path(
            &settings.world_map_path.file_path,
            &file_archive_map,
            &mut images,
            &mut materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
