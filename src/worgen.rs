// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    f32,
    ffi::OsStr,
    io,
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
            .add_systems(Update, load_selected_model);
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
    pub wmo_infos: Vec<WmoInfo>,
}

impl ArchiveInfo {
    pub fn new<P: AsRef<Path>>(
        path: P,
        model_infos: Vec<ModelInfo>,
        wmo_infos: Vec<WmoInfo>,
    ) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            model_infos,
            wmo_infos,
        }
    }

    pub fn has_models(&self) -> bool {
        !self.model_infos.is_empty() || !self.wmo_infos.is_empty()
    }
}

pub struct ModelInfo {
    pub path: PathBuf,
    pub vertex_count: usize,
    pub texture_count: usize,
    pub materials: usize,
}

pub struct WmoInfo {
    pub path: PathBuf,
    pub groups: Vec<WmoGroupInfo>,
    pub material_count: usize,
    pub texture_count: usize,
}

pub struct WmoGroupInfo {
    pub name: String,
    pub vertex_count: usize,
    pub index_count: usize,
}

fn load_selected_model(
    mut event_reader: EventReader<ModelSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        match create_mesh_from_selected_model(event) {
            Ok(loaded_mesh) => {
                if loaded_mesh.is_empty() {
                    error!("No meshes loaded for model: {}", event.model_path.display());
                    return Ok(());
                }

                // Remove the previous model
                query.into_iter().for_each(|entity| {
                    commands.entity(entity).despawn();
                });

                for mesh in loaded_mesh {
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
            let mut archive = mpq::Archive::open(&mpq_path)?;
            let model_infos = read_m2s(&mut archive)?;
            let wmo_infos = read_mwos(&mut archive)?;
            let archive_info = ArchiveInfo::new(mpq_path, model_infos, wmo_infos);
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

fn read_mwos(archive: &mut mpq::Archive) -> Result<Vec<WmoInfo>> {
    let mut infos = Vec::new();
    for entry in archive.list()?.iter() {
        let wmo_path = PathBuf::from(&entry.name);

        if is_wmo_root_path(&wmo_path)
            && let Ok(model) = read_wmo(&entry.name, archive)
        {
            let groups = match read_wmo_groups(archive, &entry.name, &model) {
                Ok(groups) => groups,
                Err(err) => {
                    error!("Failed to read WMO groups for {}: {}", entry.name, err);
                    Vec::new()
                }
            };
            let material_count = model.materials.len();
            let texture_count = model.textures.len();
            let info = WmoInfo {
                path: PathBuf::from(&entry.name),
                groups,
                material_count,
                texture_count,
            };
            infos.push(info);
        }
    }

    Ok(infos)
}

fn is_wmo_root_path<P: AsRef<Path>>(path: P) -> bool {
    if path.as_ref().extension() != Some(OsStr::new("wmo")) {
        return false;
    }
    !is_wmo_group_path(path)
}

fn is_wmo_group_path<P: AsRef<Path>>(path: P) -> bool {
    if path.as_ref().extension() != Some(OsStr::new("wmo")) {
        return false;
    }
    let file_stem = path
        .as_ref()
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    file_stem
        .split('_')
        .next_back()
        .is_some_and(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()))
}

fn read_wmo_groups<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    wmo_path: P,
    wmo: &wmo::WmoRoot,
) -> Result<Vec<WmoGroupInfo>> {
    let mut infos = Vec::new();
    for (group_index, wmo_group_info) in wmo.groups.iter().enumerate() {
        let wmo_group = read_wmo_group(archive, wmo_path.as_ref(), group_index)?;
        let name = wmo_group_info.name.clone();
        let vertex_count = wmo_group.vertices.len();
        let index_count = wmo_group.indices.len();
        let info = WmoGroupInfo {
            name,
            vertex_count,
            index_count,
        };
        infos.push(info);
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

fn read_wmo(path: &str, archive: &mut mpq::Archive) -> Result<wmo::WmoRoot> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(wmo::parse_wmo(&mut reader)?)
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
    let ext = path
        .as_ref()
        .extension()
        .ok_or_else(|| format!("Model path has no extension: {}", path.as_ref().display()))?;

    if ext == OsStr::new("m2") {
        create_meshes_from_m2_path(archive, path)
    } else if ext == OsStr::new("wmo") {
        create_meshes_from_wmo_path(archive, path)
    } else {
        Err(format!("Unsupported model format: {}", path.as_ref().display()).into())
    }
}

fn create_meshes_from_m2_path<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    path: P,
) -> Result<Vec<Mesh>> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format!("Invalid model path: {}", path.as_ref().display()))?;

    let file = archive.read_file(path_str)?;
    let mut reader = io::Cursor::new(&file);

    let mut ret = Vec::default();

    if let Ok(m2) = m2::M2Model::parse(&mut reader)
        && !m2.vertices.is_empty()
    {
        for skin_index in 0..m2.embedded_skin_count().unwrap().min(1) {
            if let Ok(mesh) = create_mesh(&m2, &file, 0) {
                ret.push(mesh);
            } else {
                return Err(format!(
                    "Failed to create mesh for skin index {} in model {} from archive {}",
                    skin_index,
                    path_str,
                    archive.path().display(),
                )
                .into());
            }
        }
    }

    Ok(ret)
}

fn create_mesh(m2: &m2::M2Model, m2_data: &[u8], skin_index: u32) -> Result<Mesh> {
    let skin = m2.parse_embedded_skin(m2_data, skin_index as _)?;
    // These are used to index into the global vertices array.
    let indices = skin.get_resolved_indices();

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

fn create_meshes_from_wmo_path<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    path: P,
) -> Result<Vec<Mesh>> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| format!("Invalid world model path: {}", path.as_ref().display()))?;

    let file = archive.read_file(path_str)?;
    let mut reader = io::Cursor::new(&file);

    let mut ret = Vec::default();

    if let Ok(wmo) = wmo::parse_wmo(&mut reader)
        && !wmo.groups.is_empty()
    {
        for group_index in 0..wmo.groups.len() {
            if let Ok(mesh) = create_mesh_from_wmo_group_path(archive, path_str, group_index) {
                ret.push(mesh);
            } else {
                error!("Failed to create mesh for group {group_index}");
            }
        }
    }

    Ok(ret)
}

fn create_mesh_from_wmo_group_path<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    wmo_path: P,
    group_index: usize,
) -> Result<Mesh> {
    let wmo_group = read_wmo_group(archive, wmo_path, group_index)?;
    create_mesh_from_wmo_group(&wmo_group)
}

fn read_wmo_group<P: AsRef<Path>>(
    archive: &mut mpq::Archive,
    wmo_path: P,
    group_index: usize,
) -> Result<wmo::WmoGroup> {
    let group_filename = get_wmo_group_filename(&wmo_path, group_index);
    info!("Loading WMO group from {}", group_filename);
    let file = archive
        .read_file(&group_filename)
        .map_err(|e| format!("Failed to read WMO group file {}: {}", group_filename, e))?;
    let mut reader = io::Cursor::new(&file);
    let wmo_group = wmo::parse_wmo_group(&mut reader, group_index as _)
        .map_err(|e| format!("Failed to parse WMO group file {}: {}", group_filename, e))?;
    Ok(wmo_group)
}

fn get_wmo_group_filename<P: AsRef<Path>>(wmo_path: P, group_index: usize) -> String {
    let base_path = wmo_path.as_ref().with_extension("");
    format!("{}_{:03}.wmo", base_path.display(), group_index)
}

fn create_mesh_from_wmo_group(wmo: &wmo::WmoGroup) -> Result<Mesh> {
    let positions: Vec<_> = wmo.vertices.iter().map(|v| [v.x, v.y, v.z]).collect();
    let normals: Vec<_> = wmo
        .normals
        .iter()
        .map(|v| normalize_vec3([v.x, v.y, v.z]))
        .collect();
    let tex_coords_0: Vec<_> = wmo.tex_coords.iter().map(|v| [v.u, v.v]).collect();
    let mut colors = Vec::new();
    if let Some(vertex_colors) = &wmo.vertex_colors {
        colors = vertex_colors
            .iter()
            .map(|v| [v.r as f32, v.g as f32, v.b as f32, v.a as f32])
            .collect();
    }

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
        positions,
    )
    .with_inserted_indices(Indices::U16(wmo.indices.clone()));

    if !normals.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    if !tex_coords_0.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords_0);
    }
    if !colors.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

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

    #[test]
    fn altar() -> Result {
        env_logger::init();
        let model = settings::load_settings()?;
        let selected_model = ModelSelected::from(&model.test_world_model);
        create_mesh_from_selected_model(&selected_model)?;
        Ok(())
    }
}
