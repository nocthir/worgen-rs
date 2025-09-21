// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{prelude::*, tasks};

use wow_mpq as mpq;
use wow_wmo as wmo;

use crate::data::file;

pub struct WorldModelInfo {
    pub world_model: wmo::WmoRoot,
    pub groups: Vec<wmo::WmoGroup>,
}

impl WorldModelInfo {
    pub fn new<P: AsRef<Path>>(file_path: &str, archive_path: P) -> Result<Self> {
        let mut archive = mpq::Archive::open(archive_path)?;
        let world_model = read_wmo(file_path, &mut archive)?;
        let groups = read_groups(file_path, &mut archive, &world_model)?;
        Ok(Self {
            world_model,
            groups,
        })
    }

    pub fn get_texture_paths(&self) -> &[String] {
        &self.world_model.textures
    }
}

fn read_wmo(path: &str, archive: &mut mpq::Archive) -> Result<wmo::WmoRoot> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(wmo::parse_wmo(&mut reader)?)
}

pub fn is_world_model_root_path(file_path: &str) -> bool {
    if !is_world_model_extension(file_path) {
        return false;
    }
    !is_world_model_group_path(file_path)
}

fn is_world_model_group_path(file_path: &str) -> bool {
    if !is_world_model_extension(file_path) {
        return false;
    }
    let file_path = Path::new(file_path);
    let file_stem = file_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    file_stem
        .split('_')
        .next_back()
        .is_some_and(|s| s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()))
}

pub fn is_world_model_extension(file_path: &str) -> bool {
    let lower = file_path.to_lowercase();
    lower.ends_with(".wmo")
}

fn read_groups(
    file_path: &str,
    archive: &mut mpq::Archive,
    wmo: &wmo::WmoRoot,
) -> Result<Vec<wmo::WmoGroup>> {
    let mut groups = Vec::new();
    for (group_index, _) in wmo.groups.iter().enumerate() {
        let wmo_group = read_group(file_path, archive, group_index)?;
        groups.push(wmo_group);
    }
    Ok(groups)
}

pub fn loading_world_model_task(task: file::LoadFileTask) -> tasks::Task<file::LoadFileTask> {
    info!("Starting to load world model: {}", task.file.path);
    tasks::IoTaskPool::get().spawn(load_world_model(task))
}

async fn load_world_model(mut task: file::LoadFileTask) -> file::LoadFileTask {
    match WorldModelInfo::new(&task.file.path, &task.file.archive_path) {
        Ok(world_model_info) => {
            task.file.set_world_model(world_model_info);
            info!("Loaded world model: {}", task.file.path);
        }
        Err(e) => {
            error!("Failed to load world model {}: {}", task.file.path, e);
            task.file.state = file::FileInfoState::Error(e.to_string());
        }
    }
    task
}

fn read_group(
    file_path: &str,
    archive: &mut mpq::Archive,
    group_index: usize,
) -> Result<wmo::WmoGroup> {
    let group_filename = get_wmo_group_filename(file_path, group_index);
    let file = archive.read_file(&group_filename)?;
    let mut reader = io::Cursor::new(&file);
    Ok(wmo::parse_wmo_group(&mut reader, group_index as _)?)
}

fn get_wmo_group_filename<P: AsRef<Path>>(wmo_path: P, group_index: usize) -> String {
    let base_path = wmo_path.as_ref().with_extension("");
    format!("{}_{:03}.wmo", base_path.display(), group_index)
}
