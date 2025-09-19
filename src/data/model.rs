// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
    tasks::{self, Task},
};
use wow_m2 as m2;
use wow_mpq as mpq;

use crate::data::{ModelBundle, file, material, normalize_vec3, texture};

#[derive(Clone)]
pub struct ModelInfo {
    pub model: m2::M2Model,
    pub data: Vec<u8>,
}

impl ModelInfo {
    pub fn new<P: AsRef<Path>>(file_path: &str, archive_path: P) -> Result<Self> {
        let mut archive = mpq::Archive::open(archive_path)?;
        let data = archive.read_file(file_path)?;
        let mut reader = io::Cursor::new(&data);
        let model = m2::M2Model::parse(&mut reader)?;
        Ok(Self { model, data })
    }

    pub fn get_texture_paths(&self) -> Vec<String> {
        self.model
            .textures
            .iter()
            .filter(|t| t.texture_type == m2::chunks::M2TextureType::Hardcoded)
            .map(|t| t.filename.string.to_string_lossy())
            .collect()
    }
}

pub fn is_model_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".m2")
        || lower_filename.ends_with(".mdx")
        || lower_filename.ends_with(".mdl")
}

pub fn loading_model_task(task: file::LoadFileTask) -> Task<file::LoadFileTask> {
    info!("Starting to load model: {}", task.file.path);
    tasks::IoTaskPool::get().spawn(load_model(task))
}

async fn load_model(mut task: file::LoadFileTask) -> file::LoadFileTask {
    match ModelInfo::new(&task.file.path, &task.file.archive_path) {
        Ok(model_info) => {
            task.file.set_model(model_info);
            info!("Loaded model: {}", task.file.path);
        }
        Err(e) => {
            error!("Failed to load model {}: {}", task.file.path, e);
            task.file.state = file::FileInfoState::Error(e.to_string());
        }
    }
    task
}

pub fn create_meshes_from_model_path(
    model_path: &str,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let model_info = file_info_map.get_model_info(model_path)?;
    create_meshes_from_model_info(model_info, file_info_map, images, materials, meshes)
}

pub fn create_meshes_from_model_info(
    model_info: &ModelInfo,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let mut ret = Vec::default();

    if !model_info.model.vertices.is_empty() {
        let image_handles =
            texture::create_textures_from_model(&model_info.model, file_info_map, images)?;
        let res = create_mesh(
            &model_info.model,
            &model_info.data,
            &image_handles,
            materials,
            meshes,
        )?;
        ret.extend(res);
    }

    Ok(ret)
}

/// Helper class to reduce the number of parameters passed around when creating meshes.
struct VertexAttributes {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tex_coords_0: Vec<[f32; 2]>,
}

impl VertexAttributes {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity),
            normals: Vec::with_capacity(capacity),
            tex_coords_0: Vec::with_capacity(capacity),
        }
    }
}

fn create_mesh(
    model: &m2::M2Model,
    model_data: &[u8],
    image_handles: &[Handle<Image>],
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let skin = model.parse_embedded_skin(model_data, 0)?;

    let vertex_count = model.vertices.len();
    let mut vertex_attributes = VertexAttributes::with_capacity(vertex_count);
    for vertex in model.vertices.iter() {
        vertex_attributes
            .positions
            .push([vertex.position.x, vertex.position.y, vertex.position.z]);
        vertex_attributes.normals.push(normalize_vec3([
            vertex.normal.x,
            vertex.normal.y,
            vertex.normal.z,
        ]));
        vertex_attributes
            .tex_coords_0
            .push([vertex.tex_coords.x, vertex.tex_coords.y]);
    }

    let mut ret = Vec::new();
    for batch_index in 0..skin.batches().len() {
        ret.push(create_mesh_for_submesh(
            model,
            &skin,
            batch_index,
            &vertex_attributes,
            image_handles,
            materials,
            meshes,
        )?);
    }
    Ok(ret)
}

fn create_mesh_for_submesh(
    model: &m2::M2Model,
    skin: &m2::skin::SkinFile,
    batch_index: usize,
    vertex_attributes: &VertexAttributes,
    image_handles: &[Handle<Image>],
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<ModelBundle> {
    let batch = &skin.batches()[batch_index];
    let submesh = &skin.submeshes()[batch.skin_section_index as usize];
    let texture_index =
        model.raw_data.texture_lookup_table[batch.texture_combo_index as usize] as usize;
    let texture = &image_handles[texture_index];

    // Determine alpha mode from material blend mode.
    // Note that multiple batches can share the same material.
    let model_material = &model.materials[batch.material_index as usize];
    let alpha_mode = material::alpha_mode_from_model_blend_mode(model_material.blend_mode);

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
        vertex_attributes.positions.clone(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vertex_attributes.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vertex_attributes.tex_coords_0.clone())
    .with_inserted_indices(Indices::U16(submesh_indices));

    let mesh_handle = meshes.add(mesh);

    Ok(ModelBundle {
        mesh: Mesh3d(mesh_handle),
        material: MeshMaterial3d(material_handle),
        transform: Transform::default(),
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn load_main_menu() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.default_model.file_path)?;
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();

        data::create_mesh_from_file_path(
            &settings.default_model.file_path,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn load_city() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.city_model.file_path)?;
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_file_path(
            &settings.city_model.file_path,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn load_dwarf() -> Result {
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.test_model.file_path)?;
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_file_path(
            &settings.test_model.file_path,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
