use super::*;

#[derive(Debug, PartialEq)]
enum InventoryReference<'a> {
    Absent,
    Null,
    Unresolved,
    Name(&'a str),
}

fn inventory_reference<'a>(actor: &'a ut99_format::ActorRecord, key: &str) -> InventoryReference<'a> {
    let property = actor.properties.iter().find(|(name, _)| name.eq_ignore_ascii_case(key)).map(|(_, p)| &p.value);
    let path = actor.object_paths.iter().find(|(name, _)| name.eq_ignore_ascii_case(key)).map(|(_, value)| value.as_str());
    if matches!(property, Some(TaggedValue::Object(0))) { return InventoryReference::Null; }
    let raw = path.or_else(|| match property {
        Some(TaggedValue::Name(value) | TaggedValue::Str(value)) => Some(value.as_str()),
        _ => None,
    });
    let Some(raw) = raw else {
        return if property.is_some() { InventoryReference::Unresolved } else { InventoryReference::Absent };
    };
    let raw = raw.trim().trim_matches('"');
    if raw.is_empty() || raw.eq_ignore_ascii_case("none") { return InventoryReference::Null; }
    let object_path = raw.split_once('\'').map(|(_, value)| value.trim_end_matches('\'')).unwrap_or(raw);
    // A path into another package or an unknown outer cannot be verified locally.
    if object_path.contains('.') && !object_path.split('.').next().is_some_and(|outer| outer.eq_ignore_ascii_case("MyLevel")) {
        return InventoryReference::Unresolved;
    }
    match object_path.rsplit('.').next().filter(|name| !name.is_empty()) {
        Some(name) => InventoryReference::Name(name),
        None => InventoryReference::Unresolved,
    }
}

fn audit_inventory_references(actors: &[ut99_format::ActorRecord]) -> Vec<MapIssue> {
    let mut issues = Vec::new();
    let mut by_name: std::collections::HashMap<String, Vec<&ut99_format::ActorRecord>> = std::collections::HashMap::new();
    for actor in actors {
        by_name.entry(actor.name.to_ascii_lowercase()).or_default().push(actor);
    }
    for actor in actors {
        let is_spot = actor.class_name.rsplit('.').next().unwrap_or(&actor.class_name).eq_ignore_ascii_case("InventorySpot");
        let (key, reverse) = if is_spot { ("markedItem", "myMarker") } else { ("myMarker", "markedItem") };
        let issue = |description: String| MapIssue {
            severity: ut99_bsp::IssueSeverity::Warning,
            category: "InventoryLinks".to_string(),
            description,
            object_name: actor.name.clone(),
            location: Some(actor.location),
            auto_fixable: false,
        };
        match inventory_reference(actor, key) {
            InventoryReference::Absent => {},
            InventoryReference::Null => issues.push(issue(format!("{} has an explicit empty {} reference.", actor.name, key))),
            InventoryReference::Unresolved => issues.push(issue(format!("{}: {} could not be resolved locally; review the reference.", actor.name, key))),
            InventoryReference::Name(target_name) => {
                let Some(matches) = by_name.get(&target_name.to_ascii_lowercase()) else {
                    issues.push(issue(format!("{}: {} references missing local actor '{}'.", actor.name, key, target_name)));
                    continue;
                };
                if matches.len() != 1 {
                    issues.push(issue(format!("{}: {} references ambiguous actor name '{}'.", actor.name, key, target_name)));
                    continue;
                }
                let target = matches[0];
                if !is_spot && !target.class_name.rsplit('.').next().unwrap_or(&target.class_name).eq_ignore_ascii_case("InventorySpot") {
                    issues.push(issue(format!("{}: {} references '{}' which is not an InventorySpot.", actor.name, key, target.name)));
                    continue;
                }
                match inventory_reference(target, reverse) {
                    InventoryReference::Name(back) if !back.eq_ignore_ascii_case(&actor.name) => {
                        issues.push(issue(format!("{}: {} references '{}', but its {} points to '{}'.", actor.name, key, target.name, reverse, back)));
                    },
                    InventoryReference::Absent | InventoryReference::Null => {
                        issues.push(issue(format!("{}: '{}' has no decoded reciprocal {} reference.", actor.name, target.name, reverse)));
                    },
                    InventoryReference::Unresolved => {
                        issues.push(issue(format!("{}: reciprocal {} on '{}' could not be resolved locally.", actor.name, reverse, target.name)));
                    },
                    InventoryReference::Name(_) => {},
                }
            }
        }
    }
    issues
}

pub(super) struct RepairSnapshot {
    brushes: Vec<Brush>,
    movers: Vec<EditorMover>,
    unr: Option<ut99_format::UnrLevel>,
    metadata: (String, String, String, String, String, String, u8, u8, u8),
}

impl EditorState {
    pub(super) fn repair_snapshot(&self) -> RepairSnapshot {
        RepairSnapshot {
            brushes: self.world_brushes.to_vec(),
            movers: self.movers.to_vec(),
            unr: self.loaded_unr.clone(),
            metadata: (self.level_title.clone(), self.level_author.clone(),
                self.level_ideal_player_count.clone(), self.level_song.clone(),
                self.level_game_type.clone(), self.level_enter_text.clone(),
                self.level_ambient_brightness, self.level_ambient_hue, self.level_ambient_saturation),
        }
    }

    pub(super) fn record_repairs(&mut self, before: RepairSnapshot, description: &str) {
        if before.brushes == self.world_brushes && before.movers == self.movers
            && before.unr.as_ref().map(|u| &u.actors) == self.loaded_unr.as_ref().map(|u| &u.actors)
        {
            return;
        }
        self.history.push(crate::history::Action::ReplaceEntireLevel {
            old_map_path: self.map_path.clone(),
            new_map_path: self.map_path.clone(),
            old_launch_args: self.ut99_launch_args.clone(),
            new_launch_args: self.ut99_launch_args.clone(),
            old_brushes: before.brushes,
            new_brushes: self.world_brushes.to_vec(),
            old_movers: before.movers,
            new_movers: self.movers.to_vec(),
            old_unr: before.unr,
            new_unr: self.loaded_unr.clone(),
            old_metadata: before.metadata.clone(),
            new_metadata: before.metadata,
            description: description.to_string(),
        });
        self.map_is_dirty = true;
    }

    pub fn level_audit_freshness(&self) -> LevelAuditFreshness {
        let Some(snapshot) = &self.level_audit_snapshot else { return LevelAuditFreshness::NotRun; };
        let actors = self.loaded_unr.as_ref().map(|level| level.actors.as_slice());
        if snapshot.world_brushes == self.world_brushes
            && snapshot.builder_brush == self.builder_brush
            && snapshot.movers == self.movers
            && snapshot.nav_graph == self.nav_graph
            && snapshot.unr_actors.as_deref() == actors
            && snapshot.t3d_actors == self.map.actors
            && snapshot.map_path == self.map_path
            && snapshot.level_title == self.level_title
            && snapshot.level_game_type == self.level_game_type
        {
            LevelAuditFreshness::Current
        } else {
            LevelAuditFreshness::Stale
        }
    }

    pub fn run_level_doctor_audit(&mut self) {
        self.map_audit_issues.clear();
        self.selected_issue_index = None;

        let brush_issues = LevelDoctor::audit_brushes(&self.world_brushes);
        self.map_audit_issues.extend(brush_issues);

        let portal_issues = LevelDoctor::audit_zone_portals(&self.world_brushes);
        self.map_audit_issues.extend(portal_issues);

        let nav_issues = LevelDoctor::audit_navigation_graph(&self.nav_graph);
        self.map_audit_issues.extend(nav_issues);

        let builder_diag = LevelDoctor::audit_builder_brush(&self.builder_brush);
        if !builder_diag.is_convex {
            self.map_audit_issues.push(MapIssue {
                severity: ut99_bsp::IssueSeverity::Warning,
                category: "BuilderBrush".to_string(),
                description: builder_diag.summary_message.clone(),
                object_name: "RedBuilderBrush".to_string(),
                location: Some(self.builder_brush.location),
                auto_fixable: false,
            });
        }

        if let Some(ref unr) = self.loaded_unr {
            let actor_tuples: Vec<_> = unr.actors.iter().map(|a| {
                (a.name.clone(), a.class_name.clone(), a.location, a.light_brightness, a.light_radius)
            }).collect();
            let actor_issues = LevelDoctor::audit_actor_records(&actor_tuples);
            self.map_audit_issues.extend(actor_issues);
            self.map_audit_issues.extend(audit_inventory_references(&unr.actors));
        } else {
            // T3D imports normally create an UNR actor view. When no such view
            // exists, inspect the original T3D actors without merging two copies.
            let actors: Vec<_> = self.map.actors.iter().map(|actor| {
                let mut record = ut99_format::ActorRecord::default();
                record.name = actor.name.clone();
                record.class_name = actor.class.clone();
                record.location = actor.get_location();
                for (key, value) in &actor.properties {
                    if key.eq_ignore_ascii_case("myMarker") || key.eq_ignore_ascii_case("markedItem") {
                        record.properties.insert(key.clone(), TaggedProperty {
                            name: key.clone(), array_index: 0, value: TaggedValue::Str(value.clone()),
                        });
                    }
                }
                record
            }).collect();
            self.map_audit_issues.extend(audit_inventory_references(&actors));
        }

        let mut link_actors = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                if !a.class_name.to_ascii_lowercase().contains("mover") {
                    link_actors.push((
                        a.name.clone(),
                        a.class_name.clone(),
                        a.tag.clone(),
                        a.event.clone(),
                        a.location,
                    ));
                    if a.class_name.to_ascii_lowercase().contains("dispatcher") {
                        for i in 0..8 {
                            let ev_key = format!("OutEvents[{}]", i);
                            if let Some(prop) = a.properties.get(&ev_key) {
                                if let TaggedValue::Str(ref ev_name) = prop.value {
                                    let trimmed = ev_name.trim();
                                    if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("None") {
                                        link_actors.push((
                                            format!("{}[OutEvents[{}]]", a.name, i),
                                            a.class_name.clone(),
                                            String::new(),
                                            trimmed.to_string(),
                                            a.location,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for m in &self.movers {
            link_actors.push((
                m.name.clone(),
                "Mover".to_string(),
                m.tag.clone(),
                m.event.clone(),
                m.brush.location,
            ));
        }
        for a in &self.map.actors {
            if !a.class.to_ascii_lowercase().contains("mover") {
                link_actors.push((
                    a.name.clone(),
                    a.class.clone(),
                    a.get_property("Tag").unwrap_or_default().trim_matches('"').to_string(),
                    a.get_property("Event").unwrap_or_default().trim_matches('"').to_string(),
                    a.get_location(),
                ));
            }
        }
        let link_issues = LevelDoctor::audit_actor_event_tag_links(&link_actors);
        self.map_audit_issues.extend(link_issues);

        let mut clearance_actors = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                clearance_actors.push((a.name.clone(), a.class_name.clone(), a.location));
            }
        }
        for a in &self.map.actors {
            clearance_actors.push((a.name.clone(), a.class.clone(), a.get_location()));
        }
        let clearance_issues = LevelDoctor::audit_gameplay_clearance(&self.world_brushes, &self.nav_graph, &clearance_actors);
        self.map_audit_issues.extend(clearance_issues);

        // Domination Setup Audit
        let mut dom_control_points = Vec::new();
        let mut dom_player_starts = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                if a.class_name.eq_ignore_ascii_case("ControlPoint") {
                    let point_name = a.properties.get("PointName").and_then(|p| match &p.value {
                        TaggedValue::Str(s) => Some(s.clone()),
                        _ => None,
                    }).unwrap_or_else(|| a.name.clone());
                    dom_control_points.push((a.name.clone(), point_name, a.location));
                } else if a.class_name.eq_ignore_ascii_case("PlayerStart") {
                    dom_player_starts.push(a.location);
                }
            }
        }
        for a in &self.map.actors {
            if a.class.eq_ignore_ascii_case("ControlPoint") {
                let point_name = a.get_property("PointName")
                    .map(|s| s.trim_matches('"').to_string())
                    .unwrap_or_else(|| a.name.clone());
                dom_control_points.push((a.name.clone(), point_name, a.get_location()));
            } else if a.class.eq_ignore_ascii_case("PlayerStart") {
                dom_player_starts.push(a.get_location());
            }
        }
        let dom_issues = LevelDoctor::audit_domination_setup(&dom_control_points, &dom_player_starts);
        self.map_audit_issues.extend(dom_issues);

        let mut ctf_flag_bases: Vec<(String, u8, Vec3)> = Vec::new();
        let mut ctf_player_starts: Vec<(String, u8, Vec3)> = Vec::new();
        let mut ctf_alternate_paths: Vec<(u8, f32, bool, Vec3)> = Vec::new();
        let mut ctf_defense_points: Vec<(String, u8, bool, Vec3)> = Vec::new();

        if let Some(unr) = &self.loaded_unr {
            for a in &unr.actors {
                let cl = a.class_name.to_lowercase();
                if cl == "flagbase" {
                    let team: u8 = a.properties.get("Team").and_then(|p| match p.value {
                        TaggedValue::Byte(b) => Some(b),
                        TaggedValue::Int(i) => Some(i as u8),
                        _ => None,
                    }).unwrap_or(0);
                    ctf_flag_bases.push((a.name.clone(), team, a.location));
                } else if cl == "playerstart" {
                    let team: u8 = a.properties.get("TeamNumber").and_then(|p| match p.value {
                        TaggedValue::Byte(b) => Some(b),
                        TaggedValue::Int(i) => Some(i as u8),
                        _ => None,
                    }).unwrap_or(0);
                    ctf_player_starts.push((a.name.clone(), team, a.location));
                } else if cl == "alternatepath" {
                    let team: u8 = a.properties.get("team").or_else(|| a.properties.get("Team")).and_then(|p| match p.value {
                        TaggedValue::Byte(b) => Some(b),
                        TaggedValue::Int(i) => Some(i as u8),
                        _ => None,
                    }).unwrap_or(0);
                    let weight: f32 = a.properties.get("SelectionWeight").and_then(|p| match p.value {
                        TaggedValue::Float(f) => Some(f),
                        _ => None,
                    }).unwrap_or(1.0);
                    let ret_only: bool = a.properties.get("bReturnOnly").and_then(|p| match p.value {
                        TaggedValue::Bool(b) => Some(b),
                        _ => None,
                    }).unwrap_or(false);
                    ctf_alternate_paths.push((team, weight, ret_only, a.location));
                } else if cl == "defensepoint" {
                    let team: u8 = a.properties.get("team").or_else(|| a.properties.get("Team")).and_then(|p| match p.value {
                        TaggedValue::Byte(b) => Some(b),
                        TaggedValue::Int(i) => Some(i as u8),
                        _ => None,
                    }).unwrap_or(0);
                    let is_sniping: bool = a.properties.get("bSniping").and_then(|p| match p.value {
                        TaggedValue::Bool(b) => Some(b),
                        _ => None,
                    }).unwrap_or(false);
                    ctf_defense_points.push((a.name.clone(), team, is_sniping, a.location));
                }
            }
        }
        for a in &self.map.actors {
            let cl = a.class.to_lowercase();
            if cl == "flagbase" {
                let team: u8 = a.get_property("Team").and_then(|s| s.parse().ok()).unwrap_or(0);
                ctf_flag_bases.push((a.name.clone(), team, a.get_location()));
            } else if cl == "playerstart" {
                let team: u8 = a.get_property("TeamNumber").and_then(|s| s.parse().ok()).unwrap_or(0);
                ctf_player_starts.push((a.name.clone(), team, a.get_location()));
            } else if cl == "alternatepath" {
                let team: u8 = a.get_property("team").or_else(|| a.get_property("Team")).and_then(|s| s.parse().ok()).unwrap_or(0);
                let weight: f32 = a.get_property("SelectionWeight").and_then(|s| s.parse().ok()).unwrap_or(1.0);
                let ret_only: bool = a.get_property("bReturnOnly").map(|s| s.eq_ignore_ascii_case("true")).unwrap_or(false);
                ctf_alternate_paths.push((team, weight, ret_only, a.get_location()));
            } else if cl == "defensepoint" {
                let team: u8 = a.get_property("team").or_else(|| a.get_property("Team")).and_then(|s| s.parse().ok()).unwrap_or(0);
                let is_sniping: bool = a.get_property("bSniping").map(|s| s.eq_ignore_ascii_case("true")).unwrap_or(false);
                ctf_defense_points.push((a.name.clone(), team, is_sniping, a.get_location()));
            }
        }
        let ctf_issues = LevelDoctor::audit_ctf_setup(&ctf_flag_bases, &ctf_player_starts, &ctf_alternate_paths, &ctf_defense_points);
        self.map_audit_issues.extend(ctf_issues);

        // Tournament Playability & Combat Readiness Audit
        let mut play_player_starts = Vec::new();
        let mut play_weapons = Vec::new();
        let mut play_ammo = Vec::new();

        let is_wep_cls = |cls: &str| -> bool {
            let l = cls.to_ascii_lowercase();
            l.contains("shock")
                || l.contains("flak")
                || l.contains("eightball")
                || l.contains("minigun")
                || l.contains("pulse")
                || l.contains("ripper")
                || l.contains("biorifle")
                || l.contains("sniper")
                || l.contains("enforcer")
        };
        let is_ammo_cls = |cls: &str| -> bool {
            let l = cls.to_ascii_lowercase();
            l.contains("ammo")
                || l.contains("core")
                || l.contains("pack")
                || l.contains("box")
                || l.contains("clip")
                || l.contains("hopper")
        };

        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                if a.class_name.eq_ignore_ascii_case("PlayerStart") {
                    play_player_starts.push(a.location);
                } else if is_ammo_cls(&a.class_name) {
                    play_ammo.push((a.class_name.clone(), a.location));
                } else if is_wep_cls(&a.class_name) {
                    play_weapons.push((a.class_name.clone(), a.location));
                }
            }
        }
        for a in &self.map.actors {
            if a.class.eq_ignore_ascii_case("PlayerStart") {
                play_player_starts.push(a.get_location());
            } else if is_ammo_cls(&a.class) {
                play_ammo.push((a.class.clone(), a.get_location()));
            } else if is_wep_cls(&a.class) {
                play_weapons.push((a.class.clone(), a.get_location()));
            }
        }

        let playability_issues = LevelDoctor::audit_tournament_playability(&play_player_starts, &play_weapons, &play_ammo);
        self.map_audit_issues.extend(playability_issues);

        // Map Prefix vs Game Mode & Objective Coherence Audit
        let map_name = self.map_path.as_ref()
            .and_then(|p| p.file_stem().and_then(|s| s.to_str()))
            .unwrap_or(&self.level_title);

        let mut fort_standards_count = 0;
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                if a.class_name.eq_ignore_ascii_case("FortStandard") {
                    fort_standards_count += 1;
                }
            }
        }
        for a in &self.map.actors {
            if a.class.eq_ignore_ascii_case("FortStandard") {
                fort_standards_count += 1;
            }
        }

        let coherence_issues = LevelDoctor::audit_mode_prefix_coherence(
            map_name,
            if self.level_game_type.is_empty() { None } else { Some(&self.level_game_type) },
            dom_control_points.len(),
            ctf_flag_bases.len(),
            fort_standards_count,
        );
        self.map_audit_issues.extend(coherence_issues);

        self.level_audit_snapshot = Some(LevelAuditSnapshot {
            world_brushes: self.world_brushes.to_vec(),
            builder_brush: self.builder_brush.clone(),
            movers: self.movers.to_vec(),
            nav_graph: self.nav_graph.clone(),
            unr_actors: self.loaded_unr.as_ref().map(|level| level.actors.to_vec()),
            t3d_actors: self.map.actors.to_vec(),
            map_path: self.map_path.clone(),
            level_title: self.level_title.clone(),
            level_game_type: self.level_game_type.clone(),
        });

        self.build_progress = Some((1.0, format!("Level Doctor: {} issue(s)", self.map_audit_issues.len())));
        self.log(format!("Level Doctor Audit complete: {} issue(s) detected.", self.map_audit_issues.len()));
    }

    pub fn autofix_level_issues(&mut self) -> usize {
        let before = self.repair_snapshot();
        let fixes = self.autofix_level_issues_without_history();
        self.record_repairs(before, "Level Doctor auto-fix");
        fixes
    }

    pub(super) fn autofix_level_issues_without_history(&mut self) -> usize {
        let def_tex = self.active_texture.clone().unwrap_or_else(|| "Indus1.Indus_B01".to_string());
        let mut total_fixes = 0;

        for brush in &mut self.world_brushes {
            let fixes = LevelDoctor::autofix_brush(brush, &def_tex);
            total_fixes += fixes;
        }

        self.run_level_doctor_audit();
        self.build_progress = Some((1.0, format!("Auto-Fix: Applied {} correction(s)", total_fixes)));
        self.log(format!("Level Doctor Auto-Fix applied {} modification(s).", total_fixes));
        total_fixes
    }

/// Returns real-time pre-build CSG diagnostics for the active Red Builder Brush.
    pub fn audit_builder_brush_csg_safety(&self) -> ut99_bsp::BuilderBrushDiagnostics {
        LevelDoctor::audit_builder_brush(&self.builder_brush)
    }
}

#[cfg(test)]
mod audit_freshness_tests {
    use super::*;

    fn reference(actor: &mut ut99_format::ActorRecord, key: &str, name: &str) {
        actor.properties.insert(key.into(), TaggedProperty {
            name: key.into(), array_index: 0, value: TaggedValue::Name(name.into()),
        });
    }

    #[test]
    fn inventory_links_check_both_directions_without_editing_actors() {
        let mut item = ut99_format::ActorRecord::default();
        item.name = "Ammo".into();
        item.class_name = "ShockAmmo".into();
        let mut spot = ut99_format::ActorRecord::default();
        spot.name = "InventorySpot5".into();
        spot.class_name = "InventorySpot".into();
        reference(&mut item, "myMarker", "InventorySpot'MyLevel.InventorySpot5'");
        reference(&mut spot, "markedItem", "ShockAmmo'MyLevel.Ammo'");
        let valid = vec![item.clone(), spot.clone()];
        assert!(audit_inventory_references(&valid).is_empty());
        assert_eq!(valid, vec![item.clone(), spot.clone()]);

        reference(&mut spot, "markedItem", "OtherAmmo");
        let issues = audit_inventory_references(&[item.clone(), spot.clone()]);
        assert!(issues.iter().any(|i| i.description.contains("points to 'OtherAmmo'")));
        assert!(issues.iter().any(|i| i.description.contains("missing local actor 'OtherAmmo'")));
        reference(&mut item, "myMarker", "MissingSpot");
        assert!(audit_inventory_references(&[item, spot]).iter().any(|i| i.description.contains("missing local actor 'MissingSpot'")));
    }

    #[test]
    fn inventory_links_distinguish_null_unresolved_external_and_ambiguous() {
        let mut item = ut99_format::ActorRecord::default();
        item.name = "Ammo".into();
        item.class_name = "ShockAmmo".into();
        item.properties.insert("myMarker".into(), TaggedProperty {
            name: "myMarker".into(), array_index: 0, value: TaggedValue::Object(0),
        });
        assert!(audit_inventory_references(&[item.clone()])[0].description.contains("explicit empty"));
        item.properties.get_mut("myMarker").unwrap().value = TaggedValue::Object(123);
        assert!(audit_inventory_references(&[item.clone()])[0].description.contains("could not be resolved"));
        item.object_paths.insert("myMarker".into(), "InventorySpot'OtherPackage.Spot'".into());
        assert!(audit_inventory_references(&[item.clone()])[0].description.contains("could not be resolved"));
        item.object_paths.clear();
        reference(&mut item, "myMarker", "Spot");
        let mut spot = ut99_format::ActorRecord::default();
        spot.name = "Spot".into();
        spot.class_name = "InventorySpot".into();
        assert!(audit_inventory_references(&[item, spot.clone(), spot]).iter().any(|i| i.description.contains("ambiguous")));
    }

    #[test]
    fn t3d_inventory_references_are_checked_without_an_unr_view() {
        let mut state = EditorState::default();
        state.loaded_unr = None;
        let mut item = ut99_format::T3dActor::new("Botpack.ShockAmmo", "Ammo");
        item.set_property("MyMarker", "InventorySpot'MyLevel.Spot'");
        let mut spot = ut99_format::T3dActor::new("Engine.InventorySpot", "Spot");
        spot.set_property("markedItem", "\"ShockAmmo'MyLevel.Ammo'\"");
        state.map.actors = vec![item, spot].into();
        state.run_level_doctor_audit();
        assert!(!state.map_audit_issues.iter().any(|i| i.category == "InventoryLinks"));
        state.map.actors[0].set_property("MyMarker", "Missing");
        state.run_level_doctor_audit();
        assert!(state.map_audit_issues.iter().any(|i| i.category == "InventoryLinks" && i.description.contains("missing local actor 'Missing'")));
    }

    #[test]
    fn imported_t3d_checks_the_synthetic_actor_view() {
        let mut state = EditorState::default();
        let mut item = ut99_format::T3dActor::new("Botpack.ShockAmmo", "Ammo");
        item.set_property("myMarker", "InventorySpot'MyLevel.Missing'");
        let mut map = ut99_format::T3dMap::new();
        map.actors.push(item);
        state.load_t3d_from_map(map, Some("DM-Import"));
        state.run_level_doctor_audit();
        assert_eq!(state.map_audit_issues.iter().filter(|i| i.category == "InventoryLinks" && i.description.contains("missing local actor 'Missing'")).count(), 1);
    }

    #[test]
    fn repairs_are_one_undoable_action_and_do_not_invent_a_spawn() {
        let mut state = EditorState::default();
        state.loaded_unr = Some(ut99_format::UnrLevel::default());
        for name in ["first", "second"] {
            let mut brush = ut99_bsp::build_cube(name, ut99_bsp::CsgOperation::Subtract, Vec3::splat(256.0));
            brush.polys[0].texture.clear();
            state.world_brushes.push(brush);
        }
        let original = state.world_brushes.clone();
        assert!(state.autofix_level_issues() >= 2);
        let repaired = state.world_brushes.clone();
        assert_eq!(state.history.undo_stack.len(), 1);
        assert!(state.loaded_unr.as_ref().unwrap().actors.is_empty());
        assert!(state.map_audit_issues.iter().any(|i| i.severity == ut99_bsp::IssueSeverity::Error && i.description.contains("0 PlayerStart")));
        state.undo();
        assert_eq!(state.world_brushes, original);
        state.redo();
        assert_eq!(state.world_brushes, repaired);
    }

    #[test]
    fn pre_export_marker_creation_is_undoable_even_when_audit_fails() {
        let mut state = EditorState::default();
        let mut level = ut99_format::UnrLevel::default();
        let mut item = ut99_format::ActorRecord::default();
        item.name = "TestShock".into();
        item.class_name = "ShockRifle".into();
        level.actors.push(item);
        state.loaded_unr = Some(level);
        let original = state.loaded_unr.as_ref().unwrap().actors.clone();
        assert!(state.audit_and_autofix_before_export().is_err());
        let repaired = state.loaded_unr.as_ref().unwrap().actors.clone();
        assert!(repaired.iter().any(|a| a.class_name == "InventorySpot"));
        assert_eq!(state.history.undo_stack.len(), 1);
        state.undo();
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, original);
        state.redo();
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, repaired);
    }

    #[test]
    fn audit_freshness_tracks_changes_to_actual_inputs() {
        let mut state = EditorState::default();
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::NotRun);
        state.run_level_doctor_audit();
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Current);

        let title = state.level_title.clone();
        state.level_title = "changed".into();
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Stale);
        state.level_title = title;
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Current);

        state.nav_graph.add_node("new", Vec3::ZERO, "PathNode");
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Stale);
        state.run_level_doctor_audit();
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Current);

        state.loaded_unr = Some(ut99_format::UnrLevel::default());
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Current);
        state.run_level_doctor_audit();
        let mut actor = ut99_format::ActorRecord::default();
        actor.name = "new actor".into();
        state.loaded_unr.as_mut().unwrap().actors.push(actor);
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Stale);

        state.run_level_doctor_audit();
        state.world_brushes.push(ut99_bsp::build_cube(
            "added", ut99_bsp::CsgOperation::Subtract, Vec3::splat(256.0)
        ));
        state.map_is_dirty = false; // A saved dirty bit is not an audit revision.
        assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::Stale);
    }
}
