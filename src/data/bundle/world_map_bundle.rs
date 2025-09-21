// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::RenderAssetUsages, pbr::ExtendedMaterial, prelude::*, render::mesh::*};

use wow_adt as adt;

use crate::{
    data::{bundle::*, file::FileInfoMap, world_map::WorldMapInfo},
    material::TerrainMaterial,
};

// Actually used in tests
#[allow(unused)]
pub fn create_meshes_from_world_map_path(
    world_map_path: &str,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    terrain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<(Vec<TerrainBundle>, Vec<ModelBundle>)> {
    let world_map_info = file_info_map.get_world_map_info(world_map_path)?;
    create_meshes_from_world_map_info(
        world_map_info,
        file_info_map,
        images,
        terrain_materials,
        materials,
        meshes,
    )
}

pub fn create_meshes_from_world_map_info(
    world_map_info: &WorldMapInfo,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    terrain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<(Vec<TerrainBundle>, Vec<ModelBundle>)> {
    let mut terrain_bundles = Vec::new();
    let mut model_bundles = Vec::new();

    terrain_bundles.extend(create_terrain_bundles_from_world_map_info(
        world_map_info,
        file_info_map,
        images,
        terrain_materials,
        meshes,
    )?);

    model_bundles.extend(create_model_bundles_from_world_map_info(
        world_map_info,
        file_info_map,
        images,
        materials,
        meshes,
    )?);

    model_bundles.extend(create_world_model_bundles_from_world_map_info(
        world_map_info,
        file_info_map,
        images,
        materials,
        meshes,
    )?);

    Ok((terrain_bundles, model_bundles))
}

fn create_terrain_bundles_from_world_map_info(
    world_map_info: &WorldMapInfo,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<TerrainBundle>> {
    let mut bundles = Vec::new();

    let textures =
        create_textures_from_world_map(&world_map_info.world_map, file_info_map, images)?;

    for chunk in &world_map_info.world_map.mcnk_chunks {
        let mesh = create_mesh_from_world_map_chunk(chunk);
        let mesh_handle = meshes.add(mesh);

        let mut level0_texture_handle = None;
        let mut level1_texture_handle = None;

        if let Some(level0) = chunk.texture_layers.first() {
            // Use just the first layer texture for now
            let texture_index = level0.texture_id as usize;
            level0_texture_handle = textures.get(texture_index).cloned();
        }

        if let Some(level1) = chunk.texture_layers.get(1) {
            // Use the second layer texture for now
            let texture_index = level1.texture_id as usize;
            level1_texture_handle = textures.get(texture_index).cloned();
        }

        let material = StandardMaterial {
            base_color_texture: level0_texture_handle,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            unlit: false,
            cull_mode: None,
            ..Default::default()
        };
        let terrain_material = TerrainMaterial {
            level_texture: level1_texture_handle,
        };

        let extended_material = ExtendedMaterial {
            base: material,
            extension: terrain_material,
        };

        let material_handle = materials.add(extended_material);

        // Each chunk is 33.33 units (100.0 / 3.0) in the world space.
        // Our grid size is 8, so we scale by (100.0 / 3.0) / 8.0 = 100.0 / 24.0
        static CHUNK_SCALE: f32 = 100.0 / 24.0;

        let transform = Transform::default()
            .with_translation(vec3(
                17066.0 - chunk.position[0],
                chunk.position[1],
                17066.0 - chunk.position[2],
            ))
            .with_scale(vec3(-CHUNK_SCALE, CHUNK_SCALE, 1.0))
            .with_rotation(Quat::from_axis_angle(Vec3::Y, -std::f32::consts::FRAC_PI_2));

        bundles.push(TerrainBundle {
            mesh: Mesh3d(mesh_handle),
            material: MeshMaterial3d(material_handle),
            transform,
        });
    }

    Ok(bundles)
}

/// Create mesh for a terrain chunk.
///
/// # Vertex layout
/// - Each chunk uses 145 vertices arranged in a staggered 17-row grid. Rows alternate
///   between 9 and 8 vertices, starting with 9 on row 0. The layout (0-based indices):
///
/// - `VERTEX_COUNT` is set to 145 (8*8 + 9*9). Positions are computed from a staggered grid
///   with a 0.5 step in the packed ADT layout; z comes from the `height_map` vector.
///
/// # Index buffer and winding
/// - The index buffer is produced by `terrain_indices()` and uses 0-based indices.
/// - The generator produces 256 triangles (768 indices) by constructing a 4-triangle fan
///   per quad using the middle (odd-row) vertex as the center. The triangles are emitted
///   with counter-clockwise (CCW) winding so front faces are consistent with the engine's
///   default.
pub fn create_mesh_from_world_map_chunk(chunk: &adt::McnkChunk) -> Mesh {
    static VERTEX_COUNT: usize = 145; // 8*8 + 9*9
    let mut positions = vec![[0.0, 0.0, 0.0]; VERTEX_COUNT];
    let mut normals = vec![[0.0, 0.0, 0.0]; VERTEX_COUNT];
    let mut tex_coords = vec![[0.0, 0.0]; VERTEX_COUNT];
    let indices = terrain_indices();

    for i in 0..VERTEX_COUNT {
        // With these offset we can imagine 17 vertices for the
        // first 8 rows, and 9 vertices for the last row.
        let row_index = i % 17;
        let y_offset = (i / 17) as f32;
        let x_offset = row_index as f32;

        let mut y_suboffset = 0.0;
        let mut x_suboffset = 0.0;
        // Step is 0.5
        if row_index >= 9 {
            // Move the last 8 vertices of this row to a new line (control)
            y_suboffset = 0.5;
            x_suboffset = 0.5 - 9.0;
        }

        let x = x_offset + x_suboffset;
        let y = y_offset + y_suboffset;

        tex_coords[i] = [x / 8.0, y / 8.0];
        positions[i] = [x, y, chunk.height_map[i]];
        normals[i] = from_normalized_vec3_u8(chunk.normals[i]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        // Each array is an [x, y, z] coordinate in local space.
        // The camera coordinate space is right-handed x-right, y-up, z-back. This means "forward" is -Z.
        // Meshes always rotate around their local [0, 0, 0] when a rotation is applied to their Transform.
        // By centering our mesh around the origin, rotating the mesh preserves its center of mass.
        positions.clone(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords.clone())
    .with_inserted_indices(Indices::U16(indices.clone()))
}

/// Generate the triangle index buffer (CCW) for a 145-vertex chunk:
/// rows alternate 9 and 8 vertices (starting with 9), for 17 total rows.
/// Returns 256 triangles = 768 indices (u16), suitable for draw_indexed.
pub fn terrain_indices() -> Vec<u16> {
    fn row_len(r: usize) -> usize {
        if r.is_multiple_of(2) { 9 } else { 8 }
    }
    fn row_start(r: usize) -> usize {
        // Prefix sum of row lengths up to r (exclusive)
        (0..r).map(row_len).sum()
    }

    // We build 4 triangles per quad (8x8 quads between even rows), using the center vertex
    // from the odd row between two even rows. This yields 256 triangles total.
    // For each band of rows (even r, odd r+1, even r+2) and each column c in 0..8 (quads 0..7),
    // we define the following vertices:
    //   t0 = (even r, c)
    //   t1 = (even r, c+1)
    //   m  = (odd  r+1, c)           // center of the quad
    //   b0 = (even r+2, c)
    //   b1 = (even r+2, c+1)
    // And add triangles (CCW):
    //   (t0, b0, m), (m, b0, b1), (m, b1, t1), (m, t1, t0)

    let mut indices = Vec::with_capacity(256 * 3);

    for r in (0..=14).step_by(2) {
        let top_start = row_start(r);
        let mid_start = row_start(r + 1);
        let bot_start = row_start(r + 2);

        for c in 0..8 {
            let t0 = top_start + c;
            let t1 = top_start + c + 1;
            let m = mid_start + c;
            let b0 = bot_start + c;
            let b1 = bot_start + c + 1;

            indices.extend_from_slice(&[
                t0 as u16, b0 as u16, m as u16, m as u16, b0 as u16, b1 as u16, m as u16,
                b1 as u16, t1 as u16, m as u16, t1 as u16, t0 as u16,
            ]);
        }
    }

    indices
}

fn create_model_bundles_from_world_map_info(
    world_map_info: &WorldMapInfo,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let mut bundles = Vec::new();

    let mut model_bundles = Vec::new();
    for model_path in &world_map_info.model_paths {
        let bundles =
            create_meshes_from_model_path(model_path, file_info_map, images, materials, meshes)?;
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

    Ok(bundles)
}

fn create_world_model_bundles_from_world_map_info(
    world_map_info: &WorldMapInfo,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let mut bundles = Vec::new();

    let mut world_model_bundles = Vec::new();
    for world_model_path in &world_map_info.world_model_paths {
        let bundles = create_meshes_from_world_model_path(
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
    use crate::{data::bundle, *};

    #[test]
    fn load_world_map() -> Result<()> {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.world_map_path.file_path)?;

        let mut images = Assets::<Image>::default();
        let mut terrain_materials =
            Assets::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        bundle::create_meshes_from_world_map_path(
            &settings.world_map_path.file_path,
            &file_info_map,
            &mut images,
            &mut terrain_materials,
            &mut materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
