// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{
    image::{ImageAddressMode, ImageSampler},
    prelude::*,
};
use bevy_egui::*;
use bevy_inspector_egui::{
    bevy_inspector::{Filter, ui_for_entities_filtered},
    inspector_egui_impls::InspectorPrimitive,
    reflect_inspector::InspectorUi,
    restricted_world_view::RestrictedWorldView,
};

use crate::{
    assets::{material::TerrainMaterial, model::Model, world_model::WorldModel},
    data::{CurrentFile, file::FileInfoMap},
    ui::get_file_icon,
};

pub fn ui(world: &mut World, context: &mut EguiContext) -> egui::InnerResponse<()> {
    let side_panel = egui::SidePanel::right("current_file_panel");

    let mut file_path = None;
    if let Ok(current_file) = world.query::<&CurrentFile>().single(world) {
        file_path.replace(current_file.path.clone());
    }

    let mut label = None;
    if let Some(file_path) = file_path
        && let Some(file_info_map) = world.get_resource::<FileInfoMap>()
        && let Ok(file_info) = file_info_map.get_file(&file_path)
    {
        label.replace(format!(
            "{} {}",
            get_file_icon(&file_info.data_type),
            file_path
        ));
    }

    if let Some(label) = label {
        side_panel
            .resizable(true)
            .min_width(240.0)
            .default_width(320.0)
            .show(context.get_mut(), |ui| {
                // Single scroll area with both vertical and horizontal scrolling so
                // the horizontal scrollbar is rendered at the bottom of the panel.
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .id_salt("current_file_scroll")
                    .show(ui, |ui| {
                        ui.label(label);
                        ui_for_entities_filtered(
                            world,
                            ui,
                            true,
                            &Filter::<With<CurrentFile>>::all(),
                        );
                    });
            })
    } else {
        // Empty panel when no file is selected
        side_panel
            .resizable(false)
            .exact_width(0.0)
            .show(context.get_mut(), |_ui| {})
    }
}

impl InspectorPrimitive for Model {
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

        ui.label(format!("Name: {}", self.name));
        images_ui("Images", &self.images, world, ui);
    }
}

impl InspectorPrimitive for WorldModel {
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

        images_ui("Images", &self.images, world, ui);
    }
}

fn images_ui<S: AsRef<str>>(
    label: S,
    images: &[Handle<Image>],
    world: &mut RestrictedWorldView,
    ui: &mut egui::Ui,
) {
    if !images.is_empty() {
        egui::CollapsingHeader::new(label.as_ref())
            .default_open(false)
            .show(ui, |ui| {
                for (index, handle) in images.iter().enumerate() {
                    image_ui(handle, index, world, ui);
                }
            });
    }
}

fn image_ui(
    handle: &Handle<Image>,
    index: usize,
    world: &mut RestrictedWorldView,
    ui: &mut egui::Ui,
) {
    let asset_path = handle
        .path()
        .as_ref()
        .map(|p| format!("{}: {}", index, p))
        .unwrap_or(format!("{}: Image", index));
    egui::CollapsingHeader::new(asset_path)
        .default_open(false)
        .show(ui, |ui| {
            let mut width = 128.0;
            let mut height = 128.0;

            let mut assets = world.get_resource_mut::<Assets<Image>>().unwrap();
            if let Some(image) = assets.get_mut(handle) {
                width = image.texture_descriptor.size.width as f32;
                height = image.texture_descriptor.size.height as f32;
                ui.label(format!("Size: {width}x{height}"));
                ui.label(format!("Format: {:?}", image.texture_descriptor.format));
                ui.label(format!(
                    "Mip Levels: {}",
                    image.texture_descriptor.mip_level_count
                ));
                sampler_info(&mut image.sampler, ui);
            } else {
                ui.colored_label(egui::Color32::YELLOW, "Image asset not found");
            };

            let textures = world.get_resource_mut::<EguiUserTextures>().unwrap();
            if let Some(tex_id) = textures.image_id(handle) {
                let w = width.min(128.0);
                let h = (height * (w / width)).min(128.0);
                let texture = egui::load::SizedTexture::new(tex_id, [w, h]);
                ui.add(egui::widgets::Image::new(texture));
            }
        });
}

fn sampler_info(sampler: &mut ImageSampler, ui: &mut egui::Ui) {
    let descriptor = sampler.get_or_init_descriptor();
    egui::CollapsingHeader::new("Sampler").show(ui, |ui| {
        address_mode_combo("Address mode U", &mut descriptor.address_mode_u, ui);
        address_mode_combo("Address mode V", &mut descriptor.address_mode_v, ui);
        address_mode_combo("Address mode W", &mut descriptor.address_mode_w, ui);
    });
}

fn address_mode_combo(label: &str, address_mode: &mut ImageAddressMode, ui: &mut egui::Ui) {
    egui::ComboBox::from_label(label)
        .selected_text(address_mode_label(*address_mode))
        .show_ui(ui, |ui| {
            let r = ui.selectable_value(address_mode, ImageAddressMode::ClampToEdge, "ClampToEdge");
            if r.clicked() {
                *address_mode = ImageAddressMode::ClampToEdge;
            }

            let r = ui.selectable_value(address_mode, ImageAddressMode::Repeat, "Repeat");
            if r.clicked() {
                *address_mode = ImageAddressMode::Repeat;
            }

            let r =
                ui.selectable_value(address_mode, ImageAddressMode::MirrorRepeat, "MirrorRepeat");
            if r.clicked() {
                *address_mode = ImageAddressMode::MirrorRepeat;
            }

            let r = ui.selectable_value(
                address_mode,
                ImageAddressMode::ClampToBorder,
                "ClampToBorder",
            );
            if r.clicked() {
                *address_mode = ImageAddressMode::ClampToBorder;
            }
        });
}

fn address_mode_label(address_mode: ImageAddressMode) -> &'static str {
    match address_mode {
        ImageAddressMode::ClampToEdge => "ClampToEdge",
        ImageAddressMode::Repeat => "Repeat",
        ImageAddressMode::MirrorRepeat => "MirrorRepeat",
        ImageAddressMode::ClampToBorder => "ClampToBorder",
    }
}

impl InspectorPrimitive for TerrainMaterial {
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

        image_ui(&self.alpha_texture, 0, world, ui);
        if let Some(level1) = &self.level1_texture {
            image_ui(level1, 1, world, ui);
        }
        if let Some(level2) = &self.level2_texture {
            image_ui(level2, 2, world, ui);
        }
        if let Some(level3) = &self.level3_texture {
            image_ui(level3, 3, world, ui);
        }
    }
}
