use super::*;

impl EditorState {
    pub fn open_uc_viewer_for_class(&mut self, class_name: &str) {
        let (sup, pkg) = self.all_classes
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(class_name))
            .map(|c| (Some(c.super_name.as_str()), Some(c.package.as_str())))
            .unwrap_or((None, None));

        let meta = crate::unrealscript::get_class_metadata(class_name, sup, pkg);
        self.uc_source_cache = meta.to_uc_source();
        self.uc_editable_defaults = meta.default_properties.clone();
        self.uc_viewer_class = class_name.to_string();
        self.uc_class_metadata = Some(meta);
        self.show_uc_viewer_dialog = true;
        self.log(format!("📜 Opened UnrealScript Studio for class '{}'", class_name));
    }

    pub fn update_class_default_property(&mut self, prop_name: &str, new_value: &str) {
        if let Some(entry) = self.uc_editable_defaults.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(prop_name)) {
            entry.1 = new_value.to_string();
        } else {
            self.uc_editable_defaults.push((prop_name.to_string(), new_value.to_string()));
        }

        if let Some(ref mut meta) = self.uc_class_metadata {
            meta.default_properties = self.uc_editable_defaults.clone();
            self.uc_source_cache = meta.to_uc_source();
        }
        self.log(format!("Updated DefaultProperty: '{}' = '{}' for class '{}'", prop_name, new_value, self.uc_viewer_class));
    }

    pub fn propagate_default_properties_to_actors(&mut self, class_name: &str) -> usize {
        let Some(ref mut unr) = self.loaded_unr else {
            return 0;
        };

        let old_actors = unr.actors.to_vec();
        let mut modified_count = 0;

        for actor in &mut unr.actors {
            if actor.class_name.eq_ignore_ascii_case(class_name) {
                modified_count += 1;
                for (key, val_str) in &self.uc_editable_defaults {
                    match key.as_str() {
                        "LightBrightness" => {
                            if let Ok(v) = val_str.parse::<u8>() { actor.light_brightness = v; }
                        }
                        "LightRadius" => {
                            if let Ok(v) = val_str.parse::<u8>() { actor.light_radius = v; }
                        }
                        "LightHue" => {
                            if let Ok(v) = val_str.parse::<u8>() { actor.light_hue = v; }
                        }
                        "LightSaturation" => {
                            if let Ok(v) = val_str.parse::<u8>() { actor.light_saturation = v; }
                        }
                        "Tag" => {
                            actor.tag = val_str.trim_matches('"').to_string();
                        }
                        "Event" => {
                            actor.event = val_str.trim_matches('"').to_string();
                        }
                        "Group" => {
                            actor.group = val_str.trim_matches('"').to_string();
                        }
                        "DrawScale" => {
                            if let Ok(v) = val_str.parse::<f32>() { actor.main_scale = glam::Vec3::splat(v); }
                        }
                        _ => {}
                    }
                    actor.properties.insert(
                        key.clone(),
                        ut99_format::TaggedProperty {
                            name: key.clone(),
                            array_index: 0,
                            value: ut99_format::TaggedValue::Str(val_str.clone()),
                        },
                    );
                }
            }
        }

        if modified_count > 0 {
            let new_actors = unr.actors.to_vec();
            self.history.push(Action::PropagateActorProperties {
                class_name: class_name.to_string(),
                old_actors,
                new_actors,
                description: format!("Propagated DefaultProperties to {} actors", modified_count),
            });
            self.log(format!("⚡ Propagated default properties to {} instanced actor(s) of class '{}'", modified_count, class_name));
        }

        modified_count
    }

    pub fn export_active_uc_file(&self, output_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
        std::fs::create_dir_all(output_dir).map_err(|e| format!("Failed to create directory: {}", e))?;
        let filename = format!("{}.uc", self.uc_viewer_class);
        let path = output_dir.join(&filename);
        std::fs::write(&path, &self.uc_source_cache).map_err(|e| format!("Failed to write .uc file: {}", e))?;
        Ok(path)
    }

    /// BL-P2-03: Validate and optionally compile the active UnrealScript `.uc` source buffer (`F7`).
    pub fn compile_active_uc_script(&mut self) -> crate::unrealscript::UcCompileReport {
        let ucc_buf = std::path::Path::new(self.ut99_executable_path.trim())
            .parent()
            .map(|p| p.join("UCC.exe"));
        let ucc_opt = ucc_buf
            .as_deref()
            .filter(|p| !self.ut99_executable_path.trim().is_empty() && p.exists());
        let report = crate::unrealscript::validate_and_compile_uc_source(
            &self.uc_source_cache,
            ucc_opt,
        );
        self.log(format!("⚙ UnrealScript [{}]: {}", self.uc_viewer_class, report.log_output));
        for diag in &report.diagnostics {
            self.log(format!(
                "  [{}] Line {}: {}",
                if diag.is_error { "ERROR" } else { "WARN" },
                diag.line,
                diag.message
            ));
        }
        if report.success {
            self.toast_success(report.log_output.clone());
        } else {
            self.toast_error(report.log_output.clone());
        }
        report
    }

}
