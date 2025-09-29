// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod image;
pub mod material;
pub mod mesh;
pub mod model;
pub mod root_aabb;
pub mod world_map;
pub mod world_model;

use image::*;
use material::*;
use mesh::*;
use model::*;
use root_aabb::*;
use world_map::*;
use world_model::*;

use bevy::prelude::*;

pub struct WorgenAssetPlugin;

impl Plugin for WorgenAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RootAabb>()
            .register_type::<Model>()
            .register_type::<WorldModel>()
            .register_type::<WorldMap>()
            .init_asset::<ModelAsset>()
            .init_asset::<WorldModelAsset>()
            .init_asset::<WorldMapAsset>()
            .init_asset_loader::<ImageLoader>()
            .init_asset_loader::<ModelAssetLoader>()
            .init_asset_loader::<WorldModelAssetLoader>()
            .init_asset_loader::<WorldMapAssetLoader>()
            .add_plugins(MaterialPlugin::<ExtTerrainMaterial>::default())
            .add_systems(PreStartup, archive::FileArchiveMap::init);
    }
}

#[cfg(test)]
pub mod test {
    use std::time::Duration;

    use crate::settings::Settings;
    use bevy::*;

    use super::*;

    pub fn test_app() -> App {
        Settings::init();
        archive::FileArchiveMap::init();

        let mut app = App::new();

        app.add_plugins((
            archive::ArchiveAssetReaderPlugin,
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    // Don't automatically exit due to having no windows.
                    exit_condition: window::ExitCondition::DontExit,
                    ..default()
                })
                // WinitPlugin will panic in environments without a display server.
                .disable::<winit::WinitPlugin>()
                .set(render::RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    render_creation: render::settings::RenderCreation::Automatic(
                        render::settings::WgpuSettings {
                            backends: None,
                            ..default()
                        },
                    ),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                }),
            // ScheduleRunnerPlugin provides an alternative to the default bevy_winit app runner, which
            // manages the loop without creating a window.
            app::ScheduleRunnerPlugin::run_loop(
                // Run 60 times per second.
                Duration::from_secs_f64(1.0 / 60.0),
            ),
            WorgenAssetPlugin,
        ));

        app.finish();
        app.cleanup();

        app
    }

    const LARGE_ITERATION_COUNT: usize = 10000;

    pub fn run_app_until(app: &mut App, mut predicate: impl FnMut(&mut World) -> Option<()>) {
        for _ in 0..LARGE_ITERATION_COUNT {
            app.update();
            if predicate(app.world_mut()).is_some() {
                return;
            }
        }

        panic!("Ran out of loops to return `Some` from `predicate`");
    }
}
