use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use ut99_bsp::{CsgOperation, PF_SEMISOLID};
use ut99_format::{T3dActor, T3dBrush, T3dMap, T3dPoly, TaggedProperty, TaggedValue};
use crate::editor_state::EditorState;

/// Conversion factor from degrees to Unreal Engine 1 Rotator units (65536 = 360 degrees).
pub const DEG_TO_ROT: f32 = 65536.0 / 360.0;

#[inline]
pub fn deg_to_rotator(deg: glam::Vec3) -> (i32, i32, i32) {
    (
        (deg.x * DEG_TO_ROT).round() as i32,
        (deg.y * DEG_TO_ROT).round() as i32,
        (deg.z * DEG_TO_ROT).round() as i32,
    )
}

fn tagged_value_to_t3d(val: &TaggedValue) -> Option<String> {
    match val {
        TaggedValue::Byte(b) => Some(b.to_string()),
        TaggedValue::Int(i) => Some(i.to_string()),
        TaggedValue::Bool(b) => Some(if *b { "True".to_string() } else { "False".to_string() }),
        TaggedValue::Float(f) => Some(format!("{:.6}", f)),
        TaggedValue::Name(s) => Some(s.clone()),
        TaggedValue::Str(s) => Some(format!("\"{}\"", s.replace('"', "\\\""))),
        TaggedValue::Vector(v) => Some(format!("(X={:.6},Y={:.6},Z={:.6})", v.x, v.y, v.z)),
        TaggedValue::Rotator { pitch, yaw, roll } => {
            Some(format!("(Pitch={},Yaw={},Roll={})", pitch, yaw, roll))
        }
        TaggedValue::Color { r, g, b, a } => {
            Some(format!("(R={},G={},B={},A={})", r, g, b, a))
        }
        TaggedValue::Scale {
            scale,
            sheer_rate,
            sheer_axis,
        } => Some(format!(
            "(Scale=(X={:.6},Y={:.6},Z={:.6}),SheerRate={:.6},SheerAxis={})",
            scale.x, scale.y, scale.z, sheer_rate, sheer_axis
        )),
        TaggedValue::Object(_) | TaggedValue::Raw(_) => None,
    }
}

/// Serialises engine builds inside this process.
static ENGINE_BUILD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Thread-safe user-configured UT99 root directory override (synced from Preferences / Welcome Wizard).
static CUSTOM_UT_DIR: std::sync::OnceLock<std::sync::RwLock<Option<PathBuf>>> = std::sync::OnceLock::new();

fn backup_existing_map(dest: &Path) -> Result<()> {
    if dest.exists() {
        let backup = dest.with_extension("unr.bak");
        fs::copy(dest, &backup).map_err(|e| anyhow::anyhow!(
            "Could not back up {:?} to {:?}: {e}", dest, backup
        ))?;
    }
    Ok(())
}

fn validate_and_commit_unr(staged: &Path, dest: &Path) -> Result<()> {
    let package = ut99_format::Package::open(staged)?;
    let validation = ut99_format::UnrLevel::validate_package(&package);
    if !validation.is_valid {
        anyhow::bail!("Saved package failed structural validation: {}", validation.errors.join("; "));
    }
    backup_existing_map(dest)?;
    fs::rename(staged, dest)?;
    Ok(())
}

/// Cross-process lock for UCC's shared files. Never expire a lock by age: a build can
/// legitimately exceed any individual UCC timeout. After a crash, a stale lock must
/// be removed explicitly once the operator has confirmed no build is running.
struct EngineFileLock {
    path: PathBuf,
    owner: String,
}

impl EngineFileLock {
    fn acquire(sys_dir: &Path) -> Result<Self> {
        Self::acquire_with_timeout(sys_dir, std::time::Duration::from_secs(900))
    }

    fn acquire_with_timeout(sys_dir: &Path, timeout: std::time::Duration) -> Result<Self> {
        use std::io::Write;
        let path = sys_dir.join("_wz_engine.lock");
        let started = std::time::Instant::now();
        let owner = format!("{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos());
        loop {
            match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    if let Err(error) = file.write_all(owner.as_bytes()) {
                        drop(file);
                        let _ = fs::remove_file(&path);
                        return Err(error.into());
                    }
                    return Ok(Self { path, owner });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if started.elapsed() >= timeout {
                        anyhow::bail!("Timed out waiting for UCC lock {:?}; if no build is running, remove the stale lock manually", path);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200).min(timeout.saturating_sub(started.elapsed())));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

impl Drop for EngineFileLock {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path).ok().as_deref() == Some(self.owner.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod engine_lock_tests {
    use super::{backup_existing_map, validate_and_commit_unr, CompilerBridge, EngineFileLock};
    use std::time::Duration;

    fn test_dir() -> std::path::PathBuf {
        let name = format!("forge99-lock-{}-{:?}", std::process::id(), std::thread::current().id());
        let path = std::env::temp_dir().join(name);
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn lock_contention_fails_closed_and_releases_for_next_builder() {
        let dir = test_dir();
        let first = EngineFileLock::acquire_with_timeout(&dir, Duration::ZERO).unwrap();
        assert!(EngineFileLock::acquire_with_timeout(&dir, Duration::from_millis(10)).is_err());
        drop(first);
        assert!(EngineFileLock::acquire_with_timeout(&dir, Duration::ZERO).is_ok());
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn lock_does_not_remove_a_replaced_owner_or_ignore_io_errors() {
        let dir = test_dir();
        let guard = EngineFileLock::acquire_with_timeout(&dir, Duration::ZERO).unwrap();
        let path = dir.join("_wz_engine.lock");
        std::fs::write(&path, "other-owner").unwrap();
        drop(guard);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "other-owner");
        assert!(EngineFileLock::acquire_with_timeout(&dir.join("missing"), Duration::ZERO).is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn lock_prevents_a_second_process_from_acquiring() {
        const CHILD_DIR: &str = "FORGE99_LOCK_TEST_CHILD_DIR";
        if let Some(dir) = std::env::var_os(CHILD_DIR) {
            assert!(EngineFileLock::acquire_with_timeout(
                std::path::Path::new(&dir), Duration::from_millis(20)
            ).is_err());
            return;
        }
        let dir = test_dir();
        let guard = EngineFileLock::acquire_with_timeout(&dir, Duration::ZERO).unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "compiler::engine_lock_tests::lock_prevents_a_second_process_from_acquiring"])
            .env(CHILD_DIR, &dir)
            .status().unwrap();
        assert!(status.success());
        drop(guard);
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn failed_backup_preserves_original_map() {
        let dir = test_dir();
        let map = dir.join("map.unr");
        let backup = dir.join("map.unr.bak");
        std::fs::write(&map, b"original").unwrap();
        std::fs::create_dir(&backup).unwrap();
        assert!(backup_existing_map(&map).is_err());
        assert_eq!(std::fs::read(&map).unwrap(), b"original");
        std::fs::remove_dir(backup).unwrap();
        std::fs::remove_file(map).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn invalid_staged_package_never_replaces_destination() {
        let dir = test_dir();
        let original = dir.join("map.unr");
        let staged = dir.join("staged.unr");
        std::fs::write(&original, b"original map").unwrap();
        std::fs::write(&staged, b"corrupt package").unwrap();
        assert!(validate_and_commit_unr(&staged, &original).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original map");
        assert!(!original.with_extension("unr.bak").exists());
        std::fs::remove_file(staged).unwrap();
        std::fs::remove_file(original).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn engine_cleanup_status_reports_failed_and_missing_outputs() {
        assert!(CompilerBridge::cleanup_pass_warning(Ok((false, true, " 0 error(s)".into())), 128).is_none());
        assert!(CompilerBridge::cleanup_pass_warning(Ok((false, true, " 0 error(s)".into())), 0).is_some());
        assert!(CompilerBridge::cleanup_pass_warning(Ok((true, false, " 0 error(s)".into())), 128).unwrap().contains("timeout=true"));
        assert!(CompilerBridge::cleanup_pass_warning(Ok((false, false, " 0 error(s)".into())), 128).unwrap().contains("exit_success=false"));
        assert!(CompilerBridge::cleanup_pass_warning(Err(anyhow::anyhow!("process failed")), 0).unwrap().contains("process failed"));
    }
}

pub struct CompilerBridge;

/// Lightweight, thread-safe snapshot of document geometry and actors captured via O(1) CoW handles.
pub struct T3dExportSnapshot {
    pub map_instance_id: u64,
    pub doc_rev: u64,
    pub map_name_stem: String,
    pub target_engine: crate::editor_state::TargetEngine,
    pub level_title: String,
    pub level_author: String,
    pub level_ideal_player_count: String,
    pub level_song: String,
    pub level_game_type: String,
    pub level_enter_text: String,
    pub level_ambient_brightness: u8,
    pub level_ambient_hue: u8,
    pub level_ambient_saturation: u8,
    pub world_brushes: std::sync::Arc<Vec<ut99_bsp::Brush>>,
    pub brush_groups: std::collections::HashMap<usize, String>,
    pub movers: std::sync::Arc<Vec<crate::editor_state::EditorMover>>,
    pub static_meshes: std::sync::Arc<Vec<ut99_bsp::Brush>>,
    pub loaded_unr_actors: Option<std::sync::Arc<Vec<ut99_format::ActorRecord>>>,
    pub fallback_map_actors: Option<std::sync::Arc<Vec<ut99_format::T3dActor>>>,
}

impl T3dExportSnapshot {
    pub fn capture(state: &EditorState) -> Self {
        let map_name_stem = state
            .map_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("UntitledLevel")
            .to_string();
        let loaded_unr_actors = state.loaded_unr.as_ref().map(|u| u.actors.shared_snapshot());
        let fallback_map_actors = if loaded_unr_actors.is_none() {
            Some(state.map.actors.shared_snapshot())
        } else {
            None
        };
        Self {
            map_instance_id: state.map_instance_id,
            doc_rev: state.document_revision(),
            map_name_stem,
            target_engine: state.target_engine,
            level_title: state.level_title.clone(),
            level_author: state.level_author.clone(),
            level_ideal_player_count: state.level_ideal_player_count.clone(),
            level_song: state.level_song.clone(),
            level_game_type: state.level_game_type.clone(),
            level_enter_text: state.level_enter_text.clone(),
            level_ambient_brightness: state.level_ambient_brightness,
            level_ambient_hue: state.level_ambient_hue,
            level_ambient_saturation: state.level_ambient_saturation,
            world_brushes: state.world_brushes.shared_snapshot(),
            brush_groups: state.brush_groups.clone(),
            movers: state.movers.shared_snapshot(),
            static_meshes: state.static_meshes.shared_snapshot(),
            loaded_unr_actors,
            fallback_map_actors,
        }
    }
}

/// Thread-safe snapshot for native `.unr` synthesis off the UI thread.
pub struct UnrExportSnapshot {
    pub t3d: T3dExportSnapshot,
    pub has_mylevel_screenshot: bool,
    pub loaded_package: Option<ut99_format::Package>,
    pub reach_specs: Vec<ut99_format::ReachSpecRecord>,
    pub embedded_textures: Vec<ut99_format::UtxTexture>,
    pub embedded_sounds: Vec<ut99_format::package::SoundResource>,
}

impl UnrExportSnapshot {
    pub fn capture(state: &EditorState) -> Self {
        let t3d = T3dExportSnapshot::capture(state);
        let mylevel_pkg = state.texture_manager.get_package("MyLevel");
        let has_mylevel_screenshot = mylevel_pkg.is_some_and(|pkg| {
            pkg.textures
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case("Screenshot"))
        });
        let embedded_textures = mylevel_pkg.map(|pkg| pkg.textures.clone()).unwrap_or_default();
        let embedded_sounds = if !state.mylevel_sounds.is_empty() {
            state.mylevel_sounds.clone()
        } else if let Some(ref unr) = state.loaded_unr {
            unr.embedded_sounds.clone()
        } else {
            Vec::new()
        };
        let mut reach_specs = Vec::with_capacity(state.nav_graph.reach_specs.len());
        for rs in &state.nav_graph.reach_specs {
            if let (Some(from_node), Some(to_node)) = (
                state.nav_graph.nodes.get(rs.from),
                state.nav_graph.nodes.get(rs.to),
            ) {
                reach_specs.push(ut99_format::ReachSpecRecord {
                    distance: rs.distance.round() as i32,
                    start_actor_name: from_node.name.clone(),
                    end_actor_name: to_node.name.clone(),
                    collision_radius: 70,
                    collision_height: 70,
                    reach_flags: rs.reach_flags as i32,
                    b_pruned: 0,
                });
            }
        }
        Self {
            t3d,
            has_mylevel_screenshot,
            loaded_package: state.loaded_unr.as_ref().map(|u| u.package.clone()),
            reach_specs,
            embedded_textures,
            embedded_sounds,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnrSaveBackend { Engine, Native }

#[derive(Debug, Clone)]
pub struct UnrSaveReport {
    pub path: PathBuf,
    pub backend: UnrSaveBackend,
    /// Why the native writer was selected, including a failed engine build.
    pub native_reason: Option<String>,
    /// A successful first engine pass can be retained when cleanup fails.
    pub engine_warning: Option<String>,
}

struct UnrStageDir(PathBuf);

impl UnrStageDir {
    fn create(parent: &Path) -> Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        loop {
            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".forge99-stage-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }
}

impl Drop for UnrStageDir {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

impl CompilerBridge {
    /// Canonicalizes poly flags for UnrealEd 2.0 / UCC T3D export.
    /// Maps legacy internal 0x00200000 to canonical UE1 PF_Semisolid (0x20 = 32).
    #[inline]
    pub fn canonicalize_t3d_poly_flags(flags: u32) -> u32 {
        let mut f = flags;
        if (f & (ut99_bsp::PF_SEMISOLID | ut99_bsp::PF_LEGACY_SEMISOLID)) != 0 {
            f = (f & !ut99_bsp::PF_LEGACY_SEMISOLID) | ut99_bsp::PF_SEMISOLID;
        }
        f
    }

    /// Helper to convert an internal ut99_bsp::Brush into a T3dBrush for T3D export
    pub fn convert_brush_to_t3d(brush: &ut99_bsp::Brush, model_name: String) -> T3dBrush {
        let mut t3d_brush = T3dBrush {
            name: model_name,
            polys: Vec::with_capacity(brush.polys.len()),
        };
        for poly in &brush.polys {
            let t3d_poly = T3dPoly {
                item: String::new(),
                flags: Self::canonicalize_t3d_poly_flags(poly.flags),
                link: 0,
                pan_u: 0,
                pan_v: 0,
                normal: poly.plane.normal,
                base: poly.base,
                texture_u: poly.texture_u,
                texture_v: poly.texture_v,
                texture: if !poly.texture.is_empty() {
                    poly.texture.clone()
                } else {
                    "Default".into()
                },
                vertices: poly.vertices.clone(),
            };
            t3d_brush.polys.push(t3d_poly);
        }
        t3d_brush
    }

    /// Converts current EditorState into an authoritative T3dMap.
    pub fn export_t3d_map(state: &EditorState) -> T3dMap {
        let snapshot = T3dExportSnapshot::capture(state);
        Self::export_t3d_map_from_snapshot(&snapshot)
    }

    /// Converts a captured `T3dExportSnapshot` into an authoritative `T3dMap` (`Send + 'static`).
    pub fn export_t3d_map_from_snapshot(snap: &T3dExportSnapshot) -> T3dMap {
        let mut map = T3dMap::new();

        // 1. LevelInfo (UnrealEd 2.0 Parity)
        let mut level_info = T3dActor::new("LevelInfo", "LevelInfo0");
        level_info.set_property("Title", format!("\"{}\"", snap.level_title));
        level_info.set_property("Author", format!("\"{}\"", snap.level_author));
        if !snap.level_ideal_player_count.is_empty() {
            level_info.set_property("IdealPlayerCount", format!("\"{}\"", snap.level_ideal_player_count));
        }
        if !snap.level_song.is_empty() {
            level_info.set_property("Song", format!("\"{}\"", snap.level_song));
        }
        if !snap.level_game_type.is_empty() {
            level_info.set_property("DefaultGameType", format!("Class'{}'", snap.level_game_type));
        }
        if !snap.level_enter_text.is_empty() {
            level_info.set_property("LevelEnterText", format!("\"{}\"", snap.level_enter_text));
        }
        if snap.level_ambient_brightness > 0 {
            level_info.set_property("AmbientBrightness", snap.level_ambient_brightness.to_string());
            level_info.set_property("AmbientHue", snap.level_ambient_hue.to_string());
            level_info.set_property("AmbientSaturation", snap.level_ambient_saturation.to_string());
        }
        map.actors.push(level_info);

        // 2. World Brushes
        for (b_idx, brush) in snap.world_brushes.iter().enumerate() {
            if !brush.name.is_empty() && snap.movers.iter().any(|m| m.name == brush.name) {
                continue;
            }
            let actor_name = if !brush.name.is_empty() {
                brush.name.clone()
            } else {
                format!("Brush_{}", b_idx)
            };
            let mut actor = T3dActor::new("Brush", actor_name);
            actor.set_location(brush.location);
            if brush.rotation != glam::Vec3::ZERO {
                let (pitch, yaw, roll) = deg_to_rotator(brush.rotation);
                actor.set_property("Rotation", format!("(Pitch={},Yaw={},Roll={})", pitch, yaw, roll));
            }

            let oper_str = match brush.csg_oper {
                CsgOperation::Subtract => "CSG_Subtract",
                _ => "CSG_Add",
            };
            actor.set_property("CsgOper", oper_str);
            let poly_flags = brush.polys.first().map(|p| Self::canonicalize_t3d_poly_flags(p.flags)).unwrap_or(0);
            if poly_flags != 0 {
                actor.set_property("PolyFlags", poly_flags.to_string());
            }
            if let Some(grp) = brush.group.as_ref().or_else(|| snap.brush_groups.get(&b_idx)) {
                if !grp.is_empty() {
                    actor.set_property("Group", grp.clone());
                }
            }
            actor.set_property("MainScale", "(SheerAxis=SHEER_ZX)");
            actor.set_property("PostScale", "(SheerAxis=SHEER_ZX)");
            let model_name = format!("Model{}", b_idx);
            actor.set_property("Brush", format!("Model'MyLevel.{}'", model_name));
            actor.brush = Some(Self::convert_brush_to_t3d(brush, model_name));
            map.actors.push(actor);
        }

        // 3. Movers (Elevators, Doors, Dynamic Platforms)
        for (m_idx, mover) in snap.movers.iter().enumerate() {
            let actor_name = if !mover.name.is_empty() {
                mover.name.clone()
            } else {
                format!("Mover_{}", m_idx)
            };
            let mover_cls = if mover.mover_class.trim().is_empty() {
                "Mover"
            } else {
                mover.mover_class.trim()
            };
            let mut actor = T3dActor::new(mover_cls, actor_name);
            actor.set_location(mover.brush.location);
            if mover.brush.rotation != glam::Vec3::ZERO {
                let (pitch, yaw, roll) = deg_to_rotator(mover.brush.rotation);
                actor.set_property("Rotation", format!("(Pitch={},Yaw={},Roll={})", pitch, yaw, roll));
            }
            actor.set_property("CsgOper", "CSG_Active");
            actor.set_property("bDynamicLight", "True");
            actor.set_property("bStatic", "False");
            if !mover.initial_state.is_empty() {
                actor.set_property("InitialState", mover.initial_state.clone());
            }
            actor.set_property("StayOpenTime", format!("{:.6}", mover.stay_open_time));
            actor.set_property("MoveTime", format!("{:.6}", mover.move_time));
            if mover.delay_time > 0.0 {
                actor.set_property("DelayTime", format!("{:.6}", mover.delay_time));
            }
            actor.set_property("KeyNum", mover.current_key.to_string());
            let num_keys = mover.keyframes.iter().filter(|k| k.is_some()).count().max(1);
            actor.set_property("NumKeys", num_keys.to_string());
            if !mover.tag.is_empty() {
                actor.set_property("Tag", format!("\"{}\"", mover.tag));
            }
            if !mover.event.is_empty() {
                actor.set_property("Event", format!("\"{}\"", mover.event));
            }
            if !mover.glide_type.is_empty() {
                actor.set_property("MoverGlideType", mover.glide_type.clone());
            }
            if !mover.encroach_type.is_empty() {
                actor.set_property("MoverEncroachType", mover.encroach_type.clone());
            }
            let fmt_sound = |s: &str| -> String {
                if s.starts_with("Sound'") {
                    s.to_string()
                } else {
                    format!("Sound'{}'", s)
                }
            };
            if !mover.opening_sound.is_empty() {
                actor.set_property("OpeningSound", fmt_sound(&mover.opening_sound));
            }
            if !mover.opened_sound.is_empty() {
                actor.set_property("OpenedSound", fmt_sound(&mover.opened_sound));
            }
            if !mover.closing_sound.is_empty() {
                actor.set_property("ClosingSound", fmt_sound(&mover.closing_sound));
            }
            if !mover.closed_sound.is_empty() {
                actor.set_property("ClosedSound", fmt_sound(&mover.closed_sound));
            }
            if !mover.move_ambient_sound.is_empty() {
                actor.set_property("MoveAmbientSound", fmt_sound(&mover.move_ambient_sound));
            }

            // Keyframe offsets relative to Key 0
            if let Some(kf0) = &mover.keyframes[0] {
                for i in 1..8 {
                    if let Some(kfi) = &mover.keyframes[i] {
                        let delta_pos = kfi.location - kf0.location;
                        actor.set_property(
                            format!("KeyPos({})", i),
                            format!("(X={:.6},Y={:.6},Z={:.6})", delta_pos.x, delta_pos.y, delta_pos.z),
                        );
                        let delta_rot = kfi.rotation - kf0.rotation;
                        if delta_rot != glam::Vec3::ZERO {
                            let (dp, dy, dr) = deg_to_rotator(delta_rot);
                            actor.set_property(
                                format!("KeyRot({})", i),
                                format!("(Pitch={},Yaw={},Roll={})", dp, dy, dr),
                            );
                        }
                    }
                }
            }

            actor.set_property("MainScale", "(SheerAxis=SHEER_ZX)");
            actor.set_property("PostScale", "(SheerAxis=SHEER_ZX)");
            let model_name = format!("MoverModel{}", m_idx);
            actor.set_property("Brush", format!("Model'MyLevel.{}'", model_name));
            actor.brush = Some(Self::convert_brush_to_t3d(&mover.brush, model_name));
            map.actors.push(actor);
        }

        // 4. Static Meshes (OBJ Models placed as Semi-Solid / Additive Brushes)
        for (sm_idx, mesh) in snap.static_meshes.iter().enumerate() {
            let actor_name = if !mesh.name.is_empty() {
                mesh.name.clone()
            } else {
                format!("StaticMesh_{}", sm_idx)
            };
            let mut actor = T3dActor::new("Brush", actor_name);
            actor.set_location(mesh.location);
            if mesh.rotation != glam::Vec3::ZERO {
                let (pitch, yaw, roll) = deg_to_rotator(mesh.rotation);
                actor.set_property("Rotation", format!("(Pitch={},Yaw={},Roll={})", pitch, yaw, roll));
            }
            actor.set_property("CsgOper", "CSG_Add");
            actor.set_property("MainScale", "(SheerAxis=SHEER_ZX)");
            actor.set_property("PostScale", "(SheerAxis=SHEER_ZX)");
            actor.set_property("PolyFlags", ut99_bsp::PF_SEMISOLID.to_string()); // Canonical UE1 PF_Semisolid = 0x20 (32)
            let model_name = format!("StaticMeshModel{}", sm_idx);
            actor.set_property("Brush", format!("Model'MyLevel.{}'", model_name));
            actor.brush = Some(Self::convert_brush_to_t3d(mesh, model_name));
            map.actors.push(actor);
        }

        // 5. Actors (Lights, PlayerStarts, PathNodes, Weapons/Pickups, Triggers, Zones, etc.)
        if let Some(ref actors) = snap.loaded_unr_actors {
            for raw_actor in actors.iter() {
                let raw_cls_lower = raw_actor.class_name.to_ascii_lowercase();
                let short_raw_cls = raw_cls_lower.rsplit('.').next().unwrap_or(&raw_cls_lower);
                if short_raw_cls == "brush"
                    || short_raw_cls == "levelinfo"
                    || (raw_cls_lower.contains("mover") && raw_actor.brush_ref > 0)
                    || snap.movers.iter().any(|m| m.name == raw_actor.name)
                {
                    continue;
                }
                let baked_actors = if snap.target_engine == crate::editor_state::TargetEngine::Ut99Standard {
                    Self::bake_actor_for_ut99_standard(raw_actor)
                } else {
                    vec![raw_actor.clone()]
                };
                for actor in baked_actors {
                    let cls_lower = actor.class_name.to_ascii_lowercase();
                    let mut t3d_act = T3dActor::new(&actor.class_name, &actor.name);
                    t3d_act.set_location(actor.location);
                    if actor.rotation != (0, 0, 0) {
                        t3d_act.set_property(
                            "Rotation",
                            format!("(Pitch={},Yaw={},Roll={})", actor.rotation.0, actor.rotation.1, actor.rotation.2),
                        );
                    }
                    if !actor.tag.is_empty() {
                        t3d_act.set_property("Tag", actor.tag.clone());
                    }
                    if !actor.event.is_empty() {
                        t3d_act.set_property("Event", actor.event.clone());
                    }
                    if !actor.group.is_empty() {
                        t3d_act.set_property("Group", actor.group.clone());
                    }
                    if cls_lower.contains("light") {
                        t3d_act.set_property("LightBrightness", actor.light_brightness.to_string());
                        t3d_act.set_property("LightRadius", actor.light_radius.to_string());
                        t3d_act.set_property("LightHue", actor.light_hue.to_string());
                        t3d_act.set_property("LightSaturation", actor.light_saturation.to_string());
                        t3d_act.set_property("LightType", actor.light_type.to_string());
                        if actor.light_effect != 0 {
                            t3d_act.set_property("LightEffect", actor.light_effect.to_string());
                        }
                    }
                    let mut sorted_keys: Vec<&String> = actor.properties.keys().collect();
                    sorted_keys.sort();
                    for key in sorted_keys {
                        let prop = &actor.properties[key];
                        let k_low = prop.name.to_ascii_lowercase();
                        if matches!(
                            k_low.as_str(),
                            "location"
                                | "rotation"
                                | "tag"
                                | "event"
                                | "group"
                                | "lightbrightness"
                                | "lightradius"
                                | "lighthue"
                                | "lightsaturation"
                                | "lighttype"
                                | "lighteffect"
                                | "region"
                                | "level"
                                | "xlevel"
                                | "oldlocation"
                                | "collocation"
                                | "base"
                                | "deleted"
                                | "brush"
                        ) {
                            continue;
                        }
                        if let Some(val_str) = tagged_value_to_t3d(&prop.value) {
                            t3d_act.set_property(key.clone(), val_str);
                        } else if let Some(obj_path) = actor.object_paths.get(key) {
                            if !obj_path.is_empty() && obj_path != "None" {
                                t3d_act.set_property(key.clone(), obj_path.clone());
                            }
                        }
                    }
                    map.actors.push(t3d_act);
                }
            }
        } else if let Some(ref fallback) = snap.fallback_map_actors {
            for t3d_actor in fallback.iter() {
                let cls_lower = t3d_actor.class.to_ascii_lowercase();
                let short_cls = cls_lower.rsplit('.').next().unwrap_or(&cls_lower);
                if short_cls == "brush"
                    || short_cls == "levelinfo"
                    || (cls_lower.contains("mover") && t3d_actor.brush.is_some())
                    || snap.movers.iter().any(|m| m.name == t3d_actor.name)
                {
                    continue;
                }
                let mut cloned = t3d_actor.clone();
                if snap.target_engine == crate::editor_state::TargetEngine::Ut99Standard {
                    cloned.class = Self::map_class_name_for_ut99_standard(&cloned.class);
                }
                map.actors.push(cloned);
            }
        }

        map
    }

    /// Checks whether a directory is a valid root for Unreal Tournament 99 or Unreal Engine 1.
    pub fn is_valid_ut_dir(dir: &Path) -> bool {
        let sys = dir.join("System");
        dir.exists()
            && (sys.join("UCC.exe").exists()
                || sys.join("UnrealTournament.exe").exists()
                || sys.join("Unreal.exe").exists()
                || sys.join("Engine.u").exists()
                || sys.join("Core.u").exists()
                || sys.join("ut").exists()
                || sys.join("ut-bin").exists()
                || sys.join("ut-bin-amd64").exists())
    }

    /// Sets or clears a user-configured UT99 root directory override.
    pub fn set_custom_ut_dir(dir: Option<PathBuf>) {
        let lock = CUSTOM_UT_DIR.get_or_init(|| std::sync::RwLock::new(None));
        if let Ok(mut guard) = lock.write() {
            *guard = dir;
        }
    }

    /// Resolves a user-configured executable or directory path (e.g. `D:\UT99\System\UnrealTournament.exe`
    /// or `D:\UT99`) to a valid UT99 root directory and stores it in `CUSTOM_UT_DIR`.
    pub fn sync_from_configured_path(exe_or_dir: &str) -> Option<PathBuf> {
        let trimmed = exe_or_dir.trim();
        if trimmed.is_empty() {
            return None;
        }
        let p = PathBuf::from(trimmed);
        if Self::is_valid_ut_dir(&p) {
            Self::set_custom_ut_dir(Some(p.clone()));
            return Some(p);
        }
        if let Some(parent) = p.parent() {
            if Self::is_valid_ut_dir(parent) {
                let root = parent.to_path_buf();
                Self::set_custom_ut_dir(Some(root.clone()));
                return Some(root);
            }
            if let Some(grandparent) = parent.parent() {
                if Self::is_valid_ut_dir(grandparent) {
                    let root = grandparent.to_path_buf();
                    Self::set_custom_ut_dir(Some(root.clone()));
                    return Some(root);
                }
            }
        }
        None
    }

    /// Dynamically locates the UnrealTournament installation directory across:
    /// 0. User-configured directory override (`CUSTOM_UT_DIR` synced from Preferences / Welcome Wizard)
    /// 1. `UT99_DIR` environment variable
    /// 2. Current working directory and parent paths (up to 5 levels)
    /// 3. Windows Registry (Retail, Unreal Anthology, GOG Galaxy)
    /// 4. Common Steam library directories and default root installations
    /// 5. Standard Linux/Wine and macOS application paths
    pub fn find_ut_dir() -> Option<PathBuf> {
        // 0. User-configured override from Preferences / Welcome Wizard
        if let Some(lock) = CUSTOM_UT_DIR.get() {
            if let Ok(guard) = lock.read() {
                if let Some(ref custom) = *guard {
                    if Self::is_valid_ut_dir(custom) {
                        return Some(custom.clone());
                    }
                }
            }
        }

        // 1. Explicit environment variable
        if let Ok(env_dir) = std::env::var("UT99_DIR") {
            let candidate = PathBuf::from(env_dir);
            if Self::is_valid_ut_dir(&candidate) {
                return Some(candidate);
            }
        }

        // 2. Relative directories from current working dir
        if let Ok(mut current) = std::env::current_dir() {
            for _ in 0..5 {
                let candidate = current.join("UnrealTournament");
                if Self::is_valid_ut_dir(&candidate) {
                    return Some(candidate);
                }
                if !current.pop() {
                    break;
                }
            }
        }

        // 3. Windows Registry detection
        #[cfg(target_os = "windows")]
        {
            let reg_candidates = [
                ("HKLM\\SOFTWARE\\WOW6432Node\\Unreal Technology\\Installed Apps\\UnrealTournament", "Folder"),
                ("HKLM\\SOFTWARE\\Unreal Technology\\Installed Apps\\UnrealTournament", "Folder"),
                ("HKCU\\Software\\GOG.com\\Games\\1197920700", "PATH"),
                ("HKLM\\SOFTWARE\\GOG.com\\Games\\1197920700", "PATH"),
                ("HKLM\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1197920700", "PATH"),
            ];

            for (key, val) in reg_candidates {
                if let Some(folder) = Self::query_windows_registry(key, val) {
                    if Self::is_valid_ut_dir(&folder) {
                        return Some(folder);
                    }
                }
            }
        }

        // 4. Common standard install locations (Steam, GOG, root drives)
        let standard_candidates = [
            "D:\\UnrealTournament",
            "C:\\UnrealTournament",
            "E:\\UnrealTournament",
            "C:\\UT99",
            "D:\\UT99",
            "C:\\GOG Games\\Unreal Tournament",
            "D:\\GOG Games\\Unreal Tournament",
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Unreal Tournament",
            "D:\\SteamLibrary\\steamapps\\common\\Unreal Tournament",
            "E:\\SteamLibrary\\steamapps\\common\\Unreal Tournament",
            "C:\\Program Files (x86)\\Unreal Tournament",
            "C:\\Program Files\\Unreal Tournament",
        ];

        for cand in standard_candidates {
            let pb = PathBuf::from(cand);
            if Self::is_valid_ut_dir(&pb) {
                return Some(pb);
            }
        }

        // 5. Unix / Linux / macOS paths
        if let Ok(home) = std::env::var("HOME") {
            let home_p = PathBuf::from(home);
            let unix_candidates = [
                home_p.join(".wine/drive_c/UnrealTournament"),
                home_p.join(".local/share/Steam/steamapps/common/Unreal Tournament"),
                home_p.join("UnrealTournament"),
                home_p.join("Library/Application Support/Steam/steamapps/common/Unreal Tournament"),
                PathBuf::from("/opt/UnrealTournament"),
                PathBuf::from("/usr/local/games/UnrealTournament"),
                PathBuf::from("/Applications/Unreal Tournament"),
            ];
            for cand in unix_candidates {
                if Self::is_valid_ut_dir(&cand) {
                    return Some(cand);
                }
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn query_windows_registry(key: &str, value_name: &str) -> Option<PathBuf> {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("reg")
            .args(["query", key, "/v", value_name])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
            .ok()?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains(value_name) {
                    if let Some(pos) = line.find("REG_SZ") {
                        let path_str = line[pos + 6..].trim();
                        let pb = PathBuf::from(path_str);
                        if pb.exists() {
                            return Some(pb);
                        }
                    }
                }
            }
        }
        None
    }

    /// Packages (relative to `UnrealTournament/System`) that the headless editor must load before
    /// importing the T3D, otherwise textures / sounds / music are silently dropped (or the save crashes).
    fn engine_preload_packages(t3d_text: &str, ut_dir: &Path) -> Vec<String> {
        fn scan(text: &str, key: &str, out: &mut Vec<String>) {
            let mut rest = text;
            while let Some(pos) = rest.find(key) {
                let after = &rest[pos + key.len()..];
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && after[name.len()..].starts_with('.') && !out.contains(&name) {
                    out.push(name);
                }
                rest = after;
            }
        }
        let mut tex = Vec::new();
        let mut snd = Vec::new();
        let mut mus = Vec::new();
        scan(t3d_text, "Texture=", &mut tex);
        scan(t3d_text, "Sound'", &mut snd);
        scan(t3d_text, "Music'", &mut mus);
        let mut cmds = Vec::new();
        for (dir, ext, names) in [("Textures", "utx", tex), ("Sounds", "uax", snd), ("Music", "umx", mus)] {
            for n in names {
                if ut_dir.join(dir).join(format!("{n}.{ext}")).exists() {
                    cmds.push(format!("OBJ LOAD FILE=..\\{dir}\\{n}.{ext}"));
                }
            }
        }
        cmds
    }

    /// Adjusts the exported T3D so the real engine importer understands it:
    ///  - object-valued properties that were authored as strings become typed object references,
    ///  - a solid additive "world shell" is prepended: in UE1 everything not carved by a `Subtract`
    ///    brush is solid, and without a surrounding solid the carved rooms have no solid world to cut
    ///    (players fail to spawn, floors vanish).
    pub fn prepare_t3d_for_engine(state: &EditorState, map: &mut T3dMap) {
        let snapshot = T3dExportSnapshot::capture(state);
        Self::prepare_t3d_for_engine_from_snapshot(&snapshot, map);
    }

    pub fn prepare_t3d_for_engine_from_snapshot(snap: &T3dExportSnapshot, map: &mut T3dMap) {
        const SOUND_PROPS: [&str; 6] = [
            "AmbientSound",
            "OpeningSound",
            "OpenedSound",
            "ClosingSound",
            "ClosedSound",
            "MoveAmbientSound",
        ];
        let unquote = |v: &str| v.trim().trim_matches('"').to_string();
        for actor in &mut map.actors {
            if actor.class.eq_ignore_ascii_case("LevelInfo") {
                if let Some(song) = actor.get_property("Song").map(|s| s.to_string()) {
                    let s = unquote(&song);
                    if !s.contains('\'') && !s.is_empty() {
                        let s = if s.eq_ignore_ascii_case("Superfist") || s.eq_ignore_ascii_case("Superfist.Superfist") {
                            "Suprfist.Suprfist".to_string()
                        } else {
                            s
                        };
                        let qualified = if s.contains('.') { s } else { format!("{s}.{s}") };
                        actor.set_property("Song", format!("Music'{qualified}'"));
                    }
                }
            }
            for key in SOUND_PROPS {
                if let Some(v) = actor.get_property(key).map(|s| s.to_string()) {
                    let s = unquote(&v);
                    if !s.contains('\'') && !s.is_empty() && s != "None" {
                        actor.set_property(key, format!("Sound'{s}'"));
                    }
                }
            }
        }

        // World shell
        let first_is_add = snap
            .world_brushes
            .first()
            .map_or(false, |b| b.csg_oper == CsgOperation::Add);
        if first_is_add {
            return;
        }
        let mut min = glam::Vec3::splat(f32::MAX);
        let mut max = glam::Vec3::splat(f32::MIN);
        let mut any = false;
        let all_brushes = snap
            .world_brushes
            .iter()
            .chain(snap.movers.iter().map(|m| &m.brush))
            .chain(snap.static_meshes.iter());
        for b in all_brushes {
            if let Some(bb) = ut99_bsp::placement::brush_aabb(b) {
                min = min.min(bb.min);
                max = max.max(bb.max);
                any = true;
            }
        }
        if let Some(ref actors) = snap.loaded_unr_actors {
            for a in actors.iter() {
                min = min.min(a.location);
                max = max.max(a.location);
                any = true;
            }
        }
        if !any {
            min = glam::Vec3::splat(-1024.0);
            max = glam::Vec3::splat(1024.0);
        }
        let margin = 1024.0;
        let size = (max - min) + glam::Vec3::splat(margin * 2.0);
        let mut shell = ut99_bsp::build_cube("WorldShell", CsgOperation::Add, size);
        for p in &mut shell.polys {
            p.texture = "Indus1.Indus_B01".to_string();
        }
        shell.location = (min + max) * 0.5;
        let mut actor = T3dActor::new("Brush", "WorldShell");
        actor.set_location(shell.location);
        actor.set_property("CsgOper", "CSG_Add");
        actor.set_property("MainScale", "(SheerAxis=SHEER_ZX)");
        actor.set_property("PostScale", "(SheerAxis=SHEER_ZX)");
        actor.set_property("Brush", "Model'MyLevel.WorldShellModel'");
        // Polygons carry local coordinates relative to `location`
        actor.brush = Some(Self::convert_brush_to_t3d(&shell, "WorldShellModel".to_string()));
        let insert_at = map
            .actors
            .iter()
            .position(|a| a.class.eq_ignore_ascii_case("LevelInfo"))
            .map_or(0, |i| i + 1);
        map.actors.insert(insert_at, actor);
    }

    /// Runs `ucc exec <script>` in the UT99 `System` directory with a timeout. The engine is very
    /// chatty, so its output goes to `log_path` (a pipe could fill up and stall the child).
    /// Returns `(timed_out, exit_success, log_text)`.
    fn run_ucc_exec(
        ucc_path: &Path,
        sys_dir: &Path,
        script_name: &str,
        log_path: &Path,
        timeout: std::time::Duration,
    ) -> Result<(bool, bool, String)> {
        let log_file = fs::File::create(log_path)?;
        let mut child = Command::new(ucc_path)
            .arg("exec")
            .arg(script_name)
            .current_dir(sys_dir)
            .stdout(std::process::Stdio::from(log_file))
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let started = std::time::Instant::now();
        let mut timed_out = false;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if started.elapsed() > timeout {
                        let _ = child.kill();
                        timed_out = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error.into());
                }
            }
        }
        let status = child.wait()?;
        let text = fs::read(log_path)
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .unwrap_or_default();
        Ok((timed_out, status.success(), text))
    }

    /// Short human summary of an engine log: the crash history if any, otherwise the last lines.
    fn engine_log_tail(log: &str) -> String {
        if let Some(h) = log.lines().find(|l| l.starts_with("History:")) {
            return h.chars().take(240).collect();
        }
        let lines: Vec<&str> = log
            .lines()
            .filter(|l| !l.contains("Failed to resolve object value for defaultproperty") && !l.trim().is_empty())
            .collect();
        lines[lines.len().saturating_sub(6)..].join(" | ")
    }

    fn cleanup_pass_warning(result: Result<(bool, bool, String)>, output_size: u64) -> Option<String> {
        match result {
            Ok((false, true, log)) if output_size > 0 && log.contains(" 0 error(s)") => None,
            Ok((timed_out, exit_success, log)) => Some(format!(
                "Camera cleanup pass failed (timeout={timed_out}, exit_success={exit_success}; {}); using pass-1 map",
                Self::engine_log_tail(&log)
            )),
            Err(error) => Some(format!("Camera cleanup pass could not run ({error}); using pass-1 map")),
        }
    }

    /// Builds the `.unr` with the REAL UT99 editor engine (`ucc exec`): import T3D, rebuild CSG + BSP
    /// (zones, portals, collision hulls), build the AI path graph, compute static lighting, save.
    /// Engine compilation alone does not establish gameplay validity or successful runtime loading.
    ///
    /// Notes learned from the engine:
    ///  - Packages used by the map must be loaded with `OBJ LOAD` before `MAP IMPORT`.
    ///  - `LIGHT APPLY` opens a temporary viewport and leaves a `Camera0` actor in the level whose
    ///    `Player` pointer dangles; saving it can crash the engine (intermittently) and the camera would
    ///    otherwise end up inside the shipped map. So pass 1 builds + lights + saves (retrying on a crash)
    ///    and pass 2 reloads the saved map in a clean session, deletes the camera and saves the final map
    ///    (BSP, ReachSpecs and lightmaps are preserved by the engine's own load/save).
    ///
    /// Returns an error (so callers can fall back to the native writer) if the engine fails, times out
    /// or produces an empty file.
    pub fn export_prepared_t3d(state: &EditorState) -> Result<(Vec<u8>, String)> {
        let snapshot = T3dExportSnapshot::capture(state);
        Self::export_prepared_t3d_from_snapshot(&snapshot)
    }

    pub fn export_prepared_t3d_from_snapshot(snap: &T3dExportSnapshot) -> Result<(Vec<u8>, String)> {
        let mut t3d = Self::export_t3d_map_from_snapshot(snap);
        Self::prepare_t3d_for_engine_from_snapshot(snap, &mut t3d);
        let mut buf = Vec::new();
        ut99_format::write_t3d(&mut buf, &t3d)?;
        let t3d_text = String::from_utf8_lossy(&buf).to_string();
        Ok((buf, t3d_text))
    }

    pub fn build_unr_with_engine(state: &EditorState, ut_dir: &Path, dest_path: &Path) -> Result<PathBuf> {
        let (buf, t3d_text) = Self::export_prepared_t3d(state)?;
        Self::build_unr_with_engine_from_t3d(ut_dir, dest_path, &buf, &t3d_text, None)
    }

    pub fn build_unr_with_engine_from_t3d(
        ut_dir: &Path,
        dest_path: &Path,
        buf: &[u8],
        t3d_text: &str,
        progress_tx: Option<&std::sync::mpsc::Sender<crate::editor_state::SaveWorkerMessage>>,
    ) -> Result<PathBuf> {
        Ok(Self::build_unr_with_engine_from_t3d_report(ut_dir, dest_path, buf, t3d_text, progress_tx)?.0)
    }

    fn build_unr_with_engine_from_t3d_report(
        ut_dir: &Path,
        dest_path: &Path,
        buf: &[u8],
        t3d_text: &str,
        progress_tx: Option<&std::sync::mpsc::Sender<crate::editor_state::SaveWorkerMessage>>,
    ) -> Result<(PathBuf, Option<String>)> {
        let sys_dir = ut_dir.join("System");
        let ucc_path = sys_dir.join("UCC.exe");
        if !ucc_path.exists() {
            anyhow::bail!("UCC.exe not found at {:?}", ucc_path);
        }
        // Two engine instances share `Maps\Save.tmp` and the log: never run two builds at once
        // (neither threads of this process nor another editor/test process).
        let _thread_guard = ENGINE_BUILD_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let _file_guard = EngineFileLock::acquire(&sys_dir)?;

        let tag = format!("_wz{}", std::process::id());
        let maps_dir = ut_dir.join("Maps");
        let stage_t3d = sys_dir.join(format!("{tag}.t3d"));
        let script1 = sys_dir.join(format!("{tag}.exec"));
        let script2 = sys_dir.join(format!("{tag}b.exec"));
        let log1 = sys_dir.join(format!("{tag}.log"));
        let log2 = sys_dir.join(format!("{tag}b.log"));
        let pass1_abs = maps_dir.join(format!("{tag}.unr"));
        let pass2_abs = maps_dir.join(format!("{tag}b.unr"));
        let _ = fs::create_dir_all(&maps_dir);
        fs::write(&stage_t3d, buf)?;

        if let Some(tx) = progress_tx {
            let _ = tx.send(crate::editor_state::SaveWorkerMessage::Stage {
                current_stage: 2,
                total_stages: 4,
                stage_message: "Rebuilding BSP, Zones, Portals and Bot Paths (UCC.exe Pass 1)...".to_string(),
            });
        }

        // ---- Pass 1: import, build, light, save
        let mut s1 = Self::engine_preload_packages(t3d_text, ut_dir);
        s1.push(format!("MAP IMPORT FILE={tag}.t3d"));
        s1.push("MAP REBUILD".to_string());
        s1.push("BSP REBUILD OPTIMAL=1 BALANCE=15 PORTALBIAS=70 ZONES=1".to_string());
        s1.push("PATHS DEFINE".to_string());
        s1.push("LIGHT APPLY".to_string());
        s1.push("SET Camera Player None".to_string());
        s1.push(format!("MAP SAVE FILE=..\\Maps\\{tag}.unr"));
        fs::write(&script1, s1.join("\r\n") + "\r\n")?;

        let mut last_err = String::from("unknown engine failure");
        let mut pass1_ok = false;
        for attempt in 1..=3 {
            let _ = fs::remove_file(&pass1_abs);
            let (timed_out, exit_success, log) = match Self::run_ucc_exec(
                &ucc_path,
                &sys_dir,
                &format!("{tag}.exec"),
                &log1,
                std::time::Duration::from_secs(240),
            ) {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("failed to run UCC.exe: {e}");
                    break;
                }
            };
            let size = fs::metadata(&pass1_abs).map(|m| m.len()).unwrap_or(0);
            if !timed_out && exit_success && size > 0 && log.contains(" 0 error(s)") {
                pass1_ok = true;
                break;
            }
            last_err = if timed_out {
                format!("UCC.exe timed out (attempt {attempt})")
            } else {
                format!(
                    "UCC.exe did not produce a valid map (attempt {attempt}, exit_success={exit_success}, {size} bytes): {}",
                    Self::engine_log_tail(&log)
                )
            };
            tracing::warn!("{last_err}");
        }

        // ---- Pass 2: clean session, remove the editor camera, final save
        let mut final_src = pass1_abs.clone();
        let mut engine_warning = None;
        if pass1_ok {
            if let Some(tx) = progress_tx {
                let _ = tx.send(crate::editor_state::SaveWorkerMessage::Stage {
                    current_stage: 3,
                    total_stages: 4,
                    stage_message: "Applying lightmaps and finalizing level (UCC.exe Pass 2)...".to_string(),
                });
            }
            let mut s2 = vec![format!("MAP LOAD FILE=..\\Maps\\{tag}.unr"), "ACTOR SELECT NONE".to_string()];
            for n in 0..4 {
                s2.push(format!("ACTOR SELECT NAME=Camera{n}"));
            }
            s2.push("ACTOR DELETE".to_string());
            s2.push(format!("MAP SAVE FILE=..\\Maps\\{tag}b.unr"));
            fs::write(&script2, s2.join("\r\n") + "\r\n")?;
            let _ = fs::remove_file(&pass2_abs);
            let result = Self::run_ucc_exec(
                &ucc_path,
                &sys_dir,
                &format!("{tag}b.exec"),
                &log2,
                std::time::Duration::from_secs(120),
            );
            let size = fs::metadata(&pass2_abs).map(|m| m.len()).unwrap_or(0);
            engine_warning = Self::cleanup_pass_warning(result, size);
            if let Some(warning) = &engine_warning {
                tracing::warn!("{warning}");
            } else {
                final_src = pass2_abs.clone();
            }
        }

        if let Some(tx) = progress_tx {
            let _ = tx.send(crate::editor_state::SaveWorkerMessage::Stage {
                current_stage: 4,
                total_stages: 4,
                stage_message: "Writing and verifying final .UNR package on disk...".to_string(),
            });
        }

        let result = if pass1_ok {
            if let Some(parent) = dest_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&final_src, dest_path)
                .map(|_| dest_path.to_path_buf())
                .map_err(anyhow::Error::from)
        } else {
            Err(anyhow::anyhow!(last_err))
        };
        // `UT99_KEEP_ENGINE_FILES=1` keeps the staged T3D / scripts / logs for debugging engine failures.
        if std::env::var("UT99_KEEP_ENGINE_FILES").map_or(true, |v| v != "1") {
            for p in [&stage_t3d, &script1, &script2, &log1, &log2, &pass1_abs, &pass2_abs] {
                let _ = fs::remove_file(p);
            }
        }
        result.map(|path| (path, engine_warning))
    }

    /// True when the map owns embedded MyLevel resources that the T3D route cannot carry
    /// (textures / sounds): such maps must be written by the native serializer.
    pub fn has_embedded_assets(state: &EditorState) -> bool {
        state
            .loaded_unr
            .as_ref()
            .map_or(false, |u| !u.embedded_textures.is_empty() || !u.embedded_sounds.is_empty())
    }

    /// Safely saves the map to a .unr file with automatic .bak backup
    pub fn save_unr_safe(state: &EditorState, dest_path: &Path) -> Result<PathBuf> {
        Ok(Self::save_unr_with_report(state, dest_path)?.path)
    }

    pub fn save_unr_with_report(state: &EditorState, dest_path: &Path) -> Result<UnrSaveReport> {
        let ut_dir = match Self::find_ut_dir() {
            Some(dir) => dir,
            None => {
                tracing::info!("UnrealTournament UCC not found; saving via native Rust .unr serializer");
                return Ok(UnrSaveReport { path: Self::save_unr_native(state, dest_path)?, backend: UnrSaveBackend::Native, native_reason: Some("UCC installation not found".into()), engine_warning: None });
            }
        };
        let sys_dir = ut_dir.join("System");
        let maps_dir = ut_dir.join("Maps");
        let ucc_path = sys_dir.join("UCC.exe");

        if !ucc_path.exists() {
            tracing::info!("UCC.exe not found at {:?}; saving via native Rust .unr serializer", ucc_path);
            return Ok(UnrSaveReport { path: Self::save_unr_native(state, dest_path)?, backend: UnrSaveBackend::Native, native_reason: Some("UCC.exe not found".into()), engine_warning: None });
        }

        let dest_path = if dest_path.is_relative() {
            if dest_path.parent().map_or(true, |p| p.as_os_str().is_empty()) {
                maps_dir.join(dest_path)
            } else {
                std::env::current_dir()?.join(dest_path)
            }
        } else {
            dest_path.to_path_buf()
        };

        // 1. Auto Backup if file already exists
        backup_existing_map(&dest_path)?;

        // 2. Prefer the real UT99 editor engine (BSP, zones, hulls, ReachSpecs, lightmaps). Maps that carry
        //    embedded MyLevel textures/sounds, or an explicit opt-out, use the native serializer.
        let force_native = std::env::var("UT99_NATIVE_SAVE").map_or(false, |v| v == "1");
        let embedded = Self::has_embedded_assets(state);
        let reason = if force_native {
            "Native writer explicitly selected".to_string()
        } else if embedded {
            "Embedded MyLevel assets require the native writer".to_string()
        } else {
            match Self::export_prepared_t3d(state).and_then(|(buf, t3d_text)|
                Self::build_unr_with_engine_from_t3d_report(&ut_dir, &dest_path, &buf, &t3d_text, None)
            ) {
                Ok((path, engine_warning)) => {
                    tracing::info!("Built UNR map with the UT99 editor engine: {:?}", path);
                    return Ok(UnrSaveReport { path, backend: UnrSaveBackend::Engine, native_reason: None, engine_warning });
                }
                Err(e) => {
                    tracing::warn!("Engine build failed ({e}); falling back to native Rust .UNR writer");
                    format!("Engine build failed: {e:#}")
                }
            }
        };

        // 3. Native Rust writer (fallback)
        Ok(UnrSaveReport { path: Self::save_unr_native(state, &dest_path)?, backend: UnrSaveBackend::Native, native_reason: Some(reason), engine_warning: None })
    }

    /// Build in a sibling directory, validate the package, then replace the
    /// destination. A validation or backup failure leaves the destination intact.
    pub fn save_unr_verified(state: &EditorState, dest_path: &Path) -> Result<UnrSaveReport> {
        let dest = if dest_path.is_absolute() { dest_path.to_path_buf() } else { std::env::current_dir()?.join(dest_path) };
        let parent = dest.parent().ok_or_else(|| anyhow::anyhow!("Destination has no parent"))?;
        let filename = dest.file_name().ok_or_else(|| anyhow::anyhow!("Destination has no file name"))?;
        fs::create_dir_all(parent)?;
        let stage = UnrStageDir::create(parent)?;
        let staged_path = stage.0.join(filename);
        let mut report = Self::save_unr_with_report(state, &staged_path)?;
        validate_and_commit_unr(&report.path, &dest)?;
        report.path = dest;
        Ok(report)
    }

    /// Exports the current EditorState directly to a native 64-bit binary .UNR package without UCC!
    pub fn save_unr_native(state: &EditorState, dest_path: &Path) -> Result<PathBuf> {
        let dest_path = if dest_path.is_relative() {
            std::env::current_dir()?.join(dest_path)
        } else {
            dest_path.to_path_buf()
        };

        if let Some(parent) = dest_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // 1. Auto Backup if file already exists
        backup_existing_map(&dest_path)?;

        let level = Self::build_unr_level_from_state(state, &dest_path)?;
        level.save(&dest_path)?;
        tracing::info!("Successfully saved native .UNR package to {:?}", dest_path);
        Ok(dest_path)
    }

    /// Pure in-memory synthesis of an `UnrLevel` from `EditorState` (Thread-safe, `Send + 'static`).
    pub fn build_unr_level_from_state(state: &EditorState, dest_path: &Path) -> Result<ut99_format::UnrLevel> {
        let snapshot = UnrExportSnapshot::capture(state);
        Self::build_unr_level_from_snapshot(&snapshot, dest_path)
    }

    /// Pure in-memory synthesis of an `UnrLevel` from an off-thread `UnrExportSnapshot`.
    pub fn build_unr_level_from_snapshot(snap: &UnrExportSnapshot, dest_path: &Path) -> Result<ut99_format::UnrLevel> {
        // Build UnrLevel from snapshot
        let t3d = &snap.t3d;
        let mut level = ut99_format::UnrLevel::default();
        level.map_name = dest_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "MyLevel".to_string());

        // 2a. Synthesize LevelInfo0 so Level Properties (Title, Author, Song, GameType, Ambient) persist in native .unr
        let mut level_info = ut99_format::ActorRecord {
            name: "LevelInfo0".to_string(),
            class_name: "LevelInfo".to_string(),
            flags: 0x02000004,
            ..Default::default()
        };
        if !t3d.level_title.is_empty() {
            level_info.properties.insert(
                "Title".to_string(),
                TaggedProperty {
                    name: "Title".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(t3d.level_title.clone()),
                },
            );
        }
        if !t3d.level_author.is_empty() {
            level_info.properties.insert(
                "Author".to_string(),
                TaggedProperty {
                    name: "Author".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(t3d.level_author.clone()),
                },
            );
        }
        if !t3d.level_ideal_player_count.is_empty() {
            level_info.properties.insert(
                "IdealPlayerCount".to_string(),
                TaggedProperty {
                    name: "IdealPlayerCount".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(t3d.level_ideal_player_count.clone()),
                },
            );
        }
        if !t3d.level_song.is_empty() {
            level_info.properties.insert(
                "Song".to_string(),
                TaggedProperty {
                    name: "Song".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(t3d.level_song.clone()),
                },
            );
        }
        if !t3d.level_game_type.is_empty() {
            level_info.properties.insert(
                "DefaultGameType".to_string(),
                TaggedProperty {
                    name: "DefaultGameType".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(t3d.level_game_type.clone()),
                },
            );
        }
        if !t3d.level_enter_text.is_empty() {
            level_info.properties.insert(
                "LevelEnterText".to_string(),
                TaggedProperty {
                    name: "LevelEnterText".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(t3d.level_enter_text.clone()),
                },
            );
        }
        if t3d.level_ambient_brightness > 0 {
            level_info.properties.insert(
                "AmbientBrightness".to_string(),
                TaggedProperty {
                    name: "AmbientBrightness".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(t3d.level_ambient_brightness),
                },
            );
            level_info.properties.insert(
                "AmbientHue".to_string(),
                TaggedProperty {
                    name: "AmbientHue".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(t3d.level_ambient_hue),
                },
            );
            level_info.properties.insert(
                "AmbientSaturation".to_string(),
                TaggedProperty {
                    name: "AmbientSaturation".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(t3d.level_ambient_saturation),
                },
            );
        }
        if snap.has_mylevel_screenshot {
            level_info.properties.insert(
                "Screenshot".to_string(),
                TaggedProperty {
                    name: "Screenshot".to_string(),
                    array_index: 0,
                    value: TaggedValue::Object(0),
                },
            );
            level_info.object_paths.insert(
                "Screenshot".to_string(),
                "Texture'MyLevel.Screenshot'".to_string(),
            );
        }
        level.other_actors.push(level_info);

        // Convert world brushes
        for (i, b) in t3d.world_brushes.iter().enumerate() {
            if !b.name.is_empty() && t3d.movers.iter().any(|m| m.name == b.name) {
                continue;
            }
            let mut raw_polys = Vec::new();
            let mut poly_tex_names = Vec::new();
            for p in &b.polys {
                poly_tex_names.push(p.texture.clone());
                raw_polys.push(ut99_format::RawPoly {
                    base: p.base,
                    normal: p.plane.normal,
                    texture_u: p.texture_u,
                    texture_v: p.texture_v,
                    vertices: p.vertices.clone(),
                    poly_flags: p.flags,
                    actor_ref: 0,
                    texture_ref: 0,
                    item_name: 0,
                    link_index: 0,
                    brush_poly_index: 0,
                    pan_u: p.pan_u,
                    pan_v: p.pan_v,
                });
            }
            let csg_oper = match b.csg_oper {
                ut99_bsp::CsgOperation::Add => 1,
                ut99_bsp::CsgOperation::Subtract => 2,
                _ => 1,
            };
            let actor_record = ut99_format::ActorRecord {
                export_index: 0,
                name: if b.name.is_empty() { format!("Brush_{i}") } else { b.name.clone() },
                class_name: "Brush".to_string(),
                flags: 0x02000004,
                location: b.location,
                rotation: deg_to_rotator(b.rotation),
                prepivot: glam::Vec3::ZERO,
                main_scale: b.scale,
                post_scale: glam::Vec3::ONE,
                group: b
                    .group
                    .clone()
                    .or_else(|| t3d.brush_groups.get(&i).cloned())
                    .unwrap_or_default(),
                brush_ref: 0,
                csg_oper,
                poly_flags: b.polys.first().map(|p| p.flags).unwrap_or(0),
                ..Default::default()
            };
            let world_raw_polys: Vec<ut99_format::RawPoly> = b
                .to_world_polys()
                .into_iter()
                .map(|wp| ut99_format::RawPoly {
                    base: wp.base,
                    normal: wp.plane.normal,
                    texture_u: wp.texture_u,
                    texture_v: wp.texture_v,
                    vertices: wp.vertices,
                    poly_flags: wp.flags,
                    actor_ref: 0,
                    texture_ref: 0,
                    item_name: 0,
                    link_index: 0,
                    brush_poly_index: 0,
                    pan_u: wp.pan_u,
                    pan_v: wp.pan_v,
                })
                .collect();
            level.brush_poly_textures.push(poly_tex_names);
            level.brushes.push(ut99_format::UnrBrush {
                actor: actor_record,
                local_polys: ut99_format::RawPolys { polys: raw_polys },
                world_polys: world_raw_polys,
            });
        }

        // Convert movers with all 8 keyframes and Mover properties
        for (i, m) in t3d.movers.iter().enumerate() {
            let mut raw_polys = Vec::new();
            let mut poly_tex_names = Vec::new();
            for p in &m.brush.polys {
                poly_tex_names.push(p.texture.clone());
                raw_polys.push(ut99_format::RawPoly {
                    base: p.base,
                    normal: p.plane.normal,
                    texture_u: p.texture_u,
                    texture_v: p.texture_v,
                    vertices: p.vertices.clone(),
                    poly_flags: p.flags,
                    actor_ref: 0,
                    texture_ref: 0,
                    item_name: 0,
                    link_index: 0,
                    brush_poly_index: 0,
                    pan_u: p.pan_u,
                    pan_v: p.pan_v,
                });
            }
            let mut mover_props = std::collections::HashMap::new();
            let mut mover_obj_paths = std::collections::HashMap::new();
            mover_props.insert(
                "bDynamicLight".to_string(),
                TaggedProperty {
                    name: "bDynamicLight".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );
            mover_props.insert(
                "bStatic".to_string(),
                TaggedProperty {
                    name: "bStatic".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(false),
                },
            );
            if !m.initial_state.is_empty() {
                mover_props.insert(
                    "InitialState".to_string(),
                    TaggedProperty {
                        name: "InitialState".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(m.initial_state.clone()),
                    },
                );
            }
            mover_props.insert(
                "StayOpenTime".to_string(),
                TaggedProperty {
                    name: "StayOpenTime".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(m.stay_open_time),
                },
            );
            mover_props.insert(
                "MoveTime".to_string(),
                TaggedProperty {
                    name: "MoveTime".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(m.move_time),
                },
            );
            if m.delay_time > 0.0 {
                mover_props.insert(
                    "DelayTime".to_string(),
                    TaggedProperty {
                        name: "DelayTime".to_string(),
                        array_index: 0,
                        value: TaggedValue::Float(m.delay_time),
                    },
                );
            }
            mover_props.insert(
                "KeyNum".to_string(),
                TaggedProperty {
                    name: "KeyNum".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(m.current_key as u8),
                },
            );
            mover_props.insert(
                "BrushRaytraceKey".to_string(),
                TaggedProperty {
                    name: "BrushRaytraceKey".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(m.current_key as u8),
                },
            );
            mover_props.insert(
                "WorldRaytraceKey".to_string(),
                TaggedProperty {
                    name: "WorldRaytraceKey".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(0),
                },
            );
            mover_props.insert(
                "bDynamicLightMover".to_string(),
                TaggedProperty {
                    name: "bDynamicLightMover".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );
            let num_keys = m.keyframes.iter().filter(|k| k.is_some()).count().max(1) as u8;
            mover_props.insert(
                "NumKeys".to_string(),
                TaggedProperty {
                    name: "NumKeys".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(num_keys),
                },
            );
            if !m.glide_type.is_empty() {
                mover_props.insert(
                    "MoverGlideType".to_string(),
                    TaggedProperty {
                        name: "MoverGlideType".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(m.glide_type.clone()),
                    },
                );
            }
            if !m.encroach_type.is_empty() {
                mover_props.insert(
                    "MoverEncroachType".to_string(),
                    TaggedProperty {
                        name: "MoverEncroachType".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(m.encroach_type.clone()),
                    },
                );
            }
            for (snd_key, snd_val) in [
                ("OpeningSound", &m.opening_sound),
                ("OpenedSound", &m.opened_sound),
                ("ClosingSound", &m.closing_sound),
                ("ClosedSound", &m.closed_sound),
                ("MoveAmbientSound", &m.move_ambient_sound),
            ] {
                if !snd_val.is_empty() {
                    mover_props.insert(
                        snd_key.to_string(),
                        TaggedProperty {
                            name: snd_key.to_string(),
                            array_index: 0,
                            value: TaggedValue::Object(0),
                        },
                    );
                    let snd_path = if snd_val.contains('\'') {
                        snd_val.clone()
                    } else {
                        format!("Sound'{}'", snd_val)
                    };
                    mover_obj_paths.insert(snd_key.to_string(), snd_path);
                }
            }
            if let Some(kf0) = &m.keyframes[0] {
                for k_idx in 1..8 {
                    if let Some(kfi) = &m.keyframes[k_idx] {
                        let delta_pos = kfi.location - kf0.location;
                        mover_props.insert(
                            format!("KeyPos({})", k_idx),
                            TaggedProperty {
                                name: "KeyPos".to_string(),
                                array_index: k_idx as i32,
                                value: TaggedValue::Vector(delta_pos),
                            },
                        );
                        let delta_rot = kfi.rotation - kf0.rotation;
                        if delta_rot != glam::Vec3::ZERO {
                            let (pitch, yaw, roll) = deg_to_rotator(delta_rot);
                            mover_props.insert(
                                format!("KeyRot({})", k_idx),
                                TaggedProperty {
                                    name: "KeyRot".to_string(),
                                    array_index: k_idx as i32,
                                    value: TaggedValue::Rotator { pitch, yaw, roll },
                                },
                            );
                        }
                    }
                }
            }

            let mover_cls = if m.mover_class.trim().is_empty() {
                "Mover".to_string()
            } else {
                m.mover_class.trim().to_string()
            };
            let actor_record = ut99_format::ActorRecord {
                export_index: 0,
                name: if m.name.is_empty() { format!("Mover_{i}") } else { m.name.clone() },
                class_name: mover_cls,
                flags: 0x02000004,
                location: m.brush.location,
                rotation: deg_to_rotator(m.brush.rotation),
                main_scale: m.brush.scale,
                post_scale: glam::Vec3::ONE,
                tag: m.tag.clone(),
                event: m.event.clone(),
                csg_oper: 0, // CSG_Active for Movers
                poly_flags: m.brush.polys.first().map(|p| p.flags).unwrap_or(0),
                properties: mover_props,
                object_paths: mover_obj_paths,
                ..Default::default()
            };
            let mover_world_polys: Vec<ut99_format::RawPoly> = m
                .brush
                .to_world_polys()
                .into_iter()
                .map(|wp| ut99_format::RawPoly {
                    base: wp.base,
                    normal: wp.plane.normal,
                    texture_u: wp.texture_u,
                    texture_v: wp.texture_v,
                    vertices: wp.vertices,
                    poly_flags: wp.flags,
                    actor_ref: 0,
                    texture_ref: 0,
                    item_name: 0,
                    link_index: 0,
                    brush_poly_index: 0,
                    pan_u: 0,
                    pan_v: 0,
                })
                .collect();
            level.brush_poly_textures.push(poly_tex_names);
            level.brushes.push(ut99_format::UnrBrush {
                actor: actor_record,
                local_polys: ut99_format::RawPolys { polys: raw_polys },
                world_polys: mover_world_polys,
            });
        }

        // Convert static meshes (as SemiSolid additive brushes: PF_SEMISOLID = 0x20)
        for (i, sm) in t3d.static_meshes.iter().enumerate() {
            let mut raw_polys = Vec::new();
            let mut poly_tex_names = Vec::new();
            for p in &sm.polys {
                poly_tex_names.push(p.texture.clone());
                raw_polys.push(ut99_format::RawPoly {
                    base: p.base,
                    normal: p.plane.normal,
                    texture_u: p.texture_u,
                    texture_v: p.texture_v,
                    vertices: p.vertices.clone(),
                    poly_flags: p.flags | PF_SEMISOLID,
                    actor_ref: 0,
                    texture_ref: 0,
                    item_name: 0,
                    link_index: 0,
                    brush_poly_index: 0,
                    pan_u: 0,
                    pan_v: 0,
                });
            }
            let actor_record = ut99_format::ActorRecord {
                export_index: 0,
                name: if sm.name.is_empty() { format!("StaticMesh_{i}") } else { sm.name.clone() },
                class_name: "Brush".to_string(),
                flags: 0x02000004,
                location: sm.location,
                rotation: deg_to_rotator(sm.rotation),
                main_scale: sm.scale,
                post_scale: glam::Vec3::ONE,
                csg_oper: 1,
                poly_flags: PF_SEMISOLID,
                ..Default::default()
            };
            let sm_world_polys: Vec<ut99_format::RawPoly> = sm
                .to_world_polys()
                .into_iter()
                .map(|wp| ut99_format::RawPoly {
                    base: wp.base,
                    normal: wp.plane.normal,
                    texture_u: wp.texture_u,
                    texture_v: wp.texture_v,
                    vertices: wp.vertices,
                    poly_flags: wp.flags | PF_SEMISOLID,
                    actor_ref: 0,
                    texture_ref: 0,
                    item_name: 0,
                    link_index: 0,
                    brush_poly_index: 0,
                    pan_u: 0,
                    pan_v: 0,
                })
                .collect();
            level.brush_poly_textures.push(poly_tex_names);
            level.brushes.push(ut99_format::UnrBrush {
                actor: actor_record,
                local_polys: ut99_format::RawPolys { polys: raw_polys },
                world_polys: sm_world_polys,
            });
        }

        // Copy existing actors if loaded from unr, or fallback from state.map.actors if imported from T3D
        if let Some(ref loaded_actors) = t3d.loaded_unr_actors {
            if let Some(ref pkg) = snap.loaded_package {
                level.package = pkg.clone();
            }
            for raw_actor in loaded_actors.iter() {
                let raw_cls_lower = raw_actor.class_name.to_ascii_lowercase();
                let short_raw_cls = raw_cls_lower.rsplit('.').next().unwrap_or(&raw_cls_lower);
                if short_raw_cls == "brush"
                    || short_raw_cls == "levelinfo"
                    || (raw_cls_lower.contains("mover") && raw_actor.brush_ref > 0)
                    || t3d.movers.iter().any(|m| m.name == raw_actor.name)
                {
                    continue;
                }
                let baked_actors = if t3d.target_engine == crate::editor_state::TargetEngine::Ut99Standard {
                    Self::bake_actor_for_ut99_standard(raw_actor)
                } else {
                    vec![raw_actor.clone()]
                };
                for mut actor in baked_actors {
                    let cls_lower = actor.class_name.to_ascii_lowercase();
                    if cls_lower.contains("mover") {
                        // TASK-06: Every Mover in UT99 MUST own a valid Brush (UModel) with compiled BSP.
                        let fallback_brush = t3d
                            .world_brushes
                            .iter()
                            .find(|b| (b.location - actor.location).length_squared() < 4.0)
                            .cloned()
                            .unwrap_or_else(|| {
                                let mut b = ut99_bsp::build_cube(
                                    &actor.name,
                                    ut99_bsp::CsgOperation::Add,
                                    glam::Vec3::new(64.0, 64.0, 16.0),
                                );
                                b.location = actor.location;
                                b
                            });
                        let mut raw_polys = Vec::new();
                        let mut poly_tex_names = Vec::new();
                        for p in &fallback_brush.polys {
                            poly_tex_names.push(p.texture.clone());
                            raw_polys.push(ut99_format::RawPoly {
                                base: p.base,
                                normal: p.plane.normal,
                                texture_u: p.texture_u,
                                texture_v: p.texture_v,
                                vertices: p.vertices.clone(),
                                poly_flags: p.flags,
                                actor_ref: 0,
                                texture_ref: 0,
                                item_name: 0,
                                link_index: 0,
                                brush_poly_index: 0,
                                pan_u: 0,
                                pan_v: 0,
                            });
                        }
                        let world_raw_polys: Vec<ut99_format::RawPoly> = fallback_brush
                            .to_world_polys()
                            .into_iter()
                            .map(|wp| ut99_format::RawPoly {
                                base: wp.base,
                                normal: wp.plane.normal,
                                texture_u: wp.texture_u,
                                texture_v: wp.texture_v,
                                vertices: wp.vertices,
                                poly_flags: wp.flags,
                                actor_ref: 0,
                                texture_ref: 0,
                                item_name: 0,
                                link_index: 0,
                                brush_poly_index: 0,
                                pan_u: 0,
                                pan_v: 0,
                            })
                            .collect();
                        actor.csg_oper = 0; // CSG_Active for Movers
                        level.brush_poly_textures.push(poly_tex_names);
                        level.brushes.push(ut99_format::UnrBrush {
                            actor,
                            local_polys: ut99_format::RawPolys { polys: raw_polys },
                            world_polys: world_raw_polys,
                        });
                    } else if cls_lower.contains("light") {
                        level.lights.push(actor);
                    } else if cls_lower.contains("playerstart") {
                        level.player_starts.push(actor);
                    } else if cls_lower.contains("pathnode") || cls_lower.contains("lift") || cls_lower.contains("jump") {
                        level.path_nodes.push(actor);
                    } else {
                        level.other_actors.push(actor);
                    }
                }
            }
        } else if let Some(ref fallback) = t3d.fallback_map_actors {
            for t3d_actor in fallback.iter() {
                let cls_lower = t3d_actor.class.to_ascii_lowercase();
                let short_cls = cls_lower.rsplit('.').next().unwrap_or(&cls_lower);
                if short_cls == "brush"
                    || short_cls == "levelinfo"
                    || (cls_lower.contains("mover") && t3d_actor.brush.is_some())
                    || t3d.movers.iter().any(|m| m.name == t3d_actor.name)
                {
                    continue;
                }
                let mut rec = ut99_format::ActorRecord::default();
                rec.name = t3d_actor.name.clone();
                rec.class_name = if t3d.target_engine == crate::editor_state::TargetEngine::Ut99Standard {
                    Self::map_class_name_for_ut99_standard(&t3d_actor.class)
                } else {
                    t3d_actor.class.clone()
                };
                rec.location = t3d_actor.get_location();
                if let Some(tag) = t3d_actor.get_property("Tag") {
                    rec.tag = tag.trim_matches('"').to_string();
                }
                if let Some(event) = t3d_actor.get_property("Event") {
                    rec.event = event.trim_matches('"').to_string();
                }
                if let Some(grp) = t3d_actor.get_property("Group") {
                    rec.group = grp.trim_matches('"').to_string();
                }
                let rec_cls_lower = rec.class_name.to_ascii_lowercase();
                if rec_cls_lower.contains("light") {
                    rec.light_brightness = t3d_actor
                        .get_property("LightBrightness")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(64);
                    rec.light_radius = t3d_actor
                        .get_property("LightRadius")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(32);
                    rec.light_hue = t3d_actor
                        .get_property("LightHue")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    rec.light_saturation = t3d_actor
                        .get_property("LightSaturation")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(255);
                    level.lights.push(rec);
                } else if rec_cls_lower.contains("playerstart") {
                    level.player_starts.push(rec);
                } else if rec_cls_lower.contains("pathnode") || rec_cls_lower.contains("lift") || rec_cls_lower.contains("jump") {
                    level.path_nodes.push(rec);
                } else {
                    level.other_actors.push(rec);
                }
            }
        }

        // Populate AI Navigation ReachSpecs into level.reach_specs
        level.reach_specs = snap.reach_specs.clone();

        // Embed any MyLevel textures (UTexture + UPalette) and sounds (USound) into the .UNR package
        level.embedded_textures = snap.embedded_textures.clone();
        level.embedded_sounds = snap.embedded_sounds.clone();

        Ok(level)
    }

    /// Maps Unreal 227 or Studio-specific actor class names to standard UT99 (v436/v469) classes.
    pub fn map_class_name_for_ut99_standard(cls_name: &str) -> String {
        let lower = cls_name.to_ascii_lowercase();
        let short = lower.rsplit('.').next().unwrap_or(&lower);
        match short {
            "ujumppad" => "Kicker".to_string(),
            "skeletalmeshactor" | "staticmeshactor" | "vegetationgenerator" => "Decoration".to_string(),
            "particleemitter" | "xparticleemitter" | "weatheremitter" => "SmokeGenerator".to_string(),
            "proceduralsoundactor" => "AmbientSound".to_string(),
            "dynamiczoneinfo" => "ZoneInfo".to_string(),
            "visibilitynotify" | "projector" => "Trigger".to_string(),
            _ => cls_name.to_string(),
        }
    }

    /// Automatically bakes/converts Unreal 227 or custom Studio actors into 100% compatible UT99 (v436/v469) actors.
    pub fn bake_actor_for_ut99_standard(actor: &ut99_format::ActorRecord) -> Vec<ut99_format::ActorRecord> {
        let lower = actor.class_name.to_ascii_lowercase();
        let short = lower.rsplit('.').next().unwrap_or(&lower);
        if short == "ujumppad" {
            let base_tag = if actor.tag.is_empty() {
                format!("{}_Jump", actor.name)
            } else {
                actor.tag.clone()
            };
            let mut kicker = actor.clone();
            kicker.class_name = "Kicker".to_string();
            kicker.name = format!("{}_Kicker", actor.name);
            kicker.tag = base_tag.clone();
            if !kicker.properties.contains_key("KickVelocity") {
                kicker.properties.insert(
                    "KickVelocity".to_string(),
                    TaggedProperty {
                        name: "KickVelocity".to_string(),
                        array_index: 0,
                        value: TaggedValue::Vector(glam::Vec3::new(0.0, 0.0, 750.0)),
                    },
                );
            }
            kicker.properties.insert(
                "KickedClasses".to_string(),
                TaggedProperty {
                    name: "KickedClasses".to_string(),
                    array_index: 0,
                    value: TaggedValue::Name("Pawn".to_string()),
                },
            );
            kicker.properties.insert(
                "bKillVelocity".to_string(),
                TaggedProperty {
                    name: "bKillVelocity".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );

            let mut lift_exit = ut99_format::ActorRecord {
                name: format!("{}_LiftExit", actor.name),
                class_name: "LiftExit".to_string(),
                flags: 0x02000004,
                location: actor.location,
                tag: base_tag.clone(),
                group: actor.group.clone(),
                ..Default::default()
            };
            lift_exit.properties.insert(
                "LiftTag".to_string(),
                TaggedProperty {
                    name: "LiftTag".to_string(),
                    array_index: 0,
                    value: TaggedValue::Name(base_tag.clone()),
                },
            );

            let mut jump_spot = ut99_format::ActorRecord {
                name: format!("{}_JumpSpot", actor.name),
                class_name: "JumpSpot".to_string(),
                flags: 0x02000004,
                location: actor.location + glam::Vec3::new(0.0, 0.0, 256.0),
                tag: base_tag.clone(),
                group: actor.group.clone(),
                ..Default::default()
            };
            jump_spot.properties.insert(
                "LiftTag".to_string(),
                TaggedProperty {
                    name: "LiftTag".to_string(),
                    array_index: 0,
                    value: TaggedValue::Name(base_tag),
                },
            );

            vec![kicker, lift_exit, jump_spot]
        } else {
            let mut converted = actor.clone();
            converted.class_name = Self::map_class_name_for_ut99_standard(&actor.class_name);
            vec![converted]
        }
    }

    /// Canonical UT99 `Package.Class` qualification for `.t3d` `Begin Actor Class=...` lines (`REQ-J02`).
    /// Preserves already-qualified `Package.Class` identifiers and maps unqualified UT99 classes
    /// to their canonical `Engine.*`, `Botpack.*`, or `UnrealShare.*` package.
    pub fn qualify_ut99_actor_class(cls_name: &str) -> String {
        let trimmed = cls_name.trim();
        if trimmed.contains('.') {
            return trimmed.to_string();
        }
        let lower = trimmed.to_ascii_lowercase();
        let pkg = match lower.as_str() {
            "levelinfo"
            | "brush"
            | "mover"
            | "assertmover"
            | "attachmover"
            | "elevatormover"
            | "gradualmover"
            | "loopmover"
            | "mixmover"
            | "rotatingmover"
            | "light"
            | "trigger"
            | "triggerlight"
            | "spotlight"
            | "playerstart"
            | "pathnode"
            | "inventoryspot"
            | "liftexit"
            | "liftcenter"
            | "teleporter"
            | "warpzoneinfo"
            | "warpzonemarker"
            | "zoneinfo"
            | "waterzone"
            | "lavazone"
            | "slimezone"
            | "nitrogenzone"
            | "vacuumzone"
            | "pressurezone"
            | "skyzoneinfo"
            | "ambientsound"
            | "musicscriptevent"
            | "specialevent"
            | "dispatcher"
            | "counter"
            | "roundrobin"
            | "stochastictrigger"
            | "interpolationpoint"
            | "blockall"
            | "blockmonsters"
            | "blockplayer"
            | "keypoint"
            | "locationid"
            | "decal" => "Engine",
            "smokegenerator"
            | "explodingwall"
            | "breakingglass"
            | "woodenbox"
            | "steelbox"
            | "barrel"
            | "tarydiumbarrel"
            | "torchflame"
            | "boulder"
            | "teamtrigger"
            | "distanceviewtrigger" => "UnrealShare",
            _ => "Botpack",
        };
        format!("{pkg}.{trimmed}")
    }

    /// Exports a `T3dMap` where every actor's `Class=` header uses canonical `Package.Class` qualification (`REQ-J02`).
    pub fn export_t3d_map_qualified(state: &EditorState) -> T3dMap {
        let mut map = Self::export_t3d_map(state);
        for actor in &mut map.actors {
            actor.class = Self::qualify_ut99_actor_class(&actor.class);
        }
        map
    }
}


