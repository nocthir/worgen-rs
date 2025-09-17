// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use worgen_rs::{
    data::{archive, world_model},
    settings,
};
use wow_mpq as mpq;

fn bench_read_world_models(c: &mut Criterion) {
    // Load settings from settings.json
    let settings = match settings::load_settings() {
        Ok(s) => s,
        Err(e) => {
            println!("[BENCH] Failed to load settings: {e}");
            return;
        }
    };
    let archive_path = &settings.model_archive_path;
    let mut archive = match mpq::Archive::open(archive_path) {
        Ok(a) => a,
        Err(e) => {
            println!("[BENCH] Failed to open archive: {e}");
            return;
        }
    };
    c.bench_function("read_world_models", |b| {
        b.iter(|| {
            let result = world_model::read_world_models(black_box(&mut archive));
            if let Err(ref e) = result {
                println!("[BENCH] read_world_models error: {e}");
            }
        })
    });
}

fn bench_read_terrain_archive(c: &mut Criterion) {
    // Load settings from settings.json
    let settings = match settings::load_settings() {
        Ok(s) => s,
        Err(e) => {
            println!("[BENCH] Failed to load settings: {e}");
            return;
        }
    };

    c.bench_function("read_terrain", |b| {
        b.iter(|| {
            let result =
                archive::ArchiveInfo::from_archive_path(black_box(&settings.terrain_archive_path));
            if let Err(ref e) = result {
                println!("[BENCH] read_terrain error: {e}");
            }
        })
    });
}

criterion_group!(benches, bench_read_world_models, bench_read_terrain_archive);
criterion_main!(benches);
