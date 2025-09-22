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

/// Create a combined RGBA texture from the alpha maps of a world map chunk.
/// Each alpha map is stored in a separate channel:
/// - R: Level 1 alpha
/// - G: Level 2 alpha
/// - B: Level 3 alpha
/// - A: Unused
///
/// If `has_big_alpha` is true, each alpha value is 8 bits (1 byte),
/// otherwise as 4 bits (half a byte).
pub fn create_alpha_texture_from_world_map_chunk(
    chunk: &adt::McnkChunk,
    images: &mut Assets<Image>,
    has_big_alpha: bool,
) -> Handle<Image> {
    let image_size: Extent3d = Extent3d {
        width: 64,
        height: 64,
        depth_or_array_layers: 1,
    };

    let mut combined_alpha = vec![0u8; (image_size.width * image_size.height * 4) as usize];

    // Put level 1 alpha in R channel, level 2 in G channel, level 3 in B channel
    for (level, alpha) in chunk.alpha_maps.iter().enumerate() {
        // Offset by level to put in correct channel
        let mut combined_alpha_index = level;
        for &alpha_value in alpha.iter() {
            if has_big_alpha {
                if combined_alpha_index < combined_alpha.len() {
                    // alpha is one byte here
                    combined_alpha[combined_alpha_index] = alpha_value;
                    combined_alpha_index += 4;
                }
            } else {
                if combined_alpha_index < combined_alpha.len() {
                    // Convert 4-bit alpha to 8-bit alpha
                    // We set two pixels at a time since each byte contains two 4-bit alpha values
                    combined_alpha[combined_alpha_index] = (alpha_value & 0x0F) * 16;
                    combined_alpha_index += 4;
                }
                if combined_alpha_index < combined_alpha.len() {
                    combined_alpha[combined_alpha_index] = ((alpha_value >> 4) & 0x0F) * 16;
                    combined_alpha_index += 4;
                }
            }
        }
    }

    let image = Image::new_fill(
        image_size,
        TextureDimension::D2,
        &combined_alpha,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    images.add(image)
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
