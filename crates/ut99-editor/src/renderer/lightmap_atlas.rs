use std::collections::{HashMap, HashSet};

pub const LIGHTMAP_ATLAS_DIM: u32 = 2048;
pub const MAX_LIGHTMAP_TILE_DIM: usize = 256;

pub fn lightmap_content_key(lm: &ut99_bsp::LightmapData) -> Option<u64> {
    let count = lm.width.checked_mul(lm.height)?;
    if lm.width == 0 || lm.height == 0 || lm.luxels.len() < count {
        return None;
    }
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    lm.width.hash(&mut h);
    lm.height.hash(&mut h);
    for lx in &lm.luxels[..count] {
        ((lx[0].clamp(0.0, 1.0) * 255.0) as u8).hash(&mut h);
        ((lx[1].clamp(0.0, 1.0) * 255.0) as u8).hash(&mut h);
        ((lx[2].clamp(0.0, 1.0) * 255.0) as u8).hash(&mut h);
    }
    Some(h.finish())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightmapAtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub u_min: f32,
    pub u_max: f32,
    pub v_min: f32,
    pub v_max: f32,
}

impl LightmapAtlasRect {
    #[inline]
    pub fn from_outer_origin(tile_x: u32, tile_y: u32, inner_w: u32, inner_h: u32, atlas_dim: u32) -> Self {
        let dim = atlas_dim as f32;
        Self {
            x: tile_x,
            y: tile_y,
            width: inner_w,
            height: inner_h,
            u_min: (tile_x as f32 + 1.5) / dim,
            u_max: (tile_x as f32 + 0.5 + inner_w as f32) / dim,
            v_min: (tile_y as f32 + 1.5) / dim,
            v_max: (tile_y as f32 + 0.5 + inner_h as f32) / dim,
        }
    }

    #[inline]
    pub fn map_uv(&self, u: f32, v: f32) -> [f32; 2] {
        let u = u.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        [
            self.u_min + (self.u_max - self.u_min) * u,
            self.v_min + (self.v_max - self.v_min) * v,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AtlasShelf {
    y: u32,
    height: u32,
    next_x: u32,
}

#[derive(Debug, Default)]
struct ShelfAllocator {
    dim: u32,
    shelves: Vec<AtlasShelf>,
    next_y: u32,
}

impl ShelfAllocator {
    fn new(dim: u32) -> Self {
        Self {
            dim,
            shelves: Vec::new(),
            next_y: 0,
        }
    }

    fn reset(&mut self) {
        self.shelves.clear();
        self.next_y = 0;
    }

    fn allocate(&mut self, tile_w: u32, tile_h: u32) -> Option<(u32, u32)> {
        if tile_w > self.dim || tile_h > self.dim {
            return None;
        }
        // 1. Best-fit shelf within 2x height to limit vertical waste.
        let best_tight = self
            .shelves
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.height >= tile_h
                    && s.height <= tile_h.saturating_mul(2)
                    && s.next_x.saturating_add(tile_w) <= self.dim
            })
            .min_by_key(|(_, s)| s.height)
            .map(|(i, _)| i);
        if let Some(idx) = best_tight {
            let shelf = &mut self.shelves[idx];
            let pos = (shelf.next_x, shelf.y);
            shelf.next_x += tile_w;
            return Some(pos);
        }

        // 2. Open a new shelf if vertical space remains.
        if self.next_y.saturating_add(tile_h) <= self.dim {
            let y = self.next_y;
            self.next_y += tile_h;
            self.shelves.push(AtlasShelf {
                y,
                height: tile_h,
                next_x: tile_w,
            });
            return Some((0, y));
        }

        // 3. Fallback to any remaining shelf that can fit tile_h x tile_w.
        let best_any = self
            .shelves
            .iter()
            .enumerate()
            .filter(|(_, s)| s.height >= tile_h && s.next_x.saturating_add(tile_w) <= self.dim)
            .min_by_key(|(_, s)| s.height)
            .map(|(i, _)| i);
        if let Some(idx) = best_any {
            let shelf = &mut self.shelves[idx];
            let pos = (shelf.next_x, shelf.y);
            shelf.next_x += tile_w;
            return Some(pos);
        }

        None
    }
}

pub struct LightmapAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    dim: u32,
    allocator: ShelfAllocator,
    dead_texels: u64,
    entries: HashMap<u64, LightmapAtlasRect>,
    upload_scratch: Vec<u8>,
}

impl LightmapAtlas {
    pub fn new(device: &wgpu::Device, dim: u32) -> Self {
        Self::new_with_usage(device, dim, wgpu::TextureUsages::empty())
    }

    pub fn new_with_usage(
        device: &wgpu::Device,
        dim: u32,
        extra_usage: wgpu::TextureUsages,
    ) -> Self {
        let max_dim = device.limits().max_texture_dimension_2d.max(16);
        let dim = dim.clamp(16, max_dim);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dedicated 2D Lightmap Atlas"),
            size: wgpu::Extent3d {
                width: dim,
                height: dim,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | extra_usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Dedicated 2D Lightmap Atlas View"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Lightmap Atlas Clamp Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
            dim,
            allocator: ShelfAllocator::new(dim),
            dead_texels: 0,
            entries: HashMap::new(),
            upload_scratch: Vec::new(),
        }
    }

    #[inline]
    pub fn dim(&self) -> u32 {
        self.dim
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    pub fn contains_key(&self, key: u64) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &u64> {
        self.entries.keys()
    }

    #[inline]
    pub fn get(&self, key: u64) -> Option<LightmapAtlasRect> {
        self.entries.get(&key).copied()
    }

    #[inline]
    pub fn vram_bytes(&self) -> usize {
        (self.dim as usize) * (self.dim as usize) * 4
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.allocator.reset();
        self.dead_texels = 0;
    }

    pub fn retain_live(&mut self, live: &HashSet<u64>, new_required_texels: u64) {
        let mut removed_texels = 0u64;
        self.entries.retain(|key, rect| {
            if live.contains(key) {
                true
            } else {
                removed_texels += (rect.width as u64 + 2) * (rect.height as u64 + 2);
                false
            }
        });
        self.dead_texels = self.dead_texels.saturating_add(removed_texels);
        let capacity_texels = (self.dim as u64) * (self.dim as u64);
        let used_shelf_texels = (self.allocator.next_y as u64) * (self.dim as u64);
        let remaining_texels = capacity_texels.saturating_sub(used_shelf_texels);
        if self.entries.is_empty()
            || self.dead_texels > capacity_texels / 4
            || (self.dead_texels > 0 && remaining_texels < new_required_texels)
        {
            self.clear();
        }
    }

    pub fn resolve_or_upload(
        &mut self,
        queue: &wgpu::Queue,
        lm: &ut99_bsp::LightmapData,
        key: u64,
    ) -> Option<LightmapAtlasRect> {
        if let Some(&rect) = self.entries.get(&key) {
            return Some(rect);
        }
        if lm.width == 0
            || lm.height == 0
            || lm.width.checked_mul(lm.height).map_or(true, |c| lm.luxels.len() < c)
        {
            return None;
        }
        let inner_w = lm.width.min(MAX_LIGHTMAP_TILE_DIM) as u32;
        let inner_h = lm.height.min(MAX_LIGHTMAP_TILE_DIM) as u32;
        let tile_w = inner_w + 2;
        let tile_h = inner_h + 2;
        let (tile_x, tile_y) = self.allocator.allocate(tile_w, tile_h)?;

        pack_tile_with_gutter(lm, inner_w, inner_h, &mut self.upload_scratch);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: tile_x,
                    y: tile_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &self.upload_scratch,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(tile_w * 4),
                rows_per_image: Some(tile_h),
            },
            wgpu::Extent3d {
                width: tile_w,
                height: tile_h,
                depth_or_array_layers: 1,
            },
        );

        let rect = LightmapAtlasRect::from_outer_origin(tile_x, tile_y, inner_w, inner_h, self.dim);
        self.entries.insert(key, rect);
        Some(rect)
    }
}

fn pack_tile_with_gutter(
    lm: &ut99_bsp::LightmapData,
    inner_w: u32,
    inner_h: u32,
    scratch: &mut Vec<u8>,
) {
    let tile_w = (inner_w + 2) as usize;
    let tile_h = (inner_h + 2) as usize;
    let iw = inner_w as usize;
    let ih = inner_h as usize;
    let stride = tile_w * 4;
    scratch.resize(stride * tile_h, 255);

    if lm.width == iw && lm.height == ih {
        for y in 0..ih {
            let src_row = &lm.luxels[y * iw..(y + 1) * iw];
            let dst_off = (y + 1) * stride + 4;
            for (x, lx) in src_row.iter().enumerate() {
                let p = dst_off + x * 4;
                scratch[p] = (lx[0].clamp(0.0, 1.0) * 255.0) as u8;
                scratch[p + 1] = (lx[1].clamp(0.0, 1.0) * 255.0) as u8;
                scratch[p + 2] = (lx[2].clamp(0.0, 1.0) * 255.0) as u8;
                scratch[p + 3] = 255;
            }
        }
    } else {
        let src_w_span = (lm.width - 1) as f32;
        let src_h_span = (lm.height - 1) as f32;
        for y in 0..ih {
            let fy = if ih > 1 {
                ((y as f32 + 0.5) / ih as f32) * src_h_span
            } else {
                0.0
            };
            let y0 = (fy.floor() as usize).min(lm.height - 1);
            let y1 = (y0 + 1).min(lm.height - 1);
            let ty = fy - y0 as f32;
            let dst_off = (y + 1) * stride + 4;
            for x in 0..iw {
                let fx = if iw > 1 {
                    ((x as f32 + 0.5) / iw as f32) * src_w_span
                } else {
                    0.0
                };
                let x0 = (fx.floor() as usize).min(lm.width - 1);
                let x1 = (x0 + 1).min(lm.width - 1);
                let tx = fx - x0 as f32;

                let c00 = lm.luxels[y0 * lm.width + x0];
                let c10 = lm.luxels[y0 * lm.width + x1];
                let c01 = lm.luxels[y1 * lm.width + x0];
                let c11 = lm.luxels[y1 * lm.width + x1];
                let p = dst_off + x * 4;
                for ch in 0..3 {
                    let top = c00[ch] * (1.0 - tx) + c10[ch] * tx;
                    let bot = c01[ch] * (1.0 - tx) + c11[ch] * tx;
                    scratch[p + ch] = ((top * (1.0 - ty) + bot * ty).clamp(0.0, 1.0) * 255.0) as u8;
                }
                scratch[p + 3] = 255;
            }
        }
    }

    // Top gutter row (y = 0) mirrors y = 1; bottom gutter row (y = ih + 1) mirrors y = ih.
    let (row0, rest) = scratch.split_at_mut(stride);
    row0[4..4 + iw * 4].copy_from_slice(&rest[4..4 + iw * 4]);
    let last_inner_off = ih * stride;
    let bot_off = (ih + 1) * stride;
    scratch.copy_within(last_inner_off + 4..last_inner_off + 4 + iw * 4, bot_off + 4);

    // Left (x = 0) and right (x = iw + 1) gutter columns across all rows including 4 corners.
    for y in 0..tile_h {
        let row_off = y * stride;
        scratch.copy_within(row_off + 4..row_off + 8, row_off);
        let src_right = row_off + iw * 4;
        let dst_right = row_off + (iw + 1) * 4;
        scratch.copy_within(src_right..src_right + 4, dst_right);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lm_solid(w: usize, h: usize, rgb: [f32; 3]) -> ut99_bsp::LightmapData {
        ut99_bsp::LightmapData {
            width: w,
            height: h,
            luxels: vec![rgb; w * h],
        }
    }

    #[test]
    fn shelf_packing_bounds_and_uv_centers() {
        let mut alloc = ShelfAllocator::new(64);
        let mut rects = Vec::new();
        // Pack 6x6 lightmaps (8x8 padded tiles) -> 8 rows x 8 cols = 64 tiles exactly.
        for _ in 0..64 {
            let (x, y) = alloc.allocate(8, 8).expect("64 tiles fit in 64x64");
            rects.push(LightmapAtlasRect::from_outer_origin(x, y, 6, 6, 64));
        }
        assert_eq!(alloc.allocate(8, 8), None);
        for (i, a) in rects.iter().enumerate() {
            assert!(a.x + a.width + 2 <= 64);
            assert!(a.y + a.height + 2 <= 64);
            assert_eq!(a.map_uv(0.0, 0.0), [(a.x as f32 + 1.5) / 64.0, (a.y as f32 + 1.5) / 64.0]);
            assert_eq!(
                a.map_uv(1.0, 1.0),
                [(a.x as f32 + 6.5) / 64.0, (a.y as f32 + 6.5) / 64.0]
            );
            for b in &rects[i + 1..] {
                let no_overlap = a.x + a.width + 2 <= b.x
                    || b.x + b.width + 2 <= a.x
                    || a.y + a.height + 2 <= b.y
                    || b.y + b.height + 2 <= a.y;
                assert!(no_overlap, "rects {a:?} and {b:?} overlapped");
            }
        }
    }

    #[test]
    fn gutter_packing_handles_1x1_4x4_and_oversized() {
        let mut scratch = Vec::new();
        // 1x1 lightmap -> 3x3 padded tile filled identically with the single luxel
        let lm1 = lm_solid(1, 1, [0.5, 0.25, 1.0]);
        pack_tile_with_gutter(&lm1, 1, 1, &mut scratch);
        assert_eq!(scratch.len(), 3 * 3 * 4);
        for px in scratch.chunks_exact(4) {
            assert_eq!(px, [127, 63, 255, 255]);
        }
        let rect1 = LightmapAtlasRect::from_outer_origin(0, 0, 1, 1, 64);
        assert_eq!(rect1.map_uv(0.0, 0.0), [1.5 / 64.0, 1.5 / 64.0]);
        assert_eq!(rect1.map_uv(1.0, 1.0), [1.5 / 64.0, 1.5 / 64.0]);

        // 4x4 distinct corners -> verify outer gutter + corner clamping
        let mut luxels = Vec::with_capacity(16);
        for y in 0..4 {
            for x in 0..4 {
                luxels.push([(x as f32 * 60.0) / 255.0, (y as f32 * 60.0) / 255.0, 200.0 / 255.0]);
            }
        }
        let lm4 = ut99_bsp::LightmapData { width: 4, height: 4, luxels };
        pack_tile_with_gutter(&lm4, 4, 4, &mut scratch);
        assert_eq!(scratch.len(), 6 * 6 * 4);
        let at = |tx: usize, ty: usize| -> [u8; 4] {
            let off = (ty * 6 + tx) * 4;
            scratch[off..off + 4].try_into().unwrap()
        };
        assert_eq!(at(0, 0), at(1, 1)); // TL corner == inner (0,0)
        assert_eq!(at(5, 0), at(4, 1)); // TR corner == inner (3,0)
        assert_eq!(at(0, 5), at(1, 4)); // BL corner == inner (0,3)
        assert_eq!(at(5, 5), at(4, 4)); // BR corner == inner (3,3)
        assert_eq!(at(3, 0), at(3, 1)); // top edge matches row 1
        assert_eq!(at(5, 2), at(4, 2)); // right edge matches col 4

        // Oversized (>256) downsamples cleanly to MAX_LIGHTMAP_TILE_DIM + 2
        let oversized = lm_solid(300, 10, [1.0, 0.0, 0.5]);
        let iw = oversized.width.min(MAX_LIGHTMAP_TILE_DIM) as u32;
        let ih = oversized.height.min(MAX_LIGHTMAP_TILE_DIM) as u32;
        pack_tile_with_gutter(&oversized, iw, ih, &mut scratch);
        assert_eq!(scratch.len(), (iw as usize + 2) * (ih as usize + 2) * 4);
        assert_eq!(&scratch[0..4], &[255, 0, 127, 255]);
    }
}
