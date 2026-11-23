//! Prepare the game command without writing files or starting a process.
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) struct PreparedLaunch {
    executable: PathBuf,
    working_directory: PathBuf,
    url_options: String,
    process_arguments: Vec<String>,
}

pub(crate) fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    let path = std::path::absolute(path).map_err(|e| format!("Could not resolve '{}': {e}", path.display()))?;
    // UE1 understands normal drive/UNC paths, not the Win32 verbatim namespace.
    #[cfg(windows)]
    {
        if let Some(text) = path.to_str() {
            if let Some(rest) = text.strip_prefix(r"\\?\UNC\") { return Ok(PathBuf::from(format!(r"\\{rest}"))); }
            if let Some(rest) = text.strip_prefix(r"\\?\") { return Ok(PathBuf::from(rest)); }
        }
    }
    Ok(path)
}

pub(crate) fn resolve_executable(configured: &str) -> Result<PathBuf, String> {
    let configured = configured.trim().trim_matches('"');
    let input = Path::new(configured);
    let mut candidates = Vec::new();
    if !configured.is_empty() && input.is_file() {
        candidates.push(input.to_path_buf());
    } else if !configured.is_empty() && input.is_dir() {
        #[cfg(windows)]
        let names = ["UnrealTournament.exe", "Unreal.exe"];
        #[cfg(not(windows))]
        let names = ["ut", "ut-bin", "ut-bin-amd64", "UnrealTournament", "Unreal"];
        for directory in [input.join("System"), input.to_path_buf()] {
            for name in names { candidates.push(directory.join(name)); }
        }
    }
    for candidate in candidates {
        if !candidate.is_file() { continue; }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if candidate.metadata().map(|m| m.permissions().mode() & 0o111 == 0).unwrap_or(true) { continue; }
        }
        return absolute_path(&candidate);
    }
    Err(format!("UT99 Executable not found at '{configured}'. Configure a game executable or UT folder in Play -> Launch Settings."))
}

impl PreparedLaunch {
    pub fn new(configured: &str, arguments: &str) -> Result<Self, String> {
        let executable = resolve_executable(configured)?;
        let working_directory = executable.parent().ok_or("Game executable has no parent directory")?.to_path_buf();
        let mut url_options = String::new();
        let mut process_arguments = Vec::new();
        for argument in split_arguments(arguments)? {
            if argument.starts_with('?') { url_options.push_str(&argument); }
            else { process_arguments.push(argument); }
        }
        Ok(Self { executable, working_directory, url_options, process_arguments })
    }

    pub fn command(&self, saved_map: &Path) -> Result<Command, String> {
        if !saved_map.is_file() {
            return Err(format!("Playtest map not found: '{}'", saved_map.display()));
        }
        if !saved_map.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("unr")) {
            return Err("Playtest requires a saved .unr map.".into());
        }
        let mut url = absolute_path(saved_map)?.into_os_string();
        url.push(&self.url_options);
        let mut command = Command::new(&self.executable);
        command.arg(url);
        command.args(&self.process_arguments);
        command.current_dir(&self.working_directory);
        Ok(command)
    }

    pub fn spawn(&self, saved_map: &Path) -> Result<(), String> {
        self.command(saved_map)?.spawn().map_err(|e| {
            format!("Failed to start game executable '{}': {e}", self.executable.display())
        })?;
        Ok(())
    }
}

/// Double quotes group arguments; backslashes remain literal, including Windows paths.
fn split_arguments(arguments: &str) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for character in arguments.chars() {
        if character == '"' {
            quoted = !quoted;
            started = true;
        } else if character.is_whitespace() && !quoted {
            if started { result.push(std::mem::take(&mut current)); started = false; }
        } else {
            current.push(character);
            started = true;
        }
    }
    if quoted { return Err("Playtest launch arguments contain an unclosed double quote.".into()); }
    if started { result.push(current); }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_paths_keep_spaces_backslashes_and_apostrophes() {
        assert_eq!(
            split_arguments(r#"?game=Botpack.CTFGame -ini="C:\Games Folder\UT.ini" -name=O'Neil"#).unwrap(),
            vec!["?game=Botpack.CTFGame", r"-ini=C:\Games Folder\UT.ini", "-name=O'Neil"]
        );
        assert!(split_arguments("-ini=\"unfinished").is_err());
    }

    #[test]
    fn relative_executable_is_resolved_before_changing_working_directory() {
        let executable = absolute_path(&std::env::current_exe().unwrap()).unwrap();
        let current = absolute_path(&std::env::current_dir().unwrap()).unwrap();
        let mut directory = current.as_path();
        let mut relative = PathBuf::new();
        while executable.strip_prefix(directory).is_err() {
            directory = directory.parent().expect("executable and test working directory share a root");
            relative.push("..");
        }
        relative.push(executable.strip_prefix(directory).unwrap());
        let launch = PreparedLaunch::new(relative.to_str().unwrap(), "-log").unwrap();
        assert!(launch.executable.is_absolute());
        assert_eq!(launch.executable.canonicalize().unwrap(), executable.canonicalize().unwrap());
        assert_eq!(launch.working_directory, launch.executable.parent().unwrap());
    }
}
