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

#[derive(Bundle, Clone)]
pub struct ModelBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

#[derive(Bundle, Clone)]
pub struct TerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    pub transform: Transform,
}

pub trait CustomBundle: Bundle {
    fn get_transform(&self) -> &Transform;
    fn get_transform_mut(&mut self) -> &mut Transform;
    fn get_mesh(&self) -> &Mesh3d;
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

#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// Compute a combined world-space bounding sphere (center, radius) for the given bundles.
/// Uses mesh positions and applies the same reorientation as `add_bundle` to match spawned transforms.
pub fn compute_bounding_sphere_from_bundles(
    bundles: &[impl CustomBundle],
    meshes: &Assets<Mesh>,
) -> Option<BoundingSphere> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    for b in bundles {
        let Some(mesh) = meshes.get(&b.get_mesh().0) else {
            continue;
        };

        // Prepare final transform: user-provided transform + reorientation applied at spawn
        let mut final_transform = *b.get_transform();
        final_transform.rotate_local_x(-f32::consts::FRAC_PI_2);
        final_transform.rotate_local_z(-f32::consts::FRAC_PI_2);

        // Extract positions
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            // Compute local AABB
            let mut local_min = Vec3::splat(f32::INFINITY);
            let mut local_max = Vec3::splat(f32::NEG_INFINITY);
            for p in positions {
                let v = Vec3::new(p[0], p[1], p[2]);
                local_min = local_min.min(v);
                local_max = local_max.max(v);
            }

            // Local center and radius (AABB-based sphere)
            let local_center = (local_min + local_max) * 0.5;
            let local_radius = (local_max - local_center).length();

            // Transform sphere to world: position by transform, scale by max scale component
            let world_center = final_transform.transform_point(local_center);
            let s = final_transform.scale.abs();
            let max_scale = s.x.max(s.y).max(s.z).max(1e-6);
            let world_radius = local_radius * max_scale;

            // Expand global AABB by the sphere
            min = min.min(world_center - Vec3::splat(world_radius));
            max = max.max(world_center + Vec3::splat(world_radius));
        }
    }

    if min.x.is_finite() && max.x.is_finite() {
        let center = (min + max) * 0.5;
        let radius = (max - center).length();
        Some(BoundingSphere { center, radius })
    } else {
        None
    }
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
