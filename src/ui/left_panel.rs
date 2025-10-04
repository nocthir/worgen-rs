// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{asset::RecursiveDependencyLoadState, prelude::*};
use bevy_egui::*;
use bevy_inspector_egui::{
    bevy_inspector::ui_for_resource, inspector_egui_impls::InspectorPrimitive,
    reflect_inspector::InspectorUi,
};

use crate::{
    data::{
        archive::{ArchiveInfo, ArchiveInfoMap},
        file::{FileInfo, FileInfoMap},
    },
    settings::TerrainSettings,
    ui::{FileSelected, get_file_icon},
};

pub fn ui(world: &mut World, context: &mut EguiContext) -> egui::InnerResponse<()> {
    egui::SidePanel::left("info_panel")
        .resizable(true)
        .min_width(240.0)
        .default_width(320.0)
        .show(context.get_mut(), |ui| {
            egui::CollapsingHeader::new("Terrain Settings")
                .default_open(false)
                .show(ui, |ui| {
                    ui_for_resource::<TerrainSettings>(world, ui);
                });

            ui.separator();

            // Single scroll area with both vertical and horizontal scrolling so
            // the horizontal scrollbar is rendered at the bottom of the panel.
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .id_salt("info_scroll")
                .show(ui, |ui| {
                    ui_for_resource::<ArchiveInfoMap>(world, ui);
                });
        })
}

impl InspectorPrimitive for ArchiveInfoMap {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        options: &dyn std::any::Any,
        id: egui::Id,
        env: InspectorUi<'_, '_>,
    ) -> bool {
        self.ui_readonly(ui, options, id, env);
        false
    }

    fn ui_readonly(
        &self,
        ui: &mut egui::Ui,
        _: &dyn std::any::Any,
        _: egui::Id,
        env: InspectorUi<'_, '_>,
    ) {
        let Some(world) = &mut env.context.world else {
            ui.label("No world available");
            return;
        };
        for archive in self.map.values() {
            archive_ui(archive, unsafe { world.world().world_mut() }, ui);
        }
    }
}

fn archive_ui(archive: &ArchiveInfo, world: &mut World, ui: &mut egui::Ui) {
    let archive_file_name = archive
        .path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("Unknown");
    let label = format!("⛃ {}", archive_file_name);

    egui::CollapsingHeader::new(label)
        .default_open(false)
        .show(ui, |ui| {
            let Some(file_info_map) = world.get_resource::<FileInfoMap>() else {
                ui.colored_label(egui::Color32::RED, "No file info map available");
                return;
            };

            let Some(asset_server) = world.get_resource::<AssetServer>() else {
                ui.colored_label(egui::Color32::RED, "No asset server available");
                return;
            };

            let mut message = None;

            if let Some(msg) = archive_files_ui(
                "Textures",
                &archive.texture_paths,
                file_info_map,
                ui,
                asset_server,
            ) {
                message.replace(msg);
            }
            if let Some(msg) = archive_files_ui(
                "Models",
                &archive.model_paths,
                file_info_map,
                ui,
                asset_server,
            ) {
                message.replace(msg);
            }
            if let Some(msg) = archive_files_ui(
                "World Models",
                &archive.world_model_paths,
                file_info_map,
                ui,
                asset_server,
            ) {
                message.replace(msg);
            }
            if let Some(msg) = archive_files_ui(
                "World Maps",
                &archive.world_map_paths,
                file_info_map,
                ui,
                asset_server,
            ) {
                message.replace(msg);
            }

            if let Some(message) = message {
                world.write_message(message);
            }
        });
}

fn archive_files_ui<S: AsRef<str>>(
    label: S,
    paths: &[String],
    file_info_map: &FileInfoMap,
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
) -> Option<FileSelected> {
    let response = egui::CollapsingHeader::new(label.as_ref()).show(ui, |ui| {
        let mut ret = None;
        for path in paths {
            if let Ok(file_info) = file_info_map.get_file(path) {
                if let Some(message) = archive_file_ui(file_info, ui, asset_server) {
                    ret.replace(message);
                }
            } else {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Failed to get info for {}", path),
                );
            }
        }
        ret
    });
    response.body_returned.flatten()
}

fn archive_file_ui(
    file_info: &FileInfo,
    ui: &mut egui::Ui,
    asset_server: &AssetServer,
) -> Option<FileSelected> {
    let r = file_ui(file_info, ui, asset_server);
    if r.header_response.clicked() && !r.header_response.is_tooltip_open() {
        Some(FileSelected {
            file_path: file_info.path.to_owned(),
        })
    } else {
        None
    }
}

fn file_ui(
    file_info: &FileInfo,
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
