// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::io;

use bevy::prelude::*;
use wow_adt as adt;
use wow_mpq as mpq;

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
            let mut models = Vec::new();
            if let Some(mmdx) = &world_map.mmdx {
                models = mmdx.filenames.clone();
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

            let info = WorldMapInfo {
                path: entry.name.clone(),
                models,
                world_models,
            };
            infos.push(info);
        }
    }

    Ok(infos)
}

fn read_world_map(path: &str, archive: &mut mpq::Archive) -> Result<adt::Adt> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(adt::Adt::from_reader(&mut reader)?)
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
