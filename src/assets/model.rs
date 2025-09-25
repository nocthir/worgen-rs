// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

//! Experimental asset-based loading pipeline.
//!
//! This module contains early scaffolding for migrating manual IO task based model
//! loading into Bevy's `AssetServer` pipeline. It is intentionally not wired into
//! the existing `DataPlugin` yet; integration will be incremental.
//!
//! Steps planned (not yet all implemented):
//! 1. Define `ModelAsset` as a lightweight, parse-only representation referencing
//!    texture handles (no mesh/material creation yet).
//! 2. Implement `AssetLoader` that parses M2 model bytes and enqueues dependent
//!    texture loads using `LoadContext::load`.
//! 3. Add a post-processing system to create meshes/materials once all textures
//!    are ready (future step).
//! 4. Replace manual model load task path with handle-based selection (future).

use std::io;

use anyhow::Result;
use bevy::asset::RenderAssetUsages;
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use bevy::render::mesh::*;
use thiserror::Error;
use wow_m2 as m2;

use crate::assets::SelectedModelHandle;
use crate::data::bundle;
use crate::settings::Settings;
// Reuse helper for normal vector normalization
use crate::{
    camera::FocusCamera,
    data::bundle::{BoundingSphere, normalize_vec3},
};

/// First pass model asset: parsed data + texture dependency handles.
/// Mesh / material baking will happen in a later preparation system.
#[derive(Asset, TypePath, Debug)]
pub struct ModelAsset {
    /// Raw parsed model (CPU-side). Retained to allow later mesh generation.
    pub model: m2::M2Model,
    /// Original file bytes (kept for potential reprocessing; may be dropped later).
    pub data: Vec<u8>,
    /// Image handles requested during load (populated by the loader).
    pub image_handles: Vec<Handle<Image>>,
    /// Generated mesh and material handles after preparation.
    pub bundles: Vec<ModelBundle>,
    /// Bounding sphere computed after preparation (model space with reorientation applied).
    pub bounding_sphere: Option<BoundingSphere>,
    /// Whether meshes/materials have been prepared.
    pub prepared: bool,
    /// Whether this asset has been spawned (experimental path) to prevent duplicate spawns.
    pub spawned: bool,
}

impl ModelAsset {
    fn new(data: Vec<u8>, load_context: &mut LoadContext<'_>) -> Result<Self> {
        let mut cursor = io::Cursor::new(&data);
        let model = m2::M2Model::parse(&mut cursor)?;
        let image_handles = Self::new_image_handles(&model, load_context);

        Ok(ModelAsset {
            model,
            data,
            image_handles,
            bundles: Vec::new(),
            bounding_sphere: None,
            prepared: false,
            spawned: false,
        })
    }

    fn new_image_handles(
        model: &m2::M2Model,
        load_context: &mut LoadContext<'_>,
    ) -> Vec<Handle<Image>> {
        let mut handles = Vec::new();
        for texture in &model.textures {
            let texture_path = Self::get_image_asset_path(texture);
            let image_handle = load_context.load(texture_path);
            handles.push(image_handle);
        }
        handles
    }

    fn get_image_asset_path(texture: &m2::chunks::texture::M2Texture) -> String {
        if texture.texture_type != m2::chunks::M2TextureType::Hardcoded {
            // Ignore non-hardcoded textures for now.
            return format!("archive://{}", Settings::get().test_image_path.clone());
        }
        let filename = texture.filename.string.to_string_lossy();
        if filename.is_empty() {
            return format!("archive://{}", Settings::get().test_image_path.clone());
        }
        format!("archive://{}", filename)
    }
}

#[derive(Default)]
pub struct ModelAssetLoader;

#[derive(Debug, Error)]
pub enum ModelAssetLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] m2::M2Error),
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl AssetLoader for ModelAssetLoader {
    type Asset = ModelAsset;
    type Settings = (); // No custom settings yet
    type Error = ModelAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ModelAsset::new(bytes, load_context)?)
    }

    fn extensions(&self) -> &[&str] {
        // M2 primary extension; MDX/MDL are historical / variant forms that may alias.
        &["m2", "mdx", "mdl"]
    }
}

/// Marker component for entities spawned from a `ModelAsset` via the experimental path.
#[derive(Component)]
pub struct ExperimentalModelInstance;

impl ModelBundle {
    fn new_from_model(
        asset: &ModelAsset,
        materials: &mut Assets<StandardMaterial>,
        meshes: &mut Assets<Mesh>,
    ) -> Result<Vec<ModelBundle>> {
        let model = &asset.model;
        let model_data = &asset.data;
        let skin = model.parse_embedded_skin(model_data, 0)?;

        let vertex_count = model.vertices.len();
        let mut vertex_attributes = VertexAttributes::with_capacity(vertex_count);
        for vertex in model.vertices.iter() {
            vertex_attributes.positions.push([
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
            ]);
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
            ret.push(Self::new_from_model_submesh(
                asset,
                &skin,
                batch_index,
                &vertex_attributes,
                materials,
                meshes,
            )?);
        }
        Ok(ret)
    }

    fn new_from_model_submesh(
        asset: &ModelAsset,
        skin: &m2::skin::SkinFile,
        batch_index: usize,
        vertex_attributes: &VertexAttributes,
        materials: &mut Assets<StandardMaterial>,
        meshes: &mut Assets<Mesh>,
    ) -> Result<ModelBundle> {
        let batch = &skin.batches()[batch_index];
        let submesh = &skin.submeshes()[batch.skin_section_index as usize];
        let texture_index =
            asset.model.raw_data.texture_lookup_table[batch.texture_combo_index as usize] as usize;
        // Textures must be already loaded.
        let image = &asset.image_handles[texture_index];

        // Determine alpha mode from material blend mode.
        // Note that multiple batches can share the same material.
        let model_material = &asset.model.materials[batch.material_index as usize];
        let alpha_mode = bundle::alpha_mode_from_model_blend_mode(model_material.blend_mode);

        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(image.clone()),
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

    /// Compute a combined world-space bounding sphere (center, radius) for the given bundles.
    /// Uses mesh positions and applies the same reorientation as `add_bundle` to match spawned transforms.
    pub fn get_bounding_sphere(
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

#[derive(Bundle, Clone, Debug)]
pub struct ModelBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

pub trait CustomBundle: Bundle {
    fn get_transform(&self) -> &Transform;
    fn get_transform_mut(&mut self) -> &mut Transform;
    fn get_mesh(&self) -> &Mesh3d;
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

/// System: prepare meshes/materials for any loaded, unprepared `ModelAsset`s.
pub fn prepare_model_assets(
    mut assets: ResMut<Assets<ModelAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> bevy::ecs::error::Result {
    for model_asset in assets.iter_mut() {
        let (_id, asset) = model_asset;
        if asset.prepared {
            continue;
        }

        asset.bundles = ModelBundle::new_from_model(asset, &mut materials, &mut meshes)?;
        asset.bounding_sphere = ModelBundle::get_bounding_sphere(&asset.bundles, &meshes);

        asset.prepared = true;

        info!(
            "Prepared ModelAsset: bundles={}, images={}",
            asset.bundles.len(),
            asset.image_handles.len(),
        );
    }

    Ok(())
}

/// System: if a selected model handle exists and is prepared & not yet spawned, spawn its submeshes.
pub fn spawn_selected_model(
    selected: Res<SelectedModelHandle>,
    entity_query: Query<Entity, With<ExperimentalModelInstance>>,
    mut assets: ResMut<Assets<ModelAsset>>,
    mut commands: Commands,
    mut focus_writer: EventWriter<FocusCamera>,
) {
    let Some(handle) = selected.0.as_ref() else {
        return;
    };
    let Some(asset) = assets.get_mut(handle) else {
        return;
    };
    if !asset.prepared || asset.spawned {
        return;
    }
    // Simple spawn: each submesh as separate entity under a parent for grouping.
    // Apply same reorientation as legacy path (-90° X then -90° Z) on parent.
    let mut root_transform = Transform::default();
    root_transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
    root_transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);

    let parent = commands
        .spawn((
            ExperimentalModelInstance,
            root_transform,
            Visibility::Visible,
            Name::new("ModelAssetRoot"),
        ))
        .id();

    let children: Vec<Entity> = asset
        .bundles
        .iter()
        .map(|bundle| commands.spawn(bundle.clone()).id())
        .collect();
    commands.entity(parent).replace_children(&children);
    asset.spawned = true;

    // Emit focus event using bounding sphere (already computed with orientation baked in).
    if let Some(bs) = asset.bounding_sphere {
        focus_writer.write(FocusCamera {
            bounding_sphere: bs,
        });
    }
}
