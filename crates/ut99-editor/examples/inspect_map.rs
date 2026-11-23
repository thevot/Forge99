use ut99_format::unr::UnrLevel;
use glam::Vec3;

fn main() {
    let map_path = r"UnrealTournament\Maps\CTF-IronThroneRing.unr";
    println!("Inspecting {}", map_path);
    let level = UnrLevel::open(map_path).expect("Failed to load map");

    let blue_loc = Vec3::new(2067.4165, 14.106846, 77.0);

    for (bi, b) in level.brushes.iter().enumerate() {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for p in &b.world_polys {
            for v in &p.vertices {
                min = min.min(*v);
                max = max.max(*v);
            }
        }
        let cyl_min = Vec3::new(blue_loc.x - 48.0, blue_loc.y - 48.0, blue_loc.z - 30.0);
        let cyl_max = Vec3::new(blue_loc.x + 48.0, blue_loc.y + 48.0, blue_loc.z + 30.0);

        let overlap_x = cyl_min.x <= max.x && cyl_max.x >= min.x;
        let overlap_y = cyl_min.y <= max.y && cyl_max.y >= min.y;
        let overlap_z = cyl_min.z <= max.z && cyl_max.z >= min.z;

        if overlap_x && overlap_y && overlap_z {
            println!("INTERSECTION with Brush #{bi} '{}' (csg_oper={}):", b.actor.name, b.actor.csg_oper);
            println!("    Brush AABB: [{:.1}, {:.1}, {:.1}] to [{:.1}, {:.1}, {:.1}]",
                min.x, min.y, min.z, max.x, max.y, max.z
            );
            println!("    Cylinder:   [{:.1}, {:.1}, {:.1}] to [{:.1}, {:.1}, {:.1}]",
                cyl_min.x, cyl_min.y, cyl_min.z, cyl_max.x, cyl_max.y, cyl_max.z
            );
        }
    }
}









