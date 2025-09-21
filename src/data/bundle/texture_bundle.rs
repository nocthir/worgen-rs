// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::*};

use wow_adt as adt;
use wow_m2 as m2;
use wow_wmo as wmo;

use crate::data::{file, texture};

pub fn create_textures_from_world_model(
    wmo: &wmo::WmoRoot,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut image_handles = Vec::new();
    for texture_path in &wmo.textures {
        // At this point we do not know which archive contains this texture.
        // But we have built a map of blp paths to their respective archives.
        let image_handle = texture::create_image_from_path(texture_path, file_info_map, images)?;
        image_handles.push(image_handle);
    }
    Ok(image_handles)
}

pub fn create_textures_from_model(
    model: &m2::M2Model,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut handles = Vec::new();
    let default_texture = images.add(Image::default());
    for texture in &model.textures {
        if texture.texture_type == m2::chunks::M2TextureType::Hardcoded {
            // Case insensitive texture filename.
            let texture_path = texture.filename.string.to_string_lossy();
            let image_handle =
                texture::create_image_from_path(&texture_path, file_info_map, images)?;
            handles.push(image_handle);
        } else {
            // Ignore non-hardcoded textures for now.
            handles.push(default_texture.clone())
        }
    }
    Ok(handles)
}

pub fn create_textures_from_world_map(
    world_map: &adt::Adt,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut image_handles = Vec::new();
    let Some(texture_chunk) = &world_map.mtex else {
        return Ok(image_handles);
    };
    for texture_path in &texture_chunk.filenames {
        // At this point we do not know which archive contains this texture.
        // But we have built a map of blp paths to their respective archives.
        let image_handle = texture::create_image_from_path(texture_path, file_info_map, images)?;
        image_handles.push(image_handle);
    }
    Ok(image_handles)
}

pub fn create_alpha_textures_from_world_map_chunk(
    chunk: &adt::McnkChunk,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    static IMAGE_SIZE: Extent3d = Extent3d {
        width: 64,
        height: 64,
        depth_or_array_layers: 1,
    };

    let mut image_handles = Vec::new();

    for alpha_map in &chunk.alpha_maps {
        let image = Image::new_fill(
            IMAGE_SIZE,
            TextureDimension::D2,
            alpha_map,
            TextureFormat::R8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image_handles.push(images.add(image));
    }

    Ok(image_handles)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn list_texture_paths() -> Result {
        let settings = settings::TestSettings::load()?;
        let file_info_map = file::test::default_file_info_map(&settings)?;
        println!("Path, Archive");
        for file_info in file_info_map.get_file_infos() {
            if texture::is_texture_extension(&file_info.path) {
                println!("{}, {}", file_info.path, file_info.archive_path.display());
            }
        }
        Ok(())
    }

    #[test]
    fn load_texture() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.test_texture.file_path)?;
        let mut images = Assets::<Image>::default();
        texture::create_image_from_path(
            &settings.test_texture.file_path,
            &file_info_map,
            &mut images,
        )?;
        Ok(())
    }
}
