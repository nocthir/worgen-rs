#import bevy_pbr::{
    pbr_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

struct TerrainSettings {
    level0: bool,
    level1: bool,
    level2: bool,
    level3: bool,
}

struct TerrainMaterial {
    level_count: u32,
};

@group(2) @binding(69) var<uniform> level_mask: u32;
@group(2) @binding(70) var<uniform> terrain_material: TerrainMaterial;

@group(2) @binding(71) var alpha_texture: texture_2d<f32>;
@group(2) @binding(72) var alpha_sampler: sampler;
@group(2) @binding(73) var level1_texture: texture_2d<f32>;
@group(2) @binding(74) var level1_sampler: sampler;
@group(2) @binding(75) var level2_texture: texture_2d<f32>;
@group(2) @binding(76) var level2_sampler: sampler;
@group(2) @binding(77) var level3_texture: texture_2d<f32>;
@group(2) @binding(78) var level3_sampler: sampler;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    
    // alpha texture
    var alpha = textureSample(
        alpha_texture,
        alpha_sampler,
        in.uv
    ).rgb;

    var level0_color = vec4<f32>(0.0);
    var level1_color = vec4<f32>(0.0);
    var level2_color = vec4<f32>(0.0);
    var level3_color = vec4<f32>(0.0);

    let level0_mask = level_mask & 1u;
    let level1_mask = (level_mask >> 1u) & 1u;
    let level2_mask = (level_mask >> 2u) & 1u;
    let level3_mask = (level_mask >> 3u) & 1u;

    if level0_mask != 0 && terrain_material.level_count > 0u {
        level0_color = pbr_input.material.base_color;
    }

    if level1_mask != 0 && terrain_material.level_count > 1u {
        level1_color = textureSample(
            level1_texture,
            level1_sampler,
            in.uv
        );
        level0_color = (1.0 - alpha.r) * level0_color;
        level1_color = alpha.r * level1_color;
    }
    if level2_mask != 0 && terrain_material.level_count > 2u {
        level2_color = textureSample(
            level2_texture,
            level2_sampler,
            in.uv
        );
        level0_color = (1.0 - alpha.g) * level0_color;
        level1_color = (1.0 - alpha.g) * level1_color;
        level2_color = alpha.g * level2_color;
    }
    if level3_mask != 0 && terrain_material.level_count > 3u {
        level3_color = textureSample(
            level3_texture,
            level3_sampler,
            in.uv
        );
        level0_color= (1.0 - alpha.b) * level0_color;
        level1_color= (1.0 - alpha.b) * level1_color;
        level2_color= (1.0 - alpha.b) * level2_color;
        level3_color = alpha.b * level3_color;
    }

    pbr_input.material.base_color = level0_color + level1_color + level2_color + level3_color;

#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    var out = deferred_output(in, pbr_input);
#else
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}