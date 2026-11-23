use super::*;
use glam::Vec3;

impl EditorState {
    pub fn add_light_at_builder(&mut self) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.lights.len();
            let mut light = ut99_format::ActorRecord::default();
            light.name = format!("Light{}", count);
            light.class_name = "Light".into();
            light.location = loc;
            light.light_brightness = 64;
            light.light_radius = 64;
            unr.lights.push(light.clone());
            unr.actors.push(light);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.selected_brush_index = None;
            self.log(format!("Added Light at ({:.0}, {:.0}, {:.0})", loc.x, loc.y, loc.z));
        }
    }

    pub fn add_player_start_at_builder(&mut self) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.player_starts.len();
            let mut start = ut99_format::ActorRecord::default();
            start.name = format!("PlayerStart{}", count);
            start.class_name = "PlayerStart".into();
            start.location = loc;
            unr.player_starts.push(start.clone());
            unr.actors.push(start);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.selected_brush_index = None;
            self.log(format!("Added PlayerStart at ({:.0}, {:.0}, {:.0})", loc.x, loc.y, loc.z));
            self.refresh_ai_path_links();
        }
    }

    pub fn add_path_node_at_builder(&mut self) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.path_nodes.len();
            let mut node = ut99_format::ActorRecord::default();
            node.name = format!("PathNode{}", count);
            node.class_name = "PathNode".into();
            node.location = loc;
            unr.path_nodes.push(node.clone());
            unr.actors.push(node);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.selected_brush_index = None;
            self.log(format!("Added PathNode at ({:.0}, {:.0}, {:.0})", loc.x, loc.y, loc.z));
            self.refresh_ai_path_links();
        }
    }

    pub fn add_weapon_or_pickup_at_builder(&mut self, class_name: &str) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.weapons_and_pickups.len();
            let mut item = ut99_format::ActorRecord::default();
            item.name = format!("{}_{}", class_name, count);
            item.class_name = class_name.to_string();
            item.location = loc;
            unr.weapons_and_pickups.push(item.clone());
            unr.actors.push(item);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.selected_brush_index = None;
            self.log(format!("Added {} at ({:.0}, {:.0}, {:.0})", class_name, loc.x, loc.y, loc.z));
            self.refresh_ai_path_links();
        }
    }


    pub fn load_actor_classes(&mut self) {
        use ut99_format::package::{build_class_tree, scan_u_package_classes, ClassResource, Package};

        let mut classes = Vec::new();
        let sys_dir = std::path::Path::new("UnrealTournament/System");

        if sys_dir.exists() {
            let u_pkgs = ["Core.u", "Engine.u", "Botpack.u", "UnrealShare.u", "UnrealI.u"];
            for pkg_file in &u_pkgs {
                let pkg_path = sys_dir.join(pkg_file);
                if pkg_path.exists() {
                    let base_name = pkg_file.trim_end_matches(".u");
                    if let Ok(pkg) = Package::open(&pkg_path) {
                        let extracted = scan_u_package_classes(&pkg, base_name);
                        classes.extend(extracted);
                    }
                }
            }
        }

        // Canonical Unreal Tournament 99 fallback classes (ensures full hierarchy even without disk install)
        let canonical_classes = [
            ("Actor", "Object", "Engine"),
            ("Brush", "Actor", "Engine"),
            ("Mover", "Brush", "Engine"),
            ("Info", "Actor", "Engine"),
            ("GameInfo", "Info", "Engine"),
            ("TournamentGameInfo", "GameInfo", "Botpack"),
            ("DeathMatchPlus", "TournamentGameInfo", "Botpack"),
            ("TeamGamePlus", "TournamentGameInfo", "Botpack"),
            ("CTFGame", "TournamentGameInfo", "Botpack"),
            ("ZoneInfo", "Info", "Engine"),
            ("SkyZoneInfo", "ZoneInfo", "Engine"),
            ("WaterZone", "ZoneInfo", "Engine"),
            ("LavaZone", "ZoneInfo", "Engine"),
            ("SlimeZone", "ZoneInfo", "Engine"),
            ("Inventory", "Actor", "Engine"),
            ("Pickup", "Inventory", "Engine"),
            ("TournamentPickup", "Pickup", "Botpack"),
            ("MedBox", "TournamentPickup", "Botpack"),
            ("HealthVial", "TournamentPickup", "Botpack"),
            ("Armor2", "TournamentPickup", "Botpack"),
            ("ThighPads", "TournamentPickup", "Botpack"),
            ("UT_ShieldBelt", "TournamentPickup", "Botpack"),
            ("UDamage", "TournamentPickup", "Botpack"),
            ("JumpBoots", "TournamentPickup", "Botpack"),
            ("Weapon", "Inventory", "Engine"),
            ("TournamentWeapon", "Weapon", "Botpack"),
            ("ImpactHammer", "TournamentWeapon", "Botpack"),
            ("Translocator", "TournamentWeapon", "Botpack"),
            ("Enforcer", "TournamentWeapon", "Botpack"),
            ("UT_BioRifle", "TournamentWeapon", "Botpack"),
            ("ShockRifle", "TournamentWeapon", "Botpack"),
            ("PulseGun", "TournamentWeapon", "Botpack"),
            ("Ripper", "TournamentWeapon", "Botpack"),
            ("Minigun2", "TournamentWeapon", "Botpack"),
            ("UT_FlakCannon", "TournamentWeapon", "Botpack"),
            ("UT_Eightball", "TournamentWeapon", "Botpack"),
            ("SniperRifle", "TournamentWeapon", "Botpack"),
            ("WarheadLauncher", "TournamentWeapon", "Botpack"),
            ("Light", "Actor", "Engine"),
            ("Spotlight", "Light", "Engine"),
            ("TorchFlame", "Light", "Engine"),
            ("NavigationPoint", "Actor", "Engine"),
            ("PathNode", "NavigationPoint", "Engine"),
            ("PlayerStart", "NavigationPoint", "Engine"),
            ("InventorySpot", "NavigationPoint", "Engine"),
            ("LiftCenter", "NavigationPoint", "Engine"),
            ("LiftExit", "NavigationPoint", "Engine"),
            ("Teleporter", "NavigationPoint", "Engine"),
            ("Pawn", "Actor", "Engine"),
            ("PlayerPawn", "Pawn", "Engine"),
            ("TournamentPlayer", "PlayerPawn", "Botpack"),
            ("Bot", "Pawn", "Botpack"),
            ("Triggers", "Actor", "Engine"),
            ("Trigger", "Triggers", "Engine"),
            ("Dispatcher", "Triggers", "Engine"),
            ("Keypoint", "Actor", "Engine"),
            ("AmbientSound", "Keypoint", "Engine"),
            ("SpecialEvent", "Keypoint", "Engine"),
        ];

        for (name, parent, pkg) in canonical_classes {
            if !classes.iter().any(|c| c.name.eq_ignore_ascii_case(name)) {
                classes.push(ClassResource {
                    name: name.to_string(),
                    super_name: parent.to_string(),
                    package: pkg.to_string(),
                });
            }
        }

        self.class_tree = Some(build_class_tree(&classes));
        self.all_classes = classes;
    }

    pub fn place_actor_of_class_at(&mut self, class_name: &str, loc: Vec3) {
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        let (sup, pkg) = self
            .all_classes
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(class_name))
            .map(|c| (Some(c.super_name.to_string()), Some(c.package.to_string())))
            .unwrap_or((None, None));
        let meta = crate::unrealscript::get_class_metadata(
            class_name,
            sup.as_deref(),
            pkg.as_deref(),
        );

        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.actors.iter().filter(|a| a.class_name == class_name).count();
            let mut actor = ut99_format::ActorRecord::default();
            actor.name = format!("{}_{}", class_name, count);
            actor.class_name = class_name.to_string();
            actor.location = loc;
            if let Some(ref p) = pkg {
                if !p.is_empty() {
                    actor
                        .object_paths
                        .insert("__class_package".to_string(), p.clone());
                }
            }
            for (k, v) in &meta.default_properties {
                actor.properties.entry(k.clone()).or_insert_with(|| ut99_format::TaggedProperty {
                    name: k.clone(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Str(v.clone()),
                });
            }
            if class_name.to_ascii_lowercase().contains("light") {
                actor.light_brightness = 64;
                actor.light_radius = 32;
                actor.light_saturation = 255;
            }
            self.history
                .push(crate::history::Action::AddActor { actor: actor.clone() });
            unr.actors.push(actor);
            unr.sync_actor_views();
            let new_idx = unr.actors.len() - 1;
            self.selected_actor_index = Some(new_idx);
            self.selected_brush_index = None;
            self.selected_mover_index = None;
            self.selected_static_mesh_index = None;
            self.map_is_dirty = true;
            self.log(format!("Placed Actor '{}_{}' ({}) at ({:.0}, {:.0}, {:.0}).", class_name, count, class_name, loc.x, loc.y, loc.z));
            self.toast_success(format!("Placed {class_name}"));
        }
    }

    pub fn place_actor_of_class(&mut self, class_name: &str) {
        let loc = self.builder_brush.location;
        self.place_actor_of_class_at(class_name, loc);
    }

}


impl EditorState {
    /// Replaces the class of all currently selected actors with `new_class_name` while preserving
    /// their transform (`location`, `rotation`, `prepivot`, `main_scale`, `post_scale`), `tag`, `event`,
    /// `group`, and existing properties, and injecting any missing default properties of the new class (`REQ-C05`).
    pub fn replace_selected_actors_with_class(&mut self, new_class_name: &str) -> usize {
        let target_cls = new_class_name.trim();
        if target_cls.is_empty() {
            return 0;
        }
        let mut selected_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if let Some(sel) = self.selected_actor_index {
            if !selected_indices.contains(&sel) {
                selected_indices.push(sel);
            }
        }
        selected_indices.sort_unstable();
        if selected_indices.is_empty() {
            self.log("Actor Replace: No actor(s) selected.".to_string());
            return 0;
        }

        let short_cls = target_cls.rsplit('.').next().unwrap_or(target_cls);
        let pkg_opt = target_cls.split_once('.').map(|(p, _)| p);
        let meta = crate::unrealscript::get_class_metadata(short_cls, None, pkg_opt);

        let mut modifications = Vec::new();
        if let Some(ref mut unr) = self.loaded_unr {
            for idx in selected_indices {
                if let Some(actor) = unr.actors.get_mut(idx) {
                    let before = actor.clone();
                    actor.class_name = target_cls.to_string();
                    // Rename actor instance prefix to match new class while keeping numeric suffix
                    let suffix: String = actor
                        .name
                        .chars()
                        .rev()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    actor.name = if suffix.is_empty() {
                        format!("{}{}", short_cls, idx)
                    } else {
                        format!("{}{}", short_cls, suffix)
                    };

                    // Inject class default properties that aren't already overridden on the actor
                    for (k, v_str) in &meta.default_properties {
                        if !actor.properties.contains_key(k) {
                            let val = Self::parse_default_prop_value(v_str);
                            actor.properties.insert(
                                k.clone(),
                                TaggedProperty {
                                    name: k.clone(),
                                    array_index: 0,
                                    value: val,
                                },
                            );
                        }
                    }

                    let after = actor.clone();
                    modifications.push((idx, before, after));
                }
            }
        }

        let count = modifications.len();
        if count > 0 {
            self.history.push(Action::BatchUpdateLightEffects {
                modifications,
                description: format!("Replace {} actor(s) with class '{}'", count, target_cls),
            });
            self.map_is_dirty = true;
            self.log(format!(
                "Replaced {} selected actor(s) with class '{}' (preserving transform, Tag, and Event).",
                count, target_cls
            ));
        }
        count
    }


    /// Selects all actors matching `class_name` (or its subclasses when `include_subclasses` is true) (`REQ-K03`).
    pub fn select_actors_of_class(&mut self, class_name: &str, include_subclasses: bool) -> usize {
        let target = class_name.trim().rsplit('.').next().unwrap_or(class_name.trim());
        if target.is_empty() {
            return 0;
        }
        self.multi_selected_actors.clear();
        self.selected_actor_index = None;

        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                let act_short = actor.class_name.rsplit('.').next().unwrap_or(&actor.class_name);
                let mut matches = act_short.eq_ignore_ascii_case(target)
                    || actor.class_name.eq_ignore_ascii_case(class_name.trim());
                if !matches && include_subclasses {
                    // Check class hierarchy via metadata super_class chain or substring match
                    let meta = crate::unrealscript::get_class_metadata(act_short, None, None);
                    if meta.super_class.eq_ignore_ascii_case(target)
                        || act_short.to_ascii_lowercase().contains(&target.to_ascii_lowercase())
                    {
                        matches = true;
                    }
                }
                if matches {
                    self.multi_selected_actors.insert(idx);
                    if self.selected_actor_index.is_none() {
                        self.selected_actor_index = Some(idx);
                    }
                }
            }
        }

        let count = self.multi_selected_actors.len();
        self.log(format!(
            "Selected {} actor(s) matching class '{}' (include_subclasses={}).",
            count, class_name, include_subclasses
        ));
        count
    }


    /// Resets an actor's properties and light/scale attributes back to its UnrealScript class `defaultproperties`
    /// while preserving its identity (`name`, `location`, `rotation`, `tag`, `event`, `group`) (`REQ-E08`).
    pub fn reset_actor_properties_to_class_defaults(&mut self, actor_idx: usize) -> usize {
        let Some(ref mut unr) = self.loaded_unr else {
            return 0;
        };
        let Some(actor) = unr.actors.get_mut(actor_idx) else {
            return 0;
        };

        let before = actor.clone();
        let short_cls = actor.class_name.rsplit('.').next().unwrap_or(&actor.class_name).to_string();
        let pkg_opt = actor.class_name.split_once('.').map(|(p, _)| p.to_string());
        let meta = crate::unrealscript::get_class_metadata(
            &short_cls,
            None,
            pkg_opt.as_deref(),
        );

        // If the user has edited live defaultproperties for this exact class in UnrealScript Studio, honor those too
        let defaults_source: Vec<(String, String)> = if self.uc_viewer_class.eq_ignore_ascii_case(&short_cls)
            && !self.uc_editable_defaults.is_empty()
        {
            self.uc_editable_defaults.clone()
        } else {
            meta.default_properties.clone()
        };

        actor.properties.clear();
        actor.main_scale = Vec3::ONE;
        actor.post_scale = Vec3::ONE;
        actor.prepivot = Vec3::ZERO;
        if short_cls.contains("Light") {
            actor.light_brightness = 64;
            actor.light_radius = 64;
            actor.light_hue = 0;
            actor.light_saturation = 255;
            actor.light_type = 1;
            actor.light_effect = 0;
        }

        for (k, v_str) in &defaults_source {
            let val = Self::parse_default_prop_value(v_str);
            match k.as_str() {
                "LightBrightness" => {
                    if let TaggedValue::Int(v) = val {
                        actor.light_brightness = v.clamp(0, 255) as u8;
                    }
                }
                "LightRadius" => {
                    if let TaggedValue::Int(v) = val {
                        actor.light_radius = v.clamp(0, 255) as u8;
                    }
                }
                "LightHue" => {
                    if let TaggedValue::Int(v) = val {
                        actor.light_hue = v.clamp(0, 255) as u8;
                    }
                }
                "LightSaturation" => {
                    if let TaggedValue::Int(v) = val {
                        actor.light_saturation = v.clamp(0, 255) as u8;
                    }
                }
                _ => {}
            }
            actor.properties.insert(
                k.clone(),
                TaggedProperty {
                    name: k.clone(),
                    array_index: 0,
                    value: val,
                },
            );
        }

        let restored_count = actor.properties.len();
        let after = actor.clone();
        self.history.push(Action::BatchUpdateLightEffects {
            modifications: vec![(actor_idx, before, after)],
            description: format!("Reset '{}' to '{}' Class Defaults", unr.actors[actor_idx].name, short_cls),
        });
        self.map_is_dirty = true;
        self.log(format!(
            "Reset actor #{actor_idx} to '{short_cls}' defaultproperties ({restored_count} default properties applied)."
        ));
        restored_count
    }


    fn parse_default_prop_value(v_str: &str) -> TaggedValue {
        let trimmed = v_str.trim().trim_matches('"');
        if trimmed.eq_ignore_ascii_case("true") {
            TaggedValue::Bool(true)
        } else if trimmed.eq_ignore_ascii_case("false") {
            TaggedValue::Bool(false)
        } else if let Ok(i) = trimmed.parse::<i32>() {
            TaggedValue::Int(i)
        } else if let Ok(f) = trimmed.parse::<f32>() {
            TaggedValue::Float(f)
        } else {
            TaggedValue::Str(trimmed.to_string())
        }
    }

}
