// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::*, prelude::*, render::mesh::*};
use wow_m2 as m2;

use crate::data::{bundle::*, file, model::*};

pub fn create_meshes_from_model_path(
    model_path: &str,
    file_info_map: &file::FileInfoMap,
    scene_assets: &mut SceneAssets,
) -> Result<Vec<ModelBundle>> {
    let model_info = file_info_map.get_model_info(model_path)?;
    create_meshes_from_model_info(model_info, file_info_map, scene_assets)
}

pub fn create_meshes_from_model_info(
    model_info: &ModelInfo,
    file_info_map: &file::FileInfoMap,
    scene_assets: &mut SceneAssets,
) -> Result<Vec<ModelBundle>> {
    let mut ret = Vec::default();

    if !model_info.model.vertices.is_empty() {
        let image_handles =
            create_textures_from_model(&model_info.model, file_info_map, &mut scene_assets.images)?;
        let res = create_mesh_from_model(
            &model_info.model,
            &model_info.data,
            &image_handles,
            &mut scene_assets.materials,
            &mut scene_assets.meshes,
        )?;
        ret.extend(res);
    }

    Ok(ret)
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

fn create_mesh_from_model(
    model: &m2::M2Model,
    model_data: &[u8],
    image_handles: &[Handle<Image>],
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let skin = model.parse_embedded_skin(model_data, 0)?;

    let vertex_count = model.vertices.len();
    let mut vertex_attributes = VertexAttributes::with_capacity(vertex_count);
    for vertex in model.vertices.iter() {
        vertex_attributes
            .positions
            .push([vertex.position.x, vertex.position.y, vertex.position.z]);
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
        ret.push(create_mesh_from_model_submesh(
            model,
            &skin,
            batch_index,
            &vertex_attributes,
            image_handles,
            materials,
            meshes,
        )?);
    }
    Ok(ret)
}

fn create_mesh_from_model_submesh(
    model: &m2::M2Model,
    skin: &m2::skin::SkinFile,
    batch_index: usize,
    vertex_attributes: &VertexAttributes,
    image_handles: &[Handle<Image>],
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<ModelBundle> {
    let batch = &skin.batches()[batch_index];
    let submesh = &skin.submeshes()[batch.skin_section_index as usize];
    let texture_index =
        model.raw_data.texture_lookup_table[batch.texture_combo_index as usize] as usize;
    let texture = &image_handles[texture_index];

    // Determine alpha mode from material blend mode.
    // Note that multiple batches can share the same material.
    let model_material = &model.materials[batch.material_index as usize];
    let alpha_mode = bundle::alpha_mode_from_model_blend_mode(model_material.blend_mode);

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(texture.clone()),
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

#[cfg(test)]
mod test {
    use bevy::pbr::ExtendedMaterial;

    use super::*;
    use crate::{data::archive, data::bundle, material::TerrainMaterial, *};

    #[test]
    fn list_model_paths() -> Result {
        settings::Settings::init();
        archive::ArchiveMap::init();
        let archive_paths = archive::ArchiveMap::get().get_archive_paths();
        println!("Path, Archive");
        for archive_path in archive_paths {
            let mut archive = archive::get_archive!(archive_path)?;
            for file_path in archive.list()? {
                if model::is_model_extension(&file_path.name) {
                    println!("{}, {}", file_path.name, archive.path().unwrap().display());
                }
            }
        }
        Ok(())
    }

    #[test]
    fn load_main_menu() -> Result {
        settings::Settings::init();
        archive::ArchiveMap::init();
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.default_model.file_path)?;
        let mut app = App::new();
        app.world_mut().init_resource::<Assets<Image>>();
        app.world_mut().init_resource::<Assets<Mesh>>();
        app.world_mut()
            .init_resource::<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>();
        app.world_mut().init_resource::<Assets<StandardMaterial>>();

        use bevy::ecs::system::SystemState;
        let mut state: SystemState<file::SceneAssets> = SystemState::new(app.world_mut());
        let mut scene_assets = state.get_mut(app.world_mut());

        let _ = bundle::create_mesh_from_file_path(
            &settings.default_model.file_path,
            &file_info_map,
            &mut scene_assets,
        )?;
        Ok(())
    }

    #[test]
    fn load_city() -> Result {
        settings::Settings::init();
        archive::ArchiveMap::init();
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.city_model.file_path)?;
        let mut app = App::new();
        app.world_mut().init_resource::<Assets<Image>>();
        app.world_mut().init_resource::<Assets<Mesh>>();
        app.world_mut()
            .init_resource::<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>();
        app.world_mut().init_resource::<Assets<StandardMaterial>>();

        use bevy::ecs::system::SystemState;
        let mut state: SystemState<file::SceneAssets> = SystemState::new(app.world_mut());
        let mut scene_assets = state.get_mut(app.world_mut());

        let _ = bundle::create_mesh_from_file_path(
            &settings.city_model.file_path,
            &file_info_map,
            &mut scene_assets,
        )?;
        Ok(())
    }

    #[test]
    fn load_test_model() -> Result {
        let settings = settings::TestSettings::load()?;
        settings::Settings::init();
        archive::ArchiveMap::init();
        file::FileArchiveMap::init();
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.test_model_path)?;
        let mut app = App::new();
        app.world_mut().init_resource::<Assets<Image>>();
        app.world_mut().init_resource::<Assets<Mesh>>();
        app.world_mut()
            .init_resource::<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>();
        app.world_mut().init_resource::<Assets<StandardMaterial>>();

        use bevy::ecs::system::SystemState;
        let mut state: SystemState<file::SceneAssets> = SystemState::new(app.world_mut());
        let mut scene_assets = state.get_mut(app.world_mut());

        let _ = bundle::create_mesh_from_file_path(
            &settings.test_model_path,
            &file_info_map,
            &mut scene_assets,
        )?;
        Ok(())
    }
}
