// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;

pub struct WorgenStatePlugin;

impl Plugin for WorgenStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WorgenState>();
    }
}

#[derive(States, Default, Debug, Hash, PartialEq, Eq, Clone, Reflect)]
pub enum WorgenState {
    #[default]
    Loading,
    Ready,
}
