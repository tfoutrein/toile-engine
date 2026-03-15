// Toile Engine — Post-processing: screen shake (UV offset)
@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct Params { offset_x: f32, offset_y: f32, _p0: f32, _p1: f32 };
@group(1) @binding(0) var<uniform> p: Params;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var uvs = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0),  vec2<f32>(2.0, 1.0),  vec2<f32>(0.0, -1.0));
    return VsOut(vec4<f32>(pos[vi], 0.0, 1.0), uvs[vi]);
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv + vec2<f32>(p.offset_x, p.offset_y);
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(t_screen, s_screen, uv);
}
