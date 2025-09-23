// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::*};

use wow_adt as adt;
use wow_m2 as m2;

use crate::data::{file, texture, world_model::WorldModelInfo};

pub fn create_textures_from_world_model(
    world_model: &WorldModelInfo,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
) -> Result<Vec<Handle<Image>>> {
    let mut image_handles = Vec::new();
    for texture_path in &world_model.get_texture_paths() {
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
///
/// If `do_not_fix_alpha` is true, we should read a 63*63 map with the last row
/// and column being equivalent to the previous one
pub fn create_alpha_texture_from_world_map_chunk(
    chunk: &adt::McnkChunk,
    images: &mut Assets<Image>,
    has_big_alpha: bool,
    do_not_fix_alpha: bool,
) -> Handle<Image> {
    let mut combined_alpha = CombinedAlphaMap::new(do_not_fix_alpha);

    // Put level 1 alpha in R channel, level 2 in G channel, level 3 in B channel
    for (level, alpha) in chunk.alpha_maps.iter().enumerate() {
        for &alpha_value in alpha.iter() {
            if has_big_alpha {
                // alpha is one byte here
                combined_alpha.set_next_alpha(level, alpha_value);
            } else {
                // Convert 4-bit alpha to 8-bit alpha
                // We set two pixels at a time since each byte contains two 4-bit alpha values
                combined_alpha.set_next_alpha(level, (alpha_value & 0x0F) * 16);
                combined_alpha.set_next_alpha(level, ((alpha_value >> 4) & 0x0F) * 16);
            }
        }
    }

    images.add(combined_alpha.to_image())
}

struct CombinedAlphaMap {
    map: [[[u8; 4]; 64]; 64],
    current_x: usize,
    current_y: usize,

    /// If `do_not_fix` is true, we should read a 63*63 map with the last row
    /// and column being equivalent to the previous one
    do_not_fix: bool,
}

impl CombinedAlphaMap {
    fn new(do_not_fix: bool) -> Self {
        Self {
            map: [[[0u8; 4]; 64]; 64],
            current_x: 0,
            current_y: 0,
            do_not_fix,
        }
    }

    fn set_alpha(&mut self, x: usize, y: usize, level: usize, alpha: u8) {
        if y < 64 && x < 64 && level < 4 {
            self.map[y][x][level] = alpha;
        }
    }

    fn set_next_alpha(&mut self, level: usize, alpha: u8) {
        self.set_alpha(self.current_x, self.current_y, level, alpha);
        self.advance();

        // If we are at the last row or column and do_not_fix is true,
        // duplicate the last value to fill the 64x64 texture
        if self.do_not_fix {
            if self.current_x == 63 {
                self.set_alpha(self.current_x, self.current_y, level, alpha);
                self.advance();
            }
            if self.current_y == 63 {
                let prev_x = if self.current_x == 0 {
                    63
                } else {
                    self.current_x - 1
                };
                let alpha = self.map[self.current_y - 1][prev_x][level];
                self.set_alpha(self.current_x, self.current_y, level, alpha);
                self.advance();
            }
        }
    }

    fn advance(&mut self) {
        self.current_x += 1;
        if self.current_x >= 64 {
            self.current_x = 0;
            self.current_y += 1;
        }
    }

    fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.map.as_ptr() as *const u8,
                std::mem::size_of_val(&self.map),
            )
        }
    }

    fn to_image(&self) -> Image {
        let image_size: Extent3d = Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        };
        Image::new_fill(
            image_size,
            TextureDimension::D2,
            self.as_slice(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
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
