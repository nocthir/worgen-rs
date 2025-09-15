// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::io;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use wow_m2 as m2;
use wow_mpq as mpq;

use crate::data::{
    normalize_vec3,
    texture::{self, TextureArchiveMap},
};

#[derive(Clone)]
pub struct ModelInfo {
    pub path: String,
    pub vertex_count: usize,
    pub texture_count: usize,
    pub materials: usize,
}

pub fn read_models(archive: &mut mpq::Archive) -> Result<Vec<ModelInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        if is_model_extension(&entry.name)
            && let Ok(model) = read_model(&entry.name, archive)
        {
            let vertex_count = model.vertices.len();
            let texture_count = model.textures.len();
            let materials = model.materials.len();
            let info = ModelInfo {
                path: entry.name.clone(),
                vertex_count,
                texture_count,
                materials,
            };
            infos.push(info);
        }
    }

    Ok(infos)
}

pub fn is_model_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".m2")
        || lower_filename.ends_with(".mdx")
        || lower_filename.ends_with(".mdl")
}

fn read_model(file_path: &str, archive: &mut mpq::Archive) -> Result<m2::M2Model> {
    let file = archive
        .read_file(file_path)
        .map_err(|e| format!("Failed to read model file {}: {}", file_path, e))?;
    let mut reader = io::Cursor::new(&file);
    let model = m2::M2Model::parse(&mut reader)
        .map_err(|e| format!("Failed to parse model file {}: {}", file_path, e))?;
    Ok(model)
}

pub fn create_meshes_from_model_path(
    archive: &mut mpq::Archive,
    file_path: &str,
    texture_archive_map: &TextureArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    let data = archive.read_file(file_path)?;
    let mut reader = io::Cursor::new(&data);

    let mut ret = Vec::default();

    if let Ok(model) = m2::M2Model::parse(&mut reader)
        && !model.vertices.is_empty()
    {
        let image_handles =
            texture::create_textures_from_model(&model, texture_archive_map, images)?;
        if let Ok(res) = create_mesh(&model, &data, &image_handles, materials, meshes) {
            ret.extend(res);
        } else {
            return Err(format!(
                "Failed to create mesh for model {} from archive {}",
                file_path,
                archive.path().display(),
            )
            .into());
        }
    }

    Ok(ret)
}

fn create_mesh(
    model: &m2::M2Model,
    model_data: &[u8],
    image_handles: &[Handle<Image>],
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    let skin = model.parse_embedded_skin(model_data, 0)?;

    let vertex_count = model.vertices.len();
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut tex_coords_0 = Vec::with_capacity(vertex_count);
    for vertex in model.vertices.iter() {
        positions.push([vertex.position.x, vertex.position.y, vertex.position.z]);
        normals.push(normalize_vec3([
            vertex.normal.x,
            vertex.normal.y,
            vertex.normal.z,
        ]));
        tex_coords_0.push([vertex.tex_coords.x, vertex.tex_coords.y]);
    }

    let mut ret = Vec::new();
    for batch_index in 0..skin.batches().len() {
        ret.push(create_mesh_for_submesh(
            model,
            &skin,
            batch_index,
            &positions,
            &normals,
            &tex_coords_0,
            image_handles,
            materials,
            meshes,
        )?);
    }
    Ok(ret)
}

#[allow(clippy::too_many_arguments)]
fn create_mesh_for_submesh(
    model: &m2::M2Model,
    skin: &m2::skin::SkinFile,
    batch_index: usize,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tex_coords_0: &[[f32; 2]],
    image_handles: &[Handle<Image>],
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<(Handle<Mesh>, Handle<StandardMaterial>)> {
    let batch = &skin.batches()[batch_index];
    let submesh = &skin.submeshes()[batch.skin_section_index as usize];
    let texture_index =
        model.raw_data.texture_lookup_table[batch.texture_combo_index as usize] as usize;
    let texture = &image_handles[texture_index];

    // Determine alpha mode from material blend mode.
    // Note that multiple batches can share the same material.
    let model_material = &model.materials[batch.material_index as usize];
    let alpha_mode = blend_mode_to_alpha_mode(model_material.blend_mode);

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture.clone()),
        perceptual_roughness: 1.0,
        alpha_mode,
        ..Default::default()
    });

    // Index into local arrays
    assert_eq!(submesh.triangle_start % 3, 0);
    assert_eq!(submesh.triangle_count % 3, 0);
    let submesh_indices = skin.get_resolved_indices()[submesh.triangle_start as usize
        ..(submesh.triangle_start + submesh.triangle_count) as usize]
        .to_vec();

    // Keep the mesh data accessible in future frames to be able to mutate it in toggle_texture.
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        // Each array is an [x, y, z] coordinate in local space.
        // The camera coordinate space is right-handed x-right, y-up, z-back. This means "forward" is -Z.
        // Meshes always rotate around their local [0, 0, 0] when a rotation is applied to their Transform.
        // By centering our mesh around the origin, rotating the mesh preserves its center of mass.
        positions.to_vec(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords_0.to_vec())
    .with_inserted_indices(Indices::U16(submesh_indices));

    let mesh_handle = meshes.add(mesh);

    Ok((mesh_handle, material_handle))
}

fn blend_mode_to_alpha_mode(blend_mode: m2::chunks::material::M2BlendMode) -> AlphaMode {
    use m2::chunks::material::M2BlendMode as BM;
    if blend_mode.intersects(BM::ALPHA_KEY | BM::NO_ALPHA_ADD) {
        AlphaMode::AlphaToCoverage
    } else if blend_mode.intersects(BM::ADD | BM::BLEND_ADD) {
        AlphaMode::Add
    } else if blend_mode.intersects(BM::MOD | BM::MOD2X) {
        AlphaMode::Multiply
    } else if blend_mode.intersects(BM::ALPHA) {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{data::texture, *};

    #[test]
    fn main_menu() -> Result {
        let settings = settings::load_settings()?;
        let texture_archive_map = texture::test::default_texture_archive_map(&settings)?;
        let selected_model = ui::ModelSelected::from(&settings.default_model);
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_model(
            &selected_model,
            &texture_archive_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn city() -> Result {
        let settings = settings::load_settings()?;
        let texture_archive_map = texture::test::default_texture_archive_map(&settings)?;
        let selected_model = ui::ModelSelected::from(&settings.city_model);
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_model(
            &selected_model,
            &texture_archive_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn dwarf() -> Result {
        env_logger::init();
        let settings = settings::load_settings()?;
        let texture_archive_map = texture::test::default_texture_archive_map(&settings)?;
        let selected_model = ui::ModelSelected::from(&settings.test_model);
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_model(
            &selected_model,
            &texture_archive_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
