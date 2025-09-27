// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::fmt;

use bevy::{
    prelude::*,
    render::{mesh::MeshAabb, primitives::Aabb},
};

use crate::assets::TransformMesh;

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct RootAabb {
    pub aabb: Aabb,
}

impl RootAabb {
    pub fn new(mesh: &TransformMesh) -> Option<Self> {
        Self::from_mesh_with_transform(&mesh.mesh, &mesh.transform)
    }

    pub fn from_mesh_with_transform(mesh: &Mesh, transform: &Transform) -> Option<Self> {
        let aabb = mesh.compute_aabb()?;
        let mut ret = Self { aabb };
        ret.transform(transform);
        Some(ret)
    }

    pub fn from_meshes_with_transform<'m>(
        meshes: impl Iterator<Item = &'m Mesh>,
        transform: &Transform,
    ) -> Self {
        let iter = meshes.filter_map(|mesh| RootAabb::from_mesh_with_transform(mesh, transform));
        Self::from_aabbs(iter)
    }

    pub fn from_transformed_meshes<'m>(meshes: impl Iterator<Item = &'m TransformMesh>) -> Self {
        let iter = meshes.filter_map(|mesh| mesh.compute_aabb());
        Self::from_aabbs(iter)
    }

    pub fn from_aabbs(mut aabbs: impl Iterator<Item = RootAabb>) -> Self {
        if let Some(first) = aabbs.next() {
            let mut acc = first;
            for next in aabbs {
                acc.extend(&next);
            }
            acc
        } else {
            // No AABBs provided
            Self::default()
        }
    }

    pub fn transform(&mut self, transform: &Transform) {
        let matrix = transform.compute_matrix();
        let (scale, rotation, _translation) = matrix.to_scale_rotation_translation();

        // Original data
        let center = Vec3::from(self.aabb.center);
        let half = Vec3::from(self.aabb.half_extents);

        // Build linear part (rotation * scale)
        let rot_mat = Mat3::from_quat(rotation);
        let linear = rot_mat * Mat3::from_diagonal(scale);

        // New center
        let new_center = (matrix * center.extend(1.0)).truncate();

        // New half extents = abs(linear) * half (component-wise matrix-vector mult)
        let m = linear.to_cols_array_2d(); // [[m00,m01,m02],[m10,...],...]
        let abs_linear = Mat3::from_cols(
            Vec3::new(m[0][0].abs(), m[0][1].abs(), m[0][2].abs()),
            Vec3::new(m[1][0].abs(), m[1][1].abs(), m[1][2].abs()),
            Vec3::new(m[2][0].abs(), m[2][1].abs(), m[2][2].abs()),
        );
        let new_half = abs_linear * half;

        self.aabb.center = new_center.into();
        self.aabb.half_extents = new_half.into();
    }

    pub fn extend(&mut self, b: &RootAabb) {
        Self::extend_aabb(&mut self.aabb, &b.aabb);
    }

    fn extend_aabb(to_extend: &mut Aabb, other: &Aabb) {
        if to_extend.half_extents == Vec3::ZERO.into() {
            *to_extend = *other;
            return;
        }
        let min_a = to_extend.min();
        let max_a = to_extend.max();
        let min_b = other.min();
        let max_b = other.max();
        let min = min_a.min(min_b);
        let max = max_a.max(max_b);
        to_extend.center = (min + max) * 0.5;
        to_extend.half_extents = (max - min) * 0.5;
    }
}

impl fmt::Display for RootAabb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Center: ({:.2}, {:.2}, {:.2}), Half Extents: ({:.2}, {:.2}, {:.2})",
            self.aabb.center.x,
            self.aabb.center.y,
            self.aabb.center.z,
            self.aabb.half_extents.x,
            self.aabb.half_extents.y,
            self.aabb.half_extents.z
        )
    }
}
