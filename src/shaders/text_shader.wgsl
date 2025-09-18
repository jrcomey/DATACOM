struct Uniforms { proj: mat4x4<f32>; };
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) v_uv: vec2<f32>,
    @location(1) v_color: vec4<f32>,
};

@group(1) @binding(0) var glyph_tex: texture_2d<f32>;
@group(1) @binding(1) var glyph_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = uniforms.proj * vec4<f32>(in.position, 0.0, 1.0);
    out.v_uv = in.uv;
    out.v_color = in.color;
    return out;
}

fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(glyph_tex, glyph_sampler, in.v_uv);
    return vec4<f32>(in.v_color.rgb, in.v_color.a * sample.r);
}