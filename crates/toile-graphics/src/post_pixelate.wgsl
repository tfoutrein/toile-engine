// Toile Engine — Post-processing: pixelate
@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct Params { pixel_size: f32, screen_w: f32, screen_h: f32, _p: f32 };
@group(1) @binding(0) var<uniform> p: Params;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var uvs = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0),  vec2<f32>(2.0, 1.0),  vec2<f32>(0.0, -1.0));
    return VsOut(vec4<f32>(pos[vi], 0.0, 1.0), uvs[vi]);
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let pw = p.pixel_size / p.screen_w;
    let ph = p.pixel_size / p.screen_h;
    let uv = vec2<f32>(
        floor(in.uv.x / pw) * pw + pw * 0.5,
        floor(in.uv.y / ph) * ph + ph * 0.5,
    );
    return textureSample(t_screen, s_screen, uv);
}
