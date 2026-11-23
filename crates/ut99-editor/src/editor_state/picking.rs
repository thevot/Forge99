use super::*;

const PICK_AABB_EPS: Vec3 = Vec3::splat(1e-3);

#[inline]
fn ray_hits_aabb_within(ray: Ray, bounds: (Vec3, Vec3), closest_dist: f32) -> bool {
    ray.intersect_aabb(bounds.0 - PICK_AABB_EPS, bounds.1 + PICK_AABB_EPS)
        .is_some_and(|t| t < closest_dist)
}

fn intersect_brush_triangles(
    ray: Ray,
    brush: &Brush,
    closest_dist: f32,
) -> Option<(f32, usize, Vec3)> {
    if brush.polys.is_empty() {
        return None;
    }
    let model = brush.model_matrix();
    let mut best_dist = closest_dist;
    let mut best: Option<(f32, usize, Vec3)> = None;

    for (p_idx, poly) in brush.polys.iter().enumerate() {
        let count = poly.vertices.len();
        if count < 3 {
            continue;
        }
        let v0 = model.transform_point3(poly.vertices[0]);
        let v1_0 = model.transform_point3(poly.vertices[1]);
        let v2_0 = model.transform_point3(poly.vertices[2]);
        let Some(world_plane) = ut99_bsp::Plane::from_points(v0, v1_0, v2_0) else {
            continue;
        };

        let mut v1 = v1_0;
        let mut v2 = v2_0;
        for i in 1..count - 1 {
            if i > 1 {
                v1 = v2;
                v2 = model.transform_point3(poly.vertices[i + 1]);
            }
            if let Some(d) = ray.intersect_triangle(v0, v1, v2) {
                if d < best_dist {
                    best_dist = d;
                    best = Some((d, p_idx, world_plane.normal));
                }
            }
        }
    }

    best
}

impl EditorState {
    pub fn select_at_ray(&mut self, ray: Ray) -> bool {
        self.select_at_ray_with_modifier(ray, false)
    }

    pub fn select_at_ray_with_modifier(&mut self, ray: Ray, multi_select: bool) -> bool {
        let mut closest_dist = f32::MAX;
        let mut hit_actor = None;
        let mut hit_brush = None;
        let mut hit_surface = None;
        let mut hit_mover = None;
        let mut hit_static = None;

        // 1. Hit test actors (with enhanced volumetric bounds and visibility/lock checks)
        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                if !self.is_actor_visible(idx) || self.is_actor_locked(idx) {
                    continue;
                }
                let (center, hit_radius) = match actor.class_name.as_str() {
                    "Light" => (actor.location, 36.0),
                    "PlayerStart" => (actor.location + Vec3::new(0.0, 0.0, 36.0), 38.0),
                    "PathNode" => (actor.location, 32.0),
                    _ => (actor.location, 36.0),
                };
                if let Some(d) = ray.intersect_sphere(center, hit_radius) {
                    if d < closest_dist {
                        closest_dist = d;
                        hit_actor = Some(idx);
                        hit_brush = None;
                        hit_surface = None;
                        hit_mover = None;
                        hit_static = None;
                    }
                }
            }
        }

        self.ensure_mover_bounds();
        self.ensure_static_mesh_bounds();
        self.ensure_world_brush_bounds();
        {
            let cache = self.scene_bounds_cache.borrow();

            // 2. Hit test movers (Doors & Lifts)
            for (m_idx, (mover, &bounds)) in self.movers.iter().zip(cache.movers.iter()).enumerate() {
                if !self.is_mover_visible(m_idx) || self.is_mover_locked(m_idx) {
                    continue;
                }
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, _)) = intersect_brush_triangles(ray, &mover.brush, closest_dist) {
                    closest_dist = d;
                    hit_mover = Some(m_idx);
                    hit_brush = None;
                    hit_surface = None;
                    hit_actor = None;
                    hit_static = None;
                }
            }

            // 3. Hit test static meshes
            for (s_idx, (mesh, &bounds)) in self.static_meshes.iter().zip(cache.static_meshes.iter()).enumerate() {
                if !self.is_static_mesh_visible(s_idx) || self.is_static_mesh_locked(s_idx) {
                    continue;
                }
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, _)) = intersect_brush_triangles(ray, mesh, closest_dist) {
                    closest_dist = d;
                    hit_static = Some(s_idx);
                    hit_mover = None;
                    hit_brush = None;
                    hit_surface = None;
                    hit_actor = None;
                }
            }

            // 4. Hit test world brushes
            for (b_idx, (brush, &bounds)) in self.world_brushes.iter().zip(cache.world_brushes.iter()).enumerate() {
                if !self.is_brush_visible(b_idx) || self.is_brush_locked(b_idx) {
                    continue;
                }
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, brush, closest_dist) {
                    closest_dist = d;
                    hit_brush = Some(b_idx);
                    hit_surface = Some((b_idx, p_idx));
                    hit_actor = None;
                    hit_mover = None;
                    hit_static = None;
                }
            }
        }

        if let Some(s_idx) = hit_static {
            if !multi_select {
                self.clear_multi_selection();
                self.selected_mover_index = None;
                self.selected_brush_index = None;
                self.selected_actor_index = None;
                self.selected_surfaces.clear();
                self.is_builder_selected = false;
            }
            self.selected_static_mesh_index = Some(s_idx);
            return true;
        }

        if let Some(m_idx) = hit_mover {
            if !multi_select {
                self.clear_multi_selection();
                self.selected_brush_index = None;
                self.selected_actor_index = None;
                self.selected_static_mesh_index = None;
                self.selected_surfaces.clear();
                self.is_builder_selected = false;
            }
            self.selected_mover_index = Some(m_idx);
            let m = &self.movers[m_idx];
            self.log(format!("Selected Mover: {} at ({:.0}, {:.0}, {:.0})", m.name, m.brush.location.x, m.brush.location.y, m.brush.location.z));
            return true;
        } else if let Some(a_idx) = hit_actor {
            if multi_select {
                if self.multi_selected_actors.contains(&a_idx) {
                    self.multi_selected_actors.remove(&a_idx);
                } else {
                    self.multi_selected_actors.insert(a_idx);
                }
            } else {
                self.clear_multi_selection();
                self.selected_brush_index = None;
                self.selected_mover_index = None;
                self.selected_static_mesh_index = None;
                self.selected_surfaces.clear();
                self.is_builder_selected = false;
                self.selected_actor_index = Some(a_idx);
            }
            if let Some(ref unr) = self.loaded_unr {
                let a = &unr.actors[a_idx];
                self.log(format!("Selected Actor: {} [{}] at ({:.0}, {:.0}, {:.0})", a.name, a.class_name, a.location.x, a.location.y, a.location.z));
            }
            return true;
        } else if let Some(b_idx) = hit_brush {
            if multi_select {
                if self.multi_selected_brushes.contains(&b_idx) {
                    self.multi_selected_brushes.remove(&b_idx);
                } else {
                    self.multi_selected_brushes.insert(b_idx);
                }
            } else {
                self.clear_multi_selection();
                self.selected_actor_index = None;
                self.selected_mover_index = None;
                self.selected_static_mesh_index = None;
                self.is_builder_selected = false;
                self.selected_brush_index = Some(b_idx);
            }
            if let Some(surf) = hit_surface {
                if multi_select {
                    if self.selected_surfaces.contains(&surf) {
                        self.selected_surfaces.remove(&surf);
                    } else {
                        self.selected_surfaces.insert(surf);
                    }
                } else {
                    self.selected_surfaces.clear();
                    self.selected_surfaces.insert(surf);
                }
                let b = &self.world_brushes[b_idx];
                let tex = b.polys.get(surf.1).map(|p| p.texture.as_str()).unwrap_or("None");
                self.log(format!("Selected Surface: Brush #{} [{}], Poly #{} (Texture: '{}')", b_idx, b.name, surf.1, tex));
            }
            return true;
        }

        false
    }

    /// Interactive Surface Magnet (MOU-02): Snaps an actor to a world surface hit point and aligns to surface normal.
    pub fn snap_actor_to_surface(&mut self, actor_idx: usize, hit_point: Vec3, hit_normal: Vec3, align_normal: bool) -> bool {
        let Some(ref mut unr) = self.loaded_unr else { return false; };
        let Some(actor) = unr.actors.get_mut(actor_idx) else { return false; };

        let profile = ut99_bsp::class_profiles::class_profile(&actor.class_name);
        let n = hit_normal.normalize();

        let clearance = if n.z > 0.7 {
            profile.rest_offset
        } else if n.z < -0.7 {
            profile.height * 0.5
        } else {
            profile.radius.clamp(16.0, 48.0)
        };

        actor.location = hit_point + n * clearance;

        if align_normal {
            if n.z > 0.999 {
                actor.rotation.0 = 0;
                actor.rotation.2 = 0;
            } else if n.z < -0.999 {
                actor.rotation.0 = 32768;
                actor.rotation.2 = 0;
            } else {
                let yaw = (n.y.atan2(n.x) * 65536.0 / std::f32::consts::TAU) as i32;
                let pitch = ((-n.z).asin() * 65536.0 / std::f32::consts::TAU) as i32;
                actor.rotation = (pitch, yaw, 0);
            }
        }

        unr.sync_actor_views();
        self.map_is_dirty = true;
        true
    }

    /// Interactive Surface Magnet (MOU-02): Snaps a world brush to a world surface hit point.
    pub fn snap_brush_to_surface(&mut self, brush_idx: usize, hit_point: Vec3, hit_normal: Vec3) -> bool {
        let Some(brush) = self.world_brushes.get_mut(brush_idx) else { return false; };
        let (min, max) = brush.world_bounds();
        let half_ext = (max - min) * 0.5;
        let n = hit_normal.normalize();
        
        let clearance = if n.z > 0.7 {
            half_ext.z
        } else if n.z < -0.7 {
            half_ext.z
        } else if n.x.abs() > n.y.abs() {
            half_ext.x
        } else {
            half_ext.y
        };

        brush.location = hit_point + n * clearance;
        self.map_is_dirty = true;
        true
    }

    /// Interactive Vertex Magnet (MOU-05): Finds the nearest vertex among all visible world brushes.
    pub fn find_nearest_world_vertex(&self, ray: Ray, max_dist_world: f32) -> Option<Vec3> {
        let mut closest_vert = None;
        let mut min_dist = max_dist_world;

        self.ensure_world_brush_bounds();
        let cache = self.scene_bounds_cache.borrow();
        for (brush, &(b_min, b_max)) in self.world_brushes.iter().zip(cache.world_brushes.iter()) {
            let pad = Vec3::splat(min_dist + 1e-3);
            if ray.intersect_aabb(b_min - pad, b_max + pad).is_none() {
                continue;
            }
            let model = brush.model_matrix();
            for poly in &brush.polys {
                if poly.vertices.len() < 3 {
                    continue;
                }
                let v0 = model.transform_point3(poly.vertices[0]);
                let v1 = model.transform_point3(poly.vertices[1]);
                let v2 = model.transform_point3(poly.vertices[2]);
                if ut99_bsp::Plane::from_points(v0, v1, v2).is_none() {
                    continue;
                }
                for v in [v0, v1, v2]
                    .into_iter()
                    .chain(poly.vertices[3..].iter().map(|&lv| model.transform_point3(lv)))
                {
                    let to_v = v - ray.origin;
                    let proj = to_v.dot(ray.direction);
                    if proj > 0.0 {
                        let ray_pt = ray.origin + ray.direction * proj;
                        let d = ray_pt.distance(v);
                        if d < min_dist {
                            min_dist = d;
                            closest_vert = Some(v);
                        }
                    }
                }
            }
        }
        closest_vert
    }

    /// Context-menu right-click ray selection (`UX-06`):
    /// Preserves an active multi-selection when the user right-clicks on an already-selected brush/actor
    /// or in empty space, so the `Multi-Selection` batch context menu remains reachable.
    /// If the user right-clicks an unselected item (or has no multi-selection), selects the clicked item.
    pub fn select_at_ray_for_context_menu(&mut self, ray: Ray) {
        if !self.has_multi_selection() {
            self.select_at_ray_with_modifier(ray, false);
            return;
        }

        let mut closest_dist = f32::MAX;
        let mut hit_actor = None;
        let mut hit_brush = None;
        let mut hit_mover = None;

        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                let hit_radius = match actor.class_name.as_str() {
                    "Light" => 32.0,
                    "PlayerStart" => 40.0,
                    "PathNode" => 28.0,
                    _ => 36.0,
                };
                if let Some(d) = ray.intersect_sphere(actor.location, hit_radius) {
                    if d < closest_dist {
                        closest_dist = d;
                        hit_actor = Some(idx);
                        hit_brush = None;
                        hit_mover = None;
                    }
                }
            }
        }

        self.ensure_world_brush_bounds();
        self.ensure_mover_bounds();
        {
            let cache = self.scene_bounds_cache.borrow();

            for (b_idx, (brush, &bounds)) in self.world_brushes.iter().zip(cache.world_brushes.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, _)) = intersect_brush_triangles(ray, brush, closest_dist) {
                    closest_dist = d;
                    hit_brush = Some(b_idx);
                    hit_actor = None;
                    hit_mover = None;
                }
            }

            for (m_idx, (mover, &bounds)) in self.movers.iter().zip(cache.movers.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, _)) = intersect_brush_triangles(ray, &mover.brush, closest_dist) {
                    closest_dist = d;
                    hit_mover = Some(m_idx);
                    hit_brush = None;
                    hit_actor = None;
                }
            }
        }

        let hits_existing_multi = match (hit_mover, hit_actor, hit_brush) {
            (None, None, None) => true,
            (None, Some(a_idx), None) => self.is_actor_selected(a_idx),
            (None, None, Some(b_idx)) => self.is_brush_selected(b_idx),
            _ => false,
        };

        if !hits_existing_multi {
            self.clear_multi_selection();
            self.select_at_ray_with_modifier(ray, false);
        }
    }

    /// Samples/eyedrops the texture and UV vectors on the polygon intersected by ray (world brushes, movers, static meshes)
    pub fn pick_texture_at_ray(&mut self, ray: Ray) -> Option<String> {
        enum PickSource {
            Brush(usize, usize),
            Mover(usize, usize),
            StaticMesh(usize, usize),
        }

        let mut closest_dist = f32::MAX;
        let mut hit_pick: Option<PickSource> = None;

        self.ensure_world_brush_bounds();
        self.ensure_mover_bounds();
        self.ensure_static_mesh_bounds();
        {
            let cache = self.scene_bounds_cache.borrow();

            // 1. World brushes
            for (b_idx, (brush, &bounds)) in self.world_brushes.iter().zip(cache.world_brushes.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, brush, closest_dist) {
                    closest_dist = d;
                    hit_pick = Some(PickSource::Brush(b_idx, p_idx));
                }
            }

            // 2. Movers
            for (m_idx, (mover, &bounds)) in self.movers.iter().zip(cache.movers.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, &mover.brush, closest_dist) {
                    closest_dist = d;
                    hit_pick = Some(PickSource::Mover(m_idx, p_idx));
                }
            }

            // 3. Static Meshes
            for (sm_idx, (mesh, &bounds)) in self.static_meshes.iter().zip(cache.static_meshes.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, mesh, closest_dist) {
                    closest_dist = d;
                    hit_pick = Some(PickSource::StaticMesh(sm_idx, p_idx));
                }
            }
        }

        let pick = hit_pick?;
        let (brush, p_idx, hit_source) = match pick {
            PickSource::Brush(b_idx, p_idx) => {
                let b = &self.world_brushes[b_idx];
                (b, p_idx, format!("Brush #{} [{}], Poly #{}", b_idx, b.name, p_idx))
            }
            PickSource::Mover(m_idx, p_idx) => {
                let m = &self.movers[m_idx];
                (&m.brush, p_idx, format!("Mover #{} [{}], Poly #{}", m_idx, m.name, p_idx))
            }
            PickSource::StaticMesh(sm_idx, p_idx) => {
                let sm = &self.static_meshes[sm_idx];
                (sm, p_idx, format!("Static Mesh #{} [{}], Poly #{}", sm_idx, sm.name, p_idx))
            }
        };
        let poly = brush.polys.get(p_idx)?;
        let model = brush.model_matrix();
        let tex = poly.texture.clone();
        let hit_uv = (
            model.transform_point3(poly.base),
            model.transform_vector3(poly.texture_u),
            model.transform_vector3(poly.texture_v),
            poly.pan_u,
            poly.pan_v,
        );

        self.active_texture = Some(tex.clone());
        self.sampled_surface_uv = Some(hit_uv);
        self.log(format!("🎨 Eyedropper picked '{}' (+UV) from {}", tex, hit_source));
        Some(tex)
    }

    /// Applies both the active texture and last sampled UV vectors (`Ctrl+Alt+LMB` in UnrealEd 2)
    pub fn apply_active_texture_and_uv_at_ray(&mut self, ray: Ray) -> bool {
        let Some(ref active_tex) = self.active_texture.clone() else {
            self.log("⚠ No active texture selected to apply. Pick a surface first.");
            return false;
        };
        let Some((world_base, tex_u, tex_v, pan_u, pan_v)) = self.sampled_surface_uv else {
            return self.apply_active_texture_at_ray(ray);
        };

        let mut closest_dist = f32::MAX;
        let mut hit_brush_poly: Option<(usize, usize)> = None;

        self.ensure_world_brush_bounds();
        {
            let cache = self.scene_bounds_cache.borrow();
            for (b_idx, (brush, &bounds)) in self.world_brushes.iter().zip(cache.world_brushes.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, brush, closest_dist) {
                    closest_dist = d;
                    hit_brush_poly = Some((b_idx, p_idx));
                }
            }
        }

        if let Some((b_idx, p_idx)) = hit_brush_poly {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                let brush_loc = brush.location;
                if let Some(poly) = brush.polys.get_mut(p_idx) {
                    poly.texture = active_tex.clone();
                    poly.base = world_base - brush_loc;
                    poly.texture_u = tex_u;
                    poly.texture_v = tex_v;
                    poly.pan_u = pan_u;
                    poly.pan_v = pan_v;
                }
                let new_polys = brush.polys.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Apply Texture '{}' + Sampled UV to Brush #{} Poly #{}", active_tex, b_idx, p_idx),
                });
                self.geometry_revision = self.geometry_revision.wrapping_add(1);
                self.selected_brush_index = Some(b_idx);
                self.selected_surfaces.clear();
                self.selected_surfaces.insert((b_idx, p_idx));
                self.log(format!("🎨 Applied texture '{}' + UV alignment to Brush #{}, Face #{}", active_tex, b_idx, p_idx));
                return true;
            }
        }

        self.apply_active_texture_at_ray(ray)
    }

    /// Applies currently active texture to the polygon/brush intersected by ray or to all selected surfaces
    pub fn apply_active_texture_at_ray(&mut self, ray: Ray) -> bool {
        let Some(ref active_tex) = self.active_texture.clone() else {
            self.log("⚠ No active texture selected to apply. Choose a texture first.");
            return false;
        };

        // If surfaces are already selected in the viewport, apply batch to all selected surfaces
        if !self.selected_surfaces.is_empty() {
            let mut modifications = Vec::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    if let Some(p) = b.polys.get_mut(p_idx) {
                        if p.texture != *active_tex {
                            modifications.push((b_idx, p_idx, p.texture.clone(), active_tex.clone()));
                            p.texture = active_tex.clone();
                        }
                    }
                }
            }

            if !modifications.is_empty() {
                let count = modifications.len();
                self.history.push(Action::BatchSetSurfaceTextures { modifications });
                self.log(format!("🎨 Batch-applied material '{}' to {} selected surface(s)", active_tex, count));
                return true;
            } else {
                self.log(format!("ℹ Selected surface(s) already have material '{}'", active_tex));
                return true;
            }
        }

        enum HitTarget {
            Brush(usize, usize),
            Mover(usize, usize),
            StaticMesh(usize, usize),
        }

        let mut closest_dist = f32::MAX;
        let mut hit_target: Option<HitTarget> = None;

        self.ensure_world_brush_bounds();
        self.ensure_mover_bounds();
        self.ensure_static_mesh_bounds();
        {
            let cache = self.scene_bounds_cache.borrow();

            // 1. Raycast world brushes
            for (b_idx, (brush, &bounds)) in self.world_brushes.iter().zip(cache.world_brushes.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, brush, closest_dist) {
                    closest_dist = d;
                    hit_target = Some(HitTarget::Brush(b_idx, p_idx));
                }
            }

            // 2. Raycast movers
            for (m_idx, (mover, &bounds)) in self.movers.iter().zip(cache.movers.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, &mover.brush, closest_dist) {
                    closest_dist = d;
                    hit_target = Some(HitTarget::Mover(m_idx, p_idx));
                }
            }

            // 3. Raycast static meshes
            for (sm_idx, (mesh, &bounds)) in self.static_meshes.iter().zip(cache.static_meshes.iter()).enumerate() {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, p_idx, _)) = intersect_brush_triangles(ray, mesh, closest_dist) {
                    closest_dist = d;
                    hit_target = Some(HitTarget::StaticMesh(sm_idx, p_idx));
                }
            }
        }

        match hit_target {
            Some(HitTarget::Brush(b_idx, p_idx)) => {
                if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                    if let Some(poly) = brush.polys.get_mut(p_idx) {
                        let old_texture = poly.texture.clone();
                        poly.texture = active_tex.clone();
                        self.history.push(Action::SetSurfaceTexture {
                            brush_index: b_idx,
                            poly_index: p_idx,
                            old_texture,
                            new_texture: active_tex.clone(),
                        });
                        self.selected_brush_index = Some(b_idx);
                        self.selected_surfaces.clear();
                        self.selected_surfaces.insert((b_idx, p_idx));
                        self.log(format!("🎨 Applied material '{}' to Brush #{}, Face #{}", active_tex, b_idx, p_idx));
                        return true;
                    }
                }
            }
            Some(HitTarget::Mover(m_idx, p_idx)) => {
                if let Some(mover) = self.movers.get_mut(m_idx) {
                    if let Some(poly) = mover.brush.polys.get_mut(p_idx) {
                        let old_texture = poly.texture.clone();
                        poly.texture = active_tex.clone();
                        self.history.push(Action::SetMoverSurfaceTexture {
                            mover_index: m_idx,
                            poly_index: p_idx,
                            old_texture,
                            new_texture: active_tex.clone(),
                        });
                        self.selected_mover_index = Some(m_idx);
                        self.log(format!("🎨 Applied material '{}' to Mover #{}, Face #{}", active_tex, m_idx, p_idx));
                        return true;
                    }
                }
            }
            Some(HitTarget::StaticMesh(sm_idx, p_idx)) => {
                if let Some(mesh) = self.static_meshes.get_mut(sm_idx) {
                    if let Some(poly) = mesh.polys.get_mut(p_idx) {
                        let old_texture = poly.texture.clone();
                        poly.texture = active_tex.clone();
                        self.history.push(Action::SetStaticMeshSurfaceTexture {
                            mesh_index: sm_idx,
                            poly_index: p_idx,
                            old_texture,
                            new_texture: active_tex.clone(),
                        });
                        self.log(format!("🎨 Applied material '{}' to Static Mesh #{}, Face #{}", active_tex, sm_idx, p_idx));
                        return true;
                    }
                }
            }
            None => {}
        }
        false
    }

    pub fn raycast_world(&self, ray: Ray) -> Option<(Vec3, Vec3)> {
        let mut closest_dist = f32::MAX;
        let mut hit_info: Option<(Vec3, Vec3)> = None;

        self.ensure_world_brush_bounds();
        self.ensure_mover_bounds();
        self.ensure_static_mesh_bounds();
        {
            let cache = self.scene_bounds_cache.borrow();

            // 1. World Brushes
            for (brush, &bounds) in self.world_brushes.iter().zip(cache.world_brushes.iter()) {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, normal)) = intersect_brush_triangles(ray, brush, closest_dist) {
                    closest_dist = d;
                    hit_info = Some((ray.origin + ray.direction * d, normal));
                }
            }

            // 2. Movers
            for (mover, &bounds) in self.movers.iter().zip(cache.movers.iter()) {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, normal)) = intersect_brush_triangles(ray, &mover.brush, closest_dist) {
                    closest_dist = d;
                    hit_info = Some((ray.origin + ray.direction * d, normal));
                }
            }

            // 3. Static Meshes
            for (mesh, &bounds) in self.static_meshes.iter().zip(cache.static_meshes.iter()) {
                if !ray_hits_aabb_within(ray, bounds, closest_dist) {
                    continue;
                }
                if let Some((d, _, normal)) = intersect_brush_triangles(ray, mesh, closest_dist) {
                    closest_dist = d;
                    hit_info = Some((ray.origin + ray.direction * d, normal));
                }
            }
        }

        hit_info
    }

    /// Completes an active drag-and-drop operation when released over the 3D viewport canvas.
    pub fn complete_drag_drop(&mut self, _screen_pos: (f32, f32), ray: Ray) -> bool {
        let Some(payload) = self.active_drag_drop.take() else {
            return false;
        };

        let hit = self.raycast_world(ray);
        match payload {
            DragDropPayload::ActorClass(class_name) => {
                let spawn_loc = if let Some((hit_pt, hit_norm)) = hit {
                    if self.snap_to_grid && self.grid_step > 0.0 {
                        let gs = self.grid_step;
                        let mut pt = hit_pt;
                        pt.x = (pt.x / gs).round() * gs;
                        pt.y = (pt.y / gs).round() * gs;
                        pt.z = (pt.z / gs).round() * gs;
                        pt
                    } else if hit_norm.z > 0.5 {
                        hit_pt + Vec3::new(0.0, 0.0, 16.0)
                    } else {
                        hit_pt + hit_norm * 16.0
                    }
                } else {
                    ray.origin + ray.direction * 400.0
                };

                self.place_actor_of_class_at(&class_name, spawn_loc);
                self.toast_success(format!("Dropped '{}' into scene", class_name));
                true
            }
            DragDropPayload::Texture(tex_name) => {
                let prev_tex = self.active_texture.clone();
                self.active_texture = Some(tex_name.clone());
                let res = self.apply_active_texture_at_ray(ray);
                if !res {
                    self.active_texture = prev_tex;
                }
                res
            }
        }
    }

    /// Toggles view frustum culling on or off.
    pub fn toggle_frustum_culling(&mut self) {
        self.frustum_culling_enabled = !self.frustum_culling_enabled;
        let status = if self.frustum_culling_enabled { "ENABLED (Optimal)" } else { "DISABLED (Full Draw)" };
        self.log(format!("Dynamic View Frustum Culling: {}", status));
        self.toast_info(format!("Frustum Culling: {}", status));
    }

}

#[cfg(test)]
mod picking_broadphase_tests {
    use super::*;

    fn rich_picking_scene() -> EditorState {
        let mut state = EditorState::default();
        state.world_brushes.clear();

        // 1. Translated & rotated room brush
        let mut room = build_cube("Room0", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
        room.location = Vec3::new(0.0, 0.0, 256.0);
        for (i, poly) in room.polys.iter_mut().enumerate() {
            poly.texture = format!("WallTex_{i}");
            poly.pan_u = (i as i16) * 4;
            poly.pan_v = -(i as i16) * 2;
        }
        state.world_brushes.push(room);

        // 2. Rotated + negative-scale solid brush
        let mut scaled = build_cube("NegScaled", CsgOperation::Add, Vec3::new(128.0, 96.0, 64.0));
        scaled.location = Vec3::new(-220.0, 140.0, 96.0);
        scaled.rotation = Vec3::new(15.0, 35.0, -10.0);
        scaled.scale = Vec3::new(-1.0, 1.5, 2.0);
        for poly in &mut scaled.polys {
            poly.texture = "MetalNeg".into();
        }
        state.world_brushes.push(scaled);

        // 3. Flat zero-thickness XY Sheet brush
        let mut sheet = build_sheet("FlatSheet", CsgOperation::Add, 128.0, 128.0);
        sheet.location = Vec3::new(260.0, -180.0, 80.0);
        for poly in &mut sheet.polys {
            poly.texture = "GrateSheet".into();
        }
        state.world_brushes.push(sheet);

        // 4. Mover
        let mut mover_b = build_cube("LiftMover", CsgOperation::Add, Vec3::new(96.0, 96.0, 32.0));
        mover_b.location = Vec3::new(220.0, 220.0, 64.0);
        for poly in &mut mover_b.polys {
            poly.texture = "MoverMetal".into();
        }
        state.movers.push(EditorMover::new(mover_b));

        // 5. Static Mesh
        let mut sm = build_cube("CrateMesh", CsgOperation::Add, Vec3::new(64.0, 64.0, 64.0));
        sm.location = Vec3::new(-180.0, -220.0, 48.0);
        sm.rotation = Vec3::new(0.0, 25.0, 0.0);
        for poly in &mut sm.polys {
            poly.texture = "MeshWood".into();
        }
        state.static_meshes.push(sm);

        // 6. Actors
        let mut level = ut99_format::UnrLevel::default();
        let mut light = ut99_format::ActorRecord::default();
        light.name = "LightPick0".into();
        light.class_name = "Light".into();
        light.location = Vec3::new(0.0, 320.0, 180.0);
        level.actors.push(light);
        let mut spawn = ut99_format::ActorRecord::default();
        spawn.name = "SpawnPick0".into();
        spawn.class_name = "PlayerStart".into();
        spawn.location = Vec3::new(120.0, -320.0, 64.0);
        level.actors.push(spawn);
        state.loaded_unr = Some(level);

        state
    }

    fn brute_force_raycast_world(state: &EditorState, ray: Ray) -> Option<(Vec3, Vec3)> {
        let mut closest_dist = f32::MAX;
        let mut hit_info = None;
        for brush in state
            .world_brushes
            .iter()
            .chain(state.movers.iter().map(|m| &m.brush))
            .chain(state.static_meshes.iter())
        {
            for poly in &brush.to_world_polys() {
                if poly.vertices.len() >= 3 {
                    let v0 = poly.vertices[0];
                    for i in 1..poly.vertices.len() - 1 {
                        if let Some(d) = ray.intersect_triangle(v0, poly.vertices[i], poly.vertices[i + 1]) {
                            if d < closest_dist {
                                closest_dist = d;
                                hit_info = Some((ray.origin + ray.direction * d, poly.plane.normal));
                            }
                        }
                    }
                }
            }
        }
        hit_info
    }

    fn brute_force_nearest_vertex(state: &EditorState, ray: Ray, max_dist: f32) -> Option<Vec3> {
        let mut closest = None;
        let mut min_d = max_dist;
        for brush in &state.world_brushes {
            for poly in &brush.to_world_polys() {
                for &v in &poly.vertices {
                    let proj = (v - ray.origin).dot(ray.direction);
                    if proj > 0.0 {
                        let d = (ray.origin + ray.direction * proj).distance(v);
                        if d < min_d {
                            min_d = d;
                            closest = Some(v);
                        }
                    }
                }
            }
        }
        closest
    }

    #[derive(Debug, PartialEq)]
    enum OraclePick {
        None,
        Actor(usize),
        Mover(usize),
        StaticMesh(usize),
        BrushSurface(usize, usize),
    }

    fn brute_force_select(state: &EditorState, ray: Ray) -> OraclePick {
        let mut closest_dist = f32::MAX;
        let mut pick = OraclePick::None;
        if let Some(ref unr) = state.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                if !state.is_actor_visible(idx) || state.is_actor_locked(idx) {
                    continue;
                }
                let (center, rad) = match actor.class_name.as_str() {
                    "Light" => (actor.location, 36.0),
                    "PlayerStart" => (actor.location + Vec3::new(0.0, 0.0, 36.0), 38.0),
                    "PathNode" => (actor.location, 32.0),
                    _ => (actor.location, 36.0),
                };
                if let Some(d) = ray.intersect_sphere(center, rad) {
                    if d < closest_dist {
                        closest_dist = d;
                        pick = OraclePick::Actor(idx);
                    }
                }
            }
        }
        for (m_idx, mover) in state.movers.iter().enumerate() {
            if !state.is_mover_visible(m_idx) || state.is_mover_locked(m_idx) {
                continue;
            }
            for poly in &mover.brush.to_world_polys() {
                if poly.vertices.len() >= 3 {
                    let v0 = poly.vertices[0];
                    for i in 1..poly.vertices.len() - 1 {
                        if let Some(d) = ray.intersect_triangle(v0, poly.vertices[i], poly.vertices[i + 1]) {
                            if d < closest_dist {
                                closest_dist = d;
                                pick = OraclePick::Mover(m_idx);
                            }
                        }
                    }
                }
            }
        }
        for (s_idx, mesh) in state.static_meshes.iter().enumerate() {
            if !state.is_static_mesh_visible(s_idx) || state.is_static_mesh_locked(s_idx) {
                continue;
            }
            for poly in &mesh.to_world_polys() {
                if poly.vertices.len() >= 3 {
                    let v0 = poly.vertices[0];
                    for i in 1..poly.vertices.len() - 1 {
                        if let Some(d) = ray.intersect_triangle(v0, poly.vertices[i], poly.vertices[i + 1]) {
                            if d < closest_dist {
                                closest_dist = d;
                                pick = OraclePick::StaticMesh(s_idx);
                            }
                        }
                    }
                }
            }
        }
        for (b_idx, brush) in state.world_brushes.iter().enumerate() {
            if !state.is_brush_visible(b_idx) || state.is_brush_locked(b_idx) {
                continue;
            }
            for (p_idx, poly) in brush.to_world_polys().iter().enumerate() {
                if poly.vertices.len() >= 3 {
                    let v0 = poly.vertices[0];
                    for i in 1..poly.vertices.len() - 1 {
                        if let Some(d) = ray.intersect_triangle(v0, poly.vertices[i], poly.vertices[i + 1]) {
                            if d < closest_dist {
                                closest_dist = d;
                                pick = OraclePick::BrushSurface(b_idx, p_idx);
                            }
                        }
                    }
                }
            }
        }
        pick
    }

    #[test]
    fn broadphase_raycast_select_and_vertex_match_brute_force_oracle() {
        let mut state = rich_picking_scene();
        let sheet_min = state.world_brushes[2].world_bounds().0;

        let rays = [
            // Perspective -> NegScaled brush
            Ray::new(Vec3::new(-450.0, 140.0, 96.0), Vec3::new(230.0, 0.0, 0.0)),
            // Ortho Top -> Flat XY Sheet exact interior and boundary
            Ray::new(Vec3::new(260.0, -180.0, 300.0), Vec3::NEG_Z),
            Ray::new(Vec3::new(sheet_min.x, sheet_min.y + 4.0, 300.0), Vec3::NEG_Z),
            // Perspective -> Mover
            Ray::new(Vec3::new(220.0, 220.0, 300.0), Vec3::NEG_Z),
            // Perspective -> Static Mesh
            Ray::new(Vec3::new(-180.0, -400.0, 48.0), Vec3::Y),
            // Perspective -> Actor Light
            Ray::new(Vec3::new(0.0, 100.0, 180.0), Vec3::Y),
            // Origin strictly inside Room0 and NegScaled AABB
            Ray::new(Vec3::new(-220.0, 140.0, 96.0), Vec3::new(1.0, 0.4, -0.2)),
            // Misses completely (far outside Room0)
            Ray::new(Vec3::new(2000.0, 2000.0, 2000.0), Vec3::X),
            // Vertex probe near Mover/NegScaled corner
            Ray::new(Vec3::new(-200.0, 200.0, 220.0), Vec3::NEG_Z),
        ];

        for ray in rays {
            let expected_rc = brute_force_raycast_world(&state, ray);
            let actual_rc = state.raycast_world(ray);
            match (expected_rc, actual_rc) {
                (None, None) => {}
                (Some((ep, en)), Some((ap, an))) => {
                    assert!(ep.distance(ap) < 1e-3, "pos mismatch: {ep} vs {ap}");
                    assert!(en.distance(an) < 1e-4, "norm mismatch: {en} vs {an}");
                }
                _ => panic!("raycast mismatch for {ray:?}: expected {expected_rc:?}, got {actual_rc:?}"),
            }

            let expected_vert = brute_force_nearest_vertex(&state, ray, 250.0);
            let actual_vert = state.find_nearest_world_vertex(ray, 250.0);
            assert_eq!(actual_vert, expected_vert, "vertex mismatch for {ray:?}");

            let oracle = brute_force_select(&state, ray);
            state.deselect_all();
            let hit = state.select_at_ray_with_modifier(ray, false);
            match oracle {
                OraclePick::None => assert!(!hit),
                OraclePick::Actor(i) => {
                    assert!(hit);
                    assert_eq!(state.selected_actor_index, Some(i));
                }
                OraclePick::Mover(i) => {
                    assert!(hit);
                    assert_eq!(state.selected_mover_index, Some(i));
                }
                OraclePick::StaticMesh(i) => {
                    assert!(hit);
                    assert_eq!(state.selected_static_mesh_index, Some(i));
                }
                OraclePick::BrushSurface(b, p) => {
                    assert!(hit);
                    assert_eq!(state.selected_brush_index, Some(b));
                    assert!(state.selected_surfaces.contains(&(b, p)));
                }
            }
        }
    }

    #[test]
    fn broadphase_eyedropper_texture_apply_and_context_menu_work() {
        let mut state = rich_picking_scene();
        let sheet_ray = Ray::new(Vec3::new(260.0, -180.0, 300.0), Vec3::NEG_Z);
        let picked = state.pick_texture_at_ray(sheet_ray);
        assert_eq!(picked.as_deref(), Some("GrateSheet"));
        assert!(state.sampled_surface_uv.is_some());

        // Apply active texture + UV onto NegScaled brush after clearing selected surfaces
        state.selected_surfaces.clear();
        state.active_texture = Some("CustomApplied".into());
        let neg_ray = Ray::new(Vec3::new(-450.0, 140.0, 96.0), Vec3::X);
        assert!(state.apply_active_texture_and_uv_at_ray(neg_ray));
        let (b_idx, p_idx) = *state.selected_surfaces.iter().next().unwrap();
        assert_eq!(b_idx, 1);
        assert_eq!(state.world_brushes[1].polys[p_idx].texture, "CustomApplied");

        // Apply active texture to Mover
        state.selected_surfaces.clear();
        state.active_texture = Some("MoverApplied".into());
        let mover_ray = Ray::new(Vec3::new(220.0, 220.0, 300.0), Vec3::NEG_Z);
        assert!(state.apply_active_texture_at_ray(mover_ray));
        assert_eq!(state.selected_mover_index, Some(0));
        assert!(state.movers[0].brush.polys.iter().any(|p| p.texture == "MoverApplied"));

        // Context menu preserves active multi-selection when right-clicking selected brush
        state.deselect_all();
        state.multi_selected_brushes.insert(1);
        state.multi_selected_brushes.insert(2);
        state.select_at_ray_for_context_menu(neg_ray);
        assert_eq!(state.multi_selected_brushes.len(), 2);
        assert!(state.multi_selected_brushes.contains(&1));
    }

    #[test]
    fn scene_bounds_cache_invalidates_on_mutations_and_new_map() {
        let mut state = rich_picking_scene();
        state.ensure_world_brush_bounds();
        state.ensure_mover_bounds();
        state.ensure_static_mesh_bounds();
        let prev_sheet_bounds = state.scene_bounds_cache.borrow().world_brushes[2];

        // Move and rescale FlatSheet via TrackedVec::IndexMut
        state.world_brushes[2].location += Vec3::new(400.0, 0.0, 50.0);
        state.world_brushes[2].scale = Vec3::new(2.0, 2.0, 1.0);
        let moved_ray = Ray::new(Vec3::new(660.0, -180.0, 300.0), Vec3::NEG_Z);
        let (hit_pos, _) = state.raycast_world(moved_ray).expect("must hit moved sheet");
        assert!((hit_pos.z - 130.0).abs() < 1e-2);
        assert_ne!(state.scene_bounds_cache.borrow().world_brushes[2], prev_sheet_bounds);

        // Push and remove a brush
        let mut extra = build_cube("Extra", CsgOperation::Add, Vec3::splat(64.0));
        extra.location = Vec3::new(900.0, 900.0, 90.0);
        state.world_brushes.push(extra);
        let extra_ray = Ray::new(Vec3::new(900.0, 900.0, 300.0), Vec3::NEG_Z);
        assert!(state.raycast_world(extra_ray).is_some());
        state.world_brushes.pop();
        state.ensure_world_brush_bounds();
        assert_eq!(state.scene_bounds_cache.borrow().world_brushes.len(), 3);

        // Mutate mover & static mesh
        state.movers[0].brush.location = Vec3::new(-400.0, 400.0, 80.0);
        assert!(state
            .raycast_world(Ray::new(Vec3::new(-400.0, 400.0, 300.0), Vec3::NEG_Z))
            .is_some());
        state.static_meshes[0].location = Vec3::new(400.0, -400.0, 80.0);
        assert!(state
            .raycast_world(Ray::new(Vec3::new(400.0, -400.0, 300.0), Vec3::NEG_Z))
            .is_some());

        // new_map resets caches cleanly
        state.new_map();
        assert!(state.scene_bounds_cache.borrow().world_brushes.is_empty());
        state.ensure_world_brush_bounds();
        assert_eq!(state.scene_bounds_cache.borrow().world_brushes.len(), 1);
    }
}
