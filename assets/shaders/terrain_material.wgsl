#import bevy_pbr::{
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
}
#endif

struct TerrainMaterial {
    level_count: u32,
};

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
    
    // apply level1 texture
    var alpha = textureSample(
        alpha_texture,
        alpha_sampler,
        in.uv / 2.0
    ).rgb;

    var level1_color = vec4<f32>(0.0);
    var level2_color = vec4<f32>(0.0);
    var level3_color = vec4<f32>(0.0);

    if terrain_material.level_count > 1u {
        level1_color = textureSample(
            level1_texture,
            level1_sampler,
            in.uv
        );
    }
    if terrain_material.level_count > 2u {
        level2_color = textureSample(
            level2_texture,
            level2_sampler,
            in.uv
        );
    }
    if terrain_material.level_count > 3u {
        level3_color = textureSample(
            level3_texture,
            level3_sampler,
            in.uv
        );
    }

    let level0_alpha = 1.0 - (alpha.r + alpha.g + alpha.b);

#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    var out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    out.color = level0_alpha * out.color + alpha.r * level1_color + alpha.g * level2_color + alpha.b * level3_color;

    return out;
}