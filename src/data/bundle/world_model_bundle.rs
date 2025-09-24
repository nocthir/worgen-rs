// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;

use wow_wmo as wmo;

use crate::data::{bundle::*, file, world_model::WorldModelInfo};

pub fn create_meshes_from_world_model_path(
    world_model_path: &str,
    file_info_map: &file::FileInfoMap,
    scene_assets: &mut SceneAssets,
) -> Result<Vec<ModelBundle>> {
    let world_model_info = file_info_map.get_world_model_info(world_model_path)?;
    create_meshes_from_world_model_info(world_model_info, file_info_map, scene_assets)
}

pub fn create_meshes_from_world_model_info(
    world_model: &WorldModelInfo,
    file_info_map: &file::FileInfoMap,
    scene_assets: &mut SceneAssets,
) -> Result<Vec<ModelBundle>> {
    let mut ret = Vec::default();

    let textures = bundle::create_textures_from_world_model(
        world_model,
        file_info_map,
        &mut scene_assets.images,
    )?;
    let materials =
        bundle::create_materials_from_world_model(world_model, &textures, &mut scene_assets.images);
    let material_handles = materials
        .into_iter()
        .map(|mat| scene_assets.materials.add(mat))
        .collect::<Vec<_>>();

    let default_material_handle = scene_assets.materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        unlit: true,
        ..Default::default()
    });

    for group_index in 0..world_model.groups.len() {
        let bundles = create_mesh_from_wmo_group(
            &world_model.groups[group_index],
            default_material_handle.clone(),
            &material_handles,
            &mut scene_assets.meshes,
        );
        ret.extend(bundles);
    }

    Ok(ret)
}

fn create_mesh_from_wmo_group(
    wmo: &wmo::group_parser::WmoGroup,
    default_material_handle: Handle<StandardMaterial>,
    material_handles: &[Handle<StandardMaterial>],
    meshes: &mut Assets<Mesh>,
) -> Vec<ModelBundle> {
    let positions: Vec<_> = wmo
        .vertex_positions
        .iter()
        .map(|v| [v.x, v.y, v.z])
        .collect();
    let normals: Vec<_> = wmo
        .vertex_normals
        .iter()
        .map(|v| normalize_vec3([v.x, v.y, v.z]))
        .collect();
    let tex_coords_0: Vec<_> = wmo.texture_coords.iter().map(|v| [v.u, v.v]).collect();
    let colors: Vec<_> = wmo
        .vertex_colors
        .iter()
        .map(|v| [v.r as f32, v.g as f32, v.b as f32, v.a as f32])
        .collect();

    let mut ret = Vec::new();

    for batch in &wmo.render_batches {
        let indices = wmo
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

        let mesh_handle = meshes.add(mesh);

        let material_index = batch.material_id as usize;
        let material_handle = if material_index < material_handles.len() {
            material_handles[material_index].clone()
        } else {
            default_material_handle.clone()
        };

        ret.push(ModelBundle {
            mesh: Mesh3d(mesh_handle),
            material: MeshMaterial3d(material_handle),
            transform: Transform::default(),
        });
    }

    ret
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{data::archive, data::bundle, *};

    #[test]
    fn list_world_model_paths() -> Result {
        settings::Settings::init();
        archive::ArchiveMap::init();
        let settings = settings::TestSettings::load()?;
        let file_info_map = file::test::default_file_info_map(&settings)?;
        println!("Path, Archive");
        for file_info in file_info_map.get_file_infos() {
            if world_model::is_world_model_extension(&file_info.path) {}
        }
        Ok(())
    }

    #[test]
    fn load_world_model() -> Result {
        settings::Settings::init();
        archive::ArchiveMap::init();
        let settings = settings::TestSettings::load()?;
        let mut file_info_map = file::test::default_file_info_map(&settings)?;
        file_info_map.load_file_and_dependencies(&settings.test_world_model.file_path)?;
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
            &settings.test_world_model.file_path,
            &file_info_map,
            &mut scene_assets,
        )?;
        Ok(())
    }
}
