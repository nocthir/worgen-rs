// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{
    asset::RenderAssetUsages,
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use wow_blp as blp;
use wow_m2 as m2;
use wow_mpq as mpq;
use wow_wmo as wmo;

#[derive(Resource, Default)]
pub struct TextureArchiveMap {
    pub map: HashMap<String, String>,
}

impl TextureArchiveMap {
    // Actually used in tests
    #[allow(unused)]
    pub fn fill<S: Into<String>>(&mut self, archive_path: S) -> Result<()> {
        let archive_path = archive_path.into();
        println!("Filling texture archive map from {}", archive_path);
        let mut archive = mpq::Archive::open(&archive_path)?;

        for file in archive.list()? {
            if file.name.to_lowercase().ends_with(".blp") {
                println!("Mapping texture {} to archive {}", file.name, archive_path);
                self.map
                    .insert(file.name.to_lowercase(), archive_path.clone());
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct TextureInfo {
    pub texture_path: String,
}

pub fn read_textures(archive: &mut mpq::Archive) -> Result<Vec<TextureInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        let lowercase_name = entry.name.to_lowercase();
        if lowercase_name.ends_with(".blp") {
            let texture_info = TextureInfo {
                texture_path: entry.name.clone(),
            };
            infos.push(texture_info);
        }
    }
    Ok(infos)
}

pub fn create_textures_from_wmo(
    wmo: &wmo::WmoRoot,
    texture_archive_map: &TextureArchiveMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut image_handles = Vec::new();
    for texture_path in &wmo.textures {
        // At this point we do not know which archive contains this texture.
        // But we have built a map of blp paths to their respective archives.
        let image_handle = create_image_from_path(texture_path, texture_archive_map, images)?;
        image_handles.push(image_handle);
    }
    Ok(image_handles)
}

pub fn create_textures_from_model(
    model: &m2::M2Model,
    texture_archive_map: &TextureArchiveMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut handles = Vec::new();
    for texture in &model.textures {
        // Case insensitive texture filename.
        let texture_path = texture.filename.string.to_string_lossy();
        let image_handle = create_image_from_path(&texture_path, texture_archive_map, images)?;
        handles.push(image_handle);
    }
    Ok(handles)
}

pub fn create_image_from_path(
    texture_path: &str,
    texture_archive_map: &TextureArchiveMap,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>> {
    // Case insensitive texture filename.
    let texture_path = texture_path.to_lowercase();

    let archive_path = texture_archive_map
        .map
        .get(&texture_path)
        .ok_or_else(|| format!("Texture {} not found in any loaded archive", texture_path))?;

    let mut archive = mpq::Archive::open(archive_path)
        .map_err(|e| format!("Failed to open archive {}: {}", archive_path, e))?;
    let file = archive.read_file(&texture_path)?;
    let blp = blp::parser::load_blp_from_buf(&file)?;
    let dyn_image = blp::convert::blp_to_image(&blp, 0)?;
    let extent = Extent3d {
        width: dyn_image.width(),
        height: dyn_image.height(),
        depth_or_array_layers: 1,
    };
    let dimension = TextureDimension::D2;
    let data = dyn_image.to_rgba8().into_raw();
    let texture_format = TextureFormat::Rgba8Unorm;
    let usage = RenderAssetUsages::RENDER_WORLD;
    let image = Image::new(extent, dimension, data, texture_format, usage);
    let image_handle = images.add(image);
    Ok(image_handle)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::*;

    pub fn default_texture_archive_map(settings: &settings::Settings) -> Result<TextureArchiveMap> {
        let mut texture_archive_map = TextureArchiveMap::default();
        texture_archive_map.fill(&settings.interface_archive_path)?;
        texture_archive_map.fill(&settings.texture_archive_path)?;
        Ok(texture_archive_map)
    }
}
