// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::collections::HashMap;

use bevy::{
    asset::RecursiveDependencyLoadState,
    ecs::system::SystemParam,
    pbr::ExtendedMaterial,
    prelude::*,
    render::{camera::Viewport, mesh::VertexAttributeValues, view::RenderLayers},
    window::PrimaryWindow,
};
use bevy_egui::*;

use crate::{
    assets::{model, world_map, world_model},
    data::{self, archive, file},
    material::TerrainMaterial,
    settings::{self, FileSettings},
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FileSelected>()
            .add_systems(Startup, setup_ui)
            .add_systems(EguiPrimaryContextPass, data_info);
    }
}

fn setup_ui(mut commands: Commands, mut egui_global_settings: ResMut<EguiGlobalSettings>) {
    // Disable the automatic creation of a primary context to set it up manually for the camera we need.
    egui_global_settings.auto_create_primary_context = false;

    // Egui camera.
    commands.spawn((
        // The `PrimaryEguiContext` component requires everything needed to render a primary context.
        PrimaryEguiContext,
        Camera2d,
        // Setting RenderLayers to none makes sure we won't render anything apart from the UI.
        RenderLayers::none(),
        Camera {
            order: 1,
            ..default()
        },
    ));
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

#[derive(SystemParam)]
struct AssetParams<'w> {
    images: ResMut<'w, Assets<Image>>,
    materials: Res<'w, Assets<StandardMaterial>>,
    terrain_materials: Res<'w, Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    meshes: Res<'w, Assets<Mesh>>,
    models: Res<'w, Assets<model::ModelAsset>>,
    world_models: Res<'w, Assets<world_model::WorldModelAsset>>,
    world_maps: Res<'w, Assets<world_map::WorldMapAsset>>,
}

#[derive(SystemParam)]
struct WindowParams<'w> {
    window: Single<'w, &'static Window, With<PrimaryWindow>>,
    camera: Single<'w, &'static mut Camera, Without<EguiContext>>,
}

#[derive(SystemParam)]
struct InfoParams<'w, 's> {
    data_info: Res<'w, archive::ArchiveInfoMap>,
    file_info_map: Res<'w, file::FileInfoMap>,
    asset_server: Res<'w, AssetServer>,
    current_file: Query<'w, 's, &'static data::CurrentFile, With<data::CurrentFile>>,
    event_writer: EventWriter<'w, FileSelected>,
}

fn data_info(
    mut contexts: EguiContexts,
    mut info: InfoParams,
    mut terrain_settings: ResMut<settings::TerrainSettings>,
    assets: AssetParams,
    // Window + camera for viewport adjustment so 3D scene doesn't render under panel
    window_camera: WindowParams,
) -> Result<()> {
    // Get egui image handles
    let image_handles = get_image_handles(
        &info.current_file,
        &info.file_info_map,
        &assets,
        &mut contexts,
    );

    // Acquire egui context once
    let ctx = contexts.ctx_mut()?;

    let left_panel_response = left_panel(&mut info, &mut terrain_settings, ctx);

    let right_panel_response = right_panel(&mut info, &assets, image_handles, ctx);

    // Adjust world camera viewport so scene starts to the right of the panel (avoid rendering beneath UI)
    let left_width_logical = left_panel_response.response.rect.width();

    let window = window_camera.window;
    let mut camera = window_camera.camera;

    let scale = window.scale_factor();
    let left_phys = (left_width_logical * scale)
        .round()
        .clamp(0.0, window.physical_width() as f32) as u32;

    let right_width_logical = right_panel_response.response.rect.width();
    let right_phys = (right_width_logical * scale)
        .round()
        .clamp(0.0, window.physical_width() as f32) as u32;
    let viewport_width = window.physical_width() - right_phys - left_phys;

    let pos = UVec2::new(left_phys, 0);
    let size = UVec2::new(viewport_width, window.physical_height());
    // Only update if changed to avoid unnecessary render graph invalidation.
    let needs_update = match &camera.viewport {
        Some(vp) => vp.physical_position != pos || vp.physical_size != size,
        None => true,
    };
    if needs_update {
        camera.viewport = Some(Viewport {
            physical_position: pos,
            physical_size: size,
            ..default()
        });
    }

    Ok(())
}

fn get_image_handles(
    current_file: &Query<&data::CurrentFile, With<data::CurrentFile>>,
    file_info_map: &file::FileInfoMap,
    assets: &AssetParams,
    contexts: &mut EguiContexts,
) -> HashMap<Handle<Image>, egui::TextureId> {
    let mut image_handles = HashMap::new();
    if let Ok(current_file) = current_file.single() {
        let file = file_info_map.get_file(&current_file.path).unwrap();
        match &file.data_type {
            file::DataType::Texture(image_handle) => {
                let texture_id = contexts.add_image(image_handle.clone_weak());
                image_handles.insert(image_handle.clone(), texture_id);
            }
            file::DataType::WorldMap(world_map_handle) => {
                if let Some(world_map) = assets.world_maps.get(world_map_handle) {
                    for image in &world_map.images {
                        let texture_id = contexts.add_image(image.clone_weak());
                        image_handles.insert(image.clone(), texture_id);
                    }
                }
            }
            _ => (),
        }
    }
    image_handles
}

fn left_panel(
    info: &mut InfoParams,
    terrain_settings: &mut settings::TerrainSettings,
    ctx: &mut egui::Context,
) -> egui::InnerResponse<()> {
    egui::SidePanel::left("info_panel")
        .resizable(true)
        .min_width(220.0)
        .default_width(260.0)
        .show(ctx, |ui| {
            terrain_settings_widget(terrain_settings, ui);
            ui.separator();

            // Single scroll area with both vertical and horizontal scrolling so
            // the horizontal scrollbar is rendered at the bottom of the panel.
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .id_salt("info_scroll")
                .show(ui, |ui| {
                    for archive in info.data_info.map.values() {
                        if let Err(err) = archive_info(
                            archive,
                            &info.file_info_map,
                            ui,
                            &mut info.event_writer,
                            &info.asset_server,
                        ) {
                            ui.colored_label(egui::Color32::RED, format!("{err}"));
                        }
                    }
                });
        })
}

fn terrain_settings_widget(terrain_settings: &mut settings::TerrainSettings, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Terrain Settings")
        .default_open(false)
        .show(ui, |ui| {
            ui.checkbox(&mut terrain_settings.level0, "Level 0");
            ui.checkbox(&mut terrain_settings.level1, "Level 1");
            ui.checkbox(&mut terrain_settings.level2, "Level 2");
            ui.checkbox(&mut terrain_settings.level3, "Level 3");
        });
}

fn right_panel(
    info: &mut InfoParams,
    assets: &AssetParams,
    image_map: HashMap<Handle<Image>, egui::TextureId>,
    ctx: &mut egui::Context,
) -> egui::InnerResponse<()> {
    let side_panel = egui::SidePanel::right("current_file_panel");
    if let Ok(current_file) = info.current_file.single() {
        side_panel
            .resizable(true)
            .min_width(220.0)
            .default_width(260.0)
            .show(ctx, |ui| {
                // Single scroll area with both vertical and horizontal scrolling so
                // the horizontal scrollbar is rendered at the bottom of the panel.
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .id_salt("current_file_scroll")
                    .show(ui, |ui| {
                        if let Ok(file_info) = info.file_info_map.get_file(&current_file.path) {
                            ui.label(&file_info.path);
                            data_type_info(&file_info.data_type, assets, &image_map, ui);
                        } else {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!("Failed to get info for {}", current_file.path),
                            );
                        }
                    });
            })
    } else {
        // Empty panel when no file is selected
        side_panel
            .resizable(false)
            .exact_width(0.0)
            .show(ctx, |_ui| {})
    }
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

fn data_type_info(
    data_type: &file::DataType,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    match data_type {
        file::DataType::Texture(image) => {
            image_type_info(image, assets, image_map, ui);
        }
        file::DataType::Model(handle) => {
            model_type_info(handle, 0, assets, image_map, ui);
        }
        file::DataType::WorldModel(handle) => {
            world_model_type_info(handle, 0, assets, image_map, ui);
        }
        file::DataType::WorldMap(handle) => {
            world_map_type_info(handle, assets, image_map, ui);
        }
        file::DataType::Unknown => {
            ui.label("The type of this file is unknown.");
        }
    }
}

fn model_type_info(
    handle: &Handle<model::ModelAsset>,
    index: usize,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if let Some(model) = assets.models.get(handle) {
        egui::CollapsingHeader::new(format!("Model {}", index))
            .default_open(false)
            .show(ui, |ui| {
                images_type_info(&model.images, assets, image_map, ui);
                materials_type_info(&model.materials, assets, ui);
                meshes_type_info(&model.meshes, assets, ui);
                ui.label(format!("Aabb: {}", model.aabb));
            });
    } else {
        ui.colored_label(egui::Color32::YELLOW, "Model not loaded");
    }
}

fn world_model_type_info(
    handle: &Handle<world_model::WorldModelAsset>,
    index: usize,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if let Some(world_model) = assets.world_models.get(handle) {
        egui::CollapsingHeader::new(format!("World Model {}", index))
            .default_open(false)
            .show(ui, |ui| {
                images_type_info(&world_model.images, assets, image_map, ui);
                materials_type_info(&world_model.materials, assets, ui);
                meshes_type_info(&world_model.meshes, assets, ui);
                ui.label(format!("Aabb: {}", world_model.aabb));
            });
    } else {
        ui.colored_label(egui::Color32::YELLOW, "World Model not loaded");
    }
}

fn world_map_type_info(
    handle: &Handle<world_map::WorldMapAsset>,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if let Some(world_map) = assets.world_maps.get(handle) {
        egui::CollapsingHeader::new("World Map")
            .default_open(false)
            .show(ui, |ui| {
                images_type_info(&world_map.images, assets, image_map, ui);
                terrains_type_info(&world_map.terrain, assets, image_map, ui);
                models_type_info(&world_map.models, assets, image_map, ui);
                world_models_type_info(&world_map.world_models, assets, image_map, ui);
                ui.label(format!("Aabb: {}", world_map.aabb));
            });
    } else {
        ui.colored_label(egui::Color32::YELLOW, "World Map not loaded");
    }
}

fn images_type_info(
    images: &[Handle<Image>],
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if !images.is_empty() {
        egui::CollapsingHeader::new("Images")
            .default_open(false)
            .show(ui, |ui| {
                for handle in images {
                    image_type_info(handle, assets, image_map, ui);
                }
            });
    }
}

fn materials_type_info(
    materials: &[Handle<StandardMaterial>],
    assets: &AssetParams,
    ui: &mut egui::Ui,
) {
    if !materials.is_empty() {
        egui::CollapsingHeader::new("Materials")
            .default_open(false)
            .show(ui, |ui| {
                for (index, material) in materials.iter().enumerate() {
                    material_type_info(material, index, assets, ui);
                }
            });
    }
}

fn meshes_type_info(meshes: &[Handle<Mesh>], assets: &AssetParams, ui: &mut egui::Ui) {
    if !meshes.is_empty() {
        egui::CollapsingHeader::new("Meshes")
            .default_open(false)
            .show(ui, |ui| {
                for (index, mesh) in meshes.iter().enumerate() {
                    mesh_type_info(mesh, index, assets, ui);
                }
            });
    }
}

fn models_type_info(
    models: &[Handle<model::ModelAsset>],
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if !models.is_empty() {
        egui::CollapsingHeader::new("Models")
            .default_open(false)
            .show(ui, |ui| {
                for (index, model) in models.iter().enumerate() {
                    model_type_info(model, index, assets, image_map, ui);
                }
            });
    }
}

fn world_models_type_info(
    world_models: &[Handle<world_model::WorldModelAsset>],
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if !world_models.is_empty() {
        egui::CollapsingHeader::new("World Models")
            .default_open(false)
            .show(ui, |ui| {
                for (index, world_model) in world_models.iter().enumerate() {
                    world_model_type_info(world_model, index, assets, image_map, ui);
                }
            });
    }
}

fn terrains_type_info(
    terrains: &[world_map::TerrainBundle],
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if !terrains.is_empty() {
        egui::CollapsingHeader::new("Terrains")
            .default_open(false)
            .show(ui, |ui| {
                for (index, terrain) in terrains.iter().enumerate() {
                    terrain_type_info(terrain, index, assets, image_map, ui);
                }
            });
    }
}

fn terrain_type_info(
    terrain: &world_map::TerrainBundle,
    terrain_index: usize,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    egui::CollapsingHeader::new(format!("Terrain {}", terrain_index))
        .default_open(false)
        .show(ui, |ui| {
            terrain_material_type_info(&terrain.material, assets, image_map, ui);
            mesh_type_info(&terrain.mesh, 0, assets, ui);
        });
}

fn image_type_info(
    handle: &Handle<Image>,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if let Some(image) = assets.images.get(handle) {
        let label = image.texture_descriptor.label.unwrap_or("Image");
        egui::CollapsingHeader::new(label)
            .default_open(false)
            .show(ui, |ui| {
                let w = image.texture_descriptor.size.width as f32;
                let h = image.texture_descriptor.size.height as f32;

                ui.label(format!("Size: {w}x{h}"));
                ui.label(format!("Format: {:?}", image.texture_descriptor.format));
                ui.label(format!(
                    "Mip Levels: {}",
                    image.texture_descriptor.mip_level_count
                ));
            });
    } else if let Some(tex_id) = image_map.get(handle) {
        let texture = egui::load::SizedTexture::new(*tex_id, [128.0, 128.0]);
        ui.add(egui::widgets::Image::new(texture));
    } else {
        ui.colored_label(egui::Color32::YELLOW, "Image not loaded");
    }
}

fn material_type_info(
    material: &Handle<StandardMaterial>,
    material_index: usize,
    assets: &AssetParams,
    ui: &mut egui::Ui,
) {
    if let Some(material) = assets.materials.get(material) {
        material_impl_type_info(material, material_index, ui);
    } else {
        ui.colored_label(egui::Color32::YELLOW, "Material not loaded");
    }
}

fn material_impl_type_info(material: &StandardMaterial, material_index: usize, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(format!("Material {}", material_index))
        .default_open(false)
        .show(ui, |ui| {
            ui.label(format!("Base Color: {:?}", material.base_color));
            if let Some(base_color_texture) = &material.base_color_texture {
                ui.label(format!(
                    "Base Color Texture: {}",
                    base_color_texture.path().as_ref().unwrap()
                ));
            }
            ui.label(format!("Emissive Color: {:?}", material.emissive));
            ui.label(format!("Alpha Mode: {:?}", material.alpha_mode));
            ui.label(format!("Double Sided: {}", material.double_sided));
        });
}

fn terrain_material_type_info(
    material: &Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    assets: &AssetParams,
    image_map: &HashMap<Handle<Image>, egui::TextureId>,
    ui: &mut egui::Ui,
) {
    if let Some(material) = assets.terrain_materials.get(material) {
        material_impl_type_info(&material.base, 0, ui);

        egui::CollapsingHeader::new("Terrain Material")
            .default_open(false)
            .show(ui, |ui| {
                image_type_info(&material.extension.alpha_texture, assets, image_map, ui);
                if let Some(level1) = &material.extension.level1_texture {
                    image_type_info(level1, assets, image_map, ui);
                }
                if let Some(level2) = &material.extension.level2_texture {
                    image_type_info(level2, assets, image_map, ui);
                }
                if let Some(level3) = &material.extension.level3_texture {
                    image_type_info(level3, assets, image_map, ui);
                }
            });
    } else {
        ui.colored_label(egui::Color32::YELLOW, "Material not loaded");
    }
}

fn mesh_type_info(
    handle: &Handle<Mesh>,
    mesh_index: usize,
    assets: &AssetParams,
    ui: &mut egui::Ui,
) {
    if let Some(mesh) = assets.meshes.get(handle) {
        let label: String = format!("Mesh{}", mesh_index);
        egui::CollapsingHeader::new(label)
            .default_open(false)
            .show(ui, |ui| {
                for (attribute_name, attribute_value) in mesh.attributes() {
                    egui::CollapsingHeader::new(format!("{:?}", attribute_name.name))
                        .default_open(false)
                        .show(ui, |ui| {
                            attribute_value_info(attribute_value, ui);
                        });
                }
            });
    } else {
        ui.colored_label(egui::Color32::YELLOW, "Mesh not loaded");
    }
}

fn attribute_value_info(attribute_value: &VertexAttributeValues, ui: &mut egui::Ui) {
    match attribute_value {
        VertexAttributeValues::Float32x3(values) => {
            if !values.is_empty() {
                let preview_count = values.len().min(5);
                for v in values.iter().take(preview_count) {
                    ui.label(format!("{:?} ", v));
                }
                if values.len() > preview_count {
                    ui.label("  ...");
                }
            }
        }
        VertexAttributeValues::Float32x2(values) => {
            if !values.is_empty() {
                let preview_count = values.len().min(5);
                for v in values.iter().take(preview_count) {
                    ui.label(format!("{:?} ", v));
                }
                if values.len() > preview_count {
                    ui.label("  ...");
                }
            }
        }
        _ => {
            ui.label("Unsupported attribute type");
        }
    }
}
