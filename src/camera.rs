// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;

pub fn setup_camera(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the box
    let red_box = GltfAssetLabel::Scene(0).from_asset("box/box.gltf");
    let red_box_asset = asset_server.load(red_box);
    commands.spawn(SceneRoot(red_box_asset));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
