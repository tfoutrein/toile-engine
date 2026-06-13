// Toile Engine — Instanced textured-quad shader
// Bind group 0: camera (view-projection matrix)
// Bind group 1: texture + sampler
// Vertex buffer 0: static unit quad (corner ±0.5, uv selector 0/1)
// Vertex buffer 1: per-sprite instance (position, size, rotation, uv rect, color)

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) corner: vec2<f32>,  // ±0.5 around centre
    @location(1) uv_sel: vec2<f32>,  // 0 or 1 per axis
};

struct InstanceInput {
    @location(2) position: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) rotation: f32,
    @location(5) uv_min: vec2<f32>,
    @location(6) uv_max: vec2<f32>,
    @location(7) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn unpack_color(c: u32) -> vec4<f32> {
    return vec4<f32>(
        f32(c & 0xFFu) / 255.0,
        f32((c >> 8u) & 0xFFu) / 255.0,
        f32((c >> 16u) & 0xFFu) / 255.0,
        f32((c >> 24u) & 0xFFu) / 255.0,
    );
}

@vertex
fn vs_main(v: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    // Scale the unit corner to half-extents, rotate, then translate. This matches
    // the previous CPU quad math exactly (rotation: x' = x*cos - y*sin, etc.).
    let local = v.corner * inst.size;
    let cs = cos(inst.rotation);
    let sn = sin(inst.rotation);
    let rotated = vec2<f32>(local.x * cs - local.y * sn, local.x * sn + local.y * cs);
    let world = rotated + inst.position;

    out.clip_position = camera.view_proj * vec4<f32>(world, 0.0, 1.0);
    out.uv = mix(inst.uv_min, inst.uv_max, v.uv_sel);
    out.color = unpack_color(inst.color);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);
    return tex_color * in.color;
}
