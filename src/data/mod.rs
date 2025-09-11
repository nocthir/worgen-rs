// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod model;
pub mod world_model;

use std::{f32, ffi::OsStr, io, path::Path};

use bevy::prelude::*;

use wow_adt as adt;
use wow_mpq as mpq;

use crate::{
    data::archive::{ArchiveInfo, ArchiveLoaded, LoadArchiveTasks},
    material::CustomMaterial,
    ui::ModelSelected,
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ArchiveLoaded>()
            .add_systems(Startup, archive::start_loading)
            .add_systems(
                Update,
                archive::check_archive_loading.run_if(resource_exists::<LoadArchiveTasks>),
            )
            .add_systems(Update, load_selected_model);
    }
}

#[derive(Component)]
pub struct CurrentModel;

#[derive(Default, Resource)]
pub struct DataInfo {
    pub archives: Vec<ArchiveInfo>,
}
fn load_selected_model(
    mut event_reader: EventReader<ModelSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_materials: ResMut<Assets<CustomMaterial>>,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        match create_mesh_from_selected_model(event, &mut custom_materials, &mut meshes) {
            Ok(bundles) => {
                if bundles.is_empty() {
                    error!("No meshes loaded for model: {}", event.model_path.display());
                    return Ok(());
                }

                // Remove the previous model
                query.into_iter().for_each(|entity| {
                    commands.entity(entity).despawn();
                });

                for (mesh, material) in bundles {
                    add_bundle(&mut commands, mesh, material);
                }

                info!("Loaded model from {}", event.model_path.display());
            }
            Err(err) => {
                error!(
                    "Error loading model {} from archive {}: {err}",
                    event.model_path.display(),
                    event.archive_path.display()
                );
            }
        }
    }
    Ok(())
}

fn create_mesh_from_selected_model(
    model_info: &ModelSelected,
    materials: &mut Assets<CustomMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<CustomMaterial>)>> {
    let mpq_path = &model_info.archive_path;
    let mut archive = mpq::Archive::open(mpq_path)?;
    info!("Loaded archive {}", mpq_path.display());
    let model_path = model_info.model_path.to_str().unwrap();
    create_mesh_from_path_archive(model_path, &mut archive, materials, meshes)
}

fn _read_adt(path: &str, archive: &mut mpq::Archive) -> Result<()> {
    let file = archive.read_file(path)?;
    if file.is_empty() {
        // Skip this
        return Ok(());
    }
    let mut reader = io::Cursor::new(file);
    let adt = adt::Adt::from_reader(&mut reader)?;
    if let Some(modf) = adt.modf
        && !modf.models.is_empty()
    {
        info!("{}: {} MOPR entries", path, modf.models.len());
        for model in &modf.models {
            if let Some(mwmo) = &adt.mwmo {
                let model_name = &mwmo.filenames[model.name_id as usize];
                info!("    - WMO: {model_name}");
            }
        }
    }
    Ok(())
}

fn create_mesh_from_path_archive<P: AsRef<Path>>(
    path: P,
    archive: &mut mpq::Archive,
    materials: &mut Assets<CustomMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<CustomMaterial>)>> {
    let ext = path
        .as_ref()
        .extension()
        .ok_or_else(|| format!("Model path has no extension: {}", path.as_ref().display()))?;

    if ext == OsStr::new("m2") {
        model::create_meshes_from_m2_path(archive, path, materials, meshes)
    } else if ext == OsStr::new("wmo") {
        world_model::create_meshes_from_wmo_path(archive, path, materials, meshes)
    } else {
        Err(format!("Unsupported model format: {}", path.as_ref().display()).into())
    }
}

fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

fn add_bundle(commands: &mut Commands, mesh: Handle<Mesh>, material: Handle<CustomMaterial>) {
    let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
    transform.rotate_local_x(-f32::consts::FRAC_PI_2);
    transform.rotate_local_z(-f32::consts::FRAC_PI_2);

    commands.spawn((
        CurrentModel,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
    ));
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn main_menu() -> Result {
        let settings = settings::load_settings()?;
        let selected_model = ModelSelected::from(&settings.default_model);
        let mut custom_materials = Assets::<CustomMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        create_mesh_from_selected_model(&selected_model, &mut custom_materials, &mut meshes)?;
        Ok(())
    }

    #[test]
    fn dwarf() -> Result {
        env_logger::init();
        let model = settings::load_settings()?;
        let selected_model = ModelSelected::from(&model.test_model);
        let mut custom_materials = Assets::<CustomMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        create_mesh_from_selected_model(&selected_model, &mut custom_materials, &mut meshes)?;
        Ok(())
    }

    #[test]
    fn altar() -> Result {
        env_logger::init();
        let model = settings::load_settings()?;
        let selected_model = ModelSelected::from(&model.test_world_model);
        let mut custom_materials = Assets::<CustomMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        create_mesh_from_selected_model(&selected_model, &mut custom_materials, &mut meshes)?;
        Ok(())
    }
}
