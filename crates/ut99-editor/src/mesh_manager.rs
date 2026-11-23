use glam::Vec3;
use ut99_bsp::{
    build_arch, build_cone, build_cube, build_cylinder, build_spiral_staircase, build_torus,
    export_brushes_to_obj, ArchConfig, ArchType, Brush, CsgOperation, ObjModel,
};

/// A reusable 3D mesh resource in the editor's library (UnrealEd 2.0 / 2.1 Mesh Browser).
#[derive(Debug, Clone)]
pub struct EditorMeshResource {
    pub name: String,
    pub category: String,
    pub description: String,
    pub brush: Brush,
}

impl EditorMeshResource {
    /// Total number of polygon faces in the mesh.
    pub fn poly_count(&self) -> usize {
        self.brush.polys.len()
    }

    /// Approximate total number of vertex references across all polygon faces.
    pub fn vertex_count(&self) -> usize {
        self.brush.polys.iter().map(|p| p.vertices.len()).sum()
    }

    /// Bounding box dimensions (X, Y, Z in Unreal Units).
    pub fn dimensions(&self) -> Vec3 {
        let (min, max) = self.brush.world_bounds();
        (max - min).max(Vec3::splat(1.0))
    }
}

/// Library of 3D meshes and static models available for 1-click placement, CSG extraction, and OBJ interop.
#[derive(Debug, Clone)]
pub struct MeshLibrary {
    pub meshes: Vec<EditorMeshResource>,
}

impl Default for MeshLibrary {
    fn default() -> Self {
        let mut library = Self { meshes: Vec::new() };

        // 1. Architecture: Fluted Classical Pillar (256 uu)
        let pillar_brush = build_cylinder("Pillar_Cylinder_256", CsgOperation::Add, 256.0, 48.0, 12);
        library.add_mesh(EditorMeshResource {
            name: "Pillar_Cylinder_256".to_string(),
            category: "Architecture".to_string(),
            description: "12-sided structural column suitable for halls, arches, and courtyard colonnades.".to_string(),
            brush: pillar_brush,
        });

        // 2. Decorations: Torch Wall / Floor Pedestal (96 uu)
        let pedestal_brush = build_cylinder("Torch_Pedestal_96", CsgOperation::Add, 96.0, 32.0, 8);
        library.add_mesh(EditorMeshResource {
            name: "Torch_Pedestal_96".to_string(),
            category: "Decorations".to_string(),
            description: "Octagonal brazier / pedestal base for torches and burning fires.".to_string(),
            brush: pedestal_brush,
        });

        // 3. Props: Industrial Cargo Crate (64x64x64 uu)
        let crate_brush = build_cube("Crate_Cargo_64", CsgOperation::Add, Vec3::splat(64.0));
        library.add_mesh(EditorMeshResource {
            name: "Crate_Cargo_64".to_string(),
            category: "Props".to_string(),
            description: "Standard industrial shipping crate; ideal for jump spots and arena cover.".to_string(),
            brush: crate_brush,
        });

        // 4. Props: Toxic / Hazard Waste Drum (80 uu)
        let barrel_brush = build_cylinder("Barrel_Hazard_80", CsgOperation::Add, 80.0, 24.0, 8);
        library.add_mesh(EditorMeshResource {
            name: "Barrel_Hazard_80".to_string(),
            category: "Props".to_string(),
            description: "Reinforced 8-sided chemical waste drum for sci-fi and industrial arenas.".to_string(),
            brush: barrel_brush,
        });

        // 5. Architecture: Gothic Vaulted Arch Portal (512 uu)
        let arch_cfg = ArchConfig {
            arch_type: ArchType::GothicPointed,
            span_width: 256.0,
            jamb_height: 256.0,
            total_height: 512.0,
            depth: 64.0,
            wall_thickness: 32.0,
            segments: 8,
            is_doorway: true,
        };
        let arch_brush = build_arch("Arch_Portal_512", CsgOperation::Add, &arch_cfg);
        library.add_mesh(EditorMeshResource {
            name: "Arch_Portal_512".to_string(),
            category: "Architecture".to_string(),
            description: "Classical Gothic pointed archway for castle doorways, hallways, and cathedral entrances.".to_string(),
            brush: arch_brush,
        });

        // 6. Architecture: Compact Spiral Staircase (384 uu)
        let spiral_brush = build_spiral_staircase(
            "Staircase_Spiral_384",
            CsgOperation::Add,
            12,     // 12 steps
            16.0,   // step height (16 uu)
            32.0,   // inner radius
            96.0,   // step width
            360.0,  // full turn
            false,  // clockwise
        );
        library.add_mesh(EditorMeshResource {
            name: "Staircase_Spiral_384".to_string(),
            category: "Architecture".to_string(),
            description: "Complete 360-degree spiral staircase with 12 steps for vertical tower navigation.".to_string(),
            brush: spiral_brush,
        });

        // 7. Decorations: Industrial Conduit Torus Ring (128 uu)
        let conduit_brush = build_torus("Conduit_Ring_128", CsgOperation::Add, 128.0, 20.0, 12, 8);
        library.add_mesh(EditorMeshResource {
            name: "Conduit_Ring_128".to_string(),
            category: "Decorations".to_string(),
            description: "Curved conduit torus ring for pipes, portal frames, and sci-fi generators.".to_string(),
            brush: conduit_brush,
        });

        // 8. Decorations: Pyramid / Obelisk Cap (64 uu)
        let pyramid_brush = build_cone("Pyramid_Cap_64", CsgOperation::Add, 64.0, 48.0, 4);
        library.add_mesh(EditorMeshResource {
            name: "Pyramid_Cap_64".to_string(),
            category: "Decorations".to_string(),
            description: "4-sided tapered pyramid capstone for pillars, roofs, and monument tops.".to_string(),
            brush: pyramid_brush,
        });

        library
    }
}

impl MeshLibrary {
    /// Returns the sorted, deduplicated list of categories available in the library (always starting with "All").
    pub fn categories(&self) -> Vec<String> {
        let mut cats = std::collections::BTreeSet::new();
        cats.insert("Architecture".to_string());
        cats.insert("Decorations".to_string());
        cats.insert("Props".to_string());
        cats.insert("Custom".to_string());

        for m in &self.meshes {
            if !m.category.is_empty() {
                cats.insert(m.category.clone());
            }
        }

        let mut res = vec!["All".to_string()];
        res.extend(cats);
        res
    }

    /// Finds a mesh by its unique identifier.
    pub fn get_mesh(&self, name: &str) -> Option<&EditorMeshResource> {
        self.meshes.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// Inserts or replaces a mesh in the library.
    pub fn add_mesh(&mut self, mesh: EditorMeshResource) {
        if let Some(pos) = self.meshes.iter().position(|m| m.name.eq_ignore_ascii_case(&mesh.name)) {
            self.meshes[pos] = mesh;
        } else {
            self.meshes.push(mesh);
        }
    }

    /// Imports a Wavefront OBJ string as a new mesh in the library.
    pub fn import_obj(&mut self, name: &str, category: &str, description: &str, obj_text: &str) -> Result<(), String> {
        let model = ObjModel::parse(obj_text)?;
        let brush = model.to_brush(name, CsgOperation::Add);
        self.add_mesh(EditorMeshResource {
            name: name.to_string(),
            category: if category.trim().is_empty() { "Custom".to_string() } else { category.trim().to_string() },
            description: description.to_string(),
            brush,
        });
        Ok(())
    }

    /// Exports an existing mesh in the library to a Wavefront OBJ string.
    pub fn export_obj(&self, name: &str) -> Option<String> {
        let mesh = self.get_mesh(name)?;
        Some(export_brushes_to_obj(&mesh.name, &[&mesh.brush]))
    }

    /// Converts a decoded `PackageMeshData` from a `.u` package into an `EditorMeshResource` (`BG-17`).
    /// When binary `FMeshVert` / `FMeshTri` geometry is decoded, constructs a real 3D triangle `Brush`.
    pub fn import_package_mesh_data(&mut self, pkg_name: &str, data: &ut99_format::package::PackageMeshData) -> bool {
        let mut brush = Brush::new(&data.name, CsgOperation::Add);
        if !data.vertices.is_empty() && !data.triangles.is_empty() {
            for &[i0, i1, i2] in &data.triangles {
                if let (Some(&v0), Some(&v1), Some(&v2)) = (
                    data.vertices.get(i0 as usize),
                    data.vertices.get(i1 as usize),
                    data.vertices.get(i2 as usize),
                ) {
                    let pts = vec![
                        Vec3::from_array(v0),
                        Vec3::from_array(v1),
                        Vec3::from_array(v2),
                    ];
                    if let Some(poly) = ut99_bsp::Polygon::new(pts) {
                        brush.polys.push(poly);
                    }
                }
            }
        }
        if brush.polys.is_empty() {
            brush = build_cylinder(&data.name, CsgOperation::Add, 64.0, 24.0, 8);
        }
        self.add_mesh(EditorMeshResource {
            name: data.name.clone(),
            category: format!("{} ({})", pkg_name, data.class_name),
            description: format!(
                "Exported {} from {}.u ({} bytes, {} verts, {} tris)",
                data.class_name,
                pkg_name,
                data.serial_size,
                data.vertices.len(),
                data.triangles.len()
            ),
            brush,
        });
        true
    }

    /// Opens a single `.u` package file and imports all `Mesh` / `LodMesh` / `SkeletalMesh` exports with decoded 3D geometry.
    pub fn import_u_package_file(&mut self, pkg_path: &std::path::Path) -> usize {
        if !pkg_path.exists() {
            return 0;
        }
        let pkg_name = pkg_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("CustomU");
        let mut added = 0;
        if let Ok(pkg) = ut99_format::package::Package::open(pkg_path) {
            for mesh_data in ut99_format::package::extract_u_package_meshes(&pkg, pkg_name) {
                if self.import_package_mesh_data(pkg_name, &mesh_data) {
                    added += 1;
                }
            }
        }
        added
    }

    /// Scans `.u` packages in `system_dir` for exported `LodMesh`, `Mesh`, and `SkeletalMesh` resources
    /// and decodes their 3D geometry into the library (`BG-10` / `BG-17`).
    pub fn scan_u_package_meshes_into_library(&mut self, system_dir: &std::path::Path) -> usize {
        if !system_dir.exists() {
            return 0;
        }
        let mut added = 0;
        let u_pkgs = ["Botpack.u", "UnrealShare.u", "UnrealI.u", "Engine.u"];
        for pkg_file in &u_pkgs {
            let pkg_path = system_dir.join(pkg_file);
            if pkg_path.exists() {
                added += self.import_u_package_file(&pkg_path);
            }
        }
        added
    }
}

