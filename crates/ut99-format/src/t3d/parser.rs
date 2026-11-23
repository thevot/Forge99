use super::ast::{parse_vector, T3dActor, T3dBrush, T3dMap, T3dPoly};
use glam::Vec3;
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, thiserror::Error)]
pub enum T3dParseError {
    #[error("I/O error during T3D parsing: {0}")]
    Io(#[from] std::io::Error),
    #[error("Malformed T3D line at {line_number}: {line}")]
    MalformedLine { line_number: usize, line: String },
    #[error("Unexpected EOF while parsing T3D block")]
    UnexpectedEof,
}

pub fn parse_t3d<R: Read>(reader: R) -> Result<T3dMap, T3dParseError> {
    let buf_reader = BufReader::new(reader);
    let mut map = T3dMap::new();

    let mut current_actor: Option<T3dActor> = None;
    let mut current_brush: Option<T3dBrush> = None;
    let mut current_poly: Option<T3dPoly> = None;

    for (idx, line_res) in buf_reader.lines().enumerate() {
        let _line_num = idx + 1;
        let line_raw = line_res?;
        let line = line_raw.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
            continue;
        }

        let upper = line.to_uppercase();

        if upper.starts_with("BEGIN MAP") {
            // Map root started
        } else if upper.starts_with("END MAP") {
            break;
        } else if upper.starts_with("BEGIN ACTOR") {
            let mut actor = T3dActor::default();
            for token in line["BEGIN ACTOR".len()..].split_whitespace() {
                if let Some((k, v)) = token.split_once('=') {
                    match k.to_uppercase().as_str() {
                        "CLASS" => actor.class = v.trim_matches('"').to_string(),
                        "NAME" => actor.name = v.trim_matches('"').to_string(),
                        _ => {}
                    }
                }
            }
            current_actor = Some(actor);
        } else if upper.starts_with("END ACTOR") {
            if let Some(mut actor) = current_actor.take() {
                if let Some(brush) = current_brush.take() {
                    actor.brush = Some(brush);
                }
                map.actors.push(actor);
            }
        } else if upper.starts_with("BEGIN BRUSH") {
            let mut brush = T3dBrush::default();
            for token in line["BEGIN BRUSH".len()..].split_whitespace() {
                if let Some((k, v)) = token.split_once('=') {
                    if k.eq_ignore_ascii_case("NAME") {
                        brush.name = v.trim_matches('"').to_string();
                    }
                }
            }
            current_brush = Some(brush);
        } else if upper.starts_with("END BRUSH") {
            // Handled when End Actor is reached
        } else if upper.starts_with("BEGIN POLYLIST") || upper.starts_with("BEGIN POLY LIST")
            || upper.starts_with("END POLYLIST") || upper.starts_with("END POLY LIST")
        {
            // Container markers
        } else if upper.starts_with("BEGIN POLYGON") || upper.starts_with("BEGIN POLY") {
            let prefix_len = if upper.starts_with("BEGIN POLYGON") {
                "BEGIN POLYGON".len()
            } else {
                "BEGIN POLY".len()
            };

            let mut poly = T3dPoly::default();
            for token in line[prefix_len..].split_whitespace() {
                if let Some((k, v)) = token.split_once('=') {
                    match k.to_uppercase().as_str() {
                        "ITEM" => poly.item = v.to_string(),
                        "TEXTURE" => poly.texture = v.to_string(),
                        "FLAGS" => poly.flags = v.parse().unwrap_or(0),
                        "LINK" => poly.link = v.parse().unwrap_or(0),
                        "NORMAL" => {
                            if let Some(n) = parse_coordinate_triplet(v) {
                                poly.normal = n;
                            }
                        }
                        _ => {}
                    }
                }
            }
            current_poly = Some(poly);
        } else if upper.starts_with("END POLYGON") || upper.starts_with("END POLY") {
            if let Some(poly) = current_poly.take() {
                if let Some(ref mut brush) = current_brush {
                    brush.polys.push(poly);
                }
            }
        } else if let Some(ref mut poly) = current_poly {
            if upper.starts_with("VERTEX") {
                let rest = line["VERTEX".len()..].trim();
                if let Some(v) = parse_coordinate_triplet(rest) {
                    poly.vertices.push(v);
                }
            } else if upper.starts_with("ORIGIN") || upper.starts_with("BASE") {
                let rest = if upper.starts_with("ORIGIN") {
                    line["ORIGIN".len()..].trim()
                } else {
                    line["BASE".len()..].trim()
                };
                if let Some(b) = parse_coordinate_triplet(rest) {
                    poly.base = b;
                }
            } else if upper.starts_with("NORMAL") {
                let rest = line["NORMAL".len()..].trim();
                if let Some(n) = parse_coordinate_triplet(rest) {
                    poly.normal = n;
                }
            } else if upper.starts_with("TEXTUREU") {
                let rest = line["TEXTUREU".len()..].trim();
                if let Some(u) = parse_coordinate_triplet(rest) {
                    poly.texture_u = u;
                }
            } else if upper.starts_with("TEXTUREV") {
                let rest = line["TEXTUREV".len()..].trim();
                if let Some(v) = parse_coordinate_triplet(rest) {
                    poly.texture_v = v;
                }
            } else if upper.starts_with("PAN") {
                let rest = line["PAN".len()..].trim();
                for token in rest.split_whitespace() {
                    if let Some((k, v)) = token.split_once('=') {
                        match k.to_ascii_uppercase().as_str() {
                            "U" => poly.pan_u = v.parse().unwrap_or(0),
                            "V" => poly.pan_v = v.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
            } else if upper.starts_with("TEXTURE") {
                let rest = line["TEXTURE".len()..].trim();
                poly.texture = rest.to_string();
            }
        } else if let Some(ref mut actor) = current_actor {
            // General Actor property
            if let Some((k, v)) = line.split_once('=') {
                actor.properties.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }

    Ok(map)
}

fn parse_coordinate_triplet(s: &str) -> Option<Vec3> {
    let clean = s.trim();
    if clean.contains('=') {
        return parse_vector(clean);
    }

    let parts: Vec<&str> = if clean.contains(',') {
        clean.split(',').collect()
    } else {
        clean.split_whitespace().collect()
    };

    if parts.len() >= 3 {
        let x: f32 = parts[0].trim().parse().ok()?;
        let y: f32 = parts[1].trim().parse().ok()?;
        let z: f32 = parts[2].trim().parse().ok()?;
        Some(Vec3::new(x, y, z))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t3d::write_t3d;

    #[test]
    fn test_t3d_pan_u_v_round_trip() {
        let sample = "\
Begin Map
   Begin Actor Class=Brush Name=Brush1
      Begin Brush Name=Model1
         Begin PolyList
            Begin Polygon Item=Wall Texture=Base1 Flags=32 Link=0
               Origin   +00000.000000,+00000.000000,+00000.000000
               Normal   +00000.000000,+00000.000000,+00001.000000
               Pan      U=24 V=-48
               TextureU +00001.000000,+00000.000000,+00000.000000
               TextureV +00000.000000,+00001.000000,+00000.000000
               Vertex   -00128.000000,-00128.000000,+00000.000000
               Vertex   +00128.000000,-00128.000000,+00000.000000
               Vertex   +00128.000000,+00128.000000,+00000.000000
               Vertex   -00128.000000,+00128.000000,+00000.000000
            End Polygon
         End PolyList
      End Brush
   End Actor
End Map
";
        let map = parse_t3d(sample.as_bytes()).expect("parse_t3d failed");
        let brush = map.actors[0].brush.as_ref().expect("missing brush");
        assert_eq!(brush.polys[0].pan_u, 24);
        assert_eq!(brush.polys[0].pan_v, -48);

        let mut buf = Vec::new();
        write_t3d(&mut buf, &map).expect("write_t3d failed");
        let written = String::from_utf8(buf).expect("invalid utf8");
        assert!(written.contains("Pan      U=24 V=-48"));

        let reparsed = parse_t3d(written.as_bytes()).expect("reparse failed");
        let poly = &reparsed.actors[0].brush.as_ref().unwrap().polys[0];
        assert_eq!(poly.pan_u, 24);
        assert_eq!(poly.pan_v, -48);
    }
}

