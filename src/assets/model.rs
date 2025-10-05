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
use std::path::PathBuf;

use anyhow::Result;
use bevy::asset::io::Reader;
use bevy::asset::*;
use bevy::image::ImageLoaderSettings;
use bevy::mesh::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use thiserror::Error;
use wow_m2 as m2;

use crate::assets::*;
use crate::settings::Settings;

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct Model {
    pub name: String,
    pub images: Vec<Handle<Image>>,
    pub materials: Vec<Handle<StandardMaterial>>,
}

impl Model {
    pub fn new(
        path: &str,
        images: Vec<Handle<Image>>,
        materials: Vec<Handle<StandardMaterial>>,
    ) -> Self {
        let fixed_path = path.replace("\\", "/");
        let name = PathBuf::from(fixed_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        Self {
            name,
            images,
            materials,
        }
    }
}

/// Labels that can be used to load part of a Model
///
/// You can use [`ModelAssetLabel::from_asset`] to add it to an asset path
///
/// ```
/// # use bevy::prelude::*;
/// # use worgen_rs::assets::*;
///
/// fn load_mesh(asset_server: Res<AssetServer>) {
///     let mesh: Handle<Scene> = asset_server.load(ModelAssetLabel::Mesh(0).from_asset("model/path/extension"));
/// }
/// ```
///
/// Or when formatting a string for the path
///
/// ```
/// # use bevy::prelude::*;
/// # use worgen_rs::assets::*;
///
/// fn load_mesh(asset_server: Res<AssetServer>) {
///     let mesh: Handle<Scene> = asset_server.load(format!("model/path.extension#{}", ModelAssetLabel::Mesh(0)));
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelAssetLabel {
    Root,
    Mesh(usize),
    Material(usize),
    Image(usize),
    BoundingSphere,
}

impl core::fmt::Display for ModelAssetLabel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ModelAssetLabel::Root => f.write_str("Root"),
            ModelAssetLabel::Mesh(index) => f.write_str(&format!("Mesh{index}")),
            ModelAssetLabel::Material(index) => f.write_str(&format!("Material{index}")),
            ModelAssetLabel::Image(index) => f.write_str(&format!("Image{index}")),
            ModelAssetLabel::BoundingSphere => f.write_str("BoundingSphere"),
        }
    }
}

impl ModelAssetLabel {
    /// Add this label to an asset path
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use worgen_rs::assets::*;
    ///
    /// fn load_model(asset_server: Res<AssetServer>) {
    ///     let model: Handle<Scene> = asset_server.load(ModelAssetLabel::Root.from_asset("model/path.extension"));
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
    /// Generated mesh handles after preparation.
    pub meshes: Vec<Handle<Mesh>>,
    /// Axis-aligned bounding box of the model's meshes.
    pub aabb: RootAabb,
}

/// Helper class to reduce the number of parameters passed around when creating meshes.
#[derive(Default)]
pub struct MeshData {
    meshes: Vec<Mesh>,
    materials: Vec<StandardMaterial>,
    geosets: Vec<Geoset>,
}

#[derive(Default)]
pub struct ModelAssetLoader;

#[derive(Debug, Error)]
pub enum ModelAssetLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] m2::M2Error),
    #[error("Read error: {0}")]
    Read(#[from] ReadAssetBytesError),
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

impl ModelAssetLoader {
    pub async fn load_path(
        model_path: &str,
        load_context: &mut LoadContext<'_>,
    ) -> Result<ModelAsset, ModelAssetLoaderError> {
        let model_asset_path = format!("archive://{}", model_path);
        let bytes = load_context.read_asset_bytes(&model_asset_path).await?;
        Self::load_model(model_path, bytes, load_context).await
    }

    pub async fn load_model(
        model_path: &str,
        bytes: Vec<u8>,
        load_context: &mut LoadContext<'_>,
    ) -> Result<ModelAsset, ModelAssetLoaderError> {
        let mut cursor = io::Cursor::new(&bytes);
        let model = m2::M2Model::parse(&mut cursor)?;

        let images = Self::load_images(&model, load_context);
        let mut data = MeshData::default();
        Self::load_meshes(&model, &bytes, &images, &mut data)?;

        let mut transform = Transform::default();
        transform.rotate_local_x(-std::f32::consts::FRAC_PI_2);
        transform.rotate_local_z(-std::f32::consts::FRAC_PI_2);

        let aabb = RootAabb::from_meshes_with_transform(data.meshes.iter(), &transform);

        let meshes: Vec<Handle<Mesh>> = data
            .meshes
            .into_iter()
            .enumerate()
            .map(|(i, m)| load_context.add_labeled_asset(ModelAssetLabel::Mesh(i).to_string(), m))
            .collect();
        let materials: Vec<Handle<StandardMaterial>> = data
            .materials
            .into_iter()
            .enumerate()
            .map(|(i, mat)| {
                load_context.add_labeled_asset(ModelAssetLabel::Material(i).to_string(), mat)
            })
            .collect();

        let mut world = World::default();

        let model = Model::new(model_path, images, materials.clone());

        let mut root = world.spawn((transform, model, aabb, Visibility::default()));

        // Make sure to hide conflicting geosets
        let mut geosets_seen = HashSet::new();

        for i in 0..meshes.len() {
            let geoset_type = GeosetType::from_geoset(data.geosets[i]);
            let visibility =
                if geoset_type == GeosetType::SkinBase || geosets_seen.insert(geoset_type) {
                    Visibility::default()
                } else {
                    Visibility::Hidden
                };

            root.with_child((
                Mesh3d(meshes[i].clone()),
                MeshMaterial3d(materials[i].clone()),
                data.geosets[i],
                visibility,
            ));
        }
        let scene_loader = load_context.begin_labeled_asset();
        let loaded_scene = scene_loader.finish(Scene::new(world));
        let scene =
            load_context.add_loaded_labeled_asset(ModelAssetLabel::Root.to_string(), loaded_scene);

        Ok(ModelAsset {
            scene,
            meshes,
            aabb,
        })
    }

    fn load_images(model: &m2::M2Model, load_context: &mut LoadContext<'_>) -> Vec<Handle<Image>> {
        let mut handles = Vec::new();
        for texture in &model.textures {
            let image_path = Self::get_image_asset_path(texture);
            let sampler = sampler_from_model_texture_flags(texture.flags);
            handles.push(
                load_context
                    .loader()
                    .with_settings(move |settings: &mut ImageLoaderSettings| {
                        settings.sampler = sampler.clone();
                    })
                    .load(image_path),
            );
        }
        handles
    }

    fn get_image_path(texture: &m2::chunks::texture::M2Texture) -> String {
        if texture.texture_type != m2::chunks::M2TextureType::Hardcoded {
            // Ignore non-hardcoded textures for now.
            return Settings::get().test_image_path.clone();
        }
        let filename = texture.filename.string.to_string_lossy();
        if filename.is_empty() {
            return Settings::get().test_image_path.clone();
        }
        filename.to_string()
    }

    fn get_image_asset_path(texture: &m2::chunks::texture::M2Texture) -> String {
        format!("archive://{}", Self::get_image_path(texture))
    }

    fn load_meshes(
        model: &m2::M2Model,
        model_bytes: &[u8],
        images: &[Handle<Image>],
        data: &mut MeshData,
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
            Self::load_submesh(model, &skin, batch_index, &vertex_attributes, images, data)?;
        }
        Ok(())
    }

    fn load_submesh(
        model: &m2::M2Model,
        skin: &m2::skin::SkinFile,
        batch_index: usize,
        vertex_attributes: &VertexAttributes,
        images: &[Handle<Image>],
        data: &mut MeshData,
    ) -> Result<()> {
        let batch = &skin.batches()[batch_index];
        let submesh = &skin.submeshes()[batch.skin_section_index as usize];

        let geoset = Geoset { id: submesh.id };
        data.geosets.push(geoset);

        let texture_index =
            model.raw_data.texture_lookup_table[batch.texture_combo_index as usize] as usize;
        // Textures must be already loaded.
        let image = &images[texture_index];

        // Determine alpha mode from material blend mode.
        // Note that multiple batches can share the same material.
        let model_material = &model.materials[batch.material_index as usize];
        let alpha_mode = alpha_mode_from_model_blend_mode(model_material.blend_mode);
        let base_color = color_from_batch_model_color(model, batch);
        let cull_mode = if model_material
            .flags
            .contains(m2::chunks::material::M2RenderFlags::NO_BACKFACE_CULLING)
        {
            None
        } else {
            Some(Face::Back)
        };
        let unlit = model_material
            .flags
            .contains(m2::chunks::material::M2RenderFlags::UNLIT);

        let material = StandardMaterial {
            base_color,
            base_color_texture: Some(image.clone()),
            perceptual_roughness: 1.0,
            alpha_mode,
            cull_mode,
            unlit,
            ..Default::default()
        };

        data.materials.push(material);

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

        data.meshes.push(mesh);

        Ok(())
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
        let model_path = load_context.path().to_string_lossy().into_owned();
        Self::load_model(&model_path, bytes, load_context).await
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

pub fn is_model_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".m2")
        || lower_filename.ends_with(".mdx")
        || lower_filename.ends_with(".mdl")
}

#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct Geoset {
    pub id: u16,
}

/// High-level grouping for character model component / equipment visibility.
///
/// This enum encodes commonly observed geoset category indices used by a
/// character model format. They are typically identified by the high-order
/// digits of the geoset id (e.g. `15xx` for a cloak / cape group). The trailing
/// digits select a style / variation within that category (e.g. different
/// silhouette shapes, beard styles, etc). The definitions are intentionally
/// coarse – they do not enumerate every style id, only the parent category. Use
/// the raw `GeosetType.id` (or a future helper) if exact variant selection is
/// required.
///
/// Sources: community reverse engineering of the binary model format and
/// inspection of shipped assets. This is best-effort and may evolve.
/// https://wowdev.wiki/Character_Customization#Geosets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GeosetType {
    /// Base skin / body root (id 0000). Often always present.
    SkinBase,
    /// Hair / head styles (`00**` except 0000) – variants encode hairstyle meshes.
    Hair,
    /// Facial hair group 1 / "Facial1" (`01**`) – usually beards (style 1..=8 typical).
    Facial1,
    /// Facial hair group 2 / sideburns / alt moustache (`02**`). Style 1 usually none.
    Facial2,
    /// Facial hair group 3 / moustache / alt sideburns (`03**`). Style 1 usually none.
    Facial3,
    /// Gloves (`04**`) – hand coverings, 1..=4 styles.
    Gloves,
    /// Boots / footwear (`05**`) – 1..=5 styles (shape / height).
    Boots,
    /// Shirt tail (race / gender specific lower garment extension) (`06**`). Rare / optional.
    ShirtTail,
    /// Ears (`07**`) – style 1 none / hidden, style 2 visible ears (race dependent).
    Ears,
    /// Wristbands / sleeves (`08**`) – 1 none, 2 normal, 3 ruffled (where supported).
    Wristbands,
    /// Legs armor pads / cuffs (`09**`) – 1 none, 2 long, 3 short variations.
    Legs,
    /// Shirt doublet / upper chest overlay (`10**`) – 1 none, 2 obscure/unused variant.
    ShirtDoublet,
    /// Pant doublet / lower garment overlay (`11**`) – styles include skirt / armored.
    PantDoublet,
    /// Tabard (`12**`) – 1 none, 2 tabard mesh.
    Tabard,
    /// Robe trousers split / dress (`13**`) – 1 legs (pants), 2 dress (robe lower).
    Robe,
    /// Loincloth / lower flap accessory (`14**`).
    Loincloth,
    /// Cloaks / capes (`15**`) – multiple silhouette styles (1..=10 common).
    Cape,
    /// Facial jewelry / adornments (`16**`) – nose rings, earrings, chin pieces, etc.
    FacialJewelry,
    /// Eye glow / special eye effects (`17**`) – 1 none, 2 primary glow, 3 alternate glow.
    EyeEffects,
    /// Belt / belt pack (`18**`) – includes bulky / monk specific variations.
    Belt,
    /// Skin extras: bones / tail / additional appendages (`19**`). Implementation dependent.
    SkinBoneTail,
    /// Toes / feet detail (`20**`) – 1 none, 2 feet (race dependent visibility control).
    Toes,
    /// Skull (additional overlay / effect) (`21**`). Rare usage.
    Skull,
    /// Torso armored overlay (`22**`) – 1 regular, 2 armored chest plating.
    Torso,
    /// Hands attachments (special hand overlays / alternative meshes) (`23**`).
    HandsAttachments,
    /// Head attachments (horns, antlers, crests, etc.) (`24**`).
    HeadAttachments,
    /// Facewear (blindfolds, runes, etc.) (`25**`).
    Facewear,
    /// Shoulders effect / attachment geosets (`26**`).
    Shoulders,
    /// Helm (object component models / extra helm geometry) (`27**`).
    Helm,
    /// Upper arm overlays / attachments (`28**`).
    ArmUpper,
    /// Unknown / not yet classified category.
    Unknown,
}

impl GeosetType {
    /// Classify from a geoset (e.g. 1507 for a cloak style). Falls back to Unknown.
    pub fn from_geoset(geoset: Geoset) -> Self {
        // High-order two digits (base 10) normally define the category, except 0000.
        if geoset.id == 0 {
            return GeosetType::SkinBase;
        }
        let group = geoset.id / 100; // e.g. 15 for 1507
        match group {
            0 => GeosetType::Hair, // 00** excluding 0000
            1 => GeosetType::Facial1,
            2 => GeosetType::Facial2,
            3 => GeosetType::Facial3,
            4 => GeosetType::Gloves,
            5 => GeosetType::Boots,
            6 => GeosetType::ShirtTail,
            7 => GeosetType::Ears,
            8 => GeosetType::Wristbands,
            9 => GeosetType::Legs,
            10 => GeosetType::ShirtDoublet,
            11 => GeosetType::PantDoublet,
            12 => GeosetType::Tabard,
            13 => GeosetType::Robe,
            14 => GeosetType::Loincloth,
            15 => GeosetType::Cape,
            16 => GeosetType::FacialJewelry,
            17 => GeosetType::EyeEffects,
            18 => GeosetType::Belt,
            19 => GeosetType::SkinBoneTail,
            20 => GeosetType::Toes,
            21 => GeosetType::Skull,
            22 => GeosetType::Torso,
            23 => GeosetType::HandsAttachments,
            24 => GeosetType::HeadAttachments,
            25 => GeosetType::Facewear,
            26 => GeosetType::Shoulders,
            27 => GeosetType::Helm,
            28 => GeosetType::ArmUpper,
            _ => GeosetType::Unknown,
        }
    }
}
