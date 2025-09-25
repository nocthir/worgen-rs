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
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::asset::{AssetPath, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::mesh::*;
use thiserror::Error;
use wow_m2 as m2;

use crate::assets::ImageLoader;
use crate::data::bundle;
use crate::settings::Settings;
// Reuse helper for normal vector normalization
use crate::data::bundle::{BoundingSphere, normalize_vec3};

pub struct ModelAssetPlugin;

impl Plugin for ModelAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ModelAsset>()
            .init_asset_loader::<ImageLoader>()
            .init_asset_loader::<ModelAssetLoader>();
    }
}

/// Labels that can be used to load part of a Model
///
/// You can use [`ModelAssetLabel::from_asset`] to add it to an asset path
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_scene::prelude::*;
/// # use bevy_gltf::prelude::*;
///
/// fn load_model(asset_server: Res<AssetServer>) {
///     let model: Handle<Scene> = asset_server.load(ModelAssetLabel::Mesh(0).from_asset("model/path/extension"));
/// }
/// ```
///
/// Or when formatting a string for the path
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_scene::prelude::*;
///
/// fn load_model(asset_server: Res<AssetServer>) {
///     let gltf_scene: Handle<Scene> = asset_server.load(format!("model/path.extension#{}", ModelAssetLabel::Mesh(0)));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelAssetLabel {
    Model,
    Mesh(usize),
    Material(usize),
}

impl core::fmt::Display for ModelAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ModelAssetLabel::Model => f.write_str("Model"),
            ModelAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            ModelAssetLabel::Material(index) => f.write_str(&format!("Material{index}")),
        }
    }
}

impl ModelAssetLabel {
    /// Add this label to an asset path
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::prelude::*;
    /// # use bevy_scene::prelude::*;
    ///
    /// fn load_model(asset_server: Res<AssetServer>) {
    ///     let model: Handle<Scene> = asset_server.load(ModelAssetLabel::Mesh(0).from_asset("model/path.extension"));
    /// }
    /// ```
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct ModelAsset {
    /// Scene loaded from the model, with reorientation applied.
    pub scene: Handle<Scene>,
    /// Image handles requested during load (populated by the loader).
    pub image_handles: Vec<Handle<Image>>,
    /// Generated mesh handles after preparation.
    pub meshes: Vec<Handle<Mesh>>,
    /// Generated material handles after preparation.
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Bounding sphere computed after preparation (model space with reorientation applied).
    pub bounding_sphere: Option<BoundingSphere>,
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

impl ModelAssetLoader {
    fn load_model(
        bytes: Vec<u8>,
        load_context: &mut LoadContext<'_>,
    ) -> Result<ModelAsset, ModelAssetLoaderError> {
        let mut cursor = io::Cursor::new(&bytes);
        let model = m2::M2Model::parse(&mut cursor)?;

        let images = Self::load_images(&model, load_context);
        let mut materials = Vec::new();
        let mut meshes = Vec::new();
        Self::load_meshes(&model, &bytes, &images, &mut meshes, &mut materials)?;
        let bounding_sphere = Self::get_bounding_sphere(&meshes);

        let meshes: Vec<Handle<Mesh>> = meshes
            .into_iter()
            .enumerate()
            .map(|(i, m)| load_context.add_labeled_asset(ModelAssetLabel::Mesh(i).to_string(), m))
            .collect();
        let materials: Vec<Handle<StandardMaterial>> = materials
            .into_iter()
            .enumerate()
            .map(|(i, mat)| {
                load_context.add_labeled_asset(ModelAssetLabel::Material(i).to_string(), mat)
            })
            .collect();

        let mut transform = Transform::default();
        transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);
        let mut world = World::default();
        world
            .spawn((transform, Visibility::default()))
            .with_children(|parent| {
                for i in 0..meshes.len() {
                    parent.spawn((
                        Mesh3d(meshes[i].clone()),
                        MeshMaterial3d(materials[i].clone()),
                    ));
                }
            });
        let scene_loader = load_context.begin_labeled_asset();
        let loaded_scene = scene_loader.finish(Scene::new(world));
        let scene =
            load_context.add_loaded_labeled_asset(ModelAssetLabel::Model.to_string(), loaded_scene);

        Ok(ModelAsset {
            scene,
            image_handles: images,
            meshes,
            materials,
            bounding_sphere,
        })
    }

    fn load_images(model: &m2::M2Model, load_context: &mut LoadContext<'_>) -> Vec<Handle<Image>> {
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

    fn load_meshes(
        model: &m2::M2Model,
        model_bytes: &[u8],
        images: &[Handle<Image>],
        meshes: &mut Vec<Mesh>,
        materials: &mut Vec<StandardMaterial>,
    ) -> Result<(), ModelAssetLoaderError> {
        let skin = model.parse_embedded_skin(model_bytes, 0)?;

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

        for batch_index in 0..skin.batches().len() {
            Self::load_submesh(
                model,
                &skin,
                batch_index,
                &vertex_attributes,
                images,
                meshes,
                materials,
            )?;
        }
        Ok(())
    }

    fn load_submesh(
        model: &m2::M2Model,
        skin: &m2::skin::SkinFile,
        batch_index: usize,
        vertex_attributes: &VertexAttributes,
        images: &[Handle<Image>],
        meshes: &mut Vec<Mesh>,
        materials: &mut Vec<StandardMaterial>,
    ) -> Result<()> {
        let batch = &skin.batches()[batch_index];
        let submesh = &skin.submeshes()[batch.skin_section_index as usize];
        let texture_index =
            model.raw_data.texture_lookup_table[batch.texture_combo_index as usize] as usize;
        // Textures must be already loaded.
        let image = &images[texture_index];

        // Determine alpha mode from material blend mode.
        // Note that multiple batches can share the same material.
        let model_material = &model.materials[batch.material_index as usize];
        let alpha_mode = bundle::alpha_mode_from_model_blend_mode(model_material.blend_mode);

        let material = StandardMaterial {
            base_color_texture: Some(image.clone()),
            perceptual_roughness: 1.0,
            alpha_mode,
            ..Default::default()
        };

        materials.push(material);

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

        meshes.push(mesh);

        Ok(())
    }

    /// Compute a combined world-space bounding sphere (center, radius) for the given bundles.
    /// Uses mesh positions and applies the same reorientation as `add_bundle` to match spawned transforms.
    pub fn get_bounding_sphere(meshes: &[Mesh]) -> Option<BoundingSphere> {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for mesh in meshes {
            // Prepare final transform: user-provided transform + reorientation applied at spawn
            let mut final_transform = Transform::default();
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
        Self::load_model(bytes, load_context)
    }

    fn extensions(&self) -> &[&str] {
        // M2 primary extension; MDX/MDL are historical / variant forms that may alias.
        &["m2", "mdx", "mdl"]
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
