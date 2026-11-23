use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use glam::Vec3;
use std::collections::HashMap;
use std::io::{self, Cursor, Read, Write};

use crate::package::compact_index::{read_compact_index, write_compact_index};
use crate::package::Package;

#[derive(Debug, Clone, PartialEq)]
pub enum TaggedValue {
    Byte(u8),
    Int(i32),
    Bool(bool),
    Float(f32),
    Object(i32),
    Name(String),
    Str(String),
    Vector(Vec3),
    Rotator { pitch: i32, yaw: i32, roll: i32 },
    Color { r: u8, g: u8, b: u8, a: u8 },
    Scale {
        scale: Vec3,
        sheer_rate: f32,
        sheer_axis: u8,
    },
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaggedProperty {
    pub name: String,
    pub array_index: i32,
    pub value: TaggedValue,
}

pub type PropertyMap = HashMap<String, Vec<TaggedProperty>>;

/// Reads all tagged properties from the reader until a `None` (index 0) tag is encountered.
pub fn read_tagged_properties<R: Read>(
    reader: &mut R,
    package: &Package,
) -> io::Result<HashMap<String, TaggedProperty>> {
    let mut props = HashMap::new();

    loop {
        let name_idx = match read_compact_index(reader) {
            Ok(idx) => idx,
            Err(_) => break,
        };

        if name_idx < 0 {
            break;
        }

        let prop_name = match package.get_name(name_idx as usize) {
            Some(n) => n.to_string(),
            None => break,
        };

        if prop_name.eq_ignore_ascii_case("None") {
            // "None" indicates end of tagged properties
            break;
        }

        let info = match reader.read_u8() {
            Ok(b) => b,
            Err(_) => break,
        };
        let prop_type = info & 0x0F;
        let size_type = (info >> 4) & 0x07;
        let has_array = (info & 0x80) != 0;

        let mut struct_name = String::new();
        if prop_type == 10 {
            let s_idx = read_compact_index(reader).unwrap_or(0);
            if let Some(s) = package.get_name(s_idx.max(0) as usize) {
                struct_name = s.to_string();
            }
        }

        let data_size: usize = match size_type {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 12,
            4 => 16,
            5 => match reader.read_u8() {
                Ok(b) => b as usize,
                Err(_) => break,
            },
            6 => match reader.read_u16::<LittleEndian>() {
                Ok(w) => w as usize,
                Err(_) => break,
            },
            7 => match reader.read_u32::<LittleEndian>() {
                Ok(d) => d as usize,
                Err(_) => break,
            },
            _ => 1,
        };

        let mut array_index = 0;
        if has_array && prop_type != 3 {
            array_index = read_compact_index(reader).unwrap_or(0);
        }

        let value = if prop_type == 3 {
            // UE1 BoolProperty encodes its boolean value in bit 7 (0x80) of the info byte.
            // Also accept bit 5 (0x20) for backwards compatibility with legacy synthetic tests.
            TaggedValue::Bool((info & 0x80) != 0 || (info & 0x20) != 0)
        } else {
            let mut payload = vec![0u8; data_size];
            if reader.read_exact(&mut payload).is_err() {
                break;
            }

            decode_payload(prop_type, &struct_name, &payload, package)
        };

        let map_key = if array_index > 0 {
            format!("{}({})", prop_name, array_index)
        } else {
            prop_name.clone()
        };

        props.insert(
            map_key,
            TaggedProperty {
                name: prop_name,
                array_index,
                value,
            },
        );
    }

    Ok(props)
}

/// Reads all tagged properties along with raw property metadata `(prop_type, struct_name)` for lossless roundtripping.
pub fn read_tagged_properties_with_meta<R: Read>(
    reader: &mut R,
    package: &Package,
) -> io::Result<(
    HashMap<String, TaggedProperty>,
    HashMap<String, (u8, Option<String>)>,
)> {
    let mut props = HashMap::new();
    let mut raw_meta = HashMap::new();

    loop {
        let name_idx = match read_compact_index(reader) {
            Ok(idx) => idx,
            Err(_) => break,
        };

        if name_idx < 0 {
            break;
        }

        let prop_name = match package.get_name(name_idx as usize) {
            Some(n) => n.to_string(),
            None => break,
        };

        if prop_name.eq_ignore_ascii_case("None") {
            break;
        }

        let info = match reader.read_u8() {
            Ok(b) => b,
            Err(_) => break,
        };
        let prop_type = info & 0x0F;
        let size_type = (info >> 4) & 0x07;
        let has_array = (info & 0x80) != 0;

        let mut struct_name = String::new();
        if prop_type == 10 {
            let s_idx = read_compact_index(reader).unwrap_or(0);
            if let Some(s) = package.get_name(s_idx.max(0) as usize) {
                struct_name = s.to_string();
            }
        }

        let data_size: usize = match size_type {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 12,
            4 => 16,
            5 => match reader.read_u8() {
                Ok(b) => b as usize,
                Err(_) => break,
            },
            6 => match reader.read_u16::<LittleEndian>() {
                Ok(w) => w as usize,
                Err(_) => break,
            },
            7 => match reader.read_u32::<LittleEndian>() {
                Ok(d) => d as usize,
                Err(_) => break,
            },
            _ => 1,
        };

        let mut array_index = 0;
        if has_array && prop_type != 3 {
            array_index = read_compact_index(reader).unwrap_or(0);
        }

        let value = if prop_type == 3 {
            TaggedValue::Bool((info & 0x80) != 0 || (info & 0x20) != 0)
        } else {
            let mut payload = vec![0u8; data_size];
            if reader.read_exact(&mut payload).is_err() {
                break;
            }

            decode_payload(prop_type, &struct_name, &payload, package)
        };

        let map_key = if array_index > 0 {
            format!("{}({})", prop_name, array_index)
        } else {
            prop_name.clone()
        };

        if matches!(value, TaggedValue::Raw(_)) {
            raw_meta.insert(
                map_key.clone(),
                (
                    prop_type,
                    if struct_name.is_empty() {
                        None
                    } else {
                        Some(struct_name)
                    },
                ),
            );
        }

        props.insert(
            map_key,
            TaggedProperty {
                name: prop_name,
                array_index,
                value,
            },
        );
    }

    Ok((props, raw_meta))
}

fn decode_payload(
    prop_type: u8,
    struct_name: &str,
    payload: &[u8],
    package: &Package,
) -> TaggedValue {
    let mut cur = Cursor::new(payload);

    match prop_type {
        1 => {
            // ByteProperty
            TaggedValue::Byte(payload.first().copied().unwrap_or(0))
        }
        2 => {
            // IntProperty (can be 1, 2, or 4 bytes)
            let val = match payload.len() {
                1 => payload[0] as i8 as i32,
                2 => cur.read_i16::<LittleEndian>().unwrap_or(0) as i32,
                _ => cur.read_i32::<LittleEndian>().unwrap_or(0),
            };
            TaggedValue::Int(val)
        }
        4 => {
            // FloatProperty
            let val = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
            TaggedValue::Float(val)
        }
        5 => {
            // ObjectProperty (CompactIndex)
            let obj_ref = read_compact_index(&mut cur).unwrap_or(0);
            TaggedValue::Object(obj_ref)
        }
        6 => {
            // NameProperty (CompactIndex)
            let nm_idx = read_compact_index(&mut cur).unwrap_or(0);
            let name_str = package
                .get_name(nm_idx.max(0) as usize)
                .unwrap_or("")
                .to_string();
            TaggedValue::Name(name_str)
        }
        7 => {
            // StringProperty (fixed-length null-terminated ASCII)
            let s = String::from_utf8_lossy(payload).trim_end_matches('\0').to_string();
            TaggedValue::Str(s)
        }
        10 => {
            // StructProperty
            match struct_name {
                "Vector" if payload.len() >= 12 => {
                    let x = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
                    let y = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
                    let z = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
                    TaggedValue::Vector(Vec3::new(x, y, z))
                }
                "Rotator" if payload.len() >= 12 => {
                    let pitch = cur.read_i32::<LittleEndian>().unwrap_or(0);
                    let yaw = cur.read_i32::<LittleEndian>().unwrap_or(0);
                    let roll = cur.read_i32::<LittleEndian>().unwrap_or(0);
                    TaggedValue::Rotator { pitch, yaw, roll }
                }
                "Color" if payload.len() >= 4 => {
                    TaggedValue::Color {
                        r: payload[0],
                        g: payload[1],
                        b: payload[2],
                        a: payload[3],
                    }
                }
                "Scale" if payload.len() >= 17 => {
                    let x = cur.read_f32::<LittleEndian>().unwrap_or(1.0);
                    let y = cur.read_f32::<LittleEndian>().unwrap_or(1.0);
                    let z = cur.read_f32::<LittleEndian>().unwrap_or(1.0);
                    let sheer_rate = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
                    let sheer_axis = payload.get(16).copied().unwrap_or(0);
                    TaggedValue::Scale {
                        scale: Vec3::new(x, y, z),
                        sheer_rate,
                        sheer_axis,
                    }
                }
                _ => TaggedValue::Raw(payload.to_vec()),
            }
        }
        11 if payload.len() >= 12 => {
            // Native Vector
            let x = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
            let y = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
            let z = cur.read_f32::<LittleEndian>().unwrap_or(0.0);
            TaggedValue::Vector(Vec3::new(x, y, z))
        }
        12 if payload.len() >= 12 => {
            // Native Rotator
            let pitch = cur.read_i32::<LittleEndian>().unwrap_or(0);
            let yaw = cur.read_i32::<LittleEndian>().unwrap_or(0);
            let roll = cur.read_i32::<LittleEndian>().unwrap_or(0);
            TaggedValue::Rotator { pitch, yaw, roll }
        }
        13 => {
            // StrProperty (UE1 FString: CompactIndex length + chars)
            let str_len = read_compact_index(&mut cur).unwrap_or(0);
            if str_len > 0 {
                let rem = payload.len().saturating_sub(cur.position() as usize);
                let len = (str_len as usize).min(rem);
                let mut buf = vec![0u8; len];
                let _ = cur.read_exact(&mut buf);
                if buf.last() == Some(&0) {
                    buf.pop();
                }
                let s: String = buf.into_iter().map(|b| b as char).collect();
                TaggedValue::Str(s)
            } else if str_len < 0 {
                let char_count = (-str_len) as usize;
                let mut chars = Vec::with_capacity(char_count);
                for _ in 0..char_count {
                    if let Ok(c) = cur.read_u16::<LittleEndian>() {
                        if c != 0 {
                            chars.push(c);
                        }
                    } else {
                        break;
                    }
                }
                TaggedValue::Str(String::from_utf16_lossy(&chars))
            } else {
                TaggedValue::Str(String::new())
            }
        }
        _ => TaggedValue::Raw(payload.to_vec()),
    }
}

pub fn write_tagged_property<W: Write>(
    writer: &mut W,
    prop: &TaggedProperty,
    package: &mut Package,
) -> io::Result<()> {
    write_tagged_property_with_meta(writer, prop, None, package)
}

pub fn write_tagged_property_with_meta<W: Write>(
    writer: &mut W,
    prop: &TaggedProperty,
    raw_meta: Option<&(u8, Option<String>)>,
    package: &mut Package,
) -> io::Result<()> {
    let (base_name, array_idx) = if prop.array_index > 0 {
        (prop.name.as_str(), prop.array_index)
    } else if let Some(open_paren) = prop.name.find('(') {
        if prop.name.ends_with(')') {
            let idx_str = &prop.name[open_paren + 1..prop.name.len() - 1];
            if let Ok(parsed_idx) = idx_str.parse::<i32>() {
                (&prop.name[..open_paren], parsed_idx)
            } else {
                (prop.name.as_str(), 0)
            }
        } else {
            (prop.name.as_str(), 0)
        }
    } else if let Some(open_bracket) = prop.name.find('[') {
        if prop.name.ends_with(']') {
            let idx_str = &prop.name[open_bracket + 1..prop.name.len() - 1];
            if let Ok(parsed_idx) = idx_str.parse::<i32>() {
                (&prop.name[..open_bracket], parsed_idx)
            } else {
                (prop.name.as_str(), 0)
            }
        } else {
            (prop.name.as_str(), 0)
        }
    } else {
        (prop.name.as_str(), 0)
    };

    let name_idx = package.names.add(base_name, 0) as i32;
    write_compact_index(writer, name_idx)?;

    let has_array = array_idx > 0;
    let array_bit = if has_array { 0x80u8 } else { 0x00u8 };

    match &prop.value {
        TaggedValue::Byte(b) => {
            let info = 0x01 | array_bit; // prop_type = 1, size_type = 0 (1 byte)
            writer.write_u8(info)?;
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_u8(*b)?;
        }
        TaggedValue::Int(val) => {
            let info = (2 << 4) | 0x02 | array_bit; // prop_type = 2, size_type = 2 (4 bytes)
            writer.write_u8(info)?;
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_i32::<LittleEndian>(*val)?;
        }
        TaggedValue::Bool(b) => {
            // UE1 FPropertyTag: CPT_Bool (3) stores its boolean value in bit 7 (0x80)
            let bool_bit = if *b { 0x80u8 } else { 0x00u8 };
            let info = 0x03 | bool_bit;
            writer.write_u8(info)?;
        }
        TaggedValue::Float(f) => {
            let info = (2 << 4) | 0x04 | array_bit; // prop_type = 4, size_type = 2 (4 bytes)
            writer.write_u8(info)?;
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_f32::<LittleEndian>(*f)?;
        }
        TaggedValue::Object(obj_ref) => {
            let mut buf = Vec::new();
            write_compact_index(&mut buf, *obj_ref)?;
            let size_type = match buf.len() {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => 5,
            };
            let info = (size_type << 4) | 0x05 | array_bit;
            writer.write_u8(info)?;
            if size_type == 5 {
                writer.write_u8(buf.len() as u8)?;
            }
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_all(&buf)?;
        }
        TaggedValue::Name(s) => {
            let n_idx = package.names.add(s, 0) as i32;
            let mut buf = Vec::new();
            write_compact_index(&mut buf, n_idx)?;
            let size_type = match buf.len() {
                1 => 0,
                2 => 1,
                4 => 2,
                _ => 5,
            };
            let info = (size_type << 4) | 0x06 | array_bit;
            writer.write_u8(info)?;
            if size_type == 5 {
                writer.write_u8(buf.len() as u8)?;
            }
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_all(&buf)?;
        }
        TaggedValue::Str(s) => {
            // Write canonical UE1 CPT_Str (prop_type = 13 / 0x0D): CompactIndex length + ASCII + NUL
            let mut str_buf = Vec::new();
            if s.is_empty() {
                write_compact_index(&mut str_buf, 0)?;
            } else {
                let bytes = s.as_bytes();
                write_compact_index(&mut str_buf, (bytes.len() + 1) as i32)?;
                str_buf.extend_from_slice(bytes);
                str_buf.push(0);
            }
            let size = str_buf.len();
            let size_type: u8 = match size {
                1 => 0,
                2 => 1,
                4 => 2,
                12 => 3,
                16 => 4,
                sz if sz <= 255 => 5,
                sz if sz <= 65535 => 6,
                _ => 7,
            };
            let info = (size_type << 4) | 13 | array_bit;
            writer.write_u8(info)?;
            match size_type {
                5 => writer.write_u8(size as u8)?,
                6 => writer.write_u16::<LittleEndian>(size as u16)?,
                7 => writer.write_u32::<LittleEndian>(size as u32)?,
                _ => {}
            }
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_all(&str_buf)?;
        }
        TaggedValue::Vector(v) => {
            let info = (3 << 4) | 10 | array_bit; // prop_type = 10 (Struct), size_type = 3 (12 bytes)
            writer.write_u8(info)?;
            let struct_name_idx = package.names.add("Vector", 0) as i32;
            write_compact_index(writer, struct_name_idx)?;
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_f32::<LittleEndian>(v.x)?;
            writer.write_f32::<LittleEndian>(v.y)?;
            writer.write_f32::<LittleEndian>(v.z)?;
        }
        TaggedValue::Rotator { pitch, yaw, roll } => {
            let info = (3 << 4) | 10 | array_bit; // prop_type = 10 (Struct), size_type = 3 (12 bytes)
            writer.write_u8(info)?;
            let struct_name_idx = package.names.add("Rotator", 0) as i32;
            write_compact_index(writer, struct_name_idx)?;
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_i32::<LittleEndian>(*pitch)?;
            writer.write_i32::<LittleEndian>(*yaw)?;
            writer.write_i32::<LittleEndian>(*roll)?;
        }
        TaggedValue::Color { r, g, b, a } => {
            let info = (2 << 4) | 10 | array_bit; // prop_type = 10 (Struct), size_type = 2 (4 bytes)
            writer.write_u8(info)?;
            let struct_name_idx = package.names.add("Color", 0) as i32;
            write_compact_index(writer, struct_name_idx)?;
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_all(&[*r, *g, *b, *a])?;
        }
        TaggedValue::Scale { scale, sheer_rate, sheer_axis } => {
            let info = (5 << 4) | 10 | array_bit;
            writer.write_u8(info)?;
            let struct_name_idx = package.names.add("Scale", 0) as i32;
            write_compact_index(writer, struct_name_idx)?;
            writer.write_u8(17)?; // 17 bytes
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_f32::<LittleEndian>(scale.x)?;
            writer.write_f32::<LittleEndian>(scale.y)?;
            writer.write_f32::<LittleEndian>(scale.z)?;
            writer.write_f32::<LittleEndian>(*sheer_rate)?;
            writer.write_u8(*sheer_axis)?;
        }
        TaggedValue::Raw(payload) => {
            let size = payload.len();
            let size_type = match size {
                1 => 0,
                2 => 1,
                4 => 2,
                12 => 3,
                16 => 4,
                s if s <= 255 => 5,
                s if s <= 65535 => 6,
                _ => 7,
            };
            let (raw_prop_type, raw_struct_name) = match raw_meta {
                Some((pt, sn)) => (*pt & 0x0F, sn.as_deref()),
                None => (10u8, Some("Vector")),
            };
            let info = (size_type << 4) | raw_prop_type | array_bit;
            writer.write_u8(info)?;
            if raw_prop_type == 10 {
                let s_name = raw_struct_name.unwrap_or("Vector");
                let struct_name_idx = package.names.add(s_name, 0) as i32;
                write_compact_index(writer, struct_name_idx)?;
            }
            match size_type {
                5 => writer.write_u8(size as u8)?,
                6 => writer.write_u16::<LittleEndian>(size as u16)?,
                7 => writer.write_u32::<LittleEndian>(size as u32)?,
                _ => {}
            }
            if has_array {
                write_compact_index(writer, array_idx)?;
            }
            writer.write_all(payload)?;
        }
    }

    Ok(())
}

pub fn write_tagged_properties<W: Write>(
    writer: &mut W,
    props: &[TaggedProperty],
    package: &mut Package,
) -> io::Result<()> {
    for prop in props {
        write_tagged_property(writer, prop, package)?;
    }
    let n_none = package.names.add("None", 0) as i32;
    write_compact_index(writer, n_none)?;
    Ok(())
}
