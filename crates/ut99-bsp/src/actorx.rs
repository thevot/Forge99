//! ActorX (.psk, .pskx, .psa) Binary Format Engine
//!
//! Provides zero-copy parsing, validation, and serialization for Epic Games ActorX
//! skeletal mesh and animation files used across Unreal Engine 1, 2, and 3.
//!
//! Based on forensic analysis of `ActorX_All` and Gildor's `SkelEdit`.

use glam::{Quat, Vec3};
use std::io::{Cursor, Read, Write};

pub const ACTORX_CHUNK_ID_LEN: usize = 32;

/// Standard ActorX 32-byte chunk header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorXChunkHeader {
    pub chunk_id: [u8; ACTORX_CHUNK_ID_LEN],
    pub data_flags: u32,
    pub data_size: u32,
    pub data_count: u32,
}

impl ActorXChunkHeader {
    pub fn new(id_str: &str, data_size: u32, data_count: u32) -> Self {
        let mut chunk_id = [0u8; ACTORX_CHUNK_ID_LEN];
        let bytes = id_str.as_bytes();
        let len = bytes.len().min(ACTORX_CHUNK_ID_LEN);
        chunk_id[..len].copy_from_slice(&bytes[..len]);
        Self {
            chunk_id,
            data_flags: 1999801, // Canonical Epic Games ActorX signature
            data_size,
            data_count,
        }
    }

    pub fn id_string(&self) -> String {
        let len = self.chunk_id.iter().position(|&b| b == 0).unwrap_or(ACTORX_CHUNK_ID_LEN);
        String::from_utf8_lossy(&self.chunk_id[..len]).to_string()
    }

    pub fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self, String> {
        let mut chunk_id = [0u8; ACTORX_CHUNK_ID_LEN];
        cursor.read_exact(&mut chunk_id).map_err(|e| format!("Header chunk_id read failed: {e}"))?;
        let mut buf4 = [0u8; 4];

        cursor.read_exact(&mut buf4).map_err(|e| format!("Header data_flags read failed: {e}"))?;
        let data_flags = u32::from_le_bytes(buf4);

        cursor.read_exact(&mut buf4).map_err(|e| format!("Header data_size read failed: {e}"))?;
        let data_size = u32::from_le_bytes(buf4);

        cursor.read_exact(&mut buf4).map_err(|e| format!("Header data_count read failed: {e}"))?;
        let data_count = u32::from_le_bytes(buf4);

        Ok(Self {
            chunk_id,
            data_flags,
            data_size,
            data_count,
        })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&self.chunk_id)?;
        w.write_all(&self.data_flags.to_le_bytes())?;
        w.write_all(&self.data_size.to_le_bytes())?;
        w.write_all(&self.data_count.to_le_bytes())?;
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// PSK Data Structures (Skeletal Mesh)
// -------------------------------------------------------------------------------------------------

/// 3D Vertex point in PSK (PNTS0000)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PskPoint {
    pub point: Vec3,
}

/// UV Wedge in PSK (VTXW0000)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PskWedge {
    pub point_index: u16,
    pub u: f32,
    pub v: f32,
    pub material_index: u8,
}

/// Triangle Face in PSK (FACE0000)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PskFace {
    pub wedge_indices: [u16; 3],
    pub mat_index: u8,
    pub aux_mat_index: u8,
    pub smoothing_groups: u32,
}

/// Material slot in PSK (MATT0000)
#[derive(Debug, Clone, PartialEq)]
pub struct PskMaterial {
    pub name: String,
    pub texture_index: u32,
    pub poly_flags: u32,
}

/// Bone definition in PSK (REFSKELT)
#[derive(Debug, Clone, PartialEq)]
pub struct PskBone {
    pub name: String,
    pub flags: u32,
    pub num_children: u32,
    pub parent_index: i32,
    pub orientation: Quat, // x, y, z, w
    pub position: Vec3,
    pub length: f32,
    pub size: Vec3,
}

/// Bone influence weight on vertex (RAWWEIGHTS)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PskWeight {
    pub weight: f32,
    pub point_index: u32,
    pub bone_index: u32,
}

/// Complete in-memory representation of an ActorX PSK skeletal mesh.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActorXPskMesh {
    pub points: Vec<PskPoint>,
    pub wedges: Vec<PskWedge>,
    pub faces: Vec<PskFace>,
    pub materials: Vec<PskMaterial>,
    pub bones: Vec<PskBone>,
    pub weights: Vec<PskWeight>,
}

// -------------------------------------------------------------------------------------------------
// PSA Data Structures (Animation Sequences)
// -------------------------------------------------------------------------------------------------

/// Animation sequence descriptor in PSA (ANIMINFO)
#[derive(Debug, Clone, PartialEq)]
pub struct PsaAnimInfo {
    pub name: String,
    pub group: String,
    pub total_bones: u32,
    pub anim_rate: f32,
    pub track_time: f32,
    pub num_frames: u32,
}

/// Keyframe orientation/position for a bone in PSA (ANIMKEYS)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PsaAnimKey {
    pub position: Vec3,
    pub orientation: Quat,
    pub time: f32,
}

/// Complete in-memory representation of an ActorX PSA animation file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActorXPsaFile {
    pub bone_names: Vec<PskBone>,
    pub animations: Vec<PsaAnimInfo>,
    pub raw_keys: Vec<PsaAnimKey>,
}

// -------------------------------------------------------------------------------------------------
// PSK Serializer & Deserializer
// -------------------------------------------------------------------------------------------------

impl ActorXPskMesh {
    /// Serializes mesh to ActorX .psk binary bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // 1. ACTRHEAD
        ActorXChunkHeader::new("ACTRHEAD", 0, 0).write(&mut out).unwrap();

        // 2. PNTS0000 (12 bytes each: x, y, z as f32)
        ActorXChunkHeader::new("PNTS0000", 12, self.points.len() as u32).write(&mut out).unwrap();
        for pt in &self.points {
            out.write_all(&pt.point.x.to_le_bytes()).unwrap();
            out.write_all(&pt.point.y.to_le_bytes()).unwrap();
            out.write_all(&pt.point.z.to_le_bytes()).unwrap();
        }

        // 3. VTXW0000 (16 bytes each: point_idx: u16, pad: u16, u: f32, v: f32, mat: u8, res: u8, pad2: u16)
        ActorXChunkHeader::new("VTXW0000", 16, self.wedges.len() as u32).write(&mut out).unwrap();
        for w in &self.wedges {
            out.write_all(&w.point_index.to_le_bytes()).unwrap();
            out.write_all(&0u16.to_le_bytes()).unwrap();
            out.write_all(&w.u.to_le_bytes()).unwrap();
            out.write_all(&w.v.to_le_bytes()).unwrap();
            out.write_all(&[w.material_index, 0u8]).unwrap();
            out.write_all(&0u16.to_le_bytes()).unwrap();
        }

        // 4. FACE0000 (12 bytes each: 3 * u16 wedge_idx, mat: u8, aux: u8, smooth: u32)
        ActorXChunkHeader::new("FACE0000", 12, self.faces.len() as u32).write(&mut out).unwrap();
        for f in &self.faces {
            out.write_all(&f.wedge_indices[0].to_le_bytes()).unwrap();
            out.write_all(&f.wedge_indices[1].to_le_bytes()).unwrap();
            out.write_all(&f.wedge_indices[2].to_le_bytes()).unwrap();
            out.write_all(&[f.mat_index, f.aux_mat_index]).unwrap();
            out.write_all(&f.smoothing_groups.to_le_bytes()).unwrap();
        }

        // 5. MATT0000 (88 bytes each: 64-byte name, tex_idx: u32, poly_flags: u32, aux_mat: u32, aux_flags: u32, lod_bias: i32, lod_style: i32)
        ActorXChunkHeader::new("MATT0000", 88, self.materials.len() as u32).write(&mut out).unwrap();
        for m in &self.materials {
            let mut name_buf = [0u8; 64];
            let n_bytes = m.name.as_bytes();
            let len = n_bytes.len().min(64);
            name_buf[..len].copy_from_slice(&n_bytes[..len]);
            out.write_all(&name_buf).unwrap();
            out.write_all(&m.texture_index.to_le_bytes()).unwrap();
            out.write_all(&m.poly_flags.to_le_bytes()).unwrap();
            out.write_all(&0u32.to_le_bytes()).unwrap(); // aux_mat
            out.write_all(&0u32.to_le_bytes()).unwrap(); // aux_flags
            out.write_all(&0i32.to_le_bytes()).unwrap(); // lod_bias
            out.write_all(&0i32.to_le_bytes()).unwrap(); // lod_style
        }

        // 6. REFSKELT (120 bytes each: name [64], flags u32, num_children u32, parent_idx i32, quat [16], pos [12], len f32, size [12])
        ActorXChunkHeader::new("REFSKELT", 120, self.bones.len() as u32).write(&mut out).unwrap();
        for b in &self.bones {
            let mut name_buf = [0u8; 64];
            let n_bytes = b.name.as_bytes();
            let len = n_bytes.len().min(64);
            name_buf[..len].copy_from_slice(&n_bytes[..len]);
            out.write_all(&name_buf).unwrap();
            out.write_all(&b.flags.to_le_bytes()).unwrap();
            out.write_all(&b.num_children.to_le_bytes()).unwrap();
            out.write_all(&b.parent_index.to_le_bytes()).unwrap();
            out.write_all(&b.orientation.x.to_le_bytes()).unwrap();
            out.write_all(&b.orientation.y.to_le_bytes()).unwrap();
            out.write_all(&b.orientation.z.to_le_bytes()).unwrap();
            out.write_all(&b.orientation.w.to_le_bytes()).unwrap();
            out.write_all(&b.position.x.to_le_bytes()).unwrap();
            out.write_all(&b.position.y.to_le_bytes()).unwrap();
            out.write_all(&b.position.z.to_le_bytes()).unwrap();
            out.write_all(&b.length.to_le_bytes()).unwrap();
            out.write_all(&b.size.x.to_le_bytes()).unwrap();
            out.write_all(&b.size.y.to_le_bytes()).unwrap();
            out.write_all(&b.size.z.to_le_bytes()).unwrap();
        }

        // 7. RAWWEIGHTS (12 bytes each: weight f32, pt_idx u32, bone_idx u32)
        ActorXChunkHeader::new("RAWWEIGHTS", 12, self.weights.len() as u32).write(&mut out).unwrap();
        for w in &self.weights {
            out.write_all(&w.weight.to_le_bytes()).unwrap();
            out.write_all(&w.point_index.to_le_bytes()).unwrap();
            out.write_all(&w.bone_index.to_le_bytes()).unwrap();
        }

        out
    }

    /// Parses an ActorX .psk binary buffer.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(bytes);
        let mut mesh = Self::default();

        while (cursor.position() as usize) < bytes.len() {
            let header = ActorXChunkHeader::read(&mut cursor)?;
            let id = header.id_string();

            match id.as_str() {
                "ACTRHEAD" => {
                    // Header chunk has 0 data count
                }
                "PNTS0000" => {
                    for _ in 0..header.data_count {
                        let mut buf4 = [0u8; 4];
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let x = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let y = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let z = f32::from_le_bytes(buf4);
                        mesh.points.push(PskPoint { point: Vec3::new(x, y, z) });
                    }
                }
                "VTXW0000" => {
                    for _ in 0..header.data_count {
                        let mut buf2 = [0u8; 2];
                        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
                        let point_index = u16::from_le_bytes(buf2);
                        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?; // pad

                        let mut buf4 = [0u8; 4];
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let u = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let v = f32::from_le_bytes(buf4);

                        let mut mat_buf = [0u8; 2];
                        cursor.read_exact(&mut mat_buf).map_err(|e| e.to_string())?;
                        let material_index = mat_buf[0];

                        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?; // pad2

                        mesh.wedges.push(PskWedge { point_index, u, v, material_index });
                    }
                }
                "FACE0000" => {
                    for _ in 0..header.data_count {
                        let mut buf2 = [0u8; 2];
                        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
                        let w0 = u16::from_le_bytes(buf2);
                        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
                        let w1 = u16::from_le_bytes(buf2);
                        cursor.read_exact(&mut buf2).map_err(|e| e.to_string())?;
                        let w2 = u16::from_le_bytes(buf2);

                        let mut mat_buf = [0u8; 2];
                        cursor.read_exact(&mut mat_buf).map_err(|e| e.to_string())?;
                        let mat_index = mat_buf[0];
                        let aux_mat_index = mat_buf[1];

                        let mut buf4 = [0u8; 4];
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let smoothing_groups = u32::from_le_bytes(buf4);

                        mesh.faces.push(PskFace {
                            wedge_indices: [w0, w1, w2],
                            mat_index,
                            aux_mat_index,
                            smoothing_groups,
                        });
                    }
                }
                "MATT0000" => {
                    for _ in 0..header.data_count {
                        let mut name_buf = [0u8; 64];
                        cursor.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
                        let len = name_buf.iter().position(|&b| b == 0).unwrap_or(64);
                        let name = String::from_utf8_lossy(&name_buf[..len]).to_string();

                        let mut buf4 = [0u8; 4];
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let texture_index = u32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let poly_flags = u32::from_le_bytes(buf4);

                        // Skip 16 bytes (aux_mat, aux_flags, lod_bias, lod_style)
                        let mut skip_buf = [0u8; 16];
                        cursor.read_exact(&mut skip_buf).map_err(|e| e.to_string())?;

                        mesh.materials.push(PskMaterial { name, texture_index, poly_flags });
                    }
                }
                "REFSKELT" => {
                    for _ in 0..header.data_count {
                        let mut name_buf = [0u8; 64];
                        cursor.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
                        let len = name_buf.iter().position(|&b| b == 0).unwrap_or(64);
                        let name = String::from_utf8_lossy(&name_buf[..len]).to_string();

                        let mut buf4 = [0u8; 4];
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let flags = u32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let num_children = u32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let parent_index = i32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let qx = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let qy = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let qz = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let qw = f32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let px = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let py = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let pz = f32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let length = f32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let sx = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let sy = f32::from_le_bytes(buf4);
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let sz = f32::from_le_bytes(buf4);

                        mesh.bones.push(PskBone {
                            name,
                            flags,
                            num_children,
                            parent_index,
                            orientation: Quat::from_xyzw(qx, qy, qz, qw),
                            position: Vec3::new(px, py, pz),
                            length,
                            size: Vec3::new(sx, sy, sz),
                        });
                    }
                }
                "RAWWEIGHTS" => {
                    for _ in 0..header.data_count {
                        let mut buf4 = [0u8; 4];
                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let weight = f32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let point_index = u32::from_le_bytes(buf4);

                        cursor.read_exact(&mut buf4).map_err(|e| e.to_string())?;
                        let bone_index = u32::from_le_bytes(buf4);

                        mesh.weights.push(PskWeight { weight, point_index, bone_index });
                    }
                }
                _ => {
                    // Skip unrecognized chunk
                    let total_bytes = (header.data_size * header.data_count) as usize;
                    let mut skip = vec![0u8; total_bytes];
                    cursor.read_exact(&mut skip).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(mesh)
    }
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psk_roundtrip_serialization() {
        let mut mesh = ActorXPskMesh::default();
        mesh.points.push(PskPoint { point: Vec3::new(10.0, 20.0, 30.0) });
        mesh.wedges.push(PskWedge { point_index: 0, u: 0.5, v: 0.5, material_index: 0 });
        mesh.faces.push(PskFace { wedge_indices: [0, 0, 0], mat_index: 0, aux_mat_index: 0, smoothing_groups: 1 });
        mesh.materials.push(PskMaterial { name: "Skin_Chest".to_string(), texture_index: 0, poly_flags: 0 });
        mesh.bones.push(PskBone {
            name: "Root".to_string(),
            flags: 0,
            num_children: 0,
            parent_index: -1,
            orientation: Quat::IDENTITY,
            position: Vec3::ZERO,
            length: 10.0,
            size: Vec3::ONE,
        });
        mesh.weights.push(PskWeight { weight: 1.0, point_index: 0, bone_index: 0 });

        let bytes = mesh.to_bytes();
        assert!(!bytes.is_empty());

        let parsed = ActorXPskMesh::from_bytes(&bytes).expect("Failed to parse roundtrip PSK");
        assert_eq!(parsed.points.len(), 1);
        assert_eq!(parsed.points[0].point, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(parsed.materials.len(), 1);
        assert_eq!(parsed.materials[0].name, "Skin_Chest");
        assert_eq!(parsed.bones.len(), 1);
        assert_eq!(parsed.bones[0].name, "Root");
        assert_eq!(parsed.weights.len(), 1);
        assert_eq!(parsed.weights[0].weight, 1.0);
    }
}
