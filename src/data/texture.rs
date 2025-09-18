// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    tasks::{self, Task},
};
use wow_blp as blp;
use wow_m2 as m2;
use wow_wmo as wmo;

use crate::data::{
    archive,
    file::{DataInfo, FileInfo, FileInfoMap},
};

#[derive(Clone)]
pub struct TextureInfo {
    image: blp::BlpImage,
}

pub fn load_texture_info(texture_path: &str, archive_path: &str) -> Result<DataInfo> {
    let mut archive = archive::open_archive(archive_path)?;
    let file = archive.read_file(texture_path)?;
    let blp = blp::parser::load_blp_from_buf(&file)?;
    Ok(DataInfo::Texture(TextureInfo { image: blp }))
}

pub fn loading_texture_task(texture_file_info: &FileInfo) -> Task<Result<FileInfo>> {
    info!("Starting to load texture: {}", texture_file_info.path);
    tasks::IoTaskPool::get().spawn(load_texture(
        texture_file_info.path.clone(),
        texture_file_info.archive_path.clone(),
    ))
}

pub async fn load_texture(path: String, archive_path: std::path::PathBuf) -> Result<FileInfo> {
    let mut archive = archive::open_archive(archive_path)?;
    let file = archive.read_file(&path)?;
    let blp = blp::parser::load_blp_from_buf(&file)?;
    Ok(FileInfo::new_texture(
        path,
        archive.path(),
        TextureInfo { image: blp },
    ))
}

pub fn create_textures_from_wmo(
    wmo: &wmo::WmoRoot,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut image_handles = Vec::new();
    for texture_path in &wmo.textures {
        // At this point we do not know which archive contains this texture.
        // But we have built a map of blp paths to their respective archives.
        let image_handle = create_image_from_path(texture_path, file_info_map, images)?;
        image_handles.push(image_handle);
    }
    Ok(image_handles)
}

pub fn create_textures_from_model(
    model: &m2::M2Model,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut handles = Vec::new();
    let default_texture = images.add(Image::default());
    for texture in &model.textures {
        if texture.texture_type == m2::chunks::M2TextureType::Hardcoded {
            // Case insensitive texture filename.
            let texture_path = texture.filename.string.to_string_lossy();
            let image_handle = create_image_from_path(&texture_path, file_info_map, images)?;
            handles.push(image_handle);
        } else {
            // Ignore non-hardcoded textures for now.
            handles.push(default_texture.clone())
        }
    }
    Ok(handles)
}

pub fn create_image_from_path(
    texture_path: &str,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>> {
    let texture = file_info_map.get_texture_info(texture_path)?;
    let dyn_image = blp::convert::blp_to_image(&texture.image, 0)?;
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
    use crate::{data::archive::ArchiveInfo, *};

    pub fn default_file_info_map(settings: &settings::Settings) -> Result<FileInfoMap> {
        let mut file_info_map = FileInfoMap::default();
        let mut archive_info = ArchiveInfo::new(&settings.interface_archive_path)?;
        file_info_map.fill(&mut archive_info)?;
        let mut archive_info = ArchiveInfo::new(&settings.texture_archive_path)?;
        file_info_map.fill(&mut archive_info)?;
        Ok(file_info_map)
    }
}
