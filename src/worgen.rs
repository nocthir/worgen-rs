// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    f32, io,
    path::{Path, PathBuf},
};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

use wow_adt as adt;
use wow_m2 as m2;
use wow_mpq as mpq;
use wow_wmo as wmo;

use crate::ui::ModelSelected;

pub struct WorgenPlugin;

impl Plugin for WorgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (start_info, load_mpqs).chain())
            .add_systems(Update, load_m2);
    }
}

fn start_info() {
    info!("Hello, Worgen!");
}

#[derive(Component)]
pub struct CurrentModel;

#[derive(Default, Resource)]
pub struct DataInfo {
    pub archives: Vec<ArchiveInfo>,
}

pub struct ArchiveInfo {
    pub path: PathBuf,
    pub model_infos: Vec<ModelInfo>,
}

impl ArchiveInfo {
    pub fn new<P: AsRef<Path>>(path: P, model_infos: Vec<ModelInfo>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            model_infos,
        }
    }
}

pub struct ModelInfo {
    pub path: PathBuf,
    pub vertex_count: usize,
    pub texture_count: usize,
    pub materials: usize,
}

fn load_m2(
    mut event_reader: EventReader<ModelSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    if let Some(event) = event_reader.read().next() {
        match create_mesh_from_selected_model(event) {
            Ok(loaded_mesh) => {
                if loaded_mesh.is_empty() {
                    error!("No meshes loaded for model: {}", event.model_path.display());
                }
                for mesh in loaded_mesh {
                    query.into_iter().for_each(|entity| {
                        commands.entity(entity).despawn();
                    });
                    add_mesh(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        mesh,
                        &event.model_path,
                    );
                }
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

fn create_mesh_from_selected_model(model_info: &ModelSelected) -> Result<Vec<Mesh>> {
    let mpq_path = &model_info.archive_path;
    info!("Reading MPQ: {}", mpq_path.display());
    let mut archive = mpq::Archive::open(mpq_path)?;
    let model_path = model_info.model_path.to_str().unwrap();
    create_mesh_from_path_archive(model_path, &mut archive)
}

fn load_mpqs(mut exit: EventWriter<AppExit>, commands: Commands) {
    if let Err(err) = load_mpqs_impl(commands) {
        error!("Error loading MPQs: {err}");
        exit.write(AppExit::error());
    }
}

fn load_mpqs_impl(mut commands: Commands) -> Result<()> {
    let mut data_info = DataInfo::default();

    let game_path = PathBuf::from(std::env::var("GAME_PATH").unwrap_or_else(|_| ".".to_string()));
    let data_path = game_path.join("Data");

    for file in data_path.read_dir()? {
        let file = file?;
        if file.file_name().to_string_lossy().ends_with(".MPQ") {
            let mpq_path = file.path();
            info!("Reading MPQ: {}", mpq_path.display());
            let mut archive = mpq::Archive::open(&mpq_path)?;
            let model_infos = read_m2s(&mut archive)?;
            let archive_info = ArchiveInfo::new(mpq_path, model_infos);
            data_info.archives.push(archive_info);
        }
    }

    commands.insert_resource(data_info);

    Ok(())
}

fn read_m2s(archive: &mut mpq::Archive) -> Result<Vec<ModelInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        if entry.name.ends_with(".m2")
            && let Ok(model) = read_m2(&entry.name, archive)
        {
            let vertex_count = model.vertices.len();
            let texture_count = model.textures.len();
            let materials = model.materials.len();
            let info = ModelInfo {
                path: PathBuf::from(&entry.name),
                vertex_count,
                texture_count,
                materials,
            };
            infos.push(info);
        }
    }

    Ok(infos)
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

fn _read_wmo(path: &str, archive: &mut mpq::Archive) -> Result<()> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    if let Ok(wmo) = wmo::WmoGroupParser::new().parse_group(&mut reader, 0)
        && !wmo.vertices.is_empty()
    {
        info!("{path}: Vertices: {}", wmo.vertices.len());
        info!("{path}: Triangles: {}", wmo.indices.len() / 3);
    }
    Ok(())
}

fn read_m2<P: AsRef<Path>>(path: P, archive: &mut mpq::Archive) -> Result<m2::M2Model> {
    let file_path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format!("Invalid model path: {}", path.as_ref().display()))?;

    let file = archive
        .read_file(file_path)
        .map_err(|e| format!("Failed to read model file {}: {}", file_path, e))?;
    let mut reader = io::Cursor::new(&file);
    let model = m2::M2Model::parse(&mut reader)
        .map_err(|e| format!("Failed to parse model file {}: {}", file_path, e))?;
    Ok(model)
}

fn create_mesh_from_path_archive<P: AsRef<Path>>(
    path: P,
    archive: &mut mpq::Archive,
) -> Result<Vec<Mesh>> {
    let file = archive.read_file(path.as_ref().to_str().unwrap())?;
    let mut reader = io::Cursor::new(&file);
    let mut ret = Vec::default();

    if let Ok(m2) = m2::M2Model::parse(&mut reader)
        && !m2.vertices.is_empty()
    {
        info!("{}: {:?}", path.as_ref().display(), m2.header.version());
        for skin_index in 0..m2.embedded_skin_count().unwrap().min(1) {
            if let Ok(mesh) = create_mesh(&m2, &file, 0) {
                ret.push(mesh);
            } else {
                return Err(format!(
                    "Failed to create mesh for skin index {} in model {} from archive {}",
                    skin_index,
                    path.as_ref().display(),
                    archive.path().display(),
                )
                .into());
            }
        }
    }

    Ok(ret)
}

fn create_mesh(m2: &m2::M2Model, m2_data: &[u8], skin_index: u32) -> Result<Mesh> {
    info!("Loading skin {skin_index}");
    let skin = m2.parse_embedded_skin(m2_data, skin_index as _)?;
    // These are used to index into the global vertices array.
    let indices = skin.get_resolved_indices();
    info!("skin indices: {}", indices.len());
    info!("skin triangles: {}", skin.triangles().len());
    info!("global vertices: {}", m2.vertices.len());

    // Local vertex attributes.
    let positions: Vec<_> = indices
        .iter()
        .copied()
        .filter_map(|i| {
            if i < m2.vertices.len() as u16 {
                let v = &m2.vertices[i as usize];
                Some([v.position.x, v.position.y, v.position.z])
            } else {
                None
            }
        })
        .collect();
    let normals: Vec<_> = indices
        .iter()
        .copied()
        .filter_map(|i| {
            if i < m2.vertices.len() as u16 {
                let v = &m2.vertices[i as usize];
                Some(normalize_vec3([v.normal.x, v.normal.y, v.normal.z]))
            } else {
                None
            }
        })
        .collect();
    let tex_coords_0: Vec<_> = indices
        .iter()
        .copied()
        .filter_map(|i| {
            if i < m2.vertices.len() as u16 {
                let v = &m2.vertices[i as usize];
                Some([v.tex_coords.x, v.tex_coords.y])
            } else {
                None
            }
        })
        .collect();

    // This is used to index into the local vertices array.
    let triangles = skin.triangles();

    let mut mesh_indices = Vec::default();
    for submesh in skin.submeshes() {
        let submesh_triangles = triangles
            .iter()
            .copied()
            .skip(submesh.triangle_start as usize)
            .take(submesh.triangle_count as usize);
        mesh_indices.extend(submesh_triangles);
    }

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
        positions,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords_0)
    .with_inserted_indices(Indices::U16(mesh_indices));

    Ok(mesh)
}

fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

fn add_mesh<P: AsRef<Path>>(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    mesh: Mesh,
    file: P,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::linear_rgb(0.8, 0.7, 0.6),
        perceptual_roughness: 0.9,
        ..default()
    });

    info!("Loaded model from {}", file.as_ref().display());
    let mesh_handle = meshes.add(mesh);

    let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
    transform.rotate_local_x(-f32::consts::FRAC_PI_2);
    transform.rotate_local_z(-f32::consts::FRAC_PI_2);

    commands.spawn((
        CurrentModel,
        Mesh3d(mesh_handle),
        MeshMaterial3d(material.clone()),
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
        create_mesh_from_selected_model(&selected_model)?;
        Ok(())
    }

    #[test]
    fn dwarf() -> Result {
        env_logger::init();
        let model = settings::load_settings()?;
        let selected_model = ModelSelected::from(&model.test_model);
        create_mesh_from_selected_model(&selected_model)?;
        Ok(())
    }
}
