// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::prelude::*;
use wow_adt as adt;
use wow_mpq as mpq;

use crate::data::{model, texture, world_model};

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
        if let Ok(world_map_info) = read_world_map_info(&entry.name, archive) {
            infos.push(world_map_info);
        }
    }

    Ok(infos)
}

fn read_world_map_info(file_name: &str, archive: &mut mpq::Archive) -> Result<WorldMapInfo> {
    let world_map = read_world_map(file_name, archive)?;
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

    Ok(WorldMapInfo {
        path: file_name.to_string(),
        models,
        world_models,
    })
}

fn read_world_map(path: &str, archive: &mut mpq::Archive) -> Result<adt::Adt> {
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
    file_archive_map: &texture::FileArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    let mut bundles = Vec::new();

    let mut archive = file_archive_map.get_archive(world_map_path)?;
    let world_map_info = read_world_map_info(world_map_path, &mut archive)?;

    for model_path in &world_map_info.models {
        bundles.extend(model::create_meshes_from_model_path(
            model_path,
            file_archive_map,
            images,
            materials,
            meshes,
        )?);
    }

    for world_model_path in &world_map_info.world_models {
        bundles.extend(world_model::create_meshes_from_world_model_path(
            world_model_path,
            file_archive_map,
            images,
            materials,
            meshes,
        )?);
    }

    Ok(bundles)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn read_adt() -> Result<()> {
        let settings = settings::load_settings()?;
        let mut archive = mpq::Archive::open(&settings.world_map_path.archive_path)?;
        let file = archive.read_file(&settings.world_map_path.file_path)?;
        assert!(!file.is_empty());
        let mut reader = io::Cursor::new(file);
        adt::Adt::from_reader(&mut reader)?;
        Ok(())
    }
}
