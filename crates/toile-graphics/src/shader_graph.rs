// Toile Engine — Shader Graph: data model + WGSL compiler (ADR-027)
//
// A `ShaderGraph` is a directed acyclic graph of nodes. Each node transforms
// typed values (Float / Vec2 / Vec4).  `compile()` traverses the graph
// topologically and emits a complete WGSL fragment shader.
//
// Shader layout (compiled output):
//   @group(0) = source texture  (shared with PostProcessor's tex_bgl)
//   @group(1) = CustomParams uniform  { time, _pad, screen_w, screen_h }

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

// ── Value types ───────────────────────────────────────────────────────────────

/// The WGSL value type that flows through a node port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    /// `f32`
    Float,
    /// `vec2<f32>`
    Vec2,
    /// `vec4<f32>`
    Vec4,
}

impl PortType {
    fn wgsl_ty(self) -> &'static str {
        match self {
            Self::Float => "f32",
            Self::Vec2  => "vec2<f32>",
            Self::Vec4  => "vec4<f32>",
        }
    }

    fn default_val(self) -> &'static str {
        match self {
            Self::Float => "0.0",
            Self::Vec2  => "vec2<f32>(0.0, 0.0)",
            Self::Vec4  => "vec4<f32>(0.0, 0.0, 0.0, 1.0)",
        }
    }
}

// ── Node kinds ────────────────────────────────────────────────────────────────

/// All node types available in the shader graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeKind {
    // ─ Source nodes ───────────────────────────────────────────────────────────
    /// → Vec2: the fragment UV coordinate.
    UV,
    /// → Float: elapsed time in seconds.
    Time,
    /// → Vec2: (screen_width, screen_height) in pixels.
    ScreenSize,
    /// (uv: Vec2) → Vec4: sample the source (scene) texture.
    SceneColor,
    ConstF32(f32),
    ConstVec2([f32; 2]),
    ConstVec4([f32; 4]),

    // ─ Float math ─────────────────────────────────────────────────────────────
    AddF, SubF, MulF, DivF,
    /// (base: Float, exp: Float) → Float
    Power,
    /// (a: Float, b: Float, t: Float) → Float = mix(a,b,t)
    LerpF,
    /// (lo: Float, hi: Float, x: Float) → Float
    Smoothstep,

    // ─ Unary float ────────────────────────────────────────────────────────────
    Sin, Cos, Abs, Fract, Floor,

    // ─ Vec2 math ──────────────────────────────────────────────────────────────
    AddV2, SubV2, MulV2,
    /// (scale: Float, v: Vec2) → Vec2
    MulFV2,

    // ─ Vec4 math ──────────────────────────────────────────────────────────────
    AddV4, MulV4,
    /// (scale: Float, v: Vec4) → Vec4
    MulFV4,
    /// (a: Vec4, b: Vec4, t: Float) → Vec4 = mix(a,b,t)
    LerpV4,

    // ─ Vector constructors / deconstructors ───────────────────────────────────
    /// (v: Vec2) → (x: Float, y: Float)
    SplitVec2,
    /// (v: Vec4) → (r: Float, g: Float, b: Float, a: Float)
    SplitVec4,
    /// (x: Float, y: Float) → Vec2
    CombineVec2,
    /// (r: Float, g: Float, b: Float, a: Float) → Vec4
    CombineVec4,
    /// (v: Vec2) → Float
    Length,
    /// (a: Vec2, b: Vec2) → Float
    Distance,
    /// (v: Vec2) → Vec2
    Normalize,

    // ─ Noise ──────────────────────────────────────────────────────────────────
    /// (p: Vec2) → Float: fast pseudo-random hash [0,1].
    Hash,
    /// (p: Vec2, scale: Float) → Float: smooth value noise [0,1].
    ValueNoise,

    // ─ SDF ────────────────────────────────────────────────────────────────────
    /// (p: Vec2, radius: Float) → Float: signed distance to circle.
    SDFCircle,
    /// (p: Vec2, half_size: Vec2) → Float: signed distance to box.
    SDFBox,

    // ─ Color ──────────────────────────────────────────────────────────────────
    /// (h: Float[0-360], s: Float[0-1], v: Float[0-1]) → Vec4
    HSVtoRGB,

    // ─ Output ─────────────────────────────────────────────────────────────────
    /// (rgba: Vec4): terminal node — its input is the final fragment color.
    FragmentColor,
}

// ── Graph data model ──────────────────────────────────────────────────────────

/// A node in the shader graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderNode {
    pub id: u32,
    pub kind: NodeKind,
    /// Visual position in the node editor.
    pub position: [f32; 2],
}

/// A directed edge: output port `from_port` of `from_node` → input port
/// `to_port` of `to_node`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderEdge {
    pub from_node: u32,
    pub from_port: usize,
    pub to_node: u32,
    pub to_port: usize,
}

/// A complete shader graph (serializable to JSON).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShaderGraph {
    pub name: String,
    pub nodes: Vec<ShaderNode>,
    pub edges: Vec<ShaderEdge>,
}

impl ShaderGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), nodes: vec![], edges: vec![] }
    }

    pub fn add_node(&mut self, id: u32, kind: NodeKind) -> u32 {
        self.nodes.push(ShaderNode { id, kind, position: [0.0; 2] });
        id
    }

    pub fn connect(&mut self, from: u32, from_port: usize, to: u32, to_port: usize) {
        self.edges.push(ShaderEdge { from_node: from, from_port, to_node: to, to_port });
    }

    /// Compile this graph to a complete WGSL shader string.
    pub fn compile(&self) -> Result<String, String> {
        compile(self)
    }

    /// Serialize the graph to a pretty JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

// ── Port type definitions ─────────────────────────────────────────────────────

/// Returns (input_port_types, output_port_types) for a given node kind.
fn port_types(kind: &NodeKind) -> (Vec<PortType>, Vec<PortType>) {
    use NodeKind::*;
    use PortType::*;
    match kind {
        // Sources
        UV          => (vec![], vec![Vec2]),
        Time        => (vec![], vec![Float]),
        ScreenSize  => (vec![], vec![Vec2]),
        SceneColor  => (vec![Vec2], vec![Vec4]),
        ConstF32(_) => (vec![], vec![Float]),
        ConstVec2(_)=> (vec![], vec![Vec2]),
        ConstVec4(_)=> (vec![], vec![Vec4]),

        // Float math (a, b) → Float
        AddF | SubF | MulF | DivF => (vec![Float, Float], vec![Float]),
        Power      => (vec![Float, Float], vec![Float]),
        LerpF      => (vec![Float, Float, Float], vec![Float]),
        Smoothstep => (vec![Float, Float, Float], vec![Float]),

        // Unary float
        Sin | Cos | Abs | Fract | Floor => (vec![Float], vec![Float]),

        // Vec2 math
        AddV2 | SubV2 | MulV2 => (vec![Vec2, Vec2], vec![Vec2]),
        MulFV2 => (vec![Float, Vec2], vec![Vec2]),

        // Vec4 math
        AddV4 | MulV4 => (vec![Vec4, Vec4], vec![Vec4]),
        MulFV4 => (vec![Float, Vec4], vec![Vec4]),
        LerpV4 => (vec![Vec4, Vec4, Float], vec![Vec4]),

        // Vector ops
        SplitVec2   => (vec![Vec2], vec![Float, Float]),
        SplitVec4   => (vec![Vec4], vec![Float, Float, Float, Float]),
        CombineVec2 => (vec![Float, Float], vec![Vec2]),
        CombineVec4 => (vec![Float, Float, Float, Float], vec![Vec4]),
        Length      => (vec![Vec2], vec![Float]),
        Distance    => (vec![Vec2, Vec2], vec![Float]),
        Normalize   => (vec![Vec2], vec![Vec2]),

        // Noise
        Hash       => (vec![Vec2], vec![Float]),
        ValueNoise => (vec![Vec2, Float], vec![Float]),

        // SDF
        SDFCircle => (vec![Vec2, Float], vec![Float]),
        SDFBox    => (vec![Vec2, Vec2], vec![Float]),

        // Color
        HSVtoRGB => (vec![Float, Float, Float], vec![Vec4]),

        // Output
        FragmentColor => (vec![Vec4], vec![]),
    }
}

// ── Code emitter ──────────────────────────────────────────────────────────────

/// Emit WGSL `let` statements for a node.
///
/// Returns `Vec<(var_name, output_type, wgsl_expr)>` — one entry per output port.
fn emit_node(id: u32, kind: &NodeKind, inputs: &[String]) -> Vec<(String, PortType, String)> {
    use NodeKind::*;
    use PortType::*;

    macro_rules! o {
        ($p:expr, $ty:expr, $e:expr) => {
            (format!("n{}_{}", id, $p), $ty, $e.to_string())
        };
    }

    match kind {
        // Sources
        UV          => vec![o!(0, Vec2, "in.uv")],
        Time        => vec![o!(0, Float, "p.time")],
        ScreenSize  => vec![o!(0, Vec2, "vec2<f32>(p.screen_w, p.screen_h)")],
        SceneColor  => vec![o!(0, Vec4, format!("textureSample(t_src, s_src, {})", inputs[0]))],
        ConstF32(v) => vec![o!(0, Float, format!("{:.6}", v))],
        ConstVec2([x, y]) => vec![o!(0, Vec2,
            format!("vec2<f32>({:.6}, {:.6})", x, y))],
        ConstVec4([r, g, b, a]) => vec![o!(0, Vec4,
            format!("vec4<f32>({:.6}, {:.6}, {:.6}, {:.6})", r, g, b, a))],

        // Float math
        AddF  => vec![o!(0, Float, format!("{} + {}", inputs[0], inputs[1]))],
        SubF  => vec![o!(0, Float, format!("{} - {}", inputs[0], inputs[1]))],
        MulF  => vec![o!(0, Float, format!("{} * {}", inputs[0], inputs[1]))],
        DivF  => vec![o!(0, Float, format!("{} / {}", inputs[0], inputs[1]))],
        Power => vec![o!(0, Float, format!("pow({}, {})", inputs[0], inputs[1]))],
        LerpF => vec![o!(0, Float, format!("mix({}, {}, {})", inputs[0], inputs[1], inputs[2]))],
        Smoothstep => vec![o!(0, Float,
            format!("smoothstep({}, {}, {})", inputs[0], inputs[1], inputs[2]))],

        // Unary float
        Sin   => vec![o!(0, Float, format!("sin({})", inputs[0]))],
        Cos   => vec![o!(0, Float, format!("cos({})", inputs[0]))],
        Abs   => vec![o!(0, Float, format!("abs({})", inputs[0]))],
        Fract => vec![o!(0, Float, format!("fract({})", inputs[0]))],
        Floor => vec![o!(0, Float, format!("floor({})", inputs[0]))],

        // Vec2 math
        AddV2  => vec![o!(0, Vec2, format!("{} + {}", inputs[0], inputs[1]))],
        SubV2  => vec![o!(0, Vec2, format!("{} - {}", inputs[0], inputs[1]))],
        MulV2  => vec![o!(0, Vec2, format!("{} * {}", inputs[0], inputs[1]))],
        MulFV2 => vec![o!(0, Vec2, format!("{} * {}", inputs[0], inputs[1]))],

        // Vec4 math
        AddV4  => vec![o!(0, Vec4, format!("{} + {}", inputs[0], inputs[1]))],
        MulV4  => vec![o!(0, Vec4, format!("{} * {}", inputs[0], inputs[1]))],
        MulFV4 => vec![o!(0, Vec4, format!("{} * {}", inputs[0], inputs[1]))],
        LerpV4 => vec![o!(0, Vec4,
            format!("mix({}, {}, {})", inputs[0], inputs[1], inputs[2]))],

        // Vector ops
        SplitVec2 => vec![
            o!(0, Float, format!("{}.x", inputs[0])),
            o!(1, Float, format!("{}.y", inputs[0])),
        ],
        SplitVec4 => vec![
            o!(0, Float, format!("{}.r", inputs[0])),
            o!(1, Float, format!("{}.g", inputs[0])),
            o!(2, Float, format!("{}.b", inputs[0])),
            o!(3, Float, format!("{}.a", inputs[0])),
        ],
        CombineVec2 => vec![o!(0, Vec2,
            format!("vec2<f32>({}, {})", inputs[0], inputs[1]))],
        CombineVec4 => vec![o!(0, Vec4,
            format!("vec4<f32>({}, {}, {}, {})", inputs[0], inputs[1], inputs[2], inputs[3]))],
        Length    => vec![o!(0, Float, format!("length({})", inputs[0]))],
        Distance  => vec![o!(0, Float,
            format!("distance({}, {})", inputs[0], inputs[1]))],
        Normalize => vec![o!(0, Vec2, format!("normalize({})", inputs[0]))],

        // Noise
        Hash       => vec![o!(0, Float, format!("sg_hash({})", inputs[0]))],
        ValueNoise => vec![o!(0, Float,
            format!("sg_value_noise({}, {})", inputs[0], inputs[1]))],

        // SDF
        SDFCircle => vec![o!(0, Float,
            format!("sg_sdf_circle({}, {})", inputs[0], inputs[1]))],
        SDFBox    => vec![o!(0, Float,
            format!("sg_sdf_box({}, {})", inputs[0], inputs[1]))],

        // Color
        HSVtoRGB => vec![o!(0, Vec4,
            format!("sg_hsv_to_rgb({}, {}, {})", inputs[0], inputs[1], inputs[2]))],

        // Output — no code emitted (handled as return value)
        FragmentColor => vec![],
    }
}

// ── Compiler ──────────────────────────────────────────────────────────────────

/// Compile a `ShaderGraph` into a complete WGSL fragment+vertex shader string.
pub fn compile(graph: &ShaderGraph) -> Result<String, String> {
    // 1. Validate: find FragmentColor output node
    let output_id = graph.nodes.iter()
        .find(|n| matches!(n.kind, NodeKind::FragmentColor))
        .map(|n| n.id)
        .ok_or("Shader graph has no FragmentColor output node")?;

    // 2. Edge lookup: (to_node, to_port) → (from_node, from_port)
    let edge_map: HashMap<(u32, usize), (u32, usize)> = graph.edges.iter()
        .map(|e| ((e.to_node, e.to_port), (e.from_node, e.from_port)))
        .collect();

    // 3. Build dependency structure for topological sort
    let mut in_degree: HashMap<u32, usize> = graph.nodes.iter()
        .map(|n| (n.id, 0))
        .collect();
    let mut successors: HashMap<u32, Vec<u32>> = graph.nodes.iter()
        .map(|n| (n.id, vec![]))
        .collect();
    for e in &graph.edges {
        *in_degree.entry(e.to_node).or_default() += 1;
        successors.entry(e.from_node).or_default().push(e.to_node);
    }

    // 4. Kahn's topological sort
    let mut queue: VecDeque<u32> = in_degree.iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&id, _)| id)
        .collect();
    let mut sorted: Vec<u32> = vec![];
    while let Some(id) = queue.pop_front() {
        sorted.push(id);
        for &s in successors.get(&id).unwrap_or(&vec![]) {
            let d = in_degree.get_mut(&s).unwrap();
            *d -= 1;
            if *d == 0 { queue.push_back(s); }
        }
    }
    if sorted.len() != graph.nodes.len() {
        return Err("Cycle detected in shader graph".into());
    }

    // 5. Code generation — walk nodes in topo order
    let node_by_id: HashMap<u32, &ShaderNode> = graph.nodes.iter().map(|n| (n.id, n)).collect();
    let mut var_for: HashMap<(u32, usize), String> = HashMap::new(); // output var names
    let mut body = String::new();

    let mut need_hash  = false;
    let mut need_noise = false;
    let mut need_sdf   = false;
    let mut need_hsv   = false;

    for &node_id in &sorted {
        let node = node_by_id[&node_id];

        // Track utility needs
        match &node.kind {
            NodeKind::Hash                      => need_hash  = true,
            NodeKind::ValueNoise                => { need_hash = true; need_noise = true; }
            NodeKind::SDFCircle | NodeKind::SDFBox => need_sdf = true,
            NodeKind::HSVtoRGB                  => need_hsv  = true,
            _ => {}
        }

        // Resolve input expressions
        let (input_types, _) = port_types(&node.kind);
        let inputs: Vec<String> = input_types.iter().enumerate().map(|(port, &ty)| {
            if let Some(&(from_id, from_port)) = edge_map.get(&(node_id, port)) {
                var_for.get(&(from_id, from_port))
                    .cloned()
                    .unwrap_or_else(|| ty.default_val().to_string())
            } else {
                ty.default_val().to_string()
            }
        }).collect();

        if matches!(&node.kind, NodeKind::FragmentColor) {
            continue; // handled as the return value
        }

        // Emit code and record output variable names
        let outputs = emit_node(node_id, &node.kind, &inputs);
        for (port_idx, (var, ty, expr)) in outputs.iter().enumerate() {
            body.push_str(&format!("    let {}: {} = {};\n", var, ty.wgsl_ty(), expr));
            var_for.insert((node_id, port_idx), var.clone());
        }
    }

    // 6. Return expression (FragmentColor input port 0)
    let ret = if let Some(&(from_id, from_port)) = edge_map.get(&(output_id, 0)) {
        var_for.get(&(from_id, from_port))
            .cloned()
            .unwrap_or_else(|| "vec4<f32>(0.0,0.0,0.0,1.0)".into())
    } else {
        "vec4<f32>(0.0,0.0,0.0,1.0)".into()
    };

    // 7. Assemble utility functions
    let mut utils = String::new();
    if need_hash  { utils.push_str(UTIL_HASH); }
    if need_noise { utils.push_str(UTIL_VALUE_NOISE); }
    if need_sdf   { utils.push_str(UTIL_SDF); }
    if need_hsv   { utils.push_str(UTIL_HSV); }

    Ok(format!(
        "{preamble}{utils}{vs}\n\
         @fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{\n\
         {body}\
             return {ret};\n\
         }}\n",
        preamble = PREAMBLE,
        vs       = VERTEX_SHADER,
    ))
}

// ── WGSL static strings ───────────────────────────────────────────────────────

const PREAMBLE: &str = "\
@group(0) @binding(0) var t_src: texture_2d<f32>;\n\
@group(0) @binding(1) var s_src: sampler;\n\
\n\
struct CustomParams { time: f32, _pad0: f32, screen_w: f32, screen_h: f32 }\n\
@group(1) @binding(0) var<uniform> p: CustomParams;\n\
\n\
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }\n";

const VERTEX_SHADER: &str = "\
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {\n\
    var pts = array<vec2<f32>, 3>(vec2(-1.0,-1.0), vec2(3.0,-1.0), vec2(-1.0,3.0));\n\
    var uvs = array<vec2<f32>, 3>(vec2(0.0,1.0),   vec2(2.0,1.0),  vec2(0.0,-1.0));\n\
    return VsOut(vec4(pts[vi], 0.0, 1.0), uvs[vi]);\n\
}";

const UTIL_HASH: &str = "
fn sg_hash(q: vec2<f32>) -> f32 {
    return fract(sin(dot(q, vec2<f32>(127.1, 311.7))) * 43758.545);
}
";

const UTIL_VALUE_NOISE: &str = "
fn sg_value_noise(q: vec2<f32>, scale: f32) -> f32 {
    let sp = q * scale;
    let ip = floor(sp);
    let fp = fract(sp);
    let u = fp * fp * (3.0 - 2.0 * fp);
    let a = sg_hash(ip + vec2<f32>(0.0, 0.0));
    let b = sg_hash(ip + vec2<f32>(1.0, 0.0));
    let c = sg_hash(ip + vec2<f32>(0.0, 1.0));
    let d = sg_hash(ip + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
";

const UTIL_SDF: &str = "
fn sg_sdf_circle(q: vec2<f32>, r: f32) -> f32 { return length(q) - r; }
fn sg_sdf_box(q: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let d = abs(q) - half_size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}
";

const UTIL_HSV: &str = "
fn sg_hsv_to_rgb(h: f32, s: f32, v: f32) -> vec4<f32> {
    let p = abs(fract(vec3<f32>(h, h, h) / 360.0 + vec3<f32>(1.0, 2.0/3.0, 1.0/3.0)) * 6.0 - 3.0);
    let rgb = v * mix(vec3<f32>(1.0), clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)), s);
    return vec4<f32>(rgb, 1.0);
}
";
