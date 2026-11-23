use super::*;
use glam::Vec3;

enum SurfaceFlagTargets {
    World(std::collections::BTreeMap<usize, Vec<usize>>),
    Builder,
}

impl EditorState {
    // --- Surface Selection & Filtering Operations (UnrealEd 2.0 Parity) ---

    pub fn is_surface_selected(&self, brush_idx: usize, poly_idx: usize) -> bool {
        self.selected_surfaces.contains(&(brush_idx, poly_idx))
    }

    pub fn apply_active_texture_to_current_selection(&mut self) {
        if self.has_multi_selection() {
            let count = self.apply_active_texture_to_multi_selected();
            self.toast_success(format!("Applied texture to {} item(s)", count));
        } else if self.selected_brush_index.is_some() {
            self.apply_active_texture_to_selected_brush();
            self.toast_success("Applied texture to selected brush");
        } else {
            self.apply_active_texture_to_builder();
            self.toast_success("Applied texture to Red Builder Brush");
        }
    }

    pub fn select_surface(&mut self, brush_idx: usize, poly_idx: usize, multi_select: bool) {
        if multi_select {
            if self.selected_surfaces.contains(&(brush_idx, poly_idx)) {
                self.selected_surfaces.remove(&(brush_idx, poly_idx));
            } else {
                self.selected_surfaces.insert((brush_idx, poly_idx));
            }
        } else {
            self.selected_surfaces.clear();
            self.selected_surfaces.insert((brush_idx, poly_idx));
        }
        self.selected_brush_index = Some(brush_idx);
        let tex = self.world_brushes.get(brush_idx)
            .and_then(|b| b.polys.get(poly_idx))
            .map(|p| p.texture.as_str())
            .unwrap_or("None");
        self.log(format!("Selected Surface: Brush #{}, Face #{} (Texture: '{}')", brush_idx, poly_idx, tex));
    }

    pub fn deselect_surface(&mut self, brush_idx: usize, poly_idx: usize) {
        self.selected_surfaces.remove(&(brush_idx, poly_idx));
    }

    pub fn select_all_surfaces(&mut self) {
        self.selected_surfaces.clear();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            for p_idx in 0..brush.polys.len() {
                self.selected_surfaces.insert((b_idx, p_idx));
            }
        }
        self.log(format!("Selected all {} surface(s) across world brushes (Shift+S).", self.selected_surfaces.len()));
    }

    pub fn deselect_all_surfaces(&mut self) {
        let count = self.selected_surfaces.len();
        self.selected_surfaces.clear();
        self.log(format!("Deselected {} surface(s) (Shift+N).", count));
    }

    pub fn invert_surface_selection(&mut self) {
        let mut inverted = HashSet::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            for p_idx in 0..brush.polys.len() {
                if !self.selected_surfaces.contains(&(b_idx, p_idx)) {
                    inverted.insert((b_idx, p_idx));
                }
            }
        }
        self.selected_surfaces = inverted;
        self.log(format!("Inverted surface selection: {} surface(s) currently selected.", self.selected_surfaces.len()));
    }

    pub fn select_surfaces_on_selected_brushes(&mut self) {
        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }
        if brush_indices.is_empty() {
            self.log("Select Brush Surfaces: No world brush selected.");
            return;
        }

        let mut added = 0;
        for b_idx in brush_indices {
            if let Some(brush) = self.world_brushes.get(b_idx) {
                for p_idx in 0..brush.polys.len() {
                    if self.selected_surfaces.insert((b_idx, p_idx)) {
                        added += 1;
                    }
                }
            }
        }
        self.log(format!("Selected {} surface(s) from active brush(es) (total selected: {}).", added, self.selected_surfaces.len()));
    }

    pub fn select_surfaces_matching_texture(&mut self) {
        let mut target_textures = HashSet::new();

        if !self.selected_surfaces.is_empty() {
            for &(b_idx, p_idx) in &self.selected_surfaces {
                if let Some(tex) = self.world_brushes.get(b_idx).and_then(|b| b.polys.get(p_idx)).map(|p| &p.texture) {
                    if !tex.is_empty() {
                        target_textures.insert(tex.to_ascii_lowercase());
                    }
                }
            }
        }

        if target_textures.is_empty() {
            if let Some(ref t) = self.active_texture {
                target_textures.insert(t.to_ascii_lowercase());
            } else if let Some(b_idx) = self.selected_brush_index {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    for p in &b.polys {
                        if !p.texture.is_empty() {
                            target_textures.insert(p.texture.to_ascii_lowercase());
                        }
                    }
                }
            }
        }

        if target_textures.is_empty() {
            self.log("Select Matching Texture: No texture selected to match.");
            return;
        }

        let mut matched = 0;
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            for (p_idx, poly) in brush.polys.iter().enumerate() {
                if target_textures.contains(&poly.texture.to_ascii_lowercase()) {
                    if self.selected_surfaces.insert((b_idx, p_idx)) {
                        matched += 1;
                    }
                }
            }
        }
        self.log(format!("Selected {} additional matching surface(s) (total selected: {}, Shift+T).", matched, self.selected_surfaces.len()));
    }

    pub fn select_surfaces_coplanar_adjacent(&mut self) {
        let seeds: Vec<(usize, usize)> = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            if b_idx < self.world_brushes.len() && !self.world_brushes[b_idx].polys.is_empty() {
                vec![(b_idx, 0)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if seeds.is_empty() {
            self.log("Select Coplanar Adjacent: Select at least one surface or brush first.");
            return;
        }

        let mut all_polys: Vec<(usize, usize, ut99_bsp::Polygon)> = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let world_polys = brush.to_world_polys();
            for (p_idx, poly) in world_polys.into_iter().enumerate() {
                all_polys.push((b_idx, p_idx, poly));
            }
        }

        let mut queue = std::collections::VecDeque::new();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        for seed in seeds {
            queue.push_back(seed);
            visited.insert(seed);
            self.selected_surfaces.insert(seed);
        }

        while let Some(curr) = queue.pop_front() {
            let curr_poly = all_polys.iter().find(|(b, p, _)| *b == curr.0 && *p == curr.1).map(|(_, _, pl)| pl);
            if let Some(cp) = curr_poly {
                for (cand_b, cand_p, cand_poly) in &all_polys {
                    let cand_key = (*cand_b, *cand_p);
                    if !visited.contains(&cand_key) {
                        if polygons_are_coplanar(cp, cand_poly) && polygons_share_edge(cp, cand_poly, 1.0) {
                            visited.insert(cand_key);
                            queue.push_back(cand_key);
                            self.selected_surfaces.insert(cand_key);
                        }
                    }
                }
            }
        }

        self.log(format!("Selected {} adjacent coplanar surface(s) (Shift+B).", self.selected_surfaces.len()));
    }

    pub fn select_surfaces_coplanar_all(&mut self) {
        let seeds: Vec<(usize, usize)> = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            if b_idx < self.world_brushes.len() && !self.world_brushes[b_idx].polys.is_empty() {
                vec![(b_idx, 0)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if seeds.is_empty() {
            self.log("Select All Coplanar: Select at least one surface or brush first.");
            return;
        }

        let mut all_polys: Vec<(usize, usize, ut99_bsp::Polygon)> = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let world_polys = brush.to_world_polys();
            for (p_idx, poly) in world_polys.into_iter().enumerate() {
                all_polys.push((b_idx, p_idx, poly));
            }
        }

        let seed_polys: Vec<ut99_bsp::Polygon> = seeds.iter().filter_map(|s| {
            all_polys.iter().find(|(b, p, _)| *b == s.0 && *p == s.1).map(|(_, _, pl)| pl.clone())
        }).collect();

        for (cand_b, cand_p, cand_poly) in &all_polys {
            let cand_key = (*cand_b, *cand_p);
            for sp in &seed_polys {
                if polygons_are_coplanar(sp, cand_poly) {
                    self.selected_surfaces.insert(cand_key);
                    break;
                }
            }
        }

        self.log(format!("Selected {} coplanar surface(s) across map.", self.selected_surfaces.len()));
    }

    pub fn select_surfaces_adjacent_walls(&mut self) {
        let seeds: Vec<(usize, usize)> = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            if b_idx < self.world_brushes.len() && !self.world_brushes[b_idx].polys.is_empty() {
                vec![(b_idx, 0)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if seeds.is_empty() {
            self.log("Select Adjacent Walls: Select at least one surface or brush first.");
            return;
        }

        let mut all_polys: Vec<(usize, usize, ut99_bsp::Polygon)> = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let world_polys = brush.to_world_polys();
            for (p_idx, poly) in world_polys.into_iter().enumerate() {
                all_polys.push((b_idx, p_idx, poly));
            }
        }

        let mut queue = std::collections::VecDeque::new();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        for seed in seeds {
            queue.push_back(seed);
            visited.insert(seed);
            self.selected_surfaces.insert(seed);
        }

        while let Some(curr) = queue.pop_front() {
            let curr_poly = all_polys.iter().find(|(b, p, _)| *b == curr.0 && *p == curr.1).map(|(_, _, pl)| pl);
            if let Some(cp) = curr_poly {
                for (cand_b, cand_p, cand_poly) in &all_polys {
                    let cand_key = (*cand_b, *cand_p);
                    if !visited.contains(&cand_key) {
                        if polygon_is_wall(cand_poly) && polygons_share_edge(cp, cand_poly, 1.0) {
                            visited.insert(cand_key);
                            queue.push_back(cand_key);
                            self.selected_surfaces.insert(cand_key);
                        }
                    }
                }
            }
        }

        self.log(format!("Selected {} adjacent wall surface(s) (Shift+W).", self.selected_surfaces.len()));
    }

    pub fn select_surfaces_adjacent_slants(&mut self) {
        let seeds: Vec<(usize, usize)> = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            if b_idx < self.world_brushes.len() && !self.world_brushes[b_idx].polys.is_empty() {
                vec![(b_idx, 0)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if seeds.is_empty() {
            self.log("Select Adjacent Slants: Select at least one surface or brush first.");
            return;
        }

        let mut all_polys: Vec<(usize, usize, ut99_bsp::Polygon)> = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let world_polys = brush.to_world_polys();
            for (p_idx, poly) in world_polys.into_iter().enumerate() {
                all_polys.push((b_idx, p_idx, poly));
            }
        }

        let mut queue = std::collections::VecDeque::new();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        for seed in seeds {
            queue.push_back(seed);
            visited.insert(seed);
            self.selected_surfaces.insert(seed);
        }

        while let Some(curr) = queue.pop_front() {
            let curr_poly = all_polys.iter().find(|(b, p, _)| *b == curr.0 && *p == curr.1).map(|(_, _, pl)| pl);
            if let Some(cp) = curr_poly {
                for (cand_b, cand_p, cand_poly) in &all_polys {
                    let cand_key = (*cand_b, *cand_p);
                    if !visited.contains(&cand_key) {
                        if (polygon_is_slant(cand_poly) || cand_poly.plane.normal.z.abs() >= 0.7) && polygons_share_edge(cp, cand_poly, 1.0) {
                            visited.insert(cand_key);
                            queue.push_back(cand_key);
                            self.selected_surfaces.insert(cand_key);
                        }
                    }
                }
            }
        }

        self.log(format!("Selected {} adjacent slant/floor surface(s) (Shift+Q).", self.selected_surfaces.len()));
    }

    /// Selects adjacent floor/ceiling surfaces (`|normal.z| >= 0.7`, UnrealEd 2.0 `Shift+F`)
    pub fn select_surfaces_adjacent_floors(&mut self) {
        let seeds: Vec<(usize, usize)> = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            if b_idx < self.world_brushes.len() && !self.world_brushes[b_idx].polys.is_empty() {
                vec![(b_idx, 0)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if seeds.is_empty() {
            self.log("Select Adjacent Floors/Ceilings: Select at least one surface or brush first.");
            return;
        }

        let mut all_polys: Vec<(usize, usize, ut99_bsp::Polygon)> = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let world_polys = brush.to_world_polys();
            for (p_idx, poly) in world_polys.into_iter().enumerate() {
                all_polys.push((b_idx, p_idx, poly));
            }
        }

        let mut queue = std::collections::VecDeque::new();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        for seed in seeds {
            queue.push_back(seed);
            visited.insert(seed);
            self.selected_surfaces.insert(seed);
        }

        while let Some(curr) = queue.pop_front() {
            let curr_poly = all_polys
                .iter()
                .find(|(b, p, _)| *b == curr.0 && *p == curr.1)
                .map(|(_, _, pl)| pl);
            if let Some(cp) = curr_poly {
                for (cand_b, cand_p, cand_poly) in &all_polys {
                    let cand_key = (*cand_b, *cand_p);
                    if !visited.contains(&cand_key) {
                        if cand_poly.plane.normal.z.abs() >= 0.7
                            && polygons_share_edge(cp, cand_poly, 1.0)
                        {
                            visited.insert(cand_key);
                            queue.push_back(cand_key);
                            self.selected_surfaces.insert(cand_key);
                        }
                    }
                }
            }
        }

        self.log(format!(
            "Selected {} adjacent floor/ceiling surface(s) (Shift+F).",
            self.selected_surfaces.len()
        ));
    }

    /// Memorizes the current surface selection into the selection memory buffer (UnrealEd 2.0 `Shift+M`)
    pub fn memorize_surface_selection(&mut self) {
        self.memorized_surfaces = self.selected_surfaces.clone();
        self.log(format!(
            "Memorized {} selected surface(s) into memory (Shift+M).",
            self.memorized_surfaces.len()
        ));
    }

    /// Recalls the memorized surface selection, replacing the current selection (UnrealEd 2.0 `Shift+R`)
    pub fn recall_surface_selection(&mut self) {
        self.selected_surfaces = self.memorized_surfaces.clone();
        self.log(format!(
            "Recalled {} surface(s) from memory (Shift+R).",
            self.selected_surfaces.len()
        ));
    }

    /// Combines current and memorized surface selections via logical OR / Union (`A ∪ M`, UnrealEd 2.0 `Shift+O`)
    pub fn or_surface_selection(&mut self) {
        for &surf in &self.memorized_surfaces {
            self.selected_surfaces.insert(surf);
        }
        self.log(format!(
            "OR surface selection with memory: {} surface(s) selected (Shift+O).",
            self.selected_surfaces.len()
        ));
    }

    /// Intersects current and memorized surface selections via logical AND (`A ∩ M`, UnrealEd 2.0 `Shift+U`)
    pub fn and_surface_selection(&mut self) {
        self.selected_surfaces
            .retain(|surf| self.memorized_surfaces.contains(surf));
        self.log(format!(
            "AND surface selection with memory: {} surface(s) selected (Shift+U).",
            self.selected_surfaces.len()
        ));
    }

    /// Combines current and memorized surface selections via logical XOR (`A Δ M`, UnrealEd 2.0 `Shift+X`)
    pub fn xor_surface_selection(&mut self) {
        for &surf in &self.memorized_surfaces {
            if !self.selected_surfaces.remove(&surf) {
                self.selected_surfaces.insert(surf);
            }
        }
        self.log(format!(
            "XOR surface selection with memory: {} surface(s) selected (Shift+X).",
            self.selected_surfaces.len()
        ));
    }

    /// Selects all surfaces belonging to brushes in the same Group(s) as the current surface/brush selection (UnrealEd 2.0 `Shift+G`)
    pub fn select_surfaces_matching_groups(&mut self) {
        let mut target_groups: HashSet<String> = HashSet::new();
        if !self.selected_surfaces.is_empty() {
            for &(b_idx, _) in &self.selected_surfaces {
                let grp = self.brush_groups.get(&b_idx).cloned().unwrap_or_default();
                target_groups.insert(grp);
            }
        } else if let Some(b_idx) = self.selected_brush_index {
            let grp = self.brush_groups.get(&b_idx).cloned().unwrap_or_default();
            target_groups.insert(grp);
        }

        if target_groups.is_empty() {
            self.log("Select Matching Groups (Shift+G): Select a surface or brush first.");
            return;
        }

        let mut added = 0;
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let grp = self.brush_groups.get(&b_idx).cloned().unwrap_or_default();
            if target_groups.contains(&grp) {
                for p_idx in 0..brush.polys.len() {
                    if self.selected_surfaces.insert((b_idx, p_idx)) {
                        added += 1;
                    }
                }
            }
        }
        self.log(format!(
            "Selected {} additional surface(s) in matching group(s) (total: {}, Shift+G).",
            added,
            self.selected_surfaces.len()
        ));
    }

    /// Selects all surfaces matching the CSG operation & PolyFlags item archetype of the current selection (UnrealEd 2.0 `Shift+I`)
    pub fn select_surfaces_matching_items(&mut self) {
        let mut target_items: Vec<(ut99_bsp::CsgOperation, u32)> = Vec::new();
        if !self.selected_surfaces.is_empty() {
            for &(b_idx, p_idx) in &self.selected_surfaces {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    if let Some(p) = b.polys.get(p_idx) {
                        let pair = (b.csg_oper, p.flags);
                        if !target_items.contains(&pair) {
                            target_items.push(pair);
                        }
                    }
                }
            }
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                for p in &b.polys {
                    let pair = (b.csg_oper, p.flags);
                    if !target_items.contains(&pair) {
                        target_items.push(pair);
                    }
                }
            }
        }

        if target_items.is_empty() {
            self.log("Select Matching Items (Shift+I): Select a surface or brush first.");
            return;
        }

        let mut added = 0;
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            for (p_idx, poly) in brush.polys.iter().enumerate() {
                if target_items.contains(&(brush.csg_oper, poly.flags)) {
                    if self.selected_surfaces.insert((b_idx, p_idx)) {
                        added += 1;
                    }
                }
            }
        }
        self.log(format!(
            "Selected {} additional surface(s) matching item/flags (total: {}, Shift+I).",
            added,
            self.selected_surfaces.len()
        ));
    }

    /// Selects all adjacent edge-sharing surfaces regardless of normal orientation (UnrealEd 2.0 `Shift+J`)
    pub fn select_surfaces_adjacent_all(&mut self) {
        let seeds: Vec<(usize, usize)> = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            if b_idx < self.world_brushes.len() && !self.world_brushes[b_idx].polys.is_empty() {
                vec![(b_idx, 0)]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if seeds.is_empty() {
            self.log("Select All Adjacent Surfaces (Shift+J): Select at least one surface or brush first.");
            return;
        }

        let mut all_polys: Vec<(usize, usize, ut99_bsp::Polygon)> = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let world_polys = brush.to_world_polys();
            for (p_idx, poly) in world_polys.into_iter().enumerate() {
                all_polys.push((b_idx, p_idx, poly));
            }
        }

        let mut queue = std::collections::VecDeque::new();
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        for seed in seeds {
            queue.push_back(seed);
            visited.insert(seed);
            self.selected_surfaces.insert(seed);
        }

        while let Some(curr) = queue.pop_front() {
            let curr_poly = all_polys
                .iter()
                .find(|(b, p, _)| *b == curr.0 && *p == curr.1)
                .map(|(_, _, pl)| pl);
            if let Some(cp) = curr_poly {
                for (cand_b, cand_p, cand_poly) in &all_polys {
                    let cand_key = (*cand_b, *cand_p);
                    if !visited.contains(&cand_key) && polygons_share_edge(cp, cand_poly, 1.0) {
                        visited.insert(cand_key);
                        queue.push_back(cand_key);
                        self.selected_surfaces.insert(cand_key);
                    }
                }
            }
        }

        self.log(format!(
            "Selected {} adjacent surface(s) (Shift+J).",
            self.selected_surfaces.len()
        ));
    }

    // --- Texture Find & Replace (UnrealEd 2.0 Parity & Supremacy) ---

    pub fn get_selected_brush_indices(&self) -> Vec<usize> {
        let mut set = self.multi_selected_brushes.clone();
        if let Some(idx) = self.selected_brush_index {
            set.insert(idx);
        }
        let mut list: Vec<usize> = set.into_iter().collect();
        list.sort_unstable();
        list
    }

    pub fn preview_replace_textures_count(&self, find: &str, scope: TextureReplaceScope, mode: TextureMatchMode) -> usize {
        let find_trimmed = find.trim();
        if find_trimmed.is_empty() {
            return 0;
        }

        let mut count = 0;
        match scope {
            TextureReplaceScope::EntireLevel => {
                for brush in &self.world_brushes {
                    for poly in &brush.polys {
                        if matches_texture(&poly.texture, find_trimmed, mode) {
                            count += 1;
                        }
                    }
                }
            }
            TextureReplaceScope::SelectedBrushes => {
                let brush_indices = self.get_selected_brush_indices();
                for &b_idx in &brush_indices {
                    if let Some(brush) = self.world_brushes.get(b_idx) {
                        for poly in &brush.polys {
                            if matches_texture(&poly.texture, find_trimmed, mode) {
                                count += 1;
                            }
                        }
                    }
                }
            }
            TextureReplaceScope::SelectedSurfaces => {
                for &(b_idx, p_idx) in &self.selected_surfaces {
                    if let Some(brush) = self.world_brushes.get(b_idx) {
                        if let Some(poly) = brush.polys.get(p_idx) {
                            if matches_texture(&poly.texture, find_trimmed, mode) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    pub fn replace_textures(
        &mut self,
        find: &str,
        replace_with: &str,
        scope: TextureReplaceScope,
        mode: TextureMatchMode,
    ) -> usize {
        let find_trimmed = find.trim();
        let replace_trimmed = replace_with.trim();
        if find_trimmed.is_empty() || replace_trimmed.is_empty() {
            return 0;
        }

        let mut replaced_count = 0;
        let mut modified_brushes: Vec<(usize, Vec<ut99_bsp::Polygon>, Vec<ut99_bsp::Polygon>)> = Vec::new();

        match scope {
            TextureReplaceScope::EntireLevel => {
                for (b_idx, brush) in self.world_brushes.iter_mut().enumerate() {
                    let old_polys = brush.polys.clone();
                    let mut brush_changed = false;
                    for poly in &mut brush.polys {
                        if matches_texture(&poly.texture, find_trimmed, mode) {
                            let new_tex = compute_replaced_texture(&poly.texture, find_trimmed, replace_trimmed, mode);
                            if poly.texture != new_tex {
                                poly.texture = new_tex;
                                brush_changed = true;
                                replaced_count += 1;
                            }
                        }
                    }
                    if brush_changed {
                        modified_brushes.push((b_idx, old_polys, brush.polys.clone()));
                    }
                }
            }
            TextureReplaceScope::SelectedBrushes => {
                let brush_indices = self.get_selected_brush_indices();
                for b_idx in brush_indices {
                    if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                        let old_polys = brush.polys.clone();
                        let mut brush_changed = false;
                        for poly in &mut brush.polys {
                            if matches_texture(&poly.texture, find_trimmed, mode) {
                                let new_tex = compute_replaced_texture(&poly.texture, find_trimmed, replace_trimmed, mode);
                                if poly.texture != new_tex {
                                    poly.texture = new_tex;
                                    brush_changed = true;
                                    replaced_count += 1;
                                }
                            }
                        }
                        if brush_changed {
                            modified_brushes.push((b_idx, old_polys, brush.polys.clone()));
                        }
                    }
                }
            }
            TextureReplaceScope::SelectedSurfaces => {
                let mut brush_surfs: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
                for &(b_idx, p_idx) in &self.selected_surfaces {
                    brush_surfs.entry(b_idx).or_default().push(p_idx);
                }
                let mut sorted_entries: Vec<(usize, Vec<usize>)> = brush_surfs.into_iter().collect();
                sorted_entries.sort_by_key(|&(b_idx, _)| b_idx);

                for (b_idx, poly_indices) in sorted_entries {
                    if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                        let old_polys = brush.polys.clone();
                        let mut brush_changed = false;
                        for p_idx in poly_indices {
                            if let Some(poly) = brush.polys.get_mut(p_idx) {
                                if matches_texture(&poly.texture, find_trimmed, mode) {
                                    let new_tex = compute_replaced_texture(&poly.texture, find_trimmed, replace_trimmed, mode);
                                    if poly.texture != new_tex {
                                        poly.texture = new_tex;
                                        brush_changed = true;
                                        replaced_count += 1;
                                    }
                                }
                            }
                        }
                        if brush_changed {
                            modified_brushes.push((b_idx, old_polys, brush.polys.clone()));
                        }
                    }
                }
            }
        }

        if !modified_brushes.is_empty() {
            self.map_is_dirty = true;
            for (b_idx, old_polys, new_polys) in modified_brushes {
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Replace texture '{}' -> '{}'", find_trimmed, replace_trimmed),
                });
            }
            let msg = format!(
                "Replaced {} surface(s) with '{}' (Scope: {}, Mode: {}).",
                replaced_count,
                replace_trimmed,
                scope.label(),
                mode.label()
            );
            self.log(msg);
        } else {
            let msg = format!(
                "No surfaces matching '{}' found in {} (Mode: {}).",
                find_trimmed,
                scope.label(),
                mode.label()
            );
            self.log(msg);
        }

        replaced_count
    }
}

/// Maps a surface illuminance / luminance (0.0 .. 2.0+, where 1.0 ~ 100 Lux)
/// to a false-color luxmeter gradient for competitive tournament readability analysis.
/// Band mappings:
/// < 0.15 (<15 Lux): Deep Indigo/Blue (Pitch black / underexposed)
/// 0.15 .. 0.50 (15..50 Lux): Cyan/Teal (Low shadow)
/// 0.50 .. 1.00 (50..100 Lux): Vibrant Green (Optimal competitive readability)
/// 1.00 .. 1.80 (100..180 Lux): Amber Yellow (Direct fixture lighting)
/// >= 1.80 (>180 Lux): Scarlet Red / White (Overexposed / glare hazard)
pub fn lux_to_heatmap_color(luminance: f32) -> [f32; 4] {
    let l = luminance.max(0.0);
    if l < 0.15 {
        let t = (l / 0.15).clamp(0.0, 1.0);
        [0.05, 0.05 + 0.35 * t, 0.40 + 0.45 * t, 1.0]
    } else if l < 0.50 {
        let t = ((l - 0.15) / 0.35).clamp(0.0, 1.0);
        [0.10 * t, 0.40 + 0.45 * t, 0.85 * (1.0 - t) + 0.20 * t, 1.0]
    } else if l < 1.00 {
        let t = ((l - 0.50) / 0.50).clamp(0.0, 1.0);
        [0.10 + 0.85 * t, 0.85 + 0.03 * t, 0.20 * (1.0 - t) + 0.05 * t, 1.0]
    } else if l < 1.80 {
        let t = ((l - 1.00) / 0.80).clamp(0.0, 1.0);
        [0.95, 0.88 - 0.68 * t, 0.05, 1.0]
    } else {
        let t = ((l - 1.80) / 0.80).clamp(0.0, 1.0);
        [0.95 + 0.05 * t, 0.20 + 0.80 * t, 0.05 + 0.95 * t, 1.0]
    }
}

pub fn mirror_brush_geometry(brush: &mut Brush, axis: MirrorAxis) {
    for poly in &mut brush.polys {
        for v in &mut poly.vertices {
            match axis {
                MirrorAxis::X => v.x = -v.x,
                MirrorAxis::Y => v.y = -v.y,
                MirrorAxis::Z => v.z = -v.z,
            }
        }
        // Invert vertex winding to maintain outward-pointing surface normal
        poly.vertices.reverse();

        // Recompute normal and plane
        if poly.vertices.len() >= 3 {
            if let Some(plane) = ut99_bsp::Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                poly.plane = plane;
            }
        }

        // Mirror base and UV projection vectors
        match axis {
            MirrorAxis::X => {
                poly.base.x = -poly.base.x;
                poly.texture_u.x = -poly.texture_u.x;
                poly.texture_v.x = -poly.texture_v.x;
            }
            MirrorAxis::Y => {
                poly.base.y = -poly.base.y;
                poly.texture_u.y = -poly.texture_u.y;
                poly.texture_v.y = -poly.texture_v.y;
            }
            MirrorAxis::Z => {
                poly.base.z = -poly.base.z;
                poly.texture_u.z = -poly.texture_u.z;
                poly.texture_v.z = -poly.texture_v.z;
            }
        }
    }
}

pub fn invert_brush_geometry(brush: &mut Brush) {
    for poly in &mut brush.polys {
        poly.vertices.reverse();
        if poly.vertices.len() >= 3 {
            if let Some(plane) = ut99_bsp::Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                poly.plane = plane;
            } else {
                poly.plane.normal = -poly.plane.normal;
                poly.plane.dist = -poly.plane.dist;
            }
        } else {
            poly.plane.normal = -poly.plane.normal;
            poly.plane.dist = -poly.plane.dist;
        }
    }
}

pub fn polygons_share_edge(poly_a: &ut99_bsp::Polygon, poly_b: &ut99_bsp::Polygon, tol: f32) -> bool {
    let mut shared = 0;
    for va in &poly_a.vertices {
        for vb in &poly_b.vertices {
            if (va.x - vb.x).abs() <= tol && (va.y - vb.y).abs() <= tol && (va.z - vb.z).abs() <= tol {
                shared += 1;
                break;
            }
        }
    }
    shared >= 2
}

pub fn polygons_are_coplanar(poly_a: &ut99_bsp::Polygon, poly_b: &ut99_bsp::Polygon) -> bool {
    let dot = poly_a.plane.normal.dot(poly_b.plane.normal);
    if dot < 0.99 {
        return false;
    }
    (poly_a.plane.dist - poly_b.plane.dist).abs() < 1.0
}

pub fn polygon_is_wall(poly: &ut99_bsp::Polygon) -> bool {
    poly.plane.normal.z.abs() < 0.15
}

pub fn polygon_is_slant(poly: &ut99_bsp::Polygon) -> bool {
    let z = poly.plane.normal.z.abs();
    z >= 0.10 && z <= 0.95
}

pub fn scale_brush_geometry(brush: &mut Brush, scale: Vec3, adjust_textures: bool) {
    if scale.x.abs() < 1e-4 || scale.y.abs() < 1e-4 || scale.z.abs() < 1e-4 {
        return;
    }

    for poly in &mut brush.polys {
        for v in &mut poly.vertices {
            v.x *= scale.x;
            v.y *= scale.y;
            v.z *= scale.z;
        }

        // If negative determinant (e.g. 1 or 3 negative axes), winding order inverts
        let det = scale.x * scale.y * scale.z;
        if det < 0.0 {
            poly.vertices.reverse();
        }

        // Recompute normal and plane
        if poly.vertices.len() >= 3 {
            if let Some(plane) = ut99_bsp::Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                poly.plane = plane;
            }
        }

        // Scale texture origin base
        poly.base.x *= scale.x;
        poly.base.y *= scale.y;
        poly.base.z *= scale.z;

        // Optionally scale texture coordinate vectors
        if adjust_textures {
            poly.texture_u.x *= scale.x;
            poly.texture_u.y *= scale.y;
            poly.texture_u.z *= scale.z;

            poly.texture_v.x *= scale.x;
            poly.texture_v.y *= scale.y;
            poly.texture_v.z *= scale.z;
        }
    }
}

pub fn shear_brush_geometry(brush: &mut Brush, axis: ShearAxis, factor: f32, anchor_to_base: bool) {
    if factor.abs() < 1e-4 {
        return;
    }

    let (min_b, _) = brush.world_bounds();

    for poly in &mut brush.polys {
        for v in &mut poly.vertices {
            match axis {
                ShearAxis::XZ => {
                    let ref_val = if anchor_to_base { v.z - min_b.z } else { v.z };
                    v.x += factor * ref_val;
                }
                ShearAxis::YZ => {
                    let ref_val = if anchor_to_base { v.z - min_b.z } else { v.z };
                    v.y += factor * ref_val;
                }
                ShearAxis::ZX => {
                    let ref_val = if anchor_to_base { v.x - min_b.x } else { v.x };
                    v.z += factor * ref_val;
                }
                ShearAxis::ZY => {
                    let ref_val = if anchor_to_base { v.y - min_b.y } else { v.y };
                    v.z += factor * ref_val;
                }
                ShearAxis::XY => {
                    let ref_val = if anchor_to_base { v.y - min_b.y } else { v.y };
                    v.x += factor * ref_val;
                }
                ShearAxis::YX => {
                    let ref_val = if anchor_to_base { v.x - min_b.x } else { v.x };
                    v.y += factor * ref_val;
                }
            }
        }

        // Recompute normal and plane
        if poly.vertices.len() >= 3 {
            if let Some(plane) = ut99_bsp::Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                poly.plane = plane;
            }
        }
    }
}

impl EditorState {
    pub fn apply_active_texture_to_builder(&mut self) {
        if let Some(ref tex) = self.active_texture {
            for p in &mut self.builder_brush.polys {
                p.texture = tex.clone();
            }
            self.log(format!("Applied texture '{}' to Builder Brush.", tex));
        }
    }

    pub fn apply_surface_material_preset(&mut self, poly_flags: u32) {
        let count = self.selected_surfaces.len();
        if count == 0 {
            self.toast_warning("No surfaces selected. Select surface(s) with Shift+Click or F5.");
            return;
        }

        for &(b_idx, p_idx) in &self.selected_surfaces {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                if let Some(poly) = b.polys.get_mut(p_idx) {
                    poly.flags = poly_flags;
                }
            }
        }

        self.log(format!("Applied Surface Material Preset (flags: 0x{:08X}) to {} surface(s)", poly_flags, count));
        self.toast_success(format!("Updated {} surface(s)", count));
    }

    pub fn apply_active_texture_to_selected_brush(&mut self) {
        let Some(ref tex) = self.active_texture else { return; };
        let tex = tex.clone();

        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            p.texture = tex.clone();
                            count += 1;
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Apply texture '{}' to selected surfaces", tex),
                    });
                }
            }
            self.log(format!("Applied texture '{}' to {} selected surface(s).", tex, count));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    p.texture = tex.clone();
                }
                let new_polys = b.polys.clone();
                let name = b.name.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys,
                    description: format!("Apply texture '{}'", tex),
                });
                self.log(format!("Applied texture '{}' to Brush '{}'.", tex, name));
            }
        } else {
            for p in &mut self.builder_brush.polys {
                p.texture = tex.clone();
            }
            self.log(format!("Applied texture '{}' to Builder Brush.", tex));
        }
    }

    pub fn snapshot_selected_brush_polys(&self) -> Vec<(usize, Vec<ut99_bsp::Polygon>)> {
        let mut snapshots = Vec::new();
        if !self.selected_surfaces.is_empty() {
            let mut brushes_seen = std::collections::HashSet::new();
            for &(b_idx, _) in &self.selected_surfaces {
                if brushes_seen.insert(b_idx) {
                    if let Some(b) = self.world_brushes.get(b_idx) {
                        snapshots.push((b_idx, b.polys.clone()));
                    }
                }
            }
        } else if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(sel_idx) {
                snapshots.push((sel_idx, b.polys.clone()));
            }
        }
        snapshots
    }

    pub fn commit_uv_modification(&mut self, initial_snapshots: &[(usize, Vec<ut99_bsp::Polygon>)], description: String) {
        let mut count = 0;
        for (b_idx, old_polys) in initial_snapshots {
            if let Some(b) = self.world_brushes.get(*b_idx) {
                if &b.polys != old_polys {
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: *b_idx,
                        old_polys: old_polys.clone(),
                        new_polys: b.polys.clone(),
                        description: description.clone(),
                    });
                    count += 1;
                }
            }
        }
        if count > 0 {
            self.log(format!("{}: updated {} brush(es)", description, count));
        } else {
            self.log(description);
        }
    }

    pub fn pan_selected_brush_texture_raw(&mut self, u_delta: f32, v_delta: f32) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            let u_dir = if p.texture_u.length_squared() > 1e-4 { p.texture_u.normalize() } else { Vec3::X };
                            let v_dir = if p.texture_v.length_squared() > 1e-4 { p.texture_v.normalize() } else { Vec3::Y };
                            p.base += u_dir * u_delta + v_dir * v_delta;
                        }
                    }
                }
            }
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                for p in &mut b.polys {
                    let u_dir = if p.texture_u.length_squared() > 1e-4 {
                        p.texture_u.normalize()
                    } else {
                        Vec3::X
                    };
                    let v_dir = if p.texture_v.length_squared() > 1e-4 {
                        p.texture_v.normalize()
                    } else {
                        Vec3::Y
                    };
                    p.base += u_dir * u_delta + v_dir * v_delta;
                }
            }
        } else {
            for p in &mut self.builder_brush.polys {
                let u_dir = if p.texture_u.length_squared() > 1e-4 {
                    p.texture_u.normalize()
                } else {
                    Vec3::X
                };
                let v_dir = if p.texture_v.length_squared() > 1e-4 {
                    p.texture_v.normalize()
                } else {
                    Vec3::Y
                };
                p.base += u_dir * u_delta + v_dir * v_delta;
            }
        }
    }

    pub fn pan_selected_brush_texture(&mut self, u_delta: f32, v_delta: f32) {
        let snapshots = self.snapshot_selected_brush_polys();
        self.pan_selected_brush_texture_raw(u_delta, v_delta);
        if !snapshots.is_empty() {
            self.commit_uv_modification(&snapshots, format!("Pan Texture U {:+}, V {:+}", u_delta, v_delta));
        } else {
            self.log(format!("Panned texture on Builder Brush: U {:+}, V {:+}", u_delta, v_delta));
        }
    }

    pub fn scale_selected_brush_texture_raw(&mut self, u_factor: f32, v_factor: f32) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            p.texture_u *= u_factor;
                            p.texture_v *= v_factor;
                        }
                    }
                }
            }
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                for p in &mut b.polys {
                    p.texture_u *= u_factor;
                    p.texture_v *= v_factor;
                }
            }
        } else {
            for p in &mut self.builder_brush.polys {
                p.texture_u *= u_factor;
                p.texture_v *= v_factor;
            }
        }
    }

    pub fn scale_selected_brush_texture(&mut self, u_factor: f32, v_factor: f32) {
        let snapshots = self.snapshot_selected_brush_polys();
        self.scale_selected_brush_texture_raw(u_factor, v_factor);
        if !snapshots.is_empty() {
            self.commit_uv_modification(&snapshots, format!("Scale Texture U x{:.2}, V x{:.2}", u_factor, v_factor));
        } else {
            self.log(format!("Scaled texture on Builder Brush: U x{:.2}, V x{:.2}", u_factor, v_factor));
        }
    }

    pub fn align_selected_brush_texture(&mut self, mode: &str) {
        if mode == "cylinder" {
            self.align_selected_brush_cylinder(1.0, 1.0);
            return;
        }

        let align_mode = match mode {
            "floor" => ut99_bsp::AlignmentMode::Floor,
            "wall_x" => ut99_bsp::AlignmentMode::WallX,
            "wall_y" => ut99_bsp::AlignmentMode::WallY,
            "wall_auto" | "auto" | "wall_direction" => ut99_bsp::AlignmentMode::WallAuto,
            "planar" => ut99_bsp::AlignmentMode::Planar,
            "box" => ut99_bsp::AlignmentMode::Box,
            "fit" => ut99_bsp::AlignmentMode::Fit { repeats_u: 1.0, repeats_v: 1.0 },
            _ => ut99_bsp::AlignmentMode::Planar,
        };

        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            ut99_bsp::align_polygon(p, align_mode);
                            count += 1;
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Align Texture to {}", mode),
                    });
                }
            }
            self.log(format!("Aligned texture on {} surface(s) to {}", count, mode));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    ut99_bsp::align_polygon(p, align_mode);
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Align Texture to {}", mode),
                });
                self.log(format!("Aligned texture on Brush to {}", mode));
            }
        } else {
            for p in &mut self.builder_brush.polys {
                ut99_bsp::align_polygon(p, align_mode);
            }
            self.log(format!("Aligned texture on Builder Brush to {}", mode));
        }
    }

    pub fn align_selected_brush_cylinder(&mut self, repeats_u: f32, repeats_v: f32) {
        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                ut99_bsp::align_brush_cylindrical(b, repeats_u, repeats_v);
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Cylindrical UV Unwrap (U:{:.1}x, V:{:.1}x)", repeats_u, repeats_v),
                });
                self.log(format!("Cylindrical UV unwrapped on Brush #{} (U:{:.1}x, V:{:.1}x)", sel_idx, repeats_u, repeats_v));
            }
        } else {
            ut99_bsp::align_brush_cylindrical(&mut self.builder_brush, repeats_u, repeats_v);
            self.log(format!("Cylindrical UV unwrapped on Builder Brush (U:{:.1}x, V:{:.1}x)", repeats_u, repeats_v));
        }
    }

    pub fn fit_selected_brush_texture(&mut self, repeats_u: f32, repeats_v: f32) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            ut99_bsp::align_polygon(p, ut99_bsp::AlignmentMode::Fit { repeats_u, repeats_v });
                            count += 1;
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Auto-Fit Texture ({:.1}x{:.1})", repeats_u, repeats_v),
                    });
                }
            }
            self.log(format!("Auto-fitted texture on {} surface(s): {:.1}x{:.1}", count, repeats_u, repeats_v));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    ut99_bsp::align_polygon(p, ut99_bsp::AlignmentMode::Fit { repeats_u, repeats_v });
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Auto-Fit Texture ({:.1}x{:.1})", repeats_u, repeats_v),
                });
                self.log(format!("Auto-fitted texture on Brush #{}: {:.1}x{:.1}", sel_idx, repeats_u, repeats_v));
            }
        } else {
            for p in &mut self.builder_brush.polys {
                ut99_bsp::align_polygon(p, ut99_bsp::AlignmentMode::Fit { repeats_u, repeats_v });
            }
            self.log(format!("Auto-fitted texture on Builder Brush: {:.1}x{:.1}", repeats_u, repeats_v));
        }
    }

    pub fn align_selected_brush_continuous_strip(&mut self, repeats_u: f32, repeats_v: f32) {
        if !self.selected_surfaces.is_empty() {
            self.align_selected_surfaces_continuous(repeats_u, repeats_v);
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                ut99_bsp::align_brush_continuous_strip(b, repeats_u, repeats_v);
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Continuous Strip Alignment ({:.1}x{:.1})", repeats_u, repeats_v),
                });
                self.log(format!("Continuously aligned texture strip on Brush #{}: {:.1}x{:.1}", sel_idx, repeats_u, repeats_v));
            }
        } else {
            ut99_bsp::align_brush_continuous_strip(&mut self.builder_brush, repeats_u, repeats_v);
            self.log(format!("Continuously aligned texture strip on Builder Brush: {:.1}x{:.1}", repeats_u, repeats_v));
        }
    }

    pub fn align_selected_surfaces_continuous(&mut self, repeats_u: f32, repeats_v: f32) {
        if !self.selected_surfaces.is_empty() {
            let mut items: Vec<(usize, usize)> = self.selected_surfaces.iter().copied().collect();
            items.sort_unstable();

            let mut brushes_old: std::collections::HashMap<usize, Vec<ut99_bsp::Polygon>> = std::collections::HashMap::new();
            for &(b_idx, _) in &items {
                if !brushes_old.contains_key(&b_idx) {
                    if let Some(b) = self.world_brushes.get(b_idx) {
                        brushes_old.insert(b_idx, b.polys.clone());
                    }
                }
            }

            let mut accumulated_u = 0.0_f32;
            let mut count = 0;

            for &(b_idx, p_idx) in &items {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    if let Some(p) = b.polys.get_mut(p_idx) {
                        let norm = p.plane.normal;
                        let t = if norm.z.abs() < 0.85 {
                            let cross = norm.cross(Vec3::new(0.0, 0.0, 1.0));
                            if cross.length_squared() > 1e-6 {
                                cross.normalize()
                            } else {
                                Vec3::X
                            }
                        } else {
                            Vec3::X
                        };

                        let mut min_t = f32::MAX;
                        let mut max_t = f32::MIN;
                        let mut min_z = f32::MAX;
                        let mut max_z = f32::MIN;

                        for &v in &p.vertices {
                            let proj = v.dot(t);
                            min_t = min_t.min(proj);
                            max_t = max_t.max(proj);
                            min_z = min_z.min(v.z);
                            max_z = max_z.max(v.z);
                        }

                        let w = (max_t - min_t).max(1.0);
                        let h = (max_z - min_z).max(1.0);

                        let r_u = repeats_u.max(0.01);
                        let r_v = repeats_v.max(0.01);

                        let scale_u = 128.0 / r_u;
                        let scale_v = (h / (128.0 * r_v)).max(0.001);

                        p.texture_u = t * (1.0 / scale_u);
                        p.texture_v = Vec3::new(0.0, 0.0, -1.0) * (1.0 / scale_v);

                        let origin = p.vertices.first().copied().unwrap_or(Vec3::ZERO);
                        let proj_origin = origin.dot(t);
                        p.base = origin - t * (proj_origin - min_t + accumulated_u);

                        accumulated_u += w;
                        count += 1;
                    }
                }
            }

            for (b_idx, old_polys) in brushes_old {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys: b.polys.clone(),
                        description: format!("Continuous Surface Wrap (U:{:.1}x, V:{:.1}x)", repeats_u, repeats_v),
                    });
                }
            }
            self.map_is_dirty = true;
            self.log(format!("Continuously wrapped texture across {} selected surface(s).", count));
            return;
        }

        self.align_selected_brush_continuous_strip(repeats_u, repeats_v);
    }

    pub fn reset_selected_surfaces_pan(&mut self) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            if let Some(&first_v) = p.vertices.first() {
                                p.base = first_v;
                                count += 1;
                            }
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: "Reset Surface Pan to Origin".to_string(),
                    });
                }
            }
            self.log(format!("Reset texture pan to origin on {} surface(s).", count));
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    if let Some(&first_v) = p.vertices.first() {
                        p.base = first_v;
                    }
                }
                let new_polys = b.polys.clone();
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: "Reset Surface Pan to Origin".to_string(),
                });
                self.log(format!("Reset texture pan to origin on brush #{}.", b_idx));
            }
        }
    }

    /// Snaps the UV texture projection of selected surfaces to the specified pixel grid (e.g. 16, 32, 64 px).
    pub fn snap_selected_surfaces_uv_to_grid(&mut self, grid_pixels: f32) {
        if grid_pixels <= 0.0 { return; }
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            if let Some(&v0) = p.vertices.first() {
                                let u_dir = if p.texture_u.length_squared() > 1e-4 { p.texture_u.normalize() } else { Vec3::X };
                                let v_dir = if p.texture_v.length_squared() > 1e-4 { p.texture_v.normalize() } else { Vec3::Y };
                                let diff = p.base - v0;
                                let u_proj = diff.dot(u_dir);
                                let v_proj = diff.dot(v_dir);
                                let u_snapped = (u_proj / grid_pixels).round() * grid_pixels;
                                let v_snapped = (v_proj / grid_pixels).round() * grid_pixels;
                                p.base = v0 + u_dir * u_snapped + v_dir * v_snapped;
                                count += 1;
                            }
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Snap UV to {:.0}px Grid", grid_pixels),
                    });
                }
            }
            self.log(format!("Snapped UV to {:.0}px grid on {} surface(s).", grid_pixels, count));
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    if let Some(&v0) = p.vertices.first() {
                        let u_dir = if p.texture_u.length_squared() > 1e-4 { p.texture_u.normalize() } else { Vec3::X };
                        let v_dir = if p.texture_v.length_squared() > 1e-4 { p.texture_v.normalize() } else { Vec3::Y };
                        let diff = p.base - v0;
                        let u_proj = diff.dot(u_dir);
                        let v_proj = diff.dot(v_dir);
                        let u_snapped = (u_proj / grid_pixels).round() * grid_pixels;
                        let v_snapped = (v_proj / grid_pixels).round() * grid_pixels;
                        p.base = v0 + u_dir * u_snapped + v_dir * v_snapped;
                    }
                }
                let new_polys = b.polys.clone();
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Snap UV to {:.0}px Grid", grid_pixels),
                });
                self.log(format!("Snapped UV to {:.0}px grid on brush #{}.", grid_pixels, b_idx));
            }
        }
    }

    pub fn rotate_selected_brush_texture_raw(&mut self, angle_deg: f32) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            ut99_bsp::rotate_texture(p, angle_deg);
                        }
                    }
                }
            }
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                for p in &mut b.polys {
                    ut99_bsp::rotate_texture(p, angle_deg);
                }
            }
        } else {
            for p in &mut self.builder_brush.polys {
                ut99_bsp::rotate_texture(p, angle_deg);
            }
        }
    }

    pub fn rotate_selected_brush_texture(&mut self, angle_deg: f32) {
        let snapshots = self.snapshot_selected_brush_polys();
        self.rotate_selected_brush_texture_raw(angle_deg);
        if !snapshots.is_empty() {
            self.commit_uv_modification(&snapshots, format!("Rotate Texture by {:.1}°", angle_deg));
        } else {
            self.log(format!("Rotated texture on Builder Brush by {:.1}°", angle_deg));
        }
    }

    pub fn flip_selected_brush_texture_u(&mut self) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            p.texture_u = -p.texture_u;
                            count += 1;
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: "Flip Texture U".into(),
                    });
                }
            }
            self.log(format!("Flipped texture U on {} surface(s)", count));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    p.texture_u = -p.texture_u;
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: "Flip Texture U".into(),
                });
                self.log("Flipped texture U on Brush");
            }
        } else {
            for p in &mut self.builder_brush.polys {
                p.texture_u = -p.texture_u;
            }
            self.log("Flipped texture U on Builder Brush");
        }
    }

    pub fn flip_selected_brush_texture_v(&mut self) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            p.texture_v = -p.texture_v;
                            count += 1;
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(crate::history::Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: "Flip Texture V".into(),
                    });
                }
            }
            self.log(format!("Flipped texture V on {} surface(s)", count));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    p.texture_v = -p.texture_v;
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: "Flip Texture V".into(),
                });
                self.log("Flipped texture V on Brush");
            }
        } else {
            for p in &mut self.builder_brush.polys {
                p.texture_v = -p.texture_v;
            }
            self.log("Flipped texture V on Builder Brush");
        }
    }

    /// Computes a screen-space adaptive world scale for transform gizmos so they remain
    /// a crisp, consistent pixel size (~80px) regardless of camera distance or ortho zoom.
    pub fn toggle_selected_brush_poly_flag(&mut self, flag: u32) {
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            p.flags ^= flag;
                            count += 1;
                        }
                    }
                    let new_polys = b.polys.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Toggle PolyFlag 0x{:08X}", flag),
                    });
                }
            }
            self.log(format!("Toggled PolyFlag 0x{:08X} on {} surface(s)", flag, count));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    p.flags ^= flag;
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Toggle PolyFlag 0x{:08X}", flag),
                });
                self.log(format!("Toggled PolyFlag 0x{:08X} on Brush", flag));
            }
        } else {
            for p in &mut self.builder_brush.polys {
                p.flags ^= flag;
            }
            self.log(format!("Toggled PolyFlag 0x{:08X} on Builder Brush", flag));
        }
    }

    pub fn apply_surface_flags_preset(&mut self, preset: SurfaceFlagsPreset) {
        let flags = preset.flags();
        let desc = preset.label();

        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            let mut count = 0;
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            p.flags = flags;
                            count += 1;
                        }
                    }
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys: b.polys.clone(),
                        description: format!("Apply Surface Preset '{}'", desc),
                    });
                }
            }
            self.map_is_dirty = true;
            self.log(format!("Applied preset '{}' to {} surface(s).", desc, count));
            return;
        }

        if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    p.flags = flags;
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: sel_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Apply Surface Preset '{}'", desc),
                });
                self.map_is_dirty = true;
                self.log(format!("Applied preset '{}' to Brush #{}.", desc, sel_idx));
            }
        } else {
            for p in &mut self.builder_brush.polys {
                p.flags = flags;
            }
            self.log(format!("Applied preset '{}' to Builder Brush.", desc));
        }
    }


}


// ==========================================
// Free-standing Surface Texture Matching Utilities
// ==========================================

/// Tests whether a polygon's texture matches the search query and matching mode.
pub fn matches_texture(poly_tex: &str, find: &str, mode: TextureMatchMode) -> bool {
    let find_trimmed = find.trim();
    if find_trimmed.is_empty() {
        return false;
    }
    match mode {
        TextureMatchMode::ExactName => {
            if poly_tex.eq_ignore_ascii_case(find_trimmed) {
                return true;
            }
            // If find does not contain a package dot, also match the terminal texture name
            if !find_trimmed.contains('.') {
                if let Some(short_name) = poly_tex.rsplit('.').next() {
                    if short_name.eq_ignore_ascii_case(find_trimmed) {
                        return true;
                    }
                }
            }
            false
        }
        TextureMatchMode::PackageOnly => {
            let poly_pkg = poly_tex.split('.').next().unwrap_or(poly_tex);
            let find_pkg = find_trimmed.split('.').next().unwrap_or(find_trimmed);
            poly_pkg.eq_ignore_ascii_case(find_pkg)
        }
        TextureMatchMode::ContainsSubstring => {
            poly_tex.to_lowercase().contains(&find_trimmed.to_lowercase())
        }
    }
}


/// Computes the replacement texture name given original texture, find term, replace term, and mode.
pub fn compute_replaced_texture(
    poly_tex: &str,
    find: &str,
    replace_with: &str,
    mode: TextureMatchMode,
) -> String {
    let replace_trimmed = replace_with.trim();
    match mode {
        TextureMatchMode::ExactName => replace_trimmed.to_string(),
        TextureMatchMode::PackageOnly => {
            if !replace_trimmed.contains('.') {
                if let Some((_pkg, rest)) = poly_tex.split_once('.') {
                    format!("{}.{}", replace_trimmed, rest)
                } else {
                    replace_trimmed.to_string()
                }
            } else {
                replace_trimmed.to_string()
            }
        }
        TextureMatchMode::ContainsSubstring => {
            let lower_poly = poly_tex.to_lowercase();
            let lower_find = find.trim().to_lowercase();
            if !replace_trimmed.contains('.') && !find.trim().contains('.') {
                if let Some(idx) = lower_poly.find(&lower_find) {
                    let end = idx + lower_find.len();
                    format!("{}{}{}", &poly_tex[..idx], replace_trimmed, &poly_tex[end..])
                } else {
                    replace_trimmed.to_string()
                }
            } else {
                replace_trimmed.to_string()
            }
        }
    }
}



impl EditorState {
    /// Copies the texture name and full 3D UV alignment (`texture_u`, `texture_v`, and world-space `base`)
    /// from `(src_brush_idx, src_poly_idx)` onto all currently selected surfaces (`REQ-D02` — `Ctrl+Alt+LMB`).
    pub fn paste_texture_and_uv_from_surface_to_selected(
        &mut self,
        src_brush_idx: usize,
        src_poly_idx: usize,
    ) -> usize {
        let (src_tex, src_u, src_v, src_pan_u, src_pan_v, src_world_base) = match self
            .world_brushes
            .get(src_brush_idx)
            .and_then(|b| b.polys.get(src_poly_idx).map(|p| (b, p)))
        {
            Some((b, p)) => (
                p.texture.clone(),
                p.texture_u,
                p.texture_v,
                p.pan_u,
                p.pan_v,
                b.location + p.base,
            ),
            None => return 0,
        };

        let mut targets: Vec<(usize, usize)> = self.selected_surfaces.iter().copied().collect();
        if targets.is_empty() {
            if let Some(b_idx) = self.selected_brush_index {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    for p_idx in 0..b.polys.len() {
                        targets.push((b_idx, p_idx));
                    }
                }
            }
        }
        targets.retain(|&(b, p)| !(b == src_brush_idx && p == src_poly_idx));
        if targets.is_empty() {
            return 0;
        }

        let mut affected_brushes: Vec<usize> = targets.iter().map(|&(b, _)| b).collect();
        affected_brushes.sort_unstable();
        affected_brushes.dedup();

        let mut actions = Vec::new();
        let mut updated_polys = 0usize;

        for b_idx in affected_brushes {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                let brush_loc = brush.location;
                for &(tb, tp) in &targets {
                    if tb == b_idx {
                        if let Some(poly) = brush.polys.get_mut(tp) {
                            poly.texture = src_tex.clone();
                            poly.texture_u = src_u;
                            poly.texture_v = src_v;
                            poly.pan_u = src_pan_u;
                            poly.pan_v = src_pan_v;
                            poly.base = src_world_base - brush_loc;
                            updated_polys += 1;
                        }
                    }
                }
                let new_polys = brush.polys.clone();
                actions.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Paste Texture & UV Alignment to Brush {}", b_idx),
                });
            }
        }

        if !actions.is_empty() {
            self.history.push(Action::Compound {
                actions,
                description: format!("Propagate Texture & UV Alignment to {} surface(s)", updated_polys),
            });
            self.active_texture = Some(src_tex);
            self.map_is_dirty = true;
            self.log(format!(
                "Propagated texture and UV vectors from ({}, {}) to {} surface(s).",
                src_brush_idx, src_poly_idx, updated_polys
            ));
        }
        updated_polys
    }


    /// Propagates the texture and UV alignment of the lowest-indexed selected surface to all other selected surfaces (`REQ-D02`).
    pub fn propagate_first_selected_surface_uv_to_all(&mut self) -> usize {
        let mut sorted: Vec<(usize, usize)> = self.selected_surfaces.iter().copied().collect();
        sorted.sort_unstable();
        if let Some(&(src_b, src_p)) = sorted.first() {
            self.paste_texture_and_uv_from_surface_to_selected(src_b, src_p)
        } else {
            0
        }
    }


    /// UnrealEd 2.0 `Select Surfaces -> Matching Zone` (`POLY SELECT ZONE` / `Shift+Z` in ClassicUnrealEd2):
    /// Computes 3D BSP zone flood-fill reachability across all brush surfaces and selects every surface
    /// that shares the same 3D BSP zone as the currently selected surface(s), stopping at solid walls and `PF_PORTAL` sheets.
    pub fn select_surfaces_matching_zone(&mut self) -> usize {
        if self.selected_surfaces.is_empty() || self.world_brushes.is_empty() {
            return 0;
        }

        // Compute an interior air probe point for every brush surface `(b_idx, p_idx)`
        let mut surface_probes: Vec<((usize, usize), String, Vec3)> = Vec::new();
        for (b_idx, b) in self.world_brushes.iter().enumerate() {
            if (b.polys.first().map(|p| p.flags & ut99_bsp::PF_PORTAL).unwrap_or(0)) != 0 {
                continue;
            }
            let world_polys = b.to_world_polys();
            let (min_b, max_b) = b.world_bounds();
            let brush_center = (min_b + max_b) * 0.5;
            for (p_idx, wp) in world_polys.iter().enumerate() {
                if wp.vertices.is_empty() || (wp.flags & ut99_bsp::PF_PORTAL) != 0 {
                    continue;
                }
                let poly_centroid = wp.vertices.iter().copied().sum::<Vec3>() / (wp.vertices.len() as f32);
                let to_center = (brush_center - poly_centroid).normalize_or_zero();
                let probe = match b.csg_oper {
                    CsgOperation::Subtract => poly_centroid + to_center * 8.0,
                    _ => poly_centroid - to_center * 8.0,
                };
                let seed_name = format!("_SurfProbe_{}_{}", b_idx, p_idx);
                surface_probes.push(((b_idx, p_idx), seed_name, probe));
            }
        }

        let seeds: Vec<(String, Vec3)> = surface_probes
            .iter()
            .map(|(_, name, pt)| (name.clone(), *pt))
            .collect();
        let flood_res = ut99_bsp::compute_bsp_zone_flood_fill(&self.world_brushes, &seeds);

        let mut target_zones = std::collections::HashSet::new();
        for &(sel_b, sel_p) in &self.selected_surfaces {
            let key = format!("_SurfProbe_{}_{}", sel_b, sel_p);
            if let Some(&z_id) = flood_res.actor_zone_ids.get(&key) {
                target_zones.insert(z_id);
            }
        }

        if target_zones.is_empty() {
            return self.selected_surfaces.len();
        }

        for ((b_idx, p_idx), seed_name, _) in &surface_probes {
            if let Some(&z_id) = flood_res.actor_zone_ids.get(seed_name) {
                if target_zones.contains(&z_id) {
                    self.selected_surfaces.insert((*b_idx, *p_idx));
                }
            }
        }

        let total = self.selected_surfaces.len();
        self.log(format!(
            "POLY SELECT ZONE: selected {} surface(s) in matching BSP zone(s) {:?}.",
            total, target_zones
        ));
        total
    }


    // Preserve explicit scope even when all selected indices are obsolete: never
    // fall back to the builder or another brush in that case.
    fn surface_flag_targets(&self) -> SurfaceFlagTargets {
        let mut targets = std::collections::BTreeMap::<usize, Vec<usize>>::new();
        if !self.selected_surfaces.is_empty() {
            for &(brush, face) in &self.selected_surfaces {
                if self.world_brushes.get(brush).and_then(|b| b.polys.get(face)).is_some() {
                    targets.entry(brush).or_default().push(face);
                }
            }
        } else if let Some(brush) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(brush) {
                targets.insert(brush, (0..b.polys.len()).collect());
            }
        } else {
            return SurfaceFlagTargets::Builder;
        }
        for faces in targets.values_mut() { faces.sort_unstable(); }
        SurfaceFlagTargets::World(targets)
    }

    /// Intersection, union and valid face count for exactly the scope edited by POLY SET.
    pub fn selected_surface_flags_summary(&self) -> (u32, u32, usize) {
        let (mut all, mut any, mut count) = (u32::MAX, 0, 0);
        let mut include = |flags| { all &= flags; any |= flags; count += 1; };
        match self.surface_flag_targets() {
            SurfaceFlagTargets::World(targets) => {
                for (brush, faces) in targets {
                    for face in faces { include(self.world_brushes[brush].polys[face].flags); }
                }
            }
            SurfaceFlagTargets::Builder => {
                for poly in &self.builder_brush.polys { include(poly.flags); }
            }
        }
        if count == 0 { (0, 0, 0) } else { (all, any, count) }
    }

    /// Sets/clears bits (clear wins), returning the number of faces actually changed.
    /// One request is one undo operation, including requests spanning several brushes.
    pub fn set_selected_surfaces_flags(&mut self, set_flags: u32, clear_flags: u32) -> usize {
        let description = format!("POLY SET SETFLAGS=0x{:X} CLEARFLAGS=0x{:X}", set_flags, clear_flags);
        let mut updated = 0;
        let mut actions = Vec::new();
        match self.surface_flag_targets() {
            SurfaceFlagTargets::World(targets) => {
                for (index, faces) in targets {
                    let brush = &mut self.world_brushes[index];
                    if !faces.iter().any(|&face| {
                        let flags = brush.polys[face].flags;
                        (flags | set_flags) & !clear_flags != flags
                    }) { continue; }
                    let old_polys = brush.polys.clone();
                    for face in faces {
                        let flags = &mut brush.polys[face].flags;
                        let next = (*flags | set_flags) & !clear_flags;
                        if next != *flags { *flags = next; updated += 1; }
                    }
                    actions.push(Action::ModifyBrushGeometry {
                        brush_index: index, old_polys, new_polys: brush.polys.clone(),
                        description: description.clone(),
                    });
                }
                if !actions.is_empty() { self.map_is_dirty = true; }
            }
            SurfaceFlagTargets::Builder => {
                if self.builder_brush.polys.iter().any(|p| (p.flags | set_flags) & !clear_flags != p.flags) {
                    let old_brush = self.builder_brush.clone();
                    for poly in &mut self.builder_brush.polys {
                        let next = (poly.flags | set_flags) & !clear_flags;
                        if next != poly.flags { poly.flags = next; updated += 1; }
                    }
                    actions.push(Action::ModifyBuilderBrush {
                        old_brush, new_brush: self.builder_brush.clone(), description: description.clone(),
                    });
                }
            }
        }
        if actions.len() == 1 {
            if let Some(action) = actions.pop() { self.history.push(action); }
        } else if !actions.is_empty() {
            self.history.push(Action::Compound { actions, description });
        }
        updated
    }


    /// Resets the texture of all currently selected surfaces (or active brush) to `"DefaultTexture"` (`TEXTURE CLEAR`).
    pub fn clear_selected_surfaces_texture(&mut self) -> usize {
        let prev_active = self.active_texture.clone();
        self.active_texture = Some("DefaultTexture".to_string());
        let cnt = if !self.selected_surfaces.is_empty() {
            self.selected_surfaces.len()
        } else if let Some(b_idx) = self.selected_brush_index {
            self.world_brushes.get(b_idx).map(|b| b.polys.len()).unwrap_or(0)
        } else {
            self.builder_brush.polys.len()
        };
        if !self.selected_surfaces.is_empty() || !self.has_multi_selection() {
            self.apply_active_texture_to_selected_brush();
        } else {
            self.apply_active_texture_to_multi_selected();
        }
        self.active_texture = prev_active;
        cnt
    }


    /// Applies an arbitrary 2x2 UV matrix transformation `(uu, uv, vu, vv)` to the selected surfaces
    /// or active brush (`POLY TEXSCALE [RELATIVE] UU=... VV=... UV=... VU=...`).
    pub fn transform_selected_brush_texture_matrix(&mut self, uu: f32, uv: f32, vu: f32, vv: f32) {
        let snapshots = self.snapshot_selected_brush_polys();
        if !self.selected_surfaces.is_empty() {
            let mut brushes_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brushes_map.entry(b_idx).or_default().push(p_idx);
            }
            for (b_idx, polys) in brushes_map {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    for p_idx in polys {
                        if let Some(p) = b.polys.get_mut(p_idx) {
                            let old_u = p.texture_u;
                            let old_v = p.texture_v;
                            p.texture_u = old_u * uu + old_v * uv;
                            p.texture_v = old_u * vu + old_v * vv;
                        }
                    }
                }
            }
        } else if let Some(sel_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                for p in &mut b.polys {
                    let old_u = p.texture_u;
                    let old_v = p.texture_v;
                    p.texture_u = old_u * uu + old_v * uv;
                    p.texture_v = old_u * vu + old_v * vv;
                }
            }
        } else {
            for p in &mut self.builder_brush.polys {
                let old_u = p.texture_u;
                let old_v = p.texture_v;
                p.texture_u = old_u * uu + old_v * uv;
                p.texture_v = old_u * vu + old_v * vv;
            }
        }
        if !snapshots.is_empty() {
            self.commit_uv_modification(
                &snapshots,
                format!("UV Matrix Transform (UU={:.2}, UV={:.2}, VU={:.2}, VV={:.2})", uu, uv, vu, vv),
            );
        }
    }


    /// UnrealEd 2.0 `Surface Properties -> Rotation -> Small Diagonal / Big Diagonal` (`REQ-D05`):
    /// - `small == true` (`Small Diagonal`): `UU=1.0, VV=1.0, UV=1.0, VU=-1.0`
    /// - `small == false` (`Big Diagonal`): `UU=1.0, VV=0.5, UV=0.5, VU=-0.5`
    pub fn skew_selected_brush_texture_diagonal(&mut self, small: bool) {
        if small {
            self.transform_selected_brush_texture_matrix(1.0, 1.0, -1.0, 1.0);
        } else {
            self.transform_selected_brush_texture_matrix(1.0, 0.5, -0.5, 0.5);
        }
    }

}
