// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::*;

use crate::{
    settings::{ModelSettings, Settings},
    state::GameState,
    worgen::{ArchiveInfo, DataInfo, ModelInfo, WmoGroupInfo, WmoInfo},
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ModelSelected>()
            .add_systems(
                OnEnter(GameState::Main),
                select_main_menu_model.run_if(resource_exists::<Settings>),
            )
            .add_systems(
                EguiPrimaryContextPass,
                data_info.run_if(resource_exists::<DataInfo>),
            );
    }
}

#[derive(Event)]
pub struct ModelSelected {
    pub archive_path: PathBuf,
    pub model_path: PathBuf,
}

impl From<&ModelSettings> for ModelSelected {
    fn from(settings: &ModelSettings) -> Self {
        Self {
            archive_path: PathBuf::from(&settings.archive_path),
            model_path: PathBuf::from(&settings.model_path),
        }
    }
}

fn select_main_menu_model(mut event_writer: EventWriter<ModelSelected>, settings: Res<Settings>) {
    event_writer.write(ModelSelected::from(&settings.default_model));
}

fn data_info(
    mut contexts: EguiContexts,
    data_info: Res<DataInfo>,
    mut event_writer: EventWriter<ModelSelected>,
) -> Result<()> {
    egui::Window::new("Info")
        .scroll([false, true])
        .show(contexts.ctx_mut()?, |ui| {
            for archive in &data_info.archives {
                archive_info(archive, ui, &mut event_writer);
            }
        });
    Ok(())
}

fn archive_info(
    archive: &ArchiveInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<ModelSelected>,
) {
    egui::CollapsingHeader::new(format!("{}", archive.path.display()))
        .default_open(false)
        .enabled(archive.has_models())
        .show(ui, |ui| {
            egui::CollapsingHeader::new("M2")
                .enabled(!archive.model_infos.is_empty())
                .show(ui, |ui| {
                    for model in &archive.model_infos {
                        model_info(archive, model, ui, event_writer);
                    }
                });
            egui::CollapsingHeader::new("WMO")
                .enabled(!archive.wmo_infos.is_empty())
                .show(ui, |ui| {
                    for wmo in &archive.wmo_infos {
                        wmo_info(archive, wmo, ui, event_writer);
                    }
                });
        });
}

fn model_info(
    archive: &ArchiveInfo,
    model: &ModelInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<ModelSelected>,
) {
    let header = egui::CollapsingHeader::new(format!("{}", model.path.display()))
        .enabled(model.vertex_count > 0)
        .show(ui, |ui| {
            ui.label(format!("Vertices: {}", model.vertex_count));
            ui.label(format!("Textures: {}", model.texture_count));
            ui.label(format!("Materials: {}", model.materials));
        });
    if header.header_response.clicked() {
        event_writer.write(ModelSelected {
            archive_path: archive.path.clone(),
            model_path: model.path.clone(),
        });
    }
}

fn wmo_info(
    archive: &ArchiveInfo,
    wmo: &WmoInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<ModelSelected>,
) {
    let any_group_with_vertices = wmo.groups.iter().any(|g| g.vertex_count > 0);

    let header = egui::CollapsingHeader::new(format!("{}", wmo.path.display()))
        .enabled(any_group_with_vertices)
        .show(ui, |ui| {
            ui.label(format!("Materials: {}", wmo.material_count));
            ui.label(format!("Textures: {}", wmo.texture_count));

            egui::CollapsingHeader::new("Groups").show(ui, |ui| {
                for group in &wmo.groups {
                    wmo_group_info(group, ui);
                }
            });
        });
    if header.header_response.clicked() {
        event_writer.write(ModelSelected {
            archive_path: archive.path.clone(),
            model_path: wmo.path.clone(),
        });
    }
}

fn wmo_group_info(group: &WmoGroupInfo, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(&group.name).show(ui, |ui| {
        ui.label(format!("Vertices: {}", group.vertex_count));
        ui.label(format!("Indices: {}", group.index_count));
    });
}
