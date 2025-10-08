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
    assets::{geoset::*, material::TerrainMaterial, model::*, world_model::WorldModel},
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
                        ui.separator();
                        geosets_models_ui(world, ui);
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
        // Note: Geoset editing UI lives in panel-level helper because we need mutable access to GeosetSelection.
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

/// Geoset editor:
fn geosets_models_ui(world: &mut World, ui: &mut egui::Ui) {
    // Gather immutable data first (entity, name, catalog) to avoid holding a mutable borrow
    let mut gather_query = world.query::<(Entity, &Model, &GeosetCatalog)>();
    let mut models: Vec<(Entity, String, GeosetCatalog)> = Vec::new();
    for (entity, model, catalog) in gather_query.iter(world) {
        models.push((entity, model.name.clone(), catalog.clone()));
    }
    if models.is_empty() {
        return;
    }
    // We'll reuse a separate query for mutable selection borrows inside UI passes.
    let mut selection_query = world.query::<&mut GeosetSelection>();
    egui::CollapsingHeader::new("👤 Geosets")
        .default_open(false)
        .show(ui, |ui| {
            models.sort_by(|a, b| a.1.cmp(&b.1));
            for (entity, name, catalog) in models.into_iter() {
                egui::CollapsingHeader::new(format!("{entity}: {name}"))
                    .default_open(false)
                    .show(ui, |ui| {
                        if let Ok(selection) = selection_query.get_mut(world, entity) {
                            geoset_catalog_ui(selection, &catalog, ui);
                        } else {
                            ui.label("Selection component not found");
                        }
                    });
            }
        });
}

fn geoset_catalog_ui(
    mut selection: Mut<GeosetSelection>,
    catalog: &GeosetCatalog,
    ui: &mut egui::Ui,
) {
    // Build sorted list of categories for stable UI (sort by debug name)
    let mut cats: Vec<GeosetType> = catalog.categories.keys().copied().collect();
    cats.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    for cat in cats {
        let variants = catalog
            .categories
            .get(&cat)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        if variants.is_empty() {
            continue;
        }
        ui.separator();
        if cat.is_exclusive() {
            exclusive_category_row(ui, cat, variants, &mut selection, catalog);
        } else {
            additive_category_row(ui, cat, variants, &mut selection);
        }
    }
}

/// Exclusive categories: compact row with prev/next, current variant, reset, none.
fn exclusive_category_row(
    ui: &mut egui::Ui,
    cat: GeosetType,
    variants: &[u16],
    selection: &mut GeosetSelection,
    catalog: &GeosetCatalog,
) {
    let current_opt = selection.selected_exclusive(cat);
    let current = current_opt.unwrap_or_else(|| variants[0]);
    ui.horizontal(|ui| {
        let label = if cat.all_variants_always_visible() {
            format!("{} (all visible)", cat)
        } else {
            cat.as_str().to_string()
        };
        ui.label(label);
        if ui.button("◀").on_hover_text("Previous variant").clicked() && current_opt.is_some() {
            // only cycle if something selected
            if let Some(idx) = variants.iter().position(|v| *v == current) {
                let prev_idx = if idx == 0 {
                    variants.len() - 1
                } else {
                    idx - 1
                };
                selection.set_exclusive(cat, variants[prev_idx]);
            }
        }
        // Display current variant id
        if current_opt.is_some() {
            ui.monospace(format!("{:02}", current));
        } else {
            ui.monospace("--");
        }
        if ui.button("▶").on_hover_text("Next variant").clicked() && current_opt.is_some() {
            selection.cycle(cat, catalog);
        }
        if ui
            .small_button("Reset")
            .on_hover_text("Reset to first variant")
            .clicked()
        {
            selection.set_exclusive(cat, variants[0]);
        }
        // Provide a way to clear selection (notably for Tabard / optional gear)
        if ui
            .small_button("None")
            .on_hover_text("Clear / hide this category")
            .clicked()
        {
            selection.clear_exclusive(cat);
        }
    });
}

/// Additive categories: collapsing header containing toggle chips.
fn additive_category_row(
    ui: &mut egui::Ui,
    cat: GeosetType,
    variants: &[u16],
    selection: &mut GeosetSelection,
) {
    egui::CollapsingHeader::new(format!("{} (additive)", cat))
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for &v in variants {
                    let enabled = selection.is_additive_enabled(cat, v);
                    let text = format!("{:02}", v);
                    let resp = ui.selectable_label(enabled, text);
                    if resp.clicked() {
                        selection.toggle_additive(cat, v);
                    }
                }
            });
        });
}
