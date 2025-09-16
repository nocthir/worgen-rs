// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;
use bevy_egui::*;

use crate::{
    data::{
        DataInfo,
        archive::{ArchiveInfo, ArchiveLoaded},
        model::ModelInfo,
        world_map::WorldMapInfo,
        world_model::{WmoGroupInfo, WmoInfo},
    },
    settings::{FileSettings, Settings},
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FileSelected>()
            .insert_resource(DataInfo::default())
            .add_systems(EguiPrimaryContextPass, data_info)
            .add_systems(
                Startup,
                select_main_menu_model.run_if(resource_exists::<Settings>),
            );
    }
}

#[derive(Event)]
pub struct FileSelected {
    pub archive_path: String,
    pub file_path: String,
}

impl From<&FileSettings> for FileSelected {
    fn from(settings: &FileSettings) -> Self {
        Self {
            archive_path: settings.archive_path.clone(),
            file_path: settings.file_path.clone(),
        }
    }
}

fn select_main_menu_model(mut event_writer: EventWriter<FileSelected>, settings: Res<Settings>) {
    event_writer.write(FileSelected::from(&settings.default_model));
}

fn data_info(
    mut contexts: EguiContexts,
    mut data_info: ResMut<DataInfo>,
    mut event_reader: EventReader<ArchiveLoaded>,
    mut event_writer: EventWriter<FileSelected>,
) -> Result<()> {
    for event in event_reader.read() {
        data_info.archives.push(event.archive.clone());
    }

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
    event_writer: &mut EventWriter<FileSelected>,
) {
    egui::CollapsingHeader::new(&archive.path)
        .default_open(false)
        .enabled(archive.has_stuff())
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Textures")
                .enabled(!archive.texture_infos.is_empty())
                .show(ui, |ui| {
                    for texture in &archive.texture_infos {
                        ui.label(&texture.path);
                    }
                });
            egui::CollapsingHeader::new("Models")
                .enabled(!archive.model_infos.is_empty())
                .show(ui, |ui| {
                    for model in &archive.model_infos {
                        model_info(archive, model, ui, event_writer);
                    }
                });
            egui::CollapsingHeader::new("World Models")
                .enabled(!archive.wmo_infos.is_empty())
                .show(ui, |ui| {
                    for wmo in &archive.wmo_infos {
                        wmo_info(archive, wmo, ui, event_writer);
                    }
                });
            egui::CollapsingHeader::new("World Maps")
                .enabled(!archive.world_map_infos.is_empty())
                .show(ui, |ui| {
                    for world_map in &archive.world_map_infos {
                        world_map_info(archive, world_map, ui, event_writer);
                    }
                });
        });
}

fn model_info(
    archive: &ArchiveInfo,
    model: &ModelInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = egui::CollapsingHeader::new(&model.path)
        .enabled(model.vertex_count > 0)
        .show(ui, |ui| {
            ui.label(format!("Vertices: {}", model.vertex_count));
            egui::CollapsingHeader::new("Textures")
                .enabled(!model.textures.is_empty())
                .show(ui, |ui| {
                    for texture in &model.textures {
                        ui.label(texture);
                    }
                });
            ui.label(format!("Materials: {}", model.materials));
        });
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            archive_path: archive.path.clone(),
            file_path: model.path.clone(),
        });
    }
}

fn wmo_info(
    archive: &ArchiveInfo,
    wmo: &WmoInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let any_group_with_vertices = wmo.groups.iter().any(|g| g.vertex_count > 0);

    let header = egui::CollapsingHeader::new(&wmo.path)
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
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            archive_path: archive.path.clone(),
            file_path: wmo.path.clone(),
        });
    }
}

fn wmo_group_info(group: &WmoGroupInfo, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(&group.name).show(ui, |ui| {
        ui.label(format!("Vertices: {}", group.vertex_count));
        ui.label(format!("Indices: {}", group.index_count));
    });
}

fn world_map_info(
    archive: &ArchiveInfo,
    world_map: &WorldMapInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = egui::CollapsingHeader::new(&world_map.path)
        .enabled(world_map.has_stuff())
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Models")
                .enabled(!world_map.models.is_empty())
                .show(ui, |ui| {
                    for model in &world_map.models {
                        ui.label(model);
                    }
                });
            egui::CollapsingHeader::new("World Models")
                .enabled(!world_map.world_models.is_empty())
                .show(ui, |ui| {
                    for world in &world_map.world_models {
                        ui.label(world);
                    }
                });
        });
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            archive_path: archive.path.clone(),
            file_path: world_map.path.clone(),
        });
    }
}
