use super::ast::{T3dActor, T3dBrush, T3dMap, T3dPoly};
use std::io::{self, Write};

pub fn write_t3d<W: Write>(writer: &mut W, map: &T3dMap) -> io::Result<()> {
    writeln!(writer, "Begin Map")?;

    for actor in &map.actors {
        write_actor(writer, actor)?;
    }

    writeln!(writer, "End Map")?;
    Ok(())
}

fn write_actor<W: Write>(writer: &mut W, actor: &T3dActor) -> io::Result<()> {
    writeln!(writer, "   Begin Actor Class={} Name={}", actor.class, actor.name)?;

    for (k, v) in &actor.properties {
        if k != "Brush" {
            writeln!(writer, "      {k}={v}")?;
        }
    }

    if let Some(ref brush) = actor.brush {
        write_brush(writer, brush)?;
    }

    if let Some(brush_prop) = actor.properties.get("Brush") {
        writeln!(writer, "      Brush={brush_prop}")?;
    }

    writeln!(writer, "   End Actor")?;
    Ok(())
}

fn write_brush<W: Write>(writer: &mut W, brush: &T3dBrush) -> io::Result<()> {
    writeln!(writer, "      Begin Brush Name={}", brush.name)?;
    writeln!(writer, "         Begin PolyList")?;

    for poly in &brush.polys {
        write_poly(writer, poly)?;
    }

    writeln!(writer, "         End PolyList")?;
    writeln!(writer, "      End Brush")?;
    Ok(())
}

fn write_poly<W: Write>(writer: &mut W, poly: &T3dPoly) -> io::Result<()> {
    let item_str = if !poly.item.is_empty() {
        format!(" Item={}", poly.item)
    } else {
        String::new()
    };
    let texture_str = if !poly.texture.is_empty() {
        format!(" Texture={}", poly.texture)
    } else {
        String::new()
    };
    let flags_str = if poly.flags != 0 {
        format!(" Flags={}", poly.flags)
    } else {
        String::new()
    };
    let link_str = format!(" Link={}", poly.link);

    writeln!(
        writer,
        "            Begin Polygon{}{}{}{}",
        item_str, texture_str, flags_str, link_str
    )?;

    writeln!(
        writer,
        "               Origin   {:+.6},{:+.6},{:+.6}",
        poly.base.x, poly.base.y, poly.base.z
    )?;
    writeln!(
        writer,
        "               Normal   {:+.6},{:+.6},{:+.6}",
        poly.normal.x, poly.normal.y, poly.normal.z
    )?;
    if poly.pan_u != 0 || poly.pan_v != 0 {
        writeln!(
            writer,
            "               Pan      U={} V={}",
            poly.pan_u, poly.pan_v
        )?;
    }
    writeln!(
        writer,
        "               TextureU {:+.6},{:+.6},{:+.6}",
        poly.texture_u.x, poly.texture_u.y, poly.texture_u.z
    )?;
    writeln!(
        writer,
        "               TextureV {:+.6},{:+.6},{:+.6}",
        poly.texture_v.x, poly.texture_v.y, poly.texture_v.z
    )?;

    for v in &poly.vertices {
        writeln!(writer, "               Vertex   {:+.6},{:+.6},{:+.6}", v.x, v.y, v.z)?;
    }

    writeln!(writer, "            End Polygon")?;
    Ok(())
}
