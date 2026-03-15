// Toile Engine — 2D point lighting + shadow pass
// Group 0: scene texture  (shared layout with post-processing)
// Group 1: lights uniform  (ambient + camera + up to 64 point lights)
// Group 2: shadow map texture array  (one layer per shadow-casting light, R32Float)

const MAX_LIGHTS:        u32 = 64u;
const MAX_SHADOW_LIGHTS: i32 = 8;
const PI:                f32 = 3.14159265358979;
const SHADOW_BIAS:       f32 = 0.02;   // prevents self-shadowing
const PCF_HALF:          i32 = 2;      // 2*PCF_HALF+1 = 5 samples

@group(0) @binding(0) var t_scene:  texture_2d<f32>;
@group(0) @binding(1) var s_scene:  sampler;

// ── LightEntry — 48 bytes, align 16, stride 48 ───────────────────────────────
struct LightEntry {
    position:    vec2<f32>,   // offset  0  (align 8)
    radius:      f32,         // offset  8
    falloff:     f32,         // offset 12
    color:       vec3<f32>,   // offset 16  (vec3 aligns to 16 in WGSL uniform)
    intensity:   f32,         // offset 28
    cast_shadow: u32,         // offset 32  (0 = no shadow)
    shadow_row:  i32,         // offset 36  (-1 = none, 0..7 = layer index)
    _pad0:       u32,         // offset 40
    _pad1:       u32,         // offset 44
};

// ── LightsUniform — 48-byte header + 64 × 48 = 3120 bytes total ──────────────
struct LightsUniform {
    ambient:           vec4<f32>,              // offset   0
    camera_pos:        vec2<f32>,              // offset  16
    viewport_half:     vec2<f32>,              // offset  24
    light_count:       u32,                    // offset  32
    shadow_resolution: u32,                    // offset  36
    _pad0:             u32,                    // offset  40
    _pad1:             u32,                    // offset  44
    lights:            array<LightEntry, 64>,  // offset  48
};

@group(1) @binding(0) var<uniform> u: LightsUniform;

// Shadow map array: layer i = 1D shadow distances for shadow light i.
// Accessed with textureLoad (no sampler needed — non-filtered read).
@group(2) @binding(0) var t_shadow: texture_2d_array<f32>;

// ── Vertex shader ─────────────────────────────────────────────────────────────

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));
    var uvs = array<vec2<f32>, 3>(vec2(0.0,1.0),   vec2(2.0,1.0),  vec2(0.0,-1.0));
    return VsOut(vec4(pos[vi], 0.0, 1.0), uvs[vi]);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn uv_to_world(uv: vec2<f32>) -> vec2<f32> {
    let ndc = vec2(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return u.camera_pos + ndc * u.viewport_half;
}

/// PCF shadow factor: 1.0 = fully lit, 0.0 = fully in shadow.
fn shadow_factor(row: i32, light_pos: vec2<f32>, world_pos: vec2<f32>, light_radius: f32) -> f32 {
    let offset    = world_pos - light_pos;
    let frag_dist = length(offset) / light_radius;

    // Map angle [-π, π] → [0, 1) for texture lookup
    let angle   = atan2(offset.y, offset.x);
    let u_coord = fract(angle / (2.0 * PI) + 1.0);

    let res    = i32(u.shadow_resolution);
    let center = i32(u_coord * f32(res)) % res;

    var lit = 0.0;
    for (var k = -PCF_HALF; k <= PCF_HALF; k++) {
        let idx         = ((center + k) % res + res) % res;
        let shadow_dist = textureLoad(t_shadow, vec2<i32>(idx, 0), row, 0).r;
        if frag_dist <= shadow_dist + SHADOW_BIAS {
            lit += 1.0;
        }
    }
    return lit / f32(2 * PCF_HALF + 1);
}

// ── Fragment shader ───────────────────────────────────────────────────────────

@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(t_scene, s_scene, in.uv);
    let world = uv_to_world(in.uv);

    // Start from ambient
    var light_acc = u.ambient.rgb * u.ambient.a;

    for (var i: u32 = 0u; i < min(u.light_count, MAX_LIGHTS); i++) {
        let L = u.lights[i];
        let d = length(L.position - world);
        if d < L.radius {
            let t   = 1.0 - d / L.radius;
            let att = pow(t, L.falloff);
            var contrib = L.color * (L.intensity * att);

            // Shadow attenuation
            if L.cast_shadow != 0u && L.shadow_row >= 0 {
                contrib *= shadow_factor(L.shadow_row, L.position, world, L.radius);
            }

            light_acc += contrib;
        }
    }

    return vec4(scene.rgb * clamp(light_acc, vec3(0.0), vec3(1.0)), scene.a);
}
