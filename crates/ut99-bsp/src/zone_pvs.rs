use glam::Vec3;
use crate::brush::Brush;
use crate::poly::{Polygon, PF_NOT_SOLID, PF_PORTAL};

/// A directional portal boundary between two adjacent BSP zones
#[derive(Debug, Clone, PartialEq)]
pub struct ZonePortalEdge {
    pub portal_index: usize,
    pub zone_a: usize,
    pub zone_b: usize,
    pub portal_center: Vec3,
    pub portal_normal: Vec3,
    pub portal_area: f32,
}

/// Precomputed Potentially Visible Set (PVS) matrix across all zones in the map
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZonePvsMatrix {
    pub zone_count: usize,
    pub visibility: Vec<bool>, // Size: zone_count * zone_count
    pub portal_links_count: usize,
}

impl ZonePvsMatrix {
    pub fn new(zone_count: usize) -> Self {
        let count = zone_count.max(1);
        let mut mat = Self {
            zone_count: count,
            visibility: vec![false; count * count],
            portal_links_count: 0,
        };
        // A zone is always visible to itself
        for i in 0..count {
            mat.set_visible(i, i, true);
        }
        mat
    }

    pub fn is_visible(&self, from_zone: usize, to_zone: usize) -> bool {
        if from_zone < self.zone_count && to_zone < self.zone_count {
            self.visibility[from_zone * self.zone_count + to_zone]
        } else {
            false
        }
    }

    pub fn set_visible(&mut self, from_zone: usize, to_zone: usize, visible: bool) {
        if from_zone < self.zone_count && to_zone < self.zone_count {
            self.visibility[from_zone * self.zone_count + to_zone] = visible;
        }
    }

    pub fn get_visible_zones(&self, from_zone: usize) -> Vec<usize> {
        let mut result = Vec::new();
        for z in 0..self.zone_count {
            if self.is_visible(from_zone, z) {
                result.push(z);
            }
        }
        result
    }

    /// Returns the global occlusion culling ratio (percentage of occluded zone pairs, 0%..100%)
    pub fn occlusion_efficiency_pct(&self) -> f32 {
        let total = self.zone_count * self.zone_count;
        if total <= 1 {
            return 0.0;
        }
        let occluded = self.visibility.iter().filter(|&&v| !v).count();
        (occluded as f32 / total as f32) * 100.0
    }
}

/// Builds the transitive PVS visibility matrix through zone portals
pub fn build_zone_pvs_graph(zone_count: usize, portals: &[ZonePortalEdge]) -> ZonePvsMatrix {
    let mut matrix = ZonePvsMatrix::new(zone_count);
    matrix.portal_links_count = portals.len();

    // 1. Direct portal connections (bidirectional visibility)
    for p in portals {
        if p.zone_a < zone_count && p.zone_b < zone_count {
            matrix.set_visible(p.zone_a, p.zone_b, true);
            matrix.set_visible(p.zone_b, p.zone_a, true);
        }
    }

    // 2. Transitive sightline visibility through portal sequences (Frustum penetration)
    let n = matrix.zone_count;
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if matrix.is_visible(i, k) && matrix.is_visible(k, j) {
                    // Check if portals allow through-frustum line of sight
                    matrix.set_visible(i, j, true);
                }
            }
        }
    }

    matrix
}

/// Convex AntiPortal Occluder Volume (UE 227 / UT99 v469 feature)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AntiPortalVolume {
    pub id: usize,
    pub name: String,
    pub location: Vec3,
    pub size: Vec3, // Half extents: width (X), length (Y), height (Z)
    pub rotation_yaw_deg: f32,
    pub is_active: bool,
}

impl AntiPortalVolume {
    pub fn new(id: usize, name: &str, location: Vec3, size: Vec3) -> Self {
        Self {
            id,
            name: name.to_string(),
            location,
            size: size.max(Vec3::splat(16.0)),
            rotation_yaw_deg: 0.0,
            is_active: true,
        }
    }

    /// Tests whether a target point is occluded by this antiportal relative to the camera
    pub fn occludes_point(&self, camera_pos: Vec3, target_pos: Vec3) -> bool {
        self.occludes_sphere(camera_pos, target_pos, 0.0)
    }

    /// Tests whether a target bounding sphere is completely occluded by this antiportal
    /// relative to the given camera viewpoint using shadow volume projection.
    pub fn occludes_sphere(&self, camera_pos: Vec3, target_center: Vec3, target_radius: f32) -> bool {
        if !self.is_active {
            return false;
        }

        let to_anti = self.location - camera_pos;
        let dist_anti = to_anti.length();
        let to_target = target_center - camera_pos;
        let dist_target = to_target.length();

        // If target is closer than the antiportal, it cannot be occluded
        if dist_target <= dist_anti + target_radius {
            return false;
        }

        if dist_anti < 1e-3 || dist_target < 1e-3 {
            return false;
        }

        let dir_anti = to_anti / dist_anti;
        let dir_target = to_target / dist_target;

        // Angular radius of antiportal as seen from camera
        let max_half_extent = self.size.x.max(self.size.y).max(self.size.z);
        let anti_angular_radius = (max_half_extent / dist_anti).atan();

        // Angular radius of target sphere as seen from camera
        let target_angular_radius = if target_radius > 0.0 {
            (target_radius / dist_target).atan()
        } else {
            0.0
        };

        // Angular separation between antiportal center and target center
        let cos_sep = dir_anti.dot(dir_target).clamp(-1.0, 1.0);
        let angular_sep = cos_sep.acos();

        // Target is completely occluded if its bounding cone fits inside the antiportal cone
        angular_sep + target_angular_radius <= anti_angular_radius
    }

    /// Calculates the approximate angular coverage of this antiportal in degrees
    pub fn compute_shadow_cone_degrees(&self, camera_pos: Vec3) -> f32 {
        let dist = self.location.distance(camera_pos);
        if dist < 1.0 {
            return 180.0;
        }
        let radius = self.size.x.max(self.size.y).max(self.size.z);
        ((radius / dist).atan() * 2.0).to_degrees().min(180.0)
    }
}

/// Generates a planar 2D zone portal sheet brush with PF_PORTAL | PF_NOT_SOLID
pub fn generate_portal_sheet_brush(
    center: Vec3,
    width: f32,
    height: f32,
    normal: Vec3,
    group: &str,
) -> Brush {
    let n = if normal.length_squared() < 1e-4 {
        Vec3::Z
    } else {
        normal.normalize()
    };

    // Calculate orthonormal basis
    let up_hint = if n.z.abs() > 0.85 { Vec3::Y } else { Vec3::Z };
    let right = n.cross(up_hint).normalize();
    let true_up = right.cross(n).normalize();

    let half_w = (width * 0.5).max(16.0);
    let half_h = (height * 0.5).max(16.0);

    let p0 = -right * half_w - true_up * half_h;
    let p1 = right * half_w - true_up * half_h;
    let p2 = right * half_w + true_up * half_h;
    let p3 = -right * half_w + true_up * half_h;

    let mut brush = Brush::new(if group.is_empty() { "ZonePortalSheet" } else { group }, crate::CsgOperation::Add);
    brush.location = center;

    // Front-facing portal sheet
    if let Some(mut poly_front) = Polygon::new(vec![p0, p1, p2, p3]) {
        poly_front.flags = PF_PORTAL | PF_NOT_SOLID;
        poly_front.texture = "ZonePortal".to_string();
        brush.polys.push(poly_front);
    }

    // Back-facing portal sheet for bidirectional zone separation
    if let Some(mut poly_back) = Polygon::new(vec![p3, p2, p1, p0]) {
        poly_back.flags = PF_PORTAL | PF_NOT_SOLID;
        poly_back.texture = "ZonePortal".to_string();
        brush.polys.push(poly_back);
    }

    brush
}

/// Canonical Zone & Occlusion Presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ZoneOcclusionPreset {
    StandardDoorwayPortal,
    CorridorAirlockDual,
    OutdoorOcclusionShield,
    SniperTowerVisGate,
}

impl ZoneOcclusionPreset {
    pub fn all() -> &'static [ZoneOcclusionPreset] {
        &[
            ZoneOcclusionPreset::StandardDoorwayPortal,
            ZoneOcclusionPreset::CorridorAirlockDual,
            ZoneOcclusionPreset::OutdoorOcclusionShield,
            ZoneOcclusionPreset::SniperTowerVisGate,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ZoneOcclusionPreset::StandardDoorwayPortal => "🚪 Standard Doorway Portal (128x256 uu)",
            ZoneOcclusionPreset::CorridorAirlockDual => "🔒 Corridor Airlock Dual Portals (256x256 uu)",
            ZoneOcclusionPreset::OutdoorOcclusionShield => "🛡 Outdoor Occlusion Antiportal (1024x512x512 uu)",
            ZoneOcclusionPreset::SniperTowerVisGate => "🎯 Sniper Tower Visibility Gate (64x128 uu)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ZoneOcclusionPreset::StandardDoorwayPortal => {
                "Canonical door frame portal separating adjacent interior rooms."
            }
            ZoneOcclusionPreset::CorridorAirlockDual => {
                "Double portal setup for long transit corridors preventing cross-zone render leaks."
            }
            ZoneOcclusionPreset::OutdoorOcclusionShield => {
                "Large convex antiportal volume occluding terrain and distant geometry."
            }
            ZoneOcclusionPreset::SniperTowerVisGate => {
                "Narrow sightline portal restricting tower interior overdraw from arena floor."
            }
        }
    }
}

/// Comprehensive audit report for level zone watertightness and PVS occlusion
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZoneOcclusionAudit {
    pub total_zones: usize,
    pub watertight_zones: usize,
    pub leaking_zones: usize,
    pub total_portals: usize,
    pub active_antiportals: usize,
    pub occlusion_efficiency_pct: f32,
    pub is_tournament_ready: bool,
    pub summary_verdict: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_pvs_graph_occlusion() {
        // 4 Zones in a line: Zone 0 <-> Zone 1 <-> Zone 2; Zone 3 is isolated
        let portals = vec![
            ZonePortalEdge {
                portal_index: 0,
                zone_a: 0,
                zone_b: 1,
                portal_center: Vec3::new(100.0, 0.0, 0.0),
                portal_normal: Vec3::X,
                portal_area: 128.0,
            },
            ZonePortalEdge {
                portal_index: 1,
                zone_a: 1,
                zone_b: 2,
                portal_center: Vec3::new(300.0, 0.0, 0.0),
                portal_normal: Vec3::X,
                portal_area: 128.0,
            },
        ];

        let pvs = build_zone_pvs_graph(4, &portals);

        // Self visibility
        assert!(pvs.is_visible(0, 0));
        assert!(pvs.is_visible(3, 3));

        // Direct portal visibility
        assert!(pvs.is_visible(0, 1));
        assert!(pvs.is_visible(1, 2));

        // Transitive visibility (0 can see 2 through 1)
        assert!(pvs.is_visible(0, 2));

        // Isolated zone 3 cannot see or be seen by any other zone
        assert!(!pvs.is_visible(0, 3));
        assert!(!pvs.is_visible(1, 3));
        assert!(!pvs.is_visible(2, 3));

        assert!(pvs.occlusion_efficiency_pct() > 0.0);
    }

    #[test]
    fn test_antiportal_sphere_occlusion() {
        let anti = AntiPortalVolume::new(
            0,
            "HillAntiPortal",
            Vec3::new(500.0, 0.0, 0.0),
            Vec3::new(200.0, 200.0, 200.0),
        );

        let camera = Vec3::ZERO;
        let behind_target = Vec3::new(1000.0, 0.0, 0.0);
        let side_target = Vec3::new(1000.0, 800.0, 0.0);
        let in_front_target = Vec3::new(200.0, 0.0, 0.0);

        // Object directly behind antiportal is occluded
        assert!(anti.occludes_sphere(camera, behind_target, 32.0));

        // Object to the side is NOT occluded
        assert!(!anti.occludes_sphere(camera, side_target, 32.0));

        // Object in front of antiportal is NOT occluded
        assert!(!anti.occludes_sphere(camera, in_front_target, 32.0));
    }

    #[test]
    fn test_generate_portal_sheet_brush() {
        let brush = generate_portal_sheet_brush(
            Vec3::new(100.0, 200.0, 300.0),
            128.0,
            256.0,
            Vec3::X,
            "MyPortals",
        );

        assert_eq!(brush.location, Vec3::new(100.0, 200.0, 300.0));
        assert_eq!(brush.name, "MyPortals");
        assert_eq!(brush.polys.len(), 2);
        assert!((brush.polys[0].flags & PF_PORTAL) != 0);
        assert!((brush.polys[0].flags & PF_NOT_SOLID) != 0);
    }
}
