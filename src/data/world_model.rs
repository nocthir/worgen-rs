// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

use bevy::{asset::RenderAssetUsages, prelude::*, render::mesh::*};

use wow_mpq as mpq;
use wow_wmo as wmo;

use crate::{data::normalize_vec3, material::CustomMaterial};

#[derive(Clone)]
pub struct WmoInfo {
    pub path: PathBuf,
    pub groups: Vec<WmoGroupInfo>,
    pub material_count: usize,
    pub texture_count: usize,
}

#[derive(Clone)]
pub struct WmoGroupInfo {
    pub name: String,
    pub vertex_count: usize,
    pub index_count: usize,
}

pub fn read_mwos(archive: &mut mpq::Archive) -> Result<Vec<WmoInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        let wmo_path = PathBuf::from(&entry.name);

        if is_wmo_root_path(&wmo_path)
            && let Ok(model) = read_wmo(&entry.name, archive)
        {
            let groups = match read_wmo_groups(archive, &entry.name, &model) {
                Ok(groups) => groups,
                Err(err) => {
                    error!("Failed to read WMO groups for {}: {}", entry.name, err);
                    Vec::new()
                }
            };
            let material_count = model.materials.len();
            let texture_count = model.textures.len();
            let info = WmoInfo {
                path: PathBuf::from(&entry.name),
                groups,
                material_count,
                texture_count,
            };
            infos.push(info);
        }
    }

    Ok(infos)
}

fn read_wmo(path: &str, archive: &mut mpq::Archive) -> Result<wmo::WmoRoot> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(wmo::parse_wmo(&mut reader)?)
}

fn is_wmo_root_path<P: AsRef<Path>>(path: P) -> bool {
    if path.as_ref().extension() != Some(OsStr::new("wmo")) {
        return false;
    }
    !is_wmo_group_path(path)
}

fn is_wmo_group_path<P: AsRef<Path>>(path: P) -> bool {
    if path.as_ref().extension() != Some(OsStr::new("wmo")) {
        return false;
    }
    let file_stem = path
        .as_ref()
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    file_stem
        .split('_')
        .next_back()
        .is_some_and(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()))
}

fn read_wmo_groups<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    wmo_path: P,
    wmo: &wmo::WmoRoot,
) -> Result<Vec<WmoGroupInfo>> {
    let mut infos = Vec::new();
    for (group_index, wmo_group_info) in wmo.groups.iter().enumerate() {
        let wmo_group = read_wmo_group(archive, wmo_path.as_ref(), group_index)?;
        let name = wmo_group_info.name.clone();
        let vertex_count = wmo_group.vertices.len();
        let index_count = wmo_group.indices.len();
        let info = WmoGroupInfo {
            name,
            vertex_count,
            index_count,
        };
        infos.push(info);
    }
    Ok(infos)
}

pub fn create_meshes_from_wmo_path<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    path: P,
    custom_materials: &mut Assets<CustomMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<CustomMaterial>)>> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format!("Invalid world model path: {}", path.as_ref().display()))?;

    let file = archive.read_file(path_str)?;
    let mut reader = io::Cursor::new(&file);

    let mut ret = Vec::default();

    if let Ok(wmo) = wmo::parse_wmo(&mut reader)
        && !wmo.groups.is_empty()
    {
        let materials = create_materials_from_wmo(&wmo);
        let material_handles = materials
            .into_iter()
            .map(|mat| custom_materials.add(mat))
            .collect::<Vec<_>>();

        for group_index in 0..wmo.groups.len() {
            if let Ok(bundle) = create_mesh_from_wmo_group_path(
                archive,
                path_str,
                group_index,
                &material_handles,
                meshes,
            ) {
                ret.push(bundle);
            } else {
                error!("Failed to create mesh for group {group_index}");
            }
        }
    }

    Ok(ret)
}

fn create_materials_from_wmo(wmo: &wmo::WmoRoot) -> Vec<CustomMaterial> {
    let mut materials = Vec::new();
    for material in &wmo.materials {
        let color = material.diffuse_color;
        let material = CustomMaterial {
            color: linear_rgba_from_model_color(color),
            ..Default::default()
        };
        materials.push(material);
    }
    materials
}

fn linear_rgba_from_model_color(color: wmo::Color) -> LinearRgba {
    LinearRgba {
        red: color.r as f32 / 255.0,
        green: color.g as f32 / 255.0,
        blue: color.b as f32 / 255.0,
        alpha: color.a as f32 / 255.0,
    }
}

fn create_mesh_from_wmo_group_path<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    wmo_path: P,
    group_index: usize,
    material_handles: &[Handle<CustomMaterial>],
    meshes: &mut Assets<Mesh>,
) -> Result<(Handle<Mesh>, Handle<CustomMaterial>)> {
    let wmo_group = read_wmo_group(archive, wmo_path, group_index)?;
    create_mesh_from_wmo_group(&wmo_group, material_handles, meshes)
}

fn read_wmo_group<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    wmo_path: P,
    group_index: usize,
) -> Result<wmo::WmoGroup> {
    let group_filename = get_wmo_group_filename(&wmo_path, group_index);
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
    material_handles: &[Handle<CustomMaterial>],
    meshes: &mut Assets<Mesh>,
) -> Result<(Handle<Mesh>, Handle<CustomMaterial>)> {
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
        positions,
    )
    .with_inserted_indices(Indices::U16(wmo.indices.clone()));

    if !normals.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    if !tex_coords_0.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords_0);
    }
    if !colors.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    let mesh_handle = meshes.add(mesh);

    // Wrong?
    let material_handle = material_handles[wmo.materials[0] as usize].clone();
    Ok((mesh_handle, material_handle))
}
