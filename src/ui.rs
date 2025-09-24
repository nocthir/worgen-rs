// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;
use bevy_egui::*;

use crate::{
    data::{archive, file},
    settings::FileSettings,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FileSelected>()
            .add_systems(EguiPrimaryContextPass, data_info);
    }
}

#[derive(Event)]
pub struct FileSelected {
    pub file_path: String,
}

impl From<&FileSettings> for FileSelected {
    fn from(settings: &FileSettings) -> Self {
        Self {
            file_path: settings.file_path.clone(),
        }
    }
}

fn data_info(
    mut contexts: EguiContexts,
    data_info: Res<archive::ArchiveInfoMap>,
    file_info_map: Res<file::FileInfoMap>,
    mut event_writer: EventWriter<FileSelected>,
) -> Result<()> {
    egui::Window::new("Info")
        .scroll([false, true])
        .show(contexts.ctx_mut()?, |ui| {
            for archive in data_info.map.values() {
                archive_info(archive, &file_info_map, ui, &mut event_writer)?;
            }
            Ok::<(), BevyError>(())
        });
    Ok(())
}

fn archive_info(
    archive: &archive::ArchiveInfo,
    file_info_map: &file::FileInfoMap,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) -> Result<()> {
    let texture_paths = &archive.texture_paths;
    let model_paths = &archive.model_paths;
    let world_model_paths = &archive.world_model_paths;
    let world_map_paths = &archive.world_map_paths;

    egui::CollapsingHeader::new(format!("⛃ {}", archive.path.display()))
        .default_open(false)
        .show(ui, |ui| {
            egui::CollapsingHeader::new("Textures")
                .enabled(!texture_paths.is_empty())
                .show(ui, |ui| {
                    for path in texture_paths {
                        let file_info = file_info_map.get_file_info(path)?;
                        file_info_header(file_info, ui);
                    }
                    Ok::<(), BevyError>(())
                });
            egui::CollapsingHeader::new("Models")
                .enabled(!model_paths.is_empty())
                .show(ui, |ui| {
                    for path in model_paths {
                        let file_info = file_info_map.get_file_info(path)?;
                        model_info(file_info, ui, event_writer);
                    }
                    Ok::<(), BevyError>(())
                });
            egui::CollapsingHeader::new("World Models")
                .enabled(!world_model_paths.is_empty())
                .show(ui, |ui| {
                    for path in world_model_paths {
                        let file_info = file_info_map.get_file_info(path)?;
                        world_model_info(file_info, ui, event_writer);
                    }
                    Ok::<(), BevyError>(())
                });
            egui::CollapsingHeader::new("World Maps")
                .enabled(!world_map_paths.is_empty())
                .show(ui, |ui| {
                    for path in world_map_paths {
                        let file_info = file_info_map.get_file_info(path)?;
                        world_map_info(file_info, ui, event_writer);
                    }
                    Ok::<(), BevyError>(())
                });
        });

    Ok(())
}

fn model_info(
    file_info: &file::FileInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = file_info_header(file_info, ui);
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            file_path: file_info.path.to_owned(),
        });
    }
}

fn world_model_info(
    file_info: &file::FileInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = file_info_header(file_info, ui);
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            file_path: file_info.path.to_owned(),
        });
    }
}

fn world_map_info(
    file_info: &file::FileInfo,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
) {
    let header = file_info_header(file_info, ui);
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            file_path: file_info.path.to_owned(),
        });
    }
}

fn file_info_header(
    file_info: &file::FileInfo,
    ui: &mut egui::Ui,
) -> egui::collapsing_header::CollapsingResponse<()> {
    let file_state = file_info.state.clone();
    let mut error_message = None;
    if let file::FileInfoState::Error(err) = &file_info.state {
        error_message.replace(err.clone());
    }

    let file_icon = get_file_icon(file_info.data_type);
    egui::CollapsingHeader::new(format!("{} {}", file_icon, file_info.path))
        .icon(move |ui, _, response| {
            let pos = response.rect.center();
            let anchor = egui::Align2::CENTER_CENTER;
            let font_id = egui::TextStyle::Button.resolve(ui.style());
            let text_color = ui.style().visuals.text_color();
            match file_state {
                file::FileInfoState::Unloaded => {
                    ui.painter().text(pos, anchor, "▶", font_id, text_color);
                }
                file::FileInfoState::Loading => {
                    ui.painter().text(pos, anchor, "⏳", font_id, text_color);
                }
                file::FileInfoState::Loaded => {
                    ui.painter()
                        .text(pos, anchor, "✔", font_id, egui::Color32::CYAN);
                }
                file::FileInfoState::Error(_) => {
                    ui.painter()
                        .text(pos, anchor, "✖", font_id, egui::Color32::RED);
                }
            };
        })
        .show(ui, |ui| {
            if let Some(msg) = error_message {
                ui.colored_label(egui::Color32::RED, msg);
            }
        })
}

fn get_file_icon(data_type: file::DataType) -> &'static str {
    match data_type {
        file::DataType::Texture => "🖼",
        file::DataType::Model => "📦",
        file::DataType::WorldModel => "🏰",
        file::DataType::WorldMap => "🗺",
        file::DataType::Unknown => "❓",
    }
}
