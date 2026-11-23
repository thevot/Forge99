pub const LINE_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    fog_params: vec4<f32>,
    fog_color: vec4<f32>,
    cam_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub const SOLID_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    fog_params: vec4<f32>,
    fog_color: vec4<f32>,
    cam_pos: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
@group(1) @binding(2)
var t_lightmap: texture_2d<f32>;
@group(1) @binding(3)
var s_lightmap: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) lm_uv: vec2<f32>,
    @location(5) packed_meta: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) packed_meta: u32,
    @location(4) lm_uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    var lighting = 1.0;
    let is_unlit = (in.packed_meta & 0x100000u) != 0u;
    if (uniforms.light_dir.w < 0.5 && !is_unlit) {
        let norm = normalize(in.normal.xyz);
        let light = normalize(uniforms.light_dir.xyz);
        let diff = max(dot(norm, light), 0.0);
        lighting = 0.25 + diff * 0.75;
    }
    out.color = vec4<f32>(in.color.rgb * lighting, in.color.a);
    out.world_pos = in.position;
    out.uv = in.uv;
    out.packed_meta = in.packed_meta;
    out.lm_uv = in.lm_uv;
    return out;
}

fn shade_solid_surface(
    in: VertexOutput,
    texel: vec4<f32>,
    ddx: vec2<f32>,
    ddy: vec2<f32>,
    is_translucent: bool,
) -> vec4<f32> {
    let diff_layer = in.packed_meta & 0xFFFFu;
    let has_lm = (in.packed_meta & 0x10000u) != 0u;
    let is_unlit = (in.packed_meta & 0x100000u) != 0u;

    var lm_rgb = vec3<f32>(1.0, 1.0, 1.0);
    if (has_lm && !is_unlit) {
        lm_rgb = textureSampleLevel(t_lightmap, s_lightmap, in.lm_uv, 0.0).rgb;
    }

    var rgb = in.color.rgb * lm_rgb;
    var alpha = 1.0;

    if (diff_layer > 0u) {
        rgb = texel.rgb * rgb;
        if (is_translucent) {
            alpha = texel.a * in.color.a;
        }
    } else {
        if (is_translucent) {
            alpha = in.color.a;
        }
        // Architectural UV texel modulation (64-unit major tile, 32-unit checker, 16-unit sub-grid)
        // Fade out sub-pixel details as UV screen-space derivative grows to eliminate moire shimmer.
        let uv_fw_vec = (abs(ddx) + abs(ddy)) * 256.0;
        let uv_fw = max(uv_fw_vec.x, uv_fw_vec.y);
        let fade16 = 1.0 - smoothstep(4.0, 14.0, uv_fw);
        let fade32 = 1.0 - smoothstep(8.0, 28.0, uv_fw);
        let fade64 = 1.0 - smoothstep(16.0, 56.0, uv_fw);
        let uv_tile = fract(in.uv / 64.0);
        let uv_sub = fract(in.uv / 16.0);
        let checker = mix(0.5, abs((floor(in.uv.x / 32.0) + floor(in.uv.y / 32.0)) % 2.0), fade32);
        let border64 = min(step(uv_tile.x, 0.03) + step(0.97, uv_tile.x) + step(uv_tile.y, 0.03) + step(0.97, uv_tile.y), 1.0) * fade64;
        let border16 = min(step(uv_sub.x, 0.04) + step(0.96, uv_sub.x) + step(0.04, uv_sub.y) + step(0.96, uv_sub.y), 1.0) * fade16;
        let tex_mod = clamp(0.93 + 0.07 * checker - 0.14 * border64 - 0.05 * border16, 0.68, 1.12);
        rgb = rgb * tex_mod;
    }

    if (is_translucent) {
        let is_translucent_flag = (in.packed_meta & 0x40000u) != 0u;
        let is_modulated_flag = (in.packed_meta & 0x80000u) != 0u;
        if (is_translucent_flag) {
            alpha = min(alpha, 0.55);
        } else if (is_modulated_flag) {
            alpha = min(alpha, 0.75);
        }
    }

    var final_color = vec4<f32>(rgb, alpha);

    // Distance Fog Simulation
    if (uniforms.fog_params.w > 0.5) {
        let dist = distance(in.world_pos, uniforms.cam_pos.xyz);
        let fog_start = uniforms.fog_params.x;
        let fog_end = uniforms.fog_params.y;
        let fog_factor = clamp((dist - fog_start) / max(fog_end - fog_start, 1.0), 0.0, 1.0);
        final_color = vec4<f32>(mix(final_color.rgb, uniforms.fog_color.rgb, fog_factor), final_color.a);
    }

    return final_color;
}

@fragment
fn fs_opaque(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv_norm = in.uv / 256.0;
    let ddx = dpdx(uv_norm);
    let ddy = dpdy(uv_norm);
    let diff_layer = in.packed_meta & 0xFFFFu;
    var texel = vec4<f32>(1.0);
    if (diff_layer > 0u) {
        texel = textureSampleGrad(t_diffuse, s_diffuse, uv_norm, i32(diff_layer), ddx, ddy);
    }
    return vec4<f32>(shade_solid_surface(in, texel, ddx, ddy, false).rgb, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv_norm = in.uv / 256.0;
    let ddx = dpdx(uv_norm);
    let ddy = dpdy(uv_norm);
    let diff_layer = in.packed_meta & 0xFFFFu;
    var texel = vec4<f32>(1.0);
    if (diff_layer > 0u) {
        texel = textureSampleGrad(t_diffuse, s_diffuse, uv_norm, i32(diff_layer), ddx, ddy);
        let is_masked = (in.packed_meta & 0x20000u) != 0u;
        if (is_masked && texel.a < 0.5) {
            discard;
        }
    }
    return vec4<f32>(shade_solid_surface(in, texel, ddx, ddy, false).rgb, 1.0);
}

@fragment
fn fs_translucent(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv_norm = in.uv / 256.0;
    let ddx = dpdx(uv_norm);
    let ddy = dpdy(uv_norm);
    let diff_layer = in.packed_meta & 0xFFFFu;
    var texel = vec4<f32>(1.0);
    if (diff_layer > 0u) {
        texel = textureSampleGrad(t_diffuse, s_diffuse, uv_norm, i32(diff_layer), ddx, ddy);
    }
    return shade_solid_surface(in, texel, ddx, ddy, true);
}
"#;

pub const VIEWPORT_BLIT_SHADER: &str = r#"
@group(0) @binding(0)
var t_viewport: texture_2d<f32>;
@group(0) @binding(1)
var s_viewport: sampler;

struct BlitOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> BlitOut {
    var pos = vec2<f32>(-1.0, -1.0);
    if (idx == 1u) {
        pos = vec2<f32>(3.0, -1.0);
    } else if (idx == 2u) {
        pos = vec2<f32>(-1.0, 3.0);
    }
    var out: BlitOut;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>((pos.x + 1.0) * 0.5, (1.0 - pos.y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: BlitOut) -> @location(0) vec4<f32> {
    return textureSample(t_viewport, s_viewport, in.uv);
}
"#;

