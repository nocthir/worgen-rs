// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::collections::HashMap;

use bevy::{
    camera::{CameraOutputMode, Viewport, visibility::RenderLayers},
    prelude::*,
    render::{render_resource::BlendState, view::Hdr},
    window::PrimaryWindow,
};
use bevy_egui::*;
use bevy_inspector_egui::inspector_egui_impls::InspectorEguiImpl;

use crate::{
    assets::{
        material::TerrainMaterial,
        model::{self, Model},
        world_map,
        world_model::{self, WorldModel},
    },
    data::{archive::ArchiveInfoMap, file},
    settings::{self, FileSettings},
};

mod left_panel;
mod right_panel;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FileSelected>()
            .register_type_data::<ArchiveInfoMap, InspectorEguiImpl>()
            .register_type_data::<Model, InspectorEguiImpl>()
            .register_type_data::<WorldModel, InspectorEguiImpl>()
            .register_type_data::<TerrainMaterial, InspectorEguiImpl>()
            .add_systems(Startup, setup_ui)
            .add_systems(EguiPrimaryContextPass, inspector_ui);
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
            // Needed because of https://github.com/bevyengine/bevy/issues/18901
            // and https://github.com/bevyengine/bevy/issues/18903
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        Hdr,
    ));
}

#[derive(Message)]
pub struct FileSelected {
    pub file_path: String,
}

impl FileSelected {
    pub fn new(file_path: String) -> Self {
        info!("File selected: {}", file_path);
        Self { file_path }
    }

    pub fn has_scene_root(&self) -> bool {
        model::is_model_extension(&self.file_path)
            || world_model::is_world_model_extension(&self.file_path)
            || world_map::is_world_map_extension(&self.file_path)
    }

    pub fn get_asset_path(&self) -> String {
        if self.has_scene_root() {
            format!("archive://{}#Root", self.file_path)
        } else {
            format!("archive://{}", self.file_path)
        }
    }
}

impl From<&FileSettings> for FileSelected {
    fn from(settings: &FileSettings) -> Self {
        Self {
            file_path: settings.file_path.clone(),
        }
    }
}

pub fn select_default_model(mut event_writer: MessageWriter<FileSelected>) {
    if let Some(default_model_path) = settings::Settings::get().test_model_path.clone() {
        event_writer.write(FileSelected::new(default_model_path));
    }
}

pub fn inspector_ui(world: &mut World) -> Result<()> {
    let _image_map = get_image_map(world);

    let mut ctx = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single_mut(world)?;

    use std::ops::DerefMut;
    let mut egui_context = ctx.deref_mut().clone();

    let left_panel_response = left_panel::ui(world, &mut egui_context);

    let right_panel_response = right_panel::ui(world, &mut egui_context);

    adjust_viewport(&left_panel_response, &right_panel_response, world)
}

fn adjust_viewport(
    left_panel_response: &egui::InnerResponse<()>,
    right_panel_response: &egui::InnerResponse<()>,
    world: &mut World,
) -> Result<()> {
    // Adjust world camera viewport so scene starts to the right of the panel (avoid rendering beneath UI)
    let left_width_logical = left_panel_response.response.rect.width();

    let Ok(window) = world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .single_mut(world)
    else {
        warn!("No primary window found");
        return Ok(());
    };

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
    let mut camera = world
        .query_filtered::<&mut Camera, Without<EguiContext>>()
        .single_mut(world)?;
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

fn get_image_map(world: &mut World) -> HashMap<Handle<Image>, egui::TextureId> {
    let mut image_handles = Vec::new();
    for model in world.query::<&Model>().iter(world) {
        image_handles.extend(model.images.iter().cloned());
    }
    for world_model in world.query::<&WorldModel>().iter(world) {
        image_handles.extend(world_model.images.iter().cloned());
    }
    for terrain_material in world.query::<&TerrainMaterial>().iter(world) {
        image_handles.push(terrain_material.alpha_texture.clone());
        if let Some(handle) = terrain_material.level1_texture.clone() {
            image_handles.push(handle);
        }
        if let Some(handle) = terrain_material.level2_texture.clone() {
            image_handles.push(handle);
        }
        if let Some(handle) = terrain_material.level3_texture.clone() {
            image_handles.push(handle);
        }
    }

    let mut image_map = HashMap::new();
    let mut textures = world.get_resource_mut::<EguiUserTextures>().unwrap();
    for image in image_handles {
        if !image_map.contains_key(&image) {
            let egui_handle = EguiTextureHandle::Strong(image.clone());
            let tex_id = textures.add_image(egui_handle);
            image_map.insert(image.clone(), tex_id);
        }
    }

    image_map
}

fn get_file_icon(data_type: &file::DataType) -> &'static str {
    match data_type {
        file::DataType::Texture(_) => "🖼",
        file::DataType::Model(_) => "📦",
        file::DataType::WorldModel(_) => "🏰",
        file::DataType::WorldMap(_) => "🗺",
        file::DataType::DataBase(_) => "📚",
        file::DataType::Unknown => "❓",
    }
}
