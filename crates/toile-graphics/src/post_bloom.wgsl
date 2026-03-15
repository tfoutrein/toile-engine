// Toile Engine — Post-processing: bloom (5x5 threshold + blur, single pass)
@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct Params { threshold: f32, intensity: f32, radius: f32, _p: f32 };
@group(1) @binding(0) var<uniform> p: Params;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var uvs = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0),  vec2<f32>(2.0, 1.0),  vec2<f32>(0.0, -1.0));
    return VsOut(vec4<f32>(pos[vi], 0.0, 1.0), uvs[vi]);
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(t_screen, s_screen, in.uv);
    let r = p.radius;
    var glow = vec3<f32>(0.0);

    // 5x5 kernel in UV space
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * r * 0.5;
            let s = textureSample(t_screen, s_screen, in.uv + offset).rgb;
            let bright = max(s - vec3<f32>(p.threshold), vec3<f32>(0.0));
            glow += bright;
        }
    }
    glow /= 25.0;

    return vec4<f32>(base.rgb + glow * p.intensity, base.a);
}
