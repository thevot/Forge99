use glam::Vec3;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct T3dPoly {
    pub item: String,
    pub flags: u32,
    pub link: i32,
    pub pan_u: i16,
    pub pan_v: i16,
    pub normal: Vec3,
    pub base: Vec3,
    pub texture_u: Vec3,
    pub texture_v: Vec3,
    pub texture: String,
    pub vertices: Vec<Vec3>,
}

impl Default for T3dPoly {
    fn default() -> Self {
        Self {
            item: String::new(),
            flags: 0,
            link: 0,
            pan_u: 0,
            pan_v: 0,
            normal: Vec3::Z,
            base: Vec3::ZERO,
            texture_u: Vec3::X,
            texture_v: Vec3::Y,
            texture: String::new(),
            vertices: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct T3dBrush {
    pub name: String,
    pub polys: Vec<T3dPoly>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct T3dActor {
    pub class: String,
    pub name: String,
    pub properties: BTreeMap<String, String>,
    pub brush: Option<T3dBrush>,
}

impl T3dActor {
    pub fn new(class: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            class: class.into(),
            name: name.into(),
            properties: BTreeMap::new(),
            brush: None,
        }
    }

    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }

    pub fn set_property(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(key.into(), value.into());
    }

    pub fn get_location(&self) -> Vec3 {
        if let Some(loc_str) = self.get_property("Location") {
            parse_vector(loc_str).unwrap_or(Vec3::ZERO)
        } else {
            Vec3::ZERO
        }
    }

    pub fn set_location(&mut self, loc: Vec3) {
        self.set_property("Location", format!("(X={:.6},Y={:.6},Z={:.6})", loc.x, loc.y, loc.z));
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct T3dMap {
    pub actors: crate::TrackedVec<T3dActor>,
}

impl T3dMap {
    pub fn new() -> Self {
        Self { actors: crate::TrackedVec::new() }
    }

    pub fn find_actors_by_class(&self, class_name: &str) -> impl Iterator<Item = &T3dActor> {
        let cls = class_name.to_lowercase();
        self.actors.iter().filter(move |a| a.class.to_lowercase() == cls)
    }

    pub fn find_brushes(&self) -> impl Iterator<Item = &T3dActor> {
        self.actors.iter().filter(|a| a.brush.is_some() || a.class.eq_ignore_ascii_case("Brush"))
    }
}

/// Helper to parse Unreal vector format like "(X=100.0,Y=200.0,Z=300.0)"
pub fn parse_vector(s: &str) -> Option<Vec3> {
    let clean = s.trim().trim_start_matches('(').trim_end_matches(')');
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut z = 0.0f32;

    for part in clean.split(',') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let val = v.trim().parse::<f32>().ok()?;
            match k.trim().to_uppercase().as_str() {
                "X" => x = val,
                "Y" => y = val,
                "Z" => z = val,
                _ => {}
            }
        } else {
            // Coordinate without key (e.g. +100.0,+200.0,+300.0)
            let coords: Vec<&str> = clean.split(',').collect();
            if coords.len() == 3 {
                return Some(Vec3::new(
                    coords[0].trim().parse().ok()?,
                    coords[1].trim().parse().ok()?,
                    coords[2].trim().parse().ok()?,
                ));
            }
        }
    }

    Some(Vec3::new(x, y, z))
}
