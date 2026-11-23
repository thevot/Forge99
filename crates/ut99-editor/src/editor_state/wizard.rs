use super::*;
use glam::Vec3;

impl EditorState {
    // =========================================================================
    // New Valid UT99 Map Generator Wizard & Official Map DNA Cloner
    // =========================================================================

    /// Scans `UnrealTournament/Maps` for official `.unr` map files so the user can clone
    /// authentic texture palettes, HSV lighting, music tracks, and weapon loadouts.
    pub fn scan_official_maps_for_wizard(&mut self) {
        let mut maps = Vec::new();
        let mut dirs = vec![
            std::path::PathBuf::from("UnrealTournament/Maps"),
            std::path::PathBuf::from("../../UnrealTournament/Maps"),
        ];
        if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
            dirs.push(ut_dir.join("Maps"));
        }

        for dir in dirs {
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.eq_ignore_ascii_case("unr"))
                            .unwrap_or(false)
                        {
                            // Wizard-generated maps (manifest next to the file) are never offered as reference.
                            if ut99_format::unr::has_generator_manifest(&path) {
                                continue;
                            }
                            if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                                let upper = fname.to_ascii_uppercase();
                                if upper.starts_with("DM-")
                                    || upper.starts_with("CTF-")
                                    || upper.starts_with("DOM-")
                                    || upper.starts_with("AS-")
                                {
                                    maps.push(fname.to_string());
                                }
                            }
                        }
                    }
                }
                if !maps.is_empty() {
                    break;
                }
            }
        }

        maps.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
        maps.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        self.available_official_maps = maps;
        if !self.available_official_maps.is_empty()
            && !self
                .available_official_maps
                .iter()
                .any(|m| m.eq_ignore_ascii_case(&self.selected_official_dna_map))
        {
            self.selected_official_dna_map = self.available_official_maps[0].clone();
        }
    }

    /// Forensically extracts the architectural, texture, lighting, audio, and gameplay DNA from an
    /// official `.unr` map in `UnrealTournament/Maps` and applies it to `self.map_wizard_config`.
    pub fn clone_dna_from_official_map(
        &mut self,
        map_filename: &str,
    ) -> Result<ut99_format::OfficialMapDna, String> {
        let clean_name = if map_filename.to_ascii_lowercase().ends_with(".unr") {
            map_filename.to_string()
        } else {
            format!("{}.unr", map_filename)
        };

        let mut candidates = vec![
            std::path::PathBuf::from("UnrealTournament/Maps").join(&clean_name),
            std::path::PathBuf::from("../../UnrealTournament/Maps").join(&clean_name),
        ];
        if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
            candidates.push(ut_dir.join("Maps").join(&clean_name));
        }

        let resolved_path = candidates
            .into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| format!("Official map '{}' not found in UnrealTournament/Maps", clean_name))?;

        let dna = ut99_format::OfficialMapDna::from_unr_file(&resolved_path)
            .map_err(|e| format!("Failed to extract DNA from '{}': {}", clean_name, e))?;

        let fallback = self.map_wizard_config.theme.palette();
        let wall_tex = dna.textures.primary_wall(&fallback.wall_texture);
        let floor_tex = dna.textures.primary_floor(&fallback.floor_texture);
        let ceil_tex = dna.textures.primary_ceiling(&fallback.ceiling_texture);
        let trim_tex = dna.textures.primary_trim(&fallback.trim_texture);
        let pillar_tex = dna
            .textures
            .wall_textures
            .get(1)
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.pillar_texture.clone());
        let liquid_tex = dna.textures.primary_liquid(&fallback.liquid_texture);
        let sky_tex = dna.textures.primary_sky(&fallback.sky_texture);

        let song = if !dna.song.is_empty() {
            dna.song.clone()
        } else {
            fallback.song.clone()
        };

        let hazard_dps = if dna.lighting.lava_zone_count > 0 {
            50
        } else if dna.lighting.slime_zone_count > 0 {
            35
        } else {
            fallback.hazard_damage_per_sec
        };

        let secondary_wall_tex = dna
            .textures
            .wall_textures
            .get(1)
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| wall_tex.clone());
        let corridor_floor_tex = dna
            .textures
            .floor_textures
            .get(1)
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| floor_tex.clone());

        let custom_palette = ut99_bsp::WizardResolvedPalette {
            wall_texture: wall_tex.clone(),
            secondary_wall_texture: secondary_wall_tex,
            floor_texture: floor_tex.clone(),
            corridor_floor_texture: corridor_floor_tex,
            ceiling_texture: ceil_tex,
            trim_texture: trim_tex,
            pillar_texture: pillar_tex,
            liquid_texture: liquid_tex,
            sky_texture: sky_tex,
            song: song.clone(),
            primary_light_hue: dna.lighting.primary_hue,
            primary_light_sat: dna.lighting.primary_saturation,
            primary_light_bright: dna.lighting.avg_brightness.max(96),
            accent_light_hue: dna.lighting.accent_hue,
            accent_light_sat: dna.lighting.accent_saturation,
            ambient_brightness: dna.ambient_brightness.max(12),
            ambient_hue: dna.ambient_hue,
            ambient_saturation: dna.ambient_saturation,
            hazard_damage_per_sec: hazard_dps,
        };

        self.map_wizard_config.custom_dna_source_map = Some(dna.map_name.clone());
        self.map_wizard_config.custom_palette_override = Some(custom_palette);
        if !dna.ideal_player_count.is_empty() {
            self.map_wizard_config.ideal_player_count = dna.ideal_player_count.clone();
        }
        if !dna.gameplay.weapon_classes.is_empty() {
            self.map_wizard_config.custom_weapon_loadout = dna
                .gameplay
                .weapon_classes
                .iter()
                .map(|(cls, _)| cls.clone())
                .collect();
            self.map_wizard_config.weapon_preset = ut99_bsp::WizardWeaponPreset::Custom;
        }
        let mut cloned_powerups = Vec::new();
        for (cls, _) in &dna.gameplay.powerup_classes {
            if !cloned_powerups.contains(cls) {
                cloned_powerups.push(cls.clone());
            }
        }
        for (cls, _) in &dna.gameplay.health_armor_classes {
            if !cloned_powerups.contains(cls) {
                cloned_powerups.push(cls.clone());
            }
        }
        if !cloned_powerups.is_empty() {
            self.map_wizard_config.custom_powerup_loadout = cloned_powerups;
            self.map_wizard_config.powerup_preset = ut99_bsp::WizardPowerupPreset::Custom;
        }
        self.map_wizard_config.custom_decoration_loadout = dna
            .gameplay
            .decoration_classes
            .iter()
            .map(|(cls, _)| cls.clone())
            .collect();
        self.map_wizard_config.custom_ambient_sounds = dna.gameplay.ambient_sounds.clone();
        self.map_wizard_config.architectural_detail = if dna.architectural.semisolid_ratio_pct >= 30.0
            || dna.architectural.thin_trim_count >= 20
            || dna.total_brushes >= 200
        {
            ut99_bsp::WizardArchitecturalDetail::EpicHighDetail
        } else {
            ut99_bsp::WizardArchitecturalDetail::StandardTournament
        };

        self.log(format!(
            "🧬 Cloned Official UT99 Map DNA from '{}' (Title: '{}', Author: '{}', Wall: '{}', Floor: '{}', Song: '{}', {} weapon classes, {} decoration classes)",
            dna.map_name,
            dna.title,
            dna.author,
            wall_tex,
            floor_tex,
            song,
            dna.gameplay.weapon_classes.len(),
            dna.gameplay.decoration_classes.len()
        ));
        if dna.read_issues.is_empty() {
            self.toast_success(format!("Cloned DNA from {}", dna.map_name));
        } else {
            self.toast_warning(format!("Cloned DNA from {} with {} decoding warning(s); profile may be incomplete", dna.map_name, dna.read_issues.len()));
        }
        self.last_cloned_dna = Some(dna.clone());
        Ok(dna)
    }

    /// Scans all official `.unr` maps in `UnrealTournament/Maps`, computes empirical per-mode statistics,
    /// and caches the corpus profiles & stats in `self.official_corpus_profiles` / `self.official_corpus_stats`.
    pub fn analyze_official_map_corpus(&mut self) -> Result<usize, String> {
        let mut candidates = Vec::new();
        if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
            candidates.push(ut_dir.join("Maps"));
        }
        candidates.push(std::path::PathBuf::from("UnrealTournament/Maps"));
        candidates.push(std::path::PathBuf::from("../../UnrealTournament/Maps"));

        let mut profiles = Vec::new();
        let mut scan_failures = Vec::new();
        let mut excluded_count = 0;
        let mut generated_count = 0;
        for dir in candidates {
            if dir.exists() && dir.is_dir() {
                match ut99_format::unr::scan_maps_directory_report(&dir) {
                    Ok(report) if !report.profiles.is_empty() => {
                        profiles = report.profiles;
                        scan_failures = report.failures;
                        excluded_count = report.excluded.len();
                        generated_count = report.generated.len();
                        break;
                    }
                    Ok(report) => {
                        for (filename, error) in report.failures {
                            self.log(format!("Corpus scan failed for '{}': {}", filename, error));
                        }
                    }
                    Err(error) => self.log(format!("Corpus scan could not read {:?}: {error}", dir)),
                }
            }
        }
        if profiles.is_empty() {
            return Err("No official .unr maps found in UnrealTournament/Maps".to_string());
        }
        let partial_count = profiles.iter().filter(|p| !p.read_issues.is_empty()).count();
        let stats = ut99_format::summarize_corpus_by_mode(&profiles);
        let count = profiles.len();
        let failure_count = scan_failures.len();
        for (filename, error) in &scan_failures {
            self.log(format!("Corpus scan failed for '{}': {}", filename, error));
        }
        if excluded_count > 0 {
            self.log(format!("Corpus scan excluded {excluded_count} utility map(s)."));
        }
        if generated_count > 0 {
            self.log(format!(
                "Corpus scan skipped {generated_count} wizard-generated map(s); generated maps are never used as reference."
            ));
        }
        self.official_corpus_profiles = profiles;
        self.official_corpus_stats = stats;
        self.log(format!(
            "📊 Corpus scan: {} profiles across {} game modes ({} with decoding warnings, {} read failures, {} utility maps excluded).",
            count,
            self.official_corpus_stats.len(),
            partial_count,
            failure_count,
            excluded_count
        ));
        if partial_count > 0 || failure_count > 0 {
            self.toast_warning(format!("Analyzed {count} maps; {partial_count} partial profile(s), {failure_count} read failure(s)"));
        } else {
            self.toast_success(format!("Analyzed {count} map profiles"));
        }
        Ok(count)
    }

    /// Formats a Markdown research report summarizing the analyzed official UT99 map corpus.
    pub fn format_official_corpus_markdown_report(&self) -> Option<String> {
        if self.official_corpus_profiles.is_empty() || self.official_corpus_stats.is_empty() {
            return None;
        }
        Some(ut99_format::format_corpus_summary_markdown(
            &self.official_corpus_profiles,
            &self.official_corpus_stats,
        ))
    }

    /// Adapts the Wizard's game mode, canonical archetype, arena scale, procedural room parameters,
    /// and subsystem toggles to match the forensic architectural footprint of `self.last_cloned_dna`.
    pub fn apply_cloned_dna_topology_and_mode(&mut self) -> bool {
        let Some(dna) = self.last_cloned_dna.clone() else {
            return false;
        };

        // 1. Infer Game Mode from prefix and ideal player count
        let upper_name = dna.map_name.to_ascii_uppercase();
        let mode = match dna.mode_prefix.to_ascii_uppercase().as_str() {
            "CTF" => ut99_bsp::WizardGameMode::CaptureTheFlag,
            "DOM" => ut99_bsp::WizardGameMode::Domination,
            "AS" => ut99_bsp::WizardGameMode::Assault,
            _ => {
                if upper_name.contains("1ON1") || dna.ideal_player_count.trim() == "2" {
                    ut99_bsp::WizardGameMode::Duel1v1
                } else {
                    ut99_bsp::WizardGameMode::DeathMatch
                }
            }
        };
        self.map_wizard_config.game_mode = mode;

        // 2. Infer Canonical Archetype from geometry & zone topology
        let max_xy = dna.world_extent.x.max(dna.world_extent.y);
        let archetype = if mode == ut99_bsp::WizardGameMode::CaptureTheFlag
            || dna.gameplay.flag_base_count >= 2
        {
            ut99_bsp::WizardArchetype::SymmetricalTwinBases
        } else if matches!(
            mode,
            ut99_bsp::WizardGameMode::Domination | ut99_bsp::WizardGameMode::Assault
        ) {
            ut99_bsp::WizardArchetype::InterconnectedComplex
        } else if upper_name.contains("MORPHEUS")
            || upper_name.contains("PHOBOS")
            || upper_name.contains("SPACENOXX")
            || upper_name.contains("HYPERBLAST")
            || (dna.lighting.has_sky_zone
                && !dna.textures.sky_textures.is_empty()
                && dna.world_extent.z > 2200.0)
        {
            ut99_bsp::WizardArchetype::OrbitalOutpostSkybox
        } else if dna.subtract_brushes <= 14
            || max_xy <= 2600.0
            || upper_name.contains("MORBIAS")
            || upper_name.contains("FRACTAL")
            || upper_name.contains("FETID")
        {
            ut99_bsp::WizardArchetype::CentralArenaMultiTier
        } else {
            ut99_bsp::WizardArchetype::InterconnectedComplex
        };
        self.map_wizard_config.archetype = archetype;

        // 3. Infer Arena Scale from world bounding box extent
        self.map_wizard_config.scale = if max_xy < 2400.0 {
            ut99_bsp::WizardMapScale::Compact
        } else if max_xy > 4800.0 {
            ut99_bsp::WizardMapScale::Large
        } else {
            ut99_bsp::WizardMapScale::Standard
        };

        // 4. Infer Procedural Parameters & Symmetry
        self.map_wizard_config.procedural_room_count = (dna.subtract_brushes / 4).clamp(3, 10);
        self.map_wizard_config.procedural_vertical_tiers =
            ((dna.world_extent.z / 600.0).round() as usize).clamp(1, 3);
        self.map_wizard_config.symmetry_mode = if mode == ut99_bsp::WizardGameMode::CaptureTheFlag {
            ut99_bsp::WizardSymmetryMode::Rotational180
        } else {
            ut99_bsp::WizardSymmetryMode::None
        };

        // 5. Align Subsystem Toggles with forensic evidence
        self.map_wizard_config.toggles.include_lighting = dna.lighting.light_count > 0;
        self.map_wizard_config.toggles.include_skybox =
            dna.lighting.has_sky_zone || !dna.textures.sky_textures.is_empty();
        self.map_wizard_config.toggles.include_elevators_and_movers =
            dna.mover_count > 0 || dna.gameplay.lift_center_count > 0;
        self.map_wizard_config.toggles.include_environmental_hazards = (dna.lighting.water_zone_count
            + dna.lighting.lava_zone_count
            + dna.lighting.slime_zone_count)
            > 0
            || !dna.textures.liquid_and_portal_textures.is_empty();
        self.map_wizard_config
            .toggles
            .include_decorations_and_ambience = !dna.gameplay.decoration_classes.is_empty()
            || !dna.gameplay.ambient_sounds.is_empty();
        self.map_wizard_config.toggles.include_jump_pads = dna.gameplay.kicker_count > 0
            || matches!(
                archetype,
                ut99_bsp::WizardArchetype::OrbitalOutpostSkybox
                    | ut99_bsp::WizardArchetype::CentralArenaMultiTier
                    | ut99_bsp::WizardArchetype::VerticalTowerShaft
            );
        self.map_wizard_config.toggles.include_teleporters = dna.gameplay.teleporter_count >= 2;
        self.map_wizard_config.toggles.include_ramps_and_bridges = true;

        self.map_wizard_config.sync_mode_conventions();
        self.log(format!(
            "🏛 Matched Wizard Topology to Official Map '{}': Mode={}, Archetype={}, Scale={}, Rooms={}",
            dna.map_name,
            self.map_wizard_config.game_mode.label(),
            self.map_wizard_config.archetype.label(),
            self.map_wizard_config.scale.label(),
            self.map_wizard_config.procedural_room_count
        ));
        self.toast_info(format!(
            "Matched Mode & Topology from {}",
            dna.map_name
        ));
        true
    }

    /// Formats a comprehensive Markdown forensic report for `self.last_cloned_dna`.
    pub fn format_cloned_dna_markdown_report(&self) -> Option<String> {
        let dna = self.last_cloned_dna.as_ref()?;
        let sub_pct = if dna.total_brushes > 0 {
            dna.subtract_brushes as f32 * 100.0 / dna.total_brushes as f32
        } else {
            0.0
        };
        let ss_pct = if dna.total_brushes > 0 {
            dna.semisolid_brushes as f32 * 100.0 / dna.total_brushes as f32
        } else {
            0.0
        };
        let top_weapons: Vec<String> = dna
            .gameplay
            .weapon_classes
            .iter()
            .take(6)
            .map(|(w, c)| format!("{w} (x{c})"))
            .collect();
        let top_decos: Vec<String> = dna
            .gameplay
            .decoration_classes
            .iter()
            .take(6)
            .map(|(d, c)| format!("{d} (x{c})"))
            .collect();
        let top_specials: Vec<String> = dna
            .gameplay
            .trigger_and_special_classes
            .iter()
            .take(6)
            .map(|(s, c)| format!("{s} (x{c})"))
            .collect();
        let top_pkgs: Vec<String> = dna
            .textures
            .texture_packages
            .iter()
            .take(6)
            .map(|(p, c)| format!("{p}.utx ({c} polys)"))
            .collect();

        Some(format!(
            "# Forensic UT99 Map DNA Report: `{}`\n\
             - **Title:** {}\n\
             - **Author:** {}\n\
             - **Mode Prefix:** `{}` (`{}`)\n\
             - **Ideal Player Count:** `{}`\n\
             - **Music Track (.umx):** `{}`\n\
             \n\
             ## 1. CSG & BSP Architecture\n\
             - **Total Brushes:** {} (Subtract: {} [{:.1}%], Add: {}, Semi-Solid: {} [{:.1}%], Portals: {})\n\
             - **Movers:** {}\n\
             - **Compiled BSP:** {} Nodes, {} Surfs, {} Zones\n\
             - **World Extent:** {:.0} × {:.0} × {:.0} UU\n\
             \n\
             ## 2. Material & Texture Palette\n\
             - **Dominant Packages:** {}\n\
             - **Primary Wall:** `{}`\n\
             - **Primary Floor:** `{}`\n\
             - **Primary Ceiling:** `{}`\n\
             - **Primary Trim:** `{}`\n\
             - **Primary Sky:** `{}`\n\
             - **Primary Liquid:** `{}`\n\
             \n\
             ## 3. Lighting & Atmosphere\n\
             - **Light Actors:** {}\n\
             - **Primary Light HSV:** Hue={}, Sat={}, AvgBrightness={}, AvgRadius={}\n\
             - **Accent Light HSV:** Hue={}, Sat={}\n\
             - **LevelInfo Ambient HSV:** Brightness={}, Hue={}, Sat={}\n\
             - **Zones:** SkyZone={}, Water={}, Lava={}, Slime={}\n\
             \n\
             ## 4. Gameplay, Props & Bot AI Navigation\n\
             - **PlayerStarts:** {}\n\
             - **Navigation:** {} PathNodes, {} InventorySpots, {} ReachSpecs\n\
             - **Vertical / Special Transit:** {} LiftCenters, {} LiftExits, {} Teleporters, {} Kickers\n\
             - **Team Objectives:** {} FlagBases, {} ControlPoints, {} FortStandards, {} DefensePoints, {} AlternatePaths\n\
             - **Weapon Loadout:** {}\n\
             - **3D Decorations & Props:** {}\n\
             - **Special Mechanics & Triggers:** {}\n\
             - **Ambient Sound Cues:** {}\n\
             \n\
             ## 5. Architectural Detail & SemiSolid Morphology\n\
             - **SemiSolid Ratio:** {:.1}%\n\
             - **Niches & Recessed Coffers:** {}\n\
             - **Architectural Trims & Beams:** {} thin trims, {} pillars, {} structural beams\n\
             - **Sloped & Angled Surfaces:** {} sloped subtract walls\n\
             - **Multi-Texture Chamber Ratio:** {:.1}%\n\
             - **Recognized Zone Names:** {}\n\
             \n\
             ## 6. Skybox & Atmosphere Dynamics\n\
             - **Skybox Present:** {} (Rotating/Rocking: {})\n\
             - **Skybox Packages:** `{}` (Texture: `{}`)\n\
             - **Cloud Pan Layer:** `{}`\n\
             - **Nebula / Celestial Backdrop:** `{}`\n\
             - **Lens Flare Coronas:** {} corona lights (Top Skin: `{}`)\n\
             \n\
             ## 7. Verticality & Tactical Flow\n\
             - **Playable Vertical Span:** {:.0} UU (across {} distinct Z tiers)\n\
             - **Spawn to Nearest Weapon:** {:.0} UU\n\
             - **Major Powerup Minimum Separation:** {:.0} UU\n\
             - **Mover Initial States:** {} StandOpenTimed, {} TriggerOpenTimed, {} TriggerControl\n",
            dna.map_name,
            dna.title,
            dna.author,
            dna.mode_prefix,
            dna.default_game_type,
            dna.ideal_player_count,
            dna.song,
            dna.total_brushes,
            dna.subtract_brushes,
            sub_pct,
            dna.add_brushes,
            dna.semisolid_brushes,
            ss_pct,
            dna.portal_brushes,
            dna.mover_count,
            dna.bsp_node_count,
            dna.bsp_surf_count,
            dna.zone_count,
            dna.world_extent.x,
            dna.world_extent.y,
            dna.world_extent.z,
            top_pkgs.join(", "),
            dna.textures.primary_wall("None"),
            dna.textures.primary_floor("None"),
            dna.textures.primary_ceiling("None"),
            dna.textures.primary_trim("None"),
            dna.textures.primary_sky("None"),
            dna.textures.primary_liquid("None"),
            dna.lighting.light_count,
            dna.lighting.primary_hue,
            dna.lighting.primary_saturation,
            dna.lighting.avg_brightness,
            dna.lighting.avg_radius,
            dna.lighting.accent_hue,
            dna.lighting.accent_saturation,
            dna.ambient_brightness,
            dna.ambient_hue,
            dna.ambient_saturation,
            dna.lighting.has_sky_zone,
            dna.lighting.water_zone_count,
            dna.lighting.lava_zone_count,
            dna.lighting.slime_zone_count,
            dna.gameplay.player_start_count,
            dna.gameplay.path_node_count,
            dna.gameplay.inventory_spot_count,
            dna.reach_spec_count,
            dna.gameplay.lift_center_count,
            dna.gameplay.lift_exit_count,
            dna.gameplay.teleporter_count,
            dna.gameplay.kicker_count,
            dna.gameplay.flag_base_count,
            dna.gameplay.control_point_count,
            dna.gameplay.fort_standard_count,
            dna.gameplay.defense_point_count,
            dna.gameplay.alternate_path_count,
            if top_weapons.is_empty() { "None".to_string() } else { top_weapons.join(", ") },
            if top_decos.is_empty() { "None".to_string() } else { top_decos.join(", ") },
            if top_specials.is_empty() { "None".to_string() } else { top_specials.join(", ") },
            if dna.gameplay.ambient_sounds.is_empty() { "None".to_string() } else { dna.gameplay.ambient_sounds.join(", ") },
            dna.architectural.semisolid_ratio_pct,
            dna.architectural.niche_coffer_count,
            dna.architectural.thin_trim_count,
            dna.architectural.pillar_count,
            dna.architectural.beam_count,
            dna.architectural.sloped_wall_count,
            dna.architectural.multi_tex_sub_ratio_pct,
            if dna.architectural.named_zones.is_empty() { "None".to_string() } else { dna.architectural.named_zones.join(", ") },
            dna.skybox.has_skybox,
            dna.skybox.has_rotating_or_rocking_sky,
            if dna.skybox.primary_sky_package.is_empty() { "None".to_string() } else { dna.skybox.primary_sky_package.clone() },
            if dna.skybox.primary_sky_texture.is_empty() { "None".to_string() } else { dna.skybox.primary_sky_texture.clone() },
            if dna.skybox.cloud_pan_texture.is_empty() { "None".to_string() } else { dna.skybox.cloud_pan_texture.clone() },
            if dna.skybox.nebula_or_planet_texture.is_empty() { "None".to_string() } else { dna.skybox.nebula_or_planet_texture.clone() },
            dna.lighting.corona_light_count,
            dna.lighting.top_corona_skins.first().map(|(s, _)| s.as_str()).unwrap_or("None"),
            dna.verticality.playable_z_span,
            dna.verticality.distinct_z_tiers,
            dna.verticality.avg_spawn_to_weapon_dist,
            dna.verticality.min_major_powerup_separation,
            dna.verticality.stand_open_mover_count,
            dna.verticality.trigger_open_mover_count,
            dna.verticality.trigger_control_mover_count
        ))
    }

    /// Applies one of the 6 curated one-click Map Generator Wizard presets (`0..=5`),
    /// generating a fresh procedural map name and title on every invocation.
    pub fn apply_map_wizard_quick_preset(&mut self, preset_idx: usize) {
        match preset_idx {
            0 => {
                // 100% Playtest Ready
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles::full_playtest_ready();
                self.map_wizard_config.auto_run_build_and_audit = true;
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: 100% Playtest Ready ({name})"));
            }
            1 => {
                // Clean Structural Blockout
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles {
                    include_lighting: true,
                    include_skybox: false,
                    include_weapons_and_pickups: false,
                    include_bot_ai_paths: true,
                    include_elevators_and_movers: false,
                    include_environmental_hazards: false,
                    include_decorations_and_ambience: false,
                    include_jump_pads: false,
                    include_ramps_and_bridges: false,
                    include_teleporters: false,
                    ensure_vertical_reachability: true,
                };
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: Clean Structural Blockout ({name})"));
            }
            2 => {
                // 1v1 Tournament Duel
                self.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::Duel1v1;
                self.map_wizard_config.layout_strategy =
                    ut99_bsp::WizardLayoutStrategy::CanonicalArchetype;
                self.map_wizard_config.archetype = ut99_bsp::WizardArchetype::CrucibleDuelPit;
                self.map_wizard_config.scale = ut99_bsp::WizardMapScale::Compact;
                self.map_wizard_config.theme = ut99_bsp::WizardThemePreset::Deck16Industrial;
                self.map_wizard_config.custom_dna_source_map = None;
                self.map_wizard_config.custom_palette_override = None;
                self.map_wizard_config.ideal_player_count = "2".to_string();
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles::full_playtest_ready();
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: 1v1 Tournament Duel ({name})"));
            }
            3 => {
                // Symmetrical CTF Citadel
                self.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::CaptureTheFlag;
                self.map_wizard_config.layout_strategy =
                    ut99_bsp::WizardLayoutStrategy::CanonicalArchetype;
                self.map_wizard_config.archetype = ut99_bsp::WizardArchetype::SymmetricalTwinBases;
                self.map_wizard_config.scale = ut99_bsp::WizardMapScale::Standard;
                self.map_wizard_config.theme = ut99_bsp::WizardThemePreset::FacingWorldsCitadel;
                self.map_wizard_config.custom_dna_source_map = None;
                self.map_wizard_config.custom_palette_override = None;
                self.map_wizard_config.ideal_player_count = "8-12".to_string();
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles::full_playtest_ready();
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: Symmetrical CTF Citadel ({name})"));
            }
            4 => {
                // Domination 3-Point Complex
                self.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::Domination;
                self.map_wizard_config.layout_strategy =
                    ut99_bsp::WizardLayoutStrategy::CanonicalArchetype;
                self.map_wizard_config.archetype = ut99_bsp::WizardArchetype::ThreeWayRingArena;
                self.map_wizard_config.scale = ut99_bsp::WizardMapScale::Standard;
                self.map_wizard_config.theme = ut99_bsp::WizardThemePreset::SesmarEgyptianTomb;
                self.map_wizard_config.custom_dna_source_map = None;
                self.map_wizard_config.custom_palette_override = None;
                self.map_wizard_config.ideal_player_count = "6-10".to_string();
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles::full_playtest_ready();
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: Domination 3-Point Complex ({name})"));
            }
            5 => {
                // Assault Sequential Fortress
                self.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::Assault;
                self.map_wizard_config.layout_strategy =
                    ut99_bsp::WizardLayoutStrategy::CanonicalArchetype;
                self.map_wizard_config.archetype = ut99_bsp::WizardArchetype::FortressSiegeLine;
                self.map_wizard_config.scale = ut99_bsp::WizardMapScale::Large;
                self.map_wizard_config.theme = ut99_bsp::WizardThemePreset::CodexGothicCrypt;
                self.map_wizard_config.custom_dna_source_map = None;
                self.map_wizard_config.custom_palette_override = None;
                self.map_wizard_config.ideal_player_count = "8-14".to_string();
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles::full_playtest_ready();
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: Assault Sequential Fortress ({name})"));
            }
            6 => {
                // Tournament DeathMatch Standard
                self.map_wizard_config.game_mode = ut99_bsp::WizardGameMode::DeathMatch;
                self.map_wizard_config.layout_strategy =
                    ut99_bsp::WizardLayoutStrategy::CanonicalArchetype;
                self.map_wizard_config.archetype = ut99_bsp::WizardArchetype::InterconnectedComplex;
                self.map_wizard_config.scale = ut99_bsp::WizardMapScale::Standard;
                self.map_wizard_config.theme = ut99_bsp::WizardThemePreset::Deck16Industrial;
                self.map_wizard_config.custom_dna_source_map = None;
                self.map_wizard_config.custom_palette_override = None;
                self.map_wizard_config.ideal_player_count = "4-8".to_string();
                self.map_wizard_config.toggles = ut99_bsp::WizardFeatureToggles::full_playtest_ready();
                self.map_wizard_config.randomize_map_identity();
                let name = self.map_wizard_config.map_name.clone();
                self.toast_info(format!("Preset: Tournament DeathMatch ({name})"));
            }
            _ => {}
        }
    }

    /// Randomly selects an installed official `.unr` map from `self.available_official_maps`
    /// and extracts its texture palette, lighting HSV, music, and actor DNA into `self.map_wizard_config`.
    pub fn randomize_official_map_dna(&mut self) -> bool {
        if self.available_official_maps.is_empty() {
            self.scan_official_maps_for_wizard();
        }
        if self.available_official_maps.is_empty() {
            self.toast_warning("No official .unr maps found on disk to randomize DNA from.");
            return false;
        }
        let entropy = ut99_bsp::next_wizard_entropy_seed();
        let mode_prefix = format!("{}-", self.map_wizard_config.game_mode.prefix());
        let matching_mode: Vec<String> = self
            .available_official_maps
            .iter()
            .filter(|m| {
                m.to_ascii_uppercase().starts_with(&mode_prefix)
                    && *m != &self.selected_official_dna_map
            })
            .cloned()
            .collect();

        let chosen = if !matching_mode.is_empty() && (self.map_wizard_lock_game_mode || (entropy % 100) < 85) {
            matching_mode[(entropy as usize) % matching_mode.len()].clone()
        } else {
            let mut idx = (entropy as usize) % self.available_official_maps.len();
            if self.available_official_maps.len() > 1
                && self.available_official_maps[idx] == self.selected_official_dna_map
            {
                idx = (idx + 1) % self.available_official_maps.len();
            }
            self.available_official_maps[idx].clone()
        };

        self.selected_official_dna_map = chosen.clone();
        match self.clone_dna_from_official_map(&chosen) {
            Ok(_) => true,
            Err(e) => {
                self.toast_error(e);
                false
            }
        }
    }

    /// Randomizes all customizable Map Generator Wizard parameters across Steps 1..4 in one click,
    /// respecting `self.map_wizard_lock_game_mode`.
    pub fn randomize_all_wizard_settings(&mut self) {
        self.map_wizard_config
            .randomize_all(self.map_wizard_lock_game_mode);
        self.map_wizard_archetype_filter = None;
        let layout_str = match self.map_wizard_config.layout_strategy {
            ut99_bsp::WizardLayoutStrategy::CanonicalArchetype => {
                self.map_wizard_config.archetype.label().to_string()
            }
            ut99_bsp::WizardLayoutStrategy::ProceduralSeed => format!(
                "Procedural Seed #{} ({} rooms)",
                self.map_wizard_config.seed, self.map_wizard_config.procedural_room_count
            ),
        };
        let msg = format!(
            "Randomized All: {} • {} • {} ({} weapons)",
            self.map_wizard_config.map_name,
            layout_str,
            self.map_wizard_config.theme.label(),
            self.map_wizard_config.active_weapons().len()
        );
        self.log(format!("⚄ {msg}"));
        self.toast_info(msg);
    }

    /// Generates a complete, valid, playtest-ready UT99 map from `self.map_wizard_config`
    /// directly into the active `EditorState` with full atomic Undo/Redo (`Action::ReplaceEntireLevel`),
    /// auto-loading referenced `.utx` packages and running `Build All` + `Level Doctor` verification.
    /// Returns `(brush_count, mover_count, actor_count)`.
    pub fn generate_map_from_wizard(&mut self) -> (usize, usize, usize) {
        self.generate_map_from_wizard_impl(false).0
    }

    /// Generate and run one pre-export audit regardless of the optional UI setting.
    /// The generated level remains editable if the audit fails, but the caller
    /// must not save or launch it through the verified workflow.
    pub fn generate_wizard_map_verified(&mut self) -> Result<(usize, usize, usize), Vec<String>> {
        let (counts, audit) = self.generate_map_from_wizard_impl(true);
        match audit {
            Some(Ok(_)) => Ok(counts),
            Some(Err(errors)) => Err(errors),
            None => Err(vec!["Pre-export audit was not run".to_string()]),
        }
    }

    fn generate_map_from_wizard_impl(&mut self, force_audit: bool) -> ((usize, usize, usize), Option<Result<usize, Vec<String>>>) {
        let expected_prefix = self.map_wizard_config.game_mode.prefix();
        let upper_map_name = self.map_wizard_config.map_name.to_ascii_uppercase();
        if !upper_map_name.starts_with(expected_prefix) {
            self.map_wizard_config.sync_mode_conventions();
        }

        let generated = ut99_bsp::generate_valid_ut99_map(&self.map_wizard_config);
        let generation_warnings = generated.warnings.clone();
        let map_name_str = self.map_wizard_config.map_name.clone();

        let old_map_path = self.map_path.clone();
        let old_launch_args = self.ut99_launch_args.clone();
        // 0. Synchronize map_path and playtest launch arguments with the generated map and game type
        let clean_fname = if map_name_str.to_ascii_lowercase().ends_with(".unr") {
            map_name_str.clone()
        } else {
            format!("{map_name_str}.unr")
        };
        let maps_dir = self.wizard_output_directory();
        let target_map_path = maps_dir.join(&clean_fname);
        self.map_path = Some(target_map_path.clone());

        let game_class = &generated.default_game_type;
        if !game_class.is_empty() {
            let game_param = format!("?game={game_class}");
            if let Some(pos) = self.ut99_launch_args.find("?game=") {
                let rest = &self.ut99_launch_args[pos..];
                let end = rest.find(|c: char| c.is_whitespace())
                    .map(|i| pos + i)
                    .unwrap_or(self.ut99_launch_args.len());
                let after = &self.ut99_launch_args[end..];
                self.ut99_launch_args = format!("{game_param}{after}");
            } else {
                self.ut99_launch_args = format!("{game_param} {}", self.ut99_launch_args.trim());
            }
        }

        let old_brushes = self.world_brushes.to_vec();
        let old_movers = self.movers.to_vec();
        let old_unr = self.loaded_unr.clone();
        let old_metadata = (
            self.level_title.clone(),
            self.level_author.clone(),
            self.level_ideal_player_count.clone(),
            self.level_song.clone(),
            self.level_game_type.clone(),
            self.level_enter_text.clone(),
            self.level_ambient_brightness,
            self.level_ambient_hue,
            self.level_ambient_saturation,
        );

        // 1. Update LevelInfo metadata
        self.level_title = self.map_wizard_config.title.clone();
        self.level_author = self.map_wizard_config.author.clone();
        self.level_ideal_player_count = self.map_wizard_config.ideal_player_count.clone();
        self.level_song = generated.song.clone();
        self.level_game_type = generated.default_game_type.clone();
        self.level_enter_text = self.map_wizard_config.enter_text.clone();
        self.level_ambient_brightness = generated.ambient_brightness;
        self.level_ambient_hue = generated.ambient_hue;
        self.level_ambient_saturation = generated.ambient_saturation;

        let new_metadata = (
            self.level_title.clone(),
            self.level_author.clone(),
            self.level_ideal_player_count.clone(),
            self.level_song.clone(),
            self.level_game_type.clone(),
            self.level_enter_text.clone(),
            self.level_ambient_brightness,
            self.level_ambient_hue,
            self.level_ambient_saturation,
        );

        // 2. Convert WizardMoverSpec -> EditorMover + ActorRecord
        let mut new_movers = Vec::with_capacity(generated.movers.len());
        let mut actors = Vec::with_capacity(generated.actors.len() + generated.movers.len() + 1);

        // LevelInfo0 actor record
        let mut level_info = ut99_format::ActorRecord::default();
        level_info.name = "LevelInfo0".to_string();
        level_info.class_name = "LevelInfo".to_string();
        level_info.properties.insert(
            "Title".to_string(),
            ut99_format::TaggedProperty {
                name: "Title".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Str(self.level_title.clone()),
            },
        );
        level_info.properties.insert(
            "Author".to_string(),
            ut99_format::TaggedProperty {
                name: "Author".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Str(self.level_author.clone()),
            },
        );
        level_info.properties.insert(
            "IdealPlayerCount".to_string(),
            ut99_format::TaggedProperty {
                name: "IdealPlayerCount".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Str(self.level_ideal_player_count.clone()),
            },
        );
        if !self.level_song.is_empty() {
            level_info.properties.insert(
                "Song".to_string(),
                ut99_format::TaggedProperty {
                    name: "Song".to_string(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Name(self.level_song.clone()),
                },
            );
        }
        level_info.properties.insert(
            "DefaultGameType".to_string(),
            ut99_format::TaggedProperty {
                name: "DefaultGameType".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Str(self.level_game_type.clone()),
            },
        );
        if !self.level_enter_text.is_empty() {
            level_info.properties.insert(
                "LevelEnterText".to_string(),
                ut99_format::TaggedProperty {
                    name: "LevelEnterText".to_string(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Str(self.level_enter_text.clone()),
                },
            );
        }
        level_info.properties.insert(
            "AmbientBrightness".to_string(),
            ut99_format::TaggedProperty {
                name: "AmbientBrightness".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(self.level_ambient_brightness),
            },
        );
        level_info.properties.insert(
            "AmbientHue".to_string(),
            ut99_format::TaggedProperty {
                name: "AmbientHue".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(self.level_ambient_hue),
            },
        );
        level_info.properties.insert(
            "AmbientSaturation".to_string(),
            ut99_format::TaggedProperty {
                name: "AmbientSaturation".to_string(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(self.level_ambient_saturation),
            },
        );
        actors.push(level_info);

        for m_spec in &generated.movers {
            let mut keyframes: [Option<MoverKeyframe>; 8] = Default::default();
            keyframes[0] = Some(MoverKeyframe {
                location: m_spec.key0_location,
                rotation: Vec3::ZERO,
            });
            keyframes[1] = Some(MoverKeyframe {
                location: m_spec.key1_location,
                rotation: Vec3::ZERO,
            });
            let ed_mover = EditorMover {
                name: m_spec.name.clone(),
                mover_class: "Mover".to_string(),
                brush: m_spec.brush.clone(),
                current_key: 0,
                keyframes,
                initial_state: m_spec.initial_state.clone(),
                stay_open_time: m_spec.stay_open_time,
                move_time: m_spec.move_time,
                delay_time: 0.0,
                glide_type: "MV_GlideByTime".to_string(),
                encroach_type: "ME_ReturnWhenEncroach".to_string(),
                tag: m_spec.tag.clone(),
                event: m_spec.event.clone(),
                opening_sound: m_spec.opening_sound.clone(),
                opened_sound: String::new(),
                closing_sound: String::new(),
                closed_sound: m_spec.closed_sound.clone(),
                move_ambient_sound: String::new(),
            };
            new_movers.push(ed_mover);

            let mut mover_act = ut99_format::ActorRecord::default();
            mover_act.name = m_spec.name.clone();
            mover_act.class_name = "Mover".to_string();
            mover_act.location = m_spec.key0_location;
            mover_act.tag = m_spec.tag.clone();
            mover_act.event = m_spec.event.clone();
            mover_act.group = "Movers".to_string();
            actors.push(mover_act);
        }

        // 3. Convert WizardActorSpec -> ActorRecord
        for a_spec in &generated.actors {
            let mut rec = ut99_format::ActorRecord::default();
            rec.name = a_spec.name.clone();
            rec.class_name = a_spec.class_name.clone();
            rec.location = a_spec.location;
            rec.rotation = a_spec.rotation;
            rec.tag = a_spec.tag.clone();
            rec.event = a_spec.event.clone();
            rec.group = a_spec.group.clone();
            rec.light_brightness = a_spec.light_brightness;
            rec.light_hue = a_spec.light_hue;
            rec.light_saturation = a_spec.light_saturation;
            rec.light_radius = a_spec.light_radius;
            if a_spec.class_name.eq_ignore_ascii_case("Light") {
                rec.light_type = 1; // LT_Steady
            }

            let parse_prop_target = |key: &str| -> (String, i32) {
                if let Some(open) = key.find('[') {
                    if key.ends_with(']') {
                        if let Ok(idx) = key[open + 1..key.len() - 1].parse::<i32>() {
                            return (key[..open].to_string(), idx);
                        }
                    }
                }
                if let Some(open) = key.find('(') {
                    if key.ends_with(')') {
                        if let Ok(idx) = key[open + 1..key.len() - 1].parse::<i32>() {
                            return (key[..open].to_string(), idx);
                        }
                    }
                }
                (key.to_string(), 0)
            };

            for (k, v) in &a_spec.int_props {
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Int(*v),
                    },
                );
            }
            for (k, v) in &a_spec.byte_props {
                if k.eq_ignore_ascii_case("LightEffect") {
                    rec.light_effect = *v;
                } else if k.eq_ignore_ascii_case("LightType") {
                    rec.light_type = *v;
                }
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Byte(*v),
                    },
                );
            }
            for (k, v) in &a_spec.float_props {
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Float(*v),
                    },
                );
            }
            for (k, v) in &a_spec.bool_props {
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Bool(*v),
                    },
                );
            }
            for (k, v) in &a_spec.name_props {
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Name(v.clone()),
                    },
                );
            }
            for (k, v) in &a_spec.str_props {
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Str(v.clone()),
                    },
                );
            }
            for (k, v) in &a_spec.vec_props {
                let (base, arr_idx) = parse_prop_target(k);
                rec.properties.insert(
                    k.clone(),
                    ut99_format::TaggedProperty {
                        name: base,
                        array_index: arr_idx,
                        value: ut99_format::TaggedValue::Vector(*v),
                    },
                );
            }

            actors.push(rec);
        }

        let mut level = ut99_format::UnrLevel {
            map_name: format!("{}.unr", map_name_str.trim_end_matches(".unr")),
            actors: actors.into(),
            ..Default::default()
        };
        level.sync_actor_views();

        let b_cnt = generated.brushes.len();
        let m_cnt = new_movers.len();
        let a_cnt = level.actors.len();

        self.world_brushes = generated.brushes.into();
        self.movers = new_movers.into();
        self.static_meshes.clear();
        self.loaded_unr = Some(level);
        self.map_instance_id = next_map_instance_id();
        self.map_path = Some(target_map_path);
        self.selected_brush_index = None;
        self.selected_actor_index = None;
        self.selected_mover_index = None;
        self.clear_multi_selection();
        self.sync_brush_groups_from_brushes();

        // 4. Auto-load referenced .utx packages from UnrealTournament/Textures if present
        let mut referenced_pkgs = std::collections::HashSet::new();
        for b in &self.world_brushes {
            for p in &b.polys {
                if let Some((pkg_name, _)) = p.texture.split_once('.') {
                    if !pkg_name.is_empty()
                        && !pkg_name.eq_ignore_ascii_case("MyLevel")
                        && !pkg_name.eq_ignore_ascii_case("Default")
                    {
                        referenced_pkgs.insert(pkg_name.to_string());
                    }
                }
            }
        }
        for pkg_name in referenced_pkgs {
            if self.texture_manager.get_package(&pkg_name).is_none() {
                let mut candidates = vec![
                    std::path::PathBuf::from("UnrealTournament/Textures").join(format!("{pkg_name}.utx")),
                    std::path::PathBuf::from("../../UnrealTournament/Textures").join(format!("{pkg_name}.utx")),
                ];
                if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
                    candidates.push(ut_dir.join("Textures").join(format!("{pkg_name}.utx")));
                }
                for cand in candidates {
                    if cand.exists() {
                        if let Ok(utx_pkg) = ut99_format::UtxPackage::open(&cand) {
                            self.texture_manager.insert_package(pkg_name.clone(), utx_pkg);
                            break;
                        }
                    }
                }
            }
        }

        // 5. Run Build All + Level Doctor Pre-Export Auto-Fix Verification if requested
        let audit_result = if force_audit || self.map_wizard_config.auto_run_build_and_audit {
            Some(self.audit_and_autofix_before_export_without_history())
        } else {
            self.refresh_ai_path_links();
            None
        };

        // 6. Push atomic ReplaceEntireLevel action to History AFTER build_ai_paths so InventorySpots are included in new_unr
        let new_brushes = self.world_brushes.to_vec();
        let new_movers = self.movers.to_vec();
        let new_unr = self.loaded_unr.clone();
        self.history.push(Action::ReplaceEntireLevel {
            old_map_path,
            new_map_path: self.map_path.clone(),
            old_launch_args,
            new_launch_args: self.ut99_launch_args.clone(),
            old_brushes,
            new_brushes,
            old_movers,
            new_movers,
            old_unr,
            new_unr,
            old_metadata,
            new_metadata,
            description: format!("Generate Valid UT99 Map '{}'", map_name_str),
        });

        // 7. Center camera on first PlayerStart
        if let Some(start_loc) = self
            .loaded_unr
            .as_ref()
            .and_then(|u| u.player_starts.first().map(|ps| ps.location))
        {
            self.camera.position = start_loc + Vec3::new(0.0, -320.0, 160.0);
            self.camera.yaw = 90.0f32.to_radians();
            self.camera.pitch = -15.0f32.to_radians();
        } else {
            self.camera.position = Vec3::new(0.0, -500.0, 240.0);
        }

        self.map_is_dirty = true;
        self.log(format!(
            "🧙 Map Generator Wizard: Created '{}' ({}) — {} brushes, {} movers, {} actors (ReachSpecs: {}).",
            map_name_str,
            self.map_wizard_config.game_mode.label(),
            b_cnt,
            m_cnt,
            a_cnt,
            self.nav_graph.reach_specs.len()
        ));
        if let Some(Err(errors)) = &audit_result {
            self.toast_warning(format!("Generated '{}' with {} audit error(s)", map_name_str, errors.len()));
        } else if !generation_warnings.is_empty() {
            self.toast_warning(format!("Generated '{}' with {} placement warning(s); see log", map_name_str, generation_warnings.len()));
        } else {
            self.toast_success(format!("Generated '{}' ({} brushes, {} actors)", map_name_str, b_cnt, a_cnt));
        }
        for warning in &generation_warnings {
            self.log(format!("Wizard placement warning: {warning}"));
        }

        ((b_cnt, m_cnt, a_cnt), audit_result)
    }

    pub fn generate_wizard_lift(&mut self) {
        let base_loc = self.builder_brush.location;
        let h = self.wizard_mover_height;
        let horiz = self.wizard_mover_horizontal;
        let delay = self.wizard_mover_delay;
        let tag = if self.wizard_mover_tag.trim().is_empty() {
            "Mover_01".to_string()
        } else {
            self.wizard_mover_tag.trim().to_string()
        };

        // 1. Setup platform builder brush (256 x 256 x 32)
        self.set_builder_cube(glam::Vec3::new(256.0, 256.0, 32.0));
        self.add_mover_from_builder();

        // 2. Configure mover keyframe 1
        if let Some(m_idx) = self.selected_mover_index {
            if let Some(m) = self.movers.get_mut(m_idx) {
                m.name = tag.clone();
                m.stay_open_time = delay;
                let delta = glam::Vec3::new(horiz, 0.0, h);
                m.keyframes[1] = Some(crate::editor_state::MoverKeyframe {
                    location: base_loc + delta,
                    rotation: m.brush.rotation,
                });
            }
        }

        // 3. Spawn Trigger actor with Event = tag
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let mut trigger = ut99_format::ActorRecord::default();
            trigger.name = format!("{}_Trigger", tag);
            trigger.class_name = "Trigger".into();
            trigger.location = base_loc + glam::Vec3::new(0.0, 0.0, 32.0);
            trigger.tag = format!("{}_Trigger", tag);
            trigger.event = tag.clone();
            trigger.properties.insert("CollisionRadius".into(), ut99_format::TaggedProperty {
                name: "CollisionRadius".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(64.0),
            });
            trigger.properties.insert("CollisionHeight".into(), ut99_format::TaggedProperty {
                name: "CollisionHeight".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(40.0),
            });
            unr.actors.push(trigger);
        }

        self.log(format!("Wizard: Generated Mover '{}' with 2 keyframes and Trigger at ({:.0}, {:.0}, {:.0})", tag, base_loc.x, base_loc.y, base_loc.z));
        self.toast_success(format!("Created Mover & Lift '{}'", tag));
    }

    pub fn generate_wizard_teleporters(&mut self) {
        let enter_loc = self.builder_brush.location;
        let exit_loc = enter_loc + glam::Vec3::new(self.wizard_teleport_offset_x, self.wizard_teleport_offset_y, self.wizard_teleport_offset_z);
        let enter_tag = if self.wizard_teleport_enter_tag.trim().is_empty() { "Teleport_A".into() } else { self.wizard_teleport_enter_tag.trim().to_string() };
        let exit_tag = if self.wizard_teleport_exit_tag.trim().is_empty() { "Teleport_B".into() } else { self.wizard_teleport_exit_tag.trim().to_string() };

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            // Entrance
            let mut ent = ut99_format::ActorRecord::default();
            ent.name = enter_tag.clone();
            ent.class_name = "Teleporter".into();
            ent.location = enter_loc;
            ent.tag = enter_tag.clone();
            ent.properties.insert("URL".into(), ut99_format::TaggedProperty {
                name: "URL".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Str(exit_tag.clone()),
            });
            ent.properties.insert("CollisionRadius".into(), ut99_format::TaggedProperty {
                name: "CollisionRadius".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(40.0),
            });
            ent.properties.insert("CollisionHeight".into(), ut99_format::TaggedProperty {
                name: "CollisionHeight".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(40.0),
            });
            unr.actors.push(ent);

            // Exit
            let mut ex = ut99_format::ActorRecord::default();
            ex.name = exit_tag.clone();
            ex.class_name = "Teleporter".into();
            ex.location = exit_loc;
            ex.tag = exit_tag.clone();
            ex.properties.insert("CollisionRadius".into(), ut99_format::TaggedProperty {
                name: "CollisionRadius".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(40.0),
            });
            ex.properties.insert("CollisionHeight".into(), ut99_format::TaggedProperty {
                name: "CollisionHeight".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(40.0),
            });
            unr.actors.push(ex);
        }

        self.log(format!("Wizard: Generated Teleporter Pair '{}' -> '{}'", enter_tag, exit_tag));
        self.toast_success(format!("Linked Teleporters: {} -> {}", enter_tag, exit_tag));
    }

    pub fn generate_wizard_water_basin(&mut self) {
        let w = self.wizard_basin_width;
        let l = self.wizard_basin_length;
        let d = self.wizard_basin_depth;
        let center = self.builder_brush.location;

        // 1. Subtract basin from world
        self.set_builder_cube(glam::Vec3::new(w, l, d));
        self.csg_subtract();

        // 2. Add water portal sheet (PF_PORTAL | PF_NOT_SOLID | PF_TRANSLUCENT)
        let sheet_z = center.z + (d * 0.5) - 4.0;
        let mut sheet_loc = center;
        sheet_loc.z = sheet_z;
        self.set_builder_sheet(w, l);
        self.builder_brush.location = sheet_loc;
        self.add_special_brush(ut99_bsp::PF_PORTAL | ut99_bsp::PF_NOT_SOLID | ut99_bsp::PF_TRANSLUCENT, false);

        // 3. Add WaterZone or SlimeZone or LavaZone actor inside basin
        let zone_class = match self.wizard_basin_type {
            1 => "SlimeZone",
            2 => "LavaZone",
            _ => "WaterZone",
        };
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let mut zone = ut99_format::ActorRecord::default();
            zone.name = format!("Zone_{}", zone_class);
            zone.class_name = zone_class.into();
            zone.location = center;
            unr.actors.push(zone);
        }

        self.log(format!("Wizard: Carved {} basin ({:.0}x{:.0}x{:.0}) with portal surface sheet", zone_class, w, l, d));
        self.toast_success(format!("Created {} Basin", zone_class));
    }

    pub fn generate_wizard_jump_pad(&mut self) {
        let loc = self.builder_brush.location;
        let h = self.wizard_jump_height;
        let dist = self.wizard_jump_dist;

        let gravity = 950.0_f32;
        let vz = (2.0 * gravity * h).sqrt();
        let time_apex = vz / gravity;
        let vx = if time_apex > 0.001 { dist / (time_apex * 2.0) } else { 0.0 };

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            // Kicker pad
            let mut kicker = ut99_format::ActorRecord::default();
            kicker.name = format!("UT_JumpPad_{}", unr.actors.len());
            kicker.class_name = "UT_JumpPad".into();
            kicker.location = loc;
            kicker.properties.insert("KickVelocity".into(), ut99_format::TaggedProperty {
                name: "KickVelocity".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Vector(glam::Vec3::new(vx, 0.0, vz)),
            });
            kicker.properties.insert("CollisionRadius".into(), ut99_format::TaggedProperty {
                name: "CollisionRadius".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(48.0),
            });
            kicker.properties.insert("CollisionHeight".into(), ut99_format::TaggedProperty {
                name: "CollisionHeight".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(16.0),
            });
            unr.actors.push(kicker);

            // Target landing PathNode
            let mut target = ut99_format::ActorRecord::default();
            target.name = format!("JumpTarget_{}", unr.actors.len());
            target.class_name = "PathNode".into();
            target.location = loc + glam::Vec3::new(dist, 0.0, 0.0);
            unr.actors.push(target);
        }

        self.log(format!("Wizard: Placed JumpPad with KickVelocity ({:.0}, 0, {:.0}) for {:.0}uu jump", vx, vz, dist));
        self.toast_success("Placed JumpPad & Landing Target");
    }

    pub fn generate_wizard_room(&mut self) {
        let w = self.wizard_room_width;
        let l = self.wizard_room_length;
        let h = self.wizard_room_height;
        let loc = self.builder_brush.location;

        // 1. Subtract room
        self.set_builder_cube(glam::Vec3::new(w, l, h));
        self.csg_subtract();

        // 2. Lights
        if self.wizard_room_lights == 1 {
            // Center chandelier
            let mut light_loc = loc;
            light_loc.z += (h * 0.5) - 32.0;
            self.builder_brush.location = light_loc;
            self.add_light_at_builder();
        } else if self.wizard_room_lights == 2 {
            // 4 corner lights
            let off_x = (w * 0.5) - 128.0;
            let off_y = (l * 0.5) - 128.0;
            let light_z = loc.z + (h * 0.5) - 32.0;
            let offsets = [
                (off_x, off_y),
                (-off_x, off_y),
                (-off_x, -off_y),
                (off_x, -off_y),
            ];
            for (ox, oy) in offsets {
                self.builder_brush.location = glam::Vec3::new(loc.x + ox, loc.y + oy, light_z);
                self.add_light_at_builder();
            }
        }

        self.builder_brush.location = loc;
        self.log(format!("Wizard: Carved arena room ({:.0}x{:.0}x{:.0}) with lights", w, l, h));
        self.toast_success("Carved Arena Room");
    }

    pub fn spawn_gameplay_station_preset(&mut self, station_type: &str) {
        match station_type {
            "HealthAndArmor" => {
                self.add_weapon_or_pickup_at_builder("Armor2");
                let loc = self.builder_brush.location;
                self.builder_brush.location = loc + glam::Vec3::new(64.0, 0.0, 0.0);
                self.add_weapon_or_pickup_at_builder("MedBox");
                self.builder_brush.location = loc + glam::Vec3::new(-64.0, 0.0, 0.0);
                self.add_weapon_or_pickup_at_builder("MedBox");
                self.builder_brush.location = loc;
                self.toast_success("Spawned MedKit & Armor Station");
            }
            "FlakStation" => {
                self.add_weapon_or_pickup_at_builder("UT_FlakCannon");
                let loc = self.builder_brush.location;
                self.builder_brush.location = loc + glam::Vec3::new(48.0, 0.0, 0.0);
                self.add_weapon_or_pickup_at_builder("FlakAmmo");
                self.builder_brush.location = loc;
                self.toast_success("Spawned Flak Cannon & Ammo");
            }
            "UDamageAltar" => {
                self.add_weapon_or_pickup_at_builder("UDamage");
                self.add_light_at_builder();
                if let Some(ref mut unr) = self.loaded_unr {
                    if let Some(actor) = unr.actors.last_mut() {
                        actor.light_hue = 190; // Deep purple
                        actor.light_saturation = 10;
                        actor.light_brightness = 200;
                        actor.light_radius = 32;
                    }
                }
                self.toast_success("Spawned UDamage Altar");
            }
            _ => {}
        }
    }


}


impl EditorState {
    pub fn apply_symmetrical_ctf_base(&mut self, config: &CtfSymmetryConfig) -> CtfSymmetryResult {
        let result = generate_symmetrical_ctf_base(&self.world_brushes, config);
        let count = result.generated_brushes.len();
        for b in result.generated_brushes.clone() {
            self.world_brushes.push(b);
        }
        self.map_is_dirty = true;
        self.log(format!(
            "CTF Base Generated: {} symmetrical brushes added, {} textures remapped.",
            count,
            result.remapped_texture_polys_count
        ));
        result
    }

    /// Adds a user-defined custom room from 2D minimap interaction.
    pub fn add_wizard_custom_room(&mut self, center_x: f32, center_y: f32) -> String {
        let count = self.map_wizard_config.custom_rooms.len() + 1;
        let id = format!("cr_{count}");
        let name = format!("CustomRoom_{count}");
        let size = Vec3::new(640.0, 640.0, 512.0);
        let center = Vec3::new(center_x, center_y, size.z * 0.5);
        let room = ut99_bsp::WizardCustomRoom {
            id,
            name: name.clone(),
            center,
            size,
            is_octagonal: false,
            has_hazard_pit: false,
            sky_ceiling: false,
            team_owner: None,
            environment: ut99_bsp::WizardRoomEnvironment::Normal,
        };
        self.map_wizard_config.custom_rooms.push(room);
        self.wizard_minimap_selected = Some(crate::editor_state::WizardMinimapSelection::Room(name.clone()));
        self.toast_info(format!("Added custom room: {name}"));
        name
    }

    /// Connects two rooms via an orthogonal custom corridor from 2D minimap interaction.
    pub fn add_wizard_custom_connection(&mut self, from_room: &str, to_room: &str) -> Option<String> {
        if from_room == to_room {
            self.toast_warning("Cannot connect a room to itself");
            return None;
        }
        let exists = self.map_wizard_config.custom_connections.iter().any(|c| {
            (c.from_room == from_room && c.to_room == to_room)
                || (c.from_room == to_room && c.to_room == from_room)
        });
        if exists {
            self.toast_warning("Connection already exists between these rooms");
            return None;
        }
        let count = self.map_wizard_config.custom_connections.len() + 1;
        let id = format!("cc_{count}");
        self.map_wizard_config.custom_connections.push(ut99_bsp::WizardCustomConnection {
            id: id.clone(),
            from_room: from_room.to_string(),
            to_room: to_room.to_string(),
            width: 288.0,
            height: 320.0,
            has_blast_door: false,
        });
        self.wizard_minimap_selected = Some(crate::editor_state::WizardMinimapSelection::CustomConnection(id.clone()));
        self.toast_info(format!("Connected {from_room} ⟷ {to_room}"));
        Some(id)
    }

    /// Deletes the currently selected custom room, custom connection, or gameplay marker on the 2D minimap.
    pub fn delete_wizard_selection(&mut self) {
        match &self.wizard_minimap_selected {
            Some(crate::editor_state::WizardMinimapSelection::Room(name)) => {
                let name = name.clone();
                if let Some(pos) = self.map_wizard_config.custom_rooms.iter().position(|r| r.name == name) {
                    self.map_wizard_config.custom_rooms.remove(pos);
                    self.map_wizard_config.custom_connections.retain(|c| c.from_room != name && c.to_room != name);
                    self.wizard_minimap_selected = None;
                    self.toast_info(format!("Deleted custom room: {name}"));
                } else {
                    self.toast_warning("Base archetype rooms cannot be deleted; use Clear Overrides to reset custom geometry.");
                }
            }
            Some(crate::editor_state::WizardMinimapSelection::CustomConnection(id)) => {
                let id = id.clone();
                if let Some(pos) = self.map_wizard_config.custom_connections.iter().position(|c| c.id == id) {
                    self.map_wizard_config.custom_connections.remove(pos);
                    self.wizard_minimap_selected = None;
                    self.toast_info("Deleted custom corridor connection");
                }
            }
            Some(crate::editor_state::WizardMinimapSelection::Marker(label)) => {
                let label = label.clone();
                if let Some(pos) = self.map_wizard_config.custom_markers.iter().position(|m| &m.label == &label || &m.id == &label) {
                    self.map_wizard_config.custom_markers.remove(pos);
                    self.wizard_minimap_selected = None;
                    self.toast_info(format!("Deleted custom marker: {label}"));
                } else {
                    if !self.map_wizard_config.suppressed_markers.contains(&label) {
                        self.map_wizard_config.suppressed_markers.push(label.clone());
                    }
                    self.wizard_minimap_selected = None;
                    self.toast_info(format!("Deleted marker: {label}"));
                }
            }
            None => {}
        }
    }

    /// Adds a new user-placed gameplay object/marker at world coordinates (x, y) on the 2D minimap.
    pub fn add_wizard_custom_marker(&mut self, kind: &str, x: f32, y: f32) -> String {
        let count = self.map_wizard_config.custom_markers.len() + 1;
        let id = format!("obj_{count}");
        let label = format!("{kind}_{count}");
        let class_name = match kind {
            "PlayerStart" => "Engine.PlayerStart",
            "ShockRifle" => "Botpack.ShockRifle",
            "FlakCannon" => "Botpack.UT_FlakCannon",
            "RocketLauncher" => "Botpack.UT_Eightball",
            "SniperRifle" => "Botpack.SniperRifle",
            "Minigun" => "Botpack.minigun2",
            "PulseGun" => "Botpack.PulseGun",
            "BioRifle" => "Botpack.UT_BioRifle",
            "Enforcer" => "Botpack.Enforcer",
            "Health" => "Botpack.MedBox",
            "HealthVial" => "Botpack.HealthVial",
            "Armor" => "Botpack.Armor2",
            "ShieldBelt" => "Botpack.UT_ShieldBelt",
            "UDamage" => "Botpack.UDamage",
            "Light" => "Engine.Light",
            "Teleporter" => "Engine.Teleporter",
            _ => "Botpack.ShockRifle",
        }
        .to_string();

        let (teleporter_tag, teleporter_url) = if kind == "Teleporter" {
            let tele_tag = format!("{label}_Tag");
            // Check if there is an existing teleporter without a destination URL, auto-pair them!
            let mut auto_url = String::new();
            for existing in self.map_wizard_config.custom_markers.iter_mut() {
                if existing.kind == "Teleporter" && existing.teleporter_url.trim().is_empty() {
                    existing.teleporter_url = tele_tag.clone();
                    auto_url = existing.teleporter_tag.clone();
                    break;
                }
            }
            (tele_tag, auto_url)
        } else {
            (String::new(), String::new())
        };

        let marker = ut99_bsp::WizardCustomMarker {
            id: id.clone(),
            label: label.clone(),
            kind: kind.to_string(),
            class_name,
            location: glam::Vec3::new(x, y, 48.0),
            team: None,
            teleporter_tag,
            teleporter_url,
        };
        self.map_wizard_config.custom_markers.push(marker);
        self.wizard_minimap_selected = Some(crate::editor_state::WizardMinimapSelection::Marker(label.clone()));
        self.toast_info(format!("Placed {kind} at ({:.0}, {:.0})", x, y));
        label
    }

    /// Moves a gameplay marker to a new world position on the 2D minimap.
    pub fn move_wizard_marker(&mut self, marker_label: &str, new_loc: glam::Vec3) {
        if let Some(cm) = self.map_wizard_config.custom_markers.iter_mut().find(|m| m.label == marker_label || m.id == marker_label) {
            cm.location = new_loc;
        } else {
            // For archetype markers, suppress the original and create a custom marker at the new location
            let preview = ut99_bsp::preview_wizard_topology(&self.map_wizard_config);
            if let Some(m) = preview.markers.iter().find(|m| m.label == marker_label) {
                if !self.map_wizard_config.suppressed_markers.contains(&marker_label.to_string()) {
                    self.map_wizard_config.suppressed_markers.push(marker_label.to_string());
                }
                let default_url = match marker_label {
                    "WizardTeleA" => "WizardTeleB",
                    "WizardTeleB" => "WizardTeleA",
                    _ => "",
                };
                let count = self.map_wizard_config.custom_markers.len() + 1;
                let id = format!("obj_{count}");
                let mut cm = ut99_bsp::WizardCustomMarker {
                    id,
                    label: marker_label.to_string(),
                    kind: m.kind.to_string(),
                    class_name: String::new(),
                    location: new_loc,
                    team: m.team,
                    teleporter_tag: if m.kind == "Teleporter" { marker_label.to_string() } else { String::new() },
                    teleporter_url: if m.kind == "Teleporter" { default_url.to_string() } else { String::new() },
                };
                if cm.class_name.is_empty() {
                    cm.class_name = match m.kind {
                        "PlayerStart" => "Engine.PlayerStart",
                        "FlagBase" => "Botpack.FlagBase",
                        "ControlPoint" => "Botpack.ControlPoint",
                        "FortStandard" => "Botpack.FortStandard",
                        "Light" => "Engine.Light",
                        "Teleporter" => "Engine.Teleporter",
                        _ => "Botpack.ShockRifle",
                    }.to_string();
                }
                self.map_wizard_config.custom_markers.push(cm);
            }
        }
    }

    /// Updates room position or size via transformation controls on the 2D minimap.
    pub fn update_wizard_room_transform(&mut self, room_name: &str, new_center: glam::Vec3, new_size: glam::Vec3) {
        if let Some(cr) = self.map_wizard_config.custom_rooms.iter_mut().find(|r| r.name == room_name) {
            cr.center = new_center;
            cr.size = new_size;
        } else {
            // Archetype room: update or create WizardRoomOverride
            if let Some(ro) = self.map_wizard_config.room_overrides.iter_mut().find(|r| r.room_name == room_name) {
                ro.center_offset = new_center;
                ro.size = new_size;
            } else {
                let current_env = self.map_wizard_config.get_room_environment(room_name);
                self.map_wizard_config.room_overrides.push(ut99_bsp::WizardRoomOverride {
                    room_name: room_name.to_string(),
                    center_offset: new_center,
                    size: new_size,
                    environment: current_env,
                });
            }
        }
    }

    /// Sets the environmental medium (Normal, Water, Slime, Lava, LowGravity) for a room.
    pub fn set_wizard_room_environment(&mut self, room_name: &str, env: ut99_bsp::WizardRoomEnvironment) {
        if let Some(cr) = self.map_wizard_config.custom_rooms.iter_mut().find(|r| r.name == room_name) {
            cr.environment = env;
            return;
        }
        if let Some(ro) = self.map_wizard_config.room_overrides.iter_mut().find(|r| r.room_name == room_name) {
            ro.environment = env;
            return;
        }
        let preview = ut99_bsp::preview_wizard_topology(&self.map_wizard_config);
        if let Some(r) = preview.rooms.iter().find(|r| r.label == room_name) {
            self.map_wizard_config.room_overrides.push(ut99_bsp::WizardRoomOverride {
                room_name: room_name.to_string(),
                center_offset: r.center,
                size: r.size,
                environment: env,
            });
        }
    }

    /// Updates attributes of a custom corridor connection (width and sliding blast door toggle).
    pub fn update_wizard_custom_connection(&mut self, id: &str, width: f32, has_blast_door: bool) {
        if let Some(conn) = self.map_wizard_config.custom_connections.iter_mut().find(|c| c.id == id) {
            conn.width = width.clamp(128.0, 512.0);
            conn.has_blast_door = has_blast_door;
        }
    }

    /// Duplicates the currently selected minimap element (marker or room) with a grid offset.
    pub fn duplicate_wizard_selection(&mut self) -> Option<crate::editor_state::WizardMinimapSelection> {
        let snap = if self.wizard_minimap_grid_snap > 0.0 { self.wizard_minimap_grid_snap } else { 64.0 };
        match self.wizard_minimap_selected.clone() {
            Some(crate::editor_state::WizardMinimapSelection::Marker(label)) => {
                if let Some(cm) = self.map_wizard_config.custom_markers.iter().find(|cm| cm.label == label || cm.id == label).cloned() {
                    let count = self.map_wizard_config.custom_markers.len() + 1;
                    let id = format!("obj_{count}");
                    let new_label = format!("{} (Copy)", cm.label);
                    let new_loc = cm.location + glam::Vec3::new(snap, snap, 0.0);
                    self.map_wizard_config.custom_markers.push(ut99_bsp::WizardCustomMarker {
                        id,
                        label: new_label.clone(),
                        kind: cm.kind.clone(),
                        class_name: cm.class_name,
                        location: new_loc,
                        team: cm.team,
                        teleporter_tag: if cm.kind == "Teleporter" { format!("{new_label}_Tag") } else { String::new() },
                        teleporter_url: if cm.kind == "Teleporter" { cm.teleporter_tag.clone() } else { String::new() },
                    });
                    let sel = crate::editor_state::WizardMinimapSelection::Marker(new_label);
                    self.wizard_minimap_selected = Some(sel.clone());
                    self.toast_info("Duplicated object");
                    return Some(sel);
                } else {
                    let preview = ut99_bsp::preview_wizard_topology(&self.map_wizard_config);
                    if let Some(m) = preview.markers.iter().find(|m| m.label == label) {
                        let new_label = self.add_wizard_custom_marker(
                            m.kind,
                            m.location.x + snap,
                            m.location.y + snap,
                        );
                        let sel = crate::editor_state::WizardMinimapSelection::Marker(new_label);
                        self.wizard_minimap_selected = Some(sel.clone());
                        self.toast_info("Duplicated marker");
                        return Some(sel);
                    }
                }
            }
            Some(crate::editor_state::WizardMinimapSelection::Room(name)) => {
                let preview = ut99_bsp::preview_wizard_topology(&self.map_wizard_config);
                if let Some(r) = preview.rooms.iter().find(|r| r.label == name) {
                    let new_x = r.center.x + snap * 2.0;
                    let new_y = r.center.y + snap * 2.0;
                    let new_name = self.add_wizard_custom_room(new_x, new_y);
                    if let Some(cr) = self.map_wizard_config.custom_rooms.iter_mut().find(|cr| cr.name == new_name) {
                        cr.size = r.size;
                        cr.environment = r.environment;
                    }
                    let sel = crate::editor_state::WizardMinimapSelection::Room(new_name);
                    self.wizard_minimap_selected = Some(sel.clone());
                    self.toast_info("Duplicated room");
                    return Some(sel);
                }
            }
            _ => {}
        }
        None
    }

    /// Exports the current wizard configuration to a formatted JSON string.
    pub fn export_wizard_preset_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.map_wizard_config)
            .map_err(|e| format!("Failed to serialize wizard preset: {e}"))
    }

    /// Imports and parses a wizard configuration from a JSON string.
    pub fn import_wizard_preset_json(&mut self, json_str: &str) -> Result<(), String> {
        let mut config: ut99_bsp::MapWizardConfig = serde_json::from_str(json_str)
            .map_err(|e| format!("Invalid wizard preset JSON: {e}"))?;
        let before = serde_json::to_value(&config).map_err(|e| format!("Failed to inspect wizard preset: {e}"))?;
        config.enforce_mode_invariants();
        let after = serde_json::to_value(&config).map_err(|e| format!("Failed to inspect normalized wizard preset: {e}"))?;
        let changed: Vec<String> = before.as_object().into_iter().flat_map(|object| object.iter())
            .filter(|(key, value)| after.get(*key) != Some(*value))
            .map(|(key, _)| key.clone())
            .collect();
        self.map_wizard_config = config;
        if changed.is_empty() {
            self.toast_info("Imported Map Wizard preset successfully.");
        } else {
            let message = format!("Imported preset with adjusted settings: {}", changed.join(", "));
            self.log(message.clone());
            self.toast_warning(message);
        }
        Ok(())
    }

    /// Clears all user-added custom rooms, connections, overrides and markers on the 2D minimap.
    pub fn clear_wizard_custom_overrides(&mut self) {
        let r_count = self.map_wizard_config.custom_rooms.len();
        let c_count = self.map_wizard_config.custom_connections.len();
        let m_count = self.map_wizard_config.custom_markers.len();
        let o_count = self.map_wizard_config.room_overrides.len();
        self.map_wizard_config.custom_rooms.clear();
        self.map_wizard_config.custom_connections.clear();
        self.map_wizard_config.room_overrides.clear();
        self.map_wizard_config.custom_markers.clear();
        self.map_wizard_config.suppressed_markers.clear();
        self.wizard_minimap_selected = None;
        self.wizard_minimap_connect_source = None;
        self.wizard_minimap_drag_state = crate::editor_state::WizardMinimapDragState::None;
        self.toast_info(format!("Cleared {r_count} rooms, {c_count} corridors, {m_count} custom objects, and {o_count} room overrides"));
    }

    /// Audits all teleporters currently planned in the map wizard.
    pub fn audit_wizard_teleporters(&self) -> ut99_bsp::WizardTeleportAuditSummary {
        ut99_bsp::audit_wizard_teleporters(&self.map_wizard_config)
    }

    /// Automatically repairs any teleporter configuration anomalies (lone, disconnected, loop hazard).
    pub fn auto_fix_wizard_teleporters(&mut self) -> Vec<String> {
        let fixes = ut99_bsp::auto_fix_wizard_teleporters(&mut self.map_wizard_config);
        if fixes.is_empty() {
            self.toast_info("Teleporters are already healthy and connected.");
        } else {
            self.toast_info(format!("Auto-fixed teleporters: {} action(s) taken.", fixes.len()));
        }
        fixes
    }

    /// Updates Tag and Destination URL for a specific teleporter marker.
    pub fn update_wizard_teleporter_properties(&mut self, marker_label: &str, tag: &str, url: &str) {
        if let Some(cm) = self.map_wizard_config.custom_markers.iter_mut().find(|m| m.label == marker_label || m.id == marker_label) {
            cm.teleporter_tag = tag.trim().to_string();
            cm.teleporter_url = url.trim().to_string();
        } else {
            let preview = ut99_bsp::preview_wizard_topology(&self.map_wizard_config);
            if let Some(m) = preview.markers.iter().find(|m| m.label == marker_label) {
                if !self.map_wizard_config.suppressed_markers.contains(&marker_label.to_string()) {
                    self.map_wizard_config.suppressed_markers.push(marker_label.to_string());
                }
                let count = self.map_wizard_config.custom_markers.len() + 1;
                self.map_wizard_config.custom_markers.push(ut99_bsp::WizardCustomMarker {
                    id: format!("obj_{count}"),
                    label: marker_label.to_string(),
                    kind: "Teleporter".to_string(),
                    class_name: "Engine.Teleporter".to_string(),
                    location: m.location,
                    team: m.team,
                    teleporter_tag: tag.trim().to_string(),
                    teleporter_url: url.trim().to_string(),
                });
            }
        }
    }

    /// Runs a comprehensive pre-export audit and auto-repair sequence using Level Doctor.
    ///
    /// 1. Rebuilds geometry, lighting, and AI navigation paths.
    /// 2. Performs Level Doctor automated repairs on all brushes (planar polygon repairs,
    ///    missing textures and degenerate polygons); missing gameplay actors are reported.
    /// 3. If geometry was modified by auto-repairs, rebuilds geometry, lighting, and AI paths.
    /// 4. Executes a full Level Doctor audit, including isolated navigation nodes.
    /// 5. Returns `Ok(total_fixes)` if zero blocking errors remain, or `Err(errors)` if blocking
    ///    errors are detected, preventing broken `.unr` files from being exported or playtested.
    pub fn audit_and_autofix_before_export(&mut self) -> Result<usize, Vec<String>> {
        let before = self.repair_snapshot();
        let result = self.audit_and_autofix_before_export_without_history();
        self.record_repairs(before, "Pre-export build and Level Doctor auto-fix");
        result
    }

    /// Removes `PathNode_Foot_*` / `PathNode_Head_*` stair waypoints that have neither an incoming nor an outgoing
    /// reach spec in the current navigation graph. Returns how many actors were removed.
    fn prune_isolated_stair_waypoints(&mut self) -> usize {
        let mut connected = vec![false; self.nav_graph.nodes.len()];
        for spec in &self.nav_graph.reach_specs {
            if let Some(c) = connected.get_mut(spec.from) { *c = true; }
            if let Some(c) = connected.get_mut(spec.to) { *c = true; }
        }
        let orphans: std::collections::HashSet<String> = self
            .nav_graph
            .nodes
            .iter()
            .zip(&connected)
            .filter(|(n, &ok)| !ok && (n.name.starts_with("PathNode_Foot_") || n.name.starts_with("PathNode_Head_")))
            .map(|(n, _)| n.name.clone())
            .collect();
        if orphans.is_empty() {
            return 0;
        }
        let Some(level) = self.loaded_unr.as_mut() else { return 0; };
        let before = level.actors.len();
        level.actors.retain(|a| !orphans.contains(&a.name));
        level.sync_actor_views();
        let removed = before - level.actors.len();
        self.log(format!("Level Doctor: removed {removed} orphan stair waypoint(s): {}", orphans.into_iter().collect::<Vec<_>>().join(", ")));
        removed
    }

    fn audit_and_autofix_before_export_without_history(&mut self) -> Result<usize, Vec<String>> {
        self.log("Level Doctor: Commencing pre-export audit and auto-repair verification...");

        // Step 1: Ensure geometry and AI paths are fully built
        self.build_geometry();
        self.build_lighting_without_history();
        self.build_ai_paths();

        // Step 1b: a stair foot/head waypoint that the collision-aware path build leaves with no connection at all
        // is an orphan (e.g. its stair was clipped by a neighbouring room); remove it rather than block export.
        let pruned_waypoints = self.prune_isolated_stair_waypoints();
        if pruned_waypoints > 0 {
            self.build_ai_paths();
        }

        // Step 2: Auto-fix geometry; missing starts require a safe, deliberate placement.
        let total_fixes = self.autofix_level_issues_without_history() + pruned_waypoints;

        // If brushes or actors were repaired, rebuild BSP and paths
        if total_fixes > 0 {
            self.build_geometry();
            self.build_lighting_without_history();
            self.build_ai_paths();
        }

        // Step 4: Run full diagnostic audit
        self.run_level_doctor_audit();

        // Step 5: Evaluate blocking errors
        let error_messages: Vec<String> = self.map_audit_issues
            .iter()
            .filter(|issue| issue.severity == ut99_bsp::IssueSeverity::Error)
            .map(|issue| format!("[{}] {}", issue.category, issue.description))
            .collect();

        if !error_messages.is_empty() {
            self.log(format!(
                "Level Doctor: Pre-export audit BLOCKED with {} error(s):\n  {}",
                error_messages.len(),
                error_messages.join("\n  ")
            ));
            return Err(error_messages);
        }

        let warning_count = self.map_audit_issues
            .iter()
            .filter(|issue| issue.severity == ut99_bsp::IssueSeverity::Warning)
            .count();

        self.log(format!(
            "Level Doctor: Pre-export audit PASSED CLEANLY (0 errors, {} warning(s), {} auto-fix(es) applied). Ready for export.",
            warning_count,
            total_fixes
        ));

        Ok(total_fixes)
    }

}
