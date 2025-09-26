// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::*,
    tasks::{self, Task},
};
use wow_blp as blp;

use crate::data::{archive, file};
