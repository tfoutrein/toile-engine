// Toile Engine — Shadow map builder (1D per light)
//
// Builds one row of the shadow texture array by ray-marching through the scene.
// Render this with viewport(0, 0, resolution, 1) per shadow-casting light.
//
// Occlusion convention: scene pixels with alpha > 0.5 are solid occluders.
// The background must be cleared with alpha = 0 so it does not block light.

const PI: f32 = 3.14159265358979;
const OCCLUDER_ALPHA: f32 = 0.5;

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;

struct ShadowBuildParams {
    light_pos:     vec2<f32>,   // world space             offset  0
    light_radius:  f32,         //                          offset  8
    resolution:    f32,         // shadow map width         offset 12
    camera_pos:    vec2<f32>,   // camera world centre      offset 16
    viewport_half: vec2<f32>,   // world-unit half-extents  offset 24
    steps:         u32,         // ray march steps          offset 32
    start_frac:    f32,         // skip [0, start_frac) near the light  offset 36
    _pad1:         u32,         //                          offset 40
    _pad2:         u32,         //                          offset 44
};

@group(1) @binding(0) var<uniform> p: ShadowBuildParams;

struct VsOut { @builtin(position) pos: vec4<f32> }

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pts = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return VsOut(vec4(pts[vi], 0.0, 1.0));
}

/// World → UV (Y-up camera, Y-down UV).
fn world_to_uv(world: vec2<f32>) -> vec2<f32> {
    let ndc = (world - p.camera_pos) / p.viewport_half;
    return vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
}

@fragment fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    // frag.x in [0, resolution) — one column = one angle
    let angle = (frag.x / p.resolution) * 2.0 * PI;
    let dir   = vec2(cos(angle), sin(angle));

    var closest = 1.0;  // default: no occluder within radius
    // Distribute steps evenly over [start_frac, 1.0] to skip the area
    // immediately around the light source (avoids self-occlusion by glow sprites).
    let range = 1.0 - p.start_frac;
    for (var s: u32 = 1u; s <= p.steps; s++) {
        let t       = p.start_frac + (f32(s) / f32(p.steps)) * range;
        let world   = p.light_pos + dir * (t * p.light_radius);
        let uv      = world_to_uv(world);

        // Ignore samples outside the screen
        if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 { continue; }

        let texel = textureSampleLevel(t_scene, s_scene, uv, 0.0);
        if texel.a > OCCLUDER_ALPHA {
            closest = t;
            break;
        }
    }

    return vec4(closest, 0.0, 0.0, 1.0);
}
