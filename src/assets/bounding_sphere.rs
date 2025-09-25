// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{prelude::*, render::mesh::VertexAttributeValues};

use crate::data::bundle::CustomBundle;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    /// Compute a combined world-space bounding sphere (center, radius) for the given bundles.
    /// Uses mesh positions and applies the same reorientation as `add_bundle` to match spawned transforms.
    pub fn new<'m>(meshes: impl Iterator<Item = &'m Mesh>) -> Option<BoundingSphere> {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        // Prepare final transform: user-provided transform + reorientation applied at spawn
        let mut final_transform = Transform::default();
        final_transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        final_transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);

        for mesh in meshes {
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
        final_transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        final_transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);

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
