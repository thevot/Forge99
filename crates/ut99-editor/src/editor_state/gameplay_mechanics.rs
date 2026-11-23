use super::*;
use glam::Vec3;

impl EditorState {
    pub fn compute_trajectories(&self) -> Vec<TrajectoryArc> {
        self.compute_trajectories_with_carved(None)
    }

    pub fn compute_trajectories_with_carved(
        &self,
        carved_polys: Option<&[(usize, ut99_bsp::Polygon)]>,
    ) -> Vec<TrajectoryArc> {
        let mut arcs = Vec::new();

        // 1. Kickers: Parabolic Ballistic Simulation
        struct KickerInfo {
            name: String,
            class: String,
            location: Vec3,
            velocity: Vec3,
        }
        let mut kickers = Vec::new();

        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if actor.class_name.contains("Kicker") || actor.class_name.contains("JumpPad") {
                    let mut vel = Vec3::new(0.0, 0.0, 1000.0);
                    if let Some(prop) = actor.properties.get("KickVelocity") {
                        match &prop.value {
                            ut99_format::unr::tagged_property::TaggedValue::Vector(v) => vel = *v,
                            ut99_format::unr::tagged_property::TaggedValue::Str(s) => {
                                if let Some(parsed) = ut99_format::parse_vector(s) {
                                    vel = parsed;
                                }
                            }
                            _ => {}
                        }
                    }
                    kickers.push(KickerInfo {
                        name: actor.name.clone(),
                        class: actor.class_name.clone(),
                        location: actor.location,
                        velocity: vel,
                    });
                }
            }
        }
        for actor in &self.map.actors {
            if actor.class.contains("Kicker") || actor.class.contains("JumpPad") {
                let mut vel = Vec3::new(0.0, 0.0, 1000.0);
                if let Some(s) = actor.get_property("KickVelocity") {
                    if let Some(parsed) = ut99_format::parse_vector(s) {
                        vel = parsed;
                    }
                }
                kickers.push(KickerInfo {
                    name: actor.name.clone(),
                    class: actor.class.clone(),
                    location: actor.get_location(),
                    velocity: vel,
                });
            }
        }

        let owned_carved: Vec<(usize, ut99_bsp::Polygon)>;
        let carved_slice: &[(usize, ut99_bsp::Polygon)] = if kickers.is_empty() {
            &[]
        } else if let Some(carved) = carved_polys {
            carved
        } else {
            owned_carved = ut99_bsp::bsp::evaluate_csg_brushes_indexed(&self.world_brushes);
            &owned_carved
        };
        let walkable_floor_polys: Vec<(&ut99_bsp::Polygon, Vec3, Vec3)> = carved_slice
            .iter()
            .map(|(_, p)| p)
            .filter(|p| {
                (p.flags & (ut99_bsp::PF_INVISIBLE | ut99_bsp::PF_PORTAL)) == 0
                    && p.plane.normal.z > 0.1
                    && p.vertices.len() >= 3
            })
            .map(|p| {
                let mut pmin = Vec3::splat(f32::MAX);
                let mut pmax = Vec3::splat(f32::MIN);
                for &v in &p.vertices {
                    pmin = pmin.min(v);
                    pmax = pmax.max(v);
                }
                (p, pmin, pmax)
            })
            .collect();

        let gravity = Vec3::new(0.0, 0.0, -950.0);
        let dt = 0.04;
        let max_time = 6.0;

        for kicker in kickers {
            let start_pos = kicker.location + Vec3::new(0.0, 0.0, 16.0);
            let mut points = vec![start_pos];
            let mut current_pos = start_pos;
            let mut current_vel = kicker.velocity;
            let mut landing_point = None;
            let mut flight_time = max_time;

            let mut t = 0.0;
            while t < max_time {
                let next_vel = current_vel + gravity * dt;
                let next_pos = current_pos + (current_vel + next_vel) * (0.5 * dt);
                t += dt;

                let mut hit_landing = false;
                let seg_dir = (next_pos - current_pos).normalize_or_zero();
                let seg_len = (next_pos - current_pos).length();

                if next_vel.z <= 0.0 {
                    let seg_min = current_pos.min(next_pos) - Vec3::splat(0.5);
                    let seg_max = current_pos.max(next_pos) + Vec3::splat(0.5);
                    for &(poly, pmin, pmax) in &walkable_floor_polys {
                        if seg_max.x < pmin.x
                            || seg_min.x > pmax.x
                            || seg_max.y < pmin.y
                            || seg_min.y > pmax.y
                            || seg_max.z < pmin.z
                            || seg_min.z > pmax.z
                        {
                            continue;
                        }
                        let v0 = poly.vertices[0];
                        for i in 1..(poly.vertices.len() - 1) {
                            let v1 = poly.vertices[i];
                            let v2 = poly.vertices[i + 1];
                            if let Some(hit_dist) = ut99_bsp::lighting::ray_intersects_triangle(current_pos, seg_dir, v0, v1, v2) {
                                if hit_dist > 0.0 && hit_dist <= seg_len {
                                    let hit_pt = current_pos + seg_dir * hit_dist;
                                    points.push(hit_pt);
                                    landing_point = Some(hit_pt);
                                    flight_time = t - dt + (hit_dist / seg_len.max(0.001)) * dt;
                                    hit_landing = true;
                                    break;
                                }
                            }
                        }
                        if hit_landing { break; }
                    }
                }

                if hit_landing {
                    break;
                }

                points.push(next_pos);
                current_pos = next_pos;
                current_vel = next_vel;

                if current_pos.z < start_pos.z - 3000.0 {
                    break;
                }
            }

            if landing_point.is_none() {
                if let Some(floor_z) = self.find_floor_z_below(current_pos, current_pos.z, None) {
                    let hit_pt = Vec3::new(current_pos.x, current_pos.y, floor_z);
                    points.push(hit_pt);
                    landing_point = Some(hit_pt);
                }
            }

            arcs.push(TrajectoryArc {
                source_name: kicker.name,
                source_class: kicker.class,
                start: start_pos,
                points,
                landing_point,
                flight_time,
                is_teleport: false,
            });
        }

        // 2. Teleporters: Elevated Quadratic Bezier Arcs
        struct TeleInfo {
            name: String,
            class: String,
            location: Vec3,
            url: String,
            tag: String,
        }
        let mut teles = Vec::new();

        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if actor.class_name.contains("Teleporter") {
                    let mut url = String::new();
                    if let Some(p) = actor.properties.get("URL") {
                        if let ut99_format::unr::tagged_property::TaggedValue::Str(s) = &p.value {
                            url = s.clone();
                        } else if let ut99_format::unr::tagged_property::TaggedValue::Name(s) = &p.value {
                            url = s.clone();
                        }
                    }
                    teles.push(TeleInfo {
                        name: actor.name.clone(),
                        class: actor.class_name.clone(),
                        location: actor.location,
                        url,
                        tag: actor.tag.clone(),
                    });
                }
            }
        }
        for actor in &self.map.actors {
            if actor.class.contains("Teleporter") {
                let url = actor.get_property("URL").unwrap_or("").to_string();
                let tag = actor.get_property("Tag").unwrap_or("").to_string();
                teles.push(TeleInfo {
                    name: actor.name.clone(),
                    class: actor.class.clone(),
                    location: actor.get_location(),
                    url,
                    tag,
                });
            }
        }

        for src in &teles {
            if src.url.is_empty() {
                continue;
            }
            for dest in &teles {
                if src.name == dest.name {
                    continue;
                }
                if src.url.eq_ignore_ascii_case(&dest.tag) || src.url.eq_ignore_ascii_case(&dest.name) {
                    let p0 = src.location + Vec3::new(0.0, 0.0, 40.0);
                    let p1 = dest.location + Vec3::new(0.0, 0.0, 40.0);
                    let dist = p0.distance(p1);
                    let arch_h = (dist * 0.25).clamp(120.0, 800.0);
                    let ctrl = (p0 + p1) * 0.5 + Vec3::new(0.0, 0.0, arch_h);

                    let num_segs = 32;
                    let mut points = Vec::with_capacity(num_segs + 1);
                    for i in 0..=num_segs {
                        let u = i as f32 / num_segs as f32;
                        let one_minus_u = 1.0 - u;
                        let pt = one_minus_u * one_minus_u * p0
                            + 2.0 * one_minus_u * u * ctrl
                            + u * u * p1;
                        points.push(pt);
                    }

                    arcs.push(TrajectoryArc {
                        source_name: src.name.clone(),
                        source_class: src.class.clone(),
                        start: p0,
                        points,
                        landing_point: Some(p1),
                        flight_time: 0.0,
                        is_teleport: true,
                    });
                }
            }
        }

        arcs
    }


    /// Simulates a real-time `TournamentPlayer` ballistic arc with UT99 gravity (`-950 UU/s^2`),
    /// `JumpZ`, `DodgeSpeed`, `Translocator` velocity, and optional `AirControl` (`BL-P3-04`).
    pub fn simulate_ut99_ballistic_trajectory(
        &self,
        start: Vec3,
        direction: Vec3,
        move_type: Ut99JumpMoveType,
        air_control: f32,
    ) -> TrajectoryArc {
        let horiz_dir = Vec3::new(direction.x, direction.y, 0.0).normalize_or_zero();
        let aim_dir = direction.normalize_or_zero();
        let initial_vel = match move_type {
            Ut99JumpMoveType::StandingJump => horiz_dir * 400.0 + Vec3::new(0.0, 0.0, 325.0),
            Ut99JumpMoveType::DodgeJump => horiz_dir * 600.0 + Vec3::new(0.0, 0.0, 210.0),
            Ut99JumpMoveType::ImpactHammerJump => horiz_dir * 400.0 + Vec3::new(0.0, 0.0, 680.0),
            Ut99JumpMoveType::JumpBootsJump => horiz_dir * 400.0 + Vec3::new(0.0, 0.0, 975.0),
            Ut99JumpMoveType::TranslocatorArc => aim_dir * 1150.0 + Vec3::new(0.0, 0.0, 200.0),
            Ut99JumpMoveType::CustomKicker(v) => v,
        };

        let gravity = Vec3::new(0.0, 0.0, -950.0);
        let air_accel = horiz_dir * (air_control.clamp(0.0, 1.0) * 400.0 * 0.35);
        let dt = 0.02;
        let max_time = 6.0;
        let mut points = vec![start];
        let mut current_pos = start;
        let mut current_vel = initial_vel;
        let mut landing_point = None;
        let mut flight_time = max_time;

        let mut t = 0.0;
        while t < max_time {
            let next_vel = current_vel + (gravity + air_accel) * dt;
            let next_pos = current_pos + (current_vel + next_vel) * (0.5 * dt);
            t += dt;

            let seg_dir = (next_pos - current_pos).normalize_or_zero();
            let seg_len = (next_pos - current_pos).length();
            let mut hit_landing = false;

            for brush in &self.world_brushes {
                for poly in brush.to_world_polys() {
                    if poly.plane.normal.z > 0.1 && poly.vertices.len() >= 3 {
                        let v0 = poly.vertices[0];
                        for i in 1..(poly.vertices.len() - 1) {
                            let v1 = poly.vertices[i];
                            let v2 = poly.vertices[i + 1];
                            if let Some(hit_dist) =
                                ut99_bsp::lighting::ray_intersects_triangle(current_pos, seg_dir, v0, v1, v2)
                            {
                                if hit_dist > 0.0 && hit_dist <= seg_len {
                                    let hit_pt = current_pos + seg_dir * hit_dist;
                                    points.push(hit_pt);
                                    landing_point = Some(hit_pt);
                                    flight_time = t - dt + (hit_dist / seg_len.max(0.001)) * dt;
                                    hit_landing = true;
                                    break;
                                }
                            }
                        }
                    }
                    if hit_landing {
                        break;
                    }
                }
                if hit_landing {
                    break;
                }
            }

            if hit_landing {
                break;
            }
            if next_pos.z <= start.z && current_vel.z < 0.0 && landing_point.is_none() && self.world_brushes.is_empty() {
                points.push(next_pos);
                landing_point = Some(next_pos);
                flight_time = t;
                break;
            }

            points.push(next_pos);
            current_pos = next_pos;
            current_vel = next_vel;

            if current_pos.z < start.z - 3000.0 {
                break;
            }
        }

        TrajectoryArc {
            source_name: format!("{:?}", move_type),
            source_class: "TournamentPlayer".to_string(),
            start,
            points,
            landing_point,
            flight_time,
            is_teleport: false,
        }
    }


    /// Solves the exact initial `KickVelocity` required to launch an Actor from `start` to `target`
    /// in `flight_time` seconds under vertical gravity `gravity_z` (default `-950.0` UU/s^2) (`BL-P3-04`).
    pub fn solve_kicker_velocity(start: Vec3, target: Vec3, flight_time: f32, gravity_z: f32) -> Vec3 {
        let t = flight_time.max(0.1);
        let delta = target - start;
        let vx = delta.x / t;
        let vy = delta.y / t;
        let vz = (delta.z - 0.5 * gravity_z * t * t) / t;
        Vec3::new(vx, vy, vz)
    }


    /// Spawns a complete UT99 v436/v469-compatible bot jump-pad chain (`Kicker` + `LiftExit` + `JumpSpot`)
    /// wired with matching `LiftTag` and solved ballistic `KickVelocity` (`BL-P3-04`).
    pub fn spawn_ut99_kicker_bot_chain(
        &mut self,
        start: Vec3,
        target: Vec3,
        flight_time: f32,
    ) -> (String, String, String) {
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        let kick_vel = Self::solve_kicker_velocity(start, target, flight_time, -950.0);
        let chain_idx = self.loaded_unr.as_ref().map_or(0, |u| u.actors.len());
        let lift_tag = format!("BotJumpPad_{}", chain_idx);

        let kicker_name = format!("Kicker_{}", chain_idx);
        let mut kicker = ut99_format::ActorRecord {
            class_name: "Kicker".to_string(),
            name: kicker_name.clone(),
            location: start,
            tag: lift_tag.clone(),
            ..Default::default()
        };
        kicker.properties.insert(
            "KickVelocity".to_string(),
            ut99_format::TaggedProperty {
                name: "KickVelocity".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Vector(kick_vel),
            },
        );
        kicker.properties.insert(
            "KickedClasses".to_string(),
            ut99_format::TaggedProperty {
                name: "KickedClasses".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Name("Pawn".to_string()),
            },
        );

        let exit_name = format!("LiftExit_{}", chain_idx + 1);
        let mut lift_exit = ut99_format::ActorRecord {
            class_name: "LiftExit".to_string(),
            name: exit_name.clone(),
            location: start,
            ..Default::default()
        };
        lift_exit.properties.insert(
            "LiftTag".to_string(),
            ut99_format::TaggedProperty {
                name: "LiftTag".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Name(lift_tag.clone()),
            },
        );

        let spot_name = format!("JumpSpot_{}", chain_idx + 2);
        let mut jump_spot = ut99_format::ActorRecord {
            class_name: "JumpSpot".to_string(),
            name: spot_name.clone(),
            location: target,
            ..Default::default()
        };
        jump_spot.properties.insert(
            "LiftTag".to_string(),
            ut99_format::TaggedProperty {
                name: "LiftTag".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Name(lift_tag.clone()),
            },
        );

        if let Some(ref mut unr) = self.loaded_unr {
            unr.actors.push(kicker.clone());
            unr.actors.push(lift_exit.clone());
            unr.actors.push(jump_spot.clone());
        }
        self.history.push(crate::history::Action::Compound {
            actions: vec![
                crate::history::Action::AddActor { actor: kicker },
                crate::history::Action::AddActor { actor: lift_exit },
                crate::history::Action::AddActor { actor: jump_spot },
            ],
            description: format!("Spawn UT99 Kicker + LiftExit + JumpSpot Chain '{}'", lift_tag),
        });
        self.log(format!(
            "Bot Ballistic Simulator: Spawned UT99 chain ({}, {}, {}) with KickVelocity ({:.1}, {:.1}, {:.1}).",
            kicker_name, exit_name, spot_name, kick_vel.x, kick_vel.y, kick_vel.z
        ));
        (kicker_name, exit_name, spot_name)
    }


    /// Computes the list of active links between trigger events (and LiftTag bot chains) and listener tags in the map
    pub fn get_event_tag_links(&self) -> Vec<(Vec3, Vec3, String)> {
        let mut links = Vec::new();

        let mut sources = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if !actor.event.is_empty() {
                    sources.push((actor.location, actor.event.clone()));
                }
                if let Some(prop) = actor.properties.get("LiftTag") {
                    match &prop.value {
                        ut99_format::TaggedValue::Name(s) | ut99_format::TaggedValue::Str(s) => {
                            if !s.is_empty() && !s.eq_ignore_ascii_case("None") {
                                sources.push((actor.location, s.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for mover in &self.movers {
            if !mover.event.is_empty() {
                sources.push((mover.brush.location, mover.event.clone()));
            }
        }

        let mut targets = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if !actor.tag.is_empty() {
                    targets.push((actor.location, actor.tag.clone()));
                }
            }
        }
        for mover in &self.movers {
            if !mover.tag.is_empty() {
                targets.push((mover.brush.location, mover.tag.clone()));
            }
        }

        let mut target_map: std::collections::HashMap<String, Vec<Vec3>> =
            std::collections::HashMap::with_capacity(targets.len());
        for (tgt_loc, tag) in &targets {
            target_map
                .entry(tag.to_ascii_lowercase())
                .or_default()
                .push(*tgt_loc);
        }

        for (src_loc, event) in &sources {
            if let Some(tgt_locs) = target_map.get(&event.to_ascii_lowercase()) {
                for &tgt_loc in tgt_locs {
                    if (*src_loc - tgt_loc).length_squared() > 1.0 {
                        links.push((*src_loc, tgt_loc, event.clone()));
                    }
                }
            }
        }

        links
    }


    /// Generates a canonical UT99 climbable ladder construct (`REQ-G05` — Definitive UnrealEd 2.0 Guide):
    /// - A thin vertical `Non-Solid` + `PF_PORTAL | PF_INVISIBLE` enclosure brush;
    /// - A `WaterZone` actor inside with `bWaterZone = true`, `ViewFog = (0,0,0)` (no underwater tint),
    ///   `FluidFriction = 0.3`, and upward `ZoneVelocity`;
    /// - Bottom and top `LiftExit` + central `LiftCenter` navigation markers so bots can climb it,
    /// all recorded atomically in `UndoHistory`.
    pub fn spawn_ut99_climbable_ladder_volume(
        &mut self,
        location: Vec3,
        width: f32,
        depth: f32,
        height: f32,
    ) -> (usize, String) {
        let w = width.max(32.0);
        let d = depth.max(16.0);
        let h = height.max(64.0);
        let ladder_idx = self.world_brushes.len();
        let ladder_tag = format!("Ladder_{}", ladder_idx);

        let mut portal_box = ut99_bsp::build_cube(
            format!("LadderPortal_{}", ladder_idx),
            CsgOperation::Add,
            Vec3::new(w, d, h),
        );
        portal_box.location = location;
        for p in &mut portal_box.polys {
            p.flags |= ut99_bsp::PF_NOT_SOLID | ut99_bsp::PF_PORTAL | ut99_bsp::PF_INVISIBLE;
            p.texture = "ZonePortal".to_string();
        }

        let mut actions = Vec::new();
        self.world_brushes.push(portal_box.clone());
        actions.push(Action::AddBrush { brush: portal_box });

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        let mut wz_name = String::new();
        if let Some(ref mut unr) = self.loaded_unr {
            let mut wz = ut99_format::ActorRecord {
                class_name: "WaterZone".to_string(),
                name: format!("WaterZone_Ladder_{}", ladder_idx),
                tag: ladder_tag.clone(),
                location,
                ..Default::default()
            };
            wz_name = wz.name.clone();
            wz.properties.insert(
                "bWaterZone".to_string(),
                TaggedProperty {
                    name: "bWaterZone".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );
            wz.properties.insert(
                "ViewFog".to_string(),
                TaggedProperty {
                    name: "ViewFog".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(Vec3::ZERO),
                },
            );
            wz.properties.insert(
                "FluidFriction".to_string(),
                TaggedProperty {
                    name: "FluidFriction".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(0.3),
                },
            );
            wz.properties.insert(
                "ZoneVelocity".to_string(),
                TaggedProperty {
                    name: "ZoneVelocity".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(Vec3::new(0.0, 0.0, 64.0)),
                },
            );
            unr.actors.push(wz.clone());
            actions.push(Action::AddActor { actor: wz });

            let mut lc = ut99_format::ActorRecord {
                class_name: "LiftCenter".to_string(),
                name: format!("LiftCenter_Ladder_{}", ladder_idx),
                location,
                ..Default::default()
            };
            lc.properties.insert(
                "LiftTag".to_string(),
                TaggedProperty {
                    name: "LiftTag".to_string(),
                    array_index: 0,
                    value: TaggedValue::Name(ladder_tag.clone()),
                },
            );
            unr.actors.push(lc.clone());
            actions.push(Action::AddActor { actor: lc });

            for (suffix, z_off) in [("Bottom", -h * 0.5 + 24.0), ("Top", h * 0.5 + 24.0)] {
                let mut le = ut99_format::ActorRecord {
                    class_name: "LiftExit".to_string(),
                    name: format!("LiftExit_Ladder_{}_{}", suffix, ladder_idx),
                    location: location + Vec3::new(0.0, 0.0, z_off),
                    ..Default::default()
                };
                le.properties.insert(
                    "LiftTag".to_string(),
                    TaggedProperty {
                        name: "LiftTag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(ladder_tag.clone()),
                    },
                );
                unr.actors.push(le.clone());
                actions.push(Action::AddActor { actor: le });
            }
        }

        self.history.push(Action::Compound {
            actions,
            description: format!("Spawn UT99 Climbable WaterZone Ladder '{}'", ladder_tag),
        });
        self.map_is_dirty = true;
        self.log(format!(
            "Spawned UT99 Climbable WaterZone Ladder '{}' ({:.0}x{:.0}x{:.0}) with ViewFog=(0,0,0) and Bot LiftTag nodes.",
            wz_name, w, d, h
        ));
        (ladder_idx, wz_name)
    }

}
