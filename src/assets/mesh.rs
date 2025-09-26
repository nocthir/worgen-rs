// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;

use crate::assets::root_aabb::RootAabb;

#[derive(Debug)]
pub struct TransformMesh {
    pub mesh: Mesh,
    pub transform: Transform,
}

impl TransformMesh {
    pub fn compute_aabb(&self) -> Option<RootAabb> {
        RootAabb::new(self)
    }
}
