// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::*, pbr::ExtendedMaterial, prelude::*, render::mesh::*};

use crate::{data::*, material::TerrainMaterial};

mod material_bundle;
mod model_bundle;
mod texture_bundle;
mod world_map_bundle;
mod world_model_bundle;

pub use material_bundle::*;
pub use model_bundle::*;
pub use texture_bundle::*;
pub use world_map_bundle::*;
pub use world_model_bundle::*;

pub trait CustomBundle: Bundle {
    fn get_transform(&self) -> &Transform;
    fn get_transform_mut(&mut self) -> &mut Transform;
    fn get_mesh(&self) -> &Mesh3d;
}

#[derive(Bundle, Clone)]
pub struct ModelBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

impl CustomBundle for ModelBundle {
    fn get_transform(&self) -> &Transform {
        &self.transform
    }
    fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
    fn get_mesh(&self) -> &Mesh3d {
        &self.mesh
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct TerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    pub transform: Transform,
}

impl CustomBundle for TerrainBundle {
    fn get_transform(&self) -> &Transform {
        &self.transform
    }
    fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
    fn get_mesh(&self) -> &Mesh3d {
        &self.mesh
    }
}

pub fn add_bundle<S: Into<String>>(
    commands: &mut Commands,
    mut bundle: impl CustomBundle,
    path: S,
) {
    bundle
        .get_transform_mut()
        .rotate_local_x(-std::f32::consts::FRAC_PI_2);
    bundle
        .get_transform_mut()
        .rotate_local_z(-std::f32::consts::FRAC_PI_2);
    commands.spawn((CurrentFile { path: path.into() }, bundle));
}

// Actually used in tests
#[allow(unused)]
pub fn create_mesh_from_file_path(
    file_path: &str,
    file_info_map: &FileInfoMap,
    scene_assets: &mut file::SceneAssets,
) -> Result<(Vec<TerrainBundle>, Vec<ModelBundle>)> {
    let file_info = file_info_map.get_file_info(file_path)?;
    create_mesh_from_file_info(file_info, file_info_map, scene_assets)
}

// Actually used in tests
#[allow(unused)]
pub fn create_mesh_from_file_info(
    file_info: &FileInfo,
    file_info_map: &FileInfoMap,
    scene_assets: &mut file::SceneAssets,
) -> Result<(Vec<TerrainBundle>, Vec<ModelBundle>)> {
    let mut terrain_bundles = Vec::new();
    let mut model_bundles = Vec::new();

    match &file_info.data_info {
        Some(DataInfo::Model(model_info)) => {
            let models = create_meshes_from_model_info(model_info, file_info_map, scene_assets)?;
            model_bundles.extend(models);
        }
        Some(DataInfo::WorldModel(world_model_info)) => {
            let models =
                create_meshes_from_world_model_info(world_model_info, file_info_map, scene_assets)?;
            model_bundles.extend(models);
        }
        Some(DataInfo::WorldMap(world_map_info)) => {
            let (terrains, models) =
                create_meshes_from_world_map_info(world_map_info, file_info_map, scene_assets)?;
            terrain_bundles.extend(terrains);
            model_bundles.extend(models);
        }
        _ => {
            return Err(format!(
                "Unsupported or missing data info for file: {}",
                file_info.path
            )
            .into());
        }
    };

    Ok((terrain_bundles, model_bundles))
}

pub fn from_normalized_vec3_u8(v: [u8; 3]) -> [f32; 3] {
    let x = u8::cast_signed(v[0]) as f32 / 127.0;
    let y = u8::cast_signed(v[1]) as f32 / 127.0;
    let z = u8::cast_signed(v[2]) as f32 / 127.0;
    normalize_vec3([x, y, z])
}

pub fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}
