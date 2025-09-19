// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{mesh::*, render_resource::Face},
    tasks,
};

use wow_mpq as mpq;
use wow_wmo as wmo;

use crate::data::{ModelBundle, file, normalize_vec3, texture};

pub struct WorldModelInfo {
    pub world_model: wmo::WmoRoot,
    pub groups: Vec<wmo::WmoGroup>,
}

impl WorldModelInfo {
    pub fn get_texture_paths(&self) -> &[String] {
        &self.world_model.textures
    }
}

fn read_wmo(path: &str, archive: &mut mpq::Archive) -> Result<wmo::WmoRoot> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(wmo::parse_wmo(&mut reader)?)
}

pub fn is_world_model_root_path(file_path: &str) -> bool {
    if !is_world_model_extension(file_path) {
        return false;
    }
    !is_world_model_group_path(file_path)
}

fn is_world_model_group_path(file_path: &str) -> bool {
    if !is_world_model_extension(file_path) {
        return false;
    }
    let file_path = Path::new(file_path);
    let file_stem = file_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    file_stem
        .split('_')
        .next_back()
        .is_some_and(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()))
}

fn is_world_model_extension(file_path: &str) -> bool {
    let lower = file_path.to_lowercase();
    lower.ends_with(".wmo")
}

fn read_groups(
    file_path: &str,
    archive: &mut mpq::Archive,
    wmo: &wmo::WmoRoot,
) -> Result<Vec<wmo::WmoGroup>> {
    let mut groups = Vec::new();
    for (group_index, _) in wmo.groups.iter().enumerate() {
        let wmo_group = read_group(file_path, archive, group_index)?;
        groups.push(wmo_group);
    }
    Ok(groups)
}

pub fn loading_world_model_task(file_info: &file::FileInfo) -> tasks::Task<Result<file::FileInfo>> {
    info!("Starting to load world model: {}", file_info.path);
    tasks::IoTaskPool::get().spawn(load_world_model(file_info.shallow_clone()))
}

async fn load_world_model(mut file_info: file::FileInfo) -> Result<file::FileInfo> {
    match load_world_model_impl(&file_info) {
        Ok(world_model_info) => {
            file_info.set_world_model(world_model_info);
            info!("Loaded world model: {}", file_info.path);
            Ok(file_info)
        }
        Err(e) => {
            error!("Failed to load world model {}: {}", file_info.path, e);
            file_info.state = file::FileInfoState::Error(e.to_string());
            Ok(file_info)
        }
    }
}

fn load_world_model_impl(file_info: &file::FileInfo) -> Result<WorldModelInfo> {
    let mut archive = mpq::Archive::open(&file_info.archive_path)?;
    let world_model = read_wmo(&file_info.path, &mut archive)?;
    let groups = read_groups(&file_info.path, &mut archive, &world_model)?;
    Ok(WorldModelInfo {
        world_model,
        groups,
    })
}

pub fn create_meshes_from_world_model_path(
    world_model_path: &str,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
    standard_materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let world_model_info = file_info_map.get_world_model_info(world_model_path)?;
    create_meshes_from_world_model_info(
        &world_model_info,
        file_info_map,
        images,
        standard_materials,
        meshes,
    )
}

pub fn create_meshes_from_world_model_info(
    world_model_info: &WorldModelInfo,
    file_info_map: &file::FileInfoMap,
    images: &mut Assets<Image>,
    standard_materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let mut ret = Vec::default();

    let wmo = &world_model_info.world_model;
    let textures = texture::create_textures_from_wmo(wmo, file_info_map, images)?;
    let materials = create_materials_from_wmo(wmo, &textures);
    let material_handles = materials
        .into_iter()
        .map(|mat| standard_materials.add(mat))
        .collect::<Vec<_>>();

    let default_material_handle = standard_materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        unlit: true,
        ..Default::default()
    });

    for group_index in 0..world_model_info.groups.len() {
        let bundles = create_mesh_from_wmo_group(
            &world_model_info.groups[group_index],
            default_material_handle.clone(),
            &material_handles,
            meshes,
        );
        ret.extend(bundles);
    }

    Ok(ret)
}

fn create_materials_from_wmo(
    wmo: &wmo::WmoRoot,
    images: &[Handle<Image>],
) -> Vec<StandardMaterial> {
    let mut materials = Vec::new();

    for material in &wmo.materials {
        let base_color = create_color_from_wmo(material.diffuse_color);
        let emissive = create_color_from_wmo(material.emissive_color).to_linear();

        let texture_index = material.get_texture1_index(&wmo.texture_offset_index_map);
        let image = images[texture_index as usize].clone();
        let unlit = material.flags.contains(wmo::WmoMaterialFlags::UNLIT);
        let cull_mode = if material.flags.contains(wmo::WmoMaterialFlags::TWO_SIDED) {
            None
        } else {
            Some(Face::Back)
        };

        let material = StandardMaterial {
            base_color,
            emissive,
            perceptual_roughness: 1.0,
            base_color_texture: Some(image),
            unlit,
            cull_mode,
            ..Default::default()
        };
        materials.push(material);
    }
    materials
}

fn create_color_from_wmo(color: wmo::types::Color) -> Color {
    Color::linear_rgba(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    )
}

fn create_mesh_from_group_path(
    file_path: &str,
    archive: &mut mpq::Archive,
    group_index: usize,
    default_material_handle: Handle<StandardMaterial>,
    material_handles: &[Handle<StandardMaterial>],
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let wmo_group = read_group(file_path, archive, group_index)?;
    Ok(create_mesh_from_wmo_group(
        &wmo_group,
        default_material_handle,
        material_handles,
        meshes,
    ))
}

fn read_group(
    file_path: &str,
    archive: &mut mpq::Archive,
    group_index: usize,
) -> Result<wmo::WmoGroup> {
    let group_filename = get_wmo_group_filename(file_path, group_index);
    let file = archive
        .read_file(&group_filename)
        .map_err(|e| format!("Failed to read WMO group file {}: {}", group_filename, e))?;
    let mut reader = io::Cursor::new(&file);
    let wmo_group = wmo::parse_wmo_group(&mut reader, group_index as _)
        .map_err(|e| format!("Failed to parse WMO group file {}: {}", group_filename, e))?;
    Ok(wmo_group)
}

fn get_wmo_group_filename<P: AsRef<Path>>(wmo_path: P, group_index: usize) -> String {
    let base_path = wmo_path.as_ref().with_extension("");
    format!("{}_{:03}.wmo", base_path.display(), group_index)
}

fn create_mesh_from_wmo_group(
    wmo: &wmo::WmoGroup,
    default_material_handle: Handle<StandardMaterial>,
    material_handles: &[Handle<StandardMaterial>],
    meshes: &mut Assets<Mesh>,
) -> Vec<ModelBundle> {
    let positions: Vec<_> = wmo.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();
    let normals: Vec<_> = wmo
        .normals
        .iter()
        .map(|v| normalize_vec3([v.x, v.y, v.z]))
        .collect();
    let tex_coords_0: Vec<_> = wmo.tex_coords.iter().map(|v| [v.u, v.v]).collect();
    let mut colors = Vec::new();
    if let Some(vertex_colors) = &wmo.vertex_colors {
        colors = vertex_colors
            .iter()
            .map(|v| [v.r as f32, v.g as f32, v.b as f32, v.a as f32])
            .collect();
    }

    let mut ret = Vec::new();

    for batch in &wmo.batches {
        let indices = wmo
            .indices
            .iter()
            .copied()
            .skip(batch.start_index as usize)
            .take(batch.count as usize)
            .collect();

        // Keep the mesh data accessible in future frames to be able to mutate it in toggle_texture.
        let mut mesh = Mesh::new(
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
        .with_inserted_indices(Indices::U16(indices));

        if !normals.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
        } else {
            mesh.compute_normals();
        }
        if !tex_coords_0.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords_0.clone());
        }
        if !colors.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors.clone());
        }

        let mesh_handle = meshes.add(mesh);

        let material_index = batch.material_id as usize;
        let material_handle = if material_index < material_handles.len() {
            material_handles[material_index].clone()
        } else {
            default_material_handle.clone()
        };

        ret.push(ModelBundle {
            mesh: Mesh3d(mesh_handle),
            material: MeshMaterial3d(material_handle),
            transform: Transform::default(),
        });
    }

    ret
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{data::texture, *};

    #[test]
    fn altar() -> Result {
        env_logger::init();
        let settings = settings::load_settings()?;
        let selected_model = ui::FileSelected::from(&settings.test_world_model);
        let file_info_map = texture::test::default_file_info_map(&settings)?;
        let mut images = Assets::<Image>::default();
        let mut custom_materials = Assets::<StandardMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        data::create_mesh_from_selected_file(
            &selected_model,
            &file_info_map,
            &mut images,
            &mut custom_materials,
            &mut meshes,
        )?;
        Ok(())
    }
}
