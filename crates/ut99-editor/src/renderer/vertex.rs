use bytemuck::{Pod, Zeroable};

pub const SOLID_META_HAS_LIGHTMAP: u32 = 1 << 16;
pub const SOLID_META_MASKED: u32 = 1 << 17;
pub const SOLID_META_TRANSLUCENT: u32 = 1 << 18;
pub const SOLID_META_MODULATED: u32 = 1 << 19;
pub const SOLID_META_UNLIT: u32 = 1 << 20;

#[inline]
pub fn f32_to_f16(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = bits & 0x007F_FFFF;

    if exp == 0xFF {
        return if mant != 0 {
            sign | 0x7E00
        } else {
            sign | 0x7BFF
        };
    }

    if exp == 0 {
        return sign;
    }

    let unbiased = exp - 127;
    if unbiased < -25 {
        return sign;
    }

    if unbiased <= -15 {
        let full_mant = 0x0080_0000 | mant;
        let rshift = (-1 - unbiased) as u32;
        let quot = full_mant >> rshift;
        let rem_mask = (1u32 << rshift) - 1;
        let halfway = 1u32 << (rshift - 1);
        let rem = full_mant & rem_mask;
        let round_up = rem > halfway || (rem == halfway && (quot & 1) != 0);
        let sub = (quot + u32::from(round_up)) as u16;
        return sign | sub;
    }

    if unbiased > 15 {
        return sign | 0x7BFF;
    }

    let f16_exp = (unbiased + 15) as u16;
    let quot = (mant >> 13) as u16;
    let rem = mant & 0x1FFF;
    let round_up = rem > 0x1000 || (rem == 0x1000 && (quot & 1) != 0);
    let raw = ((f16_exp << 10) | quot) + u16::from(round_up);
    let clamped = if raw >= 0x7C00 { 0x7BFF } else { raw };
    sign | clamped
}

#[inline]
pub fn pack_unorm8x4(color: [f32; 4]) -> [u8; 4] {
    [
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

#[inline]
pub fn pack_color_f16x4(color: [f32; 4]) -> [u16; 4] {
    [
        f32_to_f16(color[0]),
        f32_to_f16(color[1]),
        f32_to_f16(color[2]),
        f32_to_f16(color[3]),
    ]
}

#[inline]
pub fn pack_normal_snorm8x4(normal: [f32; 3]) -> [i8; 4] {
    [
        (normal[0].clamp(-1.0, 1.0) * 127.0).round() as i8,
        (normal[1].clamp(-1.0, 1.0) * 127.0).round() as i8,
        (normal[2].clamp(-1.0, 1.0) * 127.0).round() as i8,
        0,
    ]
}

#[inline]
pub fn pack_unorm16x2(uv: [f32; 2]) -> [u16; 2] {
    [
        (uv[0].clamp(0.0, 1.0) * 65535.0).round() as u16,
        (uv[1].clamp(0.0, 1.0) * 65535.0).round() as u16,
    ]
}

#[inline]
pub const fn pack_solid_meta(tex_layer: u32, lm_layer: u32, poly_flags: u32) -> u32 {
    let mut meta = tex_layer & 0xFFFF;
    if lm_layer > 0 {
        meta |= SOLID_META_HAS_LIGHTMAP;
    }
    if (poly_flags & 0x0000_0002) != 0 {
        meta |= SOLID_META_MASKED;
    }
    if (poly_flags & 0x0000_0004) != 0 {
        meta |= SOLID_META_TRANSLUCENT;
    }
    if (poly_flags & 0x0000_0040) != 0 {
        meta |= SOLID_META_MODULATED;
    }
    if (poly_flags & 0x0040_0000) != 0 {
        meta |= SOLID_META_UNLIT;
    }
    meta
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [u8; 4],
}

impl LineVertex {
    #[inline]
    pub fn pack_color(color: [f32; 4]) -> [u8; 4] {
        pack_unorm8x4(color)
    }

    #[inline]
    pub fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self {
            position,
            color: pack_unorm8x4(color),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SolidVertex {
    pub position: [f32; 3],
    pub normal: [i8; 4],
    pub color: [u16; 4],
    pub uv: [f32; 2],
    pub lm_uv: [u16; 2],
    pub packed_meta: u32,
}

impl SolidVertex {
    #[inline]
    pub fn pack_color(color: [f32; 4]) -> [u16; 4] {
        pack_color_f16x4(color)
    }

    #[inline]
    pub fn new(position: [f32; 3], normal: [f32; 3], color: [f32; 4], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal: pack_normal_snorm8x4(normal),
            color: pack_color_f16x4(color),
            uv,
            lm_uv: [0, 0],
            packed_meta: 0,
        }
    }

    #[inline]
    pub fn with_texture(
        position: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        uv: [f32; 2],
        tex_layer: u32,
        poly_flags: u32,
    ) -> Self {
        Self {
            position,
            normal: pack_normal_snorm8x4(normal),
            color: pack_color_f16x4(color),
            uv,
            lm_uv: [0, 0],
            packed_meta: pack_solid_meta(tex_layer, 0, poly_flags),
        }
    }

    #[inline]
    pub fn with_texture_and_lightmap(
        position: [f32; 3],
        normal: [f32; 3],
        color: [f32; 4],
        uv: [f32; 2],
        tex_layer: u32,
        lm_layer: u32,
        poly_flags: u32,
        lm_uv: [f32; 2],
    ) -> Self {
        Self {
            position,
            normal: pack_normal_snorm8x4(normal),
            color: pack_color_f16x4(color),
            uv,
            lm_uv: pack_unorm16x2(lm_uv),
            packed_meta: pack_solid_meta(tex_layer, lm_layer, poly_flags),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SolidVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Snorm8x4,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float16x4,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // lm_uv
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Unorm16x2,
                },
                // packed_meta
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub light_dir: [f32; 4], // w is unused padding
    pub fog_params: [f32; 4], // x: start, y: end, z: density, w: enabled (1.0 or 0.0)
    pub fog_color: [f32; 4],
    pub cam_pos: [f32; 4],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            light_dir: [-0.5, -0.5, 0.7, 0.0],
            fog_params: [200.0, 3000.0, 1.0, 0.0],
            fog_color: [0.1, 0.12, 0.15, 1.0],
            cam_pos: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_vertex_sizes_and_alignment() {
        assert_eq!(std::mem::size_of::<LineVertex>(), 16);
        assert_eq!(std::mem::align_of::<LineVertex>(), 4);
        assert_eq!(std::mem::size_of::<SolidVertex>(), 40);
        assert_eq!(std::mem::align_of::<SolidVertex>(), 4);
        assert_eq!(std::mem::size_of::<Uniforms>(), 128);
    }

    #[test]
    fn f32_to_f16_encoding_and_clamping() {
        assert_eq!(f32_to_f16(0.0), 0x0000);
        assert_eq!(f32_to_f16(-0.0), 0x8000);
        assert_eq!(f32_to_f16(1.0), 0x3C00);
        assert_eq!(f32_to_f16(0.5), 0x3800);
        assert_eq!(f32_to_f16(0.25), 0x3400);
        assert_eq!(f32_to_f16(2.0), 0x4000);
        assert_eq!(f32_to_f16(-1.0), 0xBC00);
        assert_eq!(f32_to_f16(65504.0), 0x7BFF);
        assert_eq!(f32_to_f16(100_000.0), 0x7BFF);
        assert_eq!(f32_to_f16(f32::INFINITY), 0x7BFF);
        assert_eq!(f32_to_f16(f32::NEG_INFINITY), 0xFBFF);
        assert_eq!(f32_to_f16(6e-8), 0x0001);
        assert_eq!(f32_to_f16(1e-12), 0x0000);
        assert_eq!(f32_to_f16(f32::NAN) & 0x7E00, 0x7E00);
    }

    #[test]
    fn packed_attributes_preserve_bounds_and_flags() {
        assert_eq!(pack_normal_snorm8x4([1.0, -1.0, 0.0]), [127, -127, 0, 0]);
        assert_eq!(pack_unorm16x2([0.0, 1.0]), [0, 65535]);
        let mid = pack_unorm16x2([0.5, -0.5]);
        assert!((mid[0] as i32 - 32768).abs() <= 1);
        assert_eq!(mid[1], 0);
        assert_eq!(pack_unorm8x4([0.0, 1.0, 2.0, -1.0]), [0, 255, 255, 0]);

        let meta = pack_solid_meta(255, 1, 0x0040_0046);
        assert_eq!(meta & 0xFFFF, 255);
        assert_ne!(meta & SOLID_META_HAS_LIGHTMAP, 0);
        assert_ne!(meta & SOLID_META_MASKED, 0);
        assert_ne!(meta & SOLID_META_TRANSLUCENT, 0);
        assert_ne!(meta & SOLID_META_MODULATED, 0);
        assert_ne!(meta & SOLID_META_UNLIT, 0);

        let bare = pack_solid_meta(0x1_0007, 0, 0);
        assert_eq!(bare, 7);
    }
}
