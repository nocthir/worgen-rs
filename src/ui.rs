// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::*;

use crate::{
    data::{ArchivesInfo, archive::ArchiveInfo},
    settings::{FileSettings, Settings},
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FileSelected>()
            .insert_resource(ArchivesInfo::default())
            .add_systems(EguiPrimaryContextPass, data_info)
            .add_systems(
                Startup,
                select_main_menu_model.run_if(resource_exists::<Settings>),
            );
    }
}

#[derive(Event)]
pub struct FileSelected {
    pub archive_path: PathBuf,
    pub file_path: String,
}

impl From<&FileSettings> for FileSelected {
    fn from(settings: &FileSettings) -> Self {
        Self {
            archive_path: settings.archive_path.clone().into(),
            file_path: settings.file_path.clone(),
        }
    }
}

fn select_main_menu_model(mut event_writer: EventWriter<FileSelected>, settings: Res<Settings>) {
    event_writer.write(FileSelected::from(&settings.default_model));
}

fn data_info(
    mut contexts: EguiContexts,
    mut data_info: ResMut<ArchivesInfo>,
    mut event_writer: EventWriter<FileSelected>,
) -> Result<()> {
    egui::Window::new("Info")
        .scroll([false, true])
        .show(contexts.ctx_mut()?, |ui| {
            for archive in &mut data_info.archives {
                archive_info(archive, ui, &mut event_writer);
            }
        });
    Ok(())
}

fn archive_info(
    archive: &mut ArchiveInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let texture_paths = &archive.texture_paths;
    let model_paths = &archive.model_paths;
    let world_model_paths = &archive.world_model_paths;
    let world_map_paths = &archive.world_map_paths;

    egui::CollapsingHeader::new(format!("{}", archive.path.display()))
        .default_open(false)
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Textures")
                .enabled(!texture_paths.is_empty())
                .show(ui, |ui| {
                    for path in texture_paths {
                        ui.label(path);
                    }
                });
            egui::CollapsingHeader::new("Models")
                .enabled(!model_paths.is_empty())
                .show(ui, |ui| {
                    for path in model_paths {
                        model_info(archive, path, ui, event_writer);
                    }
                });
            egui::CollapsingHeader::new("World Models")
                .enabled(!world_model_paths.is_empty())
                .show(ui, |ui| {
                    for path in world_model_paths {
                        world_model_info(archive, path, ui, event_writer);
                    }
                });
            egui::CollapsingHeader::new("World Maps")
                .enabled(!world_map_paths.is_empty())
                .show(ui, |ui| {
                    for path in world_map_paths {
                        world_map_info(archive, path, ui, event_writer);
                    }
                });
        });
}

fn model_info(
    archive: &ArchiveInfo,
    path: &str,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = egui::CollapsingHeader::new(path).show(ui, |_| {});
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            archive_path: archive.path.clone(),
            file_path: path.to_owned(),
        });
    }
}

fn world_model_info(
    archive: &ArchiveInfo,
    world_model_path: &str,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = egui::CollapsingHeader::new(world_model_path).show(ui, |_| {});
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            archive_path: archive.path.clone(),
            file_path: world_model_path.to_owned(),
        });
    }
}

fn world_map_info(
    archive: &ArchiveInfo,
    world_map_path: &str,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = egui::CollapsingHeader::new(world_map_path).show(ui, |_| {});
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            archive_path: archive.path.clone(),
            file_path: world_map_path.to_owned(),
        });
    }
}
