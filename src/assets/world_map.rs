// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::io;

use anyhow::Result;
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::asset::{AssetPath, RenderAssetUsages};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::mesh::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use thiserror::Error;
use wow_adt as adt;

use crate::assets::*;

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct WorldMap;

/// Labels that can be used to load part of a Model
///
/// You can use [`WorldMapAssetLabel::from_asset`] to add it to an asset path
///
/// ```
/// # use bevy::prelude::*;
/// # use worgen_rs::assets::*;
///
/// fn load_model(asset_server: Res<AssetServer>) {
///     let mesh: Handle<Scene> = asset_server.load(WorldMapAssetLabel::Model(0).from_asset("model/path/extension"));
/// }
/// ```
///
/// Or when formatting a string for the path
///
/// ```
/// # use bevy::prelude::*;
/// # use worgen_rs::assets::*;
///
/// fn load_chunk(asset_server: Res<AssetServer>) {
///     let mesh: Handle<Scene> = asset_server.load(format!("model/path.extension#{}", WorldMapAssetLabel::Chunk(0)));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldMapAssetLabel {
    Root,
    CombinedAlpha(usize),
    TerrainMaterial(usize),
    Chunk(usize),
    Model(usize),
    WorldModel(usize),
    Image(usize),
    BoundingSphere,
}

impl core::fmt::Display for WorldMapAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WorldMapAssetLabel::Root => f.write_str("Root"),
            WorldMapAssetLabel::CombinedAlpha(index) => {
                f.write_fmt(format_args!("CombinedAlpha{index}"))
            }
            WorldMapAssetLabel::TerrainMaterial(index) => {
                f.write_fmt(format_args!("TerrainMaterial{index}"))
            }
            WorldMapAssetLabel::Chunk(index) => f.write_fmt(format_args!("Chunk{index}")),
            WorldMapAssetLabel::Model(index) => f.write_fmt(format_args!("Model{index}")),
            WorldMapAssetLabel::WorldModel(index) => f.write_fmt(format_args!("WorldModel{index}")),
            WorldMapAssetLabel::Image(index) => f.write_fmt(format_args!("Image{index}")),
            WorldMapAssetLabel::BoundingSphere => f.write_str("BoundingSphere"),
        }
    }
}

impl WorldMapAssetLabel {
    /// Add this label to an asset path
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use worgen_rs::assets::*;
    ///
    /// fn load_map(asset_server: Res<AssetServer>) {
    ///     let map: Handle<Scene> = asset_server.load(WorldMapAssetLabel::Root.from_asset("model/path.extension"));
    /// }
    /// ```
    pub fn from_asset(&self, path: impl Into<AssetPath<'static>>) -> AssetPath<'static> {
        path.into().with_label(self.to_string())
    }
}

#[derive(Asset, Debug, TypePath)]
pub struct WorldMapAsset {
    /// Scene loaded from the model, with reorientation applied.
    pub scene: Handle<Scene>,
    /// Image handles requested during load.
    pub images: Vec<Handle<Image>>,
    /// Alpha maps combined into RGBA textures.
    pub alphas: Vec<Handle<Image>>,
    /// Terrains created from chunks.
    pub terrains: Vec<WorldMapTerrain>,
    /// Model handles requested during load.
    pub models: Vec<Handle<ModelAsset>>,
    /// World model handles requested during load.
    pub world_models: Vec<Handle<WorldModelAsset>>,
    /// Bounding box
    pub aabb: RootAabb,
}

impl WorldMapAsset {
    pub fn get_all_images(&self) -> impl Iterator<Item = &Handle<Image>> {
        self.images.iter().chain(self.alphas.iter())
    }
}

static MAP_SIZE: f32 = 1600.0 * 32.0 / 3.0; // 17066.66

#[derive(Debug)]
pub struct WorldMapTerrain {
    pub bundle: TerrainBundle,
    pub combined_alpha: Handle<Image>,
}

#[derive(Bundle, Clone, Debug)]
pub struct TerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<ExtTerrainMaterial>,
    pub transform: Transform,
}

#[derive(Default)]
pub struct WorldMapAssetLoader;

#[derive(Debug, Error)]
pub enum WorldMapAssetLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] adt::AdtError),
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl WorldMapAssetLoader {
    async fn load_model(
        bytes: Vec<u8>,
        load_context: &mut LoadContext<'_>,
    ) -> Result<WorldMapAsset, WorldMapAssetLoaderError> {
        let mut cursor = io::Cursor::new(&bytes);
        let mut world_map = adt::Adt::from_reader(&mut cursor)?;

        Self::fix_model_extensions(&mut world_map);
        let images = Self::load_images(&world_map, load_context).await?;
        let terrains = Self::load_terrains(&world_map).await;
        let aabb = RootAabb::from_transformed_meshes(terrains.iter());
        let models = Self::load_models(&world_map, load_context);
        let world_models = Self::load_world_models(&world_map, load_context);

        let terrains = Self::process_terrains(&world_map, terrains, &images, load_context)?;

        let mut transform = Transform::default();
        transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);

        let mut world = World::default();
        let mut root = world.spawn((Transform::default(), WorldMap, aabb, Visibility::default()));

        let mut alphas = Vec::new();
        for terrain in &terrains {
            alphas.push(terrain.combined_alpha.clone());
            root.with_child(terrain.bundle.clone());
        }

        Self::place_models(&mut root, &world_map, load_context);
        Self::place_world_models(&mut root, &world_map, load_context);

        let scene_loader = load_context.begin_labeled_asset();
        let loaded_scene = scene_loader.finish(Scene::new(world));
        let scene = load_context
            .add_loaded_labeled_asset(WorldMapAssetLabel::Root.to_string(), loaded_scene);

        Ok(WorldMapAsset {
            scene,
            images,
            alphas,
            terrains,
            models,
            world_models,
            aabb,
        })
    }

    fn fix_model_extensions(world_map: &mut adt::Adt) {
        if let Some(mmdx) = &mut world_map.mmdx {
            for filename in &mut mmdx.filenames {
                if filename.ends_with(".mdx") {
                    filename.replace_range(filename.len() - 4..filename.len(), ".m2");
                }
            }
        }
    }

    async fn load_images(
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Handle<Image>>> {
        let mut images = Vec::new();
        for image_path in Self::get_image_asset_paths(world_map) {
            let image = load_context.load(image_path);
            images.push(image);
        }
        Ok(images)
    }

    fn get_image_paths(world_map: &adt::Adt) -> Vec<&String> {
        let mut paths = Vec::new();
        if let Some(mtex) = &world_map.mtex {
            for filename in &mtex.filenames {
                paths.push(filename);
            }
        }
        paths
    }

    fn get_image_asset_paths(world_map: &adt::Adt) -> Vec<String> {
        Self::get_image_paths(world_map)
            .iter()
            .map(|p| format!("archive://{}", p))
            .collect()
    }

    async fn load_terrains(world_map: &adt::Adt) -> Vec<TransformMesh> {
        let mut meshes = Vec::new();
        for chunk in &world_map.mcnk_chunks {
            let mesh = Self::create_mesh_from_world_map_chunk(chunk);
            meshes.push(mesh);
        }
        meshes
    }

    fn process_terrains(
        world_map: &adt::Adt,
        meshes: Vec<TransformMesh>,
        images: &[Handle<Image>],
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<WorldMapTerrain>> {
        let mut terrains = Vec::new();

        let header = world_map.mhdr.as_ref().unwrap();
        let has_big_alpha = header.flags & 0x4 != 0;

        for (index, mesh) in meshes.into_iter().enumerate() {
            let transform = mesh.transform;
            let mesh_handle = load_context
                .add_labeled_asset(WorldMapAssetLabel::Chunk(index).to_string(), mesh.mesh);

            let chunk = &world_map.mcnk_chunks[index];

            let mut layer_textures = [None, None, None, None];
            for (i, layer) in chunk.texture_layers.iter().enumerate() {
                let image_index = layer.texture_id as usize;
                layer_textures[i] = images.get(image_index).cloned();
            }

            let bit_16th = 1 << 15;
            let fix_alpha = chunk.flags & bit_16th == 0;
            let alpha_texture = Self::create_alpha_texture_from_world_map_chunk(
                chunk,
                index,
                load_context,
                has_big_alpha,
                fix_alpha,
            );

            let material = StandardMaterial {
                base_color_texture: layer_textures[0].clone(),
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                cull_mode: None,
                ..Default::default()
            };

            let terrain_material = TerrainMaterial {
                level_mask: 0,
                level_count: chunk.texture_layers.len() as u32,
                alpha_texture: alpha_texture.clone(),
                level1_texture: layer_textures[1].clone(),
                level2_texture: layer_textures[2].clone(),
                level3_texture: layer_textures[3].clone(),
            };

            let extended_material = ExtendedMaterial {
                base: material,
                extension: terrain_material,
            };

            let material_handle = load_context.add_labeled_asset(
                WorldMapAssetLabel::TerrainMaterial(index).to_string(),
                extended_material,
            );

            let bundle = TerrainBundle {
                mesh: Mesh3d(mesh_handle),
                material: MeshMaterial3d(material_handle),
                transform,
            };

            let terrain = WorldMapTerrain {
                bundle,
                combined_alpha: alpha_texture,
            };

            terrains.push(terrain);
        }

        Ok(terrains)
    }

    /// Create mesh for a terrain chunk.
    ///
    /// # Vertex layout
    /// - Each chunk uses 145 vertices arranged in a staggered 17-row grid. Rows alternate
    ///   between 9 and 8 vertices, starting with 9 on row 0. The layout (0-based indices):
    ///
    /// - `VERTEX_COUNT` is set to 145 (8*8 + 9*9). Positions are computed from a staggered grid
    ///   with a 0.5 step in the packed ADT layout; z comes from the `height_map` vector.
    ///
    /// # Index buffer and winding
    /// - The index buffer is produced by `terrain_indices()` and uses 0-based indices.
    /// - The generator produces 256 triangles (768 indices) by constructing a 4-triangle fan
    ///   per quad using the middle (odd-row) vertex as the center. The triangles are emitted
    ///   with counter-clockwise (CCW) winding so front faces are consistent with the engine's
    ///   default.
    pub fn create_mesh_from_world_map_chunk(chunk: &adt::McnkChunk) -> TransformMesh {
        static VERTEX_COUNT: usize = 145; // 8*8 + 9*9
        let mut positions = vec![[0.0, 0.0, 0.0]; VERTEX_COUNT];
        let mut normals = vec![[0.0, 0.0, 0.0]; VERTEX_COUNT];
        let mut tex_coords = vec![[0.0, 0.0]; VERTEX_COUNT];
        let indices = Self::terrain_indices();

        for i in 0..VERTEX_COUNT {
            // With these offset we can imagine 17 vertices for the
            // first 8 rows, and 9 vertices for the last row.
            let row_index = i % 17;
            let z_offset = (i / 17) as f32;
            let x_offset = row_index as f32;

            let mut z_suboffset = 0.0;
            let mut x_suboffset = 0.0;
            // Step is 0.5
            if row_index >= 9 {
                // Move the last 8 vertices of this row to a new line (control)
                z_suboffset = 0.5;
                x_suboffset = 0.5 - 9.0;
            }

            let x = x_offset + x_suboffset;
            let z = z_offset + z_suboffset;

            static UV_SCALE: f32 = 8.0;
            tex_coords[i] = [x / UV_SCALE, z / UV_SCALE];
            positions[i] = [-x, chunk.height_map[i], -z];

            let normal: [u8; 3] = [
                chunk.normals[i][1],
                chunk.normals[i][2],
                chunk.normals[i][0],
            ];
            normals[i] = from_normalized_vec3_u8(normal);
        }

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
            positions.clone(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords.clone())
        .with_inserted_indices(Indices::U16(indices.clone()));

        // Each chunk is 100 feet -> 33.33 yards in world space.
        // Our grid size is 8, so we scale by (100.0 / 3.0) / 8.0 = 100.0 / 24.0
        static CHUNK_SCALE: f32 = 100.0 / 24.0;

        // 1600 feet -> 533.33 yards

        let x = chunk.position[0];
        let y = chunk.position[1];
        let z = chunk.position[2];
        let transform = Transform::default()
            .with_translation(vec3(x, y, z))
            .with_scale(vec3(CHUNK_SCALE, 1.0, CHUNK_SCALE));

        TransformMesh { mesh, transform }
    }

    /// Generate the triangle index buffer (CCW) for a 145-vertex chunk:
    /// rows alternate 9 and 8 vertices (starting with 9), for 17 total rows.
    /// Returns 256 triangles = 768 indices (u16), suitable for draw_indexed.
    pub fn terrain_indices() -> Vec<u16> {
        fn row_len(r: usize) -> usize {
            if r.is_multiple_of(2) { 9 } else { 8 }
        }
        fn row_start(r: usize) -> usize {
            // Prefix sum of row lengths up to r (exclusive)
            (0..r).map(row_len).sum()
        }

        // We build 4 triangles per quad (8x8 quads between even rows), using the center vertex
        // from the odd row between two even rows. This yields 256 triangles total.
        // For each band of rows (even r, odd r+1, even r+2) and each column c in 0..8 (quads 0..7),
        // we define the following vertices:
        //   t0 = (even r, c)
        //   t1 = (even r, c+1)
        //   m  = (odd  r+1, c)           // center of the quad
        //   b0 = (even r+2, c)
        //   b1 = (even r+2, c+1)
        // And add triangles (CCW):
        //   (t0, b0, m), (m, b0, b1), (m, b1, t1), (m, t1, t0)

        let mut indices = Vec::with_capacity(256 * 3);

        for r in (0..=14).step_by(2) {
            let top_start = row_start(r);
            let mid_start = row_start(r + 1);
            let bot_start = row_start(r + 2);

            for c in 0..8 {
                let t0 = top_start + c;
                let t1 = top_start + c + 1;
                let m = mid_start + c;
                let b0 = bot_start + c;
                let b1 = bot_start + c + 1;

                indices.extend_from_slice(&[
                    t0 as u16, b0 as u16, m as u16, m as u16, b0 as u16, b1 as u16, m as u16,
                    b1 as u16, t1 as u16, m as u16, t1 as u16, t0 as u16,
                ]);
            }
        }

        indices
    }

    /// Create and register a combined RGBA alpha texture for a terrain chunk.
    pub fn create_alpha_texture_from_world_map_chunk(
        chunk: &adt::McnkChunk,
        index: usize,
        load_context: &mut LoadContext<'_>,
        has_big_alpha: bool,
        fix_alpha: bool,
    ) -> Handle<Image> {
        let combined_alpha = adt::CombinedAlphaMap::new(chunk, has_big_alpha, fix_alpha);
        let image_size: Extent3d = Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        };
        let combined_alpha_image = Image::new_fill(
            image_size,
            TextureDimension::D2,
            combined_alpha.as_slice(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        load_context.add_labeled_asset(
            WorldMapAssetLabel::CombinedAlpha(index).to_string(),
            combined_alpha_image,
        )
    }

    fn load_models(
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) -> Vec<Handle<ModelAsset>> {
        let mut models = Vec::new();
        for model_path in Self::get_model_asset_paths(world_map) {
            models.push(load_context.load(model_path));
        }
        models
    }

    fn get_model_asset_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut models = Vec::new();
        if let Some(mmdx) = &world_map.mmdx {
            models.extend(
                mmdx.filenames
                    .iter()
                    .filter(|f| f.ends_with(".m2"))
                    .map(|f| format!("archive://{}", f)),
            );
        }
        models
    }

    fn place_models(
        root: &mut EntityWorldMut<'_>,
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) {
        let model_asset_paths = Self::get_model_asset_paths(world_map);

        if let Some(mddf) = &world_map.mddf {
            for placement in &mddf.doodads {
                let model_path = model_asset_paths[placement.name_id as usize].clone();

                let scene = load_context.load(ModelAssetLabel::Root.from_asset(model_path));

                let transform = Transform::default()
                    .with_translation(vec3(
                        MAP_SIZE - placement.position[0],
                        placement.position[1],
                        MAP_SIZE - placement.position[2],
                    ))
                    .with_rotation(
                        Quat::from_axis_angle(Vec3::X, placement.rotation[0].to_radians())
                            * Quat::from_axis_angle(Vec3::Y, placement.rotation[1].to_radians())
                            * Quat::from_axis_angle(Vec3::Z, placement.rotation[2].to_radians()),
                    )
                    .with_scale(Vec3::splat(placement.scale));

                root.with_child((SceneRoot(scene), transform));
            }
        }
    }

    fn load_world_models(
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) -> Vec<Handle<WorldModelAsset>> {
        let mut world_models = Vec::new();
        for world_model_path in Self::get_world_model_asset_paths(world_map) {
            world_models.push(load_context.load(&world_model_path));
        }
        world_models
    }

    fn get_world_model_asset_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(modf) = &world_map.modf
            && let Some(mwmo) = &world_map.mwmo
        {
            let filenames = &mwmo.filenames;
            for model in &modf.models {
                if let Some(filename) = filenames.get(model.name_id as usize) {
                    paths.push(format!("archive://{}", filename));
                }
            }
        }
        paths
    }

    fn place_world_models(
        root: &mut EntityWorldMut<'_>,
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) {
        let paths = Self::get_world_model_asset_paths(world_map);

        if let Some(modf) = &world_map.modf {
            for model in &modf.models {
                let model_path = &paths[model.name_id as usize];
                let scene =
                    load_context.load(WorldModelAssetLabel::Root.from_asset(model_path.clone()));

                let rotation = vec3(
                    model.rotation[0],
                    180.0 + model.rotation[1],
                    model.rotation[2],
                );

                let transform = Transform::default()
                    .with_translation(vec3(
                        MAP_SIZE - model.position[0],
                        model.position[1],
                        MAP_SIZE - model.position[2],
                    ))
                    .with_rotation(
                        Quat::from_axis_angle(Vec3::X, rotation[0].to_radians())
                            * Quat::from_axis_angle(Vec3::Y, rotation[1].to_radians())
                            * Quat::from_axis_angle(Vec3::Z, rotation[2].to_radians()),
                    );

                root.with_child((SceneRoot(scene), transform));
            }
        }
    }
}

impl AssetLoader for WorldMapAssetLoader {
    type Asset = WorldMapAsset;
    type Settings = ();
    type Error = WorldMapAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Self::load_model(bytes, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        &["adt"]
    }
}

pub fn is_world_map_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".adt")
}

#[cfg(test)]
mod test {
    use crate::{assets::test::*, settings::TestSettings};

    use super::*;

    #[test]
    fn test_terrain() -> Result<()> {
        let mut app = test_app();
        app.update();
        let settings = TestSettings::load()?;
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle: Handle<WorldMapAsset> =
            asset_server.load(format!("archive://{}", settings.test_terrain_path));
        let handle_id = handle.id();
        app.update();
        run_app_until(&mut app, |_world| {
            let load_state = asset_server.get_load_state(handle_id).unwrap();
            if load_state.is_loaded() {
                Some(())
            } else {
                None
            }
        });
        let load_state = asset_server.get_load_state(handle_id).unwrap();
        assert!(load_state.is_loaded());
        Ok(())
    }
}
