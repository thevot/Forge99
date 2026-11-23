use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::io::{self, Cursor};
use std::path::Path;

use crate::package::compact_index::{read_compact_index, write_compact_index};
use crate::package::Package;
use crate::unr::tagged_property::{
    read_tagged_properties, write_tagged_properties, TaggedProperty, TaggedValue,
};

#[derive(Debug, Clone)]
pub struct UtxTexture {
    pub name: String,
    pub group: String,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub masked: bool,
    pub rgba_pixels: Vec<u8>,
}

impl UtxTexture {
    /// Quantizes `rgba_pixels` into a 256-color UE1 `UPalette` and 8-bit `P8` Mip 0 index buffer.
    pub fn quantize_to_palette(&self) -> ([[u8; 4]; 256], Vec<u8>) {
        let pixel_count = (self.width as usize).saturating_mul(self.height as usize);
        let mut palette = [[0u8, 0u8, 0u8, 255u8]; 256];
        let mut color_to_idx: HashMap<[u8; 3], u8> = HashMap::new();
        let mut next_idx: usize = 0;

        if self.masked {
            palette[0] = [0, 0, 0, 0];
            next_idx = 1;
        }

        let mut indices = Vec::with_capacity(pixel_count);
        for i in 0..pixel_count {
            let base = i * 4;
            let (r, g, b, a) = if base + 4 <= self.rgba_pixels.len() {
                (
                    self.rgba_pixels[base],
                    self.rgba_pixels[base + 1],
                    self.rgba_pixels[base + 2],
                    self.rgba_pixels[base + 3],
                )
            } else {
                (128, 128, 128, 255)
            };

            if self.masked && a < 128 {
                indices.push(0);
                continue;
            }

            let rgb = [r, g, b];
            if let Some(&idx) = color_to_idx.get(&rgb) {
                indices.push(idx);
            } else if next_idx < 256 {
                let idx = next_idx as u8;
                palette[next_idx] = [r, g, b, 255];
                color_to_idx.insert(rgb, idx);
                next_idx += 1;
                indices.push(idx);
            } else {
                // Nearest color in palette
                let start_search = if self.masked { 1 } else { 0 };
                let mut best_idx = start_search as u8;
                let mut best_dist = u32::MAX;
                for p_i in start_search..256 {
                    let pr = palette[p_i][0] as i32 - r as i32;
                    let pg = palette[p_i][1] as i32 - g as i32;
                    let pb = palette[p_i][2] as i32 - b as i32;
                    let dist = (pr * pr + pg * pg + pb * pb) as u32;
                    if dist < best_dist {
                        best_dist = dist;
                        best_idx = p_i as u8;
                        if dist == 0 {
                            break;
                        }
                    }
                }
                indices.push(best_idx);
            }
        }

        (palette, indices)
    }

    /// Generates a complete UE1 `P8` mipmap pyramid (`Mip 0..N` down to `1x1`) using 2x2 box-filtering
    /// in RGB space and re-indexing against the shared 256-color `UPalette`.
    pub fn generate_p8_mip_chain(&self) -> ([[u8; 4]; 256], Vec<(u32, u32, Vec<u8>)>) {
        let (palette, mip0_indices) = self.quantize_to_palette();
        let mut chain = Vec::new();
        let mut cur_w = self.width.max(1);
        let mut cur_h = self.height.max(1);
        let mut cur_rgba = if self.rgba_pixels.len() >= (cur_w as usize * cur_h as usize * 4) {
            self.rgba_pixels[..(cur_w as usize * cur_h as usize * 4)].to_vec()
        } else {
            let mut full = Vec::with_capacity(cur_w as usize * cur_h as usize * 4);
            for &idx in &mip0_indices {
                full.extend_from_slice(&palette[idx as usize]);
            }
            full
        };

        chain.push((cur_w, cur_h, mip0_indices));

        while cur_w > 1 || cur_h > 1 {
            let next_w = (cur_w / 2).max(1);
            let next_h = (cur_h / 2).max(1);
            let mut next_rgba = Vec::with_capacity((next_w * next_h * 4) as usize);
            let mut next_indices = Vec::with_capacity((next_w * next_h) as usize);
            let start_search = if self.masked { 1 } else { 0 };

            for y in 0..next_h {
                for x in 0..next_w {
                    let sx0 = (x * 2).min(cur_w - 1);
                    let sx1 = (x * 2 + 1).min(cur_w - 1);
                    let sy0 = (y * 2).min(cur_h - 1);
                    let sy1 = (y * 2 + 1).min(cur_h - 1);

                    let samples = [
                        (sy0 * cur_w + sx0) as usize * 4,
                        (sy0 * cur_w + sx1) as usize * 4,
                        (sy1 * cur_w + sx0) as usize * 4,
                        (sy1 * cur_w + sx1) as usize * 4,
                    ];

                    let mut r_sum = 0u32;
                    let mut g_sum = 0u32;
                    let mut b_sum = 0u32;
                    let mut a_sum = 0u32;
                    let mut opaque_count = 0u32;

                    for &base in &samples {
                        let r = cur_rgba[base] as u32;
                        let g = cur_rgba[base + 1] as u32;
                        let b = cur_rgba[base + 2] as u32;
                        let a = cur_rgba[base + 3] as u32;
                        a_sum += a;
                        if !self.masked || a >= 128 {
                            r_sum += r;
                            g_sum += g;
                            b_sum += b;
                            opaque_count += 1;
                        }
                    }

                    let avg_a = (a_sum / 4) as u8;
                    if self.masked && (opaque_count == 0 || avg_a < 128) {
                        next_rgba.extend_from_slice(&[0, 0, 0, 0]);
                        next_indices.push(0);
                    } else {
                        let denom = opaque_count.max(1);
                        let avg_r = (r_sum / denom) as u8;
                        let avg_g = (g_sum / denom) as u8;
                        let avg_b = (b_sum / denom) as u8;
                        next_rgba.extend_from_slice(&[avg_r, avg_g, avg_b, 255]);

                        let mut best_idx = start_search as u8;
                        let mut best_dist = u32::MAX;
                        for p_i in start_search..256 {
                            let pr = palette[p_i][0] as i32 - avg_r as i32;
                            let pg = palette[p_i][1] as i32 - avg_g as i32;
                            let pb = palette[p_i][2] as i32 - avg_b as i32;
                            let dist = (pr * pr + pg * pg + pb * pb) as u32;
                            if dist < best_dist {
                                best_dist = dist;
                                best_idx = p_i as u8;
                                if dist == 0 {
                                    break;
                                }
                            }
                        }
                        next_indices.push(best_idx);
                    }
                }
            }

            cur_w = next_w;
            cur_h = next_h;
            cur_rgba = next_rgba;
            chain.push((cur_w, cur_h, next_indices));
        }

        (palette, chain)
    }
}

/// Decodes a single RGB565 16-bit color into `[R, G, B]`.
#[inline]
fn rgb565_to_rgb8(c: u16) -> [u8; 3] {
    let r5 = ((c >> 11) & 0x1F) as u8;
    let g6 = ((c >> 5) & 0x3F) as u8;
    let b5 = (c & 0x1F) as u8;
    [
        (r5 << 3) | (r5 >> 2),
        (g6 << 2) | (g6 >> 4),
        (b5 << 3) | (b5 >> 2),
    ]
}

/// Decodes a S3TC `DXT1` (`TEXF_DXT1 = 3`) buffer into `width * height * 4` RGBA8 pixels.
pub fn decode_dxt1_to_rgba8(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let bw = (width.max(1) as usize + 3) / 4;
    let bh = (height.max(1) as usize + 3) / 4;
    let required = bw.checked_mul(bh)?.checked_mul(8)?;
    if data.len() < required {
        return None;
    }
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let block = &data[(by * bw + bx) * 8..(by * bw + bx + 1) * 8];
            let c0 = u16::from_le_bytes([block[0], block[1]]);
            let c1 = u16::from_le_bytes([block[2], block[3]]);
            let lut = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
            let rgb0 = rgb565_to_rgb8(c0);
            let rgb1 = rgb565_to_rgb8(c1);
            let colors: [[u8; 4]; 4] = if c0 > c1 {
                [
                    [rgb0[0], rgb0[1], rgb0[2], 255],
                    [rgb1[0], rgb1[1], rgb1[2], 255],
                    [
                        ((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8,
                        ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8,
                        ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8,
                        255,
                    ],
                    [
                        ((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8,
                        ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8,
                        ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8,
                        255,
                    ],
                ]
            } else {
                [
                    [rgb0[0], rgb0[1], rgb0[2], 255],
                    [rgb1[0], rgb1[1], rgb1[2], 255],
                    [
                        ((rgb0[0] as u16 + rgb1[0] as u16) / 2) as u8,
                        ((rgb0[1] as u16 + rgb1[1] as u16) / 2) as u8,
                        ((rgb0[2] as u16 + rgb1[2] as u16) / 2) as u8,
                        255,
                    ],
                    [0, 0, 0, 0],
                ]
            };

            for py in 0..4usize {
                let y = by * 4 + py;
                if y >= height as usize {
                    continue;
                }
                for px in 0..4usize {
                    let x = bx * 4 + px;
                    if x >= width as usize {
                        continue;
                    }
                    let bit_idx = (py * 4 + px) * 2;
                    let code = ((lut >> bit_idx) & 0x3) as usize;
                    let dst = (y * width as usize + x) * 4;
                    out[dst..dst + 4].copy_from_slice(&colors[code]);
                }
            }
        }
    }
    Some(out)
}

/// Decodes a S3TC `DXT3` (`TEXF_DXT3 = 7`) buffer into `width * height * 4` RGBA8 pixels.
pub fn decode_dxt3_to_rgba8(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let bw = (width.max(1) as usize + 3) / 4;
    let bh = (height.max(1) as usize + 3) / 4;
    let required = bw.checked_mul(bh)?.checked_mul(16)?;
    if data.len() < required {
        return None;
    }
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let block = &data[(by * bw + bx) * 16..(by * bw + bx + 1) * 16];
            let alpha_bits = u64::from_le_bytes([
                block[0], block[1], block[2], block[3], block[4], block[5], block[6], block[7],
            ]);
            let c0 = u16::from_le_bytes([block[8], block[9]]);
            let c1 = u16::from_le_bytes([block[10], block[11]]);
            let lut = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
            let rgb0 = rgb565_to_rgb8(c0);
            let rgb1 = rgb565_to_rgb8(c1);
            let colors: [[u8; 3]; 4] = [
                rgb0,
                rgb1,
                [
                    ((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8,
                    ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8,
                    ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8,
                ],
                [
                    ((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8,
                    ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8,
                    ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8,
                ],
            ];
            for py in 0..4usize {
                let y = by * 4 + py;
                if y >= height as usize {
                    continue;
                }
                for px in 0..4usize {
                    let x = bx * 4 + px;
                    if x >= width as usize {
                        continue;
                    }
                    let idx = py * 4 + px;
                    let a4 = ((alpha_bits >> (idx * 4)) & 0xF) as u8;
                    let a8 = (a4 << 4) | a4;
                    let code = ((lut >> (idx * 2)) & 0x3) as usize;
                    let dst = (y * width as usize + x) * 4;
                    out[dst] = colors[code][0];
                    out[dst + 1] = colors[code][1];
                    out[dst + 2] = colors[code][2];
                    out[dst + 3] = a8;
                }
            }
        }
    }
    Some(out)
}

/// Decodes a S3TC `DXT5` (`TEXF_DXT5 = 8`) buffer into `width * height * 4` RGBA8 pixels.
pub fn decode_dxt5_to_rgba8(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let bw = (width.max(1) as usize + 3) / 4;
    let bh = (height.max(1) as usize + 3) / 4;
    let required = bw.checked_mul(bh)?.checked_mul(16)?;
    if data.len() < required {
        return None;
    }
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let block = &data[(by * bw + bx) * 16..(by * bw + bx + 1) * 16];
            let a0 = block[0];
            let a1 = block[1];
            let alpha_lut: u64 = (block[2] as u64)
                | ((block[3] as u64) << 8)
                | ((block[4] as u64) << 16)
                | ((block[5] as u64) << 24)
                | ((block[6] as u64) << 32)
                | ((block[7] as u64) << 40);
            let mut alphas = [0u8; 8];
            alphas[0] = a0;
            alphas[1] = a1;
            if a0 > a1 {
                for i in 1..7 {
                    alphas[i + 1] =
                        (((7 - i as u16) * a0 as u16 + (i as u16) * a1 as u16) / 7) as u8;
                }
            } else {
                for i in 1..5 {
                    alphas[i + 1] =
                        (((5 - i as u16) * a0 as u16 + (i as u16) * a1 as u16) / 5) as u8;
                }
                alphas[6] = 0;
                alphas[7] = 255;
            }
            let c0 = u16::from_le_bytes([block[8], block[9]]);
            let c1 = u16::from_le_bytes([block[10], block[11]]);
            let lut = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
            let rgb0 = rgb565_to_rgb8(c0);
            let rgb1 = rgb565_to_rgb8(c1);
            let colors: [[u8; 3]; 4] = [
                rgb0,
                rgb1,
                [
                    ((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8,
                    ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8,
                    ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8,
                ],
                [
                    ((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8,
                    ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8,
                    ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8,
                ],
            ];
            for py in 0..4usize {
                let y = by * 4 + py;
                if y >= height as usize {
                    continue;
                }
                for px in 0..4usize {
                    let x = bx * 4 + px;
                    if x >= width as usize {
                        continue;
                    }
                    let idx = py * 4 + px;
                    let a_code = ((alpha_lut >> (idx * 3)) & 0x7) as usize;
                    let code = ((lut >> (idx * 2)) & 0x3) as usize;
                    let dst = (y * width as usize + x) * 4;
                    out[dst] = colors[code][0];
                    out[dst + 1] = colors[code][1];
                    out[dst + 2] = colors[code][2];
                    out[dst + 3] = alphas[a_code];
                }
            }
        }
    }
    Some(out)
}

/// Encodes a single `[R, G, B]` color into RGB565 format (`u16`).
#[inline]
pub fn rgb8_to_rgb565(rgb: [u8; 3]) -> u16 {
    let r5 = (rgb[0] >> 3) as u16;
    let g6 = (rgb[1] >> 2) as u16;
    let b5 = (rgb[2] >> 3) as u16;
    (r5 << 11) | (g6 << 5) | b5
}

/// Computes perceptual squared distance between two RGB colors.
#[inline]
fn color_dist_sq(c1: [u8; 3], c2: [u8; 3]) -> u32 {
    let dr = c1[0] as i32 - c2[0] as i32;
    let dg = c1[1] as i32 - c2[1] as i32;
    let db = c1[2] as i32 - c2[2] as i32;
    (dr * dr * 3 + dg * dg * 4 + db * db * 2) as u32
}

/// Compresses a 2D RGBA8 pixel buffer (`width * height * 4` bytes) into S3TC `DXT1` (`BC1`) format.
///
/// When `punch_through` is true and any pixel in a 4x4 block has alpha < 128,
/// the block is encoded in 1-bit alpha punch-through mode (`c0 <= c1`).
/// Otherwise, the block is encoded as opaque DXT1 (`c0 > c1`).
pub fn encode_rgba8_to_dxt1(rgba: &[u8], width: u32, height: u32, punch_through: bool) -> Vec<u8> {
    let bw = (width.max(1) as usize + 3) / 4;
    let bh = (height.max(1) as usize + 3) / 4;
    let mut out = Vec::with_capacity(bw * bh * 8);

    for by in 0..bh {
        for bx in 0..bw {
            let mut block_rgb = [[0u8; 3]; 16];
            let mut block_a = [255u8; 16];
            let mut has_alpha = false;

            for py in 0..4usize {
                let y = (by * 4 + py).min(height.saturating_sub(1) as usize);
                for px in 0..4usize {
                    let x = (bx * 4 + px).min(width.saturating_sub(1) as usize);
                    let idx = py * 4 + px;
                    let src = (y * width as usize + x) * 4;
                    if src + 3 < rgba.len() {
                        block_rgb[idx] = [rgba[src], rgba[src + 1], rgba[src + 2]];
                        block_a[idx] = rgba[src + 3];
                        if punch_through && rgba[src + 3] < 128 {
                            has_alpha = true;
                        }
                    }
                }
            }

            if has_alpha {
                let mut min_rgb = [255u8; 3];
                let mut max_rgb = [0u8; 3];
                let mut any_opaque = false;

                for i in 0..16 {
                    if block_a[i] >= 128 {
                        any_opaque = true;
                        for c in 0..3 {
                            min_rgb[c] = min_rgb[c].min(block_rgb[i][c]);
                            max_rgb[c] = max_rgb[c].max(block_rgb[i][c]);
                        }
                    }
                }

                if !any_opaque {
                    out.extend_from_slice(&0u16.to_le_bytes());
                    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
                    out.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
                    continue;
                }

                let mut c0 = rgb8_to_rgb565(min_rgb);
                let mut c1 = rgb8_to_rgb565(max_rgb);
                if c0 > c1 {
                    std::mem::swap(&mut c0, &mut c1);
                }
                let rgb0 = rgb565_to_rgb8(c0);
                let rgb1 = rgb565_to_rgb8(c1);
                let pal = [
                    rgb0,
                    rgb1,
                    [
                        ((rgb0[0] as u16 + rgb1[0] as u16) / 2) as u8,
                        ((rgb0[1] as u16 + rgb1[1] as u16) / 2) as u8,
                        ((rgb0[2] as u16 + rgb1[2] as u16) / 2) as u8,
                    ],
                ];

                let mut lut = 0u32;
                for i in 0..16 {
                    let code = if block_a[i] < 128 {
                        3
                    } else {
                        let mut best_dist = u32::MAX;
                        let mut best_k = 0;
                        for k in 0..3 {
                            let d = color_dist_sq(block_rgb[i], pal[k]);
                            if d < best_dist {
                                best_dist = d;
                                best_k = k;
                            }
                        }
                        best_k
                    };
                    lut |= (code as u32) << (i * 2);
                }

                out.extend_from_slice(&c0.to_le_bytes());
                out.extend_from_slice(&c1.to_le_bytes());
                out.extend_from_slice(&lut.to_le_bytes());
            } else {
                let mut min_rgb = [255u8; 3];
                let mut max_rgb = [0u8; 3];
                for i in 0..16 {
                    for c in 0..3 {
                        min_rgb[c] = min_rgb[c].min(block_rgb[i][c]);
                        max_rgb[c] = max_rgb[c].max(block_rgb[i][c]);
                    }
                }

                let mut c0 = rgb8_to_rgb565(max_rgb);
                let mut c1 = rgb8_to_rgb565(min_rgb);

                if c0 == c1 {
                    if c0 < 0xFFFF {
                        c0 += 1;
                    } else if c1 > 0 {
                        c1 -= 1;
                    }
                }

                if c0 <= c1 {
                    std::mem::swap(&mut c0, &mut c1);
                }

                let rgb0 = rgb565_to_rgb8(c0);
                let rgb1 = rgb565_to_rgb8(c1);
                let pal = [
                    rgb0,
                    rgb1,
                    [
                        ((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8,
                        ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8,
                        ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8,
                    ],
                    [
                        ((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8,
                        ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8,
                        ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8,
                    ],
                ];

                let mut lut = 0u32;
                for i in 0..16 {
                    let mut best_dist = u32::MAX;
                    let mut best_k = 0;
                    for k in 0..4 {
                        let d = color_dist_sq(block_rgb[i], pal[k]);
                        if d < best_dist {
                            best_dist = d;
                            best_k = k;
                        }
                    }
                    lut |= (best_k as u32) << (i * 2);
                }

                out.extend_from_slice(&c0.to_le_bytes());
                out.extend_from_slice(&c1.to_le_bytes());
                out.extend_from_slice(&lut.to_le_bytes());
            }
        }
    }
    out
}

/// Compresses a 2D RGBA8 pixel buffer (`width * height * 4` bytes) into S3TC `DXT5` (`BC3`) format.
///
/// Each 4x4 block is encoded into 16 bytes:
/// - 8 bytes for 3-bit interpolated alpha channel (`a0`, `a1` + 16x 3-bit indices).
/// - 8 bytes for RGB color channel (opaque DXT1 `c0 > c1`).
pub fn encode_rgba8_to_dxt5(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let bw = (width.max(1) as usize + 3) / 4;
    let bh = (height.max(1) as usize + 3) / 4;
    let mut out = Vec::with_capacity(bw * bh * 16);

    for by in 0..bh {
        for bx in 0..bw {
            let mut block_rgb = [[0u8; 3]; 16];
            let mut block_a = [255u8; 16];

            for py in 0..4usize {
                let y = (by * 4 + py).min(height.saturating_sub(1) as usize);
                for px in 0..4usize {
                    let x = (bx * 4 + px).min(width.saturating_sub(1) as usize);
                    let idx = py * 4 + px;
                    let src = (y * width as usize + x) * 4;
                    if src + 3 < rgba.len() {
                        block_rgb[idx] = [rgba[src], rgba[src + 1], rgba[src + 2]];
                        block_a[idx] = rgba[src + 3];
                    }
                }
            }

            // 1. Alpha block (8 bytes)
            let mut min_a = 255u8;
            let mut max_a = 0u8;
            for &a in &block_a {
                min_a = min_a.min(a);
                max_a = max_a.max(a);
            }

            let a0 = max_a;
            let a1 = min_a;
            let mut alpha_lut = 0u64;

            if a0 == a1 {
                out.push(a0);
                out.push(a1);
                out.extend_from_slice(&[0u8; 6]);
            } else {
                let mut alphas = [0u8; 8];
                alphas[0] = a0;
                alphas[1] = a1;
                for i in 1..7 {
                    alphas[i + 1] = (((7 - i as u16) * a0 as u16 + (i as u16) * a1 as u16) / 7) as u8;
                }

                for i in 0..16 {
                    let a_val = block_a[i];
                    let mut best_diff = u32::MAX;
                    let mut best_idx = 0usize;
                    for k in 0..8 {
                        let diff = (a_val as i32 - alphas[k] as i32).abs() as u32;
                        if diff < best_diff {
                            best_diff = diff;
                            best_idx = k;
                        }
                    }
                    alpha_lut |= (best_idx as u64) << (i * 3);
                }

                out.push(a0);
                out.push(a1);
                let lut_bytes = alpha_lut.to_le_bytes();
                out.extend_from_slice(&lut_bytes[0..6]);
            }

            // 2. Color block (8 bytes)
            let mut min_rgb = [255u8; 3];
            let mut max_rgb = [0u8; 3];
            for i in 0..16 {
                for c in 0..3 {
                    min_rgb[c] = min_rgb[c].min(block_rgb[i][c]);
                    max_rgb[c] = max_rgb[c].max(block_rgb[i][c]);
                }
            }

            let mut c0 = rgb8_to_rgb565(max_rgb);
            let mut c1 = rgb8_to_rgb565(min_rgb);
            if c0 == c1 {
                if c0 < 0xFFFF {
                    c0 += 1;
                } else if c1 > 0 {
                    c1 -= 1;
                }
            }
            if c0 <= c1 {
                std::mem::swap(&mut c0, &mut c1);
            }

            let rgb0 = rgb565_to_rgb8(c0);
            let rgb1 = rgb565_to_rgb8(c1);
            let pal = [
                rgb0,
                rgb1,
                [
                    ((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8,
                    ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8,
                    ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8,
                ],
                [
                    ((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8,
                    ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8,
                    ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8,
                ],
            ];

            let mut lut = 0u32;
            for i in 0..16 {
                let mut best_dist = u32::MAX;
                let mut best_k = 0;
                for k in 0..4 {
                    let d = color_dist_sq(block_rgb[i], pal[k]);
                    if d < best_dist {
                        best_dist = d;
                        best_k = k;
                    }
                }
                lut |= (best_k as u32) << (i * 2);
            }

            out.extend_from_slice(&c0.to_le_bytes());
            out.extend_from_slice(&c1.to_le_bytes());
            out.extend_from_slice(&lut.to_le_bytes());
        }
    }
    out
}

/// Generates a full RGBA8 mipmap pyramid (`Mip 0..N` down to `1x1`) using 2x2 box-filtering.
pub fn generate_rgba8_mip_chain(rgba: &[u8], width: u32, height: u32) -> Vec<(u32, u32, Vec<u8>)> {
    let mut chain = Vec::new();
    let mut cur_w = width.max(1);
    let mut cur_h = height.max(1);
    let mut cur_rgba = if rgba.len() >= (cur_w as usize * cur_h as usize * 4) {
        rgba[..(cur_w as usize * cur_h as usize * 4)].to_vec()
    } else {
        let mut padded = rgba.to_vec();
        padded.resize((cur_w * cur_h * 4) as usize, 0);
        padded
    };

    chain.push((cur_w, cur_h, cur_rgba.clone()));

    while cur_w > 1 || cur_h > 1 {
        let next_w = (cur_w / 2).max(1);
        let next_h = (cur_h / 2).max(1);
        let mut next_rgba = Vec::with_capacity((next_w * next_h * 4) as usize);

        for y in 0..next_h {
            for x in 0..next_w {
                let sx0 = (x * 2).min(cur_w - 1);
                let sx1 = (x * 2 + 1).min(cur_w - 1);
                let sy0 = (y * 2).min(cur_h - 1);
                let sy1 = (y * 2 + 1).min(cur_h - 1);

                let samples = [
                    (sy0 * cur_w + sx0) as usize * 4,
                    (sy0 * cur_w + sx1) as usize * 4,
                    (sy1 * cur_w + sx0) as usize * 4,
                    (sy1 * cur_w + sx1) as usize * 4,
                ];

                let mut r_sum = 0u32;
                let mut g_sum = 0u32;
                let mut b_sum = 0u32;
                let mut a_sum = 0u32;

                for &base in &samples {
                    r_sum += cur_rgba[base] as u32;
                    g_sum += cur_rgba[base + 1] as u32;
                    b_sum += cur_rgba[base + 2] as u32;
                    a_sum += cur_rgba[base + 3] as u32;
                }

                next_rgba.push((r_sum / 4) as u8);
                next_rgba.push((g_sum / 4) as u8);
                next_rgba.push((b_sum / 4) as u8);
                next_rgba.push((a_sum / 4) as u8);
            }
        }

        cur_w = next_w;
        cur_h = next_h;
        cur_rgba = next_rgba;
        chain.push((cur_w, cur_h, cur_rgba.clone()));
    }

    chain
}

/// Generates a full `DXT1` (`BC1`) compressed mipmap pyramid down to `1x1`.
pub fn generate_dxt1_mip_chain(rgba: &[u8], width: u32, height: u32, punch_through: bool) -> Vec<(u32, u32, Vec<u8>)> {
    let rgba_chain = generate_rgba8_mip_chain(rgba, width, height);
    rgba_chain
        .into_iter()
        .map(|(w, h, data)| {
            let dxt1 = encode_rgba8_to_dxt1(&data, w, h, punch_through);
            (w, h, dxt1)
        })
        .collect()
}

/// Generates a full `DXT5` (`BC3`) compressed mipmap pyramid down to `1x1`.
pub fn generate_dxt5_mip_chain(rgba: &[u8], width: u32, height: u32) -> Vec<(u32, u32, Vec<u8>)> {
    let rgba_chain = generate_rgba8_mip_chain(rgba, width, height);
    rgba_chain
        .into_iter()
        .map(|(w, h, data)| {
            let dxt5 = encode_rgba8_to_dxt5(&data, w, h);
            (w, h, dxt5)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct UtxPackage {
    pub package_name: String,
    pub textures: Vec<UtxTexture>,
}

impl UtxPackage {
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let package = Package::open(path)?;
        let package_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Textures".to_string());

        Self::from_package(package, package_name)
    }

    /// Writes a slice of `UtxTexture` (along with their `UPalette` exports and full `FMipmap` chains) into an existing `Package`.
    /// When `default_outer_index > 0` (such as `MyLevel` = `1`), textures are parented under that export;
    /// otherwise they are parented under their `group` `Package` export (or `0` if group is `"None"`).
    pub fn write_textures_into_package(
        textures: &[UtxTexture],
        pkg: &mut Package,
        default_outer_index: i32,
    ) -> io::Result<()> {
        if textures.is_empty() {
            return Ok(());
        }

        pkg.names.add("None", 0);
        let n_core = pkg.names.add("Core", 0) as i32;
        let n_engine = pkg.names.add("Engine", 0) as i32;
        let n_class = pkg.names.add("Class", 0) as i32;
        let n_package = pkg.names.add("Package", 0) as i32;
        let n_palette_cls = pkg.names.add("Palette", 0) as i32;
        let n_texture_cls = pkg.names.add("Texture", 0) as i32;

        let _imp_core = pkg.add_import(n_core, n_package, 0, n_core);
        let imp_engine = pkg.add_import(n_core, n_package, 0, n_engine);
        let imp_palette = pkg.add_import(n_core, n_class, imp_engine, n_palette_cls);
        let imp_texture = pkg.add_import(n_core, n_class, imp_engine, n_texture_cls);

        let mut group_exports: HashMap<String, i32> = HashMap::new();

        for (i, tex) in textures.iter().enumerate() {
            let outer_idx = if default_outer_index < 0 {
                0
            } else if default_outer_index > 0 {
                default_outer_index
            } else if !tex.group.is_empty() && !tex.group.eq_ignore_ascii_case("None") {
                if let Some(&g_ref) = group_exports.get(&tex.group) {
                    g_ref
                } else {
                    let n_grp = pkg.names.add(tex.group.clone(), 0) as i32;
                    let g_ref = pkg.add_export(n_grp, 0, 0, 0, 0x0007_0004, Vec::new());
                    group_exports.insert(tex.group.clone(), g_ref);
                    g_ref
                }
            } else {
                0
            };

            let is_dxt1 = tex.format == 3;
            let is_dxt5 = tex.format == 8;

            let (pal_exp_ref, mip_chain, format_byte) = if is_dxt1 {
                let chain = generate_dxt1_mip_chain(&tex.rgba_pixels, tex.width, tex.height, tex.masked);
                (None, chain, 3u8)
            } else if is_dxt5 {
                let chain = generate_dxt5_mip_chain(&tex.rgba_pixels, tex.width, tex.height);
                (None, chain, 8u8)
            } else {
                let (palette_colors, chain) = tex.generate_p8_mip_chain();
                // 1. Serialize UPalette export (None CompactIndex + CompactIndex(256) + 1024 RGBA bytes)
                let n_none = pkg.names.add("None", 0) as i32;
                let pal_name = format!("Palette_{}_{i}", tex.name);
                let n_pal = pkg.names.add(pal_name, 0) as i32;
                let mut pal_buf = Vec::with_capacity(1028);
                write_compact_index(&mut pal_buf, n_none)?;
                write_compact_index(&mut pal_buf, 256)?;
                for c in &palette_colors {
                    pal_buf.extend_from_slice(c);
                }
                let pal_ref = pkg.add_export(n_pal, imp_palette, 0, outer_idx, 0x000F_0004, pal_buf);
                (Some(pal_ref), chain, 0u8)
            };

            // 2. Serialize UTexture export
            let tex_name = if tex.name.is_empty() {
                format!("Texture_{i}")
            } else {
                tex.name.clone()
            };
            let n_tex = pkg.names.add(tex_name, 0) as i32;
            let u_size = tex.width.max(1);
            let v_size = tex.height.max(1);
            let u_bits = (u_size.ilog2() & 0xFF) as u8;
            let v_bits = (v_size.ilog2() & 0xFF) as u8;

            let mut props = vec![
                TaggedProperty {
                    name: "UBits".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(u_bits),
                },
                TaggedProperty {
                    name: "VBits".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(v_bits),
                },
                TaggedProperty {
                    name: "USize".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(u_size as i32),
                },
                TaggedProperty {
                    name: "VSize".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(v_size as i32),
                },
                TaggedProperty {
                    name: "UClamp".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(u_size as i32),
                },
                TaggedProperty {
                    name: "VClamp".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(v_size as i32),
                },
                TaggedProperty {
                    name: "Format".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(format_byte),
                },
            ];
            if let Some(pref) = pal_exp_ref {
                props.push(TaggedProperty {
                    name: "Palette".to_string(),
                    array_index: 0,
                    value: TaggedValue::Object(pref),
                });
            }

            if tex.masked {
                props.push(TaggedProperty {
                    name: "bMasked".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                });
            }

            let mut tex_buf = Vec::new();
            write_tagged_properties(&mut tex_buf, &props, pkg)?;

            // Write full Mip pyramid (Mip 0..N down to 1x1)
            tex_buf.push(mip_chain.len() as u8);

            for (mip_w, mip_h, mip_indices) in &mip_chain {
                let mut size_ci = Vec::new();
                write_compact_index(&mut size_ci, mip_indices.len() as i32)?;
                let skip_offset = (pkg.raw_data.len()
                    + tex_buf.len()
                    + 4
                    + size_ci.len()
                    + mip_indices.len()) as i32;
                tex_buf.write_i32::<LittleEndian>(skip_offset)?;
                tex_buf.extend_from_slice(&size_ci);
                tex_buf.extend_from_slice(mip_indices);
                tex_buf.write_u32::<LittleEndian>(*mip_w)?;
                tex_buf.write_u32::<LittleEndian>(*mip_h)?;
                let u_bits = ((*mip_w).max(1).ilog2() & 0xFF) as u8;
                let v_bits = ((*mip_h).max(1).ilog2() & 0xFF) as u8;
                tex_buf.push(u_bits);
                tex_buf.push(v_bits);
            }

            pkg.add_export(n_tex, imp_texture, 0, outer_idx, 0x000F_0004, tex_buf);
        }

        Ok(())
    }

    /// Builds a standalone UE1 v68 `.utx` `Package` from `self.textures`.
    pub fn build_package(&self) -> io::Result<Package> {
        let mut pkg = Package::default();
        pkg.summary.file_version = 68;
        pkg.summary.pkg_flags = 0;
        pkg.summary.guid = [0x55, 0x54, 0x58, 0x39, 0x39, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];
        Self::write_textures_into_package(&self.textures, &mut pkg, 0)?;
        Ok(pkg)
    }

    /// Saves this `UtxPackage` to disk as a binary `.utx` file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let pkg = self.build_package()?;
        pkg.write_to_file(path)?;
        Ok(())
    }

    pub fn from_package(package: Package, package_name: String) -> anyhow::Result<Self> {
        // 1. First, decode all Palettes
        let mut palettes: HashMap<usize, Vec<[u8; 4]>> = HashMap::new();

        for (exp_idx, exp) in package.exports.entries.iter().enumerate() {
            let class_name = if exp.class_index < 0 {
                let imp = &package.imports.entries[(-exp.class_index - 1) as usize];
                package.get_name(imp.object_name.max(0) as usize).unwrap_or("")
            } else {
                ""
            };

            if class_name == "Palette" {
                let start = exp.serial_offset as usize;
                let end = start + exp.serial_size as usize;
                if end <= package.raw_data.len() && exp.serial_size >= 1024 {
                    let pal_data = &package.raw_data[start..end];
                    let mut pal_cur = Cursor::new(pal_data);
                    let offset = if read_tagged_properties(&mut pal_cur, &package).is_ok()
                        && read_compact_index(&mut pal_cur).is_ok()
                        && (pal_cur.position() as usize) + 1024 <= pal_data.len()
                    {
                        pal_cur.position() as usize
                    } else if pal_data.len() >= 1027 {
                        pal_data.len() - 1024
                    } else {
                        0
                    };
                    let mut colors = Vec::with_capacity(256);
                    for i in 0..256 {
                        let c_start = offset + i * 4;
                        if c_start + 4 <= pal_data.len() {
                            colors.push([
                                pal_data[c_start],
                                pal_data[c_start + 1],
                                pal_data[c_start + 2],
                                255, // Solid alpha for standard palettes
                            ]);
                        } else {
                            colors.push([0, 0, 0, 255]);
                        }
                    }
                    palettes.insert(exp_idx, colors);
                }
            }
        }

        // 2. Decode all Textures
        let mut textures = Vec::new();

        for exp in &package.exports.entries {
            let class_name = if exp.class_index < 0 {
                let imp = &package.imports.entries[(-exp.class_index - 1) as usize];
                package.get_name(imp.object_name.max(0) as usize).unwrap_or("")
            } else {
                ""
            };

            if class_name == "Texture" {
                let name = package
                    .get_name(exp.object_name.max(0) as usize)
                    .unwrap_or("Texture")
                    .to_string();

                let group = if exp.package_index > 0 {
                    let outer_idx = (exp.package_index - 1) as usize;
                    package
                        .exports
                        .entries
                        .get(outer_idx)
                        .and_then(|o| package.get_name(o.object_name.max(0) as usize))
                        .unwrap_or("None")
                        .to_string()
                } else {
                    "None".to_string()
                };

                let start = exp.serial_offset as usize;
                let end = start + exp.serial_size as usize;
                if end > package.raw_data.len() {
                    continue;
                }

                let data = &package.raw_data[start..end];
                let mut cursor = Cursor::new(data);

                if let Ok(props) = read_tagged_properties(&mut cursor, &package) {
                    let mut width = 256u32;
                    let mut height = 256u32;
                    let mut pal_ref = None;
                    let mut format = 0u32; // 0 = P8
                    let mut masked = false;

                    for (k, v) in &props {
                        match k.as_str() {
                            "USize" => {
                                if let TaggedValue::Int(w) = v.value {
                                    width = (w.clamp(1, 4096)) as u32;
                                }
                            }
                            "VSize" => {
                                if let TaggedValue::Int(h) = v.value {
                                    height = (h.clamp(1, 4096)) as u32;
                                }
                            }
                            "Palette" => {
                                if let TaggedValue::Object(idx) = v.value {
                                    if idx > 0 {
                                        pal_ref = Some((idx - 1) as usize);
                                    }
                                }
                            }
                            "Format" => {
                                if let TaggedValue::Byte(f) = v.value {
                                    format = f as u32;
                                }
                            }
                            "bMasked" => {
                                if let TaggedValue::Bool(b) = v.value {
                                    masked = b;
                                }
                            }
                            _ => {}
                        }
                    }

                    let pos = cursor.position() as usize;
                    let pixel_count = match (width as usize).checked_mul(height as usize) {
                        Some(cnt) if cnt <= 4096 * 4096 => cnt,
                        _ => continue,
                    };

                    let expected_mip0_bytes = match format {
                        3 => ((width as usize + 3) / 4) * ((height as usize + 3) / 4) * 8, // TEXF_DXT1
                        7 | 8 => ((width as usize + 3) / 4) * ((height as usize + 3) / 4) * 16, // TEXF_DXT3 / DXT5
                        1 | 5 => pixel_count * 4, // TEXF_RGBA7 / TEXF_RGBA8
                        4 => pixel_count * 3,     // TEXF_RGB8
                        2 => pixel_count * 2,     // TEXF_RGB16
                        _ => pixel_count,         // TEXF_P8 (0)
                    };

                    let mut mip0_start = pos + 1;
                    let mut mip0_byte_len = expected_mip0_bytes;
                    if package.summary.file_version >= 63 && pos + 5 < data.len() {
                        let mut lazy_cur = Cursor::new(&data[(pos + 1)..]);
                        if lazy_cur.read_i32::<LittleEndian>().is_ok() {
                            if let Ok(arr_len) = read_compact_index(&mut lazy_cur) {
                                let header_len = lazy_cur.position() as usize;
                                let arr_len_usize = arr_len.max(0) as usize;
                                if arr_len_usize > 0
                                    && pos + 1 + header_len + arr_len_usize <= data.len()
                                {
                                    mip0_start = pos + 1 + header_len;
                                    mip0_byte_len = arr_len_usize;
                                }
                            }
                        }
                    }

                    if mip0_start + mip0_byte_len <= data.len() {
                        let mip0_bytes = &data[mip0_start..(mip0_start + mip0_byte_len)];
                        let rgba_pixels = match format {
                            3 => decode_dxt1_to_rgba8(mip0_bytes, width, height)
                                .unwrap_or_else(|| vec![128; pixel_count * 4]),
                            7 => decode_dxt3_to_rgba8(mip0_bytes, width, height)
                                .unwrap_or_else(|| vec![128; pixel_count * 4]),
                            8 => decode_dxt5_to_rgba8(mip0_bytes, width, height)
                                .unwrap_or_else(|| vec![128; pixel_count * 4]),
                            1 | 5 if mip0_bytes.len() >= pixel_count * 4 => {
                                // BGRA8 / RGBA7 -> RGBA8
                                let mut out = Vec::with_capacity(pixel_count * 4);
                                for p in 0..pixel_count {
                                    let b = mip0_bytes[p * 4];
                                    let g = mip0_bytes[p * 4 + 1];
                                    let r = mip0_bytes[p * 4 + 2];
                                    let a = if format == 1 {
                                        mip0_bytes[p * 4 + 3].saturating_mul(2).min(255)
                                    } else {
                                        mip0_bytes[p * 4 + 3]
                                    };
                                    out.extend_from_slice(&[r, g, b, a]);
                                }
                                out
                            }
                            4 if mip0_bytes.len() >= pixel_count * 3 => {
                                // TEXF_RGB8 -> RGBA8
                                let mut out = Vec::with_capacity(pixel_count * 4);
                                for p in 0..pixel_count {
                                    let r = mip0_bytes[p * 3];
                                    let g = mip0_bytes[p * 3 + 1];
                                    let b = mip0_bytes[p * 3 + 2];
                                    out.extend_from_slice(&[r, g, b, 255]);
                                }
                                out
                            }
                            2 if mip0_bytes.len() >= pixel_count * 2 => {
                                // TEXF_RGB16 (5-6-5) -> RGBA8
                                let mut out = Vec::with_capacity(pixel_count * 4);
                                for p in 0..pixel_count {
                                    let v = u16::from_le_bytes([
                                        mip0_bytes[p * 2],
                                        mip0_bytes[p * 2 + 1],
                                    ]);
                                    let r = (((v >> 11) & 0x1F) * 255 / 31) as u8;
                                    let g = (((v >> 5) & 0x3F) * 255 / 63) as u8;
                                    let b = ((v & 0x1F) * 255 / 31) as u8;
                                    out.extend_from_slice(&[r, g, b, 255]);
                                }
                                out
                            }
                            _ => {
                                let mut out = Vec::with_capacity(pixel_count * 4);
                                let palette = pal_ref
                                    .and_then(|idx| palettes.get(&idx))
                                    .or_else(|| palettes.values().next());

                                if let Some(pal) = palette {
                                    for &idx in mip0_bytes.iter().take(pixel_count) {
                                        if masked && idx == 0 {
                                            let c = pal.get(0).unwrap_or(&[0, 0, 0, 0]);
                                            out.extend_from_slice(&[c[0], c[1], c[2], 0]);
                                        } else {
                                            let c = pal
                                                .get(idx as usize)
                                                .unwrap_or(&[128, 128, 128, 255]);
                                            out.extend_from_slice(c);
                                        }
                                    }
                                } else {
                                    for &idx in mip0_bytes.iter().take(pixel_count) {
                                        let alpha = if masked && idx == 0 { 0 } else { 255 };
                                        out.extend_from_slice(&[idx, idx, idx, alpha]);
                                    }
                                }
                                out
                            }
                        };

                        textures.push(UtxTexture {
                            name,
                            group,
                            width,
                            height,
                            format,
                            masked,
                            rgba_pixels,
                        });
                    }
                }
            }
        }

        Ok(Self {
            package_name,
            textures,
        })
    }
}


