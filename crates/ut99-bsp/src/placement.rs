// crates/ut99-bsp/src/placement.rs
//
// Physical placement of actors against the generated architecture.
//
// The wizard authors every actor with an approximate position; this module resolves it to a spot that
//  - rests exactly on the supporting surface (`class_profiles::rest_offset`),
//  - has its whole collision cylinder inside open space (never inside walls, pillars, pads),
//  - does not overlap any previously placed actor,
//  - never ends up over a pit / on a different level than intended.
//
// The world is described by axis-aligned boxes: `Subtract` brushes are cavities, solid `Add` brushes are
// obstacles (portal / non-solid sheets are ignored). This matches what the wizard generates; the same
// predicates are re-validated on the real engine BSP by the integration tests.

use glam::Vec3;

use crate::brush::Brush;
use crate::class_profiles::{class_profile, is_gameplay_pickup, Mount};
use crate::math::Plane;
use crate::poly::{PF_NOT_SOLID, PF_PORTAL};
use crate::CsgOperation;

const EPS: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn contains_xy(&self, x: f32, y: f32) -> bool {
        x >= self.min.x - 0.1 && x <= self.max.x + 0.1 && y >= self.min.y - 0.1 && y <= self.max.y + 0.1
    }
    pub fn contains(&self, p: Vec3) -> bool {
        self.contains_xy(p.x, p.y) && p.z >= self.min.z - 0.1 && p.z <= self.max.z + 0.1
    }
}

/// Exact convex polyhedron volume backed by its world-space bounding box and outward half-space planes.
/// Accurately models cubes, 5-sided convex ramps (`build_ramp`), octagons (`build_aligned_octagon`),
/// and cylinders without AABB corner false-positives.
#[derive(Debug, Clone, PartialEq)]
pub struct ConvexVolume {
    pub aabb: Aabb,
    pub planes: Vec<Plane>,
}

impl ConvexVolume {
    pub fn from_brush(b: &Brush) -> Option<Self> {
        if b.polys.is_empty() {
            return None;
        }
        let (min, max) = b.world_bounds();
        let aabb = Aabb { min, max };
        let planes: Vec<Plane> = b.to_world_polys().into_iter().map(|p| p.plane).collect();
        if planes.is_empty() {
            return None;
        }
        Some(Self { aabb, planes })
    }

    /// True when `p` lies strictly inside the open interior of this convex volume.
    pub fn contains(&self, p: Vec3) -> bool {
        if !self.aabb.contains(p) {
            return false;
        }
        self.planes.iter().all(|pl| pl.dist_to_point(p) < -0.05)
    }

    /// True when `p` lies inside or touching the surface of this solid obstacle.
    pub fn contains_solid(&self, p: Vec3) -> bool {
        if !self.aabb.contains(p) {
            return false;
        }
        self.planes.iter().all(|pl| pl.dist_to_point(p) <= 0.05)
    }

    /// Computes the vertical interval `((z_low, low_up_nz), (z_high, high_up_nz))` where the vertical
    /// line through `(x, y)` intersects this convex polyhedron.
    /// Returns `None` if `(x, y)` lies outside the 2D footprint of the polyhedron.
    pub fn vertical_span(&self, x: f32, y: f32) -> Option<((f32, f32), (f32, f32))> {
        if !self.aabb.contains_xy(x, y) {
            return None;
        }
        let mut z_low = self.aabb.min.z;
        let mut low_nz = 1.0_f32;
        let mut z_high = self.aabb.max.z;
        let mut high_nz = 1.0_f32;

        for pl in &self.planes {
            let n = pl.normal;
            let rhs = pl.dist - (n.x * x + n.y * y);
            if n.z.abs() < 1e-4 {
                // Vertical wall plane: point must lie strictly inside or on boundary
                if rhs < -0.1 {
                    return None;
                }
            } else if n.z < 0.0 {
                // Downward outward normal -> lower bound on Z
                let z_bound = rhs / n.z;
                if z_bound >= z_low - 1e-3 {
                    z_low = z_bound;
                    low_nz = -n.z;
                }
            } else {
                // Upward outward normal -> upper bound on Z
                let z_bound = rhs / n.z;
                if z_bound <= z_high + 1e-3 {
                    z_high = z_bound;
                    high_nz = n.z;
                }
            }
        }

        if z_low < z_high {
            Some(((z_low, low_nz), (z_high, high_nz)))
        } else {
            None
        }
    }
}

/// Axis-aligned bounds of a brush in world space.
pub fn brush_aabb(b: &Brush) -> Option<Aabb> {
    if b.polys.is_empty() {
        return None;
    }
    let (min, max) = b.world_bounds();
    Some(Aabb { min, max })
}

#[derive(Debug, Clone, Default)]
pub struct PlacementWorld {
    pub cavities: Vec<ConvexVolume>,
    pub solids: Vec<ConvexVolume>,
    placed: Vec<(Vec3, f32, f32)>, // (center, radius, half height)
}

impl PlacementWorld {
    pub fn from_brushes<'a>(brushes: impl IntoIterator<Item = &'a Brush>) -> Self {
        let mut w = PlacementWorld::default();
        for b in brushes {
            let Some(vol) = ConvexVolume::from_brush(b) else {
                continue;
            };
            let ghost = !b.polys.is_empty()
                && b.polys
                    .iter()
                    .all(|p| p.flags & (PF_NOT_SOLID | PF_PORTAL) != 0);
            match b.csg_oper {
                CsgOperation::Subtract => w.cavities.push(vol),
                CsgOperation::Add if !ghost => w.solids.push(vol),
                _ => {}
            }
        }
        w
    }

    pub fn in_cavity(&self, p: Vec3) -> bool {
        self.cavities.iter().any(|c| c.contains(p))
    }

    pub fn in_solid(&self, p: Vec3) -> bool {
        self.solids.iter().any(|s| s.contains_solid(p))
    }

    /// Open space: inside some cavity and not inside an obstacle.
    pub fn empty_at(&self, p: Vec3) -> bool {
        self.in_cavity(p) && !self.in_solid(p)
    }

    /// Height of the flat walkable surface under `(x, y)` that is not higher than `z_hint`.
    pub fn floor_under(&self, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        self.floor_under_ex(x, y, z_hint, false)
    }

    /// Height of the walkable surface under `(x, y)` up to `z_hint`.
    /// When `allow_sloped` is false, only flat horizontal floors (`normal.z >= 0.98`) are accepted;
    /// when true, walkable ramps (`normal.z >= 0.70`) are also accepted.
    pub fn floor_under_ex(&self, x: f32, y: f32, z_hint: f32, allow_sloped: bool) -> Option<f32> {
        let min_nz = if allow_sloped { 0.70 } else { 0.98 };
        let mut cands: Vec<f32> = Vec::new();
        for c in &self.cavities {
            if let Some(((z_low, low_nz), _)) = c.vertical_span(x, y) {
                if low_nz >= min_nz {
                    cands.push(z_low);
                }
            }
        }
        for s in &self.solids {
            if let Some((_, (z_high, high_nz))) = s.vertical_span(x, y) {
                if high_nz >= min_nz {
                    cands.push(z_high);
                }
            }
        }
        cands.sort_by(|a, b| b.partial_cmp(a).unwrap());
        cands.dedup_by(|a, b| (*a - *b).abs() < EPS);
        cands.into_iter().find(|&z| {
            z <= z_hint + EPS
                && self.empty_at(Vec3::new(x, y, z + 2.0))
                && !self.empty_at(Vec3::new(x, y, z - 2.0))
        })
    }

    /// Height of the flat room/corridor cavity floor under `(x, y)` up to `z_hint`.
    /// Specifically excludes additive/semisolid brushes (pads, pedestals, trim) so actors requiring
    /// a structural room floor (such as PlayerStarts) never spawn atop decorative steps or pads.
    pub fn cavity_floor_under(&self, x: f32, y: f32, z_hint: f32) -> Option<f32> {
        let mut cands: Vec<f32> = Vec::new();
        for c in &self.cavities {
            if let Some(((z_low, low_nz), _)) = c.vertical_span(x, y) {
                if low_nz >= 0.98 {
                    cands.push(z_low);
                }
            }
        }
        cands.sort_by(|a, b| b.partial_cmp(a).unwrap());
        cands.dedup_by(|a, b| (*a - *b).abs() < EPS);
        cands.into_iter().find(|&z| {
            z <= z_hint + EPS
                && self.empty_at(Vec3::new(x, y, z + 2.0))
                && !self.empty_at(Vec3::new(x, y, z - 2.0))
        })
    }

    /// Height of the lowest flat ceiling above `(x, y, z)`.
    pub fn ceiling_above(&self, x: f32, y: f32, z: f32) -> Option<f32> {
        let mut cands: Vec<f32> = Vec::new();
        for c in &self.cavities {
            if let Some((_, (z_high, high_nz))) = c.vertical_span(x, y) {
                if high_nz >= 0.95 {
                    cands.push(z_high);
                }
            }
        }
        for s in &self.solids {
            if let Some(((z_low, low_nz), _)) = s.vertical_span(x, y) {
                if low_nz >= 0.95 {
                    cands.push(z_low);
                }
            }
        }
        cands.sort_by(|a, b| a.partial_cmp(b).unwrap());
        cands.into_iter().find(|&c| {
            c >= z - EPS
                && self.empty_at(Vec3::new(x, y, c - 2.0))
                && !self.empty_at(Vec3::new(x, y, c + 2.0))
        })
    }

    /// True when the whole collision cylinder (approximated by 16 rim probes at 3 heights plus the
    /// axis) lies in open space.
    pub fn cylinder_free(&self, center: Vec3, radius: f32, half_height: f32) -> bool {
        let r = radius + 2.0;
        let h = (half_height - 1.0).max(1.0);
        for dz in [-h, 0.0, h] {
            if !self.empty_at(center + Vec3::new(0.0, 0.0, dz)) {
                return false;
            }
            for k in 0..16 {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                let p = center + Vec3::new(a.cos() * r, a.sin() * r, dz);
                if !self.empty_at(p) {
                    return false;
                }
            }
        }
        true
    }

    fn overlaps_placed(&self, center: Vec3, radius: f32, half_height: f32) -> bool {
        self.placed.iter().any(|(c, r, hh)| {
            let dxy = ((c.x - center.x).powi(2) + (c.y - center.y).powi(2)).sqrt();
            dxy < r + radius + 4.0 && (c.z - center.z).abs() < hh + half_height
        })
    }

    /// Resolves an AI navigation point: rests on the floor like any other actor, but because it does not
    /// collide it only needs a thin free column (no overlap bookkeeping).
    pub fn place_nav(&self, class_name: &str, hint: Vec3) -> Option<Vec3> {
        let prof = class_profile(class_name);
        let mut offsets: Vec<(f32, f32)> = vec![(0.0, 0.0)];
        for ring in 1..=16 {
            let dist = ring as f32 * 24.0;
            let steps = 8 + ring * 2;
            for k in 0..steps {
                let a = k as f32 / steps as f32 * std::f32::consts::TAU;
                offsets.push((a.cos() * dist, a.sin() * dist));
            }
        }
        for (ox, oy) in offsets {
            let (x, y) = (hint.x + ox, hint.y + oy);
            let Some(f) = self.floor_under_ex(x, y, hint.z + 8.0, true) else {
                continue;
            };
            if f < hint.z - 80.0 {
                continue;
            }
            let center = Vec3::new(x, y, f + prof.rest_offset);
            if self.cylinder_free(center, 18.0, 40.0) {
                return Some(center);
            }
        }
        None
    }

    /// Moves a point light to the nearest open-space point (a light buried in a wall lights nothing).
    pub fn nudge_free(&self, hint: Vec3, clearance: f32) -> Option<Vec3> {
        let mut offsets: Vec<(f32, f32)> = vec![(0.0, 0.0)];
        for ring in 1..=10 {
            let dist = ring as f32 * 24.0;
            let steps = 8 + ring * 2;
            for k in 0..steps {
                let a = k as f32 / steps as f32 * std::f32::consts::TAU;
                offsets.push((a.cos() * dist, a.sin() * dist));
            }
        }
        offsets
            .into_iter()
            .map(|(ox, oy)| Vec3::new(hint.x + ox, hint.y + oy, hint.z))
            .find(|p| self.cylinder_free(*p, clearance, clearance))
    }

    /// Resolves an authored position `hint` for `class_name` to a physically valid location.
    /// Searches outward in rings around the authored `(x, y)`. Returns `None` for classes whose
    /// placement is owned elsewhere ([`Mount::Fixed`]) or when no valid spot exists nearby.
    pub fn place(&mut self, class_name: &str, hint: Vec3) -> Option<Vec3> {
        let prof = class_profile(class_name);
        if prof.mount == Mount::Fixed {
            return None;
        }
        let half_h = prof.height;
        let is_player_start = class_name.eq_ignore_ascii_case("PlayerStart");
        let clearance_radius = if is_player_start {
            (prof.radius + 16.0).max(36.0)
        } else if prof.mount == Mount::Ceiling {
            prof.radius + 6.0
        } else {
            prof.radius + 2.0
        };
        let mut offsets: Vec<(f32, f32)> = vec![(0.0, 0.0)];
        for ring in 1..=14 {
            let dist = ring as f32 * 24.0;
            let steps = 8 + ring * 2;
            for k in 0..steps {
                let a = k as f32 / steps as f32 * std::f32::consts::TAU;
                offsets.push((a.cos() * dist, a.sin() * dist));
            }
        }
        for (ox, oy) in offsets {
            let (x, y) = (hint.x + ox, hint.y + oy);
            let z = match prof.mount {
                Mount::Floor => {
                    // The surface must be the level the author meant, not a pit below or a roof above.
                    // For PlayerStarts and decorations, strictly require the room's main cavity floor, never raised decorative pads or trim.
                    let floor_opt = if is_player_start || !is_gameplay_pickup(class_name) {
                        self.cavity_floor_under(x, y, hint.z + 8.0)
                    } else {
                        self.floor_under(x, y, hint.z + 8.0)
                    };
                    let Some(f) = floor_opt else {
                        continue;
                    };
                    if f < hint.z - 80.0 {
                        continue;
                    }
                    if is_player_start {
                        if let Some(c) = self.ceiling_above(x, y, f + prof.rest_offset) {
                            if c < f + prof.rest_offset + half_h + 16.0 {
                                continue;
                            }
                        }
                    }
                    f + prof.rest_offset
                }
                Mount::Ceiling => {
                    let Some(c) = self.ceiling_above(x, y, hint.z) else {
                        continue;
                    };
                    // Ensure the ceiling is uniform across the fixture's radius so it never sits
                    // half-on, half-off a beam or wall edge:
                    let r = prof.radius + 4.0;
                    let uniform = [
                        (r, 0.0),
                        (-r, 0.0),
                        (0.0, r),
                        (0.0, -r),
                        (r * 0.707, r * 0.707),
                        (-r * 0.707, -r * 0.707),
                    ]
                    .iter()
                    .all(|(dx, dy)| {
                        self.ceiling_above(x + dx, y + dy, hint.z)
                            .map_or(false, |c_rim| (c_rim - c).abs() < 2.0)
                    });
                    if !uniform {
                        continue;
                    }
                    c - prof.rest_offset
                }
                Mount::Fixed => unreachable!(),
            };
            let center = Vec3::new(x, y, z);
            if !self.cylinder_free(center, clearance_radius, half_h) {
                continue;
            }
            if self.overlaps_placed(center, prof.radius, half_h) {
                continue;
            }
            self.placed.push((center, prof.radius, half_h));
            return Some(center);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brush::build_cube;

    fn room(center: Vec3, size: Vec3) -> Brush {
        let mut b = build_cube("room", CsgOperation::Subtract, size);
        b.location = center;
        b
    }

    fn solid(center: Vec3, size: Vec3) -> Brush {
        let mut b = build_cube("pillar", CsgOperation::Add, size);
        b.location = center;
        b
    }

    #[test]
    fn medkit_rests_on_floor_not_floating() {
        let mut w = PlacementWorld::from_brushes(&[room(Vec3::new(0.0, 0.0, 192.0), Vec3::new(512.0, 512.0, 384.0))]);
        let p = w.place("MedBox", Vec3::new(0.0, 0.0, 36.0)).unwrap();
        assert!((p.z - 9.0).abs() < 0.01, "MedBox must sit at floor+9, got {}", p.z);
    }

    #[test]
    fn fan_hangs_from_ceiling_and_clears_the_floor() {
        let mut w = PlacementWorld::from_brushes(&[room(Vec3::new(0.0, 0.0, 192.0), Vec3::new(512.0, 512.0, 384.0))]);
        let p = w.place("Fan2", Vec3::new(100.0, 100.0, 36.0)).unwrap();
        assert!((p.z - (384.0 - 52.0)).abs() < 0.01, "Fan2 must hang 52uu below the ceiling, got {}", p.z);
        assert!(p.z - 44.0 > 0.0, "fan cylinder must not touch the floor");
    }

    #[test]
    fn actor_is_pushed_out_of_a_pillar() {
        let mut w = PlacementWorld::from_brushes(&[
            room(Vec3::new(0.0, 0.0, 192.0), Vec3::new(512.0, 512.0, 384.0)),
            solid(Vec3::new(0.0, 0.0, 192.0), Vec3::new(48.0, 48.0, 384.0)),
        ]);
        let p = w.place("PlayerStart", Vec3::new(0.0, 0.0, 48.0)).unwrap();
        assert!(w.cylinder_free(p, 18.0, 40.0));
        assert!(p.truncate().length() > 24.0 + 18.0);
    }

    #[test]
    fn actor_never_lands_in_a_pit() {
        let mut w = PlacementWorld::from_brushes(&[
            room(Vec3::new(0.0, 0.0, 192.0), Vec3::new(512.0, 512.0, 384.0)),
            room(Vec3::new(0.0, 0.0, -48.0), Vec3::new(256.0, 256.0, 96.0)),
        ]);
        let p = w.place("PlayerStart", Vec3::new(0.0, 0.0, 48.0)).unwrap();
        assert!((p.z - 41.0).abs() < 0.01, "must stand on the room floor, not in the pit: {}", p.z);
        assert!(p.truncate().abs().max_element() > 128.0);
    }

    #[test]
    fn actors_do_not_overlap_each_other() {
        let mut w = PlacementWorld::from_brushes(&[room(Vec3::new(0.0, 0.0, 192.0), Vec3::new(512.0, 512.0, 384.0))]);
        let a = w.place("ShockRifle", Vec3::new(0.0, 0.0, 36.0)).unwrap();
        let b = w.place("ShockRifle", Vec3::new(0.0, 0.0, 36.0)).unwrap();
        assert!(a.truncate().distance(b.truncate()) >= 68.0);
    }

    #[test]
    fn octagon_room_rejects_outer_chamfer_corners() {
        let mut oct = crate::brush::build_aligned_octagon(
            "oct",
            CsgOperation::Subtract,
            Vec3::new(1024.0, 1024.0, 512.0),
        );
        oct.location = Vec3::new(0.0, 0.0, 256.0);
        let mut w = PlacementWorld::from_brushes(&[oct]);

        // Point in the solid rock outside the 45-degree chamfer must not be considered open space
        assert!(!w.empty_at(Vec3::new(480.0, 480.0, 64.0)));
        assert!(w.empty_at(Vec3::new(0.0, 0.0, 64.0)));

        // Hint placed inside the chamfered corner must be nudged inward into the valid octagon floor
        let p = w.place("MedBox", Vec3::new(450.0, 450.0, 36.0)).unwrap();
        assert!(w.cylinder_free(p, 22.0, 8.0));
        assert!((p.z - 9.0).abs() < 0.01);
    }

    #[test]
    fn convex_ramp_leaves_air_above_slope_free_and_supports_nav_nodes() {
        let r = room(Vec3::new(0.0, 0.0, 256.0), Vec3::new(512.0, 1024.0, 512.0));
        let mut ramp = crate::brush::build_ramp_oriented(
            "ramp",
            CsgOperation::Add,
            Vec3::new(256.0, 384.0, 192.0),
            crate::brush::RampDirection::North,
        );
        // Ramp sits at center Y=0, spanning Y in [-192, +192] and Z in [0, 192]
        ramp.location = Vec3::new(0.0, 0.0, 96.0);
        let mut w = PlacementWorld::from_brushes(&[r, ramp]);

        // At midpoint (0, 0), slope surface is at Z = 96.0: above 96 is open air, below 96 is solid ramp
        assert!(w.empty_at(Vec3::new(0.0, 0.0, 120.0)));
        assert!(!w.empty_at(Vec3::new(0.0, 0.0, 70.0)));

        // PathNode can rest on the sloped ramp surface (Z = 96 + 51 = 147)
        let nav = w.place_nav("PathNode", Vec3::new(0.0, 0.0, 100.0)).unwrap();
        assert!((nav.z - (96.0 + 51.0)).abs() < 1.0, "expected ~147, got {}", nav.z);

        // PlayerStart requires a flat floor, so hinting on the slope pushes it onto the flat floor beside the ramp
        let ps = w.place("PlayerStart", Vec3::new(0.0, 0.0, 48.0)).unwrap();
        assert!((ps.z - 41.0).abs() < 0.01);
    }
}
