// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::PathBuf};

use bevy::prelude::*;

use wow_adt as adt;
use wow_m2 as m2;
use wow_mpq as mpq;
use wow_wmo as wmo;

pub struct WorgenPlugin;

impl Plugin for WorgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (start_info, load_mpqs).chain());
    }
}

fn start_info() {
    info!("Hello, Worgen!");
}

fn load_mpqs(mut exit: EventWriter<AppExit>) {
    if let Err(err) = load_mpqs_impl() {
        error!("Error loading MPQs: {err}");
        exit.write(AppExit::error());
    }
}

fn load_mpqs_impl() -> Result<()> {
    let game_path = PathBuf::from(std::env::var("GAME_PATH").unwrap_or_else(|_| ".".to_string()));
    let data_path = game_path.join("Data");

    for file in data_path.read_dir()? {
        let file = file?;
        if file.file_name().to_string_lossy().ends_with(".MPQ") {
            read_mpq(&file.path())?;
        }
    }
    Ok(())
}

fn read_mpq(mpq_path: &PathBuf) -> Result<()> {
    info!("Reading MPQ: {}", mpq_path.display());
    let mut archive = mpq::Archive::open(mpq_path)?;

    for entry in archive.list()? {
        if entry.name.ends_with(".adt") {
            read_adt(&entry.name, &mut archive)?;
        }
        if entry.name.ends_with(".wmo") {
            read_wmo(&entry.name, &mut archive)?;
        }
        if entry.name.ends_with(".m2") {
            read_m2(&entry.name, &mut archive)?;
        }
    }

    Ok(())
}

fn read_adt(path: &str, archive: &mut mpq::Archive) -> Result<()> {
    let file = archive.read_file(path)?;
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

                //read_wmo(model_name, archive)?;
            }
        }
    }
    Ok(())
}

fn read_wmo(path: &str, archive: &mut mpq::Archive) -> Result<()> {
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

fn read_m2(path: &str, archive: &mut mpq::Archive) -> Result<()> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    if let Ok(m2) = m2::M2Model::parse(&mut reader)
        && !m2.vertices.is_empty()
    {
        info!("{path}: {:?}", m2.header.version());
        info!("  Vertices: {}", m2.vertices.len());
        info!("  Bones: {}", m2.bones.len());
    }
    Ok(())
}
