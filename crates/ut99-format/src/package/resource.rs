use super::Package;
use std::collections::HashMap;

/// An extracted Sound resource from a .uax or .u package
#[derive(Debug, Clone, PartialEq)]
pub struct SoundResource {
    pub name: String,
    pub package: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub duration_seconds: f32,
    pub data_size: usize,
    pub wav_bytes: Vec<u8>,
}

/// An extracted Music track from a .umx package
#[derive(Debug, Clone, PartialEq)]
pub struct MusicResource {
    pub name: String,
    pub package: String,
    pub format: String, // "IT", "XM", "S3M", "MOD", "OGG", "Unknown"
    pub title: String,
    pub data_size: usize,
}

/// An UnrealScript Class definition extracted from a .u package
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassResource {
    pub name: String,
    pub super_name: String,
    pub package: String,
}

/// A node in the Actor Class hierarchy tree
#[derive(Debug, Clone, PartialEq)]
pub struct ClassTreeNode {
    pub name: String,
    pub package: String,
    pub children: Vec<ClassTreeNode>,
}

impl ClassTreeNode {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            package: package.into(),
            children: Vec::new(),
        }
    }
}

/// Parses a Sound export payload by finding and validating the RIFF WAVE header.
pub fn parse_sound_export(name: &str, package: &str, raw: &[u8]) -> Option<SoundResource> {
    if raw.len() < 44 {
        return None;
    }

    // Find "RIFF" signature
    let riff_pos = raw.windows(4).position(|w| w == b"RIFF")?;
    let wav_slice = &raw[riff_pos..];

    if wav_slice.len() < 44 || &wav_slice[8..12] != b"WAVE" {
        return None;
    }

    // Find "fmt " chunk
    let fmt_pos = wav_slice.windows(4).position(|w| w == b"fmt ")?;
    if fmt_pos + 24 > wav_slice.len() {
        return None;
    }

    let channels = u16::from_le_bytes([wav_slice[fmt_pos + 10], wav_slice[fmt_pos + 11]]);
    let sample_rate = u32::from_le_bytes([
        wav_slice[fmt_pos + 12],
        wav_slice[fmt_pos + 13],
        wav_slice[fmt_pos + 14],
        wav_slice[fmt_pos + 15],
    ]);
    let bits_per_sample = u16::from_le_bytes([wav_slice[fmt_pos + 22], wav_slice[fmt_pos + 23]]);

    // Find "data" chunk
    let data_len = if let Some(data_pos) = wav_slice.windows(4).position(|w| w == b"data") {
        if data_pos + 8 <= wav_slice.len() {
            u32::from_le_bytes([
                wav_slice[data_pos + 4],
                wav_slice[data_pos + 5],
                wav_slice[data_pos + 6],
                wav_slice[data_pos + 7],
            ]) as usize
        } else {
            wav_slice.len()
        }
    } else {
        wav_slice.len()
    };

    let bytes_per_sample = ((bits_per_sample as usize / 8) * channels as usize).max(1);
    let total_samples = data_len / bytes_per_sample;
    let duration_seconds = if sample_rate > 0 {
        total_samples as f32 / sample_rate as f32
    } else {
        0.0
    };

    Some(SoundResource {
        name: name.to_string(),
        package: package.to_string(),
        channels,
        sample_rate,
        bits_per_sample,
        duration_seconds,
        data_size: wav_slice.len(),
        wav_bytes: wav_slice.to_vec(),
    })
}

/// Parses a Music export payload by detecting module format signature.
pub fn parse_music_export(name: &str, package: &str, raw: &[u8]) -> Option<MusicResource> {
    if raw.len() < 32 {
        return None;
    }

    let mut format = "Unknown";
    let mut title = String::new();

    // Check IT (Impulse Tracker)
    if let Some(pos) = raw.windows(4).position(|w| w == b"IMPM") {
        format = "IT (Impulse Tracker)";
        if pos + 4 < raw.len() {
            // IT song title is in the 26 bytes immediately following "IMPM" (offset +4..+30)
            let end = (pos + 30).min(raw.len());
            let title_bytes = &raw[pos + 4..end];
            title = String::from_utf8_lossy(title_bytes)
                .trim_matches(|c: char| c == '\0' || c.is_whitespace())
                .to_string();
        }
    }
    // Check XM (FastTracker 2)
    else if let Some(pos) = raw.windows(16).position(|w| w == b"Extended Module:") {
        format = "XM (FastTracker II)";
        if pos + 37 <= raw.len() {
            let title_bytes = &raw[pos + 17..pos + 37];
            title = String::from_utf8_lossy(title_bytes)
                .trim_matches(|c: char| c == '\0' || c.is_whitespace())
                .to_string();
        }
    }
    // Check S3M (ScreamTracker 3)
    else if raw.windows(4).any(|w| w == b"SCRM") {
        format = "S3M (ScreamTracker 3)";
    }
    // Check Ogg Vorbis
    else if raw.windows(4).any(|w| w == b"OggS") {
        format = "OGG Vorbis";
    }
    // Check ProTracker MOD (with or without UE1 UMusic export header prefix)
    else if let Some(sig_offset) = raw.windows(4).enumerate().find_map(|(i, w)| {
        if i >= 1080 && (w == b"M.K." || w == b"4CHN" || w == b"FLT4" || w == b"8CHN") {
            Some(i)
        } else {
            None
        }
    }) {
        format = "MOD (ProTracker)";
        let mod_start = sig_offset - 1080;
        let title_bytes = &raw[mod_start..mod_start + 20];
        title = String::from_utf8_lossy(title_bytes)
            .trim_matches(|c: char| c == '\0' || c.is_whitespace())
            .to_string();
    }

    if title.is_empty() {
        title = name.to_string();
    }

    Some(MusicResource {
        name: name.to_string(),
        package: package.to_string(),
        format: format.to_string(),
        title,
        data_size: raw.len(),
    })
}

/// Scans all exported Classes in a .u package.
pub fn scan_u_package_classes(package: &Package, package_name: &str) -> Vec<ClassResource> {
    let mut classes = Vec::new();

    for export in &package.exports.entries {
        let cls_name = package.get_export_class_name(export);
        if cls_name == "Class" {
            let obj_name = package.get_export_object_name(export);
            let super_name = package.get_export_super_name(export);

            classes.push(ClassResource {
                name: obj_name,
                super_name,
                package: package_name.to_string(),
            });
        }
    }

    classes
}

/// Builds an inheritance tree starting from the "Object" root class.
pub fn build_class_tree(classes: &[ClassResource]) -> ClassTreeNode {
    let mut children_map: HashMap<String, Vec<&ClassResource>> = HashMap::new();

    for cls in classes {
        children_map.entry(cls.super_name.clone()).or_default().push(cls);
    }

    fn build_node(name: &str, pkg: &str, map: &HashMap<String, Vec<&ClassResource>>) -> ClassTreeNode {
        let mut node = ClassTreeNode::new(name, pkg);
        if let Some(children) = map.get(name) {
            for child in children {
                // Avoid infinite recursive self-reference
                if child.name != name {
                    node.children.push(build_node(&child.name, &child.package, map));
                }
            }
        }
        node.children.sort_by(|a, b| a.name.cmp(&b.name));
        node
    }

    build_node("Object", "Core", &children_map)
}

/// Extracts the `ScriptText` (`TextBuffer`) UnrealScript source code for `class_name` from a `.u` package if present.
pub fn extract_class_script_text(package: &Package, class_name: &str) -> Option<String> {
    // 1. Find the 1-based export reference of the target Class
    let mut class_exp_ref: Option<i32> = None;
    for (idx, export) in package.exports.entries.iter().enumerate() {
        let cls = package.get_export_class_name(export);
        if cls == "Class" && package.get_export_object_name(export).eq_ignore_ascii_case(class_name) {
            class_exp_ref = Some((idx + 1) as i32);
            break;
        }
    }
    let target_parent = class_exp_ref?;

    // 2. Find the child TextBuffer export (usually named "ScriptText" or with class "TextBuffer")
    for (idx, export) in package.exports.entries.iter().enumerate() {
        if export.package_index == target_parent {
            let cls = package.get_export_class_name(export);
            let obj_name = package.get_export_object_name(export);
            if cls == "TextBuffer" || obj_name.eq_ignore_ascii_case("ScriptText") {
                let raw = package.get_export_bytes(idx)?;
                if raw.is_empty() {
                    continue;
                }
                // Extract the longest contiguous printable ASCII/Latin-1/tab/newline slice (the .uc script text)
                let mut best_start = 0usize;
                let mut best_len = 0usize;
                let mut cur_start = 0usize;
                let mut cur_len = 0usize;
                for (b_i, &b) in raw.iter().enumerate() {
                    let is_text = b == b'\r'
                        || b == b'\n'
                        || b == b'\t'
                        || (0x20..=0x7E).contains(&b)
                        || b >= 0xA0;
                    if is_text {
                        if cur_len == 0 {
                            cur_start = b_i;
                        }
                        cur_len += 1;
                        if cur_len > best_len {
                            best_len = cur_len;
                            best_start = cur_start;
                        }
                    } else {
                        cur_len = 0;
                    }
                }
                if best_len >= 16 {
                    let slice = &raw[best_start..best_start + best_len];
                    let text = match std::str::from_utf8(slice) {
                        Ok(s) => s.replace("\r\n", "\n"),
                        Err(_) => slice
                            .iter()
                            .map(|&b| b as char)
                            .collect::<String>()
                            .replace("\r\n", "\n"),
                    };
                    return Some(text);
                }
            }
        }
    }
    None
}

/// Scans all `Mesh`, `LodMesh`, and `SkeletalMesh` exports in a `.u` package.
pub fn scan_u_package_meshes(package: &Package, package_name: &str) -> Vec<(String, String, usize)> {
    let mut meshes = Vec::new();
    for export in &package.exports.entries {
        let cls_name = package.get_export_class_name(export);
        if cls_name == "LodMesh" || cls_name == "Mesh" || cls_name == "SkeletalMesh" {
            let obj_name = package.get_export_object_name(export);
            meshes.push((
                format!("{}.{}", package_name, obj_name),
                cls_name,
                export.serial_size.max(0) as usize,
            ));
        }
    }
    meshes
}

/// Decoded 3D mesh data extracted from a `.u` `Mesh` / `LodMesh` export (`BG-17`).
#[derive(Debug, Clone)]
pub struct PackageMeshData {
    pub name: String,
    pub class_name: String,
    pub serial_size: usize,
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[u16; 3]>,
}

/// Packs a 3D vertex `[x, y, z]` into an Unreal Engine 1 `FMeshVert` 32-bit integer
/// (11-bit signed X, 11-bit signed Y, 10-bit signed Z).
pub fn pack_fmesh_vert(v: [f32; 3]) -> u32 {
    let x = (v[0].round() as i32).clamp(-1024, 1023) & 0x7FF;
    let y = (v[1].round() as i32).clamp(-1024, 1023) & 0x7FF;
    let z = (v[2].round() as i32).clamp(-512, 511) & 0x3FF;
    ((x as u32) & 0x7FF) | (((y as u32) & 0x7FF) << 11) | (((z as u32) & 0x3FF) << 22)
}

/// Unpacks an Unreal Engine 1 `FMeshVert` 32-bit integer (11-11-10 bit two's complement)
/// into a 3D vertex `[x, y, z]`.
pub fn unpack_fmesh_vert(packed: u32) -> [f32; 3] {
    let x = (((packed & 0x7FF) as i32) << 21) >> 21;
    let y = ((((packed >> 11) & 0x7FF) as i32) << 21) >> 21;
    let z = ((((packed >> 22) & 0x3FF) as i32) << 22) >> 22;
    [x as f32, y as f32, z as f32]
}

/// Encodes 3D vertices and triangle indices into an Unreal Engine 1 `UMesh` binary export payload
/// (`None` property terminator + 41-byte `UPrimitive` bounds + `TLazyArray<FMeshVert>` + `TLazyArray<FMeshTri>`).
pub fn encode_umesh_geometry(vertices: &[[f32; 3]], triangles: &[[u16; 3]]) -> Vec<u8> {
    use crate::package::write_compact_index;

    let mut buf = Vec::new();
    // 1. Tagged properties terminator ("None" = index 0)
    let _ = write_compact_index(&mut buf, 0);
    // 2. UPrimitive FBox (25 bytes) + FSphere (16 bytes) = 41 bytes
    buf.extend_from_slice(&[0u8; 41]);

    // 3. TLazyArray<FMeshVert>
    let mut verts_payload = Vec::new();
    let _ = write_compact_index(&mut verts_payload, vertices.len() as i32);
    for &v in vertices {
        verts_payload.extend_from_slice(&pack_fmesh_vert(v).to_le_bytes());
    }
    let skip_verts = (buf.len() + 4 + verts_payload.len()) as i32;
    buf.extend_from_slice(&skip_verts.to_le_bytes());
    buf.extend_from_slice(&verts_payload);

    // 4. TLazyArray<FMeshTri> (20 bytes per triangle)
    let mut tris_payload = Vec::new();
    let _ = write_compact_index(&mut tris_payload, triangles.len() as i32);
    for &tri in triangles {
        tris_payload.extend_from_slice(&tri[0].to_le_bytes());
        tris_payload.extend_from_slice(&tri[1].to_le_bytes());
        tris_payload.extend_from_slice(&tri[2].to_le_bytes());
        // 3x FMeshUV (u8, u8) = 6 bytes
        tris_payload.extend_from_slice(&[0, 0, 255, 0, 0, 255]);
        // PolyFlags: u32 (4 bytes) + TextureIndex: i32 (4 bytes)
        tris_payload.extend_from_slice(&0u32.to_le_bytes());
        tris_payload.extend_from_slice(&0i32.to_le_bytes());
    }
    let skip_tris = (buf.len() + 4 + tris_payload.len()) as i32;
    buf.extend_from_slice(&skip_tris.to_le_bytes());
    buf.extend_from_slice(&tris_payload);

    buf
}

/// Decodes `FMeshVert` (11-11-10 bit packed `u32`) and `FMeshTri` (20-byte struct with `[u16; 3]` indices)
/// or `ULodMesh` (`Faces` + `Wedges`) from an Unreal Engine 1 `UMesh` / `ULodMesh` binary export payload.
pub fn decode_umesh_geometry(raw: &[u8]) -> Option<(Vec<[f32; 3]>, Vec<[u16; 3]>)> {
    use crate::package::read_compact_index;
    use std::io::Cursor;

    if raw.len() < 28 {
        return None;
    }

    let try_parse_at = |start: usize| -> Option<(Vec<[f32; 3]>, Vec<[u16; 3]>)> {
        if start + 6 >= raw.len() {
            return None;
        }
        // Skip 4-byte TLazyArray skip_pos for Verts
        let mut cur = Cursor::new(&raw[start + 4..]);
        let vert_count = read_compact_index(&mut cur).ok()?;
        if !(3..=250_000).contains(&vert_count) {
            return None;
        }
        let v_count = vert_count as usize;
        let verts_data_start = start + 4 + cur.position() as usize;
        let verts_data_end = verts_data_start.checked_add(v_count.checked_mul(4)?)?;
        if verts_data_end + 5 > raw.len() {
            return None;
        }

        // Immediately after Verts is TLazyArray<FMeshTri>: 4-byte skip_pos + CompactIndex(tri_count) + tri_count * 20
        let mut tri_cur = Cursor::new(&raw[verts_data_end + 4..]);
        let tri_count = read_compact_index(&mut tri_cur).ok()?;
        if tri_count > 0 && tri_count <= 65535 {
            let t_count = tri_count as usize;
            let tris_data_start = verts_data_end + 4 + tri_cur.position() as usize;
            let tris_data_end = tris_data_start.checked_add(t_count.checked_mul(20)?)?;
            if tris_data_end <= raw.len() {
                let mut triangles = Vec::with_capacity(t_count);
                let mut max_v_idx = 0usize;
                let mut valid = true;
                for t in 0..t_count {
                    let base = tris_data_start + t * 20;
                    let i0 = u16::from_le_bytes([raw[base], raw[base + 1]]);
                    let i1 = u16::from_le_bytes([raw[base + 2], raw[base + 3]]);
                    let i2 = u16::from_le_bytes([raw[base + 4], raw[base + 5]]);
                    if (i0 as usize) >= v_count || (i1 as usize) >= v_count || (i2 as usize) >= v_count {
                        valid = false;
                        break;
                    }
                    if i0 == i1 && i1 == i2 {
                        valid = false;
                        break;
                    }
                    max_v_idx = max_v_idx.max(i0 as usize).max(i1 as usize).max(i2 as usize);
                    triangles.push([i0, i1, i2]);
                }
                if valid {
                    let keep_verts = (max_v_idx + 1).max(v_count.min(max_v_idx + 1));
                    let mut vertices = Vec::with_capacity(keep_verts);
                    for v in 0..keep_verts {
                        let base = verts_data_start + v * 4;
                        let packed = u32::from_le_bytes([
                            raw[base],
                            raw[base + 1],
                            raw[base + 2],
                            raw[base + 3],
                        ]);
                        vertices.push(unpack_fmesh_vert(packed));
                    }
                    return Some((vertices, triangles));
                }
            }
        } else if tri_count == 0 {
            // ULodMesh case: UMesh::Tris is empty (tri_count == 0), and geometry is stored in
            // FaceLevel (TArray<u16>), Faces (TArray<FMeshFace>, 8B each),
            // CollapseWedgeThus (TArray<u16>), and Wedges (TArray<FMeshWedge>, 4B each).
            let lod_search_start = verts_data_end + 4 + tri_cur.position() as usize;
            for pos in lod_search_start..raw.len().saturating_sub(16) {
                let mut fc_cur = Cursor::new(&raw[pos..]);
                let Ok(f_count_i32) = read_compact_index(&mut fc_cur) else {
                    continue;
                };
                if !(1..=32768).contains(&f_count_i32) {
                    continue;
                }
                let f_count = f_count_i32 as usize;
                let face_level_end = pos + fc_cur.position() as usize + f_count * 2;
                if face_level_end + 8 > raw.len() {
                    continue;
                }
                let mut faces_cur = Cursor::new(&raw[face_level_end..]);
                if read_compact_index(&mut faces_cur).ok() != Some(f_count_i32) {
                    continue;
                }
                let faces_data_start = face_level_end + faces_cur.position() as usize;
                let faces_data_end = faces_data_start + f_count * 8;
                if faces_data_end + 6 > raw.len() {
                    continue;
                }
                let mut cw_cur = Cursor::new(&raw[faces_data_end..]);
                let Ok(w_count_i32) = read_compact_index(&mut cw_cur) else {
                    continue;
                };
                if !(3..=65535).contains(&w_count_i32) {
                    continue;
                }
                let w_count = w_count_i32 as usize;
                let cw_end = faces_data_end + cw_cur.position() as usize + w_count * 2;
                if cw_end + 4 > raw.len() {
                    continue;
                }
                let mut w_cur = Cursor::new(&raw[cw_end..]);
                if read_compact_index(&mut w_cur).ok() != Some(w_count_i32) {
                    continue;
                }
                let wedges_data_start = cw_end + w_cur.position() as usize;
                let wedges_data_end = wedges_data_start + w_count * 4;
                if wedges_data_end > raw.len() {
                    continue;
                }

                // Read Wedges -> vertex index mapping
                let mut wedge_vert = Vec::with_capacity(w_count);
                let mut max_v_idx = 0usize;
                let mut valid = true;
                for w in 0..w_count {
                    let wb = wedges_data_start + w * 4;
                    let v_idx = u16::from_le_bytes([raw[wb], raw[wb + 1]]);
                    if (v_idx as usize) >= v_count {
                        valid = false;
                        break;
                    }
                    max_v_idx = max_v_idx.max(v_idx as usize);
                    wedge_vert.push(v_idx);
                }
                if !valid {
                    continue;
                }

                // Read Faces -> triangle vertex indices
                let mut triangles = Vec::with_capacity(f_count);
                for f in 0..f_count {
                    let fb = faces_data_start + f * 8;
                    let w0 = u16::from_le_bytes([raw[fb], raw[fb + 1]]) as usize;
                    let w1 = u16::from_le_bytes([raw[fb + 2], raw[fb + 3]]) as usize;
                    let w2 = u16::from_le_bytes([raw[fb + 4], raw[fb + 5]]) as usize;
                    if w0 >= w_count || w1 >= w_count || w2 >= w_count {
                        valid = false;
                        break;
                    }
                    triangles.push([wedge_vert[w0], wedge_vert[w1], wedge_vert[w2]]);
                }
                if !valid || triangles.is_empty() {
                    continue;
                }

                let keep_verts = (max_v_idx + 1).min(v_count);
                let mut vertices = Vec::with_capacity(keep_verts);
                for v in 0..keep_verts {
                    let base = verts_data_start + v * 4;
                    let packed = u32::from_le_bytes([
                        raw[base],
                        raw[base + 1],
                        raw[base + 2],
                        raw[base + 3],
                    ]);
                    vertices.push(unpack_fmesh_vert(packed));
                }
                return Some((vertices, triangles));
            }
        }
        None
    };

    // Fast path for canonical offset 42 (1-byte None + 41-byte UPrimitive FBox/FSphere)
    if let Some(res) = try_parse_at(42) {
        return Some(res);
    }

    // General scan across the export header
    let max_scan = raw.len().saturating_sub(28).min(4096);
    for offset in 0..=max_scan {
        if let Some(res) = try_parse_at(offset) {
            return Some(res);
        }
    }
    None
}

/// Scans all `Mesh`, `LodMesh`, and `SkeletalMesh` exports in a `.u` package and decodes
/// their 3D vertex/triangle geometry when present (`BG-17`).
pub fn extract_u_package_meshes(package: &Package, package_name: &str) -> Vec<PackageMeshData> {
    let mut meshes = Vec::new();
    for (idx, export) in package.exports.entries.iter().enumerate() {
        let cls_name = package.get_export_class_name(export);
        if cls_name == "LodMesh" || cls_name == "Mesh" || cls_name == "SkeletalMesh" {
            let obj_name = package.get_export_object_name(export);
            let (vertices, triangles) = package
                .get_export_bytes(idx)
                .and_then(decode_umesh_geometry)
                .unwrap_or_default();
            meshes.push(PackageMeshData {
                name: format!("{}.{}", package_name, obj_name),
                class_name: cls_name,
                serial_size: export.serial_size.max(0) as usize,
                vertices,
                triangles,
            });
        }
    }
    meshes
}

/// Encodes a `SoundResource` (`USound`) into a UE1 v68 export payload
/// (`None` property terminator + `FName("WAV")` + `TLazyArray<u8>`).
pub fn encode_sound_export(
    wav_bytes: &[u8],
    pkg: &mut Package,
    base_serial_offset: usize,
) -> std::io::Result<Vec<u8>> {
    use crate::package::write_compact_index;
    use byteorder::{LittleEndian, WriteBytesExt};

    pkg.names.add("None", 0);
    let n_wav = pkg.names.add("WAV", 0) as i32;

    let mut buf = Vec::with_capacity(wav_bytes.len() + 16);
    // 1. Property terminator ("None" = 0)
    write_compact_index(&mut buf, 0)?;
    // 2. FileType FName ("WAV")
    write_compact_index(&mut buf, n_wav)?;
    // 3. TLazyArray<u8>: skip_offset (i32) + CompactIndex(len) + wav_bytes
    let mut len_ci = Vec::new();
    write_compact_index(&mut len_ci, wav_bytes.len() as i32)?;
    let skip_offset = (base_serial_offset + buf.len() + 4 + len_ci.len() + wav_bytes.len()) as i32;
    buf.write_i32::<LittleEndian>(skip_offset)?;
    buf.extend_from_slice(&len_ci);
    buf.extend_from_slice(wav_bytes);
    Ok(buf)
}

/// Writes a slice of `SoundResource` into an existing `Package` (either standalone `.uax` with `default_outer_index = 0`
/// or embedded inside `MyLevel` with `default_outer_index > 0`).
pub fn write_sounds_into_package(
    sounds: &[SoundResource],
    pkg: &mut Package,
    default_outer_index: i32,
) -> std::io::Result<Vec<i32>> {
    if sounds.is_empty() {
        return Ok(Vec::new());
    }
    pkg.names.add("None", 0);
    let n_core = pkg.names.add("Core", 0) as i32;
    let n_engine = pkg.names.add("Engine", 0) as i32;
    let n_class = pkg.names.add("Class", 0) as i32;
    let n_package = pkg.names.add("Package", 0) as i32;
    let n_sound_cls = pkg.names.add("Sound", 0) as i32;

    let _imp_core = pkg.add_import(n_core, n_package, 0, n_core);
    let imp_engine = pkg.add_import(n_core, n_package, 0, n_engine);
    let imp_sound = pkg.add_import(n_core, n_class, imp_engine, n_sound_cls);

    let mut export_refs = Vec::with_capacity(sounds.len());
    for (i, snd) in sounds.iter().enumerate() {
        let snd_name = if snd.name.is_empty() {
            format!("Sound_{i}")
        } else {
            snd.name
                .rsplit('.')
                .next()
                .unwrap_or(&snd.name)
                .to_string()
        };
        let n_snd = pkg.names.add(snd_name, 0) as i32;
        let base_offset = pkg.raw_data.len();
        let payload = encode_sound_export(&snd.wav_bytes, pkg, base_offset)?;
        let exp_ref = pkg.add_export(
            n_snd,
            imp_sound,
            0,
            default_outer_index,
            0x000F_0004,
            payload,
        );
        export_refs.push(exp_ref);
    }
    Ok(export_refs)
}

/// Builds a standalone UE1 v68 `.uax` `Package` from a list of `SoundResource` items.
pub fn build_uax_package(sounds: &[SoundResource]) -> std::io::Result<Package> {
    let mut pkg = Package::default();
    pkg.summary.file_version = 68;
    pkg.summary.pkg_flags = 1;
    pkg.summary.guid = [0x55, 0x41, 0x58, 0x39, 0x39, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];
    write_sounds_into_package(sounds, &mut pkg, 0)?;
    Ok(pkg)
}

/// Encodes a tracker/audio module (`UMusic`) into a UE1 v68 `.umx` export payload.
pub fn encode_music_export(
    format_tag: &str,
    music_bytes: &[u8],
    pkg: &mut Package,
    base_serial_offset: usize,
) -> std::io::Result<Vec<u8>> {
    use crate::package::write_compact_index;
    use byteorder::{LittleEndian, WriteBytesExt};

    pkg.names.add("None", 0);
    let short_fmt = if format_tag.starts_with("IT") {
        "it"
    } else if format_tag.starts_with("XM") {
        "xm"
    } else if format_tag.starts_with("S3M") {
        "s3m"
    } else if format_tag.starts_with("MOD") {
        "mod"
    } else if format_tag.starts_with("OGG") {
        "ogg"
    } else {
        "s3m"
    };
    let n_fmt = pkg.names.add(short_fmt, 0) as i32;

    let mut buf = Vec::with_capacity(music_bytes.len() + 16);
    write_compact_index(&mut buf, 0)?;
    write_compact_index(&mut buf, n_fmt)?;
    let mut len_ci = Vec::new();
    write_compact_index(&mut len_ci, music_bytes.len() as i32)?;
    let skip_offset = (base_serial_offset + buf.len() + 4 + len_ci.len() + music_bytes.len()) as i32;
    buf.write_i32::<LittleEndian>(skip_offset)?;
    buf.extend_from_slice(&len_ci);
    buf.extend_from_slice(music_bytes);
    Ok(buf)
}

/// Builds a standalone UE1 v68 `.umx` `Package` containing a `Music` export.
pub fn build_umx_package(
    track_name: &str,
    format_tag: &str,
    music_bytes: &[u8],
) -> std::io::Result<Package> {
    let mut pkg = Package::default();
    pkg.summary.file_version = 68;
    pkg.summary.pkg_flags = 1;
    pkg.summary.guid = [0x55, 0x4D, 0x58, 0x39, 0x39, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];
    pkg.names.add("None", 0);
    let n_core = pkg.names.add("Core", 0) as i32;
    let n_engine = pkg.names.add("Engine", 0) as i32;
    let n_class = pkg.names.add("Class", 0) as i32;
    let n_package = pkg.names.add("Package", 0) as i32;
    let n_music_cls = pkg.names.add("Music", 0) as i32;

    let _imp_core = pkg.add_import(n_core, n_package, 0, n_core);
    let imp_engine = pkg.add_import(n_core, n_package, 0, n_engine);
    let imp_music = pkg.add_import(n_core, n_class, imp_engine, n_music_cls);

    let clean_name = if track_name.is_empty() {
        "Track0"
    } else {
        track_name.rsplit('.').next().unwrap_or(track_name)
    };
    let n_track = pkg.names.add(clean_name, 0) as i32;
    let base_offset = pkg.raw_data.len();
    let payload = encode_music_export(format_tag, music_bytes, &mut pkg, base_offset)?;
    pkg.add_export(n_track, imp_music, 0, 0, 0x000F_0004, payload);
    Ok(pkg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::write_compact_index;

    #[test]
    fn test_parse_music_export_it_and_mod_titles() {
        // 1. Impulse Tracker (.it) with UE1 prefix before "IMPM"
        let mut it_raw = vec![0u8, 1u8, 10, 0, 0, 0, 40];
        it_raw.extend_from_slice(b"IMPM");
        let mut title_field = [0u8; 26];
        title_field[..13].copy_from_slice(b"ForegoneDestn");
        it_raw.extend_from_slice(&title_field);
        it_raw.resize(64, 0);

        let res_it = parse_music_export("Foregone", "Foregone", &it_raw).expect("IT parse failed");
        assert_eq!(res_it.format, "IT (Impulse Tracker)");
        assert_eq!(res_it.title, "ForegoneDestn");

        // 2. ProTracker (.mod) with a 12-byte UE1 UMusic export header prefix
        let mut mod_raw = vec![0u8; 12]; // UE1 UMusic prefix
        let mut mod_stream = vec![0u8; 1100];
        mod_stream[..11].copy_from_slice(b"SpaceDebris");
        mod_stream[1080..1084].copy_from_slice(b"M.K.");
        mod_raw.extend_from_slice(&mod_stream);

        let res_mod = parse_music_export("Space", "Space", &mod_raw).expect("MOD parse failed");
        assert_eq!(res_mod.format, "MOD (ProTracker)");
        assert_eq!(res_mod.title, "SpaceDebris");
    }

    #[test]
    fn test_decode_ulodmesh_geometry() {
        // Construct a synthetic ULodMesh export:
        // - 1-byte None + 41-byte UPrimitive header
        // - TLazyArray<FMeshVert> with 4 vertices
        // - TLazyArray<FMeshTri> with 0 triangles (as in ULodMesh)
        // - FaceLevel (2 faces) + Faces (2 faces) + CollapseWedgeThus (4 wedges) + Wedges (4 wedges)
        let mut raw = Vec::new();
        raw.push(0u8);
        raw.extend_from_slice(&[0u8; 41]);

        let verts = [
            [-16.0, -16.0, 0.0],
            [16.0, -16.0, 0.0],
            [16.0, 16.0, 0.0],
            [0.0, 0.0, 32.0],
        ];
        let mut verts_payload = Vec::new();
        let _ = write_compact_index(&mut verts_payload, verts.len() as i32);
        for v in verts {
            verts_payload.extend_from_slice(&pack_fmesh_vert(v).to_le_bytes());
        }
        let skip_verts = (raw.len() + 4 + verts_payload.len()) as i32;
        raw.extend_from_slice(&skip_verts.to_le_bytes());
        raw.extend_from_slice(&verts_payload);

        // Empty UMesh::Tris (tri_count = 0)
        let skip_tris = (raw.len() + 5) as i32;
        raw.extend_from_slice(&skip_tris.to_le_bytes());
        raw.push(0u8); // CompactIndex(0)

        // Intermediate UMesh fields padding
        raw.extend_from_slice(&[0u8; 24]);

        // ULodMesh: FaceLevel (2 faces)
        let _ = write_compact_index(&mut raw, 2);
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());

        // ULodMesh: Faces (2 faces, 8 bytes each: [w0, w1, w2, mat])
        let _ = write_compact_index(&mut raw, 2);
        for &(w0, w1, w2) in &[(0u16, 1u16, 2u16), (0u16, 2u16, 3u16)] {
            raw.extend_from_slice(&w0.to_le_bytes());
            raw.extend_from_slice(&w1.to_le_bytes());
            raw.extend_from_slice(&w2.to_le_bytes());
            raw.extend_from_slice(&0u16.to_le_bytes());
        }

        // ULodMesh: CollapseWedgeThus (4 wedges)
        let _ = write_compact_index(&mut raw, 4);
        for w in 0..4u16 {
            raw.extend_from_slice(&w.to_le_bytes());
        }

        // ULodMesh: Wedges (4 wedges, 4 bytes each: [iVertex: u16, u: u8, v: u8])
        let _ = write_compact_index(&mut raw, 4);
        for v_idx in 0..4u16 {
            raw.extend_from_slice(&v_idx.to_le_bytes());
            raw.push(0);
            raw.push(0);
        }

        let (dec_verts, dec_tris) =
            decode_umesh_geometry(&raw).expect("ULodMesh decode failed");
        assert_eq!(dec_verts.len(), 4);
        assert_eq!(dec_tris.len(), 2);
        assert_eq!(dec_tris[0], [0, 1, 2]);
        assert_eq!(dec_tris[1], [0, 2, 3]);
    }
}



