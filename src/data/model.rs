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
    tasks,
};
use wow_m2 as m2;
use wow_mpq as mpq;

use crate::data::{
    ModelBundle, add_bundle,
    archive::{self, FileInfo, FileInfoMap},
    normalize_vec3, texture,
};

#[derive(Clone)]
pub struct ModelInfo {
    pub model: m2::M2Model,
    pub data: Vec<u8>,
}

impl ModelInfo {
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

#[derive(Resource, Default)]
pub struct LoadFileTask {
    tasks: Vec<tasks::Task<Result<FileInfo>>>,
    completed: Vec<FileInfo>,
}

pub fn start_loading_model<P: AsRef<Path>>(
    tasks: &mut LoadFileTask,
    file_path: &str,
    archive_path: P,
) {
    info!("Starting to load model: {}", file_path);
    let task = tasks::IoTaskPool::get().spawn(load_model(
        file_path.to_string(),
        archive_path.as_ref().to_path_buf(),
    ));
    tasks.tasks.push(task);
}

async fn load_model(file_path: String, archive_path: PathBuf) -> Result<FileInfo> {
    let mut archive = mpq::Archive::open(&archive_path)
        .map_err(|e| format!("Failed to open archive {}: {}", archive_path.display(), e))?;
    let data = archive
        .read_file(&file_path)
        .map_err(|e| format!("Failed to read model file {}: {}", file_path, e))?;
    let mut reader = io::Cursor::new(&data);
    let model = m2::M2Model::parse(&mut reader)
        .map_err(|e| format!("Failed to parse model file {}: {}", file_path, e))?;
    let model_info = ModelInfo { model, data };
    Ok(FileInfo::new_model(file_path, archive.path(), model_info))
}

// TODO: This function is getting quite large. Consider breaking it down.
pub fn check_file_loading(
    mut load_task: ResMut<LoadFileTask>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) -> Result<()> {
    let mut tasks = Vec::new();
    tasks.append(&mut load_task.tasks);

    let mut new_tasks = Vec::new();

    for mut current_task in tasks {
        let poll_result = tasks::block_on(tasks::poll_once(&mut current_task));
        if let Some(result) = poll_result {
            match result {
                Err(err) => {
                    error!("Error loading file: {err}");
                }
                Ok(file) => {
                    let file_path = file.path.clone();
                    info!("Loaded file: {}", file_path);

                    match &file.data_info {
                        Some(archive::DataInfo::Model(model_info)) => {
                            // At this point we have the model loaded, but textures may not be loaded yet.
                            // We need to check the file info map for texture files and start loading them if necessary.
                            for texture_path in model_info.get_texture_paths() {
                                let texture_file_info =
                                    file_info_map.get_file_info(&texture_path)?;
                                if texture_file_info.is_unloaded() {
                                    // Start loading the texture
                                    let new_task = texture::loading_texture_task(texture_file_info);
                                    new_tasks.push(new_task);
                                }
                            }

                            // Put the current task back to be processed later
                            load_task.completed.push(file);
                        }
                        Some(archive::DataInfo::Texture(_)) => {
                            // Texture loaded, update the file info map
                            file_info_map.insert(file);
                        }
                        _ => {
                            error!("Loaded file type is not valid: {}", file.path);
                            continue;
                        }
                    }
                }
            }
        } else {
            // Not ready yet, put it back
            load_task.tasks.push(current_task);
        }
    }

    load_task.tasks.extend(new_tasks);

    let mut completed_tasks = Vec::new();
    completed_tasks.append(&mut load_task.completed);

    for file in completed_tasks {
        let mut all_textures_loaded = true;

        match &file.data_info {
            Some(archive::DataInfo::Model(model_info)) => {
                // At this point we have the model loaded, but textures may not be loaded yet.
                // We need to check the file info map to see whether the loading has completed.
                for texture_path in model_info.get_texture_paths() {
                    let texture_file_info = file_info_map.get_file_info(&texture_path)?;
                    if texture_file_info.is_unloaded() {
                        warn!("Still waiting for texture: {}", texture_path);
                        // Put this task back to be processed later
                        all_textures_loaded = false;
                        break;
                    }
                }

                if all_textures_loaded {
                    // Update the file archive map
                    let bundles = create_meshes_from_model_info(
                        model_info,
                        &file_info_map,
                        &mut images,
                        &mut materials,
                        &mut meshes,
                    )?;

                    if bundles.is_empty() {
                        error!("No meshes loaded for file: {}", file.path);
                        return Ok(());
                    }
                    for bundle in bundles {
                        add_bundle(&mut commands, bundle);
                    }

                    info!("Added meshes from {}", file.path);

                    // All textures are loaded, update the file info map
                    file_info_map.insert(file);
                } else {
                    // Put this task back to be processed later
                    load_task.completed.push(file);
                }
            }
            _ => {
                error!("Loaded file is not a model: {}", file.path);
                continue;
            }
        }
    }

    Ok(())
}

pub fn create_meshes_from_model_path(
    model_path: &str,
    file_info_map: &archive::FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let model_info = file_info_map.get_model_info(model_path)?;
    create_meshes_from_model_info(model_info, file_info_map, images, materials, meshes)
}

pub fn create_meshes_from_model_info(
    model_info: &ModelInfo,
    file_info_map: &archive::FileInfoMap,
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
    use crate::{
        data::{archive::ArchiveInfo, texture},
        *,
    };

    #[test]
    fn main_menu() -> Result {
        let settings = settings::load_settings()?;
        let file_info_map = texture::test::default_file_info_map(&settings)?;
        let selected_model = ui::FileSelected::from(&settings.default_model);
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_file(
            &selected_model,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }

    #[test]
    fn city() -> Result {
        let settings = settings::load_settings()?;
        let file_info_map = texture::test::default_file_info_map(&settings)?;
        let selected_model = ui::FileSelected::from(&settings.city_model);
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_file(
            &selected_model,
            &file_info_map,
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
        let mut file_info_map = texture::test::default_file_info_map(&settings)?;
        let mut archive_info = ArchiveInfo::new(&settings.model_archive_path)?;
        file_info_map.fill(&mut archive_info)?;
        let selected_model = ui::FileSelected::from(&settings.test_model);
        let mut images = Assets::<Image>::default();
        let mut standard_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_file(
            &selected_model,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
