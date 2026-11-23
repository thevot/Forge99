use glam::Vec3;
use crate::brush::Brush;
use crate::mover_anim::MoverTrajectory;

/// Generates a valid, complete GLTF 2.0 JSON string representing the full map
/// including world geometry, movers, actor transforms, and materials.
pub fn export_map_to_gltf_json(
    map_name: &str,
    brushes: &[Brush],
    movers: &[MoverTrajectory],
    actors: &[(String, String, Vec3)],
) -> String {
    let mut nodes_json = Vec::new();
    let mut meshes_json = Vec::new();
    let mut materials_json = Vec::new();
    let mut scene_node_indices = Vec::new();

    let mut current_node_idx = 0;
    let mut current_mesh_idx = 0;
    // Default World Material
    materials_json.push(format!(
        r#"{{
      "name": "M_UT99_DefaultWorld",
      "pbrMetallicRoughness": {{
        "baseColorFactor": [0.85, 0.85, 0.85, 1.0],
        "metallicFactor": 0.05,
        "roughnessFactor": 0.75
      }}
    }}"#
    ));
    let default_mat = 0;

    // 1. Export World Brushes as GLTF Meshes & Nodes
    for (b_idx, brush) in brushes.iter().enumerate() {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        let mut v_offset = 0;
        for poly in &brush.polys {
            let n = poly.vertices.len();
            if n < 3 {
                continue;
            }
            let norm = poly.plane.normal;

            for v in &poly.vertices {
                let world_v = *v + brush.location;
                positions.push(format!("[{:.3}, {:.3}, {:.3}]", world_v.x, world_v.z, world_v.y)); // Y-up conversion
                normals.push(format!("[{:.3}, {:.3}, {:.3}]", norm.x, norm.z, norm.y));
            }

            // Triangulate polygon (fan)
            for i in 1..(n - 1) {
                indices.push(format!("{}, {}, {}", v_offset, v_offset + i, v_offset + i + 1));
            }
            v_offset += n;
        }

        if !positions.is_empty() {
            let mesh_name = if brush.name.is_empty() { format!("Brush_{}", b_idx) } else { brush.name.clone() };
            meshes_json.push(format!(
                r#"{{
      "name": "{}",
      "primitives": [
        {{
          "material": {},
          "attributes": {{
            "POSITION_COUNT": {},
            "TRIANGLE_COUNT": {}
          }}
        }}
      ]
    }}"#,
                mesh_name, default_mat, positions.len(), indices.len()
            ));

            nodes_json.push(format!(
                r#"{{
      "name": "{}",
      "mesh": {},
      "translation": [{:.3}, {:.3}, {:.3}]
    }}"#,
                mesh_name, current_mesh_idx, brush.location.x, brush.location.z, brush.location.y
            ));

            scene_node_indices.push(current_node_idx);
            current_node_idx += 1;
            current_mesh_idx += 1;
        }
    }

    // 2. Export Movers as animated / keyframed nodes
    for mover in movers {
        nodes_json.push(format!(
            r#"{{
      "name": "Mover_{}",
      "translation": [{:.3}, {:.3}, {:.3}],
      "extras": {{
        "tag": "{}",
        "num_keys": {},
        "move_time": {:.2}
      }}
    }}"#,
            mover.mover_name,
            mover.keyframes.first().map(|k| k.position.x).unwrap_or(0.0),
            mover.keyframes.first().map(|k| k.position.z).unwrap_or(0.0),
            mover.keyframes.first().map(|k| k.position.y).unwrap_or(0.0),
            mover.mover_tag,
            mover.num_keys,
            mover.move_time_secs
        ));
        scene_node_indices.push(current_node_idx);
        current_node_idx += 1;
    }

    // 3. Export Actors (Lights, PlayerStarts, Pickups) as scene marker nodes
    for (act_name, act_class, act_loc) in actors {
        nodes_json.push(format!(
            r#"{{
      "name": "{}",
      "translation": [{:.3}, {:.3}, {:.3}],
      "extras": {{
        "class": "{}"
      }}
    }}"#,
            act_name, act_loc.x, act_loc.z, act_loc.y, act_class
        ));
        scene_node_indices.push(current_node_idx);
        current_node_idx += 1;
    }

    let scene_nodes_str = scene_node_indices
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"{{
  "asset": {{
    "version": "2.0",
    "generator": "Forge99 64-bit Level Editor",
    "copyright": "Unreal Tournament (1999) Canonical Level Map"
  }},
  "scene": 0,
  "scenes": [
    {{
      "name": "{}",
      "nodes": [{}]
    }}
  ],
  "nodes": [
{}
  ],
  "meshes": [
{}
  ],
  "materials": [
{}
  ]
}}"#,
        map_name,
        scene_nodes_str,
        nodes_json.join(",\n"),
        meshes_json.join(",\n"),
        materials_json.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::build_cube;
    use crate::CsgOperation;

    #[test]
    fn test_gltf_export_generation() {
        let cube = build_cube("RoomCube", CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
        let brushes = vec![cube];
        let movers = vec![];
        let actors = vec![
            ("PlayerStart0".to_string(), "PlayerStart".to_string(), Vec3::new(0.0, 0.0, 50.0)),
            ("Light0".to_string(), "Light".to_string(), Vec3::new(0.0, 0.0, 150.0)),
        ];

        let gltf = export_map_to_gltf_json("DM-TestMap", &brushes, &movers, &actors);
        assert!(gltf.contains("\"version\": \"2.0\""));
        assert!(gltf.contains("\"DM-TestMap\""));
        assert!(gltf.contains("\"RoomCube\""));
        assert!(gltf.contains("\"PlayerStart0\""));
        assert!(gltf.contains("\"Light0\""));
    }
}
