// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_egui::EguiPrimaryContextPass;
use bevy_egui::egui;

use crate::state::GameState;

#[derive(Default)]
pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            show_loading_message.run_if(in_state(GameState::Loading)),
        );
    }
}

fn show_loading_message(mut contexts: EguiContexts) {
    egui::Window::new("Loading")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut().unwrap(), |ui| {
            ui.label("Loading, please wait...");
        });
}
