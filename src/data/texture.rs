// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::*,
    tasks::{self, Task},
};
use wow_blp as blp;

use crate::data::{archive, file};

#[derive(Clone)]
pub struct TextureInfo {
    image: blp::BlpImage,
}

impl TextureInfo {
    pub fn new(file_info: &file::FileInfo) -> Result<Self> {
        let mut archive = archive::get_archive!(&file_info.archive_path)?;
        let file = archive.read_file(&file_info.path)?;
        Ok(Self::from_blp(blp::parser::load_blp_from_buf(&file)?))
    }

    pub fn from_blp(image: blp::BlpImage) -> Self {
        Self { image }
    }

    pub fn get_dependencies(&self) -> Vec<String> {
        Vec::new()
    }
}

pub fn loading_texture_task(task: file::LoadFileTask) -> Task<file::LoadFileTask> {
    info!("Starting to load texture: {}", task.file.path);
    tasks::IoTaskPool::get().spawn(load_texture(task))
}

pub async fn load_texture(mut task: file::LoadFileTask) -> file::LoadFileTask {
    match TextureInfo::new(&task.file) {
        Ok(image) => {
            task.file.set_texture(image);
            info!("Loaded texture: {}", task.file.path);
        }
        Err(e) => {
            error!("Failed to load texture {}: {}", task.file.path, e);
            task.file.state = file::FileInfoState::Error(e.to_string());
        }
    }
    task
}

// Actually used in tests
#[allow(unused)]
pub fn is_texture_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".blp")
}

pub fn create_image_from_path(
    texture_path: &str,
    file_info_map: &file::FileInfoMap,
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
