use super::*;

impl EditorState {
    pub fn add_recent_map(&mut self, path: impl AsRef<std::path::Path>) {
        let p = path.as_ref().to_path_buf();
        self.recent_maps.retain(|existing| existing != &p);
        self.recent_maps.insert(0, p);
        if self.recent_maps.len() > 10 {
            self.recent_maps.truncate(10);
        }
    }


    /// Records a command ID into the Command Palette MRU history.
    pub fn record_recent_palette_command(&mut self, cmd_id: impl Into<String>) {
        let id = cmd_id.into();
        self.recent_palette_commands.retain(|existing| existing != &id);
        self.recent_palette_commands.insert(0, id);
        if self.recent_palette_commands.len() > 8 {
            self.recent_palette_commands.truncate(8);
        }
    }


    /// Saves current editor preferences to a specific JSON file path.
    pub fn save_preferences_to_path(&mut self, path: &std::path::Path) -> bool {
        self.sync_audio_installation();
        let prefs = crate::preferences::EditorPreferences::from_editor_state(self);
        match prefs.save_to_path(path) {
            Ok(()) => {
                let msg = format!("Preferences saved to {:?}", path);
                self.log(&msg);
                self.toast_success("Preferences saved successfully");
                true
            }
            Err(e) => {
                let err_msg = format!("Failed to save preferences to {:?}: {}", path, e);
                self.log(&err_msg);
                self.toast_warning(err_msg);
                false
            }
        }
    }


    /// Loads editor preferences from a specific JSON file path.
    pub fn load_preferences_from_path(&mut self, path: &std::path::Path) -> bool {
        match crate::preferences::EditorPreferences::load_from_path(path) {
            Ok(prefs) => {
                prefs.apply_to_editor_state(self);
                let msg = format!("Preferences loaded from {:?}", path);
                self.log(&msg);
                self.toast_info("Preferences loaded");
                true
            }
            Err(e) => {
                let err_msg = format!("Failed to load preferences from {:?}: {}", path, e);
                self.log(&err_msg);
                self.toast_warning(err_msg);
                false
            }
        }
    }


    /// Saves editor preferences to standard user config file.
    pub fn save_preferences(&mut self) -> bool {
        self.save_preferences_to_path(&crate::preferences::EditorPreferences::default_path())
    }


    /// Loads editor preferences from standard user config file if present.
    pub fn load_preferences(&mut self) -> bool {
        let path = crate::preferences::EditorPreferences::default_path();
        if path.exists() {
            let loaded = self.load_preferences_from_path(&path);
            if !self.first_run_completed {
                self.show_welcome_wizard = true;
            }
            loaded
        } else {
            // First launch of Forge99: launch Welcome Wizard
            self.show_welcome_wizard = true;
            false
        }
    }

}
