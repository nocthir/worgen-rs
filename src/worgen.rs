// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
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
}

fn load_m2(
    mut event_reader: EventReader<ModelSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    if let Some(event) = event_reader.read().next() {
        let mpq_path = &event.archive_path;
        info!("Reading MPQ: {}", mpq_path.display());
        let mut archive = mpq::Archive::open(mpq_path)?;
        let mesh = read_m2(event.model_path.to_str().unwrap(), &mut archive)?;
        if let Some(mesh) = mesh {
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
        } else {
            error!("Failed to read model: {}", event.model_path.display());
        }
    }
    Ok(())
}

fn load_mpqs(mut exit: EventWriter<AppExit>, commands: Commands) {
    if let Err(err) = load_mpqs_impl(commands) {
        error!("Error loading MPQs: {err}");
        exit.write(AppExit::error());
    }
}

fn load_mpqs_impl(mut commands: Commands) -> Result<()> {
    let mut data_info = DataInfo::default();

    let client_path =
        PathBuf::from(std::env::var("CLIENT_PATH").unwrap_or_else(|_| ".".to_string()));
    let data_path = client_path.join("Data");

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
        if entry.name.ends_with(".m2") {
            let info = ModelInfo {
                path: PathBuf::from(&entry.name),
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

fn read_m2<P: AsRef<Path>>(path: P, archive: &mut mpq::Archive) -> Result<Option<Mesh>> {
    let file = archive.read_file(path.as_ref().to_str().unwrap())?;
    let mut reader = io::Cursor::new(&file);
    if let Ok(m2) = m2::M2Model::parse(&mut reader)
        && !m2.vertices.is_empty()
    {
        info!("{}: {:?}", path.as_ref().display(), m2.header.version());
        info!("  Vertices: {}", m2.vertices.len());
        info!("  Bones: {}", m2.bones.len());
        return Ok(Some(create_mesh(m2, &file)?));
    }
    Ok(None)
}

fn create_mesh(m2: m2::M2Model, m2_data: &[u8]) -> Result<Mesh> {
    let skin = m2.parse_embedded_skin(m2_data, 0)?;
    // These are used to index into the global vertices array.
    let submesh = &skin.submeshes()[0];
    let indices = skin.get_resolved_indices();
    // This is used to index into the local vertices array.
    let triangles = skin
        .triangles()
        .iter()
        .copied()
        .skip(submesh.triangle_start as usize)
        .take(submesh.triangle_count as usize)
        .collect();

    // Local vertex positions and normals arrays
    let positions: Vec<_> = indices
        .iter()
        .copied()
        .map(|i| {
            let v = &m2.vertices[i as usize];
            [v.position.x, v.position.y, v.position.z]
        })
        .collect();
    let normals: Vec<_> = indices
        .iter()
        .copied()
        .map(|i| {
            let v = &m2.vertices[i as usize];
            normalize_vec3([v.normal.x, v.normal.y, v.normal.z])
        })
        .collect();
    let tex_coords_0: Vec<_> = indices
        .iter()
        .copied()
        .map(|i| {
            let v = &m2.vertices[i as usize];
            [v.tex_coords.x, v.tex_coords.y]
        })
        .collect();

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
    .with_inserted_indices(Indices::U16(triangles));

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

    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    commands.spawn((
        CurrentModel,
        Mesh3d(mesh_handle),
        MeshMaterial3d(material.clone()),
        transform,
    ));
}
