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
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use thiserror::Error;
use wow_adt as adt;

use crate::assets::*;
use crate::material::TerrainMaterial;

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
    CombinedAlpha,
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
            WorldMapAssetLabel::CombinedAlpha => f.write_str("CombinedAlpha"),
            WorldMapAssetLabel::TerrainMaterial(index) => {
                f.write_str(&format!("TerrainMaterial{index}"))
            }
            WorldMapAssetLabel::Chunk(index) => f.write_str(&format!("Chunk{index}")),
            WorldMapAssetLabel::Model(index) => f.write_str(&format!("Model{index}")),
            WorldMapAssetLabel::WorldModel(index) => f.write_str(&format!("WorldModel{index}")),
            WorldMapAssetLabel::Image(index) => f.write_str(&format!("Image{index}")),
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
    /// Image handles requested during load (populated by the loader).
    pub image_handles: Vec<Handle<Image>>,
    /// Terrain bundles created from chunks.
    pub terrain_bundles: Vec<TerrainBundle>,
    /// Bounding box
    pub aabb: RootAabb,
}

#[derive(Debug)]
pub struct WorldMapMesh {
    pub mesh: Mesh,
    pub transform: Transform,
}

impl WorldMapMesh {
    pub fn compute_aabb(&self) -> Option<RootAabb> {
        Some(RootAabb {
            aabb: self.mesh.compute_aabb()?,
        })
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct TerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
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
        let terrain = Self::load_terrain(&world_map).await;
        let aabb = RootAabb::new(&terrain);
        let models = Self::load_models(&world_map, load_context).await?;
        let world_models = Self::load_world_models(&world_map, load_context).await?;

        let terrain = Self::process_terrain(&world_map, terrain, &images, load_context)?;
        //let models = Self::process_models(models, load_context);
        //let world_models = Self::process_world_models(world_models, load_context)?;

        let mut transform = Transform::default();
        transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);

        let mut world = World::default();
        let mut root = world.spawn((Transform::default(), Visibility::default(), aabb));

        for terrain in &terrain {
            root.with_child(terrain.clone());
        }

        let scene_loader = load_context.begin_labeled_asset();
        let loaded_scene = scene_loader.finish(Scene::new(world));
        let scene = load_context
            .add_loaded_labeled_asset(WorldMapAssetLabel::Root.to_string(), loaded_scene);

        Ok(WorldMapAsset {
            scene,
            image_handles: images,
            terrain_bundles: terrain,
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
        for (index, image_path) in Self::get_image_asset_paths(world_map).iter().enumerate() {
            let image = ImageLoader::load_image(image_path, load_context).await?;
            let handle =
                load_context.add_labeled_asset(WorldMapAssetLabel::Image(index).to_string(), image);
            images.push(handle);
        }
        Ok(images)
    }

    fn get_image_asset_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut paths = Vec::new();
        if let Some(mtex) = &world_map.mtex {
            for filename in &mtex.filenames {
                paths.push(format!("archive://{}", filename));
            }
        }
        paths
    }

    async fn load_terrain(world_map: &adt::Adt) -> Vec<WorldMapMesh> {
        let mut meshes = Vec::new();
        for chunk in &world_map.mcnk_chunks {
            let mesh = Self::create_mesh_from_world_map_chunk(chunk);
            meshes.push(mesh);
        }
        meshes
    }

    fn process_terrain(
        world_map: &adt::Adt,
        meshes: Vec<WorldMapMesh>,
        images: &[Handle<Image>],
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<TerrainBundle>> {
        let mut bundles = Vec::new();

        let header = world_map.mhdr.as_ref().unwrap();
        let has_big_alpha = header.flags & 0x4 != 0;

        for (index, mesh) in meshes.into_iter().enumerate() {
            let transform = mesh.transform;
            let mesh_handle = load_context
                .add_labeled_asset(WorldMapAssetLabel::Chunk(index).to_string(), mesh.mesh);

            let chunk = &world_map.mcnk_chunks[index];

            let mut level0_texture_handle = None;

            if let Some(level0) = chunk.texture_layers.first() {
                let image_index = level0.texture_id as usize;
                level0_texture_handle = images.get(image_index).cloned();
            }

            let bit_16th = 1 << 15;
            let do_not_fix_alpha = chunk.flags & bit_16th != 0;
            let alpha_texture = Self::create_alpha_texture_from_world_map_chunk(
                chunk,
                load_context,
                has_big_alpha,
                do_not_fix_alpha,
            );

            let mut level1_texture_handle = None;
            let mut level2_texture_handle = None;
            let mut level3_texture_handle = None;

            if let Some(level1) = chunk.texture_layers.get(1) {
                let image_index = level1.texture_id as usize;
                level1_texture_handle = images.get(image_index).cloned();
            }
            if let Some(level2) = chunk.texture_layers.get(2) {
                let image_index = level2.texture_id as usize;
                level2_texture_handle = images.get(image_index).cloned();
            }
            if let Some(level3) = chunk.texture_layers.get(3) {
                let image_index = level3.texture_id as usize;
                level3_texture_handle = images.get(image_index).cloned();
            }

            let material = StandardMaterial {
                base_color_texture: level0_texture_handle,
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                unlit: false,
                cull_mode: None,
                ..Default::default()
            };

            let terrain_material = TerrainMaterial {
                level_count: chunk.texture_layers.len() as u32,
                alpha_texture,
                level1_texture: level1_texture_handle,
                level2_texture: level2_texture_handle,
                level3_texture: level3_texture_handle,
            };

            let extended_material = ExtendedMaterial {
                base: material,
                extension: terrain_material,
            };

            let material_handle = load_context.add_labeled_asset(
                WorldMapAssetLabel::TerrainMaterial(index).to_string(),
                extended_material,
            );

            bundles.push(TerrainBundle {
                mesh: Mesh3d(mesh_handle),
                material: MeshMaterial3d(material_handle),
                transform,
            });
        }

        Ok(bundles)
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
    pub fn create_mesh_from_world_map_chunk(chunk: &adt::McnkChunk) -> WorldMapMesh {
        static VERTEX_COUNT: usize = 145; // 8*8 + 9*9
        let mut positions = vec![[0.0, 0.0, 0.0]; VERTEX_COUNT];
        let mut normals = vec![[0.0, 0.0, 0.0]; VERTEX_COUNT];
        let mut tex_coords = vec![[0.0, 0.0]; VERTEX_COUNT];
        let indices = Self::terrain_indices();

        for i in 0..VERTEX_COUNT {
            // With these offset we can imagine 17 vertices for the
            // first 8 rows, and 9 vertices for the last row.
            let row_index = i % 17;
            let y_offset = (i / 17) as f32;
            let x_offset = row_index as f32;

            let mut y_suboffset = 0.0;
            let mut x_suboffset = 0.0;
            // Step is 0.5
            if row_index >= 9 {
                // Move the last 8 vertices of this row to a new line (control)
                y_suboffset = 0.5;
                x_suboffset = 0.5 - 9.0;
            }

            let x = x_offset + x_suboffset;
            let y = y_offset + y_suboffset;

            static UV_SCALE: f32 = 8.0;
            tex_coords[i] = [x / UV_SCALE, y / UV_SCALE];
            positions[i] = [x, y, chunk.height_map[i]];
            normals[i] = from_normalized_vec3_u8(chunk.normals[i]);
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
        static MAP_SIZE: f32 = 1600.0 * 32.0 / 3.0; // 17066.66

        let mut transform = Transform::default()
            .with_translation(vec3(
                MAP_SIZE - chunk.position[0],
                chunk.position[1],
                MAP_SIZE - chunk.position[2],
            ))
            .with_scale(vec3(CHUNK_SCALE, -CHUNK_SCALE, 1.0));

        //transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);
        //transform.rotate_local_y(-std::f32::consts::FRAC_PI_2);
        transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);

        WorldMapMesh { mesh, transform }
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

    /// Create a combined RGBA texture from the alpha maps of a world map chunk.
    /// Each alpha map is stored in a separate channel:
    /// - R: Level 1 alpha
    /// - G: Level 2 alpha
    /// - B: Level 3 alpha
    /// - A: Unused
    ///
    /// If `has_big_alpha` is true, each alpha value is 8 bits (1 byte),
    /// otherwise as 4 bits (half a byte).
    ///
    /// If `do_not_fix_alpha` is true, we should read a 63*63 map with the last row
    /// and column being equivalent to the previous one
    pub fn create_alpha_texture_from_world_map_chunk(
        chunk: &adt::McnkChunk,
        load_context: &mut LoadContext<'_>,
        has_big_alpha: bool,
        do_not_fix_alpha: bool,
    ) -> Handle<Image> {
        let mut combined_alpha = CombinedAlphaMap::new(do_not_fix_alpha);

        // Put level 1 alpha in R channel, level 2 in G channel, level 3 in B channel
        for (level, alpha) in chunk.alpha_maps.iter().enumerate() {
            for &alpha_value in alpha.iter() {
                if has_big_alpha {
                    // alpha is one byte here
                    combined_alpha.set_next_alpha(level, alpha_value);
                } else {
                    // Convert 4-bit alpha to 8-bit alpha
                    // We set two pixels at a time since each byte contains two 4-bit alpha values
                    combined_alpha.set_next_alpha(level, (alpha_value & 0x0F) * 16);
                    combined_alpha.set_next_alpha(level, ((alpha_value >> 4) & 0x0F) * 16);
                }
            }
        }

        load_context.add_labeled_asset(
            WorldMapAssetLabel::CombinedAlpha.to_string(),
            combined_alpha.to_image(),
        )
    }

    async fn load_models(
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<ModelAsset>> {
        let mut models = Vec::new();
        for model_path in Self::get_model_asset_paths(world_map) {
            let model = ModelAssetLoader::load_path(&model_path, load_context).await?;
            models.push(model);
        }
        Ok(models)
    }

    fn process_models(
        models: Vec<ModelAsset>,
        load_context: &mut LoadContext<'_>,
    ) -> Vec<Handle<ModelAsset>> {
        let mut handles = Vec::new();
        for (index, model) in models.into_iter().enumerate() {
            let handle =
                load_context.add_labeled_asset(WorldMapAssetLabel::Model(index).to_string(), model);
            handles.push(handle);
        }
        handles
    }

    fn get_model_asset_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut models = Vec::new();
        if let Some(mmdx) = &world_map.mmdx {
            models.extend(mmdx.filenames.iter().filter_map(|f| {
                if f.ends_with(".m2") {
                    Some(format!("archive://{}", f))
                } else {
                    None
                }
            }));
        }
        models
    }

    async fn load_world_models(
        world_map: &adt::Adt,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<WorldModelAsset>> {
        let mut world_models = Vec::new();
        for world_model_path in Self::get_world_model_asset_paths(world_map) {
            let world_model =
                WorldModelAssetLoader::load_path(&world_model_path, load_context).await?;
            world_models.push(world_model);
        }
        Ok(world_models)
    }

    fn process_world_models(
        world_models: Vec<WorldModelAsset>,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Handle<WorldModelAsset>>> {
        let mut handles = Vec::new();
        for (index, world_model) in world_models.into_iter().enumerate() {
            let handle = load_context.add_labeled_asset(
                WorldMapAssetLabel::WorldModel(index).to_string(),
                world_model,
            );
            handles.push(handle);
        }
        Ok(handles)
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
}

impl AssetLoader for WorldMapAssetLoader {
    type Asset = WorldMapAsset;
    type Settings = (); // No custom settings yet
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

struct CombinedAlphaMap {
    map: [[[u8; 4]; 64]; 64],
    current_x: usize,
    current_y: usize,

    /// If `do_not_fix` is true, we should read a 63*63 map with the last row
    /// and column being equivalent to the previous one
    do_not_fix: bool,
}

impl CombinedAlphaMap {
    fn new(do_not_fix: bool) -> Self {
        Self {
            map: [[[0u8; 4]; 64]; 64],
            current_x: 0,
            current_y: 0,
            do_not_fix,
        }
    }

    fn set_alpha(&mut self, x: usize, y: usize, level: usize, alpha: u8) {
        if y < 64 && x < 64 && level < 4 {
            self.map[y][x][level] = alpha;
        }
    }

    fn set_next_alpha(&mut self, level: usize, alpha: u8) {
        self.set_alpha(self.current_x, self.current_y, level, alpha);
        self.advance();

        // If we are at the last row or column and do_not_fix is true,
        // duplicate the last value to fill the 64x64 texture
        if self.do_not_fix {
            if self.current_x == 63 {
                self.set_alpha(self.current_x, self.current_y, level, alpha);
                self.advance();
            }
            if self.current_y == 63 {
                let prev_x = if self.current_x == 0 {
                    63
                } else {
                    self.current_x - 1
                };
                let alpha = self.map[self.current_y - 1][prev_x][level];
                self.set_alpha(self.current_x, self.current_y, level, alpha);
                self.advance();
            }
        }
    }

    fn advance(&mut self) {
        self.current_x += 1;
        if self.current_x >= 64 {
            self.current_x = 0;
            self.current_y += 1;
        }
    }

    fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.map.as_ptr() as *const u8,
                std::mem::size_of_val(&self.map),
            )
        }
    }

    fn to_image(&self) -> Image {
        let image_size: Extent3d = Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        };
        Image::new_fill(
            image_size,
            TextureDimension::D2,
            self.as_slice(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct RootAabb {
    pub aabb: Aabb,
}

impl RootAabb {
    fn new(meshes: &[WorldMapMesh]) -> Self {
        let mut ret = Self::default();
        for mesh in meshes {
            if let Some(mut mesh_aabb) = mesh.compute_aabb() {
                mesh_aabb.transform(&mesh.transform);
                ret.extend(&mesh_aabb);
            }
        }
        ret
    }

    fn transform(&mut self, transform: &Transform) {
        let matrix = transform.compute_matrix();
        self.aabb.center = matrix.transform_point3(self.aabb.center.into()).into();
        self.aabb.half_extents = matrix
            .transform_vector3(self.aabb.half_extents.into())
            .into();
    }

    fn extend(&mut self, b: &RootAabb) {
        let min_a = self.aabb.min();
        let max_a = self.aabb.max();
        let min_b = b.aabb.min();
        let max_b = b.aabb.max();
        let min = min_a.min(min_b);
        let max = max_a.max(max_b);
        self.aabb.center = (min + max) * 0.5;
        self.aabb.half_extents = (max - min) * 0.5;
    }
}

pub fn is_world_map_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".adt")
}
