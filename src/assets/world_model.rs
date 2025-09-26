// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0
use std::io;
use std::path::Path;

use anyhow::{Result, anyhow};
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::asset::{AssetPath, ReadAssetBytesError, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::mesh::*;
use bevy::render::render_resource::Face;
use thiserror::Error;
use wow_wmo as wmo;

use crate::assets::{ImageLoader, material};
// Reuse helper for normal vector normalization
use crate::data::bundle::normalize_vec3;

/// Labels that can be used to load part of a Model
///
/// You can use [`WorldModelAssetLabel::from_asset`] to add it to an asset path
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_asset::prelude::*;
/// # use bevy_scene::prelude::*;
/// # use bevy_gltf::prelude::*;
///
/// fn load_model(asset_server: Res<AssetServer>) {
///     let mesh: Handle<Scene> = asset_server.load(WorldModelAssetLabel::Mesh(0).from_asset("model/path/extension"));
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
///     let mesh: Handle<Scene> = asset_server.load(format!("model/path.extension#{}", WorldModelAssetLabel::Mesh(0)));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldModelAssetLabel {
    Root,
    // group, batch
    Mesh(usize),
    Material(usize),
    Image(usize),
    BoundingSphere,
}

impl core::fmt::Display for WorldModelAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WorldModelAssetLabel::Root => f.write_str("Root"),
            WorldModelAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            WorldModelAssetLabel::Material(index) => f.write_str(&format!("Material{index}")),
            WorldModelAssetLabel::Image(index) => f.write_str(&format!("Image{index}")),
            WorldModelAssetLabel::BoundingSphere => f.write_str("BoundingSphere"),
        }
    }
}

impl WorldModelAssetLabel {
    /// Add this label to an asset path
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::prelude::*;
    /// # use bevy_scene::prelude::*;
    ///
    /// fn load_model(asset_server: Res<AssetServer>) {
    ///     let model: Handle<Scene> = asset_server.load(WorldModelAssetLabel::Model.from_asset("model/path.extension"));
    /// }
    /// ```
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct WorldModelAsset {
    /// Scene loaded from the model, with reorientation applied.
    pub scene: Handle<Scene>,
    /// Image handles requested during load (populated by the loader).
    pub image_handles: Vec<Handle<Image>>,
    /// Generated mesh handles after preparation.
    pub meshes: Vec<WorldModelMesh>,
    /// Generated material handles after preparation.
    pub materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Debug)]
pub struct WorldModelMesh {
    pub mesh: Mesh,
    pub material: Handle<StandardMaterial>,
}

#[derive(Default)]
pub struct WorldModelAssetLoader;

#[derive(Debug, Error)]
pub enum WorldModelAssetLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] wmo::WmoError),
    #[error("Read error: {0}")]
    Read(#[from] ReadAssetBytesError),
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl WorldModelAssetLoader {
    pub async fn load_path(
        model_path: &str,
        load_context: &mut LoadContext<'_>,
    ) -> Result<WorldModelAsset, WorldModelAssetLoaderError> {
        let bytes = load_context.read_asset_bytes(model_path).await?;
        Self::load_model(model_path, bytes, load_context).await
    }

    async fn load_model(
        model_path: &str,
        bytes: Vec<u8>,
        load_context: &mut LoadContext<'_>,
    ) -> Result<WorldModelAsset, WorldModelAssetLoaderError> {
        let mut cursor = io::Cursor::new(&bytes);

        let root = Self::load_root(&mut cursor).await?;
        let groups = Self::load_groups(model_path, &root, load_context).await?;

        let images = Self::load_images(&root, load_context).await?;
        let materials = Self::load_materials(&root, &images, load_context);
        let default_material = Self::create_default_material(load_context);
        let meshes = Self::load_meshes(&groups, &materials, default_material);

        let mesh_handles: Vec<Handle<Mesh>> = meshes
            .iter()
            .enumerate()
            .map(|(i, m)| {
                load_context
                    .add_labeled_asset(WorldModelAssetLabel::Mesh(i).to_string(), m.mesh.clone())
            })
            .collect();

        let mut transform = Transform::default();
        transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);
        let mut world = World::default();
        let mut root = world.spawn((transform, Visibility::default()));
        for mesh_index in 0..meshes.len() {
            root.with_child((
                Mesh3d(mesh_handles[mesh_index].clone()),
                MeshMaterial3d(meshes[mesh_index].material.clone()),
            ));
        }
        let scene_loader = load_context.begin_labeled_asset();
        let loaded_scene = scene_loader.finish(Scene::new(world));
        let scene = load_context
            .add_loaded_labeled_asset(WorldModelAssetLabel::Root.to_string(), loaded_scene);

        Ok(WorldModelAsset {
            scene,
            image_handles: images,
            meshes,
            materials,
        })
    }

    async fn load_root(reader: &mut io::Cursor<&Vec<u8>>) -> Result<wmo::root_parser::WmoRoot> {
        let wmo::ParsedWmo::Root(root) = wmo::parse_wmo(reader)? else {
            return Err(anyhow!("WMO file is not a root WMO"));
        };
        Ok(root)
    }

    async fn load_groups(
        file_path: &str,
        root: &wmo::root_parser::WmoRoot,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<wmo::group_parser::WmoGroup>> {
        let mut groups = Vec::new();
        for group_index in 0..root.n_groups {
            let wmo_group = Self::load_group(file_path, group_index, load_context).await?;
            groups.push(wmo_group);
        }
        Ok(groups)
    }

    async fn load_group(
        file_path: &str,
        group_index: u32,
        load_context: &mut LoadContext<'_>,
    ) -> Result<wmo::group_parser::WmoGroup> {
        let group_filename = Self::get_group_filename(file_path, group_index);
        let bytes = load_context.read_asset_bytes(&group_filename).await?;
        let mut reader = io::Cursor::new(&bytes);
        let wmo::ParsedWmo::Group(group) = wmo::parse_wmo(&mut reader)? else {
            return Err(anyhow!("WMO file is not a group WMO: {}", group_filename));
        };
        Ok(group)
    }

    fn get_group_filename<P: AsRef<Path>>(wmo_path: P, group_index: u32) -> String {
        let base_path = wmo_path.as_ref().with_extension("");
        format!("archive://{}_{:03}.wmo", base_path.display(), group_index)
    }

    async fn load_images(
        root: &wmo::root_parser::WmoRoot,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Handle<Image>>> {
        let mut images = Vec::new();
        for image_path in Self::get_image_asset_paths(root) {
            // At this point we do not know which archive contains this texture.
            // But we have built a map of blp paths to their respective archives.
            let image = ImageLoader::load_image(&image_path, load_context).await?;
            images.push(image);
        }

        // Set image samplers
        for material in &root.materials {
            let texture_index = material.get_texture1_index(&root.texture_offset_index_map);
            let image = &mut images[texture_index as usize];
            let material_flags = wmo::WmoMaterialFlags::from_bits_truncate(material.flags);
            image.sampler = material::sampler_from_world_model_material_flags(material_flags);
        }

        // Now that the sampler is set, we can create image handles
        let image_handles = images
            .into_iter()
            .enumerate()
            .map(|(index, image)| {
                load_context
                    .add_labeled_asset(WorldModelAssetLabel::Image(index).to_string(), image)
            })
            .collect();

        Ok(image_handles)
    }

    fn get_image_asset_paths(root: &wmo::root_parser::WmoRoot) -> Vec<String> {
        root.textures
            .iter()
            .map(|texture| format!("archive://{}", texture))
            .collect()
    }

    fn load_materials(
        root: &wmo::root_parser::WmoRoot,
        images: &[Handle<Image>],
        load_context: &mut LoadContext<'_>,
    ) -> Vec<Handle<StandardMaterial>> {
        let mut materials = Vec::new();

        for (index, material) in root.materials.iter().enumerate() {
            let base_color = material::color_from_world_model(material.diff_color);
            let emissive = material::color_from_world_model(material.emissive_color).to_linear();

            let texture_index = material.get_texture1_index(&root.texture_offset_index_map);
            let image_handle = images[texture_index as usize].clone();

            let material_flags = wmo::WmoMaterialFlags::from_bits_truncate(material.flags);

            let unlit = material_flags.intersects(
                wmo::WmoMaterialFlags::UNLIT
                    | wmo::WmoMaterialFlags::EXTERIOR_LIGHT
                    | wmo::WmoMaterialFlags::WINDOW_LIGHT,
            );
            let cull_mode = if material_flags.contains(wmo::WmoMaterialFlags::TWO_SIDED) {
                None
            } else {
                Some(Face::Back)
            };

            let alpha_mode = material::alpha_mode_from_world_model_blend_mode(material.blend_mode);

            let material = StandardMaterial {
                base_color,
                emissive,
                perceptual_roughness: 1.0,
                base_color_texture: Some(image_handle),
                unlit,
                cull_mode,
                alpha_mode,
                ..Default::default()
            };

            let material_handle = load_context
                .add_labeled_asset(WorldModelAssetLabel::Material(index).to_string(), material);

            materials.push(material_handle);
        }

        materials
    }

    fn create_default_material(load_context: &mut LoadContext<'_>) -> Handle<StandardMaterial> {
        let default_material = StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            unlit: true,
            ..Default::default()
        };
        load_context.add_labeled_asset(
            WorldModelAssetLabel::Material(usize::MAX).to_string(),
            default_material,
        )
    }

    fn load_meshes(
        groups: &[wmo::group_parser::WmoGroup],
        materials: &[Handle<StandardMaterial>],
        default_material_handle: Handle<StandardMaterial>,
    ) -> Vec<WorldModelMesh> {
        let mut meshes = Vec::new();
        for group in groups {
            meshes.extend(Self::load_meshes_from_group(
                group,
                materials,
                default_material_handle.clone(),
            ));
        }
        meshes
    }

    fn load_meshes_from_group(
        group: &wmo::group_parser::WmoGroup,
        materials: &[Handle<StandardMaterial>],
        default_material_handle: Handle<StandardMaterial>,
    ) -> Vec<WorldModelMesh> {
        let positions: Vec<_> = group
            .vertex_positions
            .iter()
            .map(|v| [v.x, v.y, v.z])
            .collect();
        let normals: Vec<_> = group
            .vertex_normals
            .iter()
            .map(|v| normalize_vec3([v.x, v.y, v.z]))
            .collect();
        let tex_coords_0: Vec<_> = group.texture_coords.iter().map(|v| [v.u, v.v]).collect();
        let colors: Vec<_> = group
            .vertex_colors
            .iter()
            .map(|v| [v.r as f32, v.g as f32, v.b as f32, v.a as f32])
            .collect();

        let mut ret = Vec::new();

        for batch in &group.render_batches {
            let indices = group
                .vertex_indices
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

            let material_index = batch.material_id as usize;
            let material = if material_index < materials.len() {
                materials[material_index].clone()
            } else {
                default_material_handle.clone()
            };

            let world_model_mesh = WorldModelMesh { mesh, material };
            ret.push(world_model_mesh);
        }

        ret
    }
}

impl AssetLoader for WorldModelAssetLoader {
    type Asset = WorldModelAsset;
    type Settings = (); // No custom settings yet
    type Error = WorldModelAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let model_path = load_context.path().to_string_lossy().into_owned();
        Self::load_model(&model_path, bytes, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        &["wmo"]
    }
}
