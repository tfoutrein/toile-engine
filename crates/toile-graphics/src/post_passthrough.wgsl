// Toile Engine — Post-processing: passthrough (plain blit)
@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var u = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0),  vec2<f32>(2.0, 1.0),  vec2<f32>(0.0, -1.0));
    return VsOut(vec4<f32>(p[vi], 0.0, 1.0), u[vi]);
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(t_screen, s_screen, in.uv);
}
