// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::*;

use crate::worgen::DataInfo;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ModelSelected>().add_systems(
            EguiPrimaryContextPass,
            example.run_if(resource_exists::<DataInfo>),
        );
    }
}

#[derive(Event)]
pub struct ModelSelected {
    pub archive_path: PathBuf,
    pub model_path: PathBuf,
}

fn example(
    mut contexts: EguiContexts,
    data_info: Res<DataInfo>,
    mut event_writer: EventWriter<ModelSelected>,
) -> Result<()> {
    egui::Window::new("Info")
        .scroll([false, true])
        .show(contexts.ctx_mut()?, |ui| {
            for archive in &data_info.archives {
                egui::CollapsingHeader::new(format!("{}", archive.path.display()))
                    .default_open(false)
                    .enabled(!archive.model_infos.is_empty())
                    .show(ui, |ui| {
                        for model in &archive.model_infos {
                            let e = ui.button(format!("{}", model.path.display()));
                            if e.clicked() {
                                event_writer.write(ModelSelected {
                                    archive_path: archive.path.clone(),
                                    model_path: model.path.clone(),
                                });
                            }
                        }
                    });
            }
        });
    Ok(())
}
