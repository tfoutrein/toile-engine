// Toile Engine — Post-processing: CRT (scanlines + barrel distortion + chromatic aberration)
@group(0) @binding(0) var t_screen: texture_2d<f32>;
@group(0) @binding(1) var s_screen: sampler;

struct Params {
    scanline_intensity: f32,
    curvature: f32,
    chromatic_aberration: f32,
    screen_height: f32,
};
@group(1) @binding(0) var<uniform> p: Params;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var uvs = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0),  vec2<f32>(2.0, 1.0),  vec2<f32>(0.0, -1.0));
    return VsOut(vec4<f32>(pos[vi], 0.0, 1.0), uvs[vi]);
}

fn barrel(uv: vec2<f32>, amount: f32) -> vec2<f32> {
    var uv2 = uv * 2.0 - 1.0;
    let r2 = dot(uv2, uv2);
    uv2 = uv2 * (1.0 + amount * r2);
    return uv2 * 0.5 + 0.5;
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var uv = in.uv;

    // Barrel distortion
    if p.curvature > 0.0 {
        uv = barrel(uv, p.curvature);
        if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    }

    // Chromatic aberration
    let ca = p.chromatic_aberration * 0.008;
    let r = textureSample(t_screen, s_screen, uv + vec2<f32>(ca, 0.0)).r;
    let g = textureSample(t_screen, s_screen, uv).g;
    let b = textureSample(t_screen, s_screen, uv - vec2<f32>(ca, 0.0)).b;
    var color = vec4<f32>(r, g, b, 1.0);

    // Scanlines (one per screen pixel row)
    let scan = 0.5 + 0.5 * sin(uv.y * p.screen_height * 3.14159265);
    let scan_factor = 1.0 - p.scanline_intensity * (1.0 - scan);
    color = vec4<f32>(color.rgb * scan_factor, 1.0);

    // Edge darkening (screen bezel effect)
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let bezel = smoothstep(0.0, 0.03, edge);
    return vec4<f32>(color.rgb * bezel, 1.0);
}
