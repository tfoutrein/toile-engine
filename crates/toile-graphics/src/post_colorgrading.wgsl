// Toile Engine — Post-processing: color grading (saturation, brightness, contrast)
@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct Params { saturation: f32, brightness: f32, contrast: f32, _p: f32 };
@group(1) @binding(0) var<uniform> p: Params;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var uvs = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0),  vec2<f32>(2.0, 1.0),  vec2<f32>(0.0, -1.0));
    return VsOut(vec4<f32>(pos[vi], 0.0, 1.0), uvs[vi]);
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var c = textureSample(t_screen, s_screen, in.uv);

    // Brightness
    c = vec4<f32>(c.rgb * p.brightness, c.a);

    // Contrast (pivot at 0.5)
    c = vec4<f32>((c.rgb - 0.5) * p.contrast + 0.5, c.a);

    // Saturation
    let lum = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = vec4<f32>(mix(vec3<f32>(lum), c.rgb, p.saturation), c.a);

    return clamp(c, vec4<f32>(0.0), vec4<f32>(1.0));
}
