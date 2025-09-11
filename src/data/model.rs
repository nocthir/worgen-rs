// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    io,
    path::{Path, PathBuf},
};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use wow_m2 as m2;
use wow_mpq as mpq;

use crate::data::normalize_vec3;

#[derive(Clone)]
pub struct ModelInfo {
    pub path: PathBuf,
    pub vertex_count: usize,
    pub texture_count: usize,
    pub materials: usize,
}

pub fn read_m2s(archive: &mut mpq::Archive) -> Result<Vec<ModelInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        if entry.name.ends_with(".m2")
            && let Ok(model) = read_m2(&entry.name, archive)
        {
            let vertex_count = model.vertices.len();
            let texture_count = model.textures.len();
            let materials = model.materials.len();
            let info = ModelInfo {
                path: PathBuf::from(&entry.name),
                vertex_count,
                texture_count,
                materials,
            };
            infos.push(info);
        }
    }

    Ok(infos)
}

fn read_m2<P: AsRef<Path>>(path: P, archive: &mut mpq::Archive) -> Result<m2::M2Model> {
    let file_path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format!("Invalid model path: {}", path.as_ref().display()))?;

    let file = archive
        .read_file(file_path)
        .map_err(|e| format!("Failed to read model file {}: {}", file_path, e))?;
    let mut reader = io::Cursor::new(&file);
    let model = m2::M2Model::parse(&mut reader)
        .map_err(|e| format!("Failed to parse model file {}: {}", file_path, e))?;
    Ok(model)
}

pub fn create_meshes_from_m2_path<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    path: P,
    standard_materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format!("Invalid model path: {}", path.as_ref().display()))?;

    let file = archive.read_file(path_str)?;
    let mut reader = io::Cursor::new(&file);

    let mut ret = Vec::default();

    let material_handle = standard_materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..Default::default()
    });

    if let Ok(m2) = m2::M2Model::parse(&mut reader)
        && !m2.vertices.is_empty()
    {
        for skin_index in 0..m2.embedded_skin_count().unwrap().min(1) {
            if let Ok(mesh) = create_mesh(&m2, &file, 0) {
                let mesh_handle = meshes.add(mesh);
                ret.push((mesh_handle, material_handle.clone()));
            } else {
                return Err(format!(
                    "Failed to create mesh for skin index {} in model {} from archive {}",
                    skin_index,
                    path_str,
                    archive.path().display(),
                )
                .into());
            }
        }
    }

    Ok(ret)
}

fn create_mesh(m2: &m2::M2Model, m2_data: &[u8], skin_index: u32) -> Result<Mesh> {
    let skin = m2.parse_embedded_skin(m2_data, skin_index as _)?;
    // These are used to index into the global vertices array.
    let indices = skin.get_resolved_indices();

    let mut positions = Vec::with_capacity(m2.vertices.len());
    let mut normals = Vec::with_capacity(m2.vertices.len());
    let mut tex_coords_0 = Vec::with_capacity(m2.vertices.len());

    for i in 0..m2.vertices.len() {
        let v = &m2.vertices[i];
        positions.push([v.position.x, v.position.y, v.position.z]);
        normals.push(normalize_vec3([v.normal.x, v.normal.y, v.normal.z]));
        tex_coords_0.push([v.tex_coords.x, v.tex_coords.y]);
    }

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
        positions,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords_0)
    .with_inserted_indices(Indices::U16(indices));

    Ok(mesh)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn main_menu() -> Result {
        let settings = settings::load_settings()?;
        let selected_model = ui::ModelSelected::from(&settings.default_model);
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_model(
            &selected_model,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn dwarf() -> Result {
        env_logger::init();
        let model = settings::load_settings()?;
        let selected_model = ui::ModelSelected::from(&model.test_model);
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_model(
            &selected_model,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
