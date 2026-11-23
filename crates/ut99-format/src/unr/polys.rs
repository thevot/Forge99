use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use glam::Vec3;
use std::io::{self, Cursor, Read, Write};

use crate::package::compact_index::{read_compact_index, write_compact_index};

#[derive(Debug, Clone, PartialEq)]
pub struct RawPoly {
    pub base: Vec3,
    pub normal: Vec3,
    pub texture_u: Vec3,
    pub texture_v: Vec3,
    pub vertices: Vec<Vec3>,
    pub poly_flags: u32,
    pub actor_ref: i32,
    pub texture_ref: i32,
    pub item_name: i32,
    pub link_index: i32,
    pub brush_poly_index: i32,
    pub pan_u: i16,
    pub pan_v: i16,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawPolys {
    pub polys: Vec<RawPoly>,
}

fn skip_state_frame(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() {
        return None;
    }
    let mut cur = Cursor::new(bytes);
    let node = read_compact_index(&mut cur).ok()?;
    let _state_node = read_compact_index(&mut cur).ok()?;
    let _probe_mask = cur.read_u64::<LittleEndian>().ok()?;
    let _latent_action = cur.read_i32::<LittleEndian>().ok()?;
    if node != 0 {
        let _script_offset = read_compact_index(&mut cur).ok()?;
    }
    Some(cur.position() as usize)
}

impl RawPolys {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        Self::from_bytes_with_flags(bytes, 0)
    }

    pub fn from_bytes_with_flags(bytes: &[u8], object_flags: u32) -> io::Result<Self> {
        if bytes.len() < 8 {
            return Ok(Self { polys: Vec::new() });
        }

        let check_count_at = |slice: &[u8]| -> Option<(i32, usize)> {
            if slice.len() >= 9 {
                let mut cur1 = Cursor::new(&slice[1..9]);
                if let (Ok(c1), Ok(m1)) = (cur1.read_i32::<LittleEndian>(), cur1.read_i32::<LittleEndian>()) {
                    if c1 == m1 && c1 >= 0 && c1 < 50_000 {
                        return Some((c1, 1));
                    }
                }
            }
            if slice.len() >= 8 {
                let mut cur0 = Cursor::new(&slice[0..8]);
                if let (Ok(c0), Ok(m0)) = (cur0.read_i32::<LittleEndian>(), cur0.read_i32::<LittleEndian>()) {
                    if c0 == m0 && c0 >= 0 && c0 < 50_000 {
                        return Some((c0, 0));
                    }
                }
            }
            None
        };

        let mut base_offset = 0;
        let mut array_offset = 0;
        let mut found_count = None;

        // 1. If RF_HasStack (0x02000000) is set, skip FStateFrame first
        if (object_flags & 0x02000000) != 0 {
            if let Some(pos) = skip_state_frame(bytes) {
                if pos <= bytes.len() {
                    if let Some((c, off)) = check_count_at(&bytes[pos..]) {
                        base_offset = pos;
                        array_offset = off;
                        found_count = Some(c);
                    }
                }
            }
        }

        // 2. Try regular offset 0 or 1 from start
        if found_count.is_none() {
            if let Some((c, off)) = check_count_at(bytes) {
                base_offset = 0;
                array_offset = off;
                found_count = Some(c);
            }
        }

        // 3. Fallback: try skipping FStateFrame even if object_flags was 0
        if found_count.is_none() {
            if let Some(pos) = skip_state_frame(bytes) {
                if pos <= bytes.len() {
                    if let Some((c, off)) = check_count_at(&bytes[pos..]) {
                        base_offset = pos;
                        array_offset = off;
                        found_count = Some(c);
                    }
                }
            }
        }

        let count = match found_count {
            Some(c) => c,
            None => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid poly count")),
        };

        let mut reader = Cursor::new(&bytes[base_offset + array_offset..]);
        let _count = reader.read_i32::<LittleEndian>()?;
        let _max_count = reader.read_i32::<LittleEndian>()?;

        let mut polys = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let num_vertices_raw = read_compact_index(&mut reader)?;
            if num_vertices_raw <= 0 || num_vertices_raw > 64 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid vertex count"));
            }
            let num_vertices = num_vertices_raw as usize;

            let base = read_vec3(&mut reader)?;
            let normal = read_vec3(&mut reader)?;
            let texture_u = read_vec3(&mut reader)?;
            let texture_v = read_vec3(&mut reader)?;

            let mut vertices = Vec::with_capacity(num_vertices);
            for _ in 0..num_vertices {
                vertices.push(read_vec3(&mut reader)?);
            }

            let poly_flags = reader.read_u32::<LittleEndian>()?;
            let actor_ref = read_compact_index(&mut reader)?;
            let texture_ref = read_compact_index(&mut reader)?;
            let item_name = read_compact_index(&mut reader)?;
            let link_index = read_compact_index(&mut reader)?;
            let brush_poly_index = read_compact_index(&mut reader)?;
            let pan_u = reader.read_i16::<LittleEndian>()?;
            let pan_v = reader.read_i16::<LittleEndian>()?;

            polys.push(RawPoly {
                base,
                normal,
                texture_u,
                texture_v,
                vertices,
                poly_flags,
                actor_ref,
                texture_ref,
                item_name,
                link_index,
                brush_poly_index,
                pan_u,
                pan_v,
            });
        }

        Ok(Self { polys })
    }

    pub fn read<R: Read>(_reader: &mut R) -> io::Result<Self> {
        let mut buf = Vec::new();
        _reader.read_to_end(&mut buf)?;
        Self::from_bytes(&buf)
    }

    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        // Leading byte 0 (TTransArray prefix)
        buf.push(0u8);
        let count = self.polys.len() as i32;
        buf.write_i32::<LittleEndian>(count)?;
        buf.write_i32::<LittleEndian>(count)?;

        for p in &self.polys {
            write_compact_index(&mut buf, p.vertices.len() as i32)?;
            write_vec3(&mut buf, p.base)?;
            write_vec3(&mut buf, p.normal)?;
            write_vec3(&mut buf, p.texture_u)?;
            write_vec3(&mut buf, p.texture_v)?;
            for &v in &p.vertices {
                write_vec3(&mut buf, v)?;
            }
            buf.write_u32::<LittleEndian>(p.poly_flags)?;
            write_compact_index(&mut buf, p.actor_ref)?;
            write_compact_index(&mut buf, p.texture_ref)?;
            write_compact_index(&mut buf, p.item_name)?;
            write_compact_index(&mut buf, p.link_index)?;
            write_compact_index(&mut buf, p.brush_poly_index)?;
            buf.write_i16::<LittleEndian>(p.pan_u)?;
            buf.write_i16::<LittleEndian>(p.pan_v)?;
        }

        Ok(buf)
    }
}


fn read_vec3<R: Read>(reader: &mut R) -> io::Result<Vec3> {
    let x = reader.read_f32::<LittleEndian>()?;
    let y = reader.read_f32::<LittleEndian>()?;
    let z = reader.read_f32::<LittleEndian>()?;
    Ok(Vec3::new(x, y, z))
}

fn write_vec3<W: Write>(writer: &mut W, v: Vec3) -> io::Result<()> {
    writer.write_f32::<LittleEndian>(v.x)?;
    writer.write_f32::<LittleEndian>(v.y)?;
    writer.write_f32::<LittleEndian>(v.z)?;
    Ok(())
}
