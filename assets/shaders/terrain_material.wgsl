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

@group(2) @binding(100)
var level1_texture: texture_2d<f32>;
@group(2) @binding(101)
var level1_sampler: sampler;
@group(2) @binding(102)
var level1_alpha_texture: texture_2d<f32>;
@group(2) @binding(103)
var level1_alpha_sampler: sampler;

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
    let level1_alpha = textureSample(
        level1_alpha_texture,
        level1_alpha_sampler,
        in.uv / 2.0
    ).r;

    let level0_alpha = 1.0 - level1_alpha;

    let level1_color = textureSample(
        level1_texture,
        level1_sampler,
        in.uv
    );

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

    out.color = level0_alpha * out.color + level1_alpha * level1_color;

    return out;
}