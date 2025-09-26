// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::RecursiveDependencyLoadState, prelude::*};
use bevy_egui::*;

use crate::{
    data::{archive, file},
    settings::{self, FileSettings},
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

impl FileSelected {
    pub fn new(file_path: String) -> Self {
        info!("File selected: {}", file_path);
        Self { file_path }
    }

    pub fn get_asset_path(&self) -> String {
        format!("archive://{}", self.file_path)
    }
}

impl From<&FileSettings> for FileSelected {
    fn from(settings: &FileSettings) -> Self {
        Self {
            file_path: settings.file_path.clone(),
        }
    }
}

pub fn select_default_model(mut event_writer: EventWriter<FileSelected>) {
    let default_model_path = settings::Settings::get().test_model_path.clone();
    event_writer.write(FileSelected::new(default_model_path));
}

fn data_info(
    mut contexts: EguiContexts,
    data_info: Res<archive::ArchiveInfoMap>,
    file_info_map: Res<file::FileInfoMap>,
    mut event_writer: EventWriter<FileSelected>,
    asset_server: Res<AssetServer>,
) -> Result<()> {
    // Left side panel replacing the floating "Info" window.
    egui::SidePanel::left("info_panel")
        .resizable(true)
        .min_width(220.0)
        .default_width(260.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Info");
            ui.separator();
            // Single scroll area with both vertical and horizontal scrolling so
            // the horizontal scrollbar is rendered at the bottom of the panel.
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .id_salt("info_scroll")
                .show(ui, |ui| {
                    for archive in data_info.map.values() {
                        if let Err(err) = archive_info(
                            archive,
                            &file_info_map,
                            ui,
                            &mut event_writer,
                            &asset_server,
                        ) {
                            ui.colored_label(egui::Color32::RED, format!("{err}"));
                        }
                    }
                });
        });
    Ok(())
}

fn archive_info(
    archive: &archive::ArchiveInfo,
    file_info_map: &file::FileInfoMap,
    ui: &mut egui::Ui,
    event_writer: &mut EventWriter<FileSelected>,
    asset_server: &AssetServer,
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
                        let file_info = file_info_map.get_file(path)?;
                        file_info_header(file_info, ui, asset_server);
                    }
                    Ok::<(), BevyError>(())
                });
            egui::CollapsingHeader::new("Models")
                .enabled(!model_paths.is_empty())
                .show(ui, |ui| {
                    for path in model_paths {
                        let file_info = file_info_map.get_file(path)?;
                        model_info(file_info, ui, event_writer, asset_server);
                    }
                    Ok::<(), BevyError>(())
                });
            egui::CollapsingHeader::new("World Models")
                .enabled(!world_model_paths.is_empty())
                .show(ui, |ui| {
                    for path in world_model_paths {
                        let file_info = file_info_map.get_file(path)?;
                        world_model_info(file_info, ui, event_writer, asset_server);
                    }
                    Ok::<(), BevyError>(())
                });
            egui::CollapsingHeader::new("World Maps")
                .enabled(!world_map_paths.is_empty())
                .show(ui, |ui| {
                    for path in world_map_paths {
                        let file_info = file_info_map.get_file(path)?;
                        world_map_info(file_info, ui, event_writer, asset_server);
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
    asset_server: &AssetServer,
) {
    let header = file_info_header(file_info, ui, asset_server);
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
    asset_server: &AssetServer,
) {
    let header = file_info_header(file_info, ui, asset_server);
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
    asset_server: &AssetServer,
) {
    let header = file_info_header(file_info, ui, asset_server);
    if header.header_response.clicked() && !header.header_response.is_tooltip_open() {
        event_writer.write(FileSelected {
            file_path: file_info.path.to_owned(),
        });
    }
}

fn file_info_header(
    file_info: &file::FileInfo,
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
) -> egui::collapsing_header::CollapsingResponse<()> {
    let load_state = file_info.get_load_state(asset_server);
    let mut error_message = None;
    if let RecursiveDependencyLoadState::Failed(err) = &load_state {
        error_message.replace(err.to_string());
    }

    let file_icon = get_file_icon(&file_info.data_type);
    egui::CollapsingHeader::new(format!("{} {}", file_icon, file_info.path))
        .icon(move |ui, _, response| {
            let pos = response.rect.center();
            let anchor = egui::Align2::CENTER_CENTER;
            let font_id = egui::TextStyle::Button.resolve(ui.style());
            let text_color = ui.style().visuals.text_color();
            match load_state {
                RecursiveDependencyLoadState::NotLoaded => {
                    ui.painter().text(pos, anchor, "▶", font_id, text_color);
                }
                RecursiveDependencyLoadState::Loading => {
                    ui.painter().text(pos, anchor, "⏳", font_id, text_color);
                }
                RecursiveDependencyLoadState::Loaded => {
                    ui.painter()
                        .text(pos, anchor, "✔", font_id, egui::Color32::CYAN);
                }
                RecursiveDependencyLoadState::Failed(_) => {
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

fn get_file_icon(data_type: &file::DataType) -> &'static str {
    match data_type {
        file::DataType::Texture(_) => "🖼",
        file::DataType::Model(_) => "📦",
        file::DataType::WorldModel(_) => "🏰",
        file::DataType::WorldMap(_) => "🗺",
        file::DataType::Unknown => "❓",
    }
}
