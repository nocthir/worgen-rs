// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::ecs::error::Result;
use bevy_egui::*;

pub fn example(mut contexts: EguiContexts) -> Result<()> {
    let label = egui::Label::new("Test");
    egui::Window::new("Info").show(contexts.ctx_mut()?, |ui| {
        ui.add(label);
    });
    Ok(())
}
