use byteorder::{LittleEndian, ReadBytesExt};
use glam::Vec3;
use std::collections::HashMap;
use std::io::{self, Cursor};

use super::tagged_property::{
    read_tagged_properties_with_meta, TaggedProperty, TaggedValue,
};
use crate::package::compact_index::read_compact_index;
use crate::package::Package;

#[derive(Debug, Clone, PartialEq)]
pub struct ActorRecord {
    pub export_index: usize,
    pub name: String,
    pub class_name: String,
    pub flags: u32,

    // Transform
    pub location: Vec3,
    pub rotation: (i32, i32, i32),
    pub prepivot: Vec3,
    pub main_scale: Vec3,
    pub post_scale: Vec3,

    // Lighting
    pub light_brightness: u8,
    pub light_radius: u8,
    pub light_hue: u8,
    pub light_saturation: u8,
    pub light_type: u8,
    pub light_effect: u8,

    // Identification
    pub tag: String,
    pub event: String,
    pub group: String,

    // Brush-specific
    pub brush_ref: i32,
    pub csg_oper: u8,
    pub poly_flags: u32,

    // All properties
    pub properties: HashMap<String, TaggedProperty>,
    /// Symbolic UE1 object paths (`Class'Package.Group.Object'`) resolved for `TaggedValue::Object` properties.
    pub object_paths: HashMap<String, String>,
    /// Original `(prop_type, struct_name)` metadata for `TaggedValue::Raw` properties.
    pub raw_property_meta: HashMap<String, (u8, Option<String>)>,
}

impl Default for ActorRecord {
    fn default() -> Self {
        Self {
            export_index: 0,
            name: "Actor".into(),
            class_name: "Actor".into(),
            flags: 0,
            location: Vec3::ZERO,
            rotation: (0, 0, 0),
            prepivot: Vec3::ZERO,
            main_scale: Vec3::ONE,
            post_scale: Vec3::ONE,
            light_brightness: 64,
            light_radius: 32,
            light_hue: 0,
            light_saturation: 0,
            light_type: 1,
            light_effect: 0,
            tag: String::new(),
            event: String::new(),
            group: String::new(),
            brush_ref: 0,
            csg_oper: 1,
            poly_flags: 0,
            properties: HashMap::new(),
            object_paths: HashMap::new(),
            raw_property_meta: HashMap::new(),
        }
    }
}

impl ActorRecord {

    pub fn from_export(export_index: usize, package: &Package) -> io::Result<Self> {
        let export = package
            .exports
            .entries
            .get(export_index)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Export index out of range"))?;

        let name = package
            .get_name(export.object_name.max(0) as usize)
            .unwrap_or("UnknownActor")
            .to_string();

        let mut class_pkg_name = String::new();
        let class_name = if export.class_index < 0 {
            let imp_idx = (-export.class_index - 1) as usize;
            if let Some(imp) = package.imports.entries.get(imp_idx) {
                let mut cur_pkg = imp.package_index;
                while cur_pkg < 0 {
                    let p_idx = (-cur_pkg - 1) as usize;
                    if let Some(p_imp) = package.imports.entries.get(p_idx) {
                        if p_imp.package_index == 0 {
                            class_pkg_name = package
                                .get_name(p_imp.object_name.max(0) as usize)
                                .unwrap_or("")
                                .to_string();
                            break;
                        }
                        cur_pkg = p_imp.package_index;
                    } else {
                        break;
                    }
                }
                package
                    .get_name(imp.object_name.max(0) as usize)
                    .unwrap_or("Actor")
                    .to_string()
            } else {
                "Actor".to_string()
            }
        } else if export.class_index > 0 {
            class_pkg_name = "MyLevel".to_string();
            let exp_idx = (export.class_index - 1) as usize;
            if let Some(exp) = package.exports.entries.get(exp_idx) {
                package
                    .get_name(exp.object_name.max(0) as usize)
                    .unwrap_or("Actor")
                    .to_string()
            } else {
                "Actor".to_string()
            }
        } else {
            "Class".to_string()
        };

        let raw_bytes = package.get_export_bytes(export_index).unwrap_or(&[]);
        let mut cursor = Cursor::new(raw_bytes);

        // If RF_HasStack (0x02000000) is set, Actor begins with FStateFrame
        let has_stack = (export.object_flags & 0x02000000) != 0;
        if has_stack && !raw_bytes.is_empty() {
            if raw_bytes.len() >= 24
                && raw_bytes[0..8] == [0u8; 8]
                && raw_bytes[8..16] == [0xFFu8; 8]
                && raw_bytes[16..24] == [0u8; 8]
            {
                cursor.set_position(24);
            } else {
                let node = read_compact_index(&mut cursor).unwrap_or(0);
                let _state_node = read_compact_index(&mut cursor).unwrap_or(0);
                let _probe_mask = cursor.read_u64::<LittleEndian>().unwrap_or(0);
                let _latent_action = cursor.read_i32::<LittleEndian>().unwrap_or(0);
                if node != 0 {
                    let _script_offset = read_compact_index(&mut cursor).unwrap_or(0);
                }
            }
        }

        // Read Tagged Properties & Raw metadata
        let (mut properties, raw_property_meta) =
            read_tagged_properties_with_meta(&mut cursor, package)?;

        let mut object_paths = HashMap::new();
        if !class_pkg_name.is_empty() {
            object_paths.insert("__class_package".to_string(), class_pkg_name);
        }
        for (k, prop) in &mut properties {
            if let TaggedValue::Object(obj_idx) = prop.value {
                if obj_idx != 0 {
                    let full_path = package.resolve_full_object_path(obj_idx);
                    if full_path != "None" {
                        object_paths.insert(k.clone(), full_path.clone());
                        if class_name.eq_ignore_ascii_case("LevelInfo") {
                            let inner = full_path
                                .split('\'')
                                .nth(1)
                                .unwrap_or(&full_path)
                                .trim();
                            if k.eq_ignore_ascii_case("DefaultGameType") {
                                let normalized = if let Some(rest) = inner.strip_prefix("BotPack.") {
                                    format!("Botpack.{rest}")
                                } else {
                                    inner.to_string()
                                };
                                prop.value = TaggedValue::Str(normalized);
                            } else if k.eq_ignore_ascii_case("Song") {
                                let short = if let Some((a, b)) = inner.split_once('.') {
                                    if a.eq_ignore_ascii_case(b) {
                                        a.to_string()
                                    } else {
                                        inner.to_string()
                                    }
                                } else {
                                    inner.to_string()
                                };
                                prop.value = TaggedValue::Str(short);
                            }
                        }
                    }
                }
            }
        }

        // Extract standard actor properties
        let mut location = Vec3::ZERO;
        let mut rotation = (0, 0, 0);
        let mut prepivot = Vec3::ZERO;
        let mut main_scale = Vec3::ONE;
        let mut post_scale = Vec3::ONE;

        let mut light_brightness = 64;
        let mut light_radius = 64;
        let mut light_hue = 0;
        let mut light_saturation = 255;
        let mut light_type = 0;
        let mut light_effect = 0;

        let mut tag = String::new();
        let mut event = String::new();
        let mut group = String::new();

        let mut brush_ref = 0;
        let mut csg_oper = 1; // Default CSG_Add
        let mut poly_flags = 0;

        if let Some(prop) = properties.get("Location") {
            if let TaggedValue::Vector(v) = prop.value {
                location = v;
            }
        }

        if let Some(prop) = properties.get("Rotation") {
            if let TaggedValue::Rotator { pitch, yaw, roll } = prop.value {
                rotation = (pitch, yaw, roll);
            }
        }

        if let Some(prop) = properties.get("PrePivot") {
            if let TaggedValue::Vector(v) = prop.value {
                prepivot = v;
            }
        }

        if let Some(prop) = properties.get("MainScale") {
            if let TaggedValue::Scale { scale, .. } = prop.value {
                main_scale = scale;
            }
        }

        if let Some(prop) = properties.get("PostScale") {
            if let TaggedValue::Scale { scale, .. } = prop.value {
                post_scale = scale;
            }
        }

        if let Some(prop) = properties.get("LightBrightness") {
            if let TaggedValue::Byte(b) = prop.value {
                light_brightness = b;
            }
        }

        if let Some(prop) = properties.get("LightRadius") {
            if let TaggedValue::Byte(b) = prop.value {
                light_radius = b;
            }
        }

        if let Some(prop) = properties.get("LightHue") {
            if let TaggedValue::Byte(b) = prop.value {
                light_hue = b;
            }
        }

        if let Some(prop) = properties.get("LightSaturation") {
            if let TaggedValue::Byte(b) = prop.value {
                light_saturation = b;
            }
        }

        if let Some(prop) = properties.get("LightType") {
            if let TaggedValue::Byte(b) = prop.value {
                light_type = b;
            }
        }

        if let Some(prop) = properties.get("LightEffect") {
            if let TaggedValue::Byte(b) = prop.value {
                light_effect = b;
            }
        }

        if let Some(prop) = properties.get("Tag") {
            if let TaggedValue::Name(s) = &prop.value {
                tag = s.clone();
            }
        }

        if let Some(prop) = properties.get("Event") {
            if let TaggedValue::Name(s) = &prop.value {
                event = s.clone();
            }
        }

        if let Some(prop) = properties.get("Group") {
            if let TaggedValue::Name(s) = &prop.value {
                group = s.clone();
            }
        }

        if let Some(prop) = properties.get("Brush") {
            if let TaggedValue::Object(idx) = prop.value {
                brush_ref = idx;
            }
        }

        if let Some(prop) = properties.get("CsgOper") {
            if let TaggedValue::Byte(b) = prop.value {
                csg_oper = b;
            }
        }

        if let Some(prop) = properties.get("PolyFlags") {
            if let TaggedValue::Int(f) = prop.value {
                poly_flags = f as u32;
            }
        }

        Ok(Self {
            export_index,
            name,
            class_name,
            flags: export.object_flags,
            location,
            rotation,
            prepivot,
            main_scale,
            post_scale,
            light_brightness,
            light_radius,
            light_hue,
            light_saturation,
            light_type,
            light_effect,
            tag,
            event,
            group,
            brush_ref,
            csg_oper,
            poly_flags,
            properties,
            object_paths,
            raw_property_meta,
        })
    }
}
