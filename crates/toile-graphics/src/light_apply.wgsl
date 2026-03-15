// Toile Engine — 2D point lighting pass
// Group 0: scene texture  (shared layout with post-processing)
// Group 1: lights uniform  (ambient + camera + up to 64 point lights)

const MAX_LIGHTS: u32 = 64u;

@group(0) @binding(0) var t_scene: texture_2d<f32>;
@group(0) @binding(1) var s_scene: sampler;

struct LightEntry {
    position:  vec2<f32>,  // world space
    radius:    f32,        // world units
    falloff:   f32,        // 1=linear, 2=smooth quadratic
    color:     vec3<f32>,  // linear RGB
    intensity: f32,
};

struct LightsUniform {
    ambient:       vec4<f32>,           // rgb + intensity           offset   0
    camera_pos:    vec2<f32>,           // world space centre        offset  16
    viewport_half: vec2<f32>,           // half-extents in world     offset  24
    light_count:   u32,                 //                           offset  32
    _pad0:         u32,                 //                           offset  36
    _pad1:         u32,                 //                           offset  40
    _pad2:         u32,                 //                           offset  44
    lights:        array<LightEntry, 64>, //                         offset  48
};
@group(1) @binding(0) var<uniform> u: LightsUniform;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    var uvs = array<vec2<f32>, 3>(vec2<f32>(0.0, 1.0), vec2<f32>(2.0, 1.0), vec2<f32>(0.0,-1.0));
    return VsOut(vec4<f32>(pos[vi], 0.0, 1.0), uvs[vi]);
}

// Screen UV [0,1]² → world space (Y-up, camera-relative)
fn uv_to_world(uv: vec2<f32>) -> vec2<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return u.camera_pos + ndc * u.viewport_half;
}

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(t_scene, s_scene, in.uv);
    let world = uv_to_world(in.uv);

    // Start from ambient
    var light_acc = u.ambient.rgb * u.ambient.a;

    for (var i: u32 = 0u; i < min(u.light_count, MAX_LIGHTS); i++) {
        let L   = u.lights[i];
        let d   = length(L.position - world);
        if d < L.radius {
            let t   = 1.0 - d / L.radius;
            let att = pow(t, L.falloff);
            light_acc += L.color * (L.intensity * att);
        }
    }

    return vec4<f32>(scene.rgb * clamp(light_acc, vec3<f32>(0.0), vec3<f32>(1.0)), scene.a);
}
